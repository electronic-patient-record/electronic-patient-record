// A suspicion on a referral (RS-1 to RS-7), as a clinician reads it. The service owns the pathway, the
// plan (priority and due days, counted in the site's days) and the outcome; this file only shapes the
// answer for the screen, and builds the bodies for raising one and recording its outcome. Amending is
// not on a screen yet.
import { call } from "./api.js";
import type { Priority } from "./referrals.js";

export type OutcomeKind = "confirmed" | "ruled-out" | "withdrawn";

export interface Due {
  id: string;
  text: string;
  days: number;
  due_on: string;
  overdue: boolean;
}

export interface Plan {
  pathway: string;
  version: number;
  title: string;
  approved: boolean;
  latest_version: number;
  clock_from: "point-of-suspicion" | "referral-sent";
  clock_start: string;
  time_zone: string;
  priority: Priority;
  instruction: string | null;
  targets: Due[];
  no_target_reason: string | null;
  subtype: { scheme: string; title: string; code: string; name: string } | null;
}

export interface Suspicion {
  condition: string;
  subtype?: { scheme: string; code: string };
  point_of_suspicion: string;
  patient_told: "told" | "not-told";
  instruction?: string;
}

export interface StoredSuspicion {
  version: number;
  suspicion: Suspicion;
  plan: Plan;
}

export interface Outcome {
  kind: OutcomeKind;
  diagnosis?: string;
  by?: string;
  detail: string;
}

export interface SuspicionHistory {
  current: StoredSuspicion | null;
  open: boolean;
  outcome: { seq: number; outcome: Outcome } | null;
  outcomes_in_error: unknown[];
}

export interface SuspicionLoad {
  status: number;
  history: SuspicionHistory | null;
  pathways: Pathway[];
}

export async function loadSuspicion(
  patientId: string,
  referralId: string,
): Promise<SuspicionLoad> {
  const res = await call<SuspicionHistory & { pathways?: Pathway[] }>(
    "GET",
    `/api/patients/${patientId}/referrals/${referralId}/suspicion`,
  );
  return {
    status: res.status,
    history: res.status === 200 ? res.body : null,
    pathways: res.status === 200 ? (res.body.pathways ?? []) : [],
  };
}

/** How a target reads: overdue while open; once ended, whether its day had passed at the outcome. */
export type TargetMark = "due" | "overdue" | "passed-at-outcome";

export function targetMark(t: Due, ended: boolean): TargetMark {
  if (!t.overdue) return "due";
  return ended ? "passed-at-outcome" : "overdue";
}

/** The outcome's words, from its kind; a field the kind needs but lacks reads as empty, never invented. */
export function outcomeText(
  o: Outcome,
  words: Record<OutcomeKind, string>,
): string {
  return words[o.kind]
    .replace("{diagnosis}", o.diagnosis ?? "")
    .replace("{by}", o.by ?? "")
    .replace("{detail}", o.detail);
}

/** One pathway version as the service sends it (the newest of each), for raising a suspicion. */
export interface Pathway {
  id: string;
  version: number;
  title: string;
  approved_by: string;
  priority?: Priority;
  needs_agreement: boolean;
  instruction?: string;
  band: { id: string; text: string; needs_result: boolean }[];
  factor: { id: string; text: string }[];
  subtypes?: {
    scheme: string;
    title: string;
    required: boolean;
    code: { code: string; name: string }[];
  };
}

/** What the raise form holds; an empty string is "not chosen". */
export interface RaiseForm {
  pathway: string;
  condition: string;
  point: string; // a datetime-local value, in the reader's own time
  source: "" | "clinician" | "screening-programme" | "result";
  by: string;
  groundsKind: "" | "criterion" | "result" | "judgement";
  groundsDetail: string;
  resultId: string;
  told: "" | "told" | "not-told";
  band: string;
  factors: string[];
  subtype: string;
  agreed: "" | "yes" | "no";
  instruction: string;
}

export const EMPTY_FORM: RaiseForm = {
  pathway: "",
  condition: "",
  point: "",
  source: "",
  by: "",
  groundsKind: "",
  groundsDetail: "",
  resultId: "",
  told: "",
  band: "",
  factors: [],
  subtype: "",
  agreed: "",
  instruction: "",
};

/** The problem keys the form can name before sending; the service checks everything again. */
export type RaiseProblem =
  | "pathway"
  | "condition"
  | "point"
  | "source"
  | "by"
  | "grounds"
  | "result"
  | "told"
  | "band"
  | "subtype"
  | "agreed";

/**
 * The body of `POST …/suspicion`, or what is missing. Nothing is defaulted that a clinician must decide:
 * whether the patient was told, the band, a required subtype, and the person's agreement are chosen or
 * the form says so. The point of suspicion is read in the reader's own time and sent as UTC.
 */
