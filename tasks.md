# Tasks: Electronic Patient Record

Derived from [plan.md](plan.md) (research 2026-10-20). Spec: [spec/index.md](spec/index.md).
**As of 2026-10-20 no box is ticked: no code exists.**

IDs are grouped by prefix so they do not collide with the numbered build-out in
`spec/13-tasks.md`. Promote an item there (as `T-n`) when it is scheduled.

| Prefix | Area | Milestone |
|---|---|---|
| `SF-` | Scaffold and repository | M0 |
| `ID-` | Patient identity, matcher, MPI | M1 |
| `AU-` | Audit | M1 |
| `RC-` | Record core, resource store | M2 |
| `SE-` | Security, access control, consent | M3 |
| `DO-` | Documentation (notes, forms, flowsheets) | M4 |
| `OR-` | Orders, results, CDS | M5 |
| `IO-` | Interoperability, apps, export | M6 |
| `CA-` | Care coordination, scheduling, portal, downtime | M7 |
| `AS-` | Assurance: safety, usability, compliance, operations | M0–M8 |

Sizes: S ≈ one business day, M ≈ a few business days, L ≈ a business week or
more.

A box is ticked only when an automated test, or a clearly described manual
check, confirms the acceptance criterion. When an item is ticked, its entry
records what landed, what tests cover it and what is "Not built".

Conventions:

- Absent evidence is `null` with a reason (FHIR `dataAbsentReason`), never
  zero.
- Every new endpoint is pinned in `src/openapi.rs`, and FHIR interactions are
  pinned in the CapabilityStatement.
- Every read and write is audited.
- People stay references.
- The clinical record is append-only.
- Every item that changes clinical behaviour updates the hazard log
  (`AS-1`).

---

## M0 — Scaffold

- [x] **SF-1 (M) — Monorepo skeleton.** **Landed 2026-10-20.** `scripts/check-links.py` reports 0 broken links; `rust-toolchain.toml` equals the reference pin. **Not built:** `COMPARISONS.md`, `BENCHMARKS.md`, `llms.txt` (SF-8, AS-10). Description: Create
  `~/git/electronic-patient-record/electronic-patient-record` with the
  project-portfolio-management layout (plan.md §2.1):
  - root docs: `AGENTS.md`, `CLAUDE.md` containing `@AGENTS.md`, `README.md`, `index.md`, `CHANGELOG.md`, `CONTRIBUTING.md`, `GOVERNANCE.md`, `SECURITY.md`, `CODEOWNERS`, `CITATION.cff`, `AI_STATEMENT.md`, `RFC.md`, `NEWS.md`, `MAINTAINERS.md`
  - `LICENSE.md` plus `LICENSE/*.txt`
  - `rust-toolchain.toml` (1.99.1)
  - `.gitignore`, `.containerignore`, `lychee.toml`
  - `git init`

  **Acceptance:** `scripts/check-links.py` passes. `rust-toolchain.toml` matches the reference pin.
- [ ] **SF-2 (S) — Matcher crate.** **Partly landed 2026-10-20** (`src/{patient,nhs,normalize,engine}.rs`, 13 tests, fmt and clippy clean). **Not built:** `benches/`, `fuzz/`, `AGENTS.md`-level agent docs beyond the stub. Left open until those exist. Description:
  `electronic-patient-record-matcher-with-rust`:
  - lib `electronic_patient_record_matcher` plus a demo bin
  - dependencies limited to serde, strsim, unicode-normalization, thiserror and mimalloc
  - `deny.toml`, `benches/` (criterion), `fuzz/`, `spec/index.md`, `AGENTS.md`

  **Acceptance:** `cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test` is green.
- [x] **SF-3 (M) — Service crate.** **Landed 2026-10-20.** `cargo test --lib` 6 pass; `tests/audit_immutable.rs` 2 pass against Postgres 18; the release image built and `/_health` returned 200 under compose; with no keys the container exits 1 with "refusing to start"; with a key set, an unauthenticated call gets 401. **Not built:** everything past the guard and the compare endpoint (OpenAPI, rate limiting, CORS allow-list, Prometheus, OTLP). Description:
  `electronic-patient-record-service-with-rust`:
  - loco-rs 1.2 with `default-features = false` and no `auth` feature
  - SeaORM 2.0, migration crate `electronic-patient-record-migration`
  - `/_health` and `/_ping`
  - `authentication-verifier` PASETO check, login required by default; refuses to start without keys or with login off in `production`
  - `compose.test.yaml` (Postgres 18), `Containerfile` (non-root)

  **Acceptance:** `cargo test --lib` is green. A DB-gated smoke test passes against Postgres. The container boots in compose and `/_health` returns 200.
