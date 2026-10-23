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
| `CQ-` | Code quality and defects found in review (research 2026-10-21) | any |

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
- [x] **ID-3 (M) — Patient CRUD and search.** **Landed 2026-10-21, in part (native REST only).** `/api/patients` and
  `/fhir/R4/Patient`: create, read, update (versioned), search by name,
  birthdate, identifier and phonetic name; tantivy typeahead.
  **Acceptance:** search p95 ≤ 200 ms at 1M patients in the load test. Every call writes an audit row.
  **Built:** `POST/GET /api/patients`, `GET/PUT /api/patients/{id}` over append-only `patient_versions` (optimistic `expected_version`, 409 on a stale or racing write); search by family-name prefix, exact birth date and `system|value` identifier, at most 50 results, never "everyone"; NHS number check digit and birth date validated (422); role gate plus the record-level `policy::evaluate` with a declared `X-Purpose-Of-Use` (default `operations`); every success and refusal audited, no demographics in the audit detail. **Run by hand** against Keycloak 26.3.0: receptionist creates (201), clinician reads, updates and searches, a stale update is 409, the patient role, the auditor and a research purpose are 403, DELETE is 403.
  **FHIR landed 2026-10-21:** `/fhir/R4/Patient` (read, search-type on family, birthdate and identifier, create, update with `If-Match`) and `/fhir/R4/metadata`, a thin layer over the same handlers so the role gate, relationship, consent, purpose and audit row are identical. **Not built:** FHIR history, conditional create, paging and date-search prefixes; phonetic search and tantivy typeahead; any load test, so the p95 target is **unmeasured** (search is a sequential scan over the current-version view, with no index); the unmatched-name edge cases of real names; a care-relationship lookup, so the patient and proxy roles cannot read even their own record; the write and its audit row are two transactions, so a crash between them leaves an unaudited write; 422 and 400 responses are not audited; `PUT` replaces the whole record (an omitted identifier is dropped from the new version, though the old version keeps it); UI pages landed with ID-7.
- [x] **ID-4 (M) — Duplicate prevention at registration.** **Landed 2026-10-21, API only.** Before a patient
  is created, matcher candidates above the review threshold are returned.
  Override requires a reason.
  **Acceptance:** creating a likely duplicate returns 409 with candidates and their breakdowns. An override with a reason creates the patient and an audit row. The UI shows the comparison side by side (landed with ID-7).
  **Met for the API** (by hand and in tests): 409 with each candidate's stored record, score, per-field breakdown and conflicts; a blank `override_reason` is still 409; a real one creates the patient, keeps the reason in the version history and audits the candidate ids (refusals are audited too, as the candidates are disclosed).
  **Not built:** candidate retrieval only considers patients sharing an identifier, a birth date or a family name (exact, case-insensitive), so a duplicate with a misspelt name, a different birth date and no shared identifier is **not found**; two simultaneous registrations of one person can both pass the check (no lock or unique constraint); an `auto_link` candidate is blocked the same way as a `review` one; no separate permission for overriding.
- [ ] **ID-5 (L) — Merge and unmerge.** **Partly landed 2026-10-21:** `POST /api/patients/{id}/merge` and `/unmerge` (`{id}` is the surviving record; body `{source, reason}`), `GET …/links`; a merge is an append-only **link** (`patient_link_versions`), so nothing is moved or rewritten and unmerge restores both records exactly. Rules: a reason is required; the matcher must not say non-match (two different verified NHS numbers always refuse); the old record must not already be merged away and the survivor must not be; merges are serialised (no race, no cycle). Effects: the old record reads `replaced_by`, is left out of search and the duplicate check, and is read-only; the survivor's allergies include the old record's (each still naming its own patient); a consent refusal on either record applies to both; FHIR reads show `active:false` and `link` `replaced-by`/`replaces`; a care relationship does not carry over. New policy resource `PatientLink` (clinical-admin writes, clinician reads). 3 unit, 7 Postgres tests (including the property test: random sequences of adding allergies, merging and unmerging against a model, and unmerging everything restores every record; I broke the union and the reversal on purpose and the tests failed), 2 stubbed Playwright tests, 1 real-Keycloak test; hazard H-020. **Not built:** the review queue for the uncertain band; a second-person confirmation; photos, relationships and the no-known-allergies statement following a merge; clinical content other than allergies following a merge (nothing else exists yet); a merge screen that finds candidates (the page asks for an id). Left open. Original: Merge writes `Patient.link`
  (replaced-by / replaces), re-points references by provenance (it never
  rewrites history), and a review queue holds candidates in the uncertain band.
  Unmerge restores the original state exactly.
  **Acceptance:** after a merge and then an unmerge, every resource returns to its original patient (property test). Merge and unmerge each write provenance and audit. Merging two records with conflicting verified national identifiers is refused.
- [ ] **ID-6 (S) — FHIR `Patient/$match`.**
  **Acceptance:** returns a Bundle with a `search.score` and a match-grade extension. Results are ordered by score.
