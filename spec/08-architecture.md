## 8. Architecture

```
UI (SvelteKit + BFF)  ──►  Service (Axum, `axa`)  ──►  Matcher (pure, crates.io dependency)
                                │   /api/…  native REST
                                │   /fhir/R4/…  FHIR facade (built: src/controllers/fhir.rs); /fhir/R5/… read-only, derived (fhir_r5.rs)
                                │   policy engine (built: src/policy.rs, authz.rs) → audit writer (built; every-access coverage not verified)
                                ├──► PostgreSQL 18 (resource_version, search index tables, audit, outbox)
                                ├──► tantivy (patient typeahead)
                                └──► TerminologyService (Snowstorm / Ontoserver; embedded fixtures in tests)
```

Dependency direction is strictly downward. Events are to leave through a transactional outbox (subscriptions, webhooks,
CDS Hooks, HL7 v2 adapter). Multi-tenancy is to be by `tenant_id` + PostgreSQL row-level security.
**Not built as of 2026-11-08:** the outbox's sinks and producers (OS-3 built `enqueue`, the decision and the relay, which `axa-outbox` runs), tantivy, the terminology service adapters beyond fixtures, `tenant_id`
and row-level security; none is in `Cargo.toml` or a migration. The diagram shows the target; the
[benchmark](benchmark.md) shows what comparable systems run today. Authentication: OIDC upstream,
PASETO v4.public verified offline; SMART scopes map onto the same policy engine (OQ-3). Detail and rationale:
[`../plan.md`](../plan.md) §6. Built (checked against `src/` on 2026-10-30): the UI, the service with the auth guard, the policy engine, consent, break-glass, the audit chain, the FHIR R4 facade and read-only R5 view, the ADT feed, the matcher.

### Outbox (OS-3, first slice; run by `axa-outbox` 0.2 since 2026-11-08)

An event that must leave the service (a subscription notice, a webhook, a CDS Hook, an HL7 v2 message) is first written
with `outbox::enqueue` **in the same transaction as the change and its audit row**; it takes a transaction, not a pool.
A rollback therefore removes the event; a commit keeps it. `axa-outbox` keeps it as a record (migration 28):
`axa_outbox_events`, `axa_outbox_payloads` and `axa_outbox_attempts` are only ever added to, under triggers that refuse
`UPDATE`, `DELETE` and `TRUNCATE` from any role and fire even under `session_replication_role = replica`. The one change
the triggers allow is blanking a payload after an `erased` attempt has recorded who erased it and why; nothing here
calls it yet. An event's state is its latest attempt. `axa_outbox_owed` and `axa_outbox_leases` are working state that
the database derives from the record or that a live relay reports into. A payload over 64 KiB is refused, and the
caller's transaction with it. The service does not start a relay unless every table, index and trigger is there.

An event names its **recipient** (the topic: a registered sink), the **roles and purpose of use** that recipient acts
under, the **resource kind**, and the **patient** if any (the key, so one patient's events go in order and a hold on
one patient holds back no other). The roles, purpose, resource and patient are stored in the payload beside what is
sent, so the fingerprint `axa-outbox` keeps covers them: an event whose stored payload no longer matches is `failed` as
`tampered`, is never published, and is an audit row.

Before a sink is called the relay makes the read path's own decision for that recipient, in the claim's transaction:
first, any standing report that the patient has died **holds** the event (no attempt, decided again later; BV-3); then
`policy::evaluate` with the recipient's roles, no relationship (a recipient has none), the patient's consent from
`consents_store`, and the purpose. Anything but a plain allow is a `refused` attempt, is audited, and is final: nothing
puts it back. A failed death or consent lookup decides nothing: the event waits and is decided again, and is never
delivered unchecked. A disclosure that skipped this check would be a disclosure the read path refuses.

