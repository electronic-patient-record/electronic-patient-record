// The referrer's worklist (service `referral_worklist.rs`, hazard H-044): the referrals this person sent that
// have an answer they have not yet seen. The list carries ids and a state, never the referral's text.
import { siteWhenText } from "./access-log.js";
import { TEXT } from "./i18n.js";

export type WorklistState =
  | "requested"
  | "accepted"
  | "appointment-booked"
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
  /** reply: acknowledge it. The others are timeouts (PC-3) and cannot be acknowledged. */
  why: "reply" | "no-response" | "no-outcome";
  recorded_at: string;
};

/** "Declined · 2026-10-21 09:00 UTC": what came back, and when. */
export function itemHeadline(i: WorkItem, siteZone?: string): string {
  const what =
    i.why === "reply" ? TEXT.referralStates[i.state] : TEXT.wlWhy[i.why];
  return `${what} · ${siteWhenText(i.recorded_at, siteZone)}`;
}

export type Untimed = { no_response: string[]; no_outcome: string[] };

/** The warning for priorities with no timeout, or null when every priority has both. */
export function untimedWarning(u: Untimed | undefined): string | null {
  if (!u) return null;
  // In the order of urgency, not the order the service listed them.
  const order = ["routine", "urgent", "asap", "stat"];
  const words = order.filter(
    (w) => u.no_response.includes(w) || u.no_outcome.includes(w),
  );
  if (words.length === 0) return null;
  const names = words
    .map(
      (w) =>
        TEXT.referralPriorities[w as keyof typeof TEXT.referralPriorities] ?? w,
    )
    .join(", ");
  return TEXT.wlUntimed.replace("{list}", names);
}

/** The body of an acknowledge request: the version the person saw, so a newer answer is never swallowed. */
export function acknowledgeBody(i: WorkItem): { version: number } {
  return { version: i.version };
}
