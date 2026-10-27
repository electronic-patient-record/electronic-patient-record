// Vital signs (RC-6) as the page sees them. Pure, apart from `loadObservations`.
//
// Three rules the words and the form keep (hazard H-034):
//  - a missing value says why, and is never shown as 0 or as a dash;
//  - no range means "not assessed", never "normal": a flag is only ever words the API computed;
//  - an empty or unreadable number is not a number: it can never be sent as 0.
import { siteWhenText } from "./access-log.js";
import { call } from "./api.js";
import { STRINGS } from "./strings.js";

export type Flag = "critical-low" | "low" | "normal" | "high" | "critical-high";
export type AbsentReason = "not-done" | "declined" | "failed" | "unknown";
export type ObservationStatus = "final" | "entered-in-error";

export interface Range {
  low?: number | null;
  high?: number | null;
}

/** What the API sends for one observation (the record is flat). */
export interface StoredObservation {
  id: string;
  patient_id: string;
  version: number;
  author: string;
  recorded_at: string;
  site_zone: string;
  code: string;
  value?: number | null;
  unit?: string | null;
  absent_reason?: AbsentReason | null;
  range?: Range | null;
  critical?: Range | null;
  effective_at: string;
  status: ObservationStatus;
  flag: Flag | null;
}

/** The vital signs the service accepts: one unit each, no conversion. */
export const VITALS = [
  { loinc: "8480-6", key: "systolic", unit: "mm[Hg]" },
  { loinc: "8462-4", key: "diastolic", unit: "mm[Hg]" },
  { loinc: "8867-4", key: "heartRate", unit: "/min" },
  { loinc: "9279-1", key: "respiratoryRate", unit: "/min" },
  { loinc: "8310-5", key: "temperature", unit: "Cel" },
  { loinc: "59408-5", key: "oxygenSaturation", unit: "%" },
  { loinc: "29463-7", key: "weight", unit: "kg" },
  { loinc: "8302-2", key: "height", unit: "cm" },
] as const;

export type VitalKey = (typeof VITALS)[number]["key"];

const UNIT_LABEL: Record<string, string> = {
  "mm[Hg]": "mmHg",
  "/min": "/min",
  Cel: "°C",
  "%": "%",
  kg: "kg",
  cm: "cm",
};

/** The unit as a person writes it. An unknown unit is shown as it came, never guessed. */
export const unitLabel = (unit: string): string => UNIT_LABEL[unit] ?? unit;

/** The vital's name, or the code itself for one this does not know. */
export function vitalName(code: string): string {
  const v = VITALS.find((x) => x.loinc === code);
  return v ? STRINGS.vitalNames[v.key] : code;
}

/** The number text as a number, or null for anything that is not plainly one (empty is never 0). */
export function parseNumber(text: string): number | null {
  const t = text.trim();
  if (!/^-?\d+([.,]\d+)?$/.test(t)) return null;
  const n = Number(t.replace(",", "."));
  return Number.isFinite(n) ? n : null;
}

/** "72 /min", or why there is no value. Never "0" or a dash for a value that is not there. */
export function valueText(o: StoredObservation): string {
  if (o.value === null || o.value === undefined) {
    const why = o.absent_reason ? STRINGS.vitalAbsent[o.absent_reason] : STRINGS.vitalAbsentUnstated;
    return `${STRINGS.vitalNoValue} ${why}`;
  }
  const unit = o.unit ? ` ${unitLabel(o.unit)}` : "";
  return `${new Intl.NumberFormat("en", { maximumFractionDigits: 6 }).format(o.value)}${unit}`;
}

/**
 * Where the value sits, in words. A value with no flag says it was not assessed and why; nothing here
 * ever says "normal" except the API's own `normal`, which means "within the range the sender gave".
 */
export function assessmentText(o: StoredObservation): string {
  if (o.value === null || o.value === undefined) return STRINGS.vitalNotAssessedNoValue;
  if (o.flag === null || o.flag === undefined) return STRINGS.vitalNotAssessedNoRange;
  return STRINGS.vitalFlags[o.flag];
}