- [ ] **SF-4 (M) — UI shell.** **Partly landed 2026-10-20**: kit 3 shell, locale-prefixed URLs for the ten locales, CSP and security headers, BFF proxy, `check` 0 errors and 0 warnings, 26 vitest, 7 Playwright with axe; the UI image builds and serves. **Not built:** Lily widgets and themes, the `DataTable` wrapper, `@sveltia/i18n` and any translation (English only), a session cookie and sign-in. Left open. Description: `electronic-patient-record-ui-with-svelte`:
  - SvelteKit with Svelte 5 runes, TypeScript strict, adapter-node
  - Lily widgets and themes, `@sveltia/i18n`, ten locales with `<language>-<region>` URL prefixes
  - BFF cookie session (the browser holds no token)
  - CSP that forbids inline style
  - `DataTable` wrapper, `container-server.mjs`, `Containerfile`

  **Acceptance:** `pnpm run check` reports 0 errors and 0 warnings. `npx vitest run` passes. Playwright with axe passes on the shell pages.
- [ ] **SF-5 (M) — CI and release.** **Partly landed 2026-10-20**: `ci.yml`, `release.yml` (adapted from the reference) and Dependabot are written and parse as YAML. **Not verified:** none of it has run on GitHub. There is no `chaos.yml` and no `zap` job. Left open. Description: Workflows `ci.yml` (jobs api, api-db,
  restore, matcher, deny, ui, docs, container, zap, fresh-clone, ci-ok),
  `release.yml` (tag must equal crate and UI versions; push to GHCR),
  `chaos.yml` (weekly), and Dependabot.
  **Acceptance:** all jobs are green on the scaffold. A tag mismatch fails the release.
- [x] **SF-6 (S) — Root compose.** **Landed 2026-10-20.** `podman compose up -d --build --wait` reached healthy with a key set; with no key the api container exits 1 with "refusing to start". **Not built:** the `dev-insecure` and `demo` profiles. Description: `compose.yaml` runs postgres, migrate and
  api, plus the `dev-insecure` and `demo` profiles, and is secure by default.
  **Acceptance:** `podman compose up -d --build --wait` reaches healthy. With no key set, the api refuses to start.
- [x] **SF-7 (M) — Spec chapters.** **Landed 2026-10-20** as drafts: chapters 01–18, eight cross-cutting indexes, `locales.md`, three subproject specs; the link checker passes. Chapter content describes targets and says what is built. **Not built:** per-chapter depth (FR acceptance detail, an endpoint inventory beyond M0). Description: Draft `spec/01-…18-` and the
  cross-cutting topic indexes listed in `spec/index.md`. Each subproject gets
  its own `spec/index.md`.
  **Acceptance:** every link in `spec/index.md` resolves (lychee).
- [ ] **SF-8 (S) — Agent guidance.** **Partly landed 2026-10-20**: `agents/` (index, subprojects, spec-driven-development, models, matching, restful, fhir, testing) and `electronic-patient-record-skill/`. **Not built:** `llms.txt`, `llms.json`, `scripts/generate-llms.py`. Left open. Description: `agents/` holds `index`,
  `spec-driven-development`, `testing`, `restful`, `models`, `matching`, and a
  new `fhir.md`. Add `electronic-patient-record-skill/SKILL.md`, `llms.txt`,
  `llms.json` and `scripts/generate-llms.py`.
  **Acceptance:** `python3 scripts/generate-llms.py` is idempotent, and the docs job checks it.
- [x] **SF-9 (S) — Status generator.** **Landed 2026-10-20.** `sh scripts/generate-status.sh` writes the block; `--check` exits 1 on a stale file (tried by hand). **Not verified:** the CI `docs` job has not run. Description: `scripts/generate-status.sh` writes
  `spec/14-implementation-status.md` from test counts.
  **Acceptance:** the docs job fails when the status file is stale.

## M1 — Identity and audit

