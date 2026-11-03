// Referrals (CA-3), as a clinician sees them. The service owns the state table and sends, with each
// referral, the events that may follow (`next`), who sends each and whether it needs a note; this file
// copies none of that. Nothing here names a person: who is acting is the token.
import { call } from "./api.js";

export type ReferralState =
  | "requested"
  | "accepted"
  | "declined"
  | "appointment-booked"
  | "no-show"
  | "interim-report"
  | "completed"
  | "cancel-requested"
  | "cancelled"
  | "information-requested"
  | "deferred"
  | "redirected";

export type Priority = "routine" | "urgent" | "asap" | "stat";
export const PRIORITIES: Priority[] = ["routine", "urgent", "asap", "stat"];

export interface NextEvent {
  event: string;
  by: "referrer" | "recipient";
  needs_note: boolean;
}

export interface StoredReferral {
  id: string;
  version: number;
  author: string;
  recorded_at: string;
  patient_id: string;
  state: ReferralState;
  referral: {
    recipient: string;
    reason: string;
    question: string;
    priority: Priority;
  };
  next: NextEvent[];
}

/** States whose words come from one unreviewed source (spec/interoperability/closed-loop-referral.md). */
export const PROVISIONAL: ReferralState[] = [
  "information-requested",
  "deferred",
  "redirected",
];

export interface ReferralsLoad {
  status: number;
  referrals: StoredReferral[] | null;
}

export async function loadReferrals(
  patientId: string,
  options: { background?: boolean; purpose?: string } = {},
): Promise<ReferralsLoad> {
  const res = await call<{ referrals: StoredReferral[] }>(
    "GET",
    `/api/patients/${patientId}/referrals`,
    undefined,
    options.purpose,
    options,
  );
  return {
    status: res.status,
    referrals: res.status === 200 ? res.body.referrals : null,
  };
}