A sink that fails is tried 5 times in all, 30 seconds after the first failure and twice as long after each; the event is
then `failed` and waits for a person, who can put it back with `axa::outbox::release`, naming who and why. An event for
a recipient with no sink fails the same way and is not lost. A delivery, a refusal and a changed event are each an audit
row naming the event (`outbox/{id}`), never its content, written in the outcome's own transaction, so the outcome and
its audit row commit together or not at all.

Delivery is at least once: a crash after the sink answers and before the attempt commits delivers again, so every sink
receives the event id and must treat a repeat as a repeat. Claiming uses `FOR UPDATE SKIP LOCKED`, and a claim names its
relay, so two relays never hold the same event and a relay given up on cannot write over the one that took over.

On `SIGTERM` or Ctrl-C the service stops serving first, then tells the relay to stop and waits for it: a publish in
progress finishes, within `axa-outbox`'s publish timeout (30 seconds), and the rest of the batch is handed back unsent
and with no attempt charged. A relay that has not stopped 35 seconds after it was told is abandoned, is logged as an
error, and its claim is recovered by the next relay after five minutes, as after a crash.

The tables of migration 17, `outbox_events` and `outbox_attempts`, are kept with their rows and triggers, and refuse new
rows: they are a record of what was disclosed and refused, and nothing removes one. Nothing enqueued before migration 28,
so they are expected to be empty; a row in them stays where it is and is not moved.

**Not built in this slice:** a sink (none exists until IO-6), a producer (nothing calls `enqueue` yet), erasure of a
payload (`axa::outbox::erase` is not called), sealing payloads (the `outbox-encryption` feature is off), a deny list of
field names, and a route for a person to release a failed event.

## Audit partitions and archive (OS-13)

`audit_events` is partitioned by month on `recorded_at` (migration 18). The old table was not copied or dropped: it became the first partition (`audit_events_p0`), so rows keep their ids and the hash chain is untouched. The append-only triggers moved to the parent in the same transaction; each partition has its own `TRUNCATE` trigger. `audit_ensure_partitions(n)` makes this month and the next `n` (the migration makes 24; the service calls it at start and every six hours, because an insert with no partition fails and an access that cannot be audited does not happen).

`audit_archive_before(cutoff)` moves the oldest partitions that end at or before `cutoff` into the `audit_archive` schema: it records an anchor first (`first_prev_digest`, `last_digest`, `row_count`, in the append-only `audit_archive_anchors`), then detaches, moves, and puts the row trigger back (detaching drops the inherited one). Nothing is deleted. Migration 20 renamed that function `audit_archive_move` and wraps it: `audit_archive_before` calls it and then `audit_ensure_partitions(24)` in the same transaction, so archiving this month's partition leaves a new one and an access can still be audited at once. The chain check and the next append start from the newest anchor, so the chain holds across a partition boundary, across an archive, and when every live partition is empty. The service archives only when `EPR_AUDIT_ARCHIVE_AFTER_MONTHS` (1 to 1200) is set, and only whole months back; unset means never.

`audit_events_all` (migration 19) is the live table and every archived partition as one view, rebuilt in the same transaction as each move; a patient's access log and the audit search read it, so an archived row is still listed. The chain check and the append read `audit_events` and the anchors, never the view.

`audit::verify_archive` checks each archived table against its anchor: the anchors link head to tail, and the table still has the anchor's row count, first and last id and last digest, with its own chain recomputed from `first_prev_digest`. A missing table, a missing or extra row or an edited row is `Broken`. `GET /api/audit/verify` runs it after the live check and adds `archive_rows` (rows read from the archive, 0 when nothing is archived) to an intact answer; the status codes are unchanged. A broken archive is `409` with `first_broken_id` (the anchor's first id, or the row) and a `reason` that names the partition, and it outranks a live finding; a key this service does not hold is the same `424` as for live rows. The check reads the whole archive on every call.

**Not built:** calling `verify_archive` from the maintenance loop (a damaged archive is found only when someone calls the endpoint); a retention period set by governance (OQ-OS7); anything that prevents the database owner dropping a partition. Hazard H-042.

