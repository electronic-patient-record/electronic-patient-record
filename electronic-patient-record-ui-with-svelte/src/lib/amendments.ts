// Requests to amend a record (RC-4, hazard H-035), as a clinician sees and decides them. Pure, apart
// from `loadAmendments`. The service decides what is allowed; this offers only the steps that fit
// where a request stands, and sends nothing without the words the service requires. Who is acting is
// the token: nothing here names a person.
import { siteWhenText } from "./access-log.js";
import { call } from "./api.js";
import { STRINGS } from "./strings.js";
import { allergyLine, type StoredAllergy } from "./allergies.js";
import { vitalLine, type Notice, type StoredObservation } from "./vitals.js";

export type Ground = "not-ours" | "accurate-and-complete" | "not-available-to-requester" | "other";
export const GROUNDS: Ground[] = ["accurate-and-complete", "not-ours", "not-available-to-requester", "other"];

export type State =
  | { state: "requested" }
  | { state: "accepted"; by: string; reason: string }
  | { state: "applied"; version: number }
  | { state: "denied"; by: string; ground: string; reason: string; disagreement?: string | null }
  | { state: "withdrawn" };

export interface AmendmentView {
  id: string;
  patient_id: string;
  /** The version of the request, which a step names as `expected_version`. */
  version: number;
  request: {
    target: { kind: string; id: string; version: number };
    requester: string;
    text: string;
    received_at: string;
    extension?: string | null;
    state: State;
  };
  notice: Notice | null;
  due: string;
  overdue: boolean;
}

export type StepName = "accept" | "deny" | "apply" | "extend";

/** The steps a clinician is offered for a request in this state (the service still decides). */
export function stepsFor(a: AmendmentView): StepName[] {
  const s = a.request.state.state;
  if (s === "requested") return a.request.extension ? ["accept", "deny"] : ["accept", "deny", "extend"];
  if (s === "accepted") return a.request.extension ? ["apply"] : ["apply", "extend"];
  return [];
}

export interface StepForm {
  reason: string;
  ground: Ground | "";
  version: string;
}

/** The request body for a step, or the sentence saying what to fix. Nothing is sent without a reason. */
export function stepBody(
  step: StepName,
  expectedVersion: number,
  f: StepForm,
): { ok: true; body: Record<string, unknown> } | { ok: false; problem: string } {
  const reason = f.reason.trim();
  if (step === "apply") {
    if (!/^\d+$/.test(f.version.trim()) || Number(f.version) < 1) {
      return { ok: false, problem: STRINGS.amendVersionInvalid };
    }
    return { ok: true, body: { expected_version: expectedVersion, step, version: Number(f.version) } };
  }
  if (!reason) return { ok: false, problem: STRINGS.amendReasonNeeded };
  if (step === "deny") {
    if (!f.ground) return { ok: false, problem: STRINGS.amendGroundNeeded };
    return { ok: true, body: { expected_version: expectedVersion, step, ground: f.ground, reason } };
  }
  return { ok: true, body: { expected_version: expectedVersion, step, reason } };
}

/** Where a request stands, in words. */
export function stateText(s: State): string {
  switch (s.state) {
    case "requested":
      return STRINGS.amendStateRequested;
    case "accepted":
      return STRINGS.amendStateAccepted;
    case "applied":
      return STRINGS.amendStateApplied.replace("{n}", String(s.version));
    case "denied":
      return STRINGS.amendStateDenied;
    case "withdrawn":
      return STRINGS.amendStateWithdrawn;
    default:
      return STRINGS.amendStateUnknown;
  }
}

/** When a decision is due, with "overdue" said in words only while it is. */
export function dueText(a: AmendmentView, zone?: string): string {
  const open = a.request.state.state === "requested" || a.request.state.state === "accepted";
  if (!open) return "";
  const when = STRINGS.amendDue.replace("{when}", siteWhenText(a.due, zone));
  return a.overdue ? `${when} ${STRINGS.amendOverdue}` : when;
}

export type TargetKind = "Observation" | "AllergyIntolerance";

/** What each request is about, in a line, by `kind:id`. A record not in the lists has no line. */
export function targetLines(
  observations: StoredObservation[],
  allergies: StoredAllergy[],
  zone?: string,
): Map<string, string> {
  const out = new Map<string, string>();
  for (const o of observations) out.set(`Observation:${o.id}`, vitalLine(o, zone));
  for (const a of allergies) out.set(`AllergyIntolerance:${a.id}`, allergyLine(a));
  return out;
}

/** The line for a request's record, or its kind alone when the record is not in what was loaded. */
export function aboutText(a: AmendmentView, lines: Map<string, string>): string {
  const t = a.request.target;
  return lines.get(`${t.kind}:${t.id}`) ?? t.kind;
}

/** How many requests are waiting for a decision, and how many of those are overdue. */
export function openCounts(rows: AmendmentView[]): { open: number; overdue: number } {
  const open = rows.filter((a) => a.request.state.state === "requested" || a.request.state.state === "accepted");
  return { open: open.length, overdue: open.filter((a) => a.overdue).length };
}

export interface AmendmentsLoad {
  status: number;
  amendments: AmendmentView[] | null;
}

export async function loadAmendments(
  patientId: string,
  options: { background?: boolean; purpose?: string } = {},
): Promise<AmendmentsLoad> {
  const res = await call<{ amendments: AmendmentView[] }>(
    "GET",
    `/api/patients/${patientId}/amendments`,
    undefined,
    options.purpose,
    options,
  );
  return { status: res.status, amendments: res.status === 200 ? res.body.amendments : null };
}

// ---- The patient's side ----

/** The steps the requester is offered (the service still decides). */
export function ownStepsFor(a: AmendmentView): ("withdraw" | "disagree")[] {
  const s = a.request.state;
  if (s.state === "requested") return ["withdraw"];
  if (s.state === "denied" && !s.disagreement) return ["disagree"];
  return [];
}

/** The body asking for a reading to be corrected, or the sentence saying what to fix. */
export function askBody(
  target: { kind: TargetKind; id: string; version: number } | null,
  text: string,
): { ok: true; body: Record<string, unknown> } | { ok: false; problem: string } {
  if (!target) return { ok: false, problem: STRINGS.askPickReading };
  const t = text.trim();
  if (!t) return { ok: false, problem: STRINGS.askTextNeeded };
  return {
    ok: true,
    body: { kind: target.kind, target_id: target.id, version: target.version, text: t },
  };
}

/** The body stating disagreement with a refusal; the statement is kept as written. */
export function disagreeBody(
  expectedVersion: number,
  statement: string,
): { ok: true; body: Record<string, unknown> } | { ok: false; problem: string } {
  if (!statement.trim()) return { ok: false, problem: STRINGS.askStatementNeeded };
  return { ok: true, body: { expected_version: expectedVersion, step: "disagree", statement } };
}