- [ ] **ID-1 (L) — `Patient` type and matcher.** A thin identity record:
  - fields: names (with use and period), birth date, administrative sex, identifiers (system plus value; NHS number, MRN, SSN-last-4 marked as weak), addresses, telecom and links
  - Fellegi-Sunter style weighted comparators: Jaro-Winkler for names, date transposition tolerance, address normalisation
  - NFKC folding that preserves diacritics
  - a verified national identifier acts as a deterministic key
  - every score carries a per-field breakdown

  **Acceptance:** deterministic (the same input always gives the same score, checked by a property test). A labelled corpus of 200 or more pairs, including twins, junior/senior, transposed dates and married-name change, reaches precision ≥ 0.99 at the auto-link threshold. Fuzz runs 10 minutes without a panic.
- [ ] **ID-2 (M) — NHS number and identifier validation.**
  - NHS number check digit (modulus 11)
  - identifier systems registry
  - US MRN per assigning authority

  **Acceptance:** valid and invalid NHS numbers are accepted and refused in unit tests. An unknown system is kept but marked unverified.
- [ ] **ID-3 (M) — Patient CRUD and search.** `/api/patients` and
  `/fhir/R4/Patient`: create, read, update (versioned), search by name,
  birthdate, identifier and phonetic name; tantivy typeahead.
  **Acceptance:** search p95 ≤ 200 ms at 1M patients in the load test. Every call writes an audit row.
- [ ] **ID-4 (M) — Duplicate prevention at registration.** Before a patient
  is created, matcher candidates above the review threshold are returned.
  Override requires a reason.
  **Acceptance:** creating a likely duplicate returns 409 with candidates and their breakdowns. An override with a reason creates the patient and an audit row. The UI shows the comparison side by side.
- [ ] **ID-5 (L) — Merge and unmerge.** Merge writes `Patient.link`
  (replaced-by / replaces), re-points references by provenance (it never
  rewrites history), and a review queue holds candidates in the uncertain band.
  Unmerge restores the original state exactly.
  **Acceptance:** after a merge and then an unmerge, every resource returns to its original patient (property test). Merge and unmerge each write provenance and audit. Merging two records with conflicting verified national identifiers is refused.
- [ ] **ID-6 (S) — FHIR `Patient/$match`.**
  **Acceptance:** returns a Bundle with a `search.score` and a match-grade extension. Results are ordered by score.
- [ ] **ID-7 (M) — Registration UI.** `/patients/new`, `/patients`,
  `/patients/[id]` demographics with the persistent banner (photo, name, two
  identifiers, age, allergies status).
  **Acceptance:** Playwright plus axe pass. The banner is visible on every patient route (test).
- [ ] **AU-1 (L) — Audit writer.** **Partly landed 2026-10-20** (`src/audit.rs`; 6 unit tests, 7 Postgres tests in `tests/audit_chain.rs`): rows are hash-chained (keyed MAC when `EPR_INTEGRITY_MAC_KEY` is set), appended under an advisory lock, verified by `audit::verify`; `POST /api/patients/compare` is audited before it answers and fails closed. **Not built:** a verify *endpoint* (the acceptance says endpoint; see AU-2), audit of denied requests, audit on any other route (none read stored data yet), BALP/FHIR `AuditEvent` output, refusing to start in production without a key. Left open. Original description: One AuditEvent per access, using the BALP
  shape: agent, patient, entity, action (C/R/U/D/E), outcome, purpose of
  use, network address, break-the-glass flag. Rows are append-only and
  hash-chained with `integrity-mac`.
  **Acceptance:** each controller has a test asserting an audit row. A verify endpoint detects a tampered row. Audit cannot be disabled by configuration.
- [ ] **AU-2 (S) — Audit read API.** `/api/audit` and
  `/fhir/R4/AuditEvent` search, auditor role only.
  **Acceptance:** a non-auditor gets 403, and the 403 is itself audited.

## M2 — Record core

- [ ] **RC-1 (L) — Resource store.** `resource_version` and
  `resource_current` tables (plan.md §6.2), append-only, holding
  `meta.versionId` and `lastUpdated`.
  **Acceptance:** update creates version n+1. The previous version stays readable via vread. There is no SQL path that deletes a version (enforced by a migration test and a DB trigger).
- [ ] **RC-2 (L) — Search parameter indexing.** Token, string, date,
  reference, quantity and URI index tables. Support `_id`, `_lastUpdated`,
  `patient`, `code`, `date`, `status`, `category`, `_include`, `_count`,
  `_sort`.
  **Acceptance:** search results match the HAPI FHIR server (used as an oracle in CI) on a shared fixture set.
