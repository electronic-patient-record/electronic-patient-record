// The patient-visible access log (SE-6): the API's entries, turned into a sentence.
import { STRINGS } from "./strings.js";

export type LogEntry = {
  id: number;
  at: string;
  by: "you" | "staff";
  roles?: string[];
  organisations?: string[];
  did: string;
  what: string;
  refused: boolean;
  purpose: string | null;
  break_glass: boolean;
};

export type LogPage = { entries: LogEntry[]; next: number | null };

const DID = STRINGS.accessLogDid as Record<string, string>;
const WHAT = STRINGS.accessLogWhat as Record<string, string>;

/**
 * "2026-10-21T09:00:00.000000Z" as "2026-10-21 09:00 UTC", or in `zone` with the zone named
 * ("2026-10-21 10:00 GMT+1"); anything that is not an instant is shown as given.
 */
export function whenText(at: string, zone = "UTC"): string {
  const m = /^(\d{4}-\d{2}-\d{2})T(\d{2}:\d{2})/.exec(at);
  if (!m) return at;
  if (zone === "UTC") return `${m[1]} ${m[2]} UTC`;
  const t = Date.parse(at);
  if (Number.isNaN(t)) return at;
  const p = new Intl.DateTimeFormat("en-US", {
    timeZone: zone,
    hourCycle: "h23",
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    timeZoneName: "short",
  }).formatToParts(new Date(t));
  const g = (k: string) => p.find((x) => x.type === k)?.value ?? "";
  return `${g("year")}-${g("month")}-${g("day")} ${g("hour")}:${g("minute")} ${g("timeZoneName")}`;
}

/**
 * A server instant on the site's clock with the UTC time beside it: "2026-10-21 09:30 EDT (13:30 UTC)".
 * With no zone, or UTC, exactly what [`whenText`] says. An unknown zone name also falls back to that.
 */
export function siteWhenText(at: string, zone?: string): string {
  const utc = whenText(at);
  if (!zone || zone === "UTC") return utc;
  try {
    const local = whenText(at, zone);
    return local === at ? utc : `${local} (${utc.slice(11, 16)} UTC)`;
  } catch {
    return utc;
  }
}

/** The reader's IANA time zone, or UTC when the browser will not say. */
export function readerZone(): string {
  try {
    return Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC";
  } catch {
    return "UTC";
  }
}

const ROLE = STRINGS.roleNames as Record<string, string>;

/** "A member of staff (clinician, riverside)": the role and organisation the token carried, if recorded. */
function staff(e: LogEntry): string {
  // A proxy is not staff: the patient must be able to tell who is reading their record for them (PA-5).
  if ((e.roles ?? []).includes("proxy")) return STRINGS.accessLogProxy;
  const bits = [
    ...(e.roles ?? []).map((r) => ROLE[r] ?? r),
    ...(e.organisations ?? []),
  ];
  return bits.length > 0
    ? `${STRINGS.accessLogStaff} (${bits.join(", ")})`
    : STRINGS.accessLogStaff;
}

/** A purpose code in words for mid-sentence ("running the service"); a code from a newer API is shown as it is. */
function purposeWords(code: string): string {
  const label = (STRINGS.accessLogPurposes as Record<string, string>)[code];
  return label ? label.charAt(0).toLowerCase() + label.slice(1) : code;
}

/** One entry as a sentence. Unknown words from a newer API are shown, not hidden. */
export function describe(e: LogEntry): string {
  const who = e.by === "you" ? STRINGS.accessLogYou : staff(e);
  const did = DID[e.did] ?? e.did;
  const what = WHAT[e.what] ?? e.what;
  const parts = [`${who} ${did} ${STRINGS.accessLogYour} ${what}.`];
  if (e.purpose)
    parts.push(`${STRINGS.accessLogPurpose}: ${purposeWords(e.purpose)}.`);
  if (e.break_glass) parts.push(STRINGS.accessLogBreakGlass);
  return parts.join(" ");
}

export type LogFilter = {
  from: string;
  to: string;
  by: "" | "you" | "staff";
  refused: "" | "true" | "false";
  /** A purpose-of-use code the API knows, or empty for any. */
  purpose: string;
};
export const NO_FILTER: LogFilter = {
  from: "",
  to: "",
  by: "",
  refused: "",
  purpose: "",
};

/** The purpose-of-use codes the API accepts as a filter (`policy::Purpose`), in the order shown. */
export const LOG_PURPOSES = [
  "treatment",
  "emergency",
  "operations",
  "payment",
  "research",
  "patient-access",
  "audit",
] as const;

/** `YYYY-MM-DD` as a UTC time, or null when it is not a real day. */
function day(v: string): number | null {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(v);
  if (!m) return null;
  const [y, mo, d] = [Number(m[1]), Number(m[2]), Number(m[3])];
  const t = Date.UTC(y, mo - 1, d);
  const back = new Date(t);
  return back.getUTCFullYear() === y &&
    back.getUTCMonth() === mo - 1 &&
    back.getUTCDate() === d
    ? t
    : null;
}

/** How far `zone`'s wall clock is ahead of UTC at the instant `t` (ms). */
function offsetAt(t: number, zone: string): number {
  const whole = Math.floor(t / 1000) * 1000;
  const p = new Intl.DateTimeFormat("en-US", {
    timeZone: zone,
    hourCycle: "h23",
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  }).formatToParts(new Date(whole));
  const g = (k: string) => Number(p.find((x) => x.type === k)?.value);
  return (
    Date.UTC(
      g("year"),
      g("month") - 1,
      g("day"),
      g("hour"),
      g("minute"),
      g("second"),
    ) - whole
  );
}

/**
 * The instant a calendar day starts in `zone`. The offset is looked up twice so a day that starts
 * on the far side of a clock change is still right; a zone with no midnight on that day (a few
 * have none, where the clocks skip it) gets the first moment after the gap's start.
 */
export function dayStart(dayText: string, zone: string): number | null {
  const base = day(dayText);
  if (base === null) return null;
  const first = base - offsetAt(base, zone);
  return base - offsetAt(first, zone);
}

const iso = (t: number) => new Date(t).toISOString().replace(/\.\d{3}Z$/, "Z");

/**
 * The query string for the access log: the filter, and the page to continue from. The days are the
 * reader's own, in `zone`: each is sent as the UTC instant it starts at. The last day is included,
 * but the API's `to` is exclusive, so the start of the next day is sent. Null when a date is not a
 * real day (the page then says so and sends nothing).
 */
export function logQuery(
  f: LogFilter,
  before?: number,
  zone = "UTC",
): string | null {
  const q = new URLSearchParams();
  if (f.from) {
    const t = dayStart(f.from, zone);
    if (t === null) return null;
    q.set("from", iso(t));
  }
  if (f.to) {
    const t = day(f.to);
    if (t === null) return null;
    const next = new Date(t + 86_400_000).toISOString().slice(0, 10);
    const start = dayStart(next, zone);
    if (start === null) return null;
    q.set("to", iso(start));
  }
  if (f.by) q.set("by", f.by);
  if (f.refused) q.set("refused", f.refused);
  if (f.purpose) q.set("purpose", f.purpose);
  if (before !== undefined) q.set("before", String(before));
  const text = q.toString();
  return text ? `?${text}` : "";
}

/** The CSV download's query: the filter that is showing, then the format. Null when a date is not real. */
export function logCsvQuery(f: LogFilter, zone = "UTC"): string | null {
  const q = logQuery(f, undefined, zone);
  return q === null ? null : `${q}${q ? "&" : "?"}format=csv`;
}
