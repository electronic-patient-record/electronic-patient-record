# Auditability: what it means here

Public-source research read 2026-11-04 plus this repository's own files. A proposal and a reading: targets here are not commitments, and nothing was measured by the author of this note unless it says so.

## Definition

ISO/IEC 25010:2023 has no auditability characteristic. It sits under security: accountability (actions of an entity can be traced uniquely to that entity) and non-repudiation (actions or events can be proven to have taken place so that they cannot be repudiated later), with integrity (no unauthorised modification) as the base (secondary summaries only: [Sonar](https://www.sonarsource.com/resources/library/iso-iec-25010-explained/), [arc42](https://quality.arc42.org/standards/iso-25010); the standard text was not read, and the definitions above are the 2011 wording as I recall it).

Auditability here is the property that someone independent can later answer: who did what, to which record, when, why, from where, and was the answer they were given the true one, and can the record of that be trusted. Relevant public standards: IHE ATNA and the Basic Audit Log Patterns (BALP) profile ([IHE BALP](https://profiles.ihe.net/ITI/BALP/)), the FHIR `AuditEvent` resource ([HL7 FHIR R4](https://hl7.org/fhir/R4/auditevent.html)), NIST SP 800-53 family AU ([NIST](https://csrc.nist.gov/pubs/sp/800/53/r5/upd1/final)), the HIPAA audit controls standard at 45 CFR 164.312(b) ([eCFR](https://www.ecfr.gov/current/title-45/subtitle-A/subchapter-C/part-164/subpart-C/section-164.312)), and OWASP ASVS V7 on logging ([OWASP](https://owasp.org/www-project-application-security-verification-standard/)).

## Why it matters for an EPR

- The record is a legal and clinical document; its readers must know who wrote an entry and whether it was altered.
- Patients and regulators ask who looked. UK GDPR accountability (Article 5(2)) needs a controller to demonstrate compliance ([ICO](https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/accountability-and-governance/)).
- Break-glass and snooping detection depend on reviewed audit data; the control is only as good as the review.
- After ransomware, defenders need trustworthy logs to scope what was touched; attackers with database access may try to alter them. The ICO's account of the 2022 Advanced attack is an example of post-incident scoping ([Computing](https://computing.co.uk/news/2025/security/ico-fines-advanced), snippet only).
- Tamper evidence has public prior art, such as hash-chained logs and signed tree heads in certificate transparency ([RFC 6962](https://www.rfc-editor.org/rfc/rfc6962)); this repository uses a keyed MAC chain with anchors, which is different from a Merkle log.

## What the project says today

- NFR-7: append-only, tamper-evident audit, reads included. Rule from `spec/audit/index.md`: when a write's audit row cannot be written, the write is refused, in the same transaction; a read audits before data leaves, and if that row fails, nothing is disclosed.
- Shape: one FHIR AuditEvent (IHE BALP shape) per access: agent, patient, entity, action C/R/U/D/E, outcome (0/4/8/12), purpose of use, network address, break-glass flag, caller roles and organisations in `detail.actor` (rows before 2026-10-21 have none).
- Built: append-only `audit_events` with triggers refusing UPDATE, DELETE and TRUNCATE (`tests/audit_immutable.rs`, needs Postgres); hash chain with keyed MAC; verifier and `GET /api/audit/verify` (auditor only; 409 on a break; 424 for an unheld key, reported as a key problem, not tampering; a checkpoint option that confirms a row is still there unchanged); `GET /api/audit` and `GET /fhir/R4/AuditEvent` search for the auditor role; CSV or NDJSON export; patient access log (SE-6, partly, staff by role); audit of every search (OF-2, `tests/audit_search_coverage.rs`); audit of route-gate and policy refusals; rate-limit refusals audited once per subject per window.
- Evidence of the refuse-on-failure rule: `tests/http_audit_refusal.rs` and `http_audit_refusal_roles.rs` cover all 18 write routes with a trigger; a census fails when a write route lacks such a test. Not verified there: the response body, the failure counter, a real storage failure.
- Partitioning (OS-13, migration 18): monthly partitions, a chain anchor per partition, an archive step that moves old partitions to `audit_archive` and never deletes (off unless `EPR_AUDIT_ARCHIVE_AFTER_MONTHS` is set; H-042; the period is assumed, OQ-OS7). `audit_events_all` lets searches see archived rows.
- Break-glass (SE-3): 4-hour grant with reason; each access row flags the grant; each grant awaits auditor review (`break_glass_reviews`); an overdue flag exists (EC-11 partly); no real-time notification.
- Cost: one global advisory lock serialises audited requests (CQ-18; 166 to 194 audited writes a second in `BENCHMARKS.md` at 1, 8 and 32 clients, a first number).
- Not built or open: FHIR `AuditEvent` on IHE BALP in full (AU-3, unticked; only a read, registration or update of the patient record claims a profile, PX-4); a scheduled verify run (the path to production: "no schedule"); external anchoring of the chain head; correlation of request id to audit row (a CQ task); hazard statuses none accepted. Audit rows for observation and amendment hold no values by design (a minimisation choice, which limits what an audit can reconstruct).
- The keyed MAC proves tamper evidence only to someone holding `EPR_INTEGRITY_MAC_KEY`; whoever holds it and database write rights could re-forge the chain (my reading; see Gaps).

## Measures

| Measure | How to measure | Suggested target or rule for setting one | Where in our data/CI |
| --- | --- | --- | --- |
| Route audit coverage | Routes (read, write, search, refusal) without an audit test | Zero, enforced by census | Route census, `audit_search_coverage`, `http_audit_refusal*` (built) |
| Audit-append failures | `epr_audit_append_failures_total` | Zero; alert on any | `/metrics` (off by default) |
| Chain verified age | Time since last clean `GET /api/audit/verify` | Set by risk; for example daily (suggestion) | None scheduled |
| Chain break detected in drill | Alter a copy; verify must answer 409 | Always detects | Idea 2; partly in unit tests |
| Audit write latency and throughput | Audited writes a second; p95 | Decide after AS-6 (CQ-18) | `BENCHMARKS.md` |
| Break-glass review lateness | Grants unreviewed past the limit | Zero; limit is a guess today | EC-11 list, `break_glass_reviews` |
| Time to answer "who saw patient X" | Minutes in a drill using search or patient log | Set by the deployer, as a legal duty may apply | `/api/audit`, SE-6 |
| Rows lacking actor | Share of recent rows with no `detail.actor` | Zero outside migration or feed | Audit table query |
| Clock sanity | Skew of audit timestamps from a trusted source | Within seconds | PR-5 clock |
| Restore keeps chain | Restore drill verifies chain | Always | AS-7 acceptance |

## Trade-offs with other attributes

- Auditability versus performance: a global lock and a synchronous row per read cost throughput ([performance-efficiency](../performance-efficiency/index.md)).
- Versus reliability: refusing on audit failure trades availability for integrity ([reliability](../reliability/index.md)).
- Versus privacy: keeping every access for ever grows a personal-data store; minimal content mitigates ([privacy](../privacy/index.md)).
- Versus interoperability and safety: BALP shape helps exchange; see [interoperability](../interoperability/index.md). Audit that forces refusal in an emergency is a hazard ([safety](../safety/index.md)). Versus security: see [security](../security/index.md).

## Gaps and risks

- No scheduled verification, no alert, no external witness of the chain head, so a privileged insider with the MAC key and database rights is not excluded.
- Audit retention period undecided; archive moves rows but nothing is verified after a move beyond the anchors.
- MLLP feed identity rests on a configured agent name and the peer address (`EPR_MLLP_AGENT`), not a proven sender.
- Rows before 2026-10-21 lack actor detail.
- Review of break-glass is a human process; no named reviewer or rota; no real-time notification.
- Reads audited before disclosure add latency and a failure path on every read.
- Time source is the database or process clock; PR-5 (clock) is partial.

## Ideas

1. Schedule `GET /api/audit/verify` daily and alert on 409 or 424, with the result itself recorded (OS-6 follow-up).
2. Add a tamper drill test: restore, alter, verify (AS-7).
3. Publish the chain head periodically to an external witness (a signed file in object storage, or a transparency-log style entry); decide with the maintainer, since it adds an operational dependency.
4. Finish AU-3: extend the BALP claims of PX-4 beyond the patient record, and decide whether a search may hold its query.
5. Decide CQ-18: chain per partition with anchors versus global lock, with measured numbers.
6. Add a break-glass notification to the patient's privacy contact (EC-11), with a hazard row.
7. Add a request-id to audit-row correlation (the CQ task naming it).
8. Add a "who accessed" report template for auditors and a small review sampling rule, for example review every break-glass and a random sample of others (suggestion).

## Open questions

- How long must audit rows be kept (UK, US states, other places), and can old rows be archived off-line?
- Is a keyed MAC enough, or are asymmetric signatures with an offline key needed so the service cannot forge?
- Should patients see staff names, not only roles? SE-6 chose roles; the privacy trade-off is open.
- Who audits the auditor role?

## Not verified

- 25010 definitions are from summaries and memory; wording not confirmed.
- RFC, IHE, FHIR and eCFR pages were cited from memory and not re-read this session.
- I did not run the verifier or any test; all built claims are the repository's own statements.
- I did not review the audit code; claims of the hash design come from `spec/audit/index.md`.

## Sources

- [Sonar](https://www.sonarsource.com/resources/library/iso-iec-25010-explained/), [arc42](https://quality.arc42.org/standards/iso-25010): 25010 summaries.
- [IHE BALP](https://profiles.ihe.net/ITI/BALP/): basic audit log patterns.
- [FHIR R4 AuditEvent](https://hl7.org/fhir/R4/auditevent.html): resource definition.
- [NIST SP 800-53 Rev 5](https://csrc.nist.gov/pubs/sp/800/53/r5/upd1/final): AU control family.
- [45 CFR 164.312](https://www.ecfr.gov/current/title-45/subtitle-A/subchapter-C/part-164/subpart-C/section-164.312): HIPAA technical safeguards, audit controls.
- [OWASP ASVS](https://owasp.org/www-project-application-security-verification-standard/): logging requirements.
- [ICO accountability](https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/accountability-and-governance/): UK accountability principle.
- [RFC 6962](https://www.rfc-editor.org/rfc/rfc6962): transparency log prior art.
- [Computing, ICO fines Advanced](https://computing.co.uk/news/2025/security/ico-fines-advanced): 2022 incident.