- [ ] **RC-3 (M) — Provenance on every write.** Records the agent (human,
  device or AI), on-behalf-of, reason and activity.
  **Acceptance:** every write test asserts a Provenance. A write without an authenticated agent is refused.
- [ ] **RC-4 (M) — Entered-in-error and amendment.** No delete endpoint for
  clinical data. `entered-in-error` hides the resource from default views but
  keeps it in history. Patient-requested amendment workflow (HIPAA §164.526):
  accept or deny, with a statement of disagreement.
  **Acceptance:** after entered-in-error the resource is absent from search and present in `_history`. The amendment outcome is visible to readers.
- [ ] **RC-5 (L) — Problems, allergies, medications, immunisations.**
  `Condition`, `AllergyIntolerance`, `MedicationStatement`/`MedicationRequest`
  and `Immunization`, coded (SNOMED, RxNorm/dm+d, CVX). "No known allergies"
  is an explicit assertion. Verification and clinical status are required.
  **Acceptance:** UK Core and US Core profile validation passes in CI (HAPI validator). An empty allergy list shows "not recorded", never "none".
- [ ] **RC-6 (M) — Vitals and observations.** `Observation` with LOINC codes,
  UCUM units, reference ranges and abnormal flags.
  **Acceptance:** a unit mismatch is refused 422. Flags are computed from ranges and covered by unit tests.
- [ ] **RC-7 (M) — Terminology service.** `TerminologyService` trait with
  `$lookup`, `$validate-code`, `$expand` and `$translate`. Two adapters:
  Snowstorm and an embedded fixture subset for tests.
  **Acceptance:** a code from an unpinned version is refused. Tests run with no network.
- [ ] **RC-8 (L) — Patient summary UI.**
  - `/patients/[id]/summary` showing problems, allergies, active medications, recent results, vitals trend and upcoming appointments
  - pages for problems, allergies, medications and immunisations, with add, edit and entered-in-error
  - medication reconciliation view

  **Acceptance:** chart open is p95 ≤ 1 s in a Playwright performance test on the seeded demo. axe passes. Keyboard-only operation is tested.
- [ ] **RC-9 (S) — UTC and time zones.** Store UTC, keep the recorded zone,
  and render in the user's zone.
  **Acceptance:** a time-zone test runs under three zones and around a DST transition.

## M3 — Security, access control, consent

- [x] **SE-1 (L) — Policy engine.** **Landed 2026-10-21, in part.** Combines RBAC roles (plan.md §6.3) with
  ABAC attributes (care team or legitimate relationship, encounter,
  location, purpose of use, security labels). It is evaluated in the service
  on every read and write.
  **Acceptance (met):** a policy matrix test covers each role × resource × label combination (`policy::tests::invariants_hold_across_the_whole_matrix`). The UI cannot bypass the engine because no endpoint skips it (`tests/route_census.rs`; the middleware refuses any route without a table entry).
  **Not built:** no handler calls the record-level `evaluate` yet (none reads a stored record); no care-relationship lookup, consent store, break-the-glass store or masking (SE-2 to SE-4); no ABAC on location or encounter beyond a relationship value; the permission table, label lists and purpose rules are unreviewed by a clinical safety officer or privacy officer and need that before real use; authorization is off when login is off (development only); 401s are not audited.
- [ ] **SE-2 (M) — Consent.** FHIR `Consent` (permit/deny provisions by
  actor, purpose, data class and period). National data opt-out for
  secondary use. 42 CFR Part 2 segmentation via the `SUD` label.
  **Acceptance:** a denied provision hides the data and audits the denial. Bulk export honours consent.
- [ ] **SE-3 (M) — Break-the-glass.** A mandatory reason, a time-boxed grant
  (default 4 hours), a real-time notification to the privacy officer, and a
  review queue.
  **Acceptance:** access without a relationship gets 403 with a BTG offer. After BTG, access is granted and the audit row carries the flag and reason. The grant expires on time.
- [ ] **SE-4 (M) — Sensitive record masking.** `R`/`V` confidentiality,
  `PSY`, `HIV`, `SUD` and `ETH` labels mask content and show "restricted
  information exists".
  **Acceptance:** the masked response contains no restricted data, checked by a JSON scan test.