- [ ] **ID-7 (M) — Registration UI.** **Partly landed 2026-10-21**: `/patients` (search), `/patients/new` (register, with the ID-4 side-by-side duplicate view and a reasoned override) and `/patients/[id]` (view and version-checked edit); 13 new vitest, 10 stubbed Playwright tests with axe, and a real Keycloak and API end-to-end test. **Banner refresh landed 2026-10-21** (looks again when the tab is shown and once a minute while shown; never overwrites typing). **Banner landed 2026-10-21** on `/patients/[id]` and on each record in `/my-record`: name, birth date and age (years, months under two, "about" for a partial date, absent rather than guessed), sex, NHS number as `943 476 5919`, one other identifier, "No photo held", and the allergy line (now real: see RC-5; it still reads **Not recorded in this system. This does not mean there are none.** when nothing is recorded) It shows the stored record, never the form. 5 vitest and 2 stubbed Playwright tests with axe; hazard H-016. **Photo landed 2026-10-21** (PNG or JPEG up to 512 KiB, type from the bytes, append-only versions, withdrawal is a version; shown in the banner; hazard H-017). **Not built:** a check that the photo is of this patient, a FHIR `Patient.photo` mapping, a run of the banner against the real Keycloak stack beyond the allergy and photo tests, banner on `/patients/new` and the compare page (no patient is on record there), pagination; pagination; editing a second name or address; the end-to-end run is not in CI. Left open. Description: `/patients/new`, `/patients`,
  `/patients/[id]` demographics with the persistent banner (photo, name, two
  identifiers, age, allergies status).
  **Acceptance:** Playwright plus axe pass. The banner is visible on every patient route (test).
- [ ] **AU-1 (L) — Audit writer.** **Partly landed 2026-10-20** (`src/audit.rs`; 6 unit tests, 7 Postgres tests in `tests/audit_chain.rs`): rows are hash-chained (keyed MAC when `EPR_INTEGRITY_MAC_KEY` is set), appended under an advisory lock, verified by `audit::verify`; `POST /api/patients/compare` is audited before it answers and fails closed. **Verify endpoint landed 2026-10-21** (`GET /api/audit/verify`, auditor only; 409 when broken; it audits itself; 2 more Postgres tests, 9 in all; run by hand against Keycloak: 401, clinician 403, auditor 200, then 409 naming row 1 after a tampered row). Route refusals and the patient, consent and verify routes are audited too. **Unheld key landed 2026-10-21:** rows made under a key this service does not hold answer 424 `key-not-held` with the causes, not 409 (idea 2 of the openEHR comparison). **Checkpoint landed 2026-10-21** (the answer carries `head`; `?checkpoint_id=&checkpoint_digest=` confirms it later, 409 if the row is missing or different; 1 more Postgres test, 10 in all). **Not built:** nothing publishes the head for the operator, so removal of the *newest* rows is detected only against a head someone stored outside the database; the "each controller has a test asserting an audit row" acceptance is only met by hand-run checks for the patient and consent controllers; 401, 400 and 422 responses are not audited; (a write and its audit row are now ONE transaction for patient, consent and relationship create/update, landed 2026-10-21; the other audit rows, reads and refusals, are still written on their own, which is deliberate: they have no write to be atomic with, and each is audited before its data leaves, with a 500 and no disclosure when the row cannot be written); BALP/FHIR `AuditEvent` output; refusing to start in production without a key. Left open. Original description: One AuditEvent per access, using the BALP
  shape: agent, patient, entity, action (C/R/U/D/E), outcome, purpose of
  use, network address, break-the-glass flag. Rows are append-only and
  hash-chained with `integrity-mac`.
  **Acceptance:** each controller has a test asserting an audit row. A verify endpoint detects a tampered row. Audit cannot be disabled by configuration.
- [x] **AU-2 (S) — Audit read API.** **Landed 2026-10-21:** `GET /api/audit` (filters `agent`, `patient`, `action`, `outcome`, `purpose`, `break_glass`, `from`, `to`, `before`; newest first, 50 a page, blank text criteria ignored) and `GET /fhir/R4/AuditEvent` (`agent`, `patient`, `action`, `outcome`, `date=ge..&date=lt..`, `_before`; BALP-shaped, `next` link), auditor role only; every search writes its own row naming the criteria used (not their values). Run against Keycloak: no token 401, clinician 403 and the refusal is in the trail, auditor 200 for both. **`le`/`gt` and export landed 2026-10-21:** all four date prefixes (a bare date covers its whole day, an instant is exact) and `GET /api/audit/export` (CSV or NDJSON, up to 10,000 rows with the digests, formula-safe CSV, itself audited, `x-next-before` for the next file). **Not built:** text search over `detail`, FHIR `read` of one AuditEvent, BALP/IHE profile conformance (the codes for purpose of use are local, not v3 ActReason). Original note: **Not built** (`GET /api/audit/verify` exists, see AU-1; the 403-is-audited half of the acceptance already holds for every route). `/api/audit` and
  `/fhir/R4/AuditEvent` search, auditor role only.
  **Acceptance:** a non-auditor gets 403, and the 403 is itself audited.

## M2 — Record core

- [ ] **RC-1 (L) — Resource store.** `resource_version` and
  `resource_current` tables (plan.md §6.2), append-only, holding
  `meta.versionId` and `lastUpdated`.
  **Acceptance:** update creates version n+1. The previous version stays readable via vread. There is no SQL path that deletes a version (enforced by a migration test and a DB trigger).
