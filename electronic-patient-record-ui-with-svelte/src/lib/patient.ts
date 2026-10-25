// The patient identity record as the API sends it, and the pure functions that turn it into form
// fields and back. No runes and no `$app/*` imports, so unit tests can use them.
//
// The edit form shows only some fields. `formToPatient` therefore starts from the record being
// edited and changes only what the form owns: a PUT replaces the whole record, so anything the
// form dropped would be deleted from the next version.
import type { ConflictCode } from "./conflicts.js";

export const NHS_NUMBER_SYSTEM = "https://fhir.nhs.uk/Id/nhs-number";

export type Sex = "male" | "female" | "other" | "unknown";
export const SEXES: readonly Sex[] = ["male", "female", "other", "unknown"];

export interface PatientRecord {
  id?: string | null;
  names: { family?: string | null; given: string[] }[];
  birth_date?: string | null;
  sex?: Sex | null;
  identifiers: { system: string; value: string }[];
  addresses: { postal_code?: string | null }[];
  telecom: { system: string; value: string }[];
}

export interface Stored {
  id: string;
  version: number;
  author: string;
  patient: PatientRecord;
  /** The photo version to fetch, or null/absent when there is no photo. */
  photo_version?: number | null;
  /** The site's time zone, whose calendar day the age is counted on (`EPR_SITE_TIME_ZONE`); absent from an older API. */
  site_time_zone?: string;
  /** Set when this record was merged away: the surviving record's id. */
  replaced_by?: string | null;
  /** The records merged into this one and still merged. */
  replaces?: string[];
  /** A drill or test record (EC-10): not a real patient. Absent from an older API means not a drill. */
  drill?: boolean;
}

export interface MatchResult {
  score: number;
  decision: "auto_link" | "review" | "non_match";
  fields: {
    field: string;
    similarity: number | null;
    weight: number;
    note: string;
  }[];
  deterministic: string | null;
  conflicts: string[];
  /** The same reasons as codes, in the same order (matcher 0.3.0); absent from an older API. */
  conflict_codes?: ConflictCode[];
}

/** An existing patient the new record may duplicate, with the matcher's explanation. */
export type FullCandidate = MatchResult & { existing: Stored };

/**
 * A candidate, or a stand-in for one the patient's consent keeps from this reader: it is still a
 * possible duplicate, but carries no record, score or breakdown.
 */
export type Candidate = FullCandidate | { restricted: true };

export const isRestricted = (c: Candidate): c is { restricted: true } =>
  "restricted" in c;

export interface PatientForm {
  family: string;
  given: string;
  birthDate: string;
  sex: "" | Sex;
  nhsNumber: string;
  postalCode: string;
}

export const emptyForm = (): PatientForm => ({
  family: "",
  given: "",
  birthDate: "",
  sex: "",
  nhsNumber: "",
  postalCode: "",
});

const blankToNull = (s: string): string | null => s.trim() || null;

/** An NHS number is written "943 476 5919"; the API stores and checks the digits. */
export const nhsDigits = (s: string): string => s.replace(/[\s-]/g, "");

/** An NHS number as people read it: "9434765919" is "943 476 5919". Anything else is shown as given. */
export const nhsDisplay = (digits: string): string =>
  /^\d{10}$/.test(digits)
    ? `${digits.slice(0, 3)} ${digits.slice(3, 6)} ${digits.slice(6)}`
    : digits;

/**
 * Age on `today` from a stored birth date, which may be partial ("1980-03" or "1980"). A full date
 * gives whole years, or whole months under two years. A partial date gives only the year and is
 * marked approximate; nothing recorded, or nothing readable, or a date in the future gives `null`.
 * Never a guess: a banner that shows a wrong age is a wrong-patient hazard.
 */
export function ageOn(
  birthDate: string | null | undefined,
  now: Date,
  zone = "UTC",
): { years?: number; months?: number; approximate: boolean } | null {
  const { year: ty, month: tm, day: td } = calendarDay(now, zone);
  const m = /^(\d{4})(?:-(\d{2})(?:-(\d{2}))?)?$/.exec(birthDate ?? "");
  if (!m) return null;
  const year = Number(m[1]);
  if (year > ty) return null;
  if (m[2] === undefined || m[3] === undefined) {
    return { years: ty - year, approximate: true };
  }
  const month = Number(m[2]);
  const day = Number(m[3]);
  const probe = new Date(Date.UTC(year, month - 1, day));
  if (
    month < 1 ||
    month > 12 ||
    probe.getUTCMonth() !== month - 1 ||
    probe.getUTCDate() !== day
  )
    return null;
  let months = (ty - year) * 12 + (tm - month) - (td < day ? 1 : 0);
  if (months < 0) return null;
  return months < 24
    ? { months, approximate: false }
    : { years: Math.floor(months / 12), approximate: false };
}