- [ ] **SE-5 (M) — Multi-tenancy.** `tenant_id` on every table, PostgreSQL
  RLS, per-tenant data-encryption keys.
  **Acceptance:** a cross-tenant read returns nothing even with a forged id (DB-level test with RLS on).
- [ ] **SE-6 (S) — Patient access log.** The patient sees who accessed their
  record and when (role and organisation; staff names per deployment policy).
  **Acceptance:** a portal test shows an access made by a clinician.
- [ ] **SE-7 (M) — Hardening.** Security headers, CSP, rate limiting,
  session timeout with re-auth for sensitive actions, encryption at rest
  documented, ZAP baseline in CI, cargo-deny, SBOM.
  **Acceptance:** the ZAP job has no high or medium findings. The SBOM check passes.

- [x] **SE-8 (M) — Keycloak token verification.** **Landed 2026-10-20.** The guard accepts Keycloak access tokens
  (`EPR_KEYCLOAK_*`), routes by token shape (`v4.` to PASETO, else Keycloak), and records the token `sub` as the audit
  agent. A development realm and compose profile run Keycloak 26.3.0 on the shared PostgreSQL under Podman.
  **Acceptance (met):** 8 DB-free tests with a local OIDC provider; by hand against the real Keycloak, a token gets 200 and no
  token, garbage and a PASETO-shaped token get 401; four bad configurations each refuse to start; `cargo deny` clean.
  **Not built:** roles are not consulted (SE-1); TLS or a Keycloak reachable from inside compose; brokering to Entra ID;
  testing beyond Keycloak 26.3.0 `start-dev`.

- [x] **SE-9 (L) — UI sign-in with Keycloak.** **Landed 2026-10-21.** Authorization code flow with PKCE through the `epr-web` client, in the UI's
  backend-for-frontend: the browser holds one opaque `httpOnly` session cookie, the server holds the tokens and sends the
  access token to the API, refresh and sign-out work, and an idle session ends.
  **Acceptance:** a Playwright run against a Keycloak container signs in, calls a protected API route, and signs out; a
  tampered or expired session is refused; no token is readable by page scripts.
  **Met by hand** (`playwright.keycloak.config.ts`: 5 passed, 1 skipped) except expiry, which is unit-tested with a stub provider.
  **Not built:** a shared session store (it is in process memory); the end-to-end run in CI; Keycloak-initiated logout; Entra ID;
  translated sign-in strings.

## M4 — Documentation

- [ ] **DO-1 (L) — Clinical notes.**
  - `Composition` and `DocumentReference`, problem-oriented templates (SOAP and specialty)
  - draft → signed; co-sign / attestation
  - signed notes accept only addenda

  **Acceptance:** editing a signed note gets 409 and an addendum is the only path. Co-sign is required for configured roles.
- [ ] **DO-2 (M) — Copy-forward provenance.** Text copied from a prior note
  is marked with its source in storage and visibly in the UI. A note's copied
  percentage is shown.
  **Acceptance:** copied text keeps a link to its source after save. The copied percentage is computed (unit test).
- [ ] **DO-3 (M) — Smart phrases.** User and shared macros. Each insertion is
  traceable.
  **Acceptance:** inserted text records which macro and version produced it.
- [ ] **DO-4 (L) — Structured forms.** FHIR `Questionnaire` and
  `QuestionnaireResponse` (SDC subset: enableWhen, calculated items),
  authored by clinical admins without code changes.
  **Acceptance:** a form round-trips. A calculated score matches published examples, for example PHQ-9.
- [ ] **DO-5 (M) — Flowsheets and NEWS2.** A vitals charting grid. NEWS2
  is computed with escalation guidance.
  **Acceptance:** NEWS2 matches the RCP reference cases. The hazard log has an entry for score miscalculation.
- [ ] **DO-6 (M) — AI-drafted content boundary.** Content from a SMART app
  such as an ambient scribe arrives as a draft, with Provenance naming the
  device or AI agent, and must be reviewed before signing.
  **Acceptance:** an AI-drafted note cannot be signed without an explicit review action. The provenance survives signing.

## M5 — Orders, results, decision support

- [ ] **OR-1 (L) — CPOE.** `ServiceRequest` (lab, imaging, referral,
  nursing) and `MedicationRequest`, with draft → signed → active → completed
  and cancelled. Identity re-confirmation is required when the user switched
  patients in the last N seconds.
  **Acceptance:** order sign is p95 ≤ 2 s. A wrong-patient guard test passes. Every state change carries provenance.
