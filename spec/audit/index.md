# Audit

One FHIR AuditEvent per access, modelled on IHE BALP, reads included, and claiming a BALP profile only when the row meets it (see below): agent, patient, entity, action C/R/U/D/E, outcome, purpose of use, network address, break-glass flag. Append-only and hash-chained with a keyed MAC. Patients can see who accessed their record. Tasks AU-1, AU-2, SE-6. **Built today:** the append-only table, the hash-chained writer, a chain verifier, and `GET /api/audit/verify` (auditor only; 409 when the chain is broken; each check is itself audited). Every patient and consent route writes a row, and so does each refusal by the route gate or the policy engine. an unheld key reported as a key problem (`424`, `tampering_found:false`, `causes`), not as tampering (`409`); a checkpoint: each answer carries `head` (id and digest of the newest row checked), and a later call with `?checkpoint_id=&checkpoint_digest=` confirms that row is still there unchanged (409 if removed or rewritten). the patient-visible log, `GET /api/me/patients/{id}/access-log` (SE-6, partly: staff shown by role and organisation, never by name). Every row written while handling a request records the caller’s roles and organisations in `detail.actor`, set by the route-gate middleware through a task-local, so no call site can forget it; rows from before 2026-10-21 and rows written outside a request have none. `GET /api/audit` and `GET /fhir/R4/AuditEvent` search the trail for the auditor role (each search is itself audited with the criteria names only). `GET /api/audit/export` (CSV or NDJSON with the digests, up to 10,000 rows per file, itself audited). **Not built:** publishing the head automatically (the operator must store it where the database owner cannot write), so removal of the newest rows is detected only against a checkpoint the operator kept; audit of 401/400/422 responses, and a BALP profile for any row but a read, registration or update of the patient record itself.

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
create and update. `tests/http_audit_refusal.rs` (login off) and `http_audit_refusal_roles.rs`
(login on) drive all 18 write routes through the router with a trigger that makes the audit insert fail: each answers
500 and leaves no row in any store, and the same request then succeeds once audit works. A census fails when a write
route is added without such a test. **Not verified:** that the response body is useful, that the failure counter moves,
and a real storage failure (the test uses a trigger).

## IHE BALP

An `AuditEvent` claims `IHE.BasicAudit.PatientRead`, `PatientCreate` or `PatientUpdate` in `meta.profile` only when `fhir_audit::violations` finds nothing in its row (task PX-4). A row that cannot meet the profile is still exported, in the shape it always had, without the claim. Read 2026-11-07 from the `ihe.iti.balp` 1.1.4 package itself, and checked by the HL7 validator with that package loaded (`tests/fhir_validator.rs`), which also refuses a claim with its server agent removed.

| The profile requires | Held in the row | A row without it |
|---|---|---|
| An action with a RESTful profile | `action` is `R`, `C` or `U`. `D` is never used and `E` has no profile | `NoProfileForAction` |
| For a search, `entity:query.query`, the query itself | Never: the trail keeps criteria names only, on purpose, because a query names people. **So no search claims `Query` or `PatientQuery`, and none will unless that is changed** | `QueryNotHeld` |
| `subtype` from `restful-interaction`, matching the action | `detail.interaction`, set from the route table and the method together | `NoInteraction` |
| `agent:client` with `who` and `network` | `network_address`, an IP address, which is also the client's `who`, as no client id is held | `NoClientAddress` |
| `agent:server` with `who` and `network` | `detail.server`, from `EPR_AUDIT_SERVER_ADDRESS`. Unset, nothing claims | `NoServer` |
| `entity:patient`, with role 1 | `patient`, as `Patient/<id>` | `NoPatient` |
| `entity:data`, with `what` and a role | `resource` is `Patient`, so the data is the patient record and role 4 applies. A row about another record names its type and not its id | `NotThePatientRecord` |

The interaction, address and server are written with the row, inside what the digest covers, so a claim is evidence of that moment and not of today's settings. A row written before 2026-11-07, or outside a request (the HL7 feed, a job), holds none of them and claims nothing.

In a read the data flows to the client, so the client is `110152` (destination) and the user `IRCP`; in a write the client is `110153` (source) and the user `AUT`. The user is a third agent and the only `requestor`. A local `subtype` and the local purpose-of-use system stay, which the validator reports as warnings and not as errors.
