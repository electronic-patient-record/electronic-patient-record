# Changelog

Each entry says what was not done or not verified. Per-crate changelogs hold the detail.

## 2026-10-29 - Service: read-only FHIR R5 facade (IO-16)

- **Built:** `controllers/fhir_r5.rs` and five routes under `/fhir/R5` (`metadata`, `Patient`, `Patient/{id}`, `AllergyIntolerance`, `Observation`), each with a policy row. A route calls its R4 handler and converts the body with `fhir::convert::between::<R4, R5>` (`fhir` gets the `r5` feature); any loss or warning on anything but an `OperationOutcome` is a 500 outcome that names no value; `reaction.manifestation` is mapped by hand to a `CodeableReference`; a search set gets a `self` link; links are rebased to `/fhir/R5`; an `Accept` with another `fhirVersion` is 406. H-037 added (open, unreviewed).
- **Found by running it:** the converter reported `reaction.manifestation.text` as dropped for every allergy with a reaction, so without the mapping those allergies would have failed; the HL7 validator rejected a search set with no `self` link (bdl-18, an error in R5); it also needed the amendment-notice profile loaded for R5.
- **Tests:** 11 unit tests in `fhir_r5.rs` (loss in a bundle entry, planted field, `ETag` kept, self link, `Accept`); 5 in `fhir.rs` converting the bodies the service really writes (allergy with reaction, no-known and not-recorded warnings, observation value, unit, range and amendment notice, merged patient); `tests/fhir_validator.rs` adds an R5 acceptance run and an R5 control that must fail; `tests/http_observations.rs` drives the R5 routes with real roles (refusals, 406, metadata, no write route). Route census passes with the five new rows. Validator, database tests and the database-free suite were run locally.
- **Not done / not verified:** IO-17 (the R4 to R5 differences were not read from the specification; only the converter's report and the validator stand behind the output); a change that keeps the shape but alters the meaning would pass; AuditEvent, Consent and Provenance are not served in R5; no R5 write; the `fhirVersion` rule is on R5 only, not R4; no real partner or PAS has read it (PA-9 not started); the allergy and amendment-notice paths were converted in tests, not shown over HTTP with a real login; no clinician has reviewed H-037; the validator ran without a terminology service, so codes were not checked.

## 2026-10-28 - Spec: FHIR R5 as a derived read-only facade (IO-16, IO-17 planned)

- `spec/interoperability/index.md` gains "FHIR R5": R4 stays the stored baseline and R5 is derived from it at `/fhir/R5`, reads only, base R5 with no UK Core or US Core claim, a loss report from the `fhir` crate's `convert::between::<R4, R5>` that fails a clinical response, the same policy, audit and amendment notice as R4. plan.md §21 ranks seven ideas; tasks IO-16 and IO-17 added; OQ-PA3 points at them.
- **Checked:** the `fhir` 4.2.4 crate in the cargo registry has an `r5` feature and a `convert` module with a loss report. The project builds with `r4` only today.
- **Not done / not verified:** nothing is built, enabled or tested; the R4 to R5 field differences are from memory and marked uncertain, I did not read the HL7 pages; the `fhirVersion` negotiation rule is unchecked; one search found no R5 UK Core or US Core, which is not proof there is none; no hazard row yet, because no behaviour changed. I read "add FHIR R5" as a spec, plan and tasks change, as with ISO 14971; if you meant the facade itself, that is IO-16.

## 2026-10-28 - Service: the hazard register, only the Clinical Safety Officer retires (AS-17, in part)

- **Built:** role `clinical-safety-officer` and resource `Hazard` in the policy table; append-only `hazard_versions` and view `hazard_current` (migration 16, triggers refuse update, delete and truncate); pure rules in `hazard.rs`; `hazard_store.rs` (no delete function); `GET` and `POST /api/hazards`, `PUT /api/hazards/{hazard_id}` with `control`, `retire`, `reopen`. A retirement needs the role in the token (403 otherwise, and the check does not switch off when login is off), at least one evidence item with a kind, a reference, a past time and a SHA-256 (422 otherwise), and a person who did not record a control on that hazard. Any writer reopens with a reason. The audit row holds the evidence hashes and never the free text. The role is in the Keycloak realm and role map; the table is in the personal-data inventory and the retention schedule; hazard H-036 is added (open, unreviewed).
- **Tests:** 10 unit tests for the rules; `tests/hazards_store.rs` (history kept, update, delete and truncate refused by the database); `tests/http_hazards.rs` (clinician, receptionist, integrator refused; no evidence and a bad hash are 422; stale is 409; a body field cannot grant the role; reopen; no free text in the audit); both new write routes added to the audit-failure census. Run: fmt, clippy, the whole database-free suite (272 unit tests and every census), and every database-gated test against Postgres 18 on port 5543, all passing.
- **Not done / not verified:** nobody holds the role (OQ-2), so no real retirement can happen; the acceptance line about a per-permission census entry is not met, since the policy table is per resource and action and the retire rule is in the handler (recorded in `tasks.md`); no UI screen and `ROLES` in the UI is unchanged; only a hash and reference of the evidence is stored, not the evidence; nothing re-checks evidence later, links a hazard to a gate or ships a deployer baseline (AS-18); the hash shows the evidence did not change, not that it is true; the repository file items AS-13 to AS-15 are untouched; CI has not run on this commit; the Playwright and UI gates were not run since no UI file changed.

## 2026-10-28 - Spec: the hazard rule inside the product (AS-17, AS-18 planned)

- `spec/hazard/index.md` gains "In the EPR system itself": only a `clinical-safety-officer` role can retire a hazard, with evidence (kind, reference, time, SHA-256) or a 422; an append-only event table; anyone may reopen; every event audited. It separates the manufacturer's log file from a deployer's in-product register. plan.md §20.4b ranks seven ideas; tasks AS-17 and AS-18 added.
- **Checked in code:** there is no such role, table or route today; `Role` has ten variants and none is this one.
- **Not done / not verified:** nothing is built or enforced; the DCB0129 versus DCB0160 split is from memory and marked uncertain; I did not read either standard; no hazard row was added because no behaviour changed; no CSO is named (OQ-2).

## 2026-10-28 - Spec: ISO 14971 mapping (AS-16 planned)

- `spec/hazard/index.md` gains a table mapping ISO 14971 activities to the hazard log and risk plan, with the gap for each; plan.md §20.4a and task AS-16 follow. Main finding: the risk plan sets no risk-acceptability criteria.
- **Not done / not verified:** I did not read ISO 14971 or ISO/TR 24971 (paywalled); the clauses are from memory and marked uncertain. No code, CSV or hazard change, no conformity claim, and no decision that any module is a device.

## 2026-10-28 - Spec: who may remove a hazard, and the evidence (AS-13 to AS-15 planned)

- New [`spec/hazard/index.md`](spec/hazard/index.md): only the Clinical Safety Officer removes a hazard and must give evidence; "remove" means retire (the row stays, as the checker already requires); anyone may reopen. It lists what counts as evidence, what the checker enforces today, seven weaknesses with advice, and plan.md §20 ranks the ideas. Tasks AS-13 (closure columns, push guard), AS-14 (signed closing commit) and AS-15 (evidence that does not rot) are added.
- **Said plainly:** the request said "chief safety officer"; the repository and DCB0129 call the role Clinical Safety Officer, so the spec does. I read two vendor guides, not the NHS England standard, so closure criteria and retention are marked uncertain.
- **Not done:** no code or CSV change, so none of the new rules is enforced; no CSO is named (OQ-2); no hazard status changed; the gap that a direct push to `main` skips the sign-off job is recorded, not fixed (branch protection is the owner's setting).

## 2026-10-28 - CI: fixed seed for the hostile-input sweep (PA-12, in part)

- The CI database-test step sets `EPR_CHAOS_SEED=20261028`, so a failure repeats; local runs stay random. Planted check: a temporary `panic!` in `no_nul_mw` on targets containing `%2` made `chaos_api` fail (300 iterations, seed 20261028); reverted, nothing of it is committed.
- **Found, not fixed:** a first plant keyed on `%25` (a literal `%`) was NOT caught in 2000 iterations, because the generator never sends a bare `%`. So the sweep's reach is limited by its junk list; I did not widen it (that changes every seeded sequence).
- **Not done / not verified:** PA-12 stays open (login is off in the test); the fixed seed has not yet run in CI; no hazard-log change.

## 2026-10-28 - Spec: link to the MHRA

- `spec/12-compliance.md` now links the MHRA's page on GOV.UK where it names the Software as a Medical Device boundary. Wording only.
- **Not done / not verified:** I did not open the link or check what the MHRA currently says; the spec still makes no claim that this software is or is not a medical device, and `plan.md` and the hazard log are unchanged. If you meant it to go somewhere else (the README, `llms.txt`, a sources list), say where.

## 2026-10-28 - CI: type-check the Playwright specs

- The CI UI job now runs `pnpm run check:tests` (after `check`), and the gates in `AGENTS.md` list it. It passed locally before the change. This closes the "`check:tests` is not in CI" gap in the entry below.
- **Not done / not verified:** the specs' strictness is still lower than `src` (no `noUncheckedIndexedAccess`); the step has run only locally, not yet in CI; no product code changed, so no hazard-log change.

## 2026-10-28 - UI and repo: no `any` in tests, a link guard, a pre-commit hook

- **`any` removed from the tests** (35 uses; the ESLint exception for `tests/` is gone). Request bodies the tests read back are typed `Json` (`tests/e2e/json.ts`); the Keycloak spec has `Api`, `AuditEvent` and `Grant` types. New `pnpm run check:tests` type-checks the Playwright specs, which nothing did before: it needed `noUncheckedIndexedAccess` off (in `tsconfig.tests.json`) so deep reads of a `Json` body compile, and it passes. It surfaced one real type error (a `Buffer` as a `fetch` body), fixed.
- **Links:** `svelte/no-navigation-without-resolve` stays off (it knows only SvelteKit's `resolve`), but `tests/unit/links-use-localize-path.test.ts` now fails when an `href`, `goto` or `redirect` to a page of this app skips `localizePath` (allowed: `#anchors`, `/auth/…`, the identity-provider redirect). Checked by planting `href="/patients"`: it failed.
- **Pre-commit hook:** `.githooks/pre-commit` runs only what the staged files need (UI lint, `cargo fmt --check`, the generated status and `llms` files, links); `sh scripts/install-hooks.sh` sets `core.hooksPath` for the clone (local config). I installed it in this clone. First real use caught a stale `spec/14-implementation-status.md`; a badly formatted file was blocked.
- **Not done:** the hook is opt-in per clone (git cannot enforce it; CI is the real gate); it does not run clippy or any tests, to stay fast; `check:tests` is not in CI yet and the specs' own strictness is lower than `src` (no `noUncheckedIndexedAccess`); `Json` makes a wrong nested name a value, not an error; the link guard reads lines, so a link built across several lines in an unusual way could slip past it. Gates run: lint, svelte-check, check:tests, vitest, Playwright.

## 2026-10-28 - UI: ESLint and Prettier checks, in CI

- Added ESLint 10 (flat config: typescript-eslint, eslint-plugin-svelte, eslint-config-prettier) and a Prettier config with the Svelte plugin. `pnpm run lint` runs `prettier --check .` and `eslint .`, and is now a step of the CI UI job and of the gates in `AGENTS.md`. Prettier covers code and config; Markdown is ignored (the specs are hand-wrapped).
- The first run found 68 problems. Fixed in code: seven unused `event` arguments, a `let` that should be `const`, a useless assignment, an unused import, an unused variable. Kept with a written reason: one control-character regex that exists to refuse them, one `$effect` that reads storage after hydration on purpose, one query string built once. Turned off in `eslint.config.js`, each with its reason: `no-explicit-any` under `tests/` (35 stubbed JSON bodies), empty interfaces in `app.d.ts` (SvelteKit's boilerplate) and `svelte/no-navigation-without-resolve` (every link goes through `localizePath`, which the rule does not know). Prettier reformatted 59 files; that is the bulk of this commit's diff and changes no behaviour.
- **Not done / not verified:** the `any` rule is off for tests, so those 35 uses are not fixed; the navigation rule is off rather than satisfied, so a link that skips `localizePath` is not caught by lint (the locale tests are the guard); no pre-commit hook; the service (Rust) was not touched. Gates run: lint, svelte-check, 227 vitest, Playwright 84 of 85 in one loaded run (the right-to-left axe test timed out under load; it passes alone, so it is slow-machine flakiness, not a fix I made).

## 2026-10-28 - Service: single-use link tokens, library only (PA-11); small-cell suppression was already here (PA-7)

- `link_token.rs` (pure, 7 tests with fake time): opaque token, SHA-256 at rest, a link that works once, expiry refused at and after its time, and a read-only `view` so a GET cannot change state. Taken from the patient-administration-system project.
- **Small-cell suppression needed no work:** `privacy.rs` (PA-7) already hides cells under k (never below 5), hides a second cell so a single hidden one cannot be subtracted from the total, limits how many tables a caller may ask for, and refuses the simplest differencing case. It has no caller. I did not add the other project's per-caller daily budget in the database or its audit of query shapes, since nothing here produces an aggregate yet.
- **Not done:** the tokens have no table, no atomic spend and no caller, so no link works yet and PA-11 stays open; no route; nothing proves two simultaneous spends cannot both win (that needs the database). PA-12 is also noted in `tasks.md`: the chaos test does not yet meet its acceptance (random seed in CI, no planted-panic proof). Gates: fmt, clippy, the whole non-database suite.

## 2026-10-28 - UI: a translation register, a rule on API error text, and axe route coverage (from the patient-administration-system project)

- `governance/translation-review.md` lists every locale with its status (all nine are machine-written, none reviewed, no reviewer) and `translation-register.test.ts` fails if a locale has no row, a status is unknown, or `reviewed` lacks a reviewer and date. The key-parity test already existed (`translations.test.ts`) and is unchanged.
- `no-api-error-text.test.ts` scans `src`: a problem list may be shown only under a 422 check, and no `error` or `message` field of an answer may be shown. A leaking 500 on the patient page shows the page's own words and none of the leak (1 Playwright test with axe). **Deliberately different from the other project:** this UI shows the service's 422 validation wording as it stands (it is fixed English written in one place), so the rule is "422 only", not "never".
- `axe-route-coverage.test.ts` fails when a page under `src/routes` is not opened by a spec that runs axe (checked by adding a dummy page: it failed). Both new rules were also tried against a deliberate violation and failed as they should.
- **Not done:** the register covers one translated surface (the access-log filter form); nothing else is translated; no speaker has reviewed anything. The coverage test shows a page is opened in an axe spec, not that axe ran in every state of it. The 422 wording is still unreviewed English. Full UI gates run: check, vitest, Playwright, docs gates.

## 2026-10-28 - Governance: personal-data inventory, retention schedule, DPIA and record of processing (from the patient-administration-system project)

- `governance/personal-data-inventory.csv` classifies all 192 database columns (category, purpose, retention class); `tests/personal_data_inventory.rs` fails when a column is unclassified or a row is stale (database test, passed against Postgres 18; the shape test needs no database). `governance/retention-schedule.md`, `governance/dpia.md` and `governance/record-of-processing.md` are drafts from the code and specs.
- **Not done, and plainly:** no retention period is set; no one has reviewed any of these documents (no data protection officer, Clinical Safety Officer or privacy lead); the controller, legal basis, Article 9 condition, hosting and processors are all left as "controller to decide"; the DPIA does not assess residual risk and records no consultation. The category of each column is my judgement (for example `audit_events.detail` is classed free-text to be cautious, and the photo as biometric). **How deletion fits an append-only record is an open question** that the schedule names and does not answer. The inventory test is not in a CI job of its own: the CI database job runs every ignored test, so it runs there. Gates: fmt, clippy, links.

## 2026-10-28 - Service: a seeded chaos test over every route (from the patient-administration-system project)

- Read `~/git/patient-administration-system/patient-administration-system` for ideas. Taken: its seeded "monkey" that hits every route with hostile input. `tests/chaos_api.rs` does this against `controllers::all_routes()` (no panic, no 5xx other than 502/503/504, no internals in a body; seeded; refuses a non-test database). **It found a real bug on the first run:** a NUL byte (`%00`) in a query string made `GET /api/patients` and `GET /api/audit` answer 500, because PostgreSQL cannot store one. Fixed by refusing such a target 400 before authentication (`authz::no_nul_mw`, 1 unit test). After the fix, two seeds of 12,000 requests each ran clean (before it, seeds 1 and 2 of 20,000 each failed).
- **Not done:** login is off in the test, so roles are not exercised; a NUL inside a JSON string body is not generated reliably (none reached the database in these runs, so it is unproven either way); no process-kill, database-stop or clock-jump chaos; the test adds about a minute to the CI database job at 2,000 requests. The CI database job runs it with a random seed (printed on failure). Gates run: fmt, clippy, the non-database suite, the chaos test against Postgres 18.
- **Other ideas in that project, not taken (each needs a decision or is large):** a personal-data inventory CSV and retention schedule under `governance/`; a record of processing and DPIA; a translation-review register with an i18n key-parity test (this repo's translations are machine-written and unreviewed); a rule that no UI page shows API error text; an a11y coverage test that fails when a route is missing from the axe list; a note on clock-dependent tests; privacy-safe analytics with small-cell suppression; single-use hashed tokens for links.

## 2026-10-28 - Service: the amendment notice coded in FHIR (RC-4, H-035)

- Each `note` the FHIR Observation and AllergyIntolerance searches carry now has a coded extension beside the English text (`Notice::fhir_annotation`): `state` (pending, amended, refused), `version` (amended) and `ground` (refused). A machine reader can tell a pending request from a refusal without reading prose. The text is unchanged. 1 new unit test, assertions added to the Observation converter test. The StructureDefinition is `fhir/StructureDefinition-amendment-notice.json`, and the HL7 validator test now loads it with `-ig`: without it the validator rejects the extension, so the check does something.
- **Not done:** the extension URL is a placeholder under `electronic-patient-record.example` that I chose, not a canonical address the project owns, and the StructureDefinition is unreviewed, not published and not served by the API (no `StructureDefinition` read). The patient's statement of disagreement is still only in the text, not coded. No UK Core or US Core profile (RC-2). Gates run: fmt, clippy, the non-database suite, and both validator tests (jar 7.0.3). **Not run locally:** the Postgres tests (`http_amendments` reads the same `note[].text`, which did not change); CI runs them. UI gates not run (no UI change).

## 2026-10-28 - UI: a first patient summary page (RC-8, H-034)

- `/patients/[id]/summary`, linked from the patient page and read-only: the banner, the allergies with their amendment notices, the newest standing reading of each vital sign with its notice, and a list of what is not built (problems, medications, immunisations, results, appointments) saying that this does not mean there is nothing to record. A newest reading with no value shows its reason; an older value is never put in its place. A refused or failed section says so. 4 vitest, 2 stubbed Playwright tests with axe. H-034 text updated (status unchanged, still `open`).
- **Not done:** everything else RC-8 asks for (the five sections, reconciliation, add and edit), so the acceptance is not met; no performance test of chart open (p95 under 1 s) and no keyboard-only test; the page was not added to the axe list in `smoke.spec.ts` (it needs a stubbed patient, so its axe check is in `patients.spec.ts`); English only, wording mine and unreviewed by a clinician; not run against the real service or Keycloak. Gates run: UI check, 220 vitest, 84 Playwright. Docs gates (links, status, llms) pass; service gates not run (no service change).

## 2026-10-27 - Service: the amendment notice in the FHIR searches (RC-4, H-035)

- `GET /fhir/R4/Observation?patient=` and `/fhir/R4/AllergyIntolerance?patient=` put each notice on the resource as an `Annotation` in `note`, in the same English as the page (`Notice::describe()`, 1 unit test); a record with no request has no `note`. A notice that cannot be read fails the search rather than being left out. Unit tests on both converters; `http_amendments` checks both searches over HTTP with login on; the validator test now includes a disputed observation and allergy.
- **Verified for the first time:** the official HL7 validator (`validator_cli.jar` 7.0.3, base R4, no terminology server) accepts every resource the service writes, the two new ones included; these two tests had been listed as not run in earlier entries and now pass. Gates run: fmt, clippy, the whole service suite against Postgres 18 with the validator jar set (ignored tests included; no failure), and the docs and hazard-log checks. UI gates not re-run (no UI change).
- Not done: the notice is English prose in `note`, not a coded extension, so a machine reader cannot tell a pending request from a refusal; no UK Core or US Core profile check (RC-2); no read by id, `$everything` or `$export` exists yet and each must carry the notice when added; still no real-login run of the patient page, no alert across patients, no proxy asking, wording unreviewed.

## 2026-10-27 - Amendments: allergies, the record named, a count of what waits (RC-4, H-035)

- Service: `GET /api/patients/{id}/allergies` returns `amendment_notices` as the observation list does (shared `amendments_store::notices_by_target`, which also builds the observation one); `http_amendments` asks about an allergy and checks the list carries the same notice. UI: a patient can choose an allergy as well as a vital sign; the allergy list shows the notice; the clinician's list names the record each request is about and says "n waiting for a decision. m overdue." Tests: 2 vitest, 1 Playwright test added and 1 changed.
- Run against the real service (not committed): the service on Postgres 18 with login off, the built UI in front of it, requests created through the API, and a throwaway Playwright script on the clinician page. It passed: the real list shapes match what the page reads, both records are named, both notices appear beside their records, and the service's own refusal ("a request cannot be decided by the person who made it") reaches the page.
- **Not done, and why:** (1) **No real-login run of the patient page.** With login off `/my-record` has no linked record, and the dev Keycloak realm has no `patient` role, so the patient screens and the clinician-versus-patient split have still only been exercised by stubbed UI tests and the service's own login-on test. (2) **No alert outside the patient page**: the count shows only to someone who opens that patient; a queue across patients needs its own access decision per patient and is not built. (3) **A proxy still cannot ask**: that needs a proxy scope for amendments, a consent and safety decision for the Clinical Safety Officer and a privacy lead, not a code default. (4) **The wording is still English only and unreviewed**; I cannot review it, and no translation exists. Gates run: fmt, clippy, the whole service suite against Postgres 18 (ignored tests included, except the two FHIR validator tests, which need `FHIR_VALIDATOR_JAR`), UI check, 216 vitest, 82 Playwright, docs gates.

## 2026-10-27 - UI: a patient asks for a reading to be corrected (RC-4, H-035)

- On `/my-record` a patient chooses one of their vital signs, says what should change, and sees their requests with the reading each is about and the reason for a refusal; they can withdraw one before a decision and add one statement of disagreement to a refusal. Sent under `patient-access`. 3 vitest, 1 Playwright test with axe (checks each body sent, the purpose, and that an empty form sends nothing).
- Not done: only vital signs can be asked about (not allergies); a proxy is told it is not available, since the service gives it no scope; the clinician's list still shows only the kind of record; the 60-day clock is shown but nothing reminds anyone; English only, wording mine and unreviewed by a patient, clinician or privacy officer; not seen with a real service or Keycloak (Playwright stubs the API), so a real patient token reading `/observations` and posting an amendment has been exercised only by the service's login-on test, not through the UI. Full UI gates run: check, 214 vitest, 81 Playwright, all pass. Service gates not re-run (no service change).

## 2026-10-27 - UI: a clinician decides requests to correct a record (RC-4, H-035)

- A section on the patient page lists the requests (patient's words as text, state, due date, "overdue" in words) and offers only the steps that fit: agree, refuse (ground and reason), record the correction (version), take the one extension (reason). Nothing is sent without its reason, ground or a whole version. 8 vitest, 1 Playwright test with axe (checks the exact body sent and that an empty reason sends nothing).
- Not done: **the patient still cannot ask, withdraw or state disagreement from the UI**, so RC-4's acceptance is not met; the request does not show the record it is about (only its kind), so a clinician must find it on the page; the version typed is not checked against the record; no alert for new or overdue requests; English only, wording mine and unreviewed; not seen with a real service or Keycloak (the Playwright tests stub the API). Full UI gates run: check, vitest, 80 Playwright, all pass. Service gates not re-run (no service change).

## 2026-10-27 - UI: the amendment notice beside a reading (RC-4, H-035)

- The vitals section shows each reading's `amendment_notices` under it: pending, corrected in version n, or refused with the ground and the patient's statement of disagreement (shown as text, not markup). An unknown notice kind is still told in general words. 5 vitest, 1 Playwright test with axe.
- Not done: **the patient still cannot ask, and a clinician still cannot decide, from the UI** (no form or screen), so RC-4's acceptance is not met; no notice on allergies or in FHIR; no alert for new or overdue requests; English only and the wording (including the ground sentences) is mine and unreviewed by any clinician or privacy officer; not seen with a real service or Keycloak. Full UI gates run: check, 203 vitest, 79 Playwright, all pass. Service gates not re-run (no service change).

## 2026-10-27 - Service: amendment notices on the observation list (RC-4, H-035)

- `GET /api/patients/{id}/observations` returns `amendment_notices`: for each observation with a request, the notice (pending, amended in version n, or refused with the patient's statement), newest first. If the requests cannot be read the list fails; a notice is never silently left out. `tests/http_amendments.rs` checks the list carries the same notice as the request.
- Not done: the UI does not show it yet; the FHIR Observation search and the allergy list carry no notice (allergies can be amended, so a refused allergy request is still invisible to a reader); no alert for new or overdue requests. Gates run: fmt, clippy, the whole service suite against Postgres 18 (ignored tests included) and the docs gates. **Not run:** the two FHIR validator tests (no `FHIR_VALIDATOR_JAR`) and the UI gates (the UI is unchanged, though it now receives an extra field it ignores).

## 2026-10-27 - Service: amendment request routes (RC-4, H-035)

- `GET`/`POST /api/patients/{id}/amendments` and `PUT .../{amendment_id}` (one step: accept, deny, apply, extend, withdraw, disagree), policy resource `Amendment` (patient asks, withdraws and disagrees; clinician reads and decides). The requester and decider come from the token, never the body; each step belongs to one role; a stale version is 409; a write and its audit row are one transaction and the row never holds the request text, a reason or a statement. The policy matrix census grew from 13 to 14 resources (1,411,200 cells). `tests/http_amendments.rs` (login on: roles, token as requester, 403/409/422, no text in the audit rows) and both routes added to the audit-refusal test and its census. H-035 opened (status `open`).
- Not done: **no UI, and nothing shows the notice beside a record** (UI or FHIR), so the acceptance (visible to readers) is still not met; no alert for a new or overdue request; nothing checks that an applied version corrects the disputed field; a proxy is refused; deciders are just the clinician role; the 60/30 days and grounds are unreviewed guesses; not run against a real Keycloak. Gates run: fmt, clippy, the whole service suite against Postgres 18 (single-threaded, ignored tests included), and the docs gates (links, status, llms). **Not run:** the two FHIR validator tests (they need the 200 MB `FHIR_VALIDATOR_JAR`; this change writes no FHIR resource) and the UI gates (no UI change).

## 2026-10-27 - Service: amendment request store (RC-4)

- Migration 15: append-only `amendment_versions` (triggers refuse update, delete, truncate) and `amendment_current`. `amendments_store`: `open`, `advance`, `list`, `current`, `notices`. A request must name a record that exists and a version it has; applying needs the record to really have that later version; stale versions conflict; every step keeps the event and its author. 2 Postgres tests (the whole course, refusals, a statement of disagreement reaching the notice, the triggers); `amendment_current` added to the current-views test.
- Not done: **no route, audit row, policy resource or UI**, so nothing can be asked for over HTTP and the acceptance is still not met; nothing checks that the applied version actually corrects the disputed field; the audit trail for these writes does not exist yet (the rows are their own history, but not in `audit_events`); no hazard entry yet. Gates run: fmt, clippy, the whole suite against Postgres 18 (validator tests included).

## 2026-10-27 - Service: amendment request rules, pure (RC-4)

- `amendment.rs`: one request's lifecycle as a pure function (`Request::apply(Event)`): accept, deny with a closed ground and a reason, apply as a later version, withdraw (requester only, before a decision), one statement of disagreement kept unedited, one 30-day extension with a reason; the decider is never the requester; `due`, `overdue` and the `notice` a reader must see. 8 tests.
- Not done: **everything but the rules**: no table, route, audit row, UI, or display of the notice beside a record, so the acceptance (absent from search, visible to readers) is not met. The 60 and 30 days, the grounds list and the HIPAA reading are mine and unreviewed; UK/GDPR rectification, proxies and who may decide are not modelled. No hazard-log entry yet: nothing a clinician sees uses it; open one when it is wired in. Gates run: fmt, clippy, the non-database suite.

## 2026-10-27 - UI: vital signs trend table (RC-6, RC-8, H-034)

- "Show readings over time" opens one table per vital (page order), newest first: time on the site's clock, value, and where it sits in the service's words. A reading with no value stays a row with its reason; entries in error are left out; nothing is averaged, smoothed, charted or flagged by the page. The tables are drawn only when opened. 3 vitest (`trends`), 1 stubbed Playwright test with axe.
- Not done: no chart, no filter by time, no highlighting of change, no NEWS2 (DO-5); English only; no clinician has used it or reviewed the wording; no real-Keycloak run; reads whatever the list returns (no pagination, so a long history is one long table). H-034 gap text updated, status unchanged. Full UI gates run: check, 198 vitest, 78 Playwright, all pass.

## 2026-10-27 - Service: terminology service trait and fixture (RC-7)

- `terminology.rs`: `TerminologyService` (`lookup`, `validate_code`, `expand`, `translate`) and an embedded `Fixture`. Every call must name a pinned version; none, blank or another version is refused (`Unpinned`) on all four operations before any lookup. A missing code is `result: false` in `validate_code`; a translate with no target is an empty list. The fixture's value set equals the codes `observations::SUPPORTED` accepts, so the two cannot drift unnoticed. 5 tests, no network.
- Not done: **the Snowstorm adapter**; no caller (observations and allergies do not check codes against it yet), no route, no stored code version; the fixture's displays, version name and one LOINC-to-SNOMED map are test data that I wrote from memory and nobody has checked against a release. No hazard-log change: nothing a clinician sees uses it yet. Gates run: fmt, clippy, the full non-database suite.

## 2026-10-27 - Service: FHIR Observation search by patient (RC-6, H-034)

- `GET /fhir/R4/Observation?patient=` returns a `searchset` of the patient's vital signs through the same service function as the JSON list (`list_service`, now shared), so the role gate, care relationship, purpose, consent, proxy refusal and the audit row are the same. Other parameters are 400. A list with nothing standing, or only entries in error, carries an `OperationOutcome` warning that it is not a normal reading; a 403 says it does not mean there are no vital signs. Capability statement and route gate updated. Validator test gains a flagged vital sign and a declined one; both pass the HL7 validator 7.0.3 against base R4 with `-tx n/a`.
- Not done: read by id, create, update; no UK Core or US Core profile validation; the validator does not look codes up; no real-Keycloak run of the FHIR route; no clinician has reviewed the warning wording. Gates run: fmt, clippy, the whole suite against Postgres 18 with the validator tests run (all pass).

## 2026-10-27 - Service: a vital sign as a FHIR Observation, pure mapping (RC-6, H-034)

- `interop::fhir_observation::to_fhir` turns one stored observation into a typed R4 `Observation`: `vital-signs` category, LOINC code, UCUM `valueQuantity`, a missing value as `dataAbsentReason` (never zero), the flag computed on read as `interpretation` (none when the flag is null), the sender's range as `referenceRange` (left out if there is no unit). An entry in error is not exported; a record with neither value nor reason is refused. The critical range has no FHIR element and is not exported. 7 unit tests.
- Not done: **nothing serves it** (no `/fhir/R4/Observation` route, so no gate, consent or audit path exists for it yet); not run through the HL7 validator; blood pressure stays two observations, not a panel; no `performer`, `encounter` or `device`; the data-absent and interpretation code choices are mine and unreviewed. Gates run: fmt, clippy, the full non-database suite (235 unit tests pass), link, status and llms checks; the database tests were not run.

## 2026-10-27 - UI: a vital sign's version history on the page (RC-6, H-034)

- Each entry, standing or entered in error, has a "History" button that loads the service's versions route and lists every version oldest first: value (a missing value with its reason), where it sits, range, entered in error or not, time on the site's clock, author and reason ("no reason given" when empty). A retracted value stays visible. A 403 and a failed load say so in words; an open list closes when the entries reload so an old list is never shown beside a change. 1 stubbed Playwright test with axe (three versions, retraction, 403, 500).
- Not done: no real-Keycloak run of the history; no side-by-side comparison; no highlighting of what changed between versions; English only; no clinician has reviewed the wording. Full UI gates run: check, 195 vitest, 77 Playwright, all pass. H-034 gap text updated, status unchanged.

## 2026-10-26 - Service: an observation's version history (RC-6, H-034)

- `GET /api/patients/{id}/observations/{observation_id}/versions` returns every version oldest first, each with its reason, author, time and a flag computed now from that version's own range. Same authorisation as the list (role, care relationship or break-glass, purpose, consent; a proxy is refused), audited before the data leaves with ids and a count only. Not found for another patient's observation. Route-gate table updated. A store test (three versions with reasons, a retracted one still returned, another patient and a bad id not found) and an HTTP block (nurse, receptionist and proxy and no-relationship cases).
- Not done: **the page does not show it yet**; no pagination (an observation has a handful of versions); the history of a merged-in record's observation is returned under the survivor (by provenance, as the list does) but I did not test an unmerge against the history route; the flag in an old version uses today's rules, not the rules then. Whole suite run against Postgres 18 with `--no-fail-fast`: everything passes except the two FHIR validator tests, which need the external validator that I did not start. H-034 gains the new read path.

## 2026-10-26 - UI: real-Keycloak run of vital sign correction and entered-in-error (RC-6, H-034)

- New test in `tests/e2e-keycloak/signin.spec.ts`: a clinician with a care relationship records 172, corrects it to 72 (reason required first), marks it entered in error, and the real API (Keycloak tokens, Postgres 18) answers version 3, `entered-in-error`, value 72 kept. Run by hand against Keycloak 26.3.0, the API built from `main` and a fresh database; it and the earlier vital test pass. Containers removed afterwards.
- Not done: this suite is still not in CI; the first run passed, so I did not see it fail (the assertions name version 3 and the status, which a no-op would not give); the earlier versions are not read back (no endpoint returns them); one run, one machine.

## 2026-10-26 - UI: correct a vital sign from the page (RC-6, H-034)

- "Correct" loads a standing entry into the form; saving sends a PUT with the whole record, `expected_version` and a required reason. The service stores it as a new version and keeps the old. The vital cannot be changed while correcting. A critical range, which the form has no field for, is carried over unchanged (without that a correction would have erased it; the test checks it).
- Not done: the earlier version is not shown on the page; a real-Keycloak run of this or the entered-in-error action; nothing warns that another clinician already read the old value; no clinician review of the flow or wording. A correction can still be wrong inside the bounds. H-034 controls and gaps updated, status unchanged.

## 2026-10-26 - UI: mark a vital sign entered in error (RC-6, H-034)

- Each standing vital sign has "Mark entered in error", which needs a reason and sends the stored record with only `status` changed, plus `expected_version`. The entry moves under "Entered in error (n)", struck through; nothing is deleted. 2 vitest, 1 stubbed Playwright test with axe (reason required and nothing sent without it, 409 message, exact request body).
- Not done: correcting a value from the page; earlier versions; no real-Keycloak run of this action (the service's PUT is tested over HTTP, the page's call to it only against a stub); no clinician has reviewed the wording or whether one click plus a reason is enough friction for a clinical record. H-034's controls note the action; status unchanged.

## 2026-10-24 - Service: the proxy authority rule, pure (PA-5, first slice)

- `proxy.rs`: `decide` (revoked, expired, out of scope, in that order; the expiry instant is expired) and `validate` (no empty scope, no self-recorded authority, a required expiry within five years). 9 unit tests; moving the expiry or revocation boundary, or ignoring the scope list, makes tests fail.
- Not done: nothing calls it, so **proxy access is unchanged and still all-or-nothing** (hazard H-011 and the proxy part of the model are not improved yet, so the hazard log is unchanged). No store, route, audit row, access-log entry, UI or adolescent-confidentiality rule. The four scopes and the five-year limit are guesses; the basis list is not checked against anything.

## 2026-10-25 - Service and UI: the banner's age is counted on the site's calendar day (RC-9, H-016)

- Decision (maintainer): a service-recorded zone. `EPR_SITE_TIME_ZONE` (an IANA name; unset or blank is UTC; passed through `compose.yaml`) is checked once at start-up and a bad name stops the service. `GET /api/patients/{id}` carries it as `site_time_zone`; the UI's `ageOn(birthDate, now, zone)` counts on that zone's calendar day, so every reader at a site sees the same age, changing at the site's midnight. `chrono-tz` 0.10 is now a direct dependency (it was already in the lock through the loco tree, so nothing new is downloaded).
- Tests: 2 service unit tests (zone parsing), an HTTP assertion on the read, UI unit tests (New York, Auckland, London across a clock change, a birthday at midnight, an unknown zone), and a browser test with a fixed clock (46 years under Auckland, 45 under New York, 45 with no zone).
- Not done: one zone per service, not per site, patient or reader, so a service spanning zones counts every age on one zone's day; the setting is not checked against where patients are; the search list and other pages that show an age (if any) were not searched; an older API leaves the UI on UTC; no clinician has reviewed the rule. H-016's controls and residual are updated and the status is unchanged. The full Postgres suite (91 tests, validator tests skipped) passes with the change.

## 2026-10-25 - Matcher: a shared NHS number alone no longer auto-links (CQ-37, H-001), unreleased

- Decision (maintainer, answering the open question): a shared valid NHS number with a given name, family name or birth date absent on either record goes to Review with a `FieldUnavailable` conflict for each missing field; with all three present it still links, and a differing family name (marriage) does not block it. Spec (`matcher/spec/index.md`, `spec/patient-identity/index.md`), tasks, matcher CHANGELOG and H-001 updated; `a_shared_nhs_number_alone_auto_links_today` is replaced by `a_shared_nhs_number_alone_goes_to_review`, and a test helper in `fields.rs` now supplies agreeing names (it modelled a bare number, which is the case that changed). 96 matcher tests pass; clippy and fmt clean.
- **Not done:** **not released and not in the service**: the service runs the published 0.3.0, so the behaviour is unchanged in the API until a matcher release and a service bump (publishing is an outward step I have not taken without your go-ahead); the Clinical Safety Officer has not reviewed the decision; the pairs that now go to Review (imports with a number but no birth date are the common case) will add work for whoever reviews them, which was not measured. H-001 stays `controlled-unreviewed`.

## 2026-10-25 - UI: browser tests wait for hydration (flake fix)

- "an empty search sends nothing and says why" failed about once in forty runs on a loaded machine (reproduced: 1 failure in 40 with six busy loops running): the test clicked Search before the page had hydrated, which is a native form submit that does nothing. The root layout now sets `data-hydrated` on `<html>` once the handlers are attached, and `tests/e2e/fixtures.ts` makes every `page.goto` and `page.reload` wait for it (5 s, then carry on). The three spec files use the fixture. After the change: 80 of 80 runs of that test, and the whole suite twice over (132 tests), pass with the same load.
- Not done: forty runs is a small sample, so "fixed" means the failure did not recur, not that the race is proven gone; tests that navigate by clicking a link (client-side) do not go through `goto` and are not covered by the wait; a page that never sets the attribute costs a 5 s wait per navigation before the test goes on. No product behaviour changed beyond one attribute on `<html>`. No hazard-log change.

## 2026-10-25 - Dependencies, Lily, llms files, skill, Dependabot

- **UI:** `@lilydesignsystem/svelte-headless` 0.4.0 to 0.6.1, `svelte-picker-bar` 0.3.0 to 0.4.0, `themes` 0.4.0 to 0.6.0, `vite` 8.3.4 to 8.3.5. Note that no page imports any Lily package yet (they are declared in `package.json` only), so the browser tests prove the update installs and builds, not that any Lily component still works. pnpm resolved `svelte-picker-bar` to 0.4.0 though 0.5.1 exists: 0.5.x is under a day old and pnpm holds back releases that young; revisit tomorrow.
- **Not updated: TypeScript 6.0.3 stays.** TypeScript 7.0.2 (Dependabot's PR #2) was tried: `svelte-kit sync` fails because SvelteKit 3.1.0 calls `ts.readConfigFile`, which the native TypeScript 7 does not provide. Dependabot now ignores TypeScript majors (reason in `dependabot.yml`).
- **Service:** `tokio` 1.53 to 1.54 in `Cargo.toml`, and `cargo update` (icu, redis 1.8, uuid 1.29, smallvec and others, in the lock only). The matcher had nothing to update. `axum-test` and `jsonwebtoken` were not touched (they follow loco-rs).
- **Dependabot PR #1** (`dtolnay/rust-toolchain` 1.99.1 to 1.120.0) is wrong: that action's ref is the Rust version it installs, the pin is `rust-toolchain.toml`, stable Rust is 1.99.1 and 1.120.0 is not a release. Dependabot now ignores that action. PR #2 is superseded by the updates above except TypeScript.
- **llms.txt and llms.json** are new, generated by `scripts/generate-llms.py` from one table (a missing document fails it) and checked in the docs job, so they cannot drift. `AGENTS.md` and the skill (`electronic-patient-record-skill`) now name them; the skill states the TypeScript 7 limit and the translation rule. README says SvelteKit 3 (it said 2/3).
- **Not done:** there is no `electronic-patient-record.github.io` folder or GitHub Pages site (creating one is a new public project, so I did not); `AGENTS.md` and the other agent files are all well under 40 KB (the largest is 4.8 KB), so none needed trimming; the GitHub notifications other than this repository's two Dependabot PRs belong to other repositories and were left alone; Dependabot closed its own PRs #1 and #2 within a minute of this push (the config change and the updates superseded them); I marked this repository's two notifications for them done on GitHub, and left the other repositories' notifications alone; the container build and the FHIR validator job were not run locally.

## 2026-10-25 - UI: the access-log download says when the file was cut (SE-6)

- The BFF proxy now passes on `x-next-before` (and nothing else new; a unit test checks an unknown header is still dropped), `fetchBlob` returns it, and when a CSV was cut at 1000 entries the page says "The file holds only the 1000 most recent entries. Narrow the days to get older ones." (in the page's locale; machine-written, unreviewed, in seven languages). A browser test covers it.
- Not done: the page does not offer the next batch itself (the patient narrows the days); the other proxied routes that send `x-next-before` (audit export) are not used by this UI. No hazard-log change.

## 2026-10-25 - Service and UI: a refused merge carries its reasons as codes, shown translated (CQ-38)

- A merge the matcher refuses is still 422 `{errors: [...]}` and now also carries `conflict_codes` (new `StoreError::NotSamePerson`; `ApiError::Invalid` can carry extra keys). It is still audited as a refusal (the audit condition now names the new variant: leaving it out would have silently stopped recording refused merges). The merge section shows "These do not look like the same person:" and each reason in the page's locale, or the API's English message whole when there are no codes (a verified NHS number alone decided it) or a code it cannot translate.
- Tests: a unit test of the body shape, the Postgres store test (codes are a list), unit tests of the lead sentence in all ten locales, and a browser test in German (translated, then the English fallback).
- Not done: **the new lead sentence in seven languages is machine-written and unreviewed**; the earlier note that `links.rs` wrote English into link notes was wrong: it builds the refusal message, and no link note is stored for a refusal, so there was nothing stored to change; the match score is in the English message only, not in the translated one; the unmerge refusals and the other merge refusals (already merged, itself) stay English. The FHIR validator tests were not rerun. No hazard-log change beyond H-001's earlier text.

## 2026-10-25 - Service and UI: matcher 0.3.0 in the service; the reasons for no auto-link in nine locales (CQ-38, H-001)

- The service now uses matcher 0.3.0 (the CQ-19 changes: part-by-part family names, extra-given-name block, transliteration, swap, E.164, household, nicknames, phonetic Review, placeholders; `conflict_codes`). Clippy and all tests pass (Postgres tests included, see below). `Cargo.lock` also moved some Windows-only `windows-sys` entries to one version when it re-resolved.
- The compare and registration pages translate each reason from its code (all nine; the numbers in the locale's digits) in ar, cy, de, es, fr, hi, zh; an unknown code shows the matcher's English text. Unit tests cover all nine codes in all ten locales and the fallbacks; browser tests cover de, ar (with axe) and fr.
- **Not done / not verified:** **the translated reasons are machine-written and clinical wording, and no speaker or clinician has reviewed any of it**; no clinician has retested the new matching rules against the service, and the thresholds were not re-evaluated (H-001's residual says so; its status stays `controlled-unreviewed`); `links.rs` still writes the English text into link notes; the older UI code paths that print `conflicts` elsewhere, if any, were not searched beyond these two pages; other pages' words stay English. The FHIR validator tests were not rerun.

## 2026-10-25 - Service and UI: the patient's access log as a CSV file (SE-6)

- `GET /api/me/patients/{id}/access-log?format=csv` returns the same rows, under the same authorization, filters and audit row (which records `format: csv`), as `text/csv` with a download name, up to 1000 of the most recent (`x-next-before` when there are more). Cells that could run as a spreadsheet formula are guarded; no staff identifier is in it. `/my-record` has a download button that sends the filter that is showing. Tests: a unit test of the CSV, the HTTP test (header, no staff subject, a bad format is 400), unit and e2e for the button (a real download, axe clean).
- Not done: the file is not signed or hashed, so it proves nothing about itself (the audit export's chain check does not cover it); times in it are UTC, though the page shows the patient's zone; a patient with more than 1000 matching entries must narrow by day (the UI does not offer the next batch); the column names and the download button's words are English except the button (translated, machine-written, unreviewed); not a formal accounting of disclosures under any law, since nothing here says what counts as a disclosure. The Keycloak e2e was not run. Also: the existing e2e test "a clinician can declare treatment..." raced the request behind the loading text and failed once on a loaded machine; it now waits for the request (`expect.poll`), not weakened (the same value is asserted). No hazard-log change.

## 2026-10-25 - UI: the access log's entries give the purpose in words (SE-6)

- "Purpose: operations" now reads "Purpose: running the service", and `patient-access` reads "you or your proxy looking at the record"; a code from a newer API is shown as it is. `treatment` reads as before, so the existing sentence tests did not change. One unit test.
- Not done: English only (the entries' sentences are not translated, nor is this phrase); the Keycloak e2e test still expects "Purpose: treatment", which is unchanged, and was not run. No hazard-log change.

## 2026-10-25 - UI: the access-log form in the patient's zone and in the page's language (SE-6, RC-9)

- **Zone:** the filter's days are the patient's own, in the browser's time zone, named on the form. Each is sent as the UTC instant it starts at (`from=2026-09-30T11:00:00Z` for 1 October in Auckland), so a 23 or 25 hour day across a clock change is right (tested for London, New York and Auckland); the last day is included by sending the start of the next. Entries are shown in that zone with the zone named ("2026-10-21 22:00 GMT+13"). The e2e suite now runs in a fixed UTC zone and the zone test overrides it.
- **Locales:** the form's words (17, and the 7 purposes) exist in ar, cy, de, es, fr, hi and zh (`de-de` reads as `de-001`, `zh-cn` as `zh-001`) in `src/lib/translations.ts`; English falls back for any missing word. A unit test checks every locale has every word, none is left in English and `{zone}` survives; an e2e test opens the form in de, ar (right-to-left) and zh-cn with axe.
- **Not done: the translations are machine-written and no speaker has reviewed any of them** (the Welsh and Hindi are the ones I am least sure of); the rest of the page, the entries' sentences and every other page are still English; there is no translation mechanism beyond this table (SF-4 stays open); "the patient's zone" is the zone of the browser they read in, since the service records none, so a proxy reading from another zone sees their own; a zone with no midnight on a day gets only an approximate start; the `dayStart` second offset pass is not distinguished by any test; the entry sentences still print the purpose as a code. No hazard-log change.

## 2026-10-25 - Service and UI: filter the patient access log by purpose (SE-6)

- `GET /api/me/patients/{id}/access-log?purpose=<code>` keeps rows made for that purpose of use; an unknown code is 400. `/my-record` has a Purpose select with the seven codes (English labels; the entries still print the raw code, "Purpose: treatment").
- Not done: the form's strings are not in the other locales (next); days are still UTC (next); the entries' purpose is shown as a code, not a label. Postgres tests for the filter ran in `audit_chain.rs` and `http_proxy_scopes.rs`; the full suite was not rerun for this change. No hazard-log change.

## 2026-10-25 - Service: the CapabilityStatement's resources and interactions are generated from the routes (IO-1, fourth slice)

- `capability()` reads the mounted routes (`/Type` and `/Type/{id}` by method) for each resource type and its interactions, in FHIR's order; only the notes, versioning and search parameters stay hand-written. Resources are now listed alphabetically. A new test fails when a searchable type is mounted without its search parameters described, or notes exist for an unmounted type; an unknown method or path shows as `unmapped-<METHOD>` and the statement stops parsing.
- Not done: the search parameters and the `date` are still written by hand (the parameter census checks the three types it knows, not a new one); the previous census of routes against the statement now agrees by construction and only checks the mapping; `description` is hand-written. Behaviour on the wire is otherwise the same. The validator job does not validate the CapabilityStatement, so it was not checked by the official validator. No hazard-log change.

## 2026-10-25 - UI: a filter form on the access log in `/my-record` (SE-6)

- Once the log is shown, a form narrows it by first and last day, who and outcome; older entries are fetched under the same filter; Clear returns to the whole log. A last day is sent as the next day (the API's `to` is exclusive); a day that is not real sends nothing. `logQuery` has 4 unit tests and one e2e test covers apply, older entries and clear (axe clean).
- Not done: days are UTC, and the form says so, not the patient's zone (RC-9); no purpose filter; the form's strings are English only until the translations are done and reviewed; nothing was tested in `ar-001` beyond the existing right-to-left page check, which does not open the log. The API's audit row for the read records only that a filter was used. No hazard-log change.

## 2026-10-25 - Service: the patient access log narrows by day, by who and by refusal (SE-6)

- `GET /api/me/patients/{id}/access-log` takes `from` (inclusive) and `to` (exclusive), as a day or an instant in UTC, `by=you|staff` and `refused=true|false`. A filter it cannot read is a 400 and nothing is audited. The paging `next` still works under a filter. The read's own audit row says only that a filter was used, not its values.
- Tests: a Postgres test over the four filters and bad dates (`audit_chain.rs`), and over HTTP for a proxy with the access-log scope (`http_proxy_scopes.rs`).
- Not done: no filter form on `/my-record` yet (next); days are UTC, not the patient's zone (RC-9); no filter by purpose or by role; the full Postgres suite (91 tests, validator tests skipped) passes locally. The log's content is unchanged, so no hazard-log change.

## 2026-10-25 - UI: tests for remove photo and undo this merge (CQ-33, last of the write buttons)

- Two clicks in one task on "Remove photo" and on "Undo this merge" each send one POST. Each fails with its own guard removed (`photoBusy`, merge's `busy`): 2 requests, not 1.
- Not done: the CQ-33 write buttons now all have a test. Still open from CQ-33 in general: nothing known besides the read-only search and compare forms, which have no test of their own.

## 2026-10-25 - Matcher: release 0.3.0

- The matcher crate is now version 0.3.0 and is published to crates.io: the CQ-19 changes (part-by-part family names, extra-given-name block, transliteration, swap detection, `\p{M}` fold, E.164 phones, household evidence, nicknames, phonetic review, `MatchConfig::validate`), `conflict_codes` and placeholder names (EC-2). 96 tests pass; clippy and fmt clean.
- Not done: the service still pins 0.2.0, so none of this reaches the API, the UI or any link note yet (CQ-38); no decision was recorded on H-001's retest against the new matching, so the hazard log is unchanged and the hazard stays `open`; no git tag was made (a `v*` tag releases the service image, not the matcher). A published version cannot be unpublished, only yanked.

## 2026-10-25 - Service: HTTP test of the unknown-search-parameter refusal (IO-1)

- `http_patients.rs` (Postgres): `Patient?gender=`, `AllergyIntolerance?category=` and `AuditEvent?_id=` are each a 400 OperationOutcome saying "not supported"; `_count` still returns 200. With the AllergyIntolerance handler's check disabled the test fails.
- Not done: the full Postgres suite was not rerun here, only this test; `Prefer: handling=lenient` is still not offered. No behaviour changed.

## 2026-10-25 - Service: Patient and AllergyIntolerance FHIR searches refuse an unknown parameter (IO-1, third slice)

- `GET /fhir/R4/Patient?gender=female` used to ignore `gender` and return every match on the other terms, which looks filtered and is not. It, and an extra parameter on `AllergyIntolerance`, is now a 400 `not-supported` naming the parameter (same rule as AuditEvent); `_count`, `_format` and `_pretty` still pass. FHIR permits ignoring unknown parameters, so a client that sends extras will now get an error: that is the intended trade.
- Not done: no Postgres test of the 400 over HTTP (the rule is unit-tested through `search_from` and `refuse_unknown`; the AllergyIntolerance handler's call to it is not exercised by a test); `Prefer: handling=lenient` is not offered. The Postgres tests were not rerun. No hazard-log change (it only refuses more requests).

## 2026-10-25 - Service: the CapabilityStatement's search parameters match the code; AuditEvent refuses `_id` and `_lastUpdated` (IO-1, second slice)

- New test: the parameters stated for Patient, AllergyIntolerance and AuditEvent are the ones the code reads. It found a gap: `GET /fhir/R4/AuditEvent?_id=...` or `_lastUpdated=...` ignored the filter (every `_` parameter was dropped) and returned unfiltered events, which the doc comment said must not happen. Now only `_count`, `_format` and `_pretty` are ignored and any other is refused (400 with the parameter's name).
- Not done: the "accepted but not stated" check uses a fixed list of common FHIR names, so an unusual parameter added later without a statement entry would not be caught; Patient search still ignores unknown parameters (allowed by FHIR, but a client cannot tell its filter was dropped); the statement is not generated. The Postgres and validator tests were not rerun here. No hazard-log change (the change only refuses more requests, on the auditor-only search).

## 2026-10-25 - UI: action budgets for registering a patient and recording an allergy (AS-5, first slice)

- `tests/e2e/budgets.spec.ts` counts each user action (click, text box, list) through one helper and fails above `budgets.json` plus 10%: register 6, allergy 5 (measured today, so no slack). Lowering a budget one below the count made the test fail.
- Not done: no time budgets (every API call is stubbed, so a time would measure the stub); prescribe, acknowledge results and sign a note have no UI; with budgets this small the 10% allowance rounds to nothing, so any extra step fails; the count is of the test's own steps, not an independent measurement. No hazard-log change.

## 2026-10-25 - UI: the read-at clock names its time zone; time-zone tests (RC-9, first slice)

- "Record read at 10:00" now reads "10:00 UTC" (or the reader's zone name): a bare clock time means nothing across sites. New `time-zones.test.ts` runs `TZ` = UTC, New York and Auckland, and a New York DST change.
- Not done: the service keeps no recorded zone; API times are still shown in UTC, not the user's zone; `ageOn` counts on the UTC date, so near midnight a banner age can differ by a day from the reader's local date (pinned by a test as current behaviour, not endorsed; needs a decision). Zone abbreviations are locale-dependent and not checked outside en-GB. No hazard-log change.

## 2026-10-25 - Service: census of the CapabilityStatement against the mounted routes (IO-1, first slice)

- `the_capability_statement_lists_exactly_the_mounted_routes`: every mounted `/fhir/R4` method and path is stated, and every stated interaction is mounted. Adding an unlisted route made it fail.
- Not done: the statement is still hand-written, not generated; search parameters, `date` and the description are not checked against the code; no behaviour changed.

## 2026-10-25 - UI: register, search and compare ignore a second click (CQ-33)

- Fixed a real gap: the register form disabled its button only after a redraw, so two clicks in one task sent two POSTs (the new test showed 2 requests on the old code). A double click could therefore ask for the same registration twice. Register, search and compare now use `Busy`.
- Hazard H-002 controls and evidence updated; status stays `open`.
- Not done: search and compare are reads, so no test for them. The server still has no idempotency key, so a retry from another tab can still register twice (the duplicate check is the control there).

## 2026-10-25 - UI: a test for the photo buttons' one-at-a-time rule (CQ-33)

- Two clicks in one task on "Upload photo" send one PUT and switch "Remove photo" off while it is in flight; with `photoBusy` removed from the guard the test fails (2 requests, not 1).
- Not done: withdraw is not clicked twice (same guard as upload); unmerge likewise. CQ-33 still lists the register and search forms as not reviewed.

## 2026-10-25 - UI: tests for the merge and admin feed buttons' one-at-a-time rule (CQ-33)

- Two clicks in one task on "Merge into this record" and on "Close this entry" each send one POST and disable the button while in flight. Each fails with its own guard removed (merge's `if (busy) return`, and `Busy.run`'s check for the feed page): 2 requests, not 1.
- Not done: photo upload and withdraw still have no such test; unmerge is not clicked twice (it shares merge's guard).

## 2026-10-25 - UI: tests for the break-glass and admin review buttons' one-at-a-time rule (CQ-33)

- Two clicks in one task on "Open emergency access" and on "Record the review" each send one POST and disable the button while in flight; with the guard removed the first fails (2 requests, not 1).
- Not done: photo upload/withdraw, merge/unmerge and the admin feed page still have no such test. The review half of the test was not mutation-checked separately (the same `Busy` class guards both).

## 2026-10-25 - UI: a test for the allergy buttons' one-at-a-time rule (CQ-33)

- Two clicks in one task on "Record allergy" send one POST and switch "Record: no known allergies" off while it is in flight; with the guard removed the test fails.
- Not done: photo, merge, break-glass and the admin pages still have no such test.

## 2026-10-25 - UI: axe passes on the main pages in ar-001 (CQ-33, third slice)

- One e2e test visits search, register, patient, access and my-record under `/ar-001`, checks `dir="rtl"` and a visible heading, and runs axe (WCAG 2.2 AA). It passed first time, so it found nothing to fix.
- Not verified: the layout was not looked at by a person or a speaker of Arabic; axe checks rules, not whether the page reads right. Arabic strings are machine-written. Admin, break-glass, compare and the allergy section (shown on the patient page with stubbed data) were not visited. CQ-33 stays open for the untested busy buttons.

## 2026-10-25 - UI: one write at a time on every write form (CQ-33, second slice)

- A small `Busy` helper (`src/lib/busy.svelte.ts`) runs one write at a time and disables its buttons; it now covers the access page (relationships, consents), allergies, break-glass and the feed and break-glass admin pages. One e2e test sends two clicks in one task to the relationship and consent buttons and expects one POST each; with the guard removed it fails (the disabled button alone does not stop two clicks in one task).
- Not done: no test for allergies, break-glass, the admin pages, photo or merge (the same helper, but untested there); the register form keeps its own `busy` flag; no axe run with `ar-001`; RTL not looked at. CQ-33 stays open.

## 2026-10-25 - UI: one write at a time, loading is announced, logical CSS (CQ-33, first slice)

- The patient page's save and photo buttons, and the merge section's buttons, are disabled while a request is in flight and a second click is ignored in code too; the loading text on the patient and my-record pages is `role="status"` `aria-busy`; `left`/`right` in `app.css` became logical properties. One e2e test (a double click on save sends one PUT); removing the guard and the disabled attribute fails it.
- Not done: the register, access, consent, break-glass and admin forms still allow a double submit (the server's version check turns the second into a 409, which is not a fix); no double-click test for photo or merge; no axe run with `ar-001`; the RTL layout was not looked at. CQ-33 stays open.

## 2026-10-25 - UI: the access log no longer calls a proxy "a member of staff" (PA-5)

- An entry whose recorded roles include `proxy` reads "Someone acting as a proxy viewed your identity record." It used to read "A member of staff (proxy, …)". One unit test.
- Not done: the API still sends `by: "staff"`; only the wording differs, so another client would still say staff. The roles come from the token at the time, so older rows without roles still say staff. Not shown against a real log.

## 2026-10-24 - Proxy authority is enforced, stored and recorded (PA-5)

- A `proxy` relationship now carries terms (basis, scopes, expiry); a proxy reads only the scopes listed, only until the expiry, and ending the relationship revokes it. Enforced in `authorize` (identity, allergies), the photo routes, the access-log route and `/api/me/patients`. **Behaviour change: a proxy recorded before this has no terms and now reads nothing.** The access page records the terms. Hazard H-011 updated (controls, evidence, residual; status unchanged, `open`).
- Tests: `tests/http_proxy_scopes.rs` (Postgres, login on), 3 more `relationships::tests`, 3 UI unit tests and 1 e2e. Moving the allergies, photo or access-log scope to identity makes the HTTP test fail. The UI test caught that the list showed the exclusive end date, fixed.
- Not done: adolescent confidentiality; a second approver; expiry reminders; editing scopes in place; clinician review of the four scopes and the five-year limit (both guesses); any check of the basis. I did not check how the patient's access log labels a proxy's own reads. Existing proxy rows in a real database are not migrated (there are none outside tests). Not run against Keycloak.

## 2026-10-24 - UI: `/my-record` shows how old the record is (EC-12, second slice)

- The patient and proxy page shows "Record read at HH:MM" and, when a later look fails or none succeeds for 2.5 minutes, an alert that the record may not be current. It looks again once a minute and when the tab comes back, and keeps the records on screen if a look fails. One e2e test. Same rule as the patient page (`src/lib/data-age.ts`).
- Not done: other pages, offline reading, sync priority. The access log, opened on request, is not refreshed and carries no age. Not verified in a real browser against the real API; the test stubs the API. The hazard log is unchanged: H-016 already lists the notice as a control for the patient page, and I did not re-assess the patient-facing page.

## 2026-10-24 - Spec: when a write's audit row cannot be written, the write is refused (CQ-39)

- New section "The audit row is part of the write" in `spec/audit/index.md`, a sentence in NFR-7, and task CQ-39. The service already behaves this way for patient create and update (the store test and the service spec's "Write and audit in one transaction" row); the new text makes it the rule for every write.
- Not verified: only patient create and update have a test with a failing audit table. I read that the other write controllers (allergies, photos, merge, break-glass, feed, consents, relationships) begin a transaction and commit, but I did not check that each appends its row inside it, and no test drives them. The spec says so, and CQ-39 is the test. Nothing in code changed for this item.

## 2026-10-24 - Service: a differencing check for aggregates (PA-7, second slice)

- `privacy::would_difference` refuses a table that, subtracted from an earlier one by the same caller on the same measure over a **nested** population (a `Filter` of field to allowed values, a missing field meaning unrestricted), would reveal the count of 1 to `k-1` people. 7 more unit tests (18 in all); with the lower boundary moved and the subset test loosened, tests fail. No caller yet, so the hazard log is unchanged.
- Not done: it sees only nested populations on one measure and uses **true** totals, so the caller must keep them (in memory or a store nobody can read, which does not exist); overlapping populations, related measures (admissions against attendances), two callers comparing notes, and repeated queries are not seen. Two-way tables need complementary suppression across rows and columns. PA-7 stays open: no report uses any of this and there is no per-report test.

## 2026-10-24 - Service: the privacy rule for aggregates (PA-7, first slice)

- New `src/privacy.rs`: `suppress` hides counts of 1 to `k-1` (`k` at least 5), shows zeros, and with a published total also hides the smallest other nonzero cell until the hidden sum reaches `k`; `Budget` allows a number of tables per caller per window and does not count a refusal. 11 unit tests; with secondary suppression off and the boundary moved, three fail. No caller, so the hazard log is unchanged.
- Not done: the rule does **not** stop differencing (two tables over populations that differ by a few people), linkage, or repeated queries, and says so in its header; the pair-of-queries check is not built, so the acceptance "a pair of queries that would difference it out is refused or listed" is not met and PA-7 stays open. It handles one-dimensional tables with one total; a two-way table with row and column totals needs a stronger rule (complementary suppression across both). The budget is in memory only. `k` of 5 and any limit are guesses.

## 2026-10-24 - UI: the patient page shows how old the record is (EC-12, first slice)

- `src/lib/data-age.ts` (pure), a "Record read at HH:MM" line under the banner on `/patients/[id]`, and an alert when the latest background look failed or no read succeeded for 2.5 minutes; the record stays on screen. Tests: 5 unit, one Playwright (read, failed look, recovery; axe clean). `pnpm run check` clean, vitest 117, Playwright 46 pass locally. H-016's controls, evidence and residual text updated (status unchanged).
- Not done: the other pages (`/my-record`, lists, compare) show no age; **2.5 minutes is my guess**; the time is shown in the reader's browser locale and is not tested in other locales; tested with a stubbed API, not a real outage; offline reading and sync priority (CA-7, CA-8) are not built. The strings are English only.

## 2026-10-24 - Break-glass reviews: overdue grants are flagged (EC-11, first slice)

- `Grant.overdue` (service) is true for a grant with no review after `EPR_BREAK_GLASS_REVIEW_HOURS` (default 72 hours); the count route adds `overdue`; the admin page marks such grants "OVERDUE FOR REVIEW". Tests: a unit test for the hour parsing, a Postgres test (fresh, just inside, past the limit, then reviewed), and a Playwright test. The Postgres `break_glass_store` and `http_roles` suites and the UI gates pass locally (check clean, vitest 112, Playwright 45).
- Not done: **72 hours is my guess, not a policy**; no alert or escalation; no review form recording necessity, minimum data used or whether the patient was told; the overdue number is not shown anywhere but the count response; the string is English only. Not clinical in the sense of the hazard log (it changes what an administrator sees, not what a clinician sees), so the hazard log is unchanged; the review form would be.

## 2026-10-24 - Service: the gated-feature rule (PA-4, first slice)

- New `src/gate.rs`: a pure `effect(gate, now)` from a switch (`Off`, `Shadow`, `On`), required approver roles and approvals. `Off` is inert; `Shadow` records and never shows; `On` shows only with every role approved by a person inside the approval's window and not revoked; one person cannot approve two roles; a script identity never counts; a gate with no named approver never shows. 9 unit tests; with the window check and the one-person rule mutated out, two fail. Not clinical in itself: no feature uses it, so the hazard log is unchanged.
- Not done: no store, route or audit for approving or revoking, so an approval cannot be made; no caller; no record of shadow output. Two choices of mine, not the maintainer's: shadow needs no approval (so evidence can be gathered), and an approval with no end date is allowed. PA-4 stays open.

## 2026-10-24 - UI: the patient banner marks a drill record (EC-10)

- `Stored.drill`, a string `bannerDrill`, an alert at the top of `PatientBanner` and a style; one Playwright test (drill shows it, absent and false do not, axe clean). `pnpm run check` 0 errors, vitest 112, Playwright 44 pass locally. H-033's text now says the banner marks it but nothing else does (status `open`).
- Not done: no label on lists or in the tab title (a drill record is not listed in search anyway), no way to register a drill record from the UI, the string is English only and machine-written like the rest, and the alert is not tested in other locales. I did not run the UI against the real API with a drill record.

## 2026-10-24 - Tests: the last four write routes refuse when their audit row cannot be written (CQ-39 closed)

- FHIR update joins `tests/http_audit_refusal.rs`; new `tests/http_audit_refusal_roles.rs` (login on, one test) covers break-glass open and review and resolving a feed refusal. The trigger and counting helpers moved to `tests/audit_switch/mod.rs`, which also holds the two route lists the census checks; `break_glass_reviews` joined the row counts. All 18 write routes refuse with 500, store nothing, and then succeed once audit works. Nothing in the service changed. With the audit error swallowed in `audit_resource`, the roles test fails on the first route. CQ-39 is ticked.
- Not done: the tests do not assert the response body or that `epr_audit_append_failures_total` moves, and the failure is a trigger, not a real storage fault. A review by the same person who used a grant, and refusals that fail to audit, are not tested here.

## 2026-10-24 - Tests: a write whose audit row cannot be written is refused, on 14 of 18 write routes (CQ-39)

- New `tests/http_audit_refusal.rs`: a trigger on `audit_events`, switched on around one request, makes the audit insert fail. For each of 14 write routes the answer is 500 and no store gains a row, and the same request then succeeds, so the 500 is the audit row's doing. A DB-free census fails when a write route is in the policy table but neither covered nor listed as a gap. Nothing in the service had to change: every covered route already refused. With the audit error swallowed in `audit_resource`, the test fails on the first route.
- Not done: FHIR update, break-glass open and review, and resolving a feed refusal are listed as gaps, so CQ-39 stays open. The test proves the 500 and the unchanged stores, not that the response body is helpful, and not that the failure counter moves. It runs against a trigger, not a real full-disk failure.

## 2026-10-24 - Testing: the two validator tests no longer race for the package cache

- The `fhir-validator` CI job failed on the push after it had passed three times: both tests start the jar at once, and on a cold runner each installs `hl7.fhir.r4.core` into `~/.fhir/packages` under a lock ("Failed to acquire lock on file"). The earlier green runs were luck. The tests now take a mutex, and the CI step passes `--test-threads=1`. I could not reproduce it locally (the package cache is warm here), so the fix is verified only by CI.
- Not done: the R4 core package is still fetched from the network on a cold cache, so a package-server outage fails the job; the `~/.fhir` cache is not saved between runs.

## 2026-10-24 - Service: drill and test records kept apart from real patients (EC-10, first slice)

- Migration 13 adds `patient_versions.drill` (default false; `patient_current` carries it); `POST /api/patients` takes `drill`; every version keeps the flag. Ordinary search, the FHIR facade and the duplicate check see real records only; `?drill=true` searches drill records; a drill registration is compared only with drill records; `links::merge` refuses a drill/real pair (422); a drill Patient in FHIR is tagged `urn:epr:record-kind|drill`. `store::create`, `candidates` and `candidates_ranked` keep their signatures (they are the real side); `create_as`, `candidates_in` and `Search.drill` are new. `tests/drill_records.rs` (5), an HTTP test, a unit test and a drill Patient in the FHIR validator test; mutating out the search filter, the duplicate-check filter and the merge check makes three of the five fail. New hazard H-033 (status `open`).
- Not done: no UI banner or label on any screen, so a drill record is only recognisable by its flag or FHIR tag; nothing limits who may create a drill record; no aggregate or export exists to exclude it from; the flag cannot be corrected on a mistaken record. `down` for migration 13 is refused outside development like the other clinical downs. The ADT feed registers real patients only. I ran the Postgres-gated suites for the stores, links, migration and HTTP tests locally, not the whole workspace.

## 2026-10-24 - CI: a `fhir-validator` job

- New job in `.github/workflows/ci.yml` (Temurin 25, the jar cached by the script's hash, `scripts/fhir-validator.sh` re-checks its SHA-256) running `tests/fhir_validator.rs`, and added to `ci-ok`'s needs. I have not seen it run: it is verified only by pushing. `spec/11` and the service spec said "not yet in CI"; updated.
- Not done: no UK Core or US Core profile validation; a cache miss downloads 200 MB.
- Found by the first run: the validator job passed, but `api-db` failed, because its `-- --ignored` step also ran the two validator tests, which need the jar. The two tests are now named `fhir_validator_*` and the `api-db` step skips that prefix (`--skip fhir_validator_`). Not verified: that `api-db` is green again (it is verified by pushing).

## 2026-10-24 - Testing: the official HL7 FHIR validator as an oracle

- `scripts/fhir-validator.sh` fetches `validator_cli.jar` 7.0.3 (hapifhir/org.hl7.fhir.core) into the git-ignored `.cache/`, and refuses it unless its SHA-256 equals the digest GitHub publishes for that release asset (pinned in the script). `tests/fhir_validator.rs` (service, 2 tests, `#[ignore]`) writes a Patient, three AllergyIntolerance forms, an AuditEvent and a searchset Bundle from the service's own converters and validates them against R4 4.0.1; a control confirms a broken Patient (bad `gender`, stray element) fails. Both pass on Java 25 (about 6 s after the 200 MB download).
- What it found, not fixed: every resource has the `dom-6` warning (no narrative), and the AuditEvent uses `urn:epr:audited-resource` and `urn:epr:purpose-of-use`, code systems the validator cannot resolve (4 warnings). The test asserts no errors, not no warnings.
- Not done: it is not in CI (a 200 MB download and Java per run needs a cache and a decision); it runs with `-tx n/a`, so codes are not looked up (SNOMED, LOINC); no UK Core or US Core profile validation (RC-2); the searchset bundle in the test is rebuilt by hand because the service's own bundle builder is private, so the real search response is not validated; the jar's pin is trust in GitHub's digest, not a signature check.

## 2026-10-24 - Matcher: placeholder names are absent evidence (EC-2, first slice)

- `normalize::is_placeholder_name` and `is_usable_name`; every name rule (family, given, swap, phonetic) now skips a placeholder (`Unknown`, `Unidentified`, `Anonymous`, `UNK-001`), so absent evidence blocks AutoLink: two unidentified patients never match each other and an unidentified patient never auto-links to a real record. `tests/placeholders.rs` (5 tests); three fail with the list emptied. Matcher spec and H-001 updated (status untouched, `controlled-unreviewed`).
- Not done: the temporary record, the verification flag and the merge on identification (the rest of EC-2); the service pins matcher 0.2.0, so none of this is in it. The word list is short, English only and from memory; it is a guess, not a corpus result. Real data may use other placeholders (a hospital's own `ZZ` or `TEST` names), which this does not catch.

## 2026-10-24 - Service: a blank `EPR_PASETO_KEYS` counts as unset

- Found by the container job in CI: `compose.yaml` passes `EPR_PASETO_KEYS: ${EPR_PASETO_KEYS:-}`, an empty string, and the service stopped with "EPR_PASETO_KEYS is not valid JSON: EOF" instead of the startup gate's "no token keys are configured; refusing to start". It failed closed both ways, but the CI step (and anyone following the compose file) expects the gate's message. `inline_keys` now treats a blank value as unset (`auth::tests::invalid_inline_keys_fail_startup` extended; service spec updated). A blank value still cannot start the service with login on.
- CI step also ends `compose down -v` with `|| true`: the runner printed "rootless netns: kill network process: permission denied" on teardown, after the containers were removed.
- Not verified: the container job passing (it takes about 11 minutes); the Postgres-gated tests were not run locally.

## 2026-10-24 - CI: the "API refuses to start" step tolerates `compose up` failing

- The first CI run on `main` failed the container job at `podman compose up -d`, because the API exits at once by design and the step runs with `bash -e`. The step now ends `up -d` with `|| true` and the loop that checks the container state decides. I read this from the two failed logs (both stop right after `api_1` starts); I have not run the Podman job locally.
- Not verified: that the step now passes, or that the later step ("starts, and demands a token, with keys set") passes: it never ran.

## 2026-10-24 - Service: tokio 1.54.1 and ctutils 0.4.5 (RUSTSEC-2026-0384, RUSTSEC-2026-0385)

- `Cargo.lock` only: the first CI run on `main` failed `cargo deny` on both advisories, so I ran `cargo update -p tokio -p ctutils`. `cargo deny check`, fmt, clippy and the database-free tests pass locally.
- Not verified: the Postgres-gated tests were not run locally; CI runs them. I did not check whether the service uses `tokio::time::Interval`, so I do not know if the advisory affected it.

## 2026-10-24 - Research pass: the emergency care system repository (EC-1 to EC-13)

- `plan.md` §19 assesses 21 ideas from `~/git/emergency-care-system`; `tasks.md` gains the `EC-` prefix (EC-1 to EC-13); `spec/16-open-questions.md` gains OQ-E1 to OQ-E3; `scripts/generate-status.sh` counts the `EC` prefix. Planning only: no code, spec rule or hazard-log change.
- Not done or not verified: I read the README, AGENTS, titles of all 171 scenarios and about fifteen full scenarios, not the app code, the twelve suites or the site. The source is synthetic and model-written, so every UK reference (DNACPR or ReSPECT, the Mental Capacity Act, single-sex rules) is marked uncertain and unchecked. No EC item is built, and none has hazard rows yet.

## 2026-10-24 - PA-3: support-only tripwire (NFR-12)

- New `tests/support_only.rs` (service, 3 tests) and NFR-12 in `spec/07`: a migration that names a column like `score`, `rank`, `deprioritise`, `suppress`, `withhold` or `overbook` fails unless it is on an allow-list with a reason. One entry today: `match_score` (the matcher's score on a patient link). I confirmed the test fails when that entry is removed. Not clinical, no hazard-log change.
- Not done: it reads migration source names only, so a score under another name, in a view, or computed in code is not caught. It is a tripwire, not proof. The Postgres-gated tests were not run.

## 2026-10-24 - EM-1: module map proposal

- New `spec/19-module-map.md` (every module group from plan.md §17.2 with a slot, as a proposal), OQ-M1 to OQ-M3 in
  `spec/16-open-questions.md`, a pointer in `spec/02-scope.md` and `spec/index.md`. Planning only: no code, no hazard-log change.
- Not done: the maintainer has not confirmed any slot, so EM-1 is unticked; EM-16 (hazard rows) is not started. The slots rest on search summaries, not on a read of the DCF or EMRAM.

## 2026-10-24 - CQ-38: task to move the service and UI to `conflict_codes`

- `tasks.md` gains CQ-38 and CQ-19 points to it. Planning only: no code, spec or hazard-log change.
- Not done: the matcher release, the service bump and the UI mapping. Nothing is pushed and CI has never run (CQ-20).

## 2026-10-23 - CQ-17: `/metrics` with three counters

- New `src/metrics.rs` and `GET /metrics` (Prometheus text): audit-append failures, refusals that left no audit row
  (401 and 503 from the auth middleware; a 403 whose row failed) and refused HL7 messages. 404 unless `EPR_METRICS` is
  `1` or `true`. `/metrics` is a public path, so `is_public_path`, the route census and the service spec say so.
- Not clinical, so no hazard row; the counters make H-023 (a refused HL7 message never seen) and audit outages easier
  to notice but do not close either.
- **Not done or not verified:** no scrape config, alert or dashboard, so nothing yet pages anyone. Counters are in
  memory, restart at zero and have no labels. When on, `/metrics` needs no token. The DB-gated tests were not run
  (no Postgres started); `cargo fmt`, clippy and `cargo test` pass. Not run on GitHub (CQ-20).

## 2026-10-23 - CQ-31: proxy timeout and transport errors, session kept on provider failure, version header, proxy trust

- The BFF proxy handler moved to `src/lib/proxy-handler.ts` with injected dependencies. It times the upstream call out at
  30 s (504), maps other transport errors to 502 and forwards `Accepts-version`. A refresh failure other than
  `invalid_grant` keeps the session and answers 503; only `invalid_grant` or no refresh token deletes it.
- **Behaviour change for deployers:** `X-Forwarded-Proto` is now ignored unless `EPR_TRUST_FORWARDED_PROTO=1`. A UI behind a
  TLS-terminating proxy that does not set it loses the `__Host-` Secure cookie and HSTS. Nothing in this repo's compose or
  operations files runs the UI behind such a proxy, so none needed editing.
- Not clinical: no change to what a clinician sees beyond a clearer error, so no hazard row. H-025 (sessions) is
  unaffected.
- **Not done or not verified:** the real-Keycloak e2e suite was not run, there is no browser retry after a 503, and the
  proxy-trust rule is unit-tested on its helper only. Not run on GitHub (CQ-20).

## 2026-10-23 - TR-4: seven triage hazards, H-026 to H-032

- Added to `governance/clinical-safety/hazard-log.csv`: wrong flowchart or missed discriminator, under-triage of
  children and similar groups, silent re-categorisation, stale or invalid pack, clock failure or flooding, a tool
  read as an autonomous triager, and unlicensed or wrongly claimed MTS content. All `open`, none accepted or closed.
- **Not done or not verified:** every control is planned and unbuilt, so there are no evidence paths. Severity and
  likelihood are the author's proposals with no site data, and no Clinical Safety Officer has reviewed them.
  `check-hazard-log.py` and its tests pass; that checks shape, not judgement.

## 2026-10-23 - TR-3 partly done: triage spec page

- `spec/triage/index.md` (linked from `spec/index.md`) sets the content policy, the model (pack, append-only
  assessment, `not-assessed`, suggested versus confirmed), a proposed FHIR mapping and the rules.
- **Not done or not verified:** the mapping rests on search summaries; UK Core, US Core and LOINC were not opened,
  and the ESI code 75636-1 is unconfirmed. No worked example, so TR-3 stays unticked and OQ-T3 open. No code or
  tests. No hazard rows (TR-4).

## 2026-10-23 - TR-2: emergency triage is post-1.0

- `spec/02-scope.md`, `spec/15-roadmap.md`, plan.md §7.2 and OQ-T2 and OQ-T4 state the decision: post-1.0, a
  pluggable triage pack, only MTS planned.
- **Not done or not verified:** no code, no milestone assigned, and the decision rests on one chat reply from the
  user, not a maintainer review or the Clinical Safety Officer. No hazard rows yet (TR-4).

## 2026-10-23 - TR-1 partly done: MTS licensing findings and interim policy

- `plan.md` §15.1.1 records how MTS is licensed (Wiley for the Advanced Life Support Group and the Manchester Triage
  Group; framework-only or framework-and-content licences) and sets an interim policy: no MTS content in the repo.
  OQ-T1 and TR-1 point to it.
- **Not done or not verified:** only search summaries were read, not the licence, the ALSG help page or the ALSG PDF.
  Nobody has contacted Wiley or the group, so whether an open-source project may publish the flowcharts is unknown.
  There is no CI guard against MTS text, and TR-1 stays unticked.

## 2026-10-23 - Manchester Triage System research: plan.md §15, tasks TR-1 to TR-13

- `plan.md` §15 records the MTS research (method, five categories and target times, safety evidence, fit with this
  project, risks, open questions OQ-T1 to OQ-T4). `tasks.md` gains 13 unticked `TR-` items. Emergency triage is
  post-1.0, confirmed by the user. `TR` is added to `PREFIXES` in `scripts/generate-status.sh`.
- **Not done or not verified:** only abstracts and summaries were read, not the MTS book, flowcharts or licence terms,
  so the licence, the 3rd-edition changes and the target times are **[uncertain]**. The OQ entries are not yet in
  `spec/16-open-questions.md`, no hazard-log rows are written (TR-4), and no code or spec page changed.

## 2026-10-23 - A typed service layer under the JSON and FHIR patient routes (CQ-13, for the handlers FHIR uses)

- The shared helpers (`authorize`, `audit_resource`, `begin`, `commit`, `purpose_of`, `consent_verdicts`) return
  `ApiError` instead of a boxed `Response`.
- Patient create, read, update and search, and the allergy list, are now `*_service` functions returning
  `Result<Domain, ApiError>`: `Stored`, a new `PatientView` (the stored version, `photo_version`, `replaced_by`,
  `replaces`), `Vec<Stored>` and `AllergyList`. The JSON handlers render them; the FHIR controller calls the same
  services and maps an `ApiError` to an `OperationOutcome`.
- `fhir.rs` no longer calls a JSON handler, `read_back` is gone from its code (it remains as a test helper), and so
  are the `unwrap_or_default` fallbacks that turned an unreadable body into an empty result. A source census
  (`tests/fhir_census.rs`) fails if one returns.
- Wire shapes are unchanged: the 154 unit tests, the FHIR tests and the full Postgres suite (including the three
  handler-level files) pass without edits.
- **Not done or not verified:** the other handlers (consents, relationships, links, photos, allergy writes,
  break-glass, `me`, audit, feed) still build a `Response` and use `into_response()` on the helpers' errors; the
  UI and Playwright gates were not run, though the JSON bodies are meant to be identical (`PatientView` flattens the
  stored version); the FHIR allergy bundle is still built from the list serialised to JSON. Not run on GitHub (CQ-20).

## 2026-10-23 - Enum codes for conflicts (CQ-19, tenth and last listed slice)

- Matcher (unreleased, not in the service): `ConflictCode` (nine variants, with the field, similarity, minimum or
  percentage where one applies) and `MatchResult.conflict_codes`, beside the English `conflicts`, which are now generated
  from the codes and read exactly as before. Additive: no score or decision changes, so this is not clinical and no hazard
  row changed. `tests/codes.rs` (5). Gates clean: fmt, clippy `-D warnings`, 91 tests.
- **Why beside, not instead:** the UI prints `conflicts` as received (`patients/compare` and `patients/new`), so the
  English text appears untranslated in every locale, and the service writes it into link notes (`links.rs`). Neither was
  changed; moving them to the codes is not done and would let the UI translate them (a UI task, not yet in `tasks.md`).
- **Not done or not verified:** `FieldScore.field` is still a string (already a stable identifier), not an enum. The
  service and UI were read, not run. **CQ-19 is not ticked:** all eleven rules now have code and a named test, but its
  acceptance also asks the ID-1 labelled corpus to show no increase in false AutoLinks, and that corpus does not exist,
  so none of the ten slices is measured on real names. No matcher release or service bump. Not run on GitHub (CQ-20).

## 2026-10-23 - Phonetic signal (CQ-19, ninth slice)

- **Clinical.** Matcher (unreleased, not in the service): American Soundex on the transliterated family and first given
  name, kept out of the score. If both agree (`Wu`/`Woo` with `Li`/`Lee`), the spellings differ, the birth date matches
  and the spelling alone would give `NonMatch`, the pair goes to Review with a stated conflict. `MatchResult` gains
  `phonetic_match` (serde default, so older JSON loads). `tests/phonetic.rs` (6). Gates clean: fmt, clippy `-D warnings`, 86
  tests. H-001 extended, status untouched.
- **Not done or not verified:** Soundex is coarse and English-biased: it keeps the first letter (`Katarzyna` and
  `Catarzyna` differ), drops vowels so `Wu`, `Woo` and `Wei` collide, and says nothing about non-Latin scripts. I
  probed nine pairs: with an exact birth date most sound-alike pairs already reach Review on spelling, so the rule
  fires only when other fields disagree (a move and a new phone). Double Metaphone or a name-origin-aware encoder was
  not tried. My first test pair (`Kowalczyk`/`Kovalchik`) did not share a code, which is how I learned the first-letter
  limit. No ID-1 corpus, so the extra Review volume is unmeasured. One rule of CQ-19 remains (enum codes). No release.
  Not run on GitHub (CQ-20).

## 2026-10-23 - Nickname table (CQ-19, eighth slice)

- **Clinical.** Matcher (unreleased, not in the service): `Bill` and `William`, `Liz` and `Elizabeth` and 36 other
  groups of English given names now score 0.8 instead of whatever Jaro-Winkler gave. 0.8 is under the 0.85 given-name
  guard, so such a pair can reach Review but never auto-links on the name alone. `Alexander` and `Alexandra` stay
  apart though `Alex` joins both. `tests/nicknames.rs` (4). Gates clean: fmt, clippy `-D warnings`, 80 tests. H-001
  extended, status untouched.
- **Not done or not verified:** the list is mine, from memory, not from a published source, and has not been reviewed
  by anyone; it is English only (no Welsh, Irish, South Asian or other naming practice), so `Mohammed` spellings are in it only
  as a stopgap and score no higher than a nickname. Family-name nicknames and middle names are not covered. Whether 0.8
  puts too many pairs in Review is unmeasured (no ID-1 corpus). Two rules of CQ-19 remain (phonetic signal, enum codes).
  No release. Not run on GitHub (CQ-20).

## 2026-10-23 - Household evidence (CQ-19, seventh slice)

- **Clinical.** Matcher (unreleased, not in the service): relatives, flatmates and a care home share a postcode and a
  phone. The postcode field, and the telecom field unless an email address is shared, now count at half weight, so a
  household adds less to a score, and a mismatch (a move, a new phone) costs less. Scores of records that carry these
  fields change; identical records still score 1.0. `tests/household.rs` (5); the earlier tests are unchanged. Gates
  clean: fmt, clippy `-D warnings`, 76 tests. H-001 extended, status untouched.
- A first design, scoring a household match at 0.7, was dropped: it capped identical records at 0.94, below where a
  small typo would still auto-link, and the existing `identical_records_auto_link` test caught it.
- **Not done or not verified:** the relatives test (`a_household_cannot_lift_relatives_out_of_non_match`) also passed
  before the change, so it is a guard, not proof of a fix; only the two weight tests failed first. Half is a guess
  with no ID-1 corpus. The matcher cannot tell a household from a person who lives alone; a person who shares a phone
  with a spouse now has weaker supporting evidence. Three rules of CQ-19 remain (nickname table, phonetic signal, enum
  codes). No release. Not run on GitHub (CQ-20).

## 2026-10-23 - PA-1: boundary with the PAS, proposed

- Docs only. `spec/02-scope.md` gains the EPR/PAS ownership table; `spec/16-open-questions.md` gains OQ-PA1 to OQ-PA4;
  CA-1, CA-3, CA-4 and CA-5 in `tasks.md` carry a boundary note. The recommendation is a minimal scheduler behind a
  scheduling port (plan.md §18.2, option 2).
- **Not done or not verified:** this is a **proposal, not a decision**: only the maintainer can decide OQ-PA1, so PA-1 is
  not ticked and the CA items keep their original acceptance text. The table is drawn from the PAS's documents, not its
  code. No port interface exists, and nothing is built.

## 2026-10-23 - E.164 phone normalisation (CQ-19, sixth slice)

- **Clinical.** Matcher (unreleased, not in the service): `07700 900123`, `+44 7700 900123`, `0044 7700 900123` and
  `+44 (0)7700 900123` are now one number; before, only the bare digits were compared, so the national and
  international forms never matched. A number under 7 digits is now absent, not evidence (it could match another
  short number). `MatchConfig.default_calling_code` (44) says what a leading `0` means; `validate()` checks it.
  `tests/phone.rs` (7). Gates clean: fmt, clippy `-D warnings`, 71 tests. H-001 extended, status untouched.
- **Not done or not verified:** only the default country's trunk `0` is dropped after a country code; other countries'
  trunk rules, number-length validity and mobile or landline type are not checked. A shared phone still counts as
  agreement between two people (household evidence is the next rule and would down-weight it). A `MatchConfig` built
  as a struct literal without `..Default::default()` no longer compiles (a new public field). No ID-1 corpus. Four
  rules of CQ-19 remain. No release. Not run on GitHub (CQ-20).

## 2026-10-23 - Planning: typical EPR modules (EM) and the PAS (PA)

- Docs only. `plan.md` §17 (typical modules, gap analysis against the pasted six-group list and three web searches) and §18
  (the Patient Administration System as a neighbour, from its documents); `tasks.md` gains EM-1 to EM-16 and PA-1 to PA-17;
  `scripts/generate-status.sh` knows the `EM` and `PA` prefixes. No code, spec, hazard row or test changed.
- **Not done or not verified:** the web evidence is search summaries only (the NHS Digital Capabilities Framework, the
  HIMSS EMRAM criteria and the PRSB discharge standard were not read). For the PAS I read its documents, not its code,
  tests or migrations, and checked nothing running. Its claim of SNOMED acuity concepts contradicts our triage spec
  and is unreconciled (PA-10). The new items are candidates, not commitments: EM-1 and PA-1 decide scope. No hazard
  rows were added (EM-16).

## 2026-10-23 - `\p{M}`-aware name fold (CQ-19, fifth slice)

- **Clinical.** Matcher (unreleased, not in the service): `fold_name` kept only alphabetic characters, so a virama
  (U+094D) became a space: `कृष्ण` folded to `कृष ण` and was compared as two parts. Combining marks after a letter
  are now kept. This changes scores for names in Indic and other scripts with such marks (fewer false fragments,
  so more exact matches and sharper mismatches); Latin names are unchanged. A test failed first (`tests/marks.rs`, 5);
  gates clean: fmt, clippy `-D warnings`, 64 tests. H-001 extended, status untouched.
- **Not done or not verified:** only one Devanagari and one Arabic-script name were tested; no Persian, Tamil, Thai or
  other script, and no Indic transliteration or phonetic comparison. Arabic harakat were already kept (they are
  alphabetic in Rust) so that test passed before the change. No ID-1 corpus. Five rules of CQ-19 remain. No release.
  Not run on GitHub (CQ-20).

## 2026-10-23 - Given/family swap detection (CQ-19, fourth slice)

- **Clinical.** Matcher (unreleased, not in the service): `Taylor Morgan` against `Morgan Taylor` was a NonMatch on the
  names; it now scores both name fields at 0.9 of the swapped similarity and goes to Review with the conflict "given and
  family names appear swapped", never AutoLink. The same pair written the right way round still scores higher.
  `tests/swap.rs` (5: Review with conflict, symmetric, below the unswapped pair, unlike names unaffected, a direct
  given match rules a swap out). Gates clean: fmt, clippy `-D warnings`, 59 tests. H-001 extended, status untouched.
- **Not done or not verified:** only the first given name and a single family string are used, so a swap across
  several given names or compound family names is not seen; a shared valid NHS number with swapped names can still
  AutoLink (the number decides, and the penalised score no longer trips the "names disagree" check), which I judged
  right for a keying slip but did not test. The 0.9 and 0.85 are guesses with no ID-1 corpus. Six rules of CQ-19
  remain. No release. Not run on GitHub (CQ-20).

## 2026-10-23 - Transliteration (CQ-19, third slice)

- **Clinical.** Matcher (unreleased, not in the service): family and given names are also compared transliterated, and
  the better score is kept. `Müller`/`Mueller` and `José`/`Jose` now score about 1.0; `Müller`/`Miller` stays below
  0.95. Scores can rise, never fall. `tests/transliteration.rs` (5). Gates clean: fmt, clippy `-D warnings`, 54 tests.
  H-001's controls and tests are extended; its status is untouched.
- **Not done or not verified:** the table is German digraphs plus Latin base letters only: no `æ ø å` (`Soren`/`Søren`
  is not folded, since `ø` has no decomposition), no `ç`-style language rules, no Cyrillic, Greek, Arabic or CJK
  transliteration (`Mohammed`/`محمد`). Higher scores may add false Review or AutoLink pairs; there is no ID-1 corpus
  to measure that. Seven rules of CQ-19 remain. No release. Not run on GitHub (CQ-20).

## 2026-10-23 - `MatchConfig::validate()` (CQ-19, second slice)

- Matcher (unreleased): `validate()` and `MatchingEngine::try_new` refuse a negative or non-finite weight, all-zero
  weights, a threshold outside 0 to 1 or NaN, and a review line above the auto-link line. `tests/config.rs` (5) has one
  test per refusal. Gates run clean from `cargo clean -p`: fmt, clippy `-D warnings`, 49 tests pass. This is additive
  and changes no score or decision, so it is not clinical and no hazard row changed.
- **Not done or not verified:** `MatchingEngine::new` and `Default` still accept anything, so nothing forces a caller
  to validate; the service does not configure the matcher yet. Eight rules of CQ-19 remain. No release. Not run on
  GitHub (CQ-20).

## 2026-10-23 - Real-world names, first slice: compound family names and the extra given name (CQ-19, partly)

- **Clinical.** In the matcher (unreleased, not yet in the service, which uses 0.2.0):
  - a family name of several parts is compared part by part, with each part scaled by the ratio of lengths: `Smith-Jones`
    against `Smithson` fell from about 0.90 to below 0.85 (the auto-link minimum); `Smith` against `Smith-Jones`
    and `Jones Smith` against `Smith-Jones` still match fully;
  - `John` against `John Paul` (a given name on one record that nothing on the other matches at 0.85 or more) can no
    longer auto-link: the score is unchanged and the pair goes to Review with a stated conflict.
- `tests/names.rs` (4) names each rule and checks the order of the records; the 40 other matcher tests pass.
- H-001's controls are extended; its status is untouched.
- A stale Cargo fingerprint hid the new code from `cargo test` for part of this work (the library was not rebuilt
  after an edit until `cargo clean -p electronic-patient-record-matcher`); results above are from a clean build.
  Earlier green runs in this directory may have been stale. They should be rerun.
- **Not done or not verified:** nine rules of CQ-19 remain (nickname table, phonetic signal, transliteration, swap
  detection, `\p{M}` fold, E.164, household evidence, enum codes, `MatchConfig::validate()`). There is no ID-1 labelled
  corpus in the repository, so "no increase in false AutoLinks" is checked only by the existing 23 matching tests and
  the property tests, not on real names. `Jane` against `Jane Louise` (a middle name left off) now goes to Review where
  it could auto-link; that costs reviewer time and was a chosen trade, not measured. No matcher release or service
  bump. Not run on GitHub (CQ-20).

## 2026-10-23 - Consent and relationship conflicts, FHIR miss and search refusal; CQ-16 closed

- `http_conflicts.rs` also checks the stale-change 409 for consents and relationships, a FHIR `Patient` miss (404)
  and an unsupported `birthdate` prefix (400), both as `OperationOutcome`s.
- CQ-16 is ticked on its acceptance ("or is listed as still untested"): the service spec §2a lists what is covered
  and what is not.
- **Not done or not verified:** over HTTP, no patient or proxy token, access-log route, audit export, HL7 feed
  refusal, or Keycloak-expiry 401; the audit row of each refused write is not checked. Not run on GitHub (CQ-20).

## 2026-10-23 - Version conflicts and merge refusals through the router (CQ-16, third slice)

- `tests/http_conflicts.rs` (1, Postgres, login off) checks the exact 409 body `{"error": "version conflict",
  "current_version": n}` for a stale patient `PUT`, allergy change, allergy statement, photo `PUT` and photo
  withdrawal. It also checks that merging a record into itself and unmerging a record that is not merged are 422 with
  the reason.
- `http_roles.rs` and the break-glass test written in a second session were folded into one test; the duplicate file
  `http_authenticated.rs` is gone.
- Links have no sequential 409: a refused merge is a 422, and the merge race that ends in 409 is covered only at the
  store (`links_store.rs`), not over HTTP.
- **Not done or not verified:** CQ-16 stays open. Not covered: 409s for consents and relationships, the other FHIR
  refusals, and a patient or proxy token. The audit row written for each refused write is not checked here. Not run
  on GitHub (CQ-20).

## 2026-10-23 - Matcher property tests and mutation testing (CQ-30, partly)

- `tests/properties.rs` (6) over 20,000 seeded random patients each: `compare` is total and its score is in [0, 1]; a
  record compared with itself reports no clash; symmetry when the given-name lists are the same length; dropping a
  birth date, sex, postcode or telephone never creates an auto-link on pairs without a shared NHS number (a field that
  disagreed is not dropped: removing a contradiction may legitimately raise the score); and a pin of the finding below.
- **Finding (CQ-37, not fixed):** a shared valid NHS number gives `AutoLink` with no names or birth date on either
  record, and removing one side's conflicting birth date turns `Review` into `AutoLink`. That differs from the rule for
  every other pair. Behaviour is unchanged, a test pins it, and the H-001 row of the hazard log now says so (status
  untouched). It needs a clinical decision.
- `cargo mutants -f src/engine.rs -f src/nhs.rs`: the first run missed 23 of 152 (mostly the sex, postcode and
  telephone similarity, and birth-date precision). `tests/fields.rs` (11) pins those, and the last run caught 147 and
  found 5 unviable, with none missed or timed out.
- **Not done or not verified:** the `cargo fuzz` target (needs nightly; CQ-30 stays open for it); mutants on
  `normalize.rs` and `patient.rs`; the generator is hand-rolled with a fixed seed, so it explores one sequence, not a
  shrinking search; the symmetry property excludes pairs whose given-name lists differ in length, where CQ-1 does not
  promise it. Not run on GitHub (CQ-20).

## 2026-10-23 - Stale documents brought up to date (CQ-28, partly)

- `spec/14-implementation-status.md`: the hand-written area table is gone; a paragraph points to each project's
  `spec/index.md` and this changelog, and the generated block stays.
- UI `AGENTS.md`: lists the patient, my-record and admin pages that exist. `compose.yaml`: the header no longer says
  authorization is not built. `compose.test.yaml`: the `--test mod` instruction (no such test target) is replaced, the
  alternative-ports commands are given, and the container names lose the `mxi-` prefix (the running test containers
  keep the old names until recreated). `plan.md`: the header no longer says no code exists.
- Service and UI `CHANGELOG.md` gain an "Unreleased" note pointing at this file. The matcher's already stops at its
  0.2.0 entry, which is current.
- **Not done or not verified:** a reviewer has not read the files for contradictions, so CQ-28 stays open. The UI page
  list comes from the route directory, not from running the UI. `podman compose config` accepts the test compose file;
  it was not brought up under the new names.

## 2026-10-23 - Request id in the audit trail (CQ-17, partly)

- A new outermost middleware settles `x-request-id` (the caller's, cleaned, or a UUID) before anything else, and every
  audit row written during the request carries it as `detail.request_id`. The response header is the same id. A
  `uuid` dependency was added (already in the lock through loco).
- loco's `TraceLayer` and request-id middleware were already on by default (its `logger` middleware); I did not
  change or test them beyond the id agreeing with the audit row.
- Tests: 1 unit in `authz`, 1 in `audit`, 1 Postgres (`http_patients.rs`: a supplied id is cleaned, echoed and found in
  exactly one audit row; a generated one too).
- **Not done or not verified:** the Prometheus counters and `/metrics` (CQ-17 stays open; it needs a dependency, a
  gated route and a policy decision on who may read it). The id is caller-supplied, so two requests can share one on
  purpose. Rows from the HL7 feed and background work have no request, so no id. Not run on GitHub (CQ-20).

## 2026-10-23 - Role-token handler tests: route gate, consent masking, break-the-glass (CQ-16, partly)

- `tests/http_roles.rs` (1, Postgres) boots the app with login **on**, a local OIDC stand-in and a role map, and
  mints tokens for `clinical-admin`, `clinician` and `auditor`. It checks: an auditor registering a patient is 403
  `{"error":"forbidden"}` on `/api` and an `OperationOutcome` on `/fhir`; no token is 401; with a deny-for-operations
  consent on the existing record, the duplicate 409 shows only `{"restricted": true}` and never the name; a clinician
  with no relationship is 403 with `break_glass_offer`, a blank reason is 422, a grant is 201 and asking again 200,
  the read then succeeds, and the flagged audit row and the review queue name the patient. Another session wrote
  an overlapping `http_authenticated.rs`; it is folded in here (same `expires_at` on a repeat grant, exactly three
  flagged rows for the patient and no free text in them, the review count) and deleted.
- `http_patients.rs` and `http_roles.rs` now use a letters-only name, a given name and a birth date unique to each run:
  leftover patients from earlier runs made the first registration a 409 (the digit-only names scored as similar).
  This was a latent flake in the first slice, found while writing this one.
- **Not done or not verified:** CQ-16 stays open. Not covered: 409 shapes for links, allergies and photos, the other
  FHIR refusals, and a patient or proxy token. The Keycloak stand-in is Keycloak-shaped, not Keycloak. The whole
  scenario is one test because the guard is configured once per process, so a failure stops later checks. Not run on
  GitHub (CQ-20).

## 2026-10-23 - First handler-level tests through the real router (CQ-16, partly)

- `tests/http_patients.rs` (2, Postgres) boots the app with `loco_rs::testing::request` (real routes, route gate, audit
  chain, login off) and checks: a likely duplicate is 409 with candidates, a blank `override_reason` is still 409, a
  real reason creates; on `/fhir/R4/Patient` a `PUT` without `If-Match` is 428, a stale one 412, a duplicate 409 as an
  `OperationOutcome` that does not contain the family name, and an element the record cannot hold is 422.
- `config/test.yaml`: `dangerously_truncate` and `dangerously_recreate` are now `false`. With them on, every boot
  dropped the tables but left the audit trigger function, so the next migration failed; the other DB tests migrate
  with `Migrator::up` and never relied on them.
- **Not done or not verified:** CQ-16 stays open. Not covered: consent masking of candidates and the break-glass flag
  (both need tokens with roles; the harness runs with login off), 409 shapes for links, allergies and photos, and the
  other FHIR refusals. Not run on GitHub (CQ-20).

## 2026-10-23 - Down migrations refuse to drop clinical tables (CQ-15)

- The nine migrations that create tables (audit, patient, consent, relationship, photo, allergy, link, feed
  refusal, break-glass) now fail in `down` before changing anything, unless `LOCO_ENV` is `development` or `test`
  and `EPR_ALLOW_DESTRUCTIVE_DOWN=true`. An unset `LOCO_ENV` counts as production.
- Tests: 1 unit (the rule), 1 Postgres (`tests/migration_down.rs`: with `LOCO_ENV=production` and the flag set,
  `Migrator::down` errors and five tables remain).
- **Not done or not verified:** the audit and version tables have delete triggers, but a database owner can still
  drop them by hand; no database role is denied `DROP`. The test reverses migrations 12 and 11 (views and indexes,
  unguarded) before it hits the guard; the next `up` restores them. `db migrate down` through the loco CLI was not
  run. Not run on GitHub (CQ-20).

## 2026-10-23 - Fail loudly on bad input and export (CQ-14)

- An `AuditEvent` search with a parameter the service does not know (a misspelt `agnet=`) was answered with the
  unfiltered trail. It is now 400 `OperationOutcome`; `_`-prefixed result parameters are still ignored. An
  unparseable query string is also 400.
- The NDJSON audit export dropped a row it could not write. It now fails the export (500), before the export is
  audited, so nothing is recorded as having left.
- `EPR_PASETO_KEYS` set but not valid JSON now stops startup; it was read as an empty key set.
- The CSV export's formula guard (a leading apostrophe) means a guarded cell no longer equals the stored value; this
  is now stated in the code, and the NDJSON export is the one to re-verify the chain from. The two were not
  separated.
- `links::merged_into` follows 16 merges; a longer chain was cut at the sixteenth record and returned as if it were
  the top. It is now an error (a Postgres test with 17 merges). A deeper chain makes the handlers that call it
  answer 500 until the chain is unmerged.
- Tests: 2 new unit (keys, NDJSON), 1 extended (the unknown search parameter), 1 new Postgres.
- **Not done or not verified:** no handler-level test of the 400 and 500 responses (CQ-16); the chain error is
  not mapped to a message for the clinician; no merge refuses to make a chain longer than 16; the hazard log is
  unchanged.

## 2026-10-22 - Shared store helpers; a lost version race is read from the error kind (CQ-12)

- Eight store modules each had their own `pg` and `db_err`; `audit.rs` had its own `pg`. Six copies of the
  savepoint-insert-rollback mechanics appended a version, and each detected a lost race by finding
  `"…_pkey"` in the error message. `src/store.rs` now has the shared pieces:
  - one `pg`;
  - `db_err`, which names the store. Each module calls `store_errors!("consents")` for a local `db_err`, so the log
    says which store failed;
  - `is_unique_violation`, read from SeaORM's `sql_err()`, not the message;
  - `append_version`, which returns the new row, or `Taken` when another writer took the version.

  The patient, consent, relationship, allergy, no-known-allergies and photo stores use it.
- Tests:
  - two Postgres tests force the race that had no test before. A first writer holds version 2 uncommitted, and a
    second, which also saw version 1, blocks on the key until the first commits. The second must get
    `Conflict { current: 2 }`. They passed before the change too, as guards, and fail when the constraint match is
    broken. They cover the patient and consent stores;
  - a source census fails on a local `fn pg(`, `fn db_err(` or a `_pkey")` text match.
  - 165 unit and 73 Postgres tests pass.
- **Not done:**
  - `StoreError::Db` is still a string (with the store's name), not the `DbErr`, because tests compare `StoreError`
    values;
  - the race is tested for two of the six stores;
  - no test reads the log line.

## 2026-10-22 - One error type and one shared controller module (CQ-11)

- `src/api_error.rs` defines every error body once. `ApiError` is `{"error": …}` with optional extra fields, or 422
  `{"errors": […]}`, as JSON, or as an `OperationOutcome` under `/fhir/` (`for_path`). `From<&StoreError>` replaces
  the store-error mapping. The 500 body is still `internal error`, and the log now says "store failed", not "patient
  store failed", for every store.
- The 14 helpers ten controllers imported from `controllers/patients.rs` (`authorize`, `purpose_of`, `begin`,
  `commit`, `audit_resource`, `store_failure`, …) moved to `controllers/common.rs`. `json_error` is gone; about 30
  hand-built bodies became `ApiError` calls. Net 249 lines fewer.
- The route gate answered a 403 as plain text `forbidden`, even on `/fhir/R4`, where a client expects an
  `OperationOutcome`. The authentication middleware answered a 401 as plain text too. Both now answer by path: JSON
  on `/api`, `OperationOutcome` on `/fhir`.
- The REST bodies are unchanged except the gate's 403 and 401, which are now JSON; the UI reads only the status.
- Tests:
  - a source census (`tests/error_census.rs`) fails on a hand-built `"error":`/`"errors":` body, `json_error(`, or
    a plain-text refusal in controllers, `auth.rs` or `authz.rs`. It found 72 lines before. One exemption: the
    audit chain's 424 verification report, which carries `error` as one field of a report;
  - 2 unit tests for the body shapes and the FHIR form;
  - a Postgres test of the real gate: 403 on `/fhir/R4/Patient/{id}` is an `OperationOutcome`, and on `/api` it is
    `{"error": "forbidden"}`. It fails without the change.
  - 164 unit and 71 Postgres tests pass.
- **Not done:**
  - handlers still return `Result<_, Box<Response>>` and use `return *r`, not `?`. `From<ApiError> for Box<Response>`
    bridges them; moving to `?` is part of the typed service layer (CQ-13);
  - the FHIR layer still reads `body["error"]` back from the JSON handlers (CQ-13);
  - no handler-level test (CQ-16);
  - not run against a live stack.

## 2026-10-21 - The duplicate check keeps the strongest candidates when it must cut (CQ-36)

- The duplicate check compared at most 200 candidates, taken in patient-id order, which is arbitrary. When more than
  200 records shared the birth date or the family name, the real duplicate could be dropped without notice (found by
  the CQ-10 test). Candidates are now ranked before the cut:
  1. a shared identifier;
  2. the same family name and birth date together;
  3. birth date alone;
  4. family name alone.

  The query fetches one more than the limit, so it knows when it cut. `candidates_ranked` returns the list and the
  flag; `candidates` keeps its signature.
- A cut list is reported: `candidates_cut` in the 409, in the registration's audit row (refused or overridden) and in
  the HL7 feed's audit row. The registration page then adds "only the closest 200 were compared… search before
  registering anyway". Hazard H-002 updated.
- Tests:
  - a Postgres test puts 250 records on one birth date. The record sharing the NHS number ranks first, the one with
    the same family name and birth date second, and the cut is reported. It failed with the old ordering;
  - a Playwright test with axe: the note appears for a cut list and not otherwise.
  - 161 unit and 70 Postgres tests pass; check clean, 98 vitest, 43 Playwright.
- **Not done:**
  - a record sharing only a name or only a birth date can still be cut. A Smith with a mistyped birth date is the
    case that matters, and catching it needs proper blocking keys (RC-2);
  - an HL7 create with a cut list is not refused (that would refuse every common family name). It is only flagged in
    the audit row.

## 2026-10-21 - Consent for many patients in two queries (CQ-10)

- The consent check behind search and the duplicate check read each patient's merged-in records and their consents
  one patient at a time: about 100 queries for 50 search results. `links::sources_of_many` finds every patient's
  merged-in records in one recursive query. `consents_store::active_with_merged` reads all their active consents in
  one more and assembles the same lists as before. A patient with merged-in records but no consent keeps its empty
  entry, as before.
- Tests: a Postgres test compares the batched answer with the old one-at-a-time answer. It covers a three-record merge
  chain with refusals on two records, a lone refusal, a patient with nothing, a merged pair with no consent, an
  upper-case id and a malformed one. With the merged-in consents dropped (a mutation) it fails.
- **Found:** the test first broke `links_store`. Its patients shared a birth date with that test's, the test database
  passed 200 patients with that date, and the duplicate check's `LIMIT 200` (in patient-id order) dropped the real
  duplicate. That is a defect, not only a test problem: a common birth date in a large population has more than 200
  people. Recorded as CQ-36 and in hazard H-002. The test now uses its own birth date, and the throwaway test
  database was recreated (it is append-only, so the rows could not be removed).
- **Not done:** the number of queries is two by construction; it is not counted by a test.

## 2026-10-21 - Patient search by identifier or birth date seeks an index (closes CQ-9)

- `patient_current` was `DISTINCT ON (patient_id)`, and search filters it on the record body. No index could help,
  because the filter could not reach `patient_versions`. Migration 12 makes it an anti-join, like migration 11 did for
  the others. It adds a GIN index on `body->'identifiers'` (`jsonb_path_ops`) and an expression index on
  `body->>'birth_date'`, so search by NHS number or birth date seeks an index.
- Tests: `patient_current` joins the equivalence test (same rows as the old form). A plan test needs each index for
  its filter, with no `Unique` step; it failed before the migration. 161 unit and 68 Postgres tests pass.
- **Not done:**
  - search by family name (a prefix `LIKE` over the names array) and the duplicate check's family arm have no index.
    The duplicate check ORs that arm with the others, so it still scans. A folded-name index is a design choice
    (an immutable function, perhaps `pg_trgm`), recorded under RC-2;
  - `down()` not run;
  - nothing measured on a large table.

## 2026-10-21 - A patient's consents, allergies, relationships and links seek an index (CQ-9, in part)

- `consent_current`, `allergy_current`, `relationship_current` and `patient_link_current` were `DISTINCT ON` the
  record's own id. A page asks for one patient's records, and Postgres cannot push that filter below `DISTINCT ON`, so
  every such read built the current version of every record in the table first. Migration 11 replaces the four views
  with an anti-join (a version with no later version of the same record). Same columns, same rows; the patient filter
  now uses the versions table's patient index. `spec/10-persistence.md` states the rule.
- Tests (`tests/current_views.rs`, Postgres):
  - each new view equals the old `DISTINCT ON` form, by `EXCEPT` both ways. The test data has 8, 20, 4 and 183
    records with several versions;
  - a patient filter on each view gives an `Index Cond` and no `Unique` step. This test failed before the migration.
  - 161 unit and 67 Postgres tests pass.
- **Not done:**
  - `down()` (it restores the old views) was not run;
  - nothing was measured on a large table;
  - the search indexes on identifiers, birth date and folded family name are still missing.

## 2026-10-21 - Ids are compared as UUIDs, so lookups can seek the index (CQ-9, in part)

- 14 queries compared an id column as text (`patient_id::text = lower($1)`, `… = ANY($1::text[])`). That hides the
  column from its index, so a lookup scans the whole index and filters. On the test database (461 patient versions)
  the plan for a patient read went from cost 114 to 8. Every one now binds `$n::uuid` or `$n::uuid[]`.
- A new `ids::uuid` checks an id from a URL first, so a malformed id is still "not found" (or an empty list) and never
  a database error, and an upper-case id is still found. The feed queue's own copy of that check is gone.
- Tests:
  - a source census (`tests/sql_census.rs`, no database) fails on any `_id::text =`. It listed all 14 before the
    change;
  - 3 unit tests for `ids`;
  - a Postgres test: an upper-case id is found, malformed ids are not, and `patient_current` seeks on the id
    (`Index Cond`).
  - The new tests and the existing 157 unit and 64 Postgres tests pass.
- **Found, not fixed here:** `consent_current`, `allergy_current` and `relationship_current` are `DISTINCT ON` the
  record's own id, so a filter on `patient_id` cannot be pushed below it. Postgres still builds the current version of
  every consent, allergy or relationship before it filters (checked with EXPLAIN). That needs the views changed, in a
  migration.
- **Not done:** the search indexes (identifiers, birth date, folded family name).

## 2026-10-21 - The status page counts every task prefix (CQ-29, in part)

- `scripts/generate-status.sh` counted ten fixed prefixes, so the 35 `CQ-` tasks were missing from the totals in
  `spec/14-implementation-status.md`. It now counts `CQ` (9 ticked, 26 open; 20 and 111 in all), and fails (exit 2)
  when `tasks.md` uses a prefix it does not count, so a new prefix cannot go missing. A probe task with prefix `ZZ`
  was refused.
- **Not done:** trimming the five task entries of 1,500 to 2,300 characters to a status line and a changelog link.
  Their "Not built" lists are the record of what is open, and each must be checked against its changelog entry
  first.

## 2026-10-21 - CI runs every test without naming files (CQ-24)

- The `api` job named three test targets (`--lib`, `keycloak_guard`, `route_census`). A new test file whose tests need
  no database would have run nowhere, and the migration crate's tests and the doctests never ran. It now runs
  `cargo test --workspace --locked`.
- The `api-db` job looped over `tests/*.rs`. It now runs `cargo test --workspace --locked --no-fail-fast -- --ignored
  --test-threads=1`, which reaches an ignored test anywhere in the workspace and reports every binary after a
  failure.
- `api-db` applies `ci/postgres-init/00-extensions.sql` before the tests, as compose does on a fresh data directory, so
  the day an extension is added the CI database has it too. The matcher job is `--locked` too.
- Checked locally: the `api` command runs 157 tests (and the migration crate and doctests, 0 of each today); the
  `api-db` command runs all 64 database tests against Postgres 18; the init SQL applies.
- **Not done:** not run on GitHub (CQ-20); the UI job is unchanged; no `--locked` equivalent check for the UI beyond
  `pnpm install --frozen-lockfile`, which it already has.

## 2026-10-21 - Hazard-log gate: label trigger and sign-off by reviewer only (CQ-21)

- The `clinical-impact` hazard-log job ran only on the default pull-request events, so a label added after a PR was
  opened was never checked. The trigger now includes `labeled` and `unlabeled`.
- `check-hazard-log.py` only required `reviewed_by` to be non-empty, so any author, an agent included, could type a
  name and close a hazard. With `--base <ref> --author <login>` it now compares the log with the base revision. It
  fails a change that sets `accepted` or `closed`, adds a hazard in either status, or changes `reviewed_by`, unless the
  author is in `governance/clinical-safety/reviewers.txt` as that file is on the base branch, so a PR cannot add itself.
  It fails any change that removes a hazard. The allow-list is empty, as no Clinical Safety Officer is named (OQ-2).
  A new CI job `hazard-signoff` runs this on every PR and is part of `ci-ok`.
- The checker now requires `added` to be a real `YYYY-MM-DD` date. CODEOWNERS has an explicit
  `governance/clinical-safety/` entry, for the CSO to be added. The risk management plan says all this.
- Tests: 8 new checker tests (dates; sign-off, new rows, reviewer changes, reopening, removal, the allow-list
  parser), all failing before the change. A by-hand run that closed H-025 as "Mallory" was refused.
- **Not done:**
  - CI has never run (CQ-20), so neither job has run on GitHub, and the workflow was not checked with actionlint.
  - A GitHub login is not a person's identity check. Branch protection and the CODEOWNERS review are still needed
    (not configured: there is no remote).
  - Nothing warns when clinical paths change without the label.

## 2026-10-21 - The service uses matcher 0.2.0 (closes CQ-2)

- Matcher 0.2.0 is published, and its source on crates.io is identical to this repository's. The service now depends
  on it, so the CQ-1 to CQ-5 matching fixes reach registration, the duplicate check, merge and the HL7 feed.
  Registration refuses a repeated-digit placeholder NHS number with its own message ("NHS number is a placeholder…"),
  not the modulus-11 message, which a placeholder passes. Hazard H-001 no longer says the controls are missing from
  the service.
- Tests: one service unit test for the placeholder message; all 154 unit and 64 Postgres tests pass on 0.2.0.
- **Not done:** the UI shows the server's message as it comes. No UI test covers the new message, and none is
  translated.

## 2026-10-21 - Two registrations of one person at once make one record (CQ-7)

- The duplicate check ran before the creating transaction and nothing serialised registration, so two registrations
  of the same person at the same moment (two clerks, a clerk and the HL7 feed, two of the feed's eight connections)
  both found no candidate and both created a record. Registration (`POST /api/patients`, and so the FHIR create) and
  the HL7 ADT create now take a transaction-scoped advisory lock (`patients_store::lock_registration`, "EPRR") and run
  the check on that transaction. The ADT path's earlier check outside the transaction is gone (it ran twice).
- The first version of the handler change held the transaction and then used the pool for the consent check and the
  refusal's audit row. With the development pool of one connection, every registration would have hung. The feed
  tests' one-connection pool (and their comment) showed it before it ran. Both now use the transaction, and a refusal
  commits its audit row alone. Lock order is registration, then audit, everywhere.
- Hazard H-002 updated; service spec row "Duplicate check" updated.
- Tests: a Postgres test races two creators for one person through the lock; without the lock (a no-op stub) both
  created, with it one does. All 64 database tests and the unit tests pass; clippy clean.
- **By hand**, in development (one-connection pool) against the test database: two concurrent `POST /api/patients` for
  one person gave one 201 and one 409 naming the first; a third gave 409; one record exists; the two refusals' audit
  rows were committed.
- **Not done:**
  - an automated handler-level test (CQ-16);
  - a measurement of the cost of serialising registration (one at a time; registration is low-volume);
  - the same protection for merge-then-register orderings, not examined.

## 2026-10-21 - The idle sign-out holds on an open patient page (CQ-8)

- The patient page looks at the record and its allergies once a minute, and every proxied request counted as activity,
  so a workstation left open on a patient page never reached the 15-minute idle limit (only the 8-hour one). The page
  now marks those looks `x-epr-background: 1`; the hook and the proxy look the session up without counting them, and
  the header is not forwarded to the API.
- Two minutes before the idle limit the layout shows an alert ("You will be signed out in about N min…") with "Stay
  signed in", which posts to a new same-origin `/auth/keepalive` (no API call, so no audit row). The page's own note of
  activity can only lag the server's, so the warning comes early, never late. New hazard H-025.
- Tests: 6 unit tests (the store ignores background requests and the session still ends on time; the header rule; the
  client sends the marker only on a background call; the warning's minutes); 3 of them, and the new module, failed
  before the change. `pnpm run check` 0 errors, 98 vitest, 42 Playwright.
- **Not done:** an end-to-end test that sees the warning. The stubbed suite runs with sign-in off, and the real-Keycloak
  suite was not run (no stack or password here). Typing alone is not activity, so a clinician typing a long entry
  without saving is warned and can be signed out. The banner's photo fetch after a background look that finds a new
  photo still counts as activity.

## 2026-10-21 - Wrong patient on screen while moving between records (CQ-6)

- Following a link from one patient's page to another (a merged record to the surviving one, say) reuses the page. The
  previous patient's banner and allergies stayed on screen under the new address until the new record arrived, and an
  answer for the previous patient that arrived late was shown for the new one: a test reproduced Bob's banner reading
  "Allergies: Penicillin (high risk)", which was Ann's. The page now clears the previous patient's state at once and
  drops any answer that is not for the patient on screen or not the latest asked for (the record, allergies, the
  minute look, save and photo). Hazard H-016 updated; UI spec "Moving between patients".
- The first version of the fix made the loading effect depend on page state, so the page reloaded in a loop; four
  existing end-to-end tests caught it. The effect now depends on the patient id only (`untrack`).
- Tests: two Playwright tests with held responses (the second patient loading; a late allergy answer for the first);
  both failed before the change. The whole suite passes: `pnpm run check` 0 errors, 92 vitest, 42 Playwright.
- **Not done:** the same guard on the other pages that load by patient id (`/patients/[id]/access`, `/my-record`); the
  idle-session poll (CQ-8); the Svelte autofixer was not run on the page.

## 2026-10-21 - An empty given name is absent (CQ-5)

- A given name such as `-` or spaces folds to nothing. The family name already skipped such values; the given name
  scored them as `Some(0.0)`, breaking "absent is not zero", and an empty token beside a real one halved its score.
  Such tokens are now dropped, so a list of only empty tokens is absent. Spec invariant 4 says so. Hazard H-001
  updated.
- Tests: one test (three empty forms, and a real token beside an empty one); it failed before the change.
- **Not done:** reaching the service, which needs matcher 0.2.0.

## 2026-10-21 - Birth-date typo rule and impossible dates (CQ-4)

- The matcher gave "typo" credit (0.6) for any different day, any different month or any different year, so a parent
  and a child sharing a birthday 60 years apart scored as a keying slip. Credit now needs a slip: one digit different,
  or two adjacent digits swapped, with the day and month written as two digits and the year as four. Day and month
  swapped keeps its 0.8.
- The matcher's date parser accepted days the month does not have (`2020-02-31`). Such a date is now absent, so it
  blocks an automatic link instead of scoring. The service already refused these dates at registration; the gap was for
  records arriving any other way. Hazard H-001 updated.
- Tests: two tests (10 pairs scored, 4 impossible dates, 2 real leap days); both failed before the change.
- **Not done:** a slip in more than one component still scores 0 (a date with two slips needs a person anyway);
  reaching the service, which needs matcher 0.2.0.

## 2026-10-21 - Shared NHS number with a different birth date (CQ-3)

- The matcher spec (§4) says a shared NHS number with two different full birth dates goes to Review. The code added
  that conflict only when birth-date similarity was below 0.5, so a near date (`1980-01-01` against `1980-01-28`,
  scored 0.6 as a "day typo") was AutoLinked even with different names. The threshold is gone; the code now does
  what the spec says, so the spec is unchanged. Hazard H-001 updated.
- Tests: one test over four near and far dates; it failed before the change.
- **Not done:** reaching the service, which needs matcher 0.2.0 (see CQ-2).

## 2026-10-21 - Placeholder NHS numbers (CQ-2)

- Every NHS number made of ten copies of one digit (`0000000000` … `9999999999`) passes the modulus-11 check, and none
  is issued. Systems use them as placeholders, so two different people carrying `0000000000` with the same birth date
  were AutoLinked by the "shared verified NHS number" rule with no name check. `is_valid_nhs_number` now rejects them
  and the new `is_placeholder_nhs_number` says which; the 999 test range is unaffected.
- A shared valid NHS number now also goes to Review when both the given and the family name are below their auto-link
  minimums. A family name alone still links (marriage), and a test says so. Hazard H-001 updated.
- Tests: 4 new matcher tests; 3 failed before the change (the marriage test passed before and is a guard).
- **Found:** the service depends on the *published* matcher 0.1.0 from crates.io, not on this directory, so neither this
  change nor CQ-1 reaches the service until matcher 0.2.0 is published and the service is bumped. The service's own
  placeholder message waits for that bump.
- **Not done:** a configurable list of other placeholder or test numbers; a check that the 999 test range is refused in
  production.

## 2026-10-21 - Matcher symmetry (CQ-1)

- `compare(a, b)` and `compare(b, a)` could reach different decisions: with given-name lists of equal length the
  per-token average ran from `a` only, so `["John","J"]` against `["John","Smith"]` (same family name, birth date and
  sex) was AutoLink (0.975) one way and Review (0.875) the other. With equal lengths the average now runs both ways and
  the lower is kept, so the order never raises a score; that pair is now Review both ways. Hazard H-001 updated.
- Tests: a named regression test and a grid test over 122,500 ordered pairs (5 family names, 7 given-name lists
  including empty and `-`, 5 dates including invalid and partial, 2 sexes). Both failed before the change.
- **Not done:** proptest over generated patients (CQ-30; the grid needs no new dependency); the related twins risk
  where `John` against `John Paul` still AutoLinks because the shorter list is averaged (CQ-19); any evaluation on real
  data (ID-1).

## 2026-10-21 - API versioning rule

- `spec/09-api-surface.md` and the root `AGENTS.md` now say: version the REST API with the `Accepts-version` request header
  on `/api`, and never with `/api/v1`, `/api/v2` or any path segment. `/fhir/R4` is the FHIR release, not an API version.
- **Not done:** the rule is not enforced. Nothing reads the header, there is no list of versions or default, and no check
  refuses a route that has a version segment (the route census would not notice one).

## 2026-10-21 - Break-the-glass (SE-3)

- A treating clinician (clinician, nurse, pharmacist, allied health) with no care relationship who is refused clinical
  content gets a 403 with `break_glass_offer: true`. `POST /api/patients/{id}/break-glass` with a reason opens a grant for
  4 hours (fixed; asking again returns the same grant and does not extend it). `break_glass_grants` and
  `break_glass_reviews` (migration 10) are append-only.
- What a grant lifts is the policy engine's decision, unchanged: only the missing care relationship, only for reads of
  clinical content, only for a treating role, only until it expires. It does not lift a patient's refusal of consent
  (checked live: no offer is made), and a write without a relationship is still refused (checked live).
- Every audit row written while a request is served under a grant is flagged `break_glass` and carries the grant's id
  (`audit::GLASS`, set by the access check); the opening of a grant is itself a flagged row. The patient's access log
  already shows the flag. The audit row holds ids and times, never the free-text reason (the grant holds it).
- Review: `GET /api/break-glass/reviews` (unreviewed, oldest first), `GET /api/break-glass/reviews/count` and
  `POST /api/break-glass/reviews/{id}` (verdict `appropriate` or `inappropriate`, with a note). New policy resource
  `BreakGlass` (13 resources; the matrix test is now 1,310,400 requests): treating roles may open a grant; the auditor
  reads, searches and reviews. A grant is reviewed once, and not by the person who opened it.
- UI: an "Emergency access" section on the patient page when (and only when) the API makes the offer: a reason and a
  tick are required before anything is sent; the page reads again under the grant and keeps saying when it ends.
  `/admin/break-glass` ("Emergency access reviews") is the auditor's queue. A test found that the confirmation vanished
  with the form once the grant opened; it now stays on the page.
- Tests: 4 + 1 unit (audit flag), 5 Postgres tests (4 hours, not extended, expiry, reason, review rules, immutability),
  3 stubbed Playwright tests with axe, 2 real-stack tests (Keycloak, API): refused with an offer, opened, read, the read's
  audit rows flagged and naming the grant, reviewed once by the auditor; and a refusal of consent still holds. I removed
  the line that flags the audit rows and the real-stack test failed; restored, it passes. The whole real-stack suite
  passes (18, 2 skipped as before).
- **Not done:** the real-time notification to a privacy officer the task asks for (nothing here can send one); a count
  or alert for the review queue (the count route exists and the UI does not use it, so the queue is silent until the
  auditor opens the page); ending a grant early; a choice of duration; sensitive-record masking (SE-4), so a grant shows
  everything the role may read; detection of patterns (one clinician opening many grants); a break-glass grant for
  writes, for export or for a patient's refusal (deliberately not); the staff purpose picker still does not offer
  `emergency`; translations; a run with the exact Rust 1.97. The SE-3 acceptance (403 with an offer; after the grant,
  access and an audit row with the flag and reason; expiry) is met except that the audit row names the grant instead of
  holding the reason, and expiry is tested in the store only (a past `expires_at` is not active), not by waiting four
  hours.

## 2026-10-21 - Count of waiting refused messages on the admin pages

- `GET /api/feed/refusals/count` returns `{open}` (no ids, no text); same gate as the listing (`FeedQueue`, search) and
  audited like it. The UI shows "Refused messages (3 waiting)" in the navigation on the admin pages only (the queue, and a
  patient's access page), because every fetch writes an audit row. A role refused the count shows nothing.
- You asked for an "unread" count. The queue has no per-person read state, so this is the count of **open** (not closed)
  entries; an entry read but not closed still counts. Per-person unread state is not built.
- Tests: 1 Postgres assertion (the count moves with a refusal and a closing), 2 vitest, 1 stubbed Playwright test
  (not asked on patient pages, shown on admin pages, nothing for a refused role).
- **Not done:** an alert, a count outside the admin pages (so an administrator who never opens one never sees it), an age
  limit, and a run against the real stack for the count.

## 2026-10-21 - Refused-message queue screen

- `/admin/feed` ("Refused messages" in the navigation): the open entries with the kind, the event, the sender's control
  id, when it arrived and why; "Show the message" fetches one message (a listing carries none) and shows it with the
  segments on separate lines; closing needs an outcome (`handled` or `dismissed`) and a reason, and a blank reason sends
  nothing. A 403 says the role does not allow it; a 409 (someone else closed it) reloads the list. The text is escaped,
  never rendered as markup.
- 3 vitest, 2 stubbed Playwright tests with axe, and a real-stack test (Keycloak, API, MLLP): an ORU message is sent to
  the listener and answered `AR`, a clinician is refused the queue (403), the administrator signs in, finds the entry,
  reads it, closes it, and the audit trail has the read and the closing with the `clinical-admin` role. This is also the
  first run of the three queue routes over HTTP.
- **Not done:** a count or alert (the page is reached only from the navigation, so a queue nobody opens is still silent),
  a re-apply button, paging beyond the oldest 50, an age limit, a locale other than English (strings are shared).

## 2026-10-21 - HL7 refused-message work queue and parser fuzzing (OR-3)

- A message the feed refuses (`AE` or `AR`) is kept as received in `feed_refusals` (migration 9, append-only) with the
  event, control id, kind and reasons, and stays open until a person closes it. `feed_refusal_resolutions` holds one
  decision per refusal (`handled` or `dismissed`, with a reason), so "open" is the absence of a row and nothing is
  edited. A transient failure is not queued.
- `GET /api/feed/refusals` (open ones, oldest first, 50 at most, no message text), `GET /api/feed/refusals/{id}` (the
  message as received) and `POST /api/feed/refusals/{id}/resolve`. New policy resource `FeedQueue` (12 resources; the
  matrix test is now 1,209,600 requests): the clinical administrator reads, searches and closes; nobody else, and the
  feed's own role can not read its refusals back. Reads and closings are audited by id, never by content.
- Parse failures now keep their reasons (before, an invalid message was audited with none).
- Fuzzing: 5 unit tests mutate a real ADT message (every cut, every single-byte deletion, 3,000 random edits, arbitrary
  bytes, 200,000-character fields) and random MLLP frames; none may panic. They found nothing.
- Tests: 4 + 1 new unit, 1 new Postgres test (a refusal is queued, listed without text, read, closed once, can not be
  closed twice, tables can not be changed). Hazards H-023 and H-021 updated.
- The MLLP test data was flaky (about half of the runs): the birth dates came from a clock that ticks in whole
  microseconds, so they took three distinct values, and look-alike patients are (rightly) flagged as possible
  duplicates. Names, given names and birth dates are now well mixed; it is still occasionally flagged (1 run in 15
  here) because the duplicate rule flags a shared birth date and sex even with unrelated names. **Not fixed.**
- **Not done:** a screen for the queue and any alert or count, so a refusal nobody looks at is silent; a re-apply
  button (a person fixes the cause through `/api/patients` and closes the entry as handled); an age limit; no corpus of
  real sample messages, so the parser acceptance is **not met**; a conformance run; a run against the real Keycloak
  stack (the policy gate and the handlers are tested, the HTTP layer of the three routes is not).

## 2026-10-21 - FHIR AllergyIntolerance (read-only)

- `GET /fhir/R4/AllergyIntolerance?patient=` (the `Patient/` prefix is accepted) returns a `searchset` of the patient's
  allergies, from the same handler as `/api/patients/{id}/allergies`, so the role gate, care relationship, purpose of
  use, consent and audit row are identical. `interop::fhir_allergy` maps each record and checks it against the typed R4
  model. `/fhir/R4/metadata` lists it.
- What is not said: `verificationStatus` and `type` are not recorded so are left out; an unstated criticality is left
  out, never `low`; an allergy entered in error is not exported.
- An empty or inactive-only result carries an `OperationOutcome` entry (`search.mode` outcome) saying it does not mean
  there are none; a refusal is a 403 `OperationOutcome` with the same sentence, never an empty bundle. Only a standing
  clinician statement is exported, as SNOMED CT `716186003`.
- 10 new unit tests (5 for the mapping, 5 for the bundle, one of them a round trip of the handler's own answer). Hazard H-022.
- **Not done:** the profile validation the RC-5 acceptance asks for (no UK Core or US Core check and no HAPI run, so
  that acceptance is **not met**); read by id; create or update through FHIR; reading FHIR allergies in; `recorder`;
  no run against the real Keycloak stack (the unit tests do not exercise the HTTP layer).

## 2026-10-21 - Audit-head checkpoint

- `GET /api/audit/verify` returns `head` (id and digest of the newest row checked) and accepts
  `?checkpoint_id=&checkpoint_digest=` from an earlier answer. A checkpoint row that is missing (newer rows removed, or a
  checkpoint from another database) or has a different digest gives 409 at that id; one parameter without the other is
  400. `audit::verify_against` does the work; `verify` and `verify_in_batches` are unchanged. 1 new Postgres test
  (10 in `tests/audit_chain.rs`), including batch sizes 1, 2 and 100. Idea 1 of
  `spec/interoperability/openehr-comparison.md`.
- **Not done:** nothing publishes the head. Removal of the newest rows is detected only if an operator stored a head
  where the database owner cannot write; without one the answer still says so. Not run by hand against Keycloak; the
  query-string handler is not covered by an automated test (the logic is).

## 2026-10-21 - Documented minimum Rust is 1.97

- `rust-version` is now 1.97 in the service, migration and matcher `Cargo.toml`, and in `AGENTS.md` and `plan.md`: the
  current release (1.99) minus two. The pinned toolchain is unchanged (1.99.1).
- **Not done:** CI has no job that builds with exactly 1.97, so the minimum is declared, not tested. The matcher on
  crates.io still says 1.96 until it is republished.

## 2026-10-21 - Patient-visible access log (SE-6, partly)

- `GET /api/me/patients/{id}/access-log`: who accessed this record, newest first, 50 per page. Only the patient or a
  proxy (an active `self` or `proxy` relationship) may read it, at purpose `patient-access`; a clinician's treating
  relationship does not. Entries say `you` or `staff`, what was done, to what, the purpose and whether it was refused or
  a break-glass opening; they carry no staff identifier, `detail` or network address. A new migration adds the index
  `audit_events_patient_log_idx`. `/my-record` shows it on request. 1 Postgres test, 2 service unit tests, 5 vitest, 1
  stubbed browser test and 1 real-Keycloak browser test (the clinician's view appears to the patient); also by hand
  against Keycloak: 401 with no token, 403 and an audit row for a clinician.
- **Not done:** role and organisation are not shown, because audit rows record only the token `sub` (threading roles
  into the ~17 audit call sites, and a deployment policy for staff names, are separate work). No filter or date range,
  no export, no alert. The handler itself has no automated test without a Keycloak stack; the "patient but not linked"
  branch is covered by the policy engine refusing earlier. The new Keycloak test needs a fresh database or new data: the
  older ones re-run only on one.

## 2026-10-21 - Roles and organisations in audit rows and the patient's log

- Every audit row written while handling a request records the caller's roles and organisations in `detail.actor`
  (`audit::ACTOR`, a task-local set by the route-gate middleware, so the ~17 call sites did not change). `Subject` gained
  `organisations`, from the token attribute `organisation` (short lowercase tokens only). The patient's access log shows
  "A member of staff (clinician, riverside) viewed your identity record." Development: realm role `org-riverside`, mapped
  in `role-map.json`; `keycloak-dev-user.sh` takes comma-separated roles. Tests: 2 audit and 1 policy unit tests, 1
  `authz_audit` and the extended `patient_log` Postgres tests, 1 vitest, stubbed and real-Keycloak browser tests updated;
  by hand the row for a clinician with `org-riverside` read `{"roles":["clinician"],"organisations":["riverside"]}`.
- **Not done:** rows written before this change have no actor and read "staff". Rows written outside a request (tests,
  future jobs) have none. Roles are recorded as the token said at that moment, not checked against a directory. Staff
  names are still not recorded or shown. The hash chain covers `actor` (it is in `detail`); no chain format change.

## 2026-10-21 - FHIR Patient endpoint (ID-3, in part)

- `GET/POST /fhir/R4/Patient`, `GET/PUT /fhir/R4/Patient/{id}` and `GET /fhir/R4/metadata`, over the existing
  `interop::fhir_patient` mapping. The controller only translates: it calls the `/api/patients` handlers, so access
  control, consent and the audit row are the same code. `Stored` is now `Deserialize`. 7 unit tests; run by hand and in the
  real-Keycloak suite (create 201 with `ETag` and `Location`, read, search Bundle, duplicate 409 without candidates,
  `maritalStatus` 422, no `If-Match` 428, stale 412, exact-date-only search 400). The route table and census know the five
  routes.
- **Not done:** FHIR `_history`, `vread`, delete, conditional create/update, `_include`, paging, `_count`, `_sort` and
  date prefixes; Patient elements beyond identity (those are refused, not stored); no US Core or UK Core profile claim and
  no conformance run (e.g. Inspector or a validator); `metadata` needs a token (it is not public). No handler-level
  automated test in `cargo test` (no `AppContext` harness): the translation is unit tested and the whole path is run
  against Keycloak by hand and in the opt-in browser suite.

## 2026-10-21 - An unheld key is a key problem, not tampering

- `Chain::check` tells "the digest is wrong" (`Mismatch`) from "this process cannot check it" (`KeyNotHeld`,
  `SchemeUnknown`). `GET /api/audit/verify` answers **424** with `tampering_found:false`, the unchecked count, the first
  unchecked row and the causes, instead of 409 naming a row as broken. The scan continues past such rows, so a real break
  further down still gives 409. Idea 2 of `spec/interoperability/openehr-comparison.md`. Tests: 2 unit, 1 Postgres
  (rotation without keeping the old key, the endpoint answer, the held-but-wrong key, a break behind unchecked rows).
- **Not done:** an unkeyed `sha256:` row in a keyed chain is still 409, so enabling a key on an existing chain makes the old
  rows read as broken; the openEHR "unkeyed only" state is not modelled. Not run by hand against Keycloak or a rotated
  key in the real service. A monitor that treats every non-200 the same will not notice the difference.

## 2026-10-21 - Clinical hazard log (AS-1, partly)

- `governance/clinical-safety/hazard-log.csv`: 15 hazards for what the code does today (wrong-patient link, missed
  duplicate, NHS number, lost update, unentitled access, ignored consent, unmasked sensitive data, no emergency access,
  audit tamper and gaps, wrong relationship, bad ingestion, misleading access log, unreviewed translations, use before
  ready), each with cause, effect, 1-5 ratings, controls, evidence paths and residual risk. Plus an unsigned risk
  management plan and safety case report template. `scripts/check-hazard-log.py` (shape, score arithmetic, evidence
  paths exist, no `accepted`/`closed` without a named reviewer; 9 unit tests) runs in the docs job;
  `scripts/check-clinical-impact.sh` and a `hazard-log` job fail a `clinical-impact` PR that leaves the log alone.
- **Not done:** the ratings are the author's proposals and were reviewed by nobody; there is no named Clinical Safety
  Officer (OQ-2) and no safety case report; this is not DCB0129 compliance. The workflow job has not run on GitHub (no
  remote) and the label has to be applied by a person. Seven hazards are `open` with no adequate control (e.g. H-008,
  no emergency access).

## 2026-10-21 - Audit read API (AU-2)

- `GET /api/audit` and `GET /fhir/R4/AuditEvent`: search the audit trail, auditor role only, newest first, 50 per page.
  `audit::search` builds one parameterised query (no string-built SQL; dates are checked by the database before use, so a
  bad date is 400, not 500). A search writes its own audit row with the names of the criteria, not their values. New
  `interop::fhir_audit` (typed R4 `AuditEvent`, validated). The route table and census know the two routes.
  Tests: 1 Postgres (every filter, paging, blank criteria, injection-shaped input), 3 + 1 unit, and by hand and in the
  real-Keycloak suite (no token 401, clinician 403 with the refusal visible to the auditor, auditor 200).
- **Not done:** `le`/`gt` date prefixes (the store compares whole instants), full-text search of `detail`, export, an
  AuditEvent `read` by id, the digests in the output, any IHE BALP conformance claim (purpose-of-use codes are local),
  and a patient-facing use of this search (the patient log, SE-6, has its own narrower endpoint). The gate's 403 is a plain
  `forbidden`, not an `OperationOutcome`, on the FHIR routes.

## 2026-10-21 - Patient banner (ID-7, in part)

- `PatientBanner.svelte` on the patient page and on each `/my-record` record: name, birth date, age, sex, NHS number,
  one other identifier, photo status and allergy status, from the stored record only. `ageOn` and `nhsDisplay` in
  `patient.ts`. Hazard H-016 added to the log in the same commit. 5 vitest, 2 stubbed Playwright tests (with axe), and the
  my-record test now expects a banner per record.
- **Not done:** no photo; the allergy line is a fixed "not recorded" placeholder until allergies exist (RC-5); the banner
  does not refresh when another user edits (a stale save is still refused); not run against the real Keycloak stack;
  ages use UTC dates; the ten locales share the English strings (machine-written, unreviewed); the compare page and
  `/patients/new` have no banner because no stored patient is shown there. The task stays open.

## 2026-10-21 - A write and its audit row are one transaction

- Patient, consent and relationship create and update now write the new version and its audit row in one transaction
  (idea 3 of `spec/interoperability/openehr-comparison.md`; hazard H-010). `audit::append` is generic over a connection or a
  transaction and joins the latter as a savepoint; the three stores' write functions are generic too, and each `update`
  runs its insert in a savepoint so a lost version race does not abort the caller's transaction. 1 new Postgres test (a
  trigger makes the audit insert fail: the patient is not created; a stale update inside a transaction leaves it usable;
  a rolled-back write leaves no audit row). Run through all six handlers by the real-Keycloak suite (11 passed).
- **Not done:** reads, refusals, `compare`, verify and search rows are still written in their own transaction (a read
  that cannot be audited still answers 500 by the existing rule, but it is not atomic with anything). One audit row for a
  set of writes (the openEHR "contribution") is not modelled. The advisory lock is now held a little longer on writes. No
  handler-level automated test in `cargo test`. I did not read the status of the last verify call on the final run.
  The test-side `exact: true` change fixes a heading clash my banner caused in the Keycloak suite.

## 2026-10-21 - Audit search: le/gt, and export

- `after` (`gt`) and `through` (`le`) bounds on `audit::search`, with day-precision meaning the whole day; the FHIR
  `AuditEvent` search now takes all four prefixes. `GET /api/audit/export` returns CSV or NDJSON with the digests (up to
  10,000 rows, `x-next-before` for more), is auditor-only (`Export`) and is itself audited. The CSV defuses formula
  injection. Tests: 1 Postgres (instants exact, days whole, digests chained), 3 unit.
- **Not done:** no text search of `detail`; no streaming (the file is built in memory, hence the cap); no signed or
  encrypted export; not run against Keycloak by hand; `eq` and bare FHIR dates are still refused.

## 2026-10-21 - Banner refresh

- The patient page re-reads the record when the tab is shown and once a minute while shown. A clean form catches up; a
  dirty form keeps the typing while the banner shows the newer record and an alert explains. 1 stubbed Playwright test
  (with axe) covers clean, dirty and reload.
- **Not done:** a minute is a poll, not a push (no SSE or WebSocket); each look is one more audited read; an edit made in
  the last minute is not seen until the next look; not run against the real Keycloak stack; `/my-record` does not refresh.

## 2026-10-21 - Patient photo (ID-7, in part)

- `patient_photos` (append-only) and the routes above; the banner shows the photo, the patient page uploads and removes one.
  The proxy now forwards request bodies as bytes (it used `text()`, which corrupts a photo) and passes `cache-control`,
  `nosniff`, `etag` and `content-disposition` back; the UI CSP allows `blob:` images. Hazard H-017 added. 2 unit and 4
  Postgres tests, 3 stubbed Playwright tests with axe, 1 real-Keycloak test (an SVG sent as `image/png` is 422; the
  patient's banner decodes the image; the raw route is `nosniff` and `no-store`).
- Found by the live run and fixed in the same change: `GET /api/audit/export?outcome=0` failed (a `#[serde(flatten)]`
  query cannot read numbers), so the export now parses its pairs with `AuditQuery::from_pairs` (1 unit test); the by-hand
  run then showed CSV, NDJSON, 400s, 403 and the FHIR `gt`/`le` filters working.
- **Not done:** nothing checks the photo shows this patient, or a face; no content scan; no FHIR `Patient.photo` mapping
  (a FHIR body with `photo` is still refused); no resizing or thumbnail (a 512 KiB image is fetched for a 6 rem box);
  hostile metadata inside a PNG or JPEG is stored as it came; the banner on the staff pages fetches under the picker's
  purpose.

## 2026-10-21 - Allergies (RC-5, in part): the banner now has real allergy data

- The first clinical content: `allergy_versions` and `allergy_assertion_versions` (append-only), the four routes above,
  an allergy section on the patient page, and the banner line driven by one shared answer. The patient's consent now
  applies to every clinical read (it covered only identity before). Hazards H-018 and H-019 added; H-008 notes that a
  clinician without a care relationship has no override. 6 + 4 + 3 tests (unit, Postgres, vitest), 4 stubbed Playwright
  tests with axe, and a real-Keycloak test: a care-team clinician records a high-risk allergy under `treatment`; "none known"
  is refused (422); an administrator is refused by the gate and a clinician with no relationship is refused by the record
  decision; the patient's banner shows "Penicillin (high risk)"; an auditor finds the rows and none holds the substance.
- **Not done:** problems, medications, immunisations; FHIR `AllergyIntolerance` and the profile validation the RC-5
  acceptance asks for (not met); a medicines or drug-class check at prescribing (OR-6); break-the-glass; verification status;
  duplicate detection between two allergies recorded in different words; a history screen; no pharmacist or patient alert
  when an allergy is ended; the substance list is free text. The statement voids on any later allergy version, including
  one entered in error (conservative: it falls back to what the records show).

## 2026-10-21 - Decision: reads and refusals stay in their own audit transaction

- Asked to put reads, refusals and the verify and search rows in a transaction. There is nothing to make atomic: those
  paths write nothing but the audit row, which is already written before the data leaves and fails the request (500, no
  disclosure) when it cannot be written. The earlier changelog line calling this a remaining gap overstated it; the
  service spec and `tasks.md` now say it is by design. The writes that had two transactions (patient, consent,
  relationship, photo, allergy) are single ones.
- **Not done:** a consistent snapshot between the data a read returns and the audit row (nothing requires one).

## 2026-10-21 - Access and consent page

- `/patients/[id]/access`: the admin UI for care relationships and consent over the existing APIs (no service change).
  `src/lib/access.ts` builds the requests and words the provisions. 3 vitest, 2 stubbed Playwright tests with axe, and
  1 real-Keycloak test through the page: an administrator grants a care-team link (the clinician's allergy read goes from
  403 to 200), records a deny for clinicians (403), withdraws it (200) and ends the link (403).
- **Not done:** one provision per consent in the form (several need several consents); no in-place edit of a consent;
  people are token ids, not names, and there is no picker; not linked from `/my-record`, so a patient cannot yet use it for
  their own consent; the opt-out remains local only; no FHIR `Consent`.

## 2026-10-21 - Merge and unmerge (ID-5, in part)

- A merge is a link (`patient_link_versions`, append-only), so nothing moves and unmerging restores both records exactly.
  `links.rs` holds the rules, the routes are `POST …/merge`, `POST …/unmerge`, `GET …/links`, and the policy table gains
  `PatientLink` (11 resources; the matrix test is now 1,108,800 requests). Effects are all reads of the link: the old
  record is read-only and out of search and the duplicate check, the survivor shows the merged-in allergies, consent
  refusals on either record apply to both, the FHIR Patient carries `active:false` and `link`. A page section merges and
  undoes. Hazard H-020 added. 3 + 7 + 2 tests plus a real-Keycloak test; mutation checks: dropping the union, and
  leaving the link active on unmerge, each made tests fail.
- **Not done:** the review queue for uncertain pairs and any screen that finds candidates; a second person confirming a
  merge; photos, relationships and the no-known-allergies statement do not follow a merge; any future clinical content
  must be taught to follow it (H-020 says so); the matcher threshold is unvalidated; FHIR `$merge` operations;
  merging more than two at once is by repeating.

## 2026-10-21 - HL7 v2 ADT feed over MLLP (OR-3, in part)

- `mllp.rs` (framing, listener, settings) and `adt_feed.rs` (matching, applying, auditing, answering), started from
  `after_routes` when `EPR_MLLP_BIND` is set. The integrator role may now create and update identity (policy; it still
  cannot read or export). `operations/hl7/README.md` says how to turn it on and what it does not do. Hazard H-021. Tokio
  gains `io-util`. Tests: 8 + 7 unit, 4 over a real TCP socket and Postgres, all with a pool of one connection.
- Found by the by-hand run and fixed before commit: the first version held a transaction while asking the pool for a
  second connection (the duplicate check), which timed out on the development pool of one; the tests had passed because
  their pool was larger. Reads now happen before the transaction, and the tests use a pool of one so it cannot recur. Two
  of my own test expectations were wrong (a timestamp, and test patients that were rightly taken for one person).
- **Not done:** the work queue (a refused or unmatched message is answered `AE` and the sender must hold it, so the
  acceptance "queued, not dropped" is not met); `A03`, `A40`, `PV1`, ORU, SIU; TLS and authentication (stated, not built);
  character sets other than UTF-8; fuzzing and a sample corpus; a run with a real sender; a record merged away reappears as
  a possible duplicate rather than being followed to its survivor.

## [0.1.0] - 2026-10-20

Milestone M0, scaffold. **Not for clinical use.**

### Added

- **Matcher** (`electronic-patient-record-matcher-with-rust`): `Patient` identity type; deterministic, symmetric,
  explainable matching with per-field breakdown; NHS number modulus-11 validation; refusals for twins, missing
  given names, differing verified NHS numbers; 13 tests.
- **Service** (`electronic-patient-record-service-with-rust`): loco.rs app; PASETO bearer guard, on by default,
  refusing to start without keys or with login off in production; `POST /api/patients/compare`; append-only
  `audit_events` table with triggers; 6 unit tests and 2 database-gated tests.
- **UI** (`electronic-patient-record-ui-with-svelte`): SvelteKit shell, locale-prefixed URLs, strict CSP and
  security headers, proxy that only forwards `/api`, a compare page; 26 unit tests, 7 browser tests with axe.
- CI, release workflow, root `compose.yaml`, Containerfiles, scripts.

### Added after the first cut: audit writer (AU-1, partly)

- `audit::append` / `audit::verify`: hash-chained, keyed with `EPR_INTEGRITY_MAC_KEY` when set. `compare` now writes an
  audit row before answering and returns 500, with no result, if it cannot. 6 unit and 7 Postgres tests.
- Found and fixed while testing by hand: my first key variable name was wrong (`EPR_INTEGRITY_KEY`; the crate derives
  `EPR_INTEGRITY_MAC_KEY`), so the chain silently stayed unkeyed. A unit test now pins the names.

### Added after the first cut: Keycloak (SE-8)

- The guard accepts Keycloak access tokens; `v4.` tokens still go to PASETO, everything else to Keycloak. The audit agent is the
  token `sub`. Invalid Keycloak configuration refuses to start. 8 DB-free tests plus a by-hand run against Keycloak 26.3.0.
- `keycloak` compose profile (Keycloak on the shared PostgreSQL, realm `epr` imported), `scripts/keycloak-dev-user.sh`,
  `scripts/keycloak-dev-token.sh`, and standalone `scripts/podman-postgres.sh` / `scripts/podman-redis.sh` (both tried).
- **Not done:** roles are not consulted; the UI cannot sign in (SE-9); the API cannot reach a plain-HTTP Keycloak from inside compose
  (it needs HTTPS); the CI container job does not exercise Keycloak; tested only on one node, `start-dev`.

### Added after the first cut: UI sign-in (SE-9)

- The UI signs readers in through Keycloak (`epr-web`, authorization code flow with PKCE) in its backend-for-frontend. The browser holds one
  opaque `httpOnly`, `SameSite=Lax` session cookie (`__Host-` prefixed over HTTPS); the tokens stay in the server's session store, which
  sends the access token to the API, refreshes it before it expires, ends idle (15 min) and over-long (8 h) sessions, and signs out
  locally first and then at Keycloak. Cross-site POSTs to the proxy and to sign-out are refused. Off unless `EPR_OIDC_ISSUER` is set;
  a bad value stops the UI at startup. 24 new unit tests (50 in all).
- Verified by hand on 2026-10-21 against Keycloak 26.3.0 and the real API (`playwright.keycloak.config.ts`, 5 passed, 1 skipped): sign-in, a
  protected call that returned 200 with the audit row's agent set to the Keycloak `sub`, sign-out, a tampered cookie, a replayed
  pre-sign-out cookie, a forged callback, and cross-site POSTs.
- **Not done:** the session store is in process memory, so a restart signs everyone out and two UI replicas cannot share sessions; token
  refresh and idle expiry are tested with a stub provider only (Keycloak's access token lives 300 s and the run did not wait); the
  ID token is decoded, not signature-checked (allowed for a token straight from the token endpoint, OIDC Core 3.1.3.7; the API checks
  the access token on every call); no front-channel or back-channel logout from Keycloak into the UI; the Keycloak end-to-end run is
  not in CI; no Entra ID; the sign-in and sign-out text is English only.

### Added after the first cut: policy engine and route gate (SE-1, in part)

- `policy.rs`: ten roles, nine resource kinds, six actions (no delete), care relationship, security labels, purpose of use, consent and
  break-the-glass, as a pure engine. `authz.rs` gates every route on the token's `role` attribute; a route with no entry in
  `routes.rs` is 403 for everyone; each refusal is audited (route template, method and roles, never the body). `tests/route_census.rs`
  fails when the served routes and the table disagree (checked by emptying the table). Keycloak realm roles map to policy roles through
  `operations/keycloak/role-map.json`; `scripts/keycloak-dev-user.sh` takes a role.
- Tested: 15 engine tests including a 907,200-request matrix checked against invariants; 9 DB-free guard tests; 3 Postgres tests of the
  gate and its audit rows. By hand against Keycloak 26.3.0 and the real API: clinician 200, auditor 403, no role 403, unlisted route 403, no
  token 401; the UI shows "Your role does not allow this." for the no-role user (6 end-to-end tests passed).
- **Behaviour change:** a valid Keycloak or PASETO token with no recognised role used to be enough for `compare`; it now gets 403.
- **Not done:** the record-level `evaluate` is called by no handler; no consent store, break-the-glass grant store or masking (SE-2 to SE-4);
  no care-relationship lookup. **The permission table, label lists and purpose rules are an engineer's first draft that no clinical safety
  officer or privacy officer has reviewed.** With login off there is no authorization (development only). 401s are not audited.

### Added after the first cut: patient create, read, update, search (ID-3, in part)

- Migration `patient_versions` (append-only, triggers) and view `patient_current`; `POST/GET /api/patients`, `GET/PUT /api/patients/{id}`. Gated by role and by
  `policy::evaluate` (the engine's first caller), validated (NHS number check digit, real birth dates), every access audited without demographics.
- Tested: 7 Postgres tests (versions, stale and racing writers, immutability, search, wildcards, the 50-row cap); 4 validation tests. By hand against Keycloak 26.3.0
  and the real API: 201, 422, 200, 409, 403 for patient role, auditor and a research purpose.
- **Not done:** FHIR `Patient`; phonetic and typeahead search; no load test (the 200 ms p95 target is unmeasured, and search scans); the write and its audit row are
  separate transactions; 422 and 400 are not audited; patient and proxy roles cannot read their own record (no relationship lookup); no UI page; no duplicate check at
  registration (ID-4), so the same person can be registered twice.

### Added after the first cut: duplicate prevention at registration (ID-4, API only)

- `POST /api/patients` scores the new record against candidates (same identifier, birth date or family name) with the matcher; any that is not a `non_match` gives
  **409** with each candidate's record, score, breakdown and conflicts. `override_reason` registers anyway; the reason is kept in the version history and the audit row
  lists the candidate ids. A failed candidate lookup fails the registration rather than skipping the check.
- Tested: 3 unit tests and 1 Postgres test; by hand: first 201, repeat 409, blank override 409, real override 201, audit rows as expected.
- **Not done:** the UI comparison page; misspelt-name-and-different-birth-date duplicates are not found; concurrent registrations can both pass; no separate override permission.

### Added after the first cut: patient pages in the UI (ID-7, in part; finishes the ID-4 UI line)

- `/patients` searches (family-name prefix, birth date, NHS number; an empty search is not sent), `/patients/new` registers, `/patients/[id]` shows and edits a record with the
  version it was read at (a stale save says the record changed and offers a reload), `/patients/compare` keeps the two-patient comparison. On a 409 duplicate the new page shows each
  candidate beside what was entered, with the matcher's score, reasons and a link to the existing record, and registers anyway only with a typed reason. The edit form changes only
  the fields it shows: other names, identifiers, addresses and contact details are carried into the new version (unit-tested). The proxy now forwards `PUT`.
- Tested: 13 new vitest (63 total); 10 new stubbed Playwright tests with axe on each page state (17 total); and against a real Keycloak and API, one new end-to-end test
  (bad NHS number, register, duplicate, override, two-window stale edit, search) with the audit rows read back. 6 passed, 2 skipped there.
- Found while running it: the Podman VM's clock was 18 minutes behind the host, so Keycloak's tokens were already expired by the API's clock, and `axum-keycloak-auth` reports an
  expired token as `InvalidSignature` (it keeps only the last key's error). Fixed by setting the VM clock; noted in `AGENTS.md`.
- **Not done:** the persistent patient banner (photo, two identifiers, age, allergies status) the ID-7 acceptance asks for; the page does not send `X-Purpose-Of-Use`, so every call is
  `operations`; no pagination past 50 results; no phonetic or typeahead search; the form edits the first name, NHS number and first postcode only (a second name or address cannot be
  edited, only kept); English strings only; the Keycloak end-to-end run is not in CI.

### Added after the first cut: patient consent (SE-2, identity only)

- `consent.rs` (rules), `consents_store.rs` (append-only `consent_versions`, optimistic updates), `controllers/consents.rs`, migration `…000003`. A consent holds permit and deny provisions by
  role, purpose, resource kind, security label and period. `consent::decide` reduces a patient's current active consents to the policy engine's consent value: deny wins; a permit must
  name every label on the record; an unreadable provision denies. A refusal now stops reads of that patient's identity (403, audited with the consent ids), drops them from search and
  redacts them in the duplicate check (`{"restricted": true}`; the duplicate still blocks registration). The UI shows a restricted candidate as a message with no record.
- Tested: 19 unit tests, each core rule broken in turn to see a test fail; 5 Postgres tests; by hand against Keycloak and the API as clinician, receptionist and clinical admin (read 403 then
  200 after withdrawal, search 0 vs 1, `restricted` candidate, route-gate refusals, 422 for unknown values, 404, stale 409, audit rows read back); 1 new UI test.
- Found while running it: the Podman VM clock was behind again (see `AGENTS.md`).
- **Not done:** acceptance not met (no export to honour consent, no clinical record to hide); the opt-out changes nothing today because no role may use the `research` purpose; patient and proxy
  cannot manage consent; no consent UI or FHIR `Consent`; no handler-level automated test of the denial paths; no emergency override (a privacy-officer decision); consent lookups cost one query
  per read or search, unmeasured.

### Added after the first cut: audit-chain verify endpoint (AU-1 acceptance)

- `GET /api/audit/verify`, auditor only: recomputes the chain and answers `{intact, rows, keyed, assurance}` (200) or the first broken row (**409**). Each check writes its own audit row
  (outcome and counts, never row contents) and the endpoint fixes the purpose of use at `audit`. `audit::verify` now reads in batches of 1000 by keyset, so memory no longer grows with the table.
- Tested: 2 new Postgres tests (batch sizes 1 to 1000 agree; a break either side of a batch boundary is found; the endpoint logic reports intact then 409 and writes two rows);
  by hand against Keycloak and the API: no token 401, clinician 403 (audited), auditor 200, then 409 naming row 1 after I turned the triggers off and edited it.
- **Not done:** removal of the newest rows is not detected (no external anchor); an unkeyed chain (no `EPR_INTEGRITY_MAC_KEY`) can be recomputed by anyone who can write to the database
  (the response says so); no handler-level automated test (the endpoint logic is tested, the HTTP wiring only by hand); no search API over the log (AU-2); no UI; verifying a very long chain is a
  full scan with no rate limit.

### Added after the first cut: care-relationship lookup (SE-1 gap)

- `relationships.rs`, `controllers/relationships.rs`, migration `…000004`, policy resource `CareRelationship` (clinical-admin only; the matrix is now 10 × 10 × 6 × 8 × 5 × 3 × 7 × 2 = 1,008,000 requests).
  `authorize` looks up the caller's relationship to the patient on every patient, consent and relationship route, so the `patient` and `proxy` roles are no longer refused by construction.
- Tested: 3 unit and 3 Postgres tests; by hand against Keycloak and the API with a patient-role user: refused before a link (403), self-grant refused by the route gate, linked by an administrator
  (201), reads own record with `patient-access` (200) but not with the default purpose, not another patient's, a second patient user not A's, manages own consent and is not stopped by their own
  deny, refused again once the relationship is ended; a change of subject is 400.
- **Not done:** relationships are only what an administrator types: nothing derives care team or encounter from clinical activity, there is no expiry job (periods are checked at read time), no
  UI, and the service cannot check that the `sub` is the patient. The UI proxy does not send `X-Purpose-Of-Use`, so a patient cannot read through the UI yet. One extra query per request, unmeasured.
  Care-team access still has nothing to gate (no clinical content); break-the-glass (SE-3) is still not built.

### Added after the first cut: purpose of use in the UI and a patient "my record" page

- `GET /api/me/patients` (service) lists the records linked to the caller by a `self` or `proxy` relationship, each through the full decision, purpose fixed at `patient-access`. The UI proxy forwards
  `X-Purpose-Of-Use` when it is a known code and drops anything else; `/my-record` shows the records read-only and says plainly when nothing is linked.
- Tested: 1 Postgres test (`own_patients`), 3 vitest, 2 stubbed Playwright tests with axe, and one real Keycloak end-to-end test: an administrator links a patient-role user, who signs in through the UI, sees
  only that record, is refused another (403), and sees nothing once the link is ended. 7 passed, 2 skipped there.
- **Not done:** staff pages still send no purpose (so `operations`); no purpose picker; the UI proxy\'s forwarding is unit-tested and exercised by the end-to-end run but has no test with a stub upstream;
  patients cannot edit, print or download anything, or see who accessed their record (SE-6); no relationship or consent UI for administrators; the linking is still an API call.

### Added after the first cut: staff declare treatment from the UI

- A header picker (Operations or Treatment) sets the `X-Purpose-Of-Use` sent on staff calls; it is kept per tab and `/my-record` is unaffected. Real run: a clinician chose Treatment, searched, and the audit
  trail holds a row with purpose `treatment`.
- Tested: 3 vitest (70 in all), 1 stubbed Playwright test (21 in all) and 1 real Keycloak test (7 passed, 3 skipped there).
- **Not done:** the choice is a declaration, not a check: nothing confirms the clinician is treating this patient (that needs the relationship gate on clinical content, which has no content yet); no
  `emergency` option; the default is still Operations; a role that may not use a purpose just gets 403 with no hint which purpose would work.

### Added after the first cut: the `hl7`, `er7`, `fhir` and `snomed` crates, and a comparison with `openehr`

- Service dependencies: `hl7` 0.2.3, `er7` 0.2.2, `fhir` 4.2.4 (R4 only), `snomed` 0.56.1 (all this project\'s author\'s crates; MSRV 1.96, no build scripts; +19 crates in the lock file; `cargo deny` clean).
  Each is used, in `src/interop` (library only, no endpoint): FHIR R4 `Patient` both ways with dropped elements named, HL7 v2 ADT to an identity record with `AA`/`AE`/`AR` acknowledgements, and a
  SNOMED CT identifier check. 19 unit tests.
- `spec/interoperability/openehr-comparison.md`: the `openehr` 0.10 crate against this project, from reading its source and running two of its examples. Verdict: not a dependency now (a second clinical
  model next to FHIR, MSRV 1.97 against our documented 1.96, nothing yet to hold); six ideas worth taking, chiefly an externally published audit-head checkpoint, reporting an unheld key as a key problem
  rather than tampering, and a contribution that makes a write and its audit row one transaction. It also records that its example 04 labels an oldest-first list "most recent first".
- **Not done:** no `/fhir/R4/Patient` endpoint, no MLLP listener, no HL7 message stored or matched (ID-3 and OR-3 stay open); the FHIR mapping covers the thin identity record only; HL7 mapping reads
  `PID` and `MSH` only (no `PV1`, no merge `MRG`, so A40 is unsupported); `fhir` R4 lengthens a clean build (the first build after adding it took about two minutes here, not separately measured); none of the six ideas from the
  comparison is built; the comparison has not tested openEHR conformance or mapped our record into openEHR.

### Changed after the first cut

- The matcher is published as `electronic-patient-record-matcher` 0.1.0 on crates.io; the service depends on it from
  the registry (no path dependency), so its container build no longer copies the matcher.
- Docker replaced by Podman everywhere: `Containerfile` and `.containerignore` instead of `Dockerfile` and
  `.dockerignore`; `podman build` / `podman compose` in CI and release. Image names still use the `docker.io/…`
  registry, which is only a registry name.

### Not done or not verified

- No clinical record, no FHIR endpoints, no patient storage, no patient CRUD, no merge.
- Audit: no verify endpoint, denied requests are not audited, no other route is audited (none reads stored data), and
  an existing unkeyed chain cannot adopt a key later. Without `EPR_INTEGRITY_MAC_KEY` the chain is unkeyed SHA-256
  (warned at startup), so someone who can write SQL can recompute it.
- Authorization is a role gate per route only; consent, break-the-glass and masking are not built (SE-2 to SE-4). The UI signs in through
  Keycloak, but with sign-in off (no `EPR_OIDC_ISSUER`) its proxy sends no token and only works against an API with login off.
- The matcher has 13 hand-written tests, not the 200-pair labelled corpus the acceptance criterion in ID-1 asks
  for, and no fuzz target. Its thresholds are unvalidated guesses.
- Only English strings exist; the other nine locales fall back to English. No Lily widgets are used yet.
- Run by hand on 2026-10-20: both images built; the compose stack reached healthy; with no keys the API container
  exits 1 ("refusing to start"); with a key set an unauthenticated call gets 401; the UI image serves, redirects to
  `/en-001`, and its proxy 404s outside `/api`.
- **Not run:** the GitHub Actions workflows (`ci.yml`, `release.yml`) have never executed (the Podman steps in them, including `podman-compose` and the multi-arch `podman manifest` push, are untested; `podman compose` locally delegates to a Homebrew `docker-compose` client talking to Podman), so any error in them is
  unknown; there is no zap job and no chaos workflow; the release image and SBOM steps are untested.
- No hazard log, no clinical safety officer, no governance pack (AS-1 to AS-3).

## 2026-10-26 - Matcher 0.4.0 released; the service uses it (CQ-37, H-001)

- Matcher 0.4.0 is published to crates.io (CQ-37: a shared NHS number with a given name, family name or birth date absent on either record goes to Review). The service pins 0.4.0. Matcher gates and `cargo publish --dry-run` passed; service fmt, clippy and unit tests passed; Postgres suite 91 tests, 0 failed (validator tests skipped).
- Not done: Clinical Safety Officer review of CQ-37; H-001 stays `open` and its retest of the CQ-19 rules and thresholds against 0.4.0 has not happened; no git tag (a `v*` tag releases the service image); a published version cannot be unpublished, only yanked. The UI was not rerun for this bump (it does not depend on the matcher crate).

## 2026-10-26 - Service and UI: merge and unmerge refusals by code, in the page's language (CQ-38)

- The service adds `refusal` (and `refusal_into`, the other record's id) to the 422 for the six rule refusals: reason required, record into itself, drill with real, not merged, source already merged, surviving record itself merged. The merge section says each in ar, cy, de, es, fr, hi and zh (machine-written, unreviewed), naming the other record, and shows the API's English text for a code it does not know or a missing id. Tests: a service unit test that every message has a code, the HTTP test asserts two codes, 2 unit tests and a browser test (de, then English fallback).
- Not done: no speaker or clinician has reviewed the words; the codes come from matching the service's own message text (constants in `links.rs`), not from a typed error, so a reworded message without a code falls back to English; entry sentences and registration messages are still English. The hazard log is unchanged: the wording does not change what a merge does.

## 2026-10-26 - Service and UI: a refused patient's reasons by code, in the page's language (CQ-38)

- The service adds `error_codes` (same order as `errors`) to the 422 for a refused patient on create and update: `family_name_required`, `birth_date_invalid`, `nhs_number_placeholder`, `nhs_number_check_digit`, `identifier_incomplete` (`patients_store::validate_coded`; `validate` is unchanged). The registration and edit pages say each in ar, cy, de, es, fr, hi and zh (machine-written, unreviewed) and show the API's English for a code they do not know, or all the English when the lists do not line up. Tests: a service unit test, an HTTP test (Postgres) of the codes, 2 UI unit tests (every locale; unknown and mismatched), and a browser test (de, with an unknown code).
- Not done: no speaker or clinician has reviewed the words; the birth-date sentence no longer quotes the value that was refused; the HL7 feed's refusals, the photo's 422 text and the FHIR OperationOutcome still carry English only; the other `validate` functions (allergy, consent, relationship, proxy) have no codes; the form labels on these pages are still English. The hazard log is unchanged: the wording does not change what is accepted.

## 2026-10-26 - UI: idle warning seen on the real stack (CQ-8); English readers keep the API's words (CQ-38 fix)

- CQ-8: a Keycloak browser test signs in with a 140 s idle limit, waits for the warning, presses Stay signed in, sees it clear and the API answer 200. `playwright.keycloak.config.ts` takes `EPR_E2E_IDLE_SECONDS` (default 900); the test skips without it. The run was against Keycloak 26.3.0 in Podman, the API on the host and the preview UI: 8 passed, 13 skipped (they need other users or settings I did not set up).
- **Fix to my earlier commit (e5ad2ca):** the real-stack registration test failed because English readers were shown my new English wording, not the API's. A locale with no translation now shows the API's `errors` text, as before; the stubbed browser tests missed it because they sent no codes. Unit test updated.
- Not done: the test does not break the keep-alive on purpose to see it fail; it proves the warning and the press, not that the server expires a session whose only traffic is the poll (the unit tests and "an idle session ends" cover the pieces); the API ran against the test database, not compose Postgres (not published to the host); no hazard-log change.

## 2026-10-26 - Service and UI: break-glass and allergy-statement times on the site's clock (RC-9, H-016, H-024)

- Decision (maintainer): the site zone, consistent with the banner's age. `GET /api/break-glass/reviews` now sends `site_time_zone`; the UI's `siteWhenText` shows `2026-10-21 09:30 EDT (13:30 UTC)` for a grant's start and expiry (patient page, `/admin/break-glass`) and for "No known allergies (stated …)" in the banner. No zone, UTC or an unknown name gives the old UTC text. Tests: 3 unit (three process zones, a New York clock change, fallbacks), 1 browser test, and an assertion in the Postgres roles test.
- Correction to my proposal: I first said no other page showed a stored instant; the allergy statement and the HL7 feed queue did. The statement is changed; the feed queue's received time is **not** (the reviews list for it would need the zone too).
- Not done: no recorded zone is stored (still undecided, see RC-9); one zone per service; an older API leaves these times in UTC; no clinician has reviewed the display. H-016 and H-024 controls and residuals are updated and their status is unchanged.

## 2026-10-26 - Service and UI: the refused-message queue's received time on the site's clock (RC-9)

- `GET /api/feed/refusals` sends `site_time_zone`; the queue's headline uses `siteWhenText` (`received 2026-10-21 05:00 EDT (09:00 UTC)`), UTC text when there is no zone. Tests: 1 unit, 1 browser, an assertion in the Postgres refusal-roles test; the rest of the UI and Playwright suites pass (172, 71).
- Not done: this closes the "times still UTC" part of RC-9 for every page I found by searching for `whenText` and `UTC` in the UI, not by auditing every screen; no recorded zone is stored; one zone per service. The hazard log changes only to drop the caveat I added to H-016 earlier today ("the HL7 feed queue's received time is still UTC"), which is no longer true; no control or status changed.

## 2026-10-26 - Tasks: RC-9 closed, the recorded zone carried to RC-6

- RC-9 is ticked: its acceptance (a time-zone test under three zones and around a DST change) is met by `tests/unit/time-zones.test.ts`, and the site-clock display is in. The recorded zone is now a line under RC-6, where a clinician first types a local time.
- Not done / not confirmed: **this scoping call was mine**, taken from my own recommendation after the maintainer's "next" without an answer to it; it changes no code and can be undone by unticking RC-9. No clinician or maintainer has reviewed it. No hazard-log change.

## 2026-10-26 - Service: the observation rules, pure (RC-6, first slice)

- `observations.rs`: `validate` and `flag` for eight vital signs (LOINC), one UCUM unit each with no conversion, a value or a stated reason for none (never zero), implausible values refused, and the sender's reference and critical ranges; no range means the flag is `None`, never normal. 15 unit tests; I moved three boundaries on purpose (low end, critical high end, the maximum bound) and each broke a test. Writing the tests found two flaws in my first version (a Fahrenheit value was also called implausible; a broken critical range drew a second complaint); both are fixed.
- Decisions taken from my own proposal, which the maintainer accepted with "y": store only the sender's range, one unit per code, refuse rather than flag the impossible. **Not done:** the plausibility bounds and the eight codes are guesses and no clinician has reviewed them; no store, route, audit row, UI, FHIR, or the whole-HTTP 422, so RC-6's acceptance is **not met** and no clinician sees anything new. The hazard log is unchanged because nothing can alter what a clinician sees yet: a hazard row for wrong flags is owed when this is wired. The time-zone decision for a time of day is still open.

## 2026-10-26 - Service: observations stored, read and corrected over HTTP (RC-6, H-034)

- Migration 14 adds the append-only `observation_versions` (triggers refuse update, delete and truncate) and the `observation_current` view in the anti-join form `tests/current_views.rs` requires. `GET`/`POST /api/patients/{id}/observations` and `PUT …/{observation_id}` follow the allergy routes: role, care relationship or break-glass, purpose, consent; the write and its audit row in one transaction; a refused record is 422 `{errors, error_codes}` and is audited; entered-in-error is final and needs a permission a nurse lacks; reads follow a merge. `chrono` is now a direct dependency (it was already in the lock; only the dependency edge was added).
- Decisions of mine, not confirmed by the maintainer: the measured time is a UTC instant with the site's zone stored beside it (this settles RC-6's question for the first slice, and the author's and patient's zones are not recorded); a measured time more than 5 minutes ahead is refused; a proxy has no scope for observations; the flag is computed on read, never stored.
- Found by the tests: `serde(flatten)` cannot read a number back when serde_json's `arbitrary_precision` is on (loco turns it on), so the record is explicit and flat. The first HTTP test failed on exactly this.
- Tests: 22 unit, 7 store (versions, a stale writer, final mistake, UTC, flag not stored, race, immutability, merge by provenance), 1 HTTP roles test (receptionist 403, nurse cannot retract, proxy 403, no relationship 403 with the break-glass offer, 422 codes, no value in any audit row), and the two new routes in the audit-failure census (500 and no row, then 200). The full database suite passes except the two FHIR validator tests, which need `FHIR_VALIDATOR_JAR` and were not run.
- **Not done: any UI**, so no clinician sees an observation; FHIR `Observation`; a device feed; trends and NEWS2; a proxy scope; the sender's range is trusted as given; the eight codes, the bounds and the 5-minute limit are guesses and no clinician has reviewed them. H-034 is new and `open`, with ratings (severity 4, likelihood 3) that nobody has reviewed. The status is for the Clinical Safety Officer.

## 2026-10-26 - UI: vital signs list and record form (RC-6, H-034)

- `/patients/[id]` gains a "Vital signs" section: the list (value and unit, where it sits in words, the range given, the site's time with UTC, author, version) and a form to record one. Decisions I made after your "y" to my design, kept as I proposed them: a missing range says "Not assessed (no range given)", never normal; units are fixed per vital; entered-in-error entries are collapsed, not hidden; the first slice is a list and the form, correction and retraction come later.
- Details worth knowing: the form's time is the site's wall clock converted to UTC in the page (a skipped hour reads an hour earlier, a repeated hour the first of two: arbitrary but fixed, and tested); a number box that is empty or unreadable is refused in the page, so nothing is sent as 0; "No value" needs a reason. The refusal text from the API is shown in English.
- Tests: 21 vitest (parsing, every no-value and flag wording, the time conversion on both sides of both New York clock changes, the form as a request), 3 stubbed Playwright tests with axe (list words; none, not shown, could not load; recording with the unit, UTC time, the no-value path, a 422 and a 409), and a real-Keycloak test (record through the page, high with a range, not assessed without one, a future time refused by the real API, a patient with no relationship says "not shown to your role"). The rest of the UI (193 vitest, 74 Playwright) and the Keycloak suite (11 passed, 11 skipped for missing users) pass.
- **Not done:** correcting or retracting from the page, earlier versions of an entry, trends, any translation of the section, a clinician using or reviewing it. H-034's controls and residual are updated and its status is still `open`; the Clinical Safety Officer has not seen the screen. A wrongly keyed value inside the bounds is shown as recorded.
