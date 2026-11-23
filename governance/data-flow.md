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

## The native apps (MP-11, DRAFT)

A new flow: **in and out, a phone to `/api`** (the same requests and answers as the web UI, over HTTPS, with a bearer token; spec `mobile-apps` §2). And a new place data may rest: **the phone**. The design allows nothing of the record to rest there (spec §3 rule 5); the table is what could go wrong with that, read from the spec, with no app yet touching a record.

| Where | Main threats | Already in place (by the spec; not built) | Gap or question |
|---|---|---|---|
| Phone to `/api` | S a stolen token; I traffic read in transit; T a changed request | HTTPS only (plain http for loopback in a debug build); the service's token checks and policies; a path outside `/api` refused before any request | A certificate pinning decision is not made; a debug build must not reach a real service |
| The token on the phone | I a refresh token read by another app or from a backup; E a token used from another device | Access token in memory; refresh token in the Keychain or Keystore, this device only; no `offline_access` | Android backup exclusion and iOS backup behaviour not yet set or tested; revocation procedure not written (see the [MFA plan](mfa-plan.md)) |
| Record data on screen | I a record seen by someone else: an unlocked phone put down, a shoulder, a screenshot, the app-switcher thumbnail | Idle lock; snapshot cover on iOS; `FLAG_SECURE` on Android | **iOS cannot block a screenshot.** A screenshot of a record is then a copy outside the system and the audit trail; H-061 records it |
| Record data in the OS | I a notification, widget, Spotlight entry, clipboard, keyboard dictionary, log line or crash report holding record text | The spec forbids all of these; no response cache on disk | The clipboard and keyboard are not addressed in the spec; no check exists that a build respects any of this |
| A lost or stolen phone | I everything above, for as long as the phone is unlocked or its token lives | Passcode and biometrics (device), idle lock (app), wipe if managed (OQ-MP1) | **Unmanaged devices cannot be wiped by the trust**; the report route and the owner of revocation are undefined |
| A shared phone | R an action blamed on the wrong clinician; I one clinician reading another's session | One user per install | Shared ward devices are not designed (see the MFA plan) |
| The build | S a fake app with the same sign-in; T a tampered build; supply chain in Lily and the Gradle and SwiftPM trees | Managed distribution recommended (OQ-MP3); Lily pinned to one commit | No signing, no release build, no dependency scanning for the apps (MP-12); Lily is not a published package |

Not done: a ranking, a review by a security engineer, a test on a real device, and the answers to OQ-MP1 and OQ-MP3.

## Not checked

- No network capture or running deployment was inspected; this is code and configuration reading only.
- Whether any client sends data to a third party (analytics, fonts, CDN) was not audited in the UI beyond the CSP the build sets.
- Cross-border transfer: nothing here says where a deployment's database or Keycloak runs.
- The FHIR routes' exact field coverage is in `spec/interoperability`, not repeated here.
