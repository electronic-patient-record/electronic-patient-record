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
| A probe | `/_health`, `/_ping`, `/_readiness`, `/_dependencies` | Liveness, and which dependency is down, as labels | Public |
| Other systems | The transactional outbox (`axa_outbox_events`, `axa_outbox_payloads`) | Nothing yet: `outbox::enqueue` has no caller outside its own module, and the only sink is a log line | Delivery to a real recipient is not built |
| The log | `tracing` | Request outcomes and job results | What the log may contain is not audited here |

## Where the data rests

PostgreSQL 18 (clinical record, audit chain, idempotency keys, outbox). Redis is in `compose.yaml`; this note did not check what the service keeps there. Patient photos are stored in the database. Backups: none exist (see the [retention schedule](retention-schedule.md)).

## Not checked

- No network capture or running deployment was inspected; this is code and configuration reading only.
- Whether any client sends data to a third party (analytics, fonts, CDN) was not audited in the UI beyond the CSP the build sets.
- Cross-border transfer: nothing here says where a deployment's database or Keycloak runs.
- The FHIR routes' exact field coverage is in `spec/interoperability`, not repeated here.
