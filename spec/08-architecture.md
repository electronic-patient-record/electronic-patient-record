## 8. Architecture

```
UI (SvelteKit + BFF)  ──►  Service (loco.rs / Axum)  ──►  Matcher (pure, crates.io dependency)
                                │   /api/…  native REST
                                │   /fhir/R4/…  FHIR facade (built: src/controllers/fhir.rs); /fhir/R5/… read-only, derived (fhir_r5.rs)
                                │   policy engine (built: src/policy.rs, authz.rs) → audit writer (built; every-access coverage not verified)
                                ├──► PostgreSQL 18 (resource_version, search index tables, audit, outbox)
                                ├──► tantivy (patient typeahead)
                                └──► TerminologyService (Snowstorm / Ontoserver; embedded fixtures in tests)
```

Dependency direction is strictly downward. Events are to leave through a transactional outbox (subscriptions, webhooks,
CDS Hooks, HL7 v2 adapter). Multi-tenancy is to be by `tenant_id` + PostgreSQL row-level security.
**Not built as of 2026-10-30:** the outbox's sinks, loop and producers (OS-3 built only the table, `enqueue` and the delivery runner), tantivy, the terminology service adapters beyond fixtures, `tenant_id`
and row-level security; none is in `Cargo.toml` or a migration. The diagram shows the target; the
[benchmark](benchmark.md) shows what comparable systems run today. Authentication: OIDC upstream,
PASETO v4.public verified offline; SMART scopes map onto the same policy engine (OQ-3). Detail and rationale:
[`../plan.md`](../plan.md) §6. Built (checked against `src/` on 2026-10-30): the UI, the service with the auth guard, the policy engine, consent, break-glass, the audit chain, the FHIR R4 facade and read-only R5 view, the ADT feed, the matcher.

### Outbox (OS-3, first slice)

An event that must leave the service (a subscription notice, a webhook, a CDS Hook, an HL7 v2 message) is first written
to `outbox_events` **in the same transaction as the change and its audit row**. A rollback therefore removes the event;
a commit keeps it. The table is append-only like every record table: triggers refuse `UPDATE`, `DELETE` and `TRUNCATE`.
What happened to an event is a second append-only table, `outbox_attempts` (`delivered`, `refused` or `failed`, with a
reason that names no clinical content). "Pending" means no `delivered` or `refused` attempt and fewer than five `failed`
ones. A failed event waits before its retry: 30 seconds after the first failure, doubling after each (capped at 2^10 times).

An event names its **recipient** (a registered sink), the **roles and purpose of use** that recipient acts under, the
**resource kind**, and the **patient** if any. Before a sink is called the delivery runner makes the read path's own
decision for that recipient: `policy::evaluate` with the recipient's roles, no relationship (a recipient has none), the
patient's consent from `consents_store`, and the purpose. Anything but an allow is a `refused` attempt, is audited, and is
never retried. A failed consent lookup is a `failed` attempt, never a delivery. A disclosure that skipped this check
would be a disclosure the read path refuses.

Delivery is at least once: a crash after the sink answers and before the attempt commits delivers again, so every sink
receives the event id and must treat a repeat as a repeat. Claiming uses `FOR UPDATE SKIP LOCKED`, so two runners never
hold the same event.

**Not built in this slice:** a sink (none exists until IO-6), a loop that calls the runner, a loco
worker (loco's queue needs Redis; the table is the queue here), a queue-depth gauge (the three outcome counters exist), and any producer: nothing calls
`enqueue` yet.

## Audit partitions and archive (OS-13)

`audit_events` is partitioned by month on `recorded_at` (migration 18). The old table was not copied or dropped: it became the first partition (`audit_events_p0`), so rows keep their ids and the hash chain is untouched. The append-only triggers moved to the parent in the same transaction; each partition has its own `TRUNCATE` trigger. `audit_ensure_partitions(n)` makes this month and the next `n` (the migration makes 24; the service calls it at start and every six hours, because an insert with no partition fails and an access that cannot be audited does not happen).

`audit_archive_before(cutoff)` moves the oldest partitions that end at or before `cutoff` into the `audit_archive` schema: it records an anchor first (`first_prev_digest`, `last_digest`, `row_count`, in the append-only `audit_archive_anchors`), then detaches, moves, and puts the row trigger back (detaching drops the inherited one). Nothing is deleted. The chain check and the next append start from the newest anchor, so the chain holds across a partition boundary, across an archive, and when every live partition is empty. The service archives only when `EPR_AUDIT_ARCHIVE_AFTER_MONTHS` (1 to 1200) is set, and only whole months back; unset means never.

`audit_events_all` (migration 19) is the live table and every archived partition as one view, rebuilt in the same transaction as each move; a patient's access log and the audit search read it, so an archived row is still listed. The chain check and the append read `audit_events` and the anchors, never the view.

`audit::verify_archive` checks each archived table against its anchor: the anchors link head to tail, and the table still has the anchor's row count, first and last id and last digest, with its own chain recomputed from `first_prev_digest`. A missing table, a missing or extra row or an edited row is `Broken`. `GET /api/audit/verify` runs it after the live check and adds `archive_rows` (rows read from the archive, 0 when nothing is archived) to an intact answer; the status codes are unchanged. A broken archive is `409` with `first_broken_id` (the anchor's first id, or the row) and a `reason` that names the partition, and it outranks a live finding; a key this service does not hold is the same `424` as for live rows. The check reads the whole archive on every call.

**Not built:** calling `verify_archive` from the maintenance loop (a damaged archive is found only when someone calls the endpoint); a retention period set by governance (OQ-OS7); anything that prevents the database owner dropping a partition. Hazard H-042.