export function buildSuspicion(
  f: RaiseForm,
  pathways: Pathway[],
): { body: unknown; problems: RaiseProblem[] } {
  const problems: RaiseProblem[] = [];
  const p = pathways.find((x) => x.id === f.pathway);
  if (!p) problems.push("pathway");
  if (!f.condition.trim()) problems.push("condition");
  const at = f.point ? new Date(f.point) : null;
  if (!at || Number.isNaN(at.getTime())) problems.push("point");
  if (!f.source) problems.push("source");
  if (!f.by.trim()) problems.push("by");
  if (!f.groundsKind || !f.groundsDetail.trim()) problems.push("grounds");
  if (f.groundsKind === "result" && !f.resultId.trim()) problems.push("result");
  if (!f.told) problems.push("told");
  if (p && p.band.length > 0 && !p.band.some((b) => b.id === f.band))
    problems.push("band");
  if (p?.subtypes?.required && !f.subtype) problems.push("subtype");
  if (p?.needs_agreement && !f.agreed) problems.push("agreed");
  if (problems.length > 0 || !p || !at) return { body: null, problems };
  const suspicion: Record<string, unknown> = {
    condition: f.condition.trim(),
    point_of_suspicion: at.toISOString(),
    source: f.source,
    by: f.by.trim(),
    grounds: {
      kind: f.groundsKind,
      detail: f.groundsDetail.trim(),
      ...(f.groundsKind === "result" ? { result_id: f.resultId.trim() } : {}),
    },
    patient_told: f.told,
    pathway: p.id,
    pathway_version: p.version,
  };
  if (f.instruction.trim()) suspicion.instruction = f.instruction.trim();
  if (p.band.length > 0) suspicion.band = f.band;
  const factors = f.factors.filter((x) => p.factor.some((y) => y.id === x));
  if (factors.length > 0) suspicion.factors = factors;
  if (p.subtypes && f.subtype)
    suspicion.subtype = { scheme: p.subtypes.scheme, code: f.subtype };
  if (p.needs_agreement) suspicion.agreed = f.agreed === "yes";
  return { body: { suspicion }, problems };
}

/** What the outcome form holds; an empty string is "not chosen". */
export interface OutcomeForm {
  kind: "" | OutcomeKind;
  diagnosis: string;
  by: string;
  detail: string;
}

export const EMPTY_OUTCOME: OutcomeForm = {
  kind: "",
  diagnosis: "",
  by: "",
  detail: "",
};

export type OutcomeProblem = "kind" | "diagnosis" | "by" | "detail";

/**
 * The body of `POST …/suspicion/outcome` against the version the reader saw, or what is missing:
 * confirmed needs the diagnosis, ruled out needs who ruled it out, and every outcome needs what it rests
 * on. Only the fields its kind carries are sent.
 */
export function buildOutcome(
  f: OutcomeForm,
  version: number,
): { body: unknown; problems: OutcomeProblem[] } {
  const problems: OutcomeProblem[] = [];
  if (!f.kind) problems.push("kind");
  if (f.kind === "confirmed" && !f.diagnosis.trim()) problems.push("diagnosis");
  if (f.kind === "ruled-out" && !f.by.trim()) problems.push("by");
  if (!f.detail.trim()) problems.push("detail");
  if (problems.length > 0) return { body: null, problems };
  const outcome: Record<string, unknown> = {
    kind: f.kind,
    detail: f.detail.trim(),
  };
  if (f.kind === "confirmed") outcome.diagnosis = f.diagnosis.trim();
  if (f.kind === "ruled-out") outcome.by = f.by.trim();
  return { body: { outcome, expected_version: version }, problems };
}

/** What the amend form holds: the fields a clinician most often corrects, and the reason. */
export interface AmendForm {
  told: "told" | "not-told";
  point: string; // a datetime-local value, in the reader's own time
  instruction: string;
  reason: string;
}

/** A datetime-local value for an RFC 3339 time, in the reader's own time. */
export function localInput(at: string): string {
  const d = new Date(at);
  if (Number.isNaN(d.getTime())) return "";
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}T${p(d.getHours())}:${p(d.getMinutes())}`;
}

export type AmendProblem = "reason" | "point" | "unchanged";

/**
 * The body of `PUT …/suspicion`: the whole current suspicion with the changed fields, the version seen
 * and the reason. Every other field (pathway, band, factors, subtype, grounds) is sent as it is, so the
 * amendment stays under the same pathway version. A point of suspicion left as shown is not moved.
 */
export function buildAmendment(
  current: Record<string, unknown>,
  version: number,
  f: AmendForm,
): { body: unknown; problems: AmendProblem[] } {
  const problems: AmendProblem[] = [];
  if (!f.reason.trim()) problems.push("reason");
  const next: Record<string, unknown> = { ...current, patient_told: f.told };
  const wasPoint = String(current.point_of_suspicion ?? "");
  if (f.point !== localInput(wasPoint)) {
    const at = f.point ? new Date(f.point) : null;
    if (!at || Number.isNaN(at.getTime())) problems.push("point");
    else next.point_of_suspicion = at.toISOString();
  }
  const instruction = f.instruction.trim();
  if (instruction) next.instruction = instruction;
  else delete next.instruction;
  const same =
    next.patient_told === current.patient_told &&
    next.point_of_suspicion === current.point_of_suspicion &&
    (next.instruction ?? null) === (current.instruction ?? null);
  if (same && !problems.includes("point")) problems.push("unchanged");
  if (problems.length > 0) return { body: null, problems };
  return {
    body: {
      suspicion: next,
      expected_version: version,
      reason: f.reason.trim(),
    },
    problems,
  };
}
