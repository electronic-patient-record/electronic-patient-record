# Data flow: who sends what to whom (PR-7, DRAFT)

**NOT FOR CLINICAL USE.** Read from `src/routes.rs`, `src/mllp.rs`, `src/outbox.rs`, `compose.yaml` and the Keycloak notes on 2026-11-04. It is a map of what the code does, not a claim that any deployment is configured this way. It has not been reviewed by a data protection officer or checked against a running system. See the [DPIA](dpia.md) and the [record of processing](record-of-processing.md) for the controller's side.

## What comes in

| From | How | What | Authenticated? | Where it lands |
|---|---|---|---|---|
| A clinician, nurse, clinical admin, integrator, patient or proxy (through the UI or a client) | REST `/api/...` and FHIR `/fhir/R4`, `/fhir/R5` | Identity, consent, relationships, allergies, observations, amendment requests, referrals, break-glass grants | Yes: a PASETO v4 token or a Keycloak access token; every route has a policy entry (`routes::TABLE`) | PostgreSQL, append-only tables |
| The UI server (backend-for-frontend) | REST, with the user's token | The same calls on the user's behalf | Yes | The same |
| A referral recipient's system | `POST /api/referral-messages` | An outcome, decline or interim note for a referral | Yes (integrator role) | `referral_versions`, or the unmatched queue when it cannot be matched |
| A hospital administration system | HL7 v2 ADT over MLLP, only when `EPR_MLLP_BIND` is set | Patient demographics (A01, A04, A08, A28, A31) | **No.** MLLP has no authentication or encryption; the audit agent is `EPR_MLLP_AGENT` and the peer address is recorded | The patient identity record; refusals go to a review queue |
| An operator | Environment variables | Keys, periods, switches (`EPR_*`) | The host | Process memory |

## What goes out

| To | How | What | Notes |
|---|---|---|---|
| A caller | The responses above | Whatever the policy lets that role see; `404` rather than `403` where existence would leak | Break-glass access is audited and reviewed |
| The audit trail | `audit_events`, a hash chain, append-only | Who did what to which record, with the outcome and the client's IP address, a personal datum (shown to an auditor, never in the patient's access log) | A write whose audit row cannot be written is refused (`tests/http_audit_refusal.rs`) |
| A scraper | `/metrics`, only when `EPR_METRICS` is `1` or `true` | Counters; needs no token | Off by default; **when on it is public to anyone who can reach the port** |
| A probe | `/_health/live`, `/_health/ready`, `/_health/startup`, `/_dependencies` | Liveness, and which dependency is down, as labels | Public |
| Other systems | The transactional outbox (`axa_outbox_events`, `axa_outbox_payloads`) | Nothing yet: `outbox::enqueue` has no caller outside its own module, and the only sink is a log line | Delivery to a real recipient is not built |
| The log | `tracing` | Request outcomes and job results | What the log may contain is not audited here |

## Where the data rests

PostgreSQL 18 (clinical record, audit chain, idempotency keys, outbox). Redis is in `compose.yaml`; this note did not check what the service keeps there. Patient photos are stored in the database. Backups: none exist (see the [retention schedule](retention-schedule.md)).

## Threats per flow (STRIDE, QA-10, DRAFT)

One short entry per row above. S spoofing, T tampering, R repudiation, I information disclosure, D denial of service, E elevation of privilege. Each entry names the main threat and what the code already does about it, taken from the tables above; it is a reading of the design, not a penetration test, and no control was exercised for this note. A dash means no control is described in this document, not that none exists.

| Flow | Main threats | Already in place | Gap or question |
|---|---|---|---|
| In: REST and FHIR from a person or client | S stolen token; E a role reaching a route it should not; I a `403` that reveals a record exists | PASETO or Keycloak token; a policy entry per route; `404` where existence would leak | Token theft and revocation are not modelled here; MFA is in the [MFA plan](mfa-plan.md) |
| In: UI server on the user's behalf | S the UI server acting as someone else; E the UI server's own credentials being broader than the user's | The user's own token is passed on | Whether the UI server ever holds a token of its own was not checked |
| In: referral recipient (`/api/referral-messages`) | S a forged outcome or decline; T a changed referral; R a recipient denying a message | Integrator role; append-only `referral_versions`; an unmatched queue | No message signature; the integrator credential is a shared secret of one system |
| In: hospital administration system over MLLP | S anyone on the network sending an ADT message; T a changed demographic; I traffic read in transit; D a flood of messages | Off unless `EPR_MLLP_BIND` is set; a fixed audit agent; the peer address is recorded; refusals go to a review queue | **No authentication or encryption.** Needs a private network, a tunnel or mutual TLS before real use; no rate limit is described |
| In: operator environment variables | T a changed key or switch; I a key in a process listing or log | The host's own access control | Secret handling and rotation are not described |
| Out: responses to a caller | I more than the policy allows; R an access with no trace | Policy per role; break-glass is audited and reviewed | The patient's access log hides the client IP address by design; whether every field is covered by a policy test is not stated |
| Out: the audit trail | T an edited or deleted row; R a missing row; I the IP address shown too widely | Append-only table with database triggers; a hash chain; a write is refused when its audit row cannot be written | The chain head is not published outside the database (QA-11); a database superuser can still rewrite it |
| Out: `/metrics` | I counters visible to anyone who can reach the port; D scraping load | Off unless `EPR_METRICS` is `1` or `true` | **Public when on.** Bind it to an internal interface or put it behind the proxy |
| Out: probes and `/_dependencies` | I which dependency is down, as labels; D probe load | Labels only | Public by design; whether the labels help an attacker was not assessed |
| Out: the transactional outbox | T a changed payload; I a payload sent to the wrong recipient | Nothing is delivered yet: the only sink is a log line | The recipient, its authentication and its payload filtering are undesigned |
| Out: the log | I personal data in a log line; R a log that can be edited | A line records outcomes and job results | What a log may contain is not audited; log retention and access are not described |
| At rest: PostgreSQL | I a stolen disk or backup; T a changed row; D loss of the only copy | Append-only tables; audit triggers | No backups exist; encryption at rest is a deployment matter not checked here; patient photos sit in the database |

Not done: a ranking by likelihood and impact, a data flow diagram, and a review by a security engineer. The [hazard log](clinical-safety/hazard-log.csv) is the place for a gap that could harm a patient; this table is not.

## Not checked

- No network capture or running deployment was inspected; this is code and configuration reading only.
- Whether any client sends data to a third party (analytics, fonts, CDN) was not audited in the UI beyond the CSP the build sets.
- Cross-border transfer: nothing here says where a deployment's database or Keycloak runs.
- The FHIR routes' exact field coverage is in `spec/interoperability`, not repeated here.
