// The referrer's worklist (service `referral_worklist.rs`, hazard H-044): the referrals this person sent that
// have an answer they have not yet seen. The list carries ids and a state, never the referral's text.
import { siteWhenText } from "./access-log.js";
import { STRINGS } from "./strings.js";

export type WorklistState =
  | "declined"
  | "redirected"
  | "no-show"
  | "deferred"
  | "information-requested"
  | "interim-report"
  | "completed";

export type WorkItem = {
  referral_id: string;
  patient_id: string;
  version: number;
  state: WorklistState;
  recorded_at: string;
};

/** "Declined · 2026-10-21 09:00 UTC": what came back, and when. */
export function itemHeadline(i: WorkItem, siteZone?: string): string {
  return `${STRINGS.referralStates[i.state]} · ${siteWhenText(i.recorded_at, siteZone)}`;
}

/** The body of an acknowledge request: the version the person saw, so a newer answer is never swallowed. */
export function acknowledgeBody(i: WorkItem): { version: number } {
  return { version: i.version };
}