- [ ] **RC-2 (L) — Search parameter indexing.** **Note (2026-10-21, from CQ-9):** patient search by family name (prefix over the names array) and the duplicate check's family arm have no index, and the duplicate check scans because of it; decide the folded-name index here. Token, string, date,
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
- [ ] **RC-5 (L) — Problems, allergies, medications, immunisations.** **Allergies partly landed 2026-10-21** (the first clinical content, native REST only): append-only `allergy_versions` and `allergy_assertion_versions`; `GET`/`POST /api/patients/{id}/allergies`, `PUT …/allergies/{allergy_id}`, `PUT …/allergy-statement`; "no known allergies" is a recorded statement by a named author that cannot be made while an active allergy exists and is voided by anything recorded after it; a care relationship, purpose and consent apply; the banner and an allergy section on the patient page use one answer. 6 unit and 4 Postgres tests, 4 stubbed Playwright tests with axe, 1 real-Keycloak test; hazards H-018 and H-019. **Not built:** problems, medications, immunisations; the HAPI validation and UK Core or US Core profile check (so the acceptance on profiles is **not met**; **read-only FHIR `AllergyIntolerance` search by patient landed 2026-10-21**, hazard H-022: no read by id, no write); coded substances beyond a shape check; drug-class checks (OR-6); break-the-glass; a history screen; verification status (confirmed or not). Left open. Original:
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
  **Not built:** no handler calls the record-level `evaluate` yet (none reads a stored record); the care-relationship lookup landed afterwards (administrator-recorded only, see `relationships.rs`); no break-the-glass store or masking (SE-3, SE-4); no ABAC on location or encounter beyond a relationship value; the permission table, label lists and purpose rules are unreviewed by a clinical safety officer or privacy officer and need that before real use; authorization is off when login is off (development only); 401s are not audited.
- [ ] **SE-2 (M) — Consent.** **Partly landed 2026-10-21 (identity only, native REST).** `POST`/`GET /api/patients/{id}/consents` and `PUT …/{consent_id}`; append-only `consent_versions`; `consent.rs` reduces a patient's current active consents to the engine's consent value (deny wins, a permit must name every label so a general permit never unmasks `SUD`, unreadable denies); applied to identity reads (403, audited with consent ids), search (hidden results dropped) and the duplicate check (a candidate the registrar may not see is returned as `restricted`). 19 unit tests (each core rule fails under a mutation), 5 Postgres tests, and a by-hand run against Keycloak and the API as clinician, receptionist and clinical admin. **Not built, so the acceptance is NOT met:** "bulk export honours consent" (no export exists); any clinical record for a denial to hide (allergies now exist and consent applies to them; nothing else yet); the national data opt-out *service* (the local record exists, but `research` is permitted to no role, so it changes nothing today); the UI sending a purpose of use, so patients can use it; FHIR `Consent`; (**the consent and relationship page landed 2026-10-21**, see below); automated handler-level tests of the denial paths (the by-hand run is the only check of the controller wiring); an emergency override (today a refusal stands even in an emergency, which needs a privacy-officer decision). **Page landed 2026-10-21:** `/patients/[id]/access` lists and edits the care relationships and the consent of a patient: record or end a relationship, record a deny or permit (roles, purposes, kinds of record, labels, dates) or the local national data opt-out, withdraw or retract a consent; roles that may not are told so. 3 vitest, 2 stubbed Playwright tests with axe, and a real-Keycloak test (grant 200, deny 403, withdraw 200, end 403). **Not built for the page:** editing more than one provision at once, editing a consent in place (record a new one), a patient-facing version of the page (the patient role may manage their own consent through the API, but the page is not linked from `/my-record`), names for the people instead of token ids, a picker for people. Left open. Description: FHIR `Consent` (permit/deny provisions by
  actor, purpose, data class and period). National data opt-out for
  secondary use. 42 CFR Part 2 segmentation via the `SUD` label.
  **Acceptance:** a denied provision hides the data and audits the denial. Bulk export honours consent.
