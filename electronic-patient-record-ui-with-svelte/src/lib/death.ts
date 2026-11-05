// Death assertions (BV-1, H-046), as staff see and act on them. Pure, apart from the two loaders. The
// service decides what is allowed and who may do it; this offers only the steps that fit where an
// assertion stands, and sends nothing without the words the service requires. Who is acting is the
// token: nothing here names a person. "Not known to have died" never means alive, so no text here says so.
import { siteWhenText } from "./access-log.js";
import { call } from "./api.js";
import { STRINGS } from "./strings.js";

export type Status =
  "reported" | "verified" | "certified" | "registered" | "withdrawn-in-error";
export type Source =
  | "registrar"
  | "national-demographics"
  | "hospital"
  | "general-practice"
  | "care-home"
  | "coroner"
  | "family-member"
  | "other-system";
export const SOURCES: Source[] = [
  "family-member",
  "hospital",
  "general-practice",
  "care-home",
  "coroner",
  "registrar",
  "national-demographics",
  "other-system",
];

export type Occurred =
  { kind: "known"; at: string } | { kind: "not-known"; reason: string };

export interface StoredAssertion {
  id: string;
  patient_id: string;
  version: number;
  author: string;
  recorded_at: string;
  assertion: {
    source: Source;
    reported_by: { kind: "person" | "feed"; id: string };
    occurred: Occurred;
    status: Status;
    reason: string;
  };
}

export type Answer =
  | { answer: "not-known-to-have-died"; holds_contact: false }
  | {
      answer: "asserted";
      status: Exclude<Status, "withdrawn-in-error">;
      confirmed: boolean;
      occurred: string | null;
      dates_differ: boolean;
      holds_contact: true;
    };

export interface DeathLoad {
  status: number;
  answer: Answer | null;
}

/** The one answer for a patient. Any status but 200 is a look that failed, never "nothing reported". */
export async function loadDeath(
  patientId: string,
  options: { background?: boolean; purpose?: string } = {},
): Promise<DeathLoad> {
  const res = await call<Answer>(
    "GET",
    `/api/patients/${patientId}/death`,
    undefined,
    options.purpose,
    options,
  );
  return {
    status: res.status,
    answer: res.status === 200 ? res.body : null,
  };
}

export async function loadAssertions(patientId: string): Promise<{
  status: number;
  assertions: StoredAssertion[];
}> {
  const res = await call<{ assertions: StoredAssertion[] }>(
    "GET",
    `/api/patients/${patientId}/death/assertions`,
  );
  return {
    status: res.status,
    assertions: res.status === 200 ? res.body.assertions : [],
  };
}

export type BannerDeath =
  | { kind: "none" }
  | { kind: "unknown"; text: string }
  | { kind: "asserted"; text: string };

/**
 * The banner's line. Nothing while loading, or when the page did not ask (a patient's own view);
 * nothing when no assertion stands, because silence there is not a statement that the person is
 * living; a failed look says so, because silence there would read as "nothing reported".
 */
export function bannerDeath(load: DeathLoad | null): BannerDeath {
  if (!load) return { kind: "none" };
  const a = load.answer;
  if (!a) return { kind: "unknown", text: STRINGS.bannerDeathUnknown };
  if (a.answer !== "asserted") return { kind: "none" };
  const head = a.confirmed
    ? STRINGS.bannerDeathConfirmed.replace("{status}", statusText(a.status))
    : STRINGS.bannerDeathReported;
  const date = a.occurred
    ? STRINGS.bannerDeathDate.replace("{date}", a.occurred)
    : STRINGS.bannerDeathNoDate;
  const differ = a.dates_differ ? ` ${STRINGS.bannerDeathDatesDiffer}` : "";
  return {
    kind: "asserted",
    text: `${head} ${date}${differ} ${STRINGS.bannerDeathHeld}`,
  };
}

export function statusText(s: Status): string {
  return STRINGS.deathStatuses[s];
}

/** The steps offered for an assertion (the service still decides, and keeps withdrawal to a clinician). */
export function stepsFor(status: Status): Status[] {
  const order: Status[] = ["verified", "certified", "registered"];
  if (status === "withdrawn-in-error") return [];
  const from = order.indexOf(status);
  return [...order.slice(from + 1), "withdrawn-in-error"];
}

export interface ReportForm {
  source: Source | "";
  date: string;
  noDateReason: string;
  reason: string;
}

export const emptyReport = (): ReportForm => ({
  source: "",
  date: "",
  noDateReason: "",
  reason: "",
});

/** The body of a report, or the sentence saying what to fix. A date, or why there is none, is required. */
export function reportBody(
  f: ReportForm,
):
  { ok: true; body: Record<string, unknown> } | { ok: false; problem: string } {
  if (!f.source) return { ok: false, problem: STRINGS.deathSourceNeeded };
  const date = f.date.trim();
  const noDate = f.noDateReason.trim();
  if (!date && !noDate) return { ok: false, problem: STRINGS.deathDateNeeded };
  const reason = f.reason.trim();
  if (!reason) return { ok: false, problem: STRINGS.deathReasonNeeded };
  return {
    ok: true,
    body: {
      source: f.source,
      occurred: date
        ? { kind: "known", at: date }
        : { kind: "not-known", reason: noDate },
      reason,
    },
  };
}

/** The body of a step, or the sentence saying what to fix. */
export function stepBody(
  to: Status,
  expectedVersion: number,
  reason: string,
):
  { ok: true; body: Record<string, unknown> } | { ok: false; problem: string } {
  const why = reason.trim();
  if (!why) return { ok: false, problem: STRINGS.deathReasonNeeded };
  return {
    ok: true,
    body: { expected_version: expectedVersion, to, reason: why },
  };
}

/** One assertion in a line: what, from where, and when it was written. */
export function assertionLine(a: StoredAssertion, zone?: string): string {
  const o = a.assertion.occurred;
  const date =
    o.kind === "known"
      ? STRINGS.bannerDeathDate.replace("{date}", o.at)
      : STRINGS.deathNoDateBecause.replace("{reason}", o.reason);
  return STRINGS.deathLine
    .replace("{status}", statusText(a.assertion.status))
    .replace("{source}", STRINGS.deathSources[a.assertion.source])
    .replace("{date}", date)
    .replace("{when}", siteWhenText(a.recorded_at, zone));
}