- [ ] **OR-2 (M) — Order sets.** Versioned, clinician-curated sets with
  defaults.
  **Acceptance:** applying a set creates draft orders and none is signed automatically.
- [ ] **OR-3 (L) — HL7 v2 inbound.** An MLLP listener plus a parser for
  ADT A01/A03/A04/A08/A40 and ORU R01, mapped to FHIR. Unmatched messages go
  to a work queue.
  **Acceptance:** the sample message corpus parses. A message for an unknown patient is queued, not dropped. The parser is fuzzed.
- [ ] **OR-4 (L) — Results inbox and closed loop.** Routing to the ordering
  clinician or the team. Abnormal results first. Bulk acknowledge of normal
  results. An unacknowledged abnormal result escalates after a configured
  time (SAFER guide: test results).
  **Acceptance:** an escalation test uses a fake clock. Acknowledgement is audited. A result whose order has no clinician goes to a fallback queue.
- [ ] **OR-5 (L) — CDS Hooks client.** `patient-view`, `order-select` and
  `order-sign` hooks with prefetch. Cards are tiered: interruptive only for
  high severity and actionable; others passive. Override reasons are
  captured.
  **Acceptance:** a stub CDS service returns cards that render at the right tier. Overrides are stored. A CDS timeout never blocks signing (fail open, recorded in the hazard log).
- [ ] **OR-6 (M) — Built-in safety checks.** Drug-allergy matching (class
  level, via terminology) and duplicate-order detection. The drug
  interaction knowledge base comes from an external CDS service only. Ship
  demo fixtures labelled "not for clinical use".
  **Acceptance:** an allergy to penicillin fires on an amoxicillin order. The fixture banner is present in the demo profile.
- [ ] **OR-7 (S) — Alert analytics.** Override rate per rule and alert
  volume per user per day.
  **Acceptance:** the dashboard numbers match seeded data.

## M6 — Interoperability and apps

- [ ] **IO-1 (M) — CapabilityStatement.** `/fhir/R4/metadata`, generated
  from the mounted routes.
  **Acceptance:** a census test asserts every FHIR route is listed and nothing extra is.
- [ ] **IO-2 (L) — SMART App Launch 2.2.**
  - `/.well-known/smart-configuration`
  - EHR launch and standalone launch, PKCE
  - granular scopes mapped to the policy engine
  - backend services (asymmetric client auth)

  **Acceptance:** the Inferno SMART tests pass in CI. A scope narrower than the policy wins.
- [ ] **IO-3 (L) — US Core and UK Core conformance.** Validate every
  stored and returned resource against the profiles.
  **Acceptance:** the Inferno (g)(10) test kit runs in CI and failures are listed in the status file (not hidden). The UK Core validator runs clean on the fixtures.
- [ ] **IO-4 (M) — Bulk Data `$export`.** Patient, Group and system level.
  NDJSON, asynchronous, honouring consent and labels.
  **Acceptance:** an export of the seeded demo matches the resource counts. Restricted data is excluded.
- [ ] **IO-5 (M) — IPS `$summary` and `$everything`.**
  **Acceptance:** the IPS document validates against the IPS IG. `$everything` is audited as an export.
- [ ] **IO-6 (M) — Subscriptions and webhooks.** R5-backport topic-based
  subscriptions with signed webhook delivery from the outbox.
  **Acceptance:** delivery retries with backoff. The signature is verified in the test.
- [ ] **IO-7 (S) — SQL-on-FHIR v2 views.** ViewDefinitions for the main
  analytics tables.
  **Acceptance:** the views run against the export and produce the expected columns.
- [ ] **IO-8 (M) — EHI export.** A full single-patient electronic health
  information export (ONC (b)(10) shape), downloadable by the patient.
  **Acceptance:** the export includes every resource type held for the patient (census test).

## M7 — Care coordination, scheduling, portal, downtime

- [ ] **CA-1 (L) — Encounters and ADT.** `Encounter` and `EpisodeOfCare`.
  Admit, transfer, discharge, bed board, discharge summary.
  **Acceptance:** an ADT state machine test passes. The discharge summary pulls the active problems and medications.
- [ ] **CA-2 (M) — Care plans, goals, care teams, tasks.**
  **Acceptance:** a task assigned to a care team appears in each member's worklist.
