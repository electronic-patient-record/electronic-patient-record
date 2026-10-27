// Requests to amend a record (RC-4, hazard H-035), as a clinician sees and decides them. Pure, apart
// from `loadAmendments`. The service decides what is allowed; this offers only the steps that fit
// where a request stands, and sends nothing without the words the service requires. Who is acting is
// the token: nothing here names a person.
import { siteWhenText } from "./access-log.js";
import { call } from "./api.js";
import { STRINGS } from "./strings.js";
import type { Notice } from "./vitals.js";

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

export interface AmendmentsLoad {
  status: number;
  amendments: AmendmentView[] | null;
}

export async function loadAmendments(
  patientId: string,
  options: { background?: boolean } = {},
): Promise<AmendmentsLoad> {
  const res = await call<{ amendments: AmendmentView[] }>(
    "GET",
    `/api/patients/${patientId}/amendments`,
    undefined,
    undefined,
    options,
  );
  return { status: res.status, amendments: res.status === 200 ? res.body.amendments : null };
}
