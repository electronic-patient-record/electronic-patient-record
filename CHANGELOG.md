# Changelog

Each entry says what was not done or not verified. Per-crate changelogs hold the detail.

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
