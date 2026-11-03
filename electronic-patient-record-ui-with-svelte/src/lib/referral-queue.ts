// The work queue for inbound referral messages that matched no referral (service `referral_unmatched.rs`,
// PC-2, hazard H-044): the API's entries as text. The list never carries the message; one read does.
import { siteWhenText } from "./access-log.js";
import { STRINGS } from "./strings.js";

export type Cleared = {
  cleared_by: string;
  cleared_at: string;
  how: "matched" | "dismissed";
  referral_id: string | null;
  reason: string;
};

export type QueueEntry = {
  id: string;
  received_at: string;
  agent: string;
  kind: "no-id" | "unknown-id" | "refused-by-table";
  referral_ref: string | null;
  event: string | null;
  reasons: string[];
  note?: string | null;
  message?: string | null;
  cleared: Cleared | null;
};

export const HOWS = ["dismissed", "matched"] as const;
export type How = (typeof HOWS)[number];

/** "outcome · reference R-1 · received 2026-10-21 09:00 UTC": which message, from the sender's own words. */
export function queueHeadline(e: QueueEntry, siteZone?: string): string {
  const ref = e.referral_ref
    ? `${STRINGS.rqReference} ${e.referral_ref}`
    : STRINGS.rqNoReference;
  return `${e.event ?? STRINGS.rqNoEvent} · ${ref} · ${STRINGS.rqReceived} ${siteWhenText(e.received_at, siteZone)}`;
}

/**
 * The body of a clear request; null while it would be refused: no reason, or a hand match with no referral id.
 * A dismissal sends no referral id.
 */
export function clearBody(
  how: How,
  referralId: string,
  reason: string,
): { how: How; reason: string; referral_id?: string } | null {
  const why = reason.trim();
  const id = referralId.trim();
  if (!why) return null;
  if (how === "matched")
    return id ? { how, reason: why, referral_id: id } : null;
  return { how, reason: why };
}
