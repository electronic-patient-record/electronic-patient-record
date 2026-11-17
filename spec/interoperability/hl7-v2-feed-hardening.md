# HL7 v2 feed hardening (IH)

Status: **research 2026-11-16, specs only. Nothing on this page is built** unless it says so. Tasks IH-1 to IH-12 in
[`tasks.md`](../../tasks.md); ranked in [`plan.md` §36](../../plan.md#36-research-pass-2026-11-16-an-hl7-v2-integration-hub-rewritten-in-rust);
open questions OQ-IH1 to OQ-IH6 in [`16-open-questions.md`](../16-open-questions.md).

The source is a Rust rewrite of an HL7 v2 integration hub (`integration-hub-beta-riir`, read locally at commit `77de740`,
2026-11-16): ingress servers for MLLP, SOAP and REST, a message bus, per-feed validation and transformers, senders, a
message store, replay, an operator dashboard, telemetry with redaction, fuzzing and a local gate. It is a message router
and keeps no clinical record, so its domain rules do not carry over. Its mechanisms do: how it acknowledges, what it never
logs, how it retries, how it tests and how it cuts over. Citations below are to that repository's own files. Its
statements were read, not run.

## Where this project stands (2026-11-16)

Built (service spec, row "HL7 ADT feed (MLLP)"; task OR-3):

- An MLLP listener, off unless configured, loopback unless allowed, at most 1 MiB a frame, 8 connections, 60 s idle.
- ADT A01/A04/A08/A28/A31 matched on identifiers; the write and its audit row in one transaction, **before** the
  acknowledgement. `AA` applied, `AE` refused for content (the reason in `MSA-3`) or a temporary failure, `AR` a type not
  handled.
- A refused message kept in `feed_refusals` until an administrator closes it with an outcome; a screen at `/admin/feed`.
- Fuzz tests on the parser and the framing; an unlabelled counter of refused messages.

Two things here are already stronger than the hub, and should stay so: the change is committed before `AA` (the hub's
store write is non-blocking, `spec/07-hl7-server.md` §5, so it can acknowledge with no durable copy), and the listener has
a connection cap and a loopback default (none found in the hub's ingress spec or configuration).

Found while reading for this pass:

- **A refused message is answered `AE`, and each refusal adds a row.** `feed_queue::record` inserts with no check on the
  sender or control id. The hub's rule, and a common sender setting, is that `AE` means "send it again" and `AR` means
  "do not" (`spec/07-hl7-server.md` §4.3; `spec/10-senders.md` §3.7.1). A sender that retries `AE` adds a queue row per
  retry, and one that keeps strict order holds every later message behind the refused one. Not observed with a real
  sender: inferred from the code and the hub's rule.
- **The control id (MSH-10) is kept in plain text** in the audit row, in `feed_refusals.control_id` and in the info log
  line "ADT message applied". The hub found a sender that builds MSH-10 from a timestamp and the national patient
  number, so an allowlist that keeps MSH-10 leaked it (`spec/04-observability.md` §2.13).
- **A stale comment.** `src/adt_feed.rs` says the work queue for refused messages is not built; it is.
- **An older message can overwrite a newer one.** A resent or delayed A08 is applied as it arrives; nothing compares its
  message time with the last one applied for that patient from that sender. The hub keeps order with bus sessions
  (`spec/01-architecture.md` §6.2), which this service does not have.

## Rules proposed

Each is a design to be confirmed, not a built behaviour. Clinical ones add hazard rows with their first code.

1. **A refused message the service has kept is not sent again (IH-1, clinical). Built 2026-11-17**, with one setting per listener (`EPR_MLLP_KEPT_ANSWER`), not yet per sender. Once a refusal is stored in the work
   queue, the answer tells the sender not to resend it: `AR` with `MSA-3` saying it was kept for review, or a per-sender
   setting (OQ-IH1). `AE` is kept for what a resend can fix: the database unavailable, the refusal not stored. A resend
   of a message already in the queue (same sender, same MSH-10, same text digest) adds no row and gets the same answer.
2. **Acknowledgement hygiene (IH-2). Built 2026-11-17.** MSH-11 (processing id) is copied from the message into the ACK, so a test message
   is never acknowledged as production (hub `spec/07-hl7-server.md` §4.1); a message whose MSH-11 does not match the
   deployment's (`P` on a test system, `T` on a live one) is refused `AR` and kept. `MSA-3` is one line, escaped for HL7
   delimiters, and capped (the hub uses 199 characters; OQ-IH5). A frame that is HL7-shaped but does not parse gets an
   `AE` that echoes MSH-10 when it can be read, so the sender can match it, rather than a closed connection.
3. **MSH-10 is not kept in plain text (IH-3, privacy). Built 2026-11-17**, in the existing `control_id` column rather than a new one. The audit row, the queue's `control_id` column and every log line
   carry a keyed digest of the sender and MSH-10, which still joins a resend to its original; the plain value is in the
   stored message text only, which the queue already holds under its access rule (OQ-IH2).
4. **No message value in any log, error, metric or `MSA-3` (IH-4). Built 2026-11-17**; the marker test found the dictionary's findings and two warnings quoting values. A marker test pushes a distinctive value through
   every field and every refusal path and asserts it appears nowhere but the stored message (hub `spec/06-validation.md`
   §3.3.3, `spec/17-testing-and-quality.md` §6). Errors carry a structural path and a reason, never a value. Should a frame
   ever be logged, it goes through a deny-by-default ER7 redactor: segment ids and MSH-2 to MSH-7 and MSH-9 to MSH-12
   kept (MSH-10 only by rule 3), every other populated field masked, an empty field left empty, anything that is not ER7
   masked whole, with the separator read as the fourth character (a fuzz-found panic in the hub,
   `spec/04-observability.md` §2.5a).
5. **A late message does not undo a newer one (IH-5, clinical). Built 2026-11-17** (H-058), kept as kind `conflict` with the out-of-order reason. For each patient and sender, the message time (MSH-7,
   or EVN-2 when present) of the last applied update is kept. An update older than it is not applied: it is kept in the
   work queue as `out-of-order` and answered as rule 1. A message with no usable time is applied as now, with a warning
   in the audit row (OQ-IH3).
6. **Per-sender rules (IH-6).** Each configured sender names its sending application and facility (MSH-3, MSH-4), the
   HL7 version it sends (MSH-12) and the trigger events it may send; anything else is `AR` and kept (hub
   `spec/07-hl7-server.md` §3.3, §7). The audit agent stays configured, not taken from the message.
7. **Shutdown drains (IH-7).** On a stop signal the listener stops accepting, lets each message already read finish
   (commit and answer), then closes the pool, within a bound; a message read but not committed gets no `AA`. The hub
   recorded the opposite order as a defect of the system it replaces (`plan.md` §9.9 there).
8. **Feed health is never "healthy" without data (IH-8). Built 2026-11-17 in the service**; the UI part and a notifier are not. Per sender: an alarm for no message in a window, with windows
   for day, evening and weekend in the site's time zone; an alarm for refusals in a window; any open refusal is shown
   outside the admin pages. No data is `unknown`, never `healthy`. Alarm state is stored so a restart does not raise it
   again (hub `spec/15-dashboard.md` §3, §5). Thresholds are the deployer's (OQ-IH4). This is OR-3's missing alert.
9. **Counters by sender (IH-9).** `epr_feed_messages_total` and `epr_feed_refused_total` with the labels `sender` (the
   configured name), `event` and `answer` (`AA`, `AE`, `AR`) only: never a control id or a patient (the hub's refusal
   counters carry the correlation and message ids, a cardinality and privacy fault, `spec/04-observability.md` §3).
   Names, once published, are not renamed, since alerts depend on them.
10. **TLS on MLLP (IH-10). Built 2026-11-17**, with optional client certificates. rustls only. A missing, empty or unusable CA bundle, certificate or key stops the service
    starting; TLS asked for is never downgraded to plain text (hub `spec/18-deployment.md` §4, `spec/12-message-store.md`
    §5.8.2). Already listed as not built in OR-3.
11. **Tools for integrators (IH-11).** A dry run: a message through the real path in a transaction that is rolled back,
    answering what would have happened, audited as a dry run, for an administrator only (hub
    `spec/integration-hub-tester-component-preview.md`). A test sender for the MLLP port that sends a file of messages
    with varying fields (hub `spec/ultra7-test-message-sender.md`). Golden acknowledgements for the sample messages,
    byte-for-byte, never re-captured to make a test pass (hub `spec/17-testing-and-quality.md` §4.4).
12. **Going live with a feed (IH-12, documentation). Written 2026-11-17:** [`operations/hl7/going-live.md`](../../operations/hl7/going-live.md), never used. Before a feed moves from an existing interface engine: its messages
    replayed through a test deployment and the outcomes compared; one feed at a time; held for one full business day;
    the old route kept, not deleted, so going back is a switch; queue depth and refusals watched, and any new refusal a
    stop signal. Every intended difference from the old route is a row in a register with the decision, who and when
    left blank until a named person signs (hub `docs/cutover-runbook.md`, `docs/deviation-sign-off.md`).

## When the outbox sends (later, with IO-6 and HL7 v2 out)

The hub's sender rules fit the outbox relay's sinks once one sends anywhere (today only `log` exists):
complete only on a positive answer; a permanent refusal (`AR`, an HTTP 4xx other than 401 and 429) is final, described
by ids only, never the payload; a temporary one (`AE`, 5xx, a timeout) backs off; 401 refreshes the token once; 429
honours `Retry-After` up to a cap (hub `spec/10-senders.md` §3.7.1, §8.2). An HTTP sink sends `Idempotency-Key`, which
the hub's target lacked, so its retries could resubmit. Recorded under IH-9's task as a note for OS-3, not a task of its
own.

## Not adopted

- **The hub's message bus, store-and-replay tables and bus inspector.** This service's durable copy is the record and
  the work queue; a bus inspector that shows bodies unredacted is for development only even in the hub
  (`spec/14-buswatch.md`).
- **A strict dictionary per sender** that refuses unknown and `Z` segments (hub `spec/06-validation.md` §3.3.2). This
  service reads leniently on purpose and refuses on what it applies; a per-sender strict mode can be a later option.
- **Acknowledging before a durable write.** Rule 1 depends on the opposite.

## Not verified

- Whether a real sender retries `AE` and blocks: inferred, not observed.
- What `hl7::v2::builder::acknowledge` copies today (MSH-11 in particular) and how `set("MSA-3", …)` escapes: not read.
- Whether the service's shutdown already drains the listener: not tested.
- The hub's statements are its own documents, read at one commit; none was run here.