/** The reference range as text ("60 to 100"), or null when none was given. */
export function rangeText(r: Range | null | undefined): string | null {
  if (!r) return null;
  const low = r.low ?? null;
  const high = r.high ?? null;
  if (low === null && high === null) return null;
  if (low !== null && high !== null) return `${low} ${STRINGS.vitalRangeTo} ${high}`;
  return low !== null ? `${STRINGS.vitalRangeFrom} ${low}` : `${STRINGS.vitalRangeUpTo} ${high}`;
}

/** One line: name, value, the words on where it sits, and when, on the site's clock. */
export function vitalLine(o: StoredObservation, siteZone?: string): string {
  return [
    vitalName(o.code),
    valueText(o),
    assessmentText(o),
    siteWhenText(o.effective_at, siteZone),
  ].join(" · ");
}

/** One vital's readings over time. */
export interface Trend {
  code: string;
  name: string;
  /** Newest first. */
  rows: StoredObservation[];
}

/**
 * The standing readings grouped by vital, in the order of `VITALS` (a code this page does not know
 * comes last, under its code), each newest first. An entry in error is left out. A reading with no
 * value stays as a row, with its reason: a gap is not a zero and not a missing row. Nothing is
 * averaged, smoothed or interpolated, and no flag is computed here.
 */
export function trends(observations: StoredObservation[]): Trend[] {
  const standing = observations.filter((o) => o.status !== "entered-in-error");
  const known: string[] = VITALS.map((v) => v.loinc);
  const codes = [
    ...known.filter((c) => standing.some((o) => o.code === c)),
    ...[...new Set(standing.map((o) => o.code))].filter((c) => !known.includes(c)).sort(),
  ];
  return codes.map((code) => ({
    code,
    name: vitalName(code),
    rows: standing
      .filter((o) => o.code === code)
      .sort((a, b) => Date.parse(b.effective_at) - Date.parse(a.effective_at)),
  }));
}

/** One stored version, with the reason it was written. */
export type ObservationVersion = StoredObservation & { reason?: string | null };

export interface HistoryLoad {
  status: number;
  versions: ObservationVersion[] | null;
}

/** Every version of one observation, oldest first. */
export async function loadHistory(patientId: string, observationId: string): Promise<HistoryLoad> {
  const res = await call<{ versions: ObservationVersion[] }>(
    "GET",
    `/api/patients/${patientId}/observations/${observationId}/versions`,
  );
  return { status: res.status, versions: res.status === 200 ? res.body.versions : null };
}

export interface VitalsLoad {
  status: number;
  observations: StoredObservation[] | null;
}

export async function loadObservations(
  patientId: string,
  options: { background?: boolean } = {},
): Promise<VitalsLoad> {
  const res = await call<{ observations: StoredObservation[] }>(
    "GET",
    `/api/patients/${patientId}/observations`,
    undefined,
    undefined,
    options,
  );
  return {
    status: res.status,
    observations: res.status === 200 ? res.body.observations : null,
  };
}

// ---- Time: a wall-clock time typed in the form is the site's, and is sent as the UTC instant ----

function zoneOffsetMs(zone: string, atUtcMs: number): number {
  const p = new Intl.DateTimeFormat("en-US", {
    timeZone: zone,
    hourCycle: "h23",
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  }).formatToParts(new Date(atUtcMs));
  const g = (k: string) => Number(p.find((x) => x.type === k)?.value ?? "0");
  const asUtc = Date.UTC(g("year"), g("month") - 1, g("day"), g("hour"), g("minute"), g("second"));
  return asUtc - Math.floor(atUtcMs / 1000) * 1000;
}

/**
 * A wall-clock time ("2026-10-26T09:30", as a datetime-local box gives it) in `zone`, as an RFC 3339
 * UTC instant. Null when it is not a real time or the zone is unknown. A time the zone skips (the
 * hour a clock goes forward) is read as the same wall time an hour **earlier** (02:30 that does not
 * exist becomes 01:30 standard time), and a time it repeats (the hour a clock goes back) as the first
 * of the two. Both are arbitrary but fixed, and the server's 5-minute future limit still applies.
 */
