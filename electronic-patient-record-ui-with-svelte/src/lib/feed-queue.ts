// The work queue for refused HL7 messages (service `feed_queue.rs`): the API's entries as text.
import { siteWhenText } from "./access-log.js";
import { STRINGS } from "./strings.js";

export type Resolution = {
  resolved_by: string;
  resolved_at: string;
  outcome: "handled" | "dismissed";
  reason: string;
};

export type Refusal = {
  id: string;
  received_at: string;
  agent: string;
  peer: string | null;
  event: string;
  control_id: string;
  kind: "unsupported" | "conflict";
  reasons: string[];
  message?: string | null;
  resolution: Resolution | null;
};

export const OUTCOMES = ["handled", "dismissed"] as const;
export type Outcome = (typeof OUTCOMES)[number];

/** "A08 · message M-1 · received 2026-10-21 09:00 UTC": which message, from the sender's own ids. */
export function headline(r: Refusal, siteZone?: string): string {
  return `${r.event} · ${STRINGS.feedControl} ${r.control_id} · ${STRINGS.feedReceived} ${siteWhenText(r.received_at, siteZone)}`;
}

/** The message as a person reads it: HL7 separates segments with a carriage return, shown as lines. */
export function readable(message: string): string {
  return message.replace(/\r\n|\r/g, "\n").trimEnd();
}

/** The body of a resolve request; null while the reason is blank, which the API would refuse. */
export function resolveBody(
  outcome: Outcome,
  reason: string,
): { outcome: Outcome; reason: string } | null {
  const text = reason.trim();
  return text ? { outcome, reason: text } : null;
}
