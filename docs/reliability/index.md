# Reliability: what it means here

Public-source research read 2026-11-04 plus this repository's own files. A proposal and a reading: targets here are not commitments, and nothing was measured by the author of this note unless it says so.

## Definition

ISO/IEC 25010:2023 names reliability as the degree to which a system performs specified functions under specified conditions for a specified period. Per secondary summaries (Sonar and arc42, search snippets and one page; the ISO text itself was not read) its sub-characteristics are maturity or faultlessness, availability, fault tolerance and recoverability ([Sonar](https://www.sonarsource.com/resources/library/iso-iec-25010-explained/), [arc42](https://quality.arc42.org/standards/iso-25010)). The 2011 edition had maturity, availability, fault tolerance and recoverability; the 2023 edition renames maturity as faultlessness and adds safety as a ninth characteristic (same summaries). Bass, Clements and Kazman express availability as scenarios: a stimulus (fault), a response (detect, recover, mask), and a response measure (time to detect, time to repair, percentage available) ([SEI/Addison-Wesley](https://www.sei.cmu.edu/library/software-architecture-in-practice-fourth-edition/), from memory).

For this project, three terms matter. Availability is whether clinicians can reach the record when they need it. Recoverability is whether the record, including the audit chain, can be restored after loss, with a known data-loss window (RPO) and time to restore (RTO). Fault tolerance is whether a failed part (audit table, Redis, one replica, a partner feed) degrades safely instead of silently.

## Why it matters for an EPR

- When an EPR is down, clinicians fall back to paper and memory. The NAO found that WannaCry in May 2017 affected at least 81 of 236 trusts in England and that thousands of appointments were cancelled ([NAO](https://www.nao.org.uk/reports/investigation-wannacry-cyber-attack-and-the-nhs/), from memory, page not re-read this session).
- In August 2022 a ransomware attack on Advanced disrupted NHS 111 and other services for weeks; the ICO fined the processor about 3.07 million pounds in 2025, after provisionally proposing about 6.09 million, and reported that the entry point was a customer account without multi-factor authentication ([Computing](https://computing.co.uk/news/2025/security/ico-fines-advanced), search snippet only).
- Recovery matters as much as prevention: a backup nobody has restored is a hope. The HIPAA Security Rule NPRM of January 2025 proposed restoring critical systems within 72 hours ([Federal Register](https://www.federalregister.gov/documents/2025/01/06/2024-30983/hipaa-security-rule-to-strengthen-the-cyber-security-of-electronic-protected-health-information), from memory; proposed, not final).
- The NCSC Cyber Assessment Framework has an objective on resilience and recovery planning for essential services ([NCSC CAF](https://www.ncsc.gov.uk/collection/cyber-assessment-framework)).

## What the project says today

- NFR-3 (`spec/07-non-functional-requirements.md`): 99.9 per cent availability, stateless app tier, PostgreSQL replication, RPO at most 5 minutes, RTO at most 4 hours, restoration within 72 hours. The spec says plainly that nothing is measured.
- **No backup procedure exists** (`governance/retention-schedule.md`, `governance/production-readiness.md`, `governance/data-flow.md`). AS-7 (backup, restore, recovery, with a CI restore job that verifies the audit hash chain) is unticked in `tasks.md`.
- Built fault handling: a write whose audit row cannot be written is refused, in one transaction (`spec/audit/index.md`; `tests/http_audit_refusal.rs` and `http_audit_refusal_roles.rs` over 18 write routes, using a trigger, not a real storage failure). Version conflicts are answered as conflicts, not corruption (`spec/10-persistence.md`). `idempotency_keys` make a retried create safe for 24 hours (OF-1, hazard H-043 open). A transactional outbox exists but delivers nowhere yet (OS-3).
- Background jobs run on one replica via a lock (PR-4, built; not tried with two real processes).
- Known limits: the audit chain takes one global advisory lock, so audited requests serialise (CQ-18; 166 to 194 audited writes a second in `BENCHMARKS.md`, a first number, not proof). The request limit is per process (OS-2). The UI's stateless tier needs `EPR_SESSION_REDIS_URL` (OS-8, not run behind two replicas). Probes are `/_health/live`, `/_health/ready`, `/_health/startup`.
- No alerts, no on-call, no runbook, no downtime procedure (`governance/production-readiness.md`). Load and chaos tests (AS-6) are not built. A seeded chaos test sends hostile input to every route (no 5xx), which tests robustness to input, not to infrastructure faults.
- Hazard H-039 covers a rate limit refusing a clinician; the hazard log has no accepted or closed entries.

## Measures

| Measure | How to measure | Suggested target or rule for setting one | Where in our data/CI |
| --- | --- | --- | --- |
| Availability | Successful probe or request ratio per month, excluding agreed maintenance | Set by the deployer from clinical need; NFR-3 says 99.9 per cent, unmeasured | `/metrics` histogram (OS-6); no collector yet |
| RPO achieved | Time between the last restorable write and failure in a drill | NFR-3: 5 minutes; confirm with the replication design | None; AS-7 |
| RTO achieved | Wall-clock time of a timed restore drill, to a verified chain | NFR-3: 4 hours; set after the first drill, not before | None; AS-7 restore job |
| Restore verified | Restore run ends with `GET /api/audit/verify` clean | Binary; every drill | Planned in AS-7 acceptance |
| Audit-append failure count | `epr_audit_append_failures_total` | Alert on any non-zero rate | `/metrics` (off by default) |
| Error rate by route | 5xx share, p95 latency under load | Rule: no 5xx under the chaos test | `tests/` chaos test; AS-6 for load |
| Mean time to detect | Alert-to-acknowledge in a game day | Deployer sets; write down the first result | None |
| Duplicate side effects | Retried creates producing one record | Zero | Idempotency tests (OF-1) |
| Downtime-procedure readiness | Paper-fallback drill done and timed | At least once a year (suggestion) | None |

## Trade-offs with other attributes

- Auditability versus availability: refusing a write when its audit row fails is right for the record but turns an audit-table fault into a write outage, and the single chain lock caps throughput (CQ-18). See [auditability](../auditability/index.md).
- Security versus availability: rate limits and lockouts protect but can refuse a clinician (H-039). Break-glass exists so access is not blocked in an emergency ([security](../security/index.md)).
- Append-only retention versus recoverability and cost: nothing is deleted, so backups and restores grow without bound ([privacy](../privacy/index.md)).
- Performance: synchronous replication lowers RPO but adds latency ([performance-efficiency](../performance-efficiency/index.md)). Safety: see [safety](../safety/index.md).

## Gaps and risks

- No backup, restore or recovery procedure, so NFR-3's RPO and RTO are untestable. This is the largest gap.
- No monitoring or alerting is in place; the first signal of a failure is a user.
- Hazard-relevant: an outage with no downtime procedure leaves a clinician without allergy data. This should be a logged hazard if it is not.
- Replica behaviour is untried: per-process limits, jobs and sessions.
- MLLP (HL7 v2) has no authentication or encryption, and a partner feed failing is only partly visible (refusal queue has no owner named).
- Dependency on Keycloak and Redis availability is not stated anywhere.

## Ideas

1. Build AS-7 first: a scripted PostgreSQL base backup plus WAL archive, a CI job that restores into a scratch database and runs the audit verify route, and a runbook. Record the first RTO honestly.
2. Add a "downtime mode" read-only summary export (allergies, current medicines) per EC-11 outage access, with a hazard row.
3. Add a quarterly restore drill checklist to `governance/production-readiness.md` with a named person and a dated record.
4. Ship alert rules for audit-append failures, readiness failing and rate-limit surges alongside `/metrics` (OS-6).
5. Run AS-6 chaos faults against Postgres and Redis (kill, slow, partition) and publish results in `BENCHMARKS.md`.
6. Write service-level objectives as burn-rate alerts rather than a single 99.9 number, after measuring (SRE practice, [Google SRE book](https://sre.google/sre-book/service-level-objectives/), from memory).
7. Test two real replicas for jobs, limits and sessions (PR-4, OS-2, OS-8) and decide on a shared limit store.

## Open questions

- Is 99.9 per cent right for a primary care practice, an emergency department and a small clinic in a low-income setting? The equity rule (`spec/02-scope.md`) suggests a tiered answer.
- Is audit-or-refuse acceptable for reads during an emergency, or does break-glass need an audit-degraded mode, and who would accept that hazard?
- Who owns backups: this project's scripts or the deployer?

## Not verified

- ISO 25010:2023 sub-characteristics are from secondary summaries; the standard was not purchased or read.
- The NAO and Federal Register pages were cited from memory and not re-read; the ICO fine was seen as a search snippet only.
- No availability, RPO, RTO or throughput figure was measured by the author; the 166 to 194 figure is the repository's own.

## Sources

- [Sonar guide to ISO/IEC 25010](https://www.sonarsource.com/resources/library/iso-iec-25010-explained/): sub-characteristic lists, secondary.
- [arc42 quality model summary](https://quality.arc42.org/standards/iso-25010): 2011 versus 2023 changes, secondary.
- [SEI, Software Architecture in Practice](https://www.sei.cmu.edu/library/software-architecture-in-practice-fourth-edition/): quality attribute scenarios.
- [NAO on WannaCry and the NHS](https://www.nao.org.uk/reports/investigation-wannacry-cyber-attack-and-the-nhs/): 2017 impact.
- [Computing on the ICO fine of Advanced](https://computing.co.uk/news/2025/security/ico-fines-advanced): 2022 ransomware, MFA gap.
- [Federal Register HIPAA Security Rule NPRM](https://www.federalregister.gov/documents/2025/01/06/2024-30983/hipaa-security-rule-to-strengthen-the-cyber-security-of-electronic-protected-health-information): proposed 72-hour restoration.
- [NCSC Cyber Assessment Framework](https://www.ncsc.gov.uk/collection/cyber-assessment-framework): resilience objective.
- [Google SRE book, service level objectives](https://sre.google/sre-book/service-level-objectives/): SLO practice.