- [ ] **CA-3 (M) — Referrals with closed loop.** `ServiceRequest` plus
  `Task`, with status tracked until the outcome is received.
  **Acceptance:** a referral with no response after N days escalates (fake clock).
- [ ] **CA-4 (L) — Scheduling.** `Schedule`, `Slot` and `Appointment`.
  Templates, a waitlist and reminders by email.
  **Acceptance:** double-booking is refused unless overbooking is configured. Reminder emails are sent in a test.
- [ ] **CA-5 (L) — Patient portal.**
  - `/portal`: results with configured release rules, letters, secure messaging, booking, questionnaires and record download
  - proxy access (parent/carer), with adolescent confidentiality rules

  **Acceptance:** a proxy cannot see data the policy marks adolescent-confidential. Results release follows the rule table.
- [ ] **CA-6 (M) — Inbox and worklists.** A team inbox with delegation and
  keyboard-first triage.
  **Acceptance:** keyboard-only triage of 10 items has a Playwright click/time budget.
- [ ] **CA-7 (M) — Downtime snapshot.** A per-location encrypted read-only
  export of active patients' summaries, medications, allergies and MAR, and
  a printable IPS.
  **Acceptance:** the snapshot opens with the service stopped. Its age is shown on every page.

## Assurance (spans all milestones)

- [ ] **AS-1 (M, then ongoing) — Clinical safety file (DCB0129).** A
  clinical risk management plan, a hazard log (`governance/clinical-safety/hazard-log.csv`)
  and a safety case report template. The CSO role is named (OQ-2).
  **Acceptance:** CI fails if a PR labelled `clinical-impact` does not touch the hazard log.
- [ ] **AS-2 (M) — Governance pack.** DPIA, record of processing, personal
  data inventory, retention schedule (UK Records Management Code of Practice
  2023 and US ranges), DTAC 2026 checklist, accessibility statement, and a
  HIPAA Security Rule mapping (to the 2025 NPRM baseline, marked "not final").
  **Acceptance:** every document is linked from `governance/README.md`, and no document claims certification.
- [ ] **AS-3 (S) — SAFER Guides mapping.** The eight 2025 guides mapped to
  features and gaps.
  **Acceptance:** each practice is marked met, partial or not met with evidence.
- [ ] **AS-4 (L) — Usability programme.**
  - a documented UCD process (NISTIR 7741 / ISO 9241-210)
  - formative testing from M2
  - summative NISTIR 7804 testing at M8 on the (g)(3) critical tasks with ≥ 10 users each, reported in NISTIR 7742 format

  **Acceptance:** the report exists and SUS is ≥ 75, or the shortfall and its remediation are recorded.
- [ ] **AS-5 (M) — Click and time budgets.** Playwright budgets for critical
  tasks: register a patient, record an allergy, prescribe, acknowledge
  results, sign a note.
  **Acceptance:** CI fails when a budget regresses by more than 10%.
- [ ] **AS-6 (M) — Load and chaos tests.** `scripts/load-test.mjs` with
  1M patients and 50M resources, and `scripts/chaos-test.sh` (database and
  service faults).
  **Acceptance:** the plan.md §6.6 targets are met or the gap is recorded in `BENCHMARKS.md`.
- [ ] **AS-7 (M) — Backup, restore, recovery.** Scripts, a restore test in
  CI, the recovery objectives documented, a runbook, and downtime
  procedures.
  **Acceptance:** the restore job restores and verifies the audit hash chain.
- [ ] **AS-8 (M) — Demo dataset.** Fictitious patients (Synthea-style),
  seeded by `scripts/seed-demo.mjs`, and the compose `demo` profile.
  **Acceptance:** the demo contains no real identifiers. NHS numbers come from the 999 test range only.
- [ ] **AS-9 (S) — Role-targeted documents.** `locales/en-gb-oxendict/for/`
  documents for the chief clinical information officer, Caldicott guardian,
  clinical safety officer, CIO, CISO and data protection officer.
  **Acceptance:** linked from `index.md`.
- [ ] **AS-10 (S) — Comparisons.** `COMPARISONS.md` against OpenEMR,
  OpenMRS, Medplum, HAPI FHIR and EHRbase, stating plainly where we are
  behind.
  **Acceptance:** each claim is sourced or marked unverified.
- [ ] **AS-11 (S) — Primary-source checks.** Resolve the plan.md §12 list.
  **Acceptance:** each item is confirmed, corrected or dropped, with a source link.