/**
 * The calendar day it is at `now` in `zone` (an IANA name): the site's own day, so a patient's age
 * changes at the site's midnight for everyone. An unknown zone name falls back to UTC rather than
 * failing the banner.
 */
export function calendarDay(now: Date, zone: string): { year: number; month: number; day: number } {
  const parts = (tz: string) =>
    new Intl.DateTimeFormat("en-US", { timeZone: tz, year: "numeric", month: "2-digit", day: "2-digit" }).formatToParts(now);
  let p;
  try {
    p = parts(zone);
  } catch {
    p = parts("UTC");
  }
  const n = (k: string) => Number(p.find((x) => x.type === k)?.value);
  return { year: n("year"), month: n("month"), day: n("day") };
}

/** The form's view of a record: the first name, the NHS number, the first postcode. */
export function patientToForm(p: PatientRecord): PatientForm {
  const name = p.names[0];
  return {
    family: name?.family ?? "",
    given: (name?.given ?? []).join(" "),
    birthDate: p.birth_date ?? "",
    sex: p.sex ?? "",
    nhsNumber:
      p.identifiers.find((i) => i.system === NHS_NUMBER_SYSTEM)?.value ?? "",
    postalCode: p.addresses[0]?.postal_code ?? "",
  };
}

/**
 * The record to send. `base` is the record being edited; names after the first, identifiers in
 * other systems, other addresses and every phone number or email survive untouched. The `id` is
 * never sent: the server assigns it.
 */
export function formToPatient(
  form: PatientForm,
  base?: PatientRecord,
): PatientRecord {
  const first = {
    family: blankToNull(form.family),
    given: form.given.split(/\s+/).filter(Boolean),
  };
  const nhs = nhsDigits(form.nhsNumber);
  // Change the NHS number where it already sits, so the order of the other identifiers holds.
  const had = (base?.identifiers ?? []).some(
    (i) => i.system === NHS_NUMBER_SYSTEM,
  );
  const identifiers = (base?.identifiers ?? []).flatMap((i) =>
    i.system !== NHS_NUMBER_SYSTEM
      ? [i]
      : nhs
        ? [{ system: NHS_NUMBER_SYSTEM, value: nhs }]
        : [],
  );
  if (nhs && !had)
    identifiers.unshift({ system: NHS_NUMBER_SYSTEM, value: nhs });
  const postal = blankToNull(form.postalCode);
  const addresses = [...(base?.addresses ?? [])];
  if (addresses[0] || postal) {
    addresses[0] = { ...addresses[0], postal_code: postal };
  }
  return {
    names: [first, ...(base?.names ?? []).slice(1)],
    birth_date: blankToNull(form.birthDate),
    sex: form.sex || null,
    identifiers,
    addresses,
    telecom: base?.telecom ?? [],
  };
}

/**
 * The query string for a search, or "" when nothing was entered (the API refuses an empty
 * search, so the UI does not send one).
 */
export function searchQuery(form: PatientForm): string {
  const q = new URLSearchParams();
  if (form.family.trim()) q.set("family", form.family.trim());
  if (form.birthDate.trim()) q.set("birth_date", form.birthDate.trim());
  const nhs = nhsDigits(form.nhsNumber);
  if (nhs) q.set("identifier", `${NHS_NUMBER_SYSTEM}|${nhs}`);
  return q.toString();
}

/** One displayed line per compared field, so two records can sit side by side. */
export const COMPARED_FIELDS = [
  "family",
  "given",
  "birthDate",
  "sex",
  "nhsNumber",
  "postalCode",
] as const satisfies readonly (keyof PatientForm)[];

export function comparisonRows(
  entered: PatientForm,
  existing: PatientRecord,
): {
  field: (typeof COMPARED_FIELDS)[number];
  entered: string;
  existing: string;
  same: boolean;
}[] {
  const other = patientToForm(existing);
  const norm = (f: (typeof COMPARED_FIELDS)[number], v: string) =>
    (f === "nhsNumber" ? nhsDigits(v) : v).trim().toLowerCase();
  return COMPARED_FIELDS.map((field) => ({
    field,
    entered: entered[field],
    existing: other[field],
    same:
      norm(field, entered[field]) === norm(field, other[field]) &&
      norm(field, entered[field]) !== "",
  }));
}
