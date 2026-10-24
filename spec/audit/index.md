# Audit

One FHIR AuditEvent (IHE BALP shape) per access, reads included: agent, patient, entity, action C/R/U/D/E, outcome, purpose of use, network address, break-glass flag. Append-only and hash-chained with a keyed MAC. Patients can see who accessed their record. Tasks AU-1, AU-2, SE-6. **Built today:** the append-only table, the hash-chained writer, a chain verifier, and `GET /api/audit/verify` (auditor only; 409 when the chain is broken; each check is itself audited). Every patient and consent route writes a row, and so does each refusal by the route gate or the policy engine. an unheld key reported as a key problem (`424`, `tampering_found:false`, `causes`), not as tampering (`409`); a checkpoint: each answer carries `head` (id and digest of the newest row checked), and a later call with `?checkpoint_id=&checkpoint_digest=` confirms that row is still there unchanged (409 if removed or rewritten). the patient-visible log, `GET /api/me/patients/{id}/access-log` (SE-6, partly: staff shown by role and organisation, never by name). Every row written while handling a request records the caller’s roles and organisations in `detail.actor`, set by the route-gate middleware through a task-local, so no call site can forget it; rows from before 2026-10-21 and rows written outside a request have none. `GET /api/audit` and `GET /fhir/R4/AuditEvent` search the trail for the auditor role (each search is itself audited with the criteria names only). `GET /api/audit/export` (CSV or NDJSON with the digests, up to 10,000 rows per file, itself audited). **Not built:** publishing the head automatically (the operator must store it where the database owner cannot write), so removal of the newest rows is detected only against a checkpoint the operator kept; audit of 401/400/422 responses, BALP/FHIR `AuditEvent` output, the patient-visible log.

Planned; nothing here is built yet unless stated. Source: [`../../plan.md`](../../plan.md).

## The audit row is part of the write

**Rule.** When a write's audit row cannot be written, the write is refused: nothing is stored, and the caller gets a
failure (HTTP 500), not a success. A change to the record that leaves no trace of who made it is worse than a change
that did not happen, because the record is append-only and cannot be taken back.

- The write and its audit row are one database transaction. The row is appended inside it, and the commit makes both
  durable or neither.
- This holds for every write: patients, consents, relationships, allergies and the "no known allergies" statement,
  photos, merge and unmerge, break-the-glass grants, resolving a feed refusal, and a record registered by the ADT feed.
  A new write route is not complete until it joins this rule.
- A **read** audits in its own transaction before the data leaves. If that row cannot be written, the answer is a
  failure and nothing is disclosed.
- A refusal (a 4xx) is audited too, where it discloses something. If its row cannot be written, the refusal still
  stands, because refusing is the safe outcome.
- The failure is counted (`epr_audit_append_failures_total`) and logged, so a failing audit table is seen before
  users are.

**Evidence today.** The store-level test `a_write_and_its_audit_row_commit_or_roll_back_together` proves it for patient
create and update. `tests/http_audit_refusal.rs` drives 14 of the 18 write routes through the router with a trigger
that makes the audit insert fail: each answers 500 and leaves no row in any store, and the same request then succeeds
once audit works. A census fails when a write route is added without such a test or an entry in its list of gaps.
**Not yet verified** (CQ-39): FHIR update, the two break-the-glass writes, and resolving a feed refusal; for those
the rule is a requirement, not a verified property.