- [ ] **SE-3 (M) — Break-the-glass.** **Landed 2026-10-21 except the notification** (grant, 4 hours, reason, flagged audit rows, auditor review queue and UI; hazards H-008 and H-024; see CHANGELOG for what is not done: no real-time notification to a privacy officer, no alert or count in the UI, no masking). A mandatory reason, a time-boxed grant
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
  **Partly landed 2026-10-21** (acceptance met): `GET /api/me/patients/{id}/access-log` (patient or proxy only, purpose
  `patient-access`, 50 per page, newest first) and a section on `/my-record`, read on request. Run against real Keycloak:
  a clinician's treatment view appears to the patient as "A member of staff viewed your identity record. Purpose:
  treatment." with no staff identifier in the page; the clinician cannot read the log (403, audited). **Role and organisation landed 2026-10-21:** every audit row
  written while handling a request carries `detail.actor` `{roles, organisations}` from the token, and the log shows
  "A member of staff (clinician, riverside) viewed your identity record." (older rows say only staff). **Not built:**
  staff names by deployment policy (none are recorded), a filter or date range, an accounting-of-disclosures export, any
  alert to the patient. Left open.
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
- [ ] **OR-3 (L) — HL7 v2 inbound.** **Partly landed 2026-10-21 (library only):** `src/interop/hl7_adt.rs` reads ADT A01/A04/A08/A28/A31 into an identity record and builds `AA`/`AE`/`AR` acknowledgements; 9 unit tests. **MLLP listener landed 2026-10-21** (`src/mllp.rs`, `src/adt_feed.rs`, `operations/hl7/README.md`): off unless `EPR_MLLP_BIND` is set, loopback unless explicitly allowed, framing with size, connection and idle limits; a message is matched on its identifiers and creates, updates or changes nothing, is audited (configured agent, peer address, no content) in one transaction with the write, and is answered `AA`/`AE`/`AR`; a possible duplicate, a different birth date or NHS number, and an ambiguous match are refused (`AE`). The integrator role may now create and update identity (policy). 8 + 7 unit tests, 4 TCP-and-Postgres tests (with a pool of one connection), and a by-hand run against the running service (created, unchanged, corrected, refused, unsupported, audited). Hazard H-021. **Work queue landed 2026-10-21** (`feed_refusals`, `/api/feed/refusals`, administrator only; a refused message is kept until a person closes it once with an outcome and a reason; hazard H-023; fuzz tests on the parser and framing). **Queue screen landed 2026-10-21** (`/admin/feed`; tested against the real stack). **Not built:** an alert for the queue and a count outside the admin pages (a refusal nobody opens is silent; the open count shows on the admin pages), re-apply, `A03`/`A40`, `PV1`, ORU, SIU, mapping to FHIR beyond the identity record, TLS and authentication, other character sets, a sample corpus of real messages (the parser acceptance is **not met**), a conformance run with a real sender. Left open. Original description: An MLLP listener plus a parser for
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
  **Partly landed 2026-10-21:** `governance/clinical-safety/hazard-log.csv` (15 hazards for what exists, 7 open, none
  accepted or closed), unsigned plan and safety-case templates, `scripts/check-hazard-log.py` (with 9 unit tests, in
  the docs job) and `scripts/check-clinical-impact.sh` plus a `hazard-log` job for `clinical-impact` PRs. **Not built:**
  a named CSO (OQ-2), any review of the ratings (the author's proposals, not validated), a safety case report. The
  workflow has never run on GitHub (no remote); the script was run locally. Left open.
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

---

## Research 2026-10-21 — code quality and new capabilities

From [plan.md §14](plan.md#14-research-pass-2026-10-21-code-quality-and-new-capabilities). The code was read, not run,
except for `pnpm run check` and a throwaway matcher probe; each defect below must be reproduced by a failing test
before it is fixed. Items marked **clinical** change what a clinician sees or what the matcher decides: they need a
hazard-log row (AS-1) in the same commit. Keep each entry to description, acceptance and status; put history in
`CHANGELOG.md`. `scripts/generate-status.sh` counts the `CQ` prefix.

### Patient-safety defects (do first)

- [x] **CQ-1 (S) — Matcher symmetry.** **Landed 2026-10-21:** equal-length given names are averaged both ways and the lower kept; a regression test and a 122,500-pair grid test (not proptest: that is CQ-30), H-001 updated. **Clinical.** `given_similarity` (`src/engine.rs:309`) averages from `a` when both
  given-name lists have equal length, so `compare(a, b)` and `compare(b, a)` can reach different decisions.
  Score both directions and take the minimum (or a symmetric assignment).
  **Acceptance:** a proptest asserts `compare(a, b) == compare(b, a)` over generated patients with valid dates; the
  `["John","J"]` vs `["John","Smith"]` case is a named regression test.
- [x] **CQ-2 (S) — Placeholder NHS numbers.** **Landed 2026-10-21** (repeated-digit numbers invalid; shared NHS number with both names disagreeing goes to Review; 4 matcher tests; the service on matcher 0.2.0 refuses a placeholder at registration with its own message, 1 test; H-001). **Clinical.** `is_valid_nhs_number` (`src/nhs.rs:9`) accepts
  `0000000000`, `1111111111`, `9999999999` and other repeated-digit numbers. Reject them (and any configured
  placeholder list) as identifiers that can decide a link; add a conflict when a shared NHS number comes with very
  different names. Keep the 999 test range valid for demo data (AS-8).
  **Acceptance:** two different people carrying `0000000000` and the same birth date are not AutoLinked (test).
- [x] **CQ-3 (S) — NHS number plus different birth date.** **Landed 2026-10-21** in the matcher (threshold removed, 1 test over 4 dates, H-001); in the service from matcher 0.2.0. **Clinical.** `src/engine.rs:180` adds the birth-date
  conflict only when similarity is below 0.5; spec §4 says any two different full dates go to Review. Drop the
  threshold (or change the spec and record why).
  **Acceptance:** shared NHS number with `1980-01-01` vs `1980-01-28` gives Review with a conflict (test).
- [x] **CQ-4 (S) — Birth-date typo rule and impossible dates.** **Landed 2026-10-21** in the matcher (digit-slip rule, calendar check, 2 tests, H-001); in the service from matcher 0.2.0. **Clinical.** Give typo credit only for a one-digit
  edit or an adjacent-digit or day/month swap (`src/engine.rs:350`); reject impossible dates such as `2020-02-31`
  (`src/normalize.rs:55`).
  **Acceptance:** 1950 vs 2010 scores as a mismatch; `2020-02-31` is absent with a reason; each branch has a test.
- [x] **CQ-5 (S) — Empty given name is absent, not zero.** **Landed 2026-10-21** in the matcher (1 test, H-001); in the service from matcher 0.2.0. **Clinical.** Filter tokens whose fold is empty in
  `given_similarity`, as `family_similarity` already does.
  **Acceptance:** given `["-"]` yields `None` for the given-name field (test).
- [x] **CQ-6 (S) — Wrong-patient display on navigation.** **Landed 2026-10-21** for `/patients/[id]` (state cleared on change, stale answers dropped, 2 Playwright tests, H-016). **Not built:** the same guard on `/patients/[id]/access` and `/my-record`. **Clinical.** `patients/[id]/+page.svelte:41` keeps the
  previous patient's banner and allergies until the next response arrives, and a late response can overwrite the
  current patient. Clear state when `id` changes and drop any response whose id is no longer current (as
  `PatientBanner` does for the photo); same for `reloadAllergies` and `look()`.
  **Acceptance:** a Playwright test with a delayed route for patient A, navigating to B, never shows A's name or
  allergies under B's URL.
- [x] **CQ-7 (M) — Duplicate-creation race.** **Landed 2026-10-21:** one registration lock for every patient creation (registration and HL7 ADT), check on the transaction; a Postgres race test, and the handler checked by hand under a one-connection pool (no handler-level test yet: CQ-16); H-002. **Clinical.** The duplicate check (`controllers/patients.rs:473`,
  `adt_feed.rs:272`) runs outside the creating transaction. Inside the transaction take
  `pg_advisory_xact_lock` per identifier (or per family name and birth date when there is none), re-run the
  candidate search on the transaction, then insert.
  **Acceptance:** a DB-gated test fires two concurrent creates for the same person and gets one record plus one
  duplicate warning.
- [x] **CQ-36 (S/M) — The duplicate check drops candidates past 200.** **Landed 2026-10-21** (ranked by evidence before the cut; `candidates_cut` in the 409, audit rows and the page; Postgres and Playwright tests; H-002). **Not covered:** a name-only or birth-date-only match can still be cut (RC-2). **Clinical.** `patients_store::candidates`
  takes `LIMIT 200` ordered by patient id. When more than 200 records share a birth date (a large population) or a
  family name, the real duplicate can be cut, and nothing says so (found by CQ-10's test). Rank the strongest evidence
  first (a shared identifier, then family name and birth date together), and say when the list was cut.
  **Acceptance:** with 250 records sharing a birth date, the one with the same identifier, or the same family name and
  birth date, is still a candidate (Postgres test); a cut list is flagged in the 409.
- [ ] **CQ-8 (S) — Background poll keeps the session alive.** **Landed 2026-10-21 except the end-to-end test** (background marker honoured by hook and proxy, warning with Stay signed in, 6 unit tests, H-025). **Open until** a real-Keycloak test sees the warning. The 60-second `look()` poll refreshes `lastSeen`
  (`src/lib/server/session.ts:72`), so the 15-minute idle timeout never fires on an open page. Mark background
  requests so the BFF does not count them as activity, and warn before the idle timeout.
  **Acceptance:** a unit test shows a background request does not extend the session; an e2e test sees the
  warning.

### Service

- [x] **CQ-9 (S/M) — Typed ids and indexes.** **Landed 2026-10-21:** 14 queries bind `$n::uuid` (`ids::uuid`, source census); migrations 11 and 12 turn the `*_current` views filtered off their key into anti-joins and index identifiers and birth date; equivalence and plan tests. **Moved to RC-2:** a folded family-name index for name search and the duplicate check. Parse ids to `Uuid` and bind `$1::uuid` instead of
  `patient_id::text = lower($1)` (11 queries). Add search indexes on identifiers, birth date and folded family name;
  review the `DISTINCT ON` `*_current` views.
  **Acceptance:** `EXPLAIN` of a patient read and a search uses an index (DB-gated test asserting no seq scan on a
  seeded table); a malformed id returns 404 without a query.
- [x] **CQ-10 (M) — Batch consent checks.** **Landed 2026-10-21** (`sources_of_many` and `active_with_merged`: two queries; equivalence test against the old method; not counted by a test). Replace the per-patient `links::sources_of` + `active_for` loop
  (`controllers/patients.rs:180`) with one query over `uuid[]`.
  **Acceptance:** a 50-result search issues a constant number of queries (counted in a DB-gated test), with the
  same verdicts as before.
- [x] **CQ-11 (M) — `ApiError` and a shared controller module.** **Landed 2026-10-22** (`api_error.rs`; `controllers/common.rs`; gate and 401 by path; error census; Postgres gate test). **Not done:** handlers on `?` (CQ-13). Move the 14 shared helpers out of
  `controllers/patients.rs` into `controllers/common.rs`; add an `ApiError` enum implementing `IntoResponse` so
  handlers use `?` and every body has one shape. Make the route gate's refusal path-aware (an `OperationOutcome`
  on `/fhir/R4`, JSON on `/api`, not `text/plain`).
  **Acceptance:** no handler builds `json!({"error": …})` by hand (grep census test); the FHIR 403 is an
  `OperationOutcome` (test).
- [x] **CQ-12 (M) — Store helpers and typed conflicts.** **Landed 2026-10-22** (`store.rs`: `pg`, `db_err` naming the store, `is_unique_violation`, `append_version`; race tests for patients and consents; census). **Not done:** `Db` keeps a string; race tested in 2 of 6 stores. One `append_version` helper for the six stores; keep the
  `DbErr` cause instead of `Db(String)`; detect version conflicts by `SqlErr::UniqueConstraintViolation`, not by
  matching `"_pkey"` in the message.
  **Acceptance:** a store test forces a conflict in each store and gets `Conflict`; the error log names the store.
- [ ] **CQ-13 (L) — Typed service layer under JSON and FHIR.** Split each handler into a service function returning
  `Result<Domain, ApiError>`, rendered by the JSON and FHIR controllers; remove `read_back`
  (`controllers/fhir.rs:69`) and its `unwrap_or_default` fallbacks. Depends on CQ-11 and CQ-16.
  **Acceptance:** `fhir.rs` no longer calls JSON handlers; FHIR tests unchanged and green.
- [x] **CQ-14 (S) — Fail loudly on bad input and export.** **Landed 2026-10-23** (see CHANGELOG). A malformed `AuditEvent` query (`fhir.rs:472`) returns
  400 `OperationOutcome`, not an unfiltered search; the NDJSON audit export (`controllers/audit.rs:269`) fails
  instead of dropping a row; invalid `EPR_PASETO_KEYS` JSON fails startup (`auth.rs:110`); document or separate the
  CSV formula guard so the exported value still matches its digest; `merged_into` past `MAX_DEPTH` is an error.
  **Acceptance:** one test per path.
- [x] **CQ-15 (S) — No delete path in down migrations.** **Landed 2026-10-23** (see CHANGELOG). `down()` drops clinical tables (patient, consent,
  relationship, allergy, photo, link, audit). Make it refuse for clinical tables, or allow it only in development
  behind an explicit check.
  **Acceptance:** a test runs `down` with a production environment and gets an error; the tables remain.
- [ ] **CQ-16 (M/L) — Handler-level tests.** DB-gated `tests/http_*.rs` that drive `controllers::all_routes()` with
  `loco_rs::testing` and PASETO test keys: consent masking of duplicate candidates, 409 shapes, the break-glass
  flag, FHIR mapping of refusals.
  **Acceptance:** each path listed as by-hand-only in `spec/index.md` has an automated test, or is listed as still
  untested.
- [ ] **CQ-17 (M) — Observability.** tower-http `TraceLayer`, `x-request-id` carried into the audit `detail`, and
  Prometheus counters for audit-append failures, unaudited refusals and refused feed messages.
  **Acceptance:** a test sees the request id in an audit row; `/metrics` exposes the counters.
- [ ] **CQ-18 (L) — Audit chain throughput.** One global advisory lock (`audit.rs:25`) is held until the outer
  commit, so audited requests run one at a time. Measure it (AS-6), then decide: a shorter critical section, or a
  chain per partition with a periodic anchor. Spec change first.
  **Acceptance:** the measured audited-write rate is recorded in `BENCHMARKS.md` with the decision.

### Matcher

- [ ] **CQ-19 (M/L) — Real-world names.** **Clinical.**
  - Per-token family matching, so `Smith-Jones` does not match `Smithson`.
  - Stop the average over the shorter given-name list from auto-linking `John` with `John Paul` (a twins risk).
  - A nickname table.
  - A phonetic signal, kept separate from string similarity.
  - Transliteration (`Müller`/`Mueller`).
  - Given/family swap detection.
  - A `\p{M}`-aware fold, so Indic and Persian names are not split.
  - E.164 phone normalisation.
  - Shared phone or postcode counted as weak household evidence.
  - `enum` codes for fields and conflicts instead of English strings.
  - `MatchConfig::validate()`.

  **Acceptance:** each rule has a named test, and the ID-1 labelled corpus shows no increase in false AutoLinks.
- [ ] **CQ-30 (M) — Matcher test depth.** proptest for symmetry, totality and "removing a field never raises the
  decision"; cargo-mutants on `engine.rs` and `nhs.rs`; a `cargo fuzz` target (closes part of SF-2).
  **Acceptance:** surviving mutants are fixed or listed with a reason.

### CI, release and documents

- [ ] **CQ-20 (S) — Run CI for the first time.** Add a remote, push, and get `ci-ok` green; confirm every action
  version exists (`checkout@v7`, `upload-artifact@v7`, `download-artifact@v8`, `attest-build-provenance@v4`).
  **Acceptance:** a green run linked from `CHANGELOG.md`; SF-5 and SF-9 updated.
- [x] **CQ-21 (S/M) — Hazard-log gate.** **Landed 2026-10-21** (label trigger; `--base/--author` sign-off check against a base-branch allow-list, empty until a CSO is named; no row removed; ISO dates; CODEOWNERS entry; 8 checker tests). **Not run on GitHub** (CQ-20). Add `types: [opened, synchronize, reopened, labeled, unlabeled]` to the
  `pull_request` trigger (ci.yml:9); fail a PR that moves a hazard to `accepted` or `closed` unless its author is on
  a Clinical Safety Officer allow-list; a CODEOWNERS entry for `governance/clinical-safety/`; validate ISO dates.
  **Acceptance:** fixture tests for `check-hazard-log.py` cover each rule.
- [ ] **CQ-22 (S/M) — Release the UI image.** `release.yml` builds and pushes only the API image; add the UI with
  provenance; set `org.opencontainers.image.source`.
  **Acceptance:** a dry-run release produces both images.
- [ ] **CQ-23 (M) — Supply chain.** Pin actions by SHA (Dependabot keeps them current), pin base images by digest,
  add Dependabot `docker` entries, pin one pnpm version for CI and the Containerfile; give each `deny.toml` ignore a
  crate, reason and re-check date, and set unknown sources to `deny`.
  **Acceptance:** a CI check fails on an unpinned `uses:` line.
- [x] **CQ-24 (S) — Test selection.** **Landed 2026-10-21** (`cargo test --workspace --locked`; `--ignored` over the workspace with `--no-fail-fast`; init SQL applied; both commands run locally). **Not run on GitHub** (CQ-20). The `api` job hard-codes `--test` names (ci.yml:63), so a new non-ignored test
  file runs nowhere; use `cargo test --workspace --locked` and keep the `--ignored` job. Apply `ci/postgres-init` in
  the CI Postgres.
  **Acceptance:** a new test file runs in CI without a workflow edit.
- [ ] **CQ-25 (M) — Missing gates.** Adds the gates that SF-5 and SE-7 call for. Each lands as its own job, and each job is a commit:
  - an MSRV check (`cargo +1.97 check`);
  - a ZAP baseline;
  - Trivy or Grype on the images;
  - an SBOM attestation and a cosign signature;
  - `pnpm audit` or OSV;
  - coverage reports (`cargo llvm-cov`, vitest);
  - a weekly cargo-mutants run on the matcher and the policy engine;
  - CodeQL and secret scanning.

  **Acceptance:** all eight jobs exist and are green.
- [ ] **CQ-26 (L) — Real-stack end-to-end.** A CI job runs compose with the Keycloak and UI profiles and a small
  Playwright suite that does not stub the API.
  - It covers sign-in, the BFF proxy, the CSP and one patient read.
  - Playwright uses `127.0.0.1`, sets `retries` so traces are captured, and uploads its report when a test fails.

  **Acceptance:** the job is green, and a deliberately broken proxy makes it fail.
- [ ] **CQ-27 (S/M) — Container and compose hardening.** Hardens the containers and compose services:
  - `read_only`, `cap_drop: [ALL]`, `no-new-privileges` and tmpfs for api, ui and migrate;
  - an explicit UID for the UI image;
  - the test Postgres and the API bound to `127.0.0.1`;
  - a health check without `curl` in the runtime image.

  **Acceptance:** the stack boots under compose with these settings, and `/_health` returns 200.
- [ ] **CQ-28 (S) — Stale documents.** Brings these documents up to date:
  - the hand-written area table in `spec/14-implementation-status.md`: generate it or delete it;
  - the UI `AGENTS.md` ("patient pages not built");
  - the `compose.yaml` break-the-glass comment;
  - the `compose.test.yaml` `--test mod` instruction and its `mxi-` container names;
  - the per-crate changelogs, which release notes read and which stop at 0.1.0, or else point release notes at the root changelog;
  - the plan.md header ("no code exists").

  **Acceptance:** `check-links.py` passes and a reviewer finds no contradiction among the files listed.
- [ ] **CQ-29 (S) — Status generator and tasks hygiene.** **Generator part landed 2026-10-21** (`CQ` counted; an unknown prefix fails the generator). **Open:** trimming the long entries, each checked against its changelog entry so no "Not built" item is lost. Add `CQ` to the prefix loop in
  `scripts/generate-status.sh:72`; trim the long inline status logs in tasks.md (ID-3, ID-4, SE-2, OR-3 and others,
  up to 2,266 characters each) to a status line plus a CHANGELOG link; optionally warn on very long lines.
  **Acceptance:** `generate-status.sh --check` passes and counts CQ.

### UI

- [ ] **CQ-31 (S) — Proxy and session resilience.** Adds `AbortSignal.timeout` to the upstream call
  (`api/proxy/[...path]/+server.ts:43`).
  - Transport errors map to 502 or 504 JSON.
  - A session is kept on a transient refresh failure (`auth.ts:156`); it is deleted only on `invalid_grant`.
  - The proxy forwards `Accepts-version`.
  - `X-Forwarded-Proto` is trusted only behind a configured proxy.

  **Acceptance:** each case has a unit test of the handler.
- [ ] **CQ-32 (M) — Decoded responses and one request core.** Merge `call` and `sendBytes` (`src/lib/api.ts`), decode
  each response against a schema instead of `as T`, and move the compare page onto it.
  - One `handle(res, {ok, conflict, invalid})` helper replaces the status chains repeated on each page.

  **Acceptance:** no `as T` on response bodies (grep census test); a malformed response shows an error, not a blank
  field.
- [ ] **CQ-33 (S) — In-flight and accessibility states.** Disable save, upload and withdraw while busy; give the loading
  text `role="status"` and `aria-busy`; replace physical `left`/`right` CSS with logical properties for `dir="rtl"`.
  **Acceptance:** an e2e double-click test makes one request; axe passes with `ar-001`.
- [ ] **CQ-34 (M) — Translation readiness and local time.** **Clinical** (times). A message catalogue keyed by locale
  with ICU plurals and placeholders instead of `STRINGS` and `.replace`.
  - Times are shown with `Intl.DateTimeFormat` in local time, with the zone.
  - Error codes come from the API, not raw English.

  Translations stay marked machine-written.
  **Acceptance:** a summer timestamp shows BST in `en-GB` (test).
- [ ] **CQ-35 (M) — Component and server tests.** Component tests for the patient page and banner, plus tests for the
  proxy handler and `hooks.server.ts`; split the photo and edit form out of `[id]/+page.svelte`.
  **Acceptance:** each new test fails under a deliberate mutation of the code it covers.

### New capabilities

These are ranked in plan.md §14.3. Where a source is secondary only, the item says **[uncertain]**, and AS-11
confirms it before anything depends on it.

- [ ] **RC-10 (L) — Medication reconciliation.** **Clinical.** On admission, transfer and discharge, show the external
  list (GP Connect, IPS or CCD) beside the local one.
  - Each line is marked continue, stop or modify, with who decided, when and why (Provenance).
  - It follows NHS DAPB4013 for the UK.
  - It depends on RC-5, CA-1 and RC-3.

  **Acceptance:** an e2e test reconciles a three-line list. Every decision is in the audit and in Provenance, and
  an unreconciled line blocks "complete".
- [ ] **AU-3 (M) — FHIR `AuditEvent` on IHE BALP.** `/fhir/R4/AuditEvent` read and search, profiled on BALP.
  **Acceptance:** responses validate against BALP profiles in CI.
- [ ] **ID-8 (M) — IHE PDQm and PIXm.** PDQm search and `$match`, and PIXm `$ihe-pix` over the matcher and the link
  table (ID-5, ID-6).
  **Acceptance:** the IHE Gazelle or Touchstone test scripts pass, or the gaps are listed.
- [ ] **SE-10 (S/M) — Passkeys and fast user switching.** A Keycloak WebAuthn policy for clinicians, with tap-to-unlock
  user switching on a shared workstation and an AAL mapping in the spec. NIST SP 800-63-4 wording is
  **[uncertain]**.
  **Acceptance:** a real-Keycloak test signs in with a virtual authenticator, and switching user ends the
  previous session.
- [ ] **DO-7 (M) — SDC adaptive forms.** `$populate` from the chart, `$extract` to Observations, and
  `$next-question`, on SDC STU4. This scopes DO-4.
  **Acceptance:** a form prefills from the record, and `$extract` writes Observations with Provenance.
- [ ] **DO-8 (M) — Ambient scribe governance.** **Clinical.** A capture-consent record, and a required review with the
  transcript beside the draft.
  - Every AI-drafted note carries agent Provenance, and a DCB0160 deployer template is included.
  - It builds on DO-6. The current NHS England and MHRA guidance version is **[uncertain]**.

  **Acceptance:** a note drafted by an agent cannot be signed until its review is recorded (test).
- [ ] **SE-11 (M) — AI agent / MCP access boundary.** **Clinical.** Agents are their own principal type, with per-tool
  scopes.
  - Every call passes the policy engine, consent and audit.
  - A FHIR-scoped MCP server is read-only first.

  **Acceptance:** an agent without the scope gets 403 (audited), and an agent read is shown in the patient
  access log as an agent.
- [ ] **IO-9 (L) — NHS national APIs.** Split into one item per API before it is scheduled: PDS FHIR (with ID-2), GP
  Connect Access Record Structured (STU3 adapter), NRL and BaRS (with CA-3). Current NRL and BaRS versions are
  **[uncertain]**.
  **Acceptance:** each sub-item passes the NHS sandbox conformance tests.
- [ ] **IO-10 (M) — SMART Health Links and Cards.** The portal issues an encrypted link or QR code to the patient's IPS,
  and registration can scan one into the duplicate check (ID-4).
  **Acceptance:** a link round-trips, expires on time, and every access is audited.
- [ ] **IO-11 (S/M) — European Patient Summary.** Validate `$summary` against IPS 2.0 and HL7 Europe EPS, ahead of
  EHDS. Dates are **[uncertain]**.
  **Acceptance:** the validator passes in CI on demo patients.
- [ ] **SE-12 (S then M) — Post-quantum readiness.** A crypto inventory (audit MAC, PASETO, TLS, at-rest keys) and an
  algorithm identifier on every signed or MACed artefact, so it can be migrated later, following the NCSC 2028/2031/2035 timeline.
  **Acceptance:** the inventory is in `spec/`, and the audit chain verifies across an algorithm change in a test.
- [ ] **RC-11 (M) — Patient-generated and device observations.** **Clinical.** Readings from the portal or a device
  gateway are tagged by source. They stay out of flowsheets and early-warning scores until a clinician accepts them.
  **Acceptance:** an unaccepted reading never changes NEWS2 (test).
- [ ] **AS-12 (S) — Accessibility conformance evidence.** A manual screen-reader audit (NVDA, VoiceOver), an ACR or
  VPAT, and the accessibility statement for the portal. The HHS Section 504 dates are **[uncertain]**.
  **Acceptance:** the report is in `governance/` with its remediation list.
- [ ] **OR-8 (L) — Electronic prior authorisation (US).** Da Vinci CRD through CDS Hooks `order-sign`, DTR on DO-7, and
  PAS submission. US only (OQ-4), and the dates are **[uncertain]**.
  **Acceptance:** the Da Vinci reference implementations accept a round trip.
- [ ] **IO-12 (M) — Single Patient Record feed (England).** Watch item until a specification exists, then an outbound
  feed through IO-4 and IO-6. Bill status is **[uncertain]**.
  **Acceptance:** an OQ entry records the decision. Build only once a specification is published.
- [ ] **IO-13 (L) — TEFCA via Facilitated FHIR (US).** Participant or subparticipant query for Treatment and IAS, using
  IO-2 with UDAP. The SOP text is **[uncertain]**.
  **Acceptance:** a connection test against a QHIN sandbox.
- [ ] **OR-9 (L) — UK EPS FHIR Prescribing (post-1.0).** Prescribe, cancel and sign through the EPS Prescribing API.
  It needs CIS2 high assurance. §7.2 keeps it out of 1.0.
  **Acceptance:** EPS sandbox conformance.
- [ ] **IO-14 (M) — Digital quality measures (post-1.0).** CQL on QI-Core, run over SQL-on-FHIR (IO-7) or Bulk
  export (IO-4) and never against the clinical database. The CMS dates are **[uncertain]**.
  **Acceptance:** one published dQM gives the expected counts on demo data.
- [ ] **CA-8 (M) — Offline-first entry spike.** A client outbox of append-only writes, merged with conflict review by
  Provenance. This resolves OQ-5.
  **Acceptance:** a written spike result, with a decision recorded in `spec/16-open-questions.md`.
- [ ] **RC-12 (M) — Genomics reporting (post-1.0).** FHIR Genomics Reporting STU3 results in the results inbox (OR-4).
  **Acceptance:** an example report renders, and its variants are searchable.
- [ ] **IO-15 (S) — FHIR R6 watch.** A spec note that keeps storage version-neutral, revisited when R6 is final.
  **Acceptance:** the note is in `spec/09-api-surface.md`, with a re-check date.