export function wallTimeToUtc(wall: string, zone: string): string | null {
  const m = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})$/.exec(wall.trim());
  if (!m) return null;
  const [y, mo, d, h, mi] = m.slice(1).map(Number) as [number, number, number, number, number];
  const probe = new Date(Date.UTC(y, mo - 1, d, h, mi));
  if (
    probe.getUTCFullYear() !== y ||
    probe.getUTCMonth() !== mo - 1 ||
    probe.getUTCDate() !== d ||
    probe.getUTCHours() !== h ||
    probe.getUTCMinutes() !== mi
  ) {
    return null;
  }
  try {
    const guess = probe.getTime();
    // The offset at the guess, then at the answer it gives: they differ only across a clock change.
    let utc = guess - zoneOffsetMs(zone, guess);
    utc = guess - zoneOffsetMs(zone, utc);
    return new Date(utc).toISOString().replace(/\.\d{3}Z$/, "Z");
  } catch {
    return null;
  }
}

/** Now as the site's wall-clock "YYYY-MM-DDTHH:mm", for the form's default. */
export function nowInZone(now: Date, zone: string): string {
  try {
    const p = new Intl.DateTimeFormat("en-US", {
      timeZone: zone,
      hourCycle: "h23",
      year: "numeric",
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
    }).formatToParts(now);
    const g = (k: string) => p.find((x) => x.type === k)?.value ?? "00";
    return `${g("year")}-${g("month")}-${g("day")}T${g("hour")}:${g("minute")}`;
  } catch {
    return now.toISOString().slice(0, 16);
  }
}

// ---- The form's fields as a request ----

export interface VitalForm {
  vital: VitalKey;
  value: string;
  /** No value: a reason is required, and the value box is ignored. */
  noValue: boolean;
  absentReason: AbsentReason | "";
  low: string;
  high: string;
  /** Wall time in the site's zone. */
  measuredAt: string;
  siteZone: string;
}

/** The request body for the form, or the sentence saying what to fix. Nothing is sent as 0 by default. */
export function formToRequest(
  f: VitalForm,
): { ok: true; record: Record<string, unknown> } | { ok: false; problem: string } {
  const vital = VITALS.find((v) => v.key === f.vital);
  if (!vital) return { ok: false, problem: STRINGS.vitalPickOne };
  const at = wallTimeToUtc(f.measuredAt, f.siteZone);
  if (at === null) return { ok: false, problem: STRINGS.vitalTimeInvalid };
  const record: Record<string, unknown> = {
    code: vital.loinc,
    effective_at: at,
    status: "final",
  };
  if (f.noValue) {
    if (!f.absentReason) return { ok: false, problem: STRINGS.vitalReasonNeeded };
    record.absent_reason = f.absentReason;
  } else {
    const value = parseNumber(f.value);
    if (value === null) return { ok: false, problem: STRINGS.vitalValueInvalid };
    record.value = value;
    record.unit = vital.unit;
  }
  const lowText = f.low.trim();
  const highText = f.high.trim();
  if (lowText || highText) {
    const low = lowText ? parseNumber(lowText) : null;
    const high = highText ? parseNumber(highText) : null;
    if ((lowText && low === null) || (highText && high === null)) {
      return { ok: false, problem: STRINGS.vitalRangeInvalid };
    }
    record.range = {
      ...(low !== null ? { low } : {}),
      ...(high !== null ? { high } : {}),
    };
  }
  return { ok: true, record };
}

/**
 * The whole record for "entered in error" (the service's PUT takes the whole record): the stored
 * fields unchanged, with only the status changed. Nothing is recomputed, so no value is altered.
 */
export function enteredInErrorRecord(o: StoredObservation): Record<string, unknown> {
  const record: Record<string, unknown> = {
    code: o.code,
    effective_at: o.effective_at,
    status: "entered-in-error",
  };
  if (o.value !== null && o.value !== undefined) record.value = o.value;
  if (o.unit) record.unit = o.unit;
  if (o.absent_reason) record.absent_reason = o.absent_reason;
  if (o.range) record.range = o.range;
  if (o.critical) record.critical = o.critical;
  return record;
}
