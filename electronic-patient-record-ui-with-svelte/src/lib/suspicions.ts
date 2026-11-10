// A suspicion on a referral (RS-1 to RS-7), as a clinician reads it. The service owns the pathway, the
// plan (priority and due days, counted in the site's days) and the outcome; this file only shapes the
// answer for the screen. Read only: raising, amending and ending a suspicion are not on a screen yet.
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
}

export async function loadSuspicion(
  patientId: string,
  referralId: string,
): Promise<SuspicionLoad> {
  const res = await call<SuspicionHistory>(
    "GET",
    `/api/patients/${patientId}/referrals/${referralId}/suspicion`,
  );
  return {
    status: res.status,
    history: res.status === 200 ? res.body : null,
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
