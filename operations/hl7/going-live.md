# Going live with an HL7 feed

Status: **written 2026-11-17 (IH-12). Never used**: no feed has gone live, and no clinician, Clinical Safety Officer or
interface engineer has reviewed this method. The software is not for clinical use ([path to production](../../governance/production-readiness.md)).

This is how a feed moves from the system that serves it today (an interface engine, or another record) to this
service. It is adapted from an HL7 v2 integration hub's cutover runbook and its register of deliberate differences
([plan.md §36](../../plan.md#36-research-pass-2026-11-16-an-hl7-v2-integration-hub-rewritten-in-rust)). The rule
underneath every step: **going back must stay a switch, never a repair.**

## Before: prove it on copies

1. **Name the feed.** One sender, one `EPR_MLLP_AGENT`, one listener. Agree with the sender's owner how it treats each
   answer: `AA` done, `AR` do not resend (kept for a person here), `AE` resend later
   ([what a message does](README.md#what-a-message-does), OQ-IH1). A sender that cannot treat `AR` that way is set
   `EPR_MLLP_KEPT_ANSWER=AE` and written down here.
2. **Collect a sample.** Messages the sender really sends (at least a week's, every event it sends, with corrections,
   deaths and messages that are refused today), copied with the data owner's agreement to a test deployment only. Never
   to a laptop, a ticket or a chat.
3. **Replay and compare.** Send the sample, in its original order, to a test deployment of this service. For every
   message, compare what this service did (applied, unchanged, kept, and why) with what the current system did. Every
   difference is either a defect to fix first, or a row in the register below.
4. **Prove the safety rules on the sample:** a late update is kept, not applied (IH-5, H-058); a possible duplicate is
   kept, not created; no field is blanked; no birth date or national number changes; the control id appears only in the
   stored messages (IH-3); refusals show in `/api/feed/health` (IH-8).
5. **Check the settings** on the live deployment: TLS on (`EPR_MLLP_TLS_*`, IH-10), the client CA if the sender can
   present a certificate, `EPR_MLLP_ALLOW_REMOTE` only for its address, the health thresholds set by the deployer
   (`EPR_FEED_*`, OQ-IH4) and an alert on the "HL7 feed alarm" log line or `epr_feed_health`.

## The register of differences

Every intended difference from the current system is a row. A feed goes live only when **every row that touches it is
signed**. The decision, who and when are left blank until a named person fills them: never pre-filled, never by the
person who wrote the row.

| # | What the current system does | What this service does | Messages affected | Why | Decision | Who | When |
|---|---|---|---|---|---|---|---|
| 1 | | | | | | | |

Defects found in the current system are not rows here: they go to its owner.

## The switch

1. **One feed at a time**, lowest risk first.
2. **Point the sender here**, and keep the old route configured but idle. **Never delete it**: going back is pointing
   the sender back.
3. **Hold for one full business day** before the next feed, with someone watching.

## What to watch, and when to go back

| Watch | Healthy | Go back when |
|---|---|---|
| `epr_feed_messages_total{answer="AA"}` | Rising in step with the sender's own count | It stops while the sender is sending |
| `answer="AR"` and the work queue | Only what the register expects | Any refusal the register does not explain |
| `answer="AE"` | Zero, or a brief blip | Rising: the database or the queue is failing |
| `epr_feed_health` / `/api/feed/health` | `healthy` | `critical` for longer than the window |
| The sender's own queue | Draining | Growing |

Any new kind of refusal is a stop signal. **Go back first, then find out why**: do not fix forward while patients' details
are arriving. Going back is a switch, and the messages this service kept are still in its work queue for a person.

## After

Record in this file, per feed: the date it went live, the sample's size and dates, the register rows and who signed
them, and anything the hold found. A feed that went back is recorded too, with why.

| Feed | Live on | Sample | Rows signed | Held by | Notes |
|---|---|---|---|---|---|
| | | | | | |
