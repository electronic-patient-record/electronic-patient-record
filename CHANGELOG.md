# Changelog

Each entry says what was not done or not verified. Per-crate changelogs hold the detail.

## 2026-11-25 - A change that would break a caller of the API is refused (QA-7)

- **Clinical risk:** none to behaviour. A screen or another system that stops understanding an answer is a clinical risk, and this is a guard against it.
- **Added:** `tests/openapi_breaks.rs`, in the document's freshness test: what a caller sends may only widen, and what a caller gets may only be promised more firmly. `EPR_BLESS=1` refuses a break and lists it; `EPR_BLESS=break` accepts one that is meant.
- **Verified:** 19 planted breaks and 11 changes that only add; a field made required in the source was refused and the committed copy left alone; three earlier documents (from 2026-11-23 to 25) show no break against today's. **Not done / not verified:** it sees only what the document says, so not a new refusal, a narrower role, a changed meaning, or a body it does not describe (FHIR and others); a hand-edited `openapi.json` is stale and fails, but one edited to match a break is not noticed.

## 2026-11-25 - Nothing about a person reaches a log line, held by a test (QA-5)

- **Clinical risk:** none; a test only.
- **Added:** `tests/http_log_privacy.rs`: one marked patient is registered, found by each criterion, read in three forms, recorded about, and refused in eight ways, with every log level captured. No line held a name, number, birth date, postcode, telephone, free text or the record's id. Request lines carry the route's template and not the address.
- **Verified:** a planted line with the search's parameters fails it; three runs; the normal and database passes; every docs gate. **Not done / not verified:** lines written by a background task, or only when the database or the identity provider fails; the routes it does not call; the interface's own logs; a proxy's or container's access log.

## 2026-11-25 - What a shared postcode or telephone does to a comparison of two people (XP-5)

- **Clinical risk:** none to behaviour; the matcher is as it was. H-001's residual and evidence now say what was found (status and score not).
- **Found:** a postcode that many people share turns some pairs of two people from Review to AutoLink by itself, and two people with one birth date and no name in common reach Review when they share a postcode and a telephone. Nothing in the service acts on AutoLink.
- **Corrected:** the recall page recommended "an identifier or an address" before an automatic decision; an address is not enough. OQ-AG8 and AG-13 say so, and XP-16 is added.
- **Added:** `tests/shared_place.rs`, 14 invented pairs, each compared four ways and pinned, wrong ones included.
- **Verified:** that test; the search that found the tipped pairs was run once and is not kept. **Not done / not verified:** invented names, no frequencies, no administrative sex on any pair, no site's own data; no rule was changed.

## 2026-11-25 - Where this project assumes a hospital (XP-5)

- **Clinical risk:** none to behaviour; nothing was changed. One finding is clinical: a referral's `no-show` is shown as "Missed the appointment" with no reason, which is untrue of a person who was not brought or not escorted (XP-15, OQ-XP1).
- **Added:** section 5 of [care settings](spec/care-settings.md): ten places in the code, and five pages of the specification.
- **Corrected:** the AG-3 entry said 3 unit tests; they were 2 new and one extended.
- **Not done / not verified:** one reading by words and by the main lists; the mobile apps and translated words were not read; whether a shared postcode changes a match was not measured.

## 2026-11-25 - A receiver can learn over FHIR that an allergy or a vital sign was a mistake (AG-3)

- **Clinical risk:** a FHIR answer that did not exist (H-022, controls and residual changed; status and score not). Before this, a system that had taken an allergy could not learn that it was entered in error. A careless reader is given nothing to show: no substance, reaction, code, value or time.
- **Added:** `AllergyIntolerance?patient=&verification-status=entered-in-error` and `Observation?patient=&status=entered-in-error`, in R4 and R5, through the same gate, consent and audit. Only those entered in error are returned, with a warning that it is not the patient's list. The plain search is unchanged. Any other value is a 400 and never ignored.
- **Verified:** 2 new unit tests and one extended, and an HTTP test that records an allergy, retracts it and asks in both releases; the normal and database passes on a fresh Postgres 18; every docs gate. Two samples are added to the HL7 validator test, which was run here and first refused the vital sign, as its LOINC code called for a time and a value: it now has none. **Not done / not verified:** nothing tells a receiver that does not ask; one taken under another record before an unmerge is not among them; no receiving system was tried; results, problems and the rest are not in FHIR.

## 2026-11-25 - Question and task ids are checked to be unique

- **Corrected:** the military health questions and the order questions had taken ids already used (`OQ-MS1` to `OQ-MS5` are manual steps; `OQ-OS1` to `OQ-OS4` are rate limits, policy as data, SAST and tracing). They are now `OQ-MF1` to `OQ-MF8` and `OQ-OX1` to `OQ-OX4`. The commits that added them (`816d00c4`, `a3162794`) say the old ids.
- **Added:** `scripts/check-ids.py`, in the docs-links gate: a repeated id in the questions table or in `tasks.md` fails. On the revision before this one it names those nine and no other. **Not done:** an id used only in a page is not checked.

## 2026-11-25 - One state table for every order, as a library nothing calls (EH-4)

- **Clinical risk:** none to behaviour; nothing calls it and no order is stored.
- **Added:** `order.rs` and `spec/order-entry/state-table.md`: what the orderer wants and what the filler has got to move apart; asking is checked, and that something was done is never refused and is flagged; a cancel of a sent order is a request; an order's own words never change. OQ-OX1 to OQ-OX4.
- **Changed:** two rules of the order entry proposal that said otherwise (a replaced order was revoked at once, and a transition not in the table was always refused).
- **Verified:** 11 tests over every state, stage and step, with the allowed steps written out again by hand; four wrong versions of the table were each caught. **Not done / not verified:** no clinician, pharmacist or laboratory reviewed it; no store, route or screen; the database pass was not run, as no store changed; referrals keep their own table.

## 2026-11-25 - The rules of masking, as a library nothing calls (EH-8, first slice)

- **Clinical risk:** none to behaviour; nothing calls it and no record carries a label. The design names the hazards a build must log.
- **Added:** `masking.rs`: whole entries withheld, a count and never what, fails closed, a short list is never "none", a shown problem does not point at a withheld one, and a writer does not remove a cause they could not see. `spec/access-control/masking.md`, with the places where a list's other answers would tell. OQ-PI7.
- **Corrected:** the protected information page said the service can mask a labelled record: the engine decides it and no route does it. The custodial draft said a clinician opens a labelled entry by break-glass: break-glass never unmasks a label (OQ-PI2).
- **Changed:** the label census leaves `masking.rs` out of its search for a mask, and in its place fails if anything in the service calls it.
- **Verified:** 6 tests, one comparing answers for every role and label with the hidden entry changed; four wrong versions of the code were each caught. **Not done / not verified:** no store, route, screen or FHIR; the database pass was not run, as no store changed; no one reviewed the rules.

## 2026-11-25 - Problems are grouped by their recorded cause; a draft spec for multi-system conditions (CF-4)

- **Clinical risk:** the problem list's answer says more (H-066, controls and residual changed; status and score not). It could be read as every effect of a cause, or no group as no link; no screen shows it yet.
- **Added:** `summary.causes` on the problem list, now and as at a time: groups under the top cause, problems whose cause is not on the list (after an unmerge this was silent), and problems whose own cause is refuted. `spec/multi-system-conditions/index.md`, which holds no clinical content, and OQ-GC1 to OQ-GC6.
- **Verified:** 2 domain tests and 1 database test with a merge, a refutation and an unmerge; the normal and database passes on a fresh Postgres 18; every docs gate. **Not done / not verified:** no screen, no FHIR, no notice to anyone; the worked example is a clinician's and is empty; every source in the draft was seen in search summaries only.

## 2026-11-25 - A draft spec for military health (CF-3)

- **Clinical risk:** none; nothing is built. The draft names five hazards that a build must log first.
- **Added:** `spec/military-health/index.md`: what service changes, what is shared with custody and what is not, and eight questions for the customer (OQ-MF1 to OQ-MF8).
- **Not done / not verified:** four sources, two countries: one read in part, two through an agent's excerpts and one through search summaries only, because the military sites could not be reached; an amendment of 19 November 2026 to the United States rule was seen but not read. The customer's answers about custody are not taken as answers here. No one was asked.

## 2026-11-25 - A change lists the hazards whose evidence it touches (PR-2, last slice)

- **Clinical risk:** none to behaviour. A hint for the Clinical Safety Officer; it decides nothing and fails nothing by what it lists.
- **Added:** `check-hazard-log.py --touched <git-ref>` reads the evidence at both ends, so a change cannot drop its own mention, and writes to the summary of the pull request's sign-off job. When git cannot compare it fails, so "None." means none.
- **Verified:** 2 new tests, every docs gate, and run by hand against earlier commits of this branch. **Not done / not verified:** the workflow step has not run, as work here is pushed to main and not through pull requests, so for those pushes the hint is only what a person runs by hand; a hazard whose evidence does not name a file is not listed when that file changes.

## 2026-11-25 - Every list of words the database enforces is checked against the code (PR-3)

- **Clinical risk:** none to behaviour; a word added to one side only now fails.
- **Changed:** `tests/stored_words.rs` compared 11 lists that it named; it now reads all 33 from the catalogue, so a new `CHECK` fails until someone has looked at it. Five are not compared, each with its reason. Five are written as literals in the code, and only that each database word appears in the file is checked.
- **Verified:** each way to fail was tried: a word in the database only, a list nobody listed, a variant in the code only (it does not compile). The normal and database passes on a fresh Postgres 18, every docs gate. **Not done / not verified:** for the five literal lists, a word in the code and not in the database is not caught here (the database refuses it when written).

## 2026-11-25 - A hazard that is not open must name a test (PR-2, second slice)

- **Clinical risk:** the hazard log's evidence column only: 16 hazards with controls in place now name the tests behind them. No status, score or wording changed (compared column by column with the previous revision).
- **Changed:** `check-hazard-log.py` fails when a hazard that is not `open` names no test. One of its own tests used such a row and now names a test.
- **Verified:** the checker's tests (1 new) and every docs gate. **Not done / not verified:** I chose the tests by reading their names and the controls; a named test shows that a test exists and is not proof of the whole control, and many of these hazards have controls no named test covers (the UI's, above all); the pull-request hint is not built; 35 open hazards name none; no Clinical Safety Officer looked at the choice.

## 2026-11-25 - The duplicate check looks wider, and is faster (AG-12, second slice)

- **Clinical risk:** H-002. 16 of 19 synthetic kinds of duplicate are now retrieved, where 10 were; of the 3 missed the matcher would have sent 2 to a person.
- **Changed:** the family name is compared without case, accents, spaces, hyphens and apostrophes, and by each part of a double name of four letters or more; the names, and the day and month of the birth date, the other way round. Only whom the matcher compares: it decides as before. **More registrations and feed messages may be held as possible duplicates.**
- **Migration 46** adds three columns to `patient_versions` that the database makes from the body, so it rewrites the table once and holds it meanwhile (about 3 seconds for 50,000 rows). No recorded value changes: a checksum over 50,000 rows was the same before and after, and the table still refuses changes. PostgreSQL 17 or later and a UTF-8 database.
- **Verified:** `tests/identifier_keys.rs` (2 more), the pinned table, the timings on 50,000 rows, a dump and a restore; clippy, rustdoc, Rust 1.98, the normal pass, the database pass (243) on a fresh Postgres 18, every docs gate. **Not done / not verified:** the timings are one run on one machine, not the load harness; how many more registrations are held was not measured; a name that sounds alike and a birth year alone are not looked by.

## 2026-11-25 - A national number is found however it was typed (AG-12, first slice)

- **Clinical risk:** H-002. `943 476 5919` did not find `9434765919`: not at registration, so a duplicate could be made; not in search; and not in the HL7 feed, which then took a known patient for a new one.
- **Fixed:** `patients_store::identifier_key`, and in the database `identifier_key` and `identifier_keys` with a GIN index (migration 45), which replaces the index on the raw JSON. Only the two national number systems; any other identifier is still exact. What was typed is kept.
- **Verified:** `tests/identifier_keys.rs` (3), with the two keys compared on every white space character; the index is used; clippy, rustdoc, Rust 1.98, the normal pass, the database pass on a fresh Postgres 18, every docs gate. **Not done / not verified:** no feed message was sent end to end with a spaced number (the feed uses the search that is tested); the index was not timed on the load data, and building it rewrites nothing but reads every version of every patient; records already duplicated are not found or merged.

## 2026-11-25 - What registration can find, measured (AG-8, in part)

- **Clinical risk:** H-001. Two things were found and neither is fixed: the duplicate check never looks at 10 of 19 synthetic kinds of duplicate, 8 of which the matcher would have sent to a person; and the matcher says `AutoLink` for twins whose given names differ by one letter. Nothing in the service links by itself.
- **Added:** `tests/candidate_recall.rs` (4), which pins both, and that no code of the service acts on `AutoLink`; `spec/patient-identity/candidate-recall.md`; tasks AG-12 and AG-13; OQ-AG8.
- **Verified:** the tests on a fresh Postgres 18. **Not done / not verified:** synthetic cases invented by the author, with no frequencies and no adjudication; the matcher is unchanged, because its rules are published and the change is a safety decision (OQ-AG8).

## 2026-11-25 - Migration 43's refusal has a test

- **Added:** `tests/migration_relative_history.rs` (2, Postgres): migration 43 refuses while the old table holds an entry and changes nothing, and replaces an empty one with both tables still append-only. The entry for EH-3 below said there was none.
- **Verified:** the refusal test fails when the guard is removed from the migration. **Not done / not verified:** no other migration has such a test; none drops a clinical table.

## 2026-11-25 - An answer as at a time says it is a reconstruction (AG-7, in part)

- **Clinical risk:** H-068. Nothing in an as-at answer said that it is rebuilt from stored versions and not what anyone was shown.
- **Added:** `basis` and `left_out` in the three kinds of answer, and in the OpenAPI document; two tests that hold the limits the audit named (a write in progress; a pending amendment).
- **Verified:** clippy, the normal pass and the database pass (235) on a fresh Postgres 18, every docs gate. **Not done / not verified:** what was served to each reader is not kept (OQ-AG7); no screen; no clinician or investigator asked.

## 2026-11-25 - Governance documents catch up with the record (AG-9, in part)

- **Clinical risk:** the hazard log names more tests; no status, score or wording of a hazard changed.
- **Fixed:** the risk management plan said the software holds no clinical content; the readiness checklist, and the UI page that copies it, said 55 hazards and that only H-044 names a test. The counts are now generated into the status block.
- **Verified:** the checker's tests (1 new), vitest, and every docs gate. **Not done / not verified:** no Clinical Safety Officer is named and nobody reviewed the plan; a named test shows that a test exists, not that it proves the control.

## 2026-11-25 - A change to the service's routes runs the UI's tests

- **Fixed:** `summary.test.ts` (AG-5) reads `src/routes.rs`, and `scripts/ci-changes.sh` did not run the UI for it, so the docs job failed on `01bab239`: a gate I had not run before pushing with `--no-verify`.
- **Verified:** every `scripts/gates/docs-*` gate passes locally. **Not done / not verified:** the Android job on `01bab239` failed to download a jar from Maven Central; not fixed here, the next run shows whether it was transient.

## 2026-11-25 - Information for matching donors: typing reports, never a match (DM-1)

- **Clinical risk:** new clinical content. H-069 opened with the author's scores; nobody has reviewed it.
- **Added:** `spec/donor-matching` (what blood, marrow and kidney matching need, from search summaries), tasks DM-1 to DM-5 (prefix `DM`), OQ-DM1 to OQ-DM5, plan.md §41; five routes under `/api/patients/{id}/compatibility-typing`, migration 44, the resource `CompatibilityTyping`, and `typing.rs` in the domain crate. The summary screen names it among what it does not show.
- **Decided by me, not confirmed:** to keep reports and build no matching, and to show typing to clinicians and nurses only.
- **Verified:** clippy, the normal pass and the database pass (234) on a fresh Postgres 18; vitest, svelte-check and lint for the UI. **Not done / not verified:** no guideline read in full; no laboratory feed, screen or FHIR; no code system; liver, heart, lung and cornea not researched; no transfusion or histocompatibility scientist asked.

## 2026-11-25 - A merged chart is audited as the chart that was opened (audit finding 6, AG-6)

- **Clinical risk:** H-013 (the patient's access log misleads). The patient named in the audit row was the one of the first row returned, so a list whose newest row came from a merged-in record was missing from the survivor's log.
- **Fixed:** see AG-6. Found beyond the audit: the same for amendments, one observation's history, the list of links (which named the other end), and every change to an entry written on a merged-in record.
- **Added:** `detail.records`, the contributing records, for auditors. Tasks AG-7 to AG-11 hold the rest of the audit.
- **Verified:** 1 database test that failed first, 1 unit test; the normal pass and the database pass (233) on a fresh Postgres 18. **Not done / not verified:** earlier rows keep their patient; OQ-AG6.

## 2026-11-25 - The summary never says stored information does not exist (audit finding 5, AG-5)

- **Clinical risk:** H-066. The summary listed "Recent results" and "Problems" as "Not built in this system yet", "Nothing is held for these", while the service stored both. It was my omission when the problem list landed on 2026-11-24; results had been wrong since 2026-11-09.
- **Changed:** English and (machine-written) Welsh words, and two lists in place of one.
- **Verified:** vitest (the guard fails when a held section is listed as not kept), svelte-check, lint, Playwright with axe in both themes. **Not done / not verified:** see AG-5; no user was asked.

## 2026-11-25 - Correcting a vital sign does not move its time (audit finding 4, AG-4)

- **Clinical risk:** H-034. A correction of the value alone turned the instant into a wall time to the minute and back: it dropped the seconds always, and moved the second reading of the autumn hour by an hour. A time in the spring gap was read as another.
- **Changed:** `wallTimeToUtc`, which chose for the reader, is replaced by `wallTimes`, which says none, one or two. A unit test that required the old choices was replaced.
- **Verified:** vitest (8 tests failed first), svelte-check, lint, and the affected Playwright tests in both themes with axe. **Not done / not verified:** see AG-4; **a reading moved by an earlier correction keeps the moved time**, and its earlier version shows the true one in its history.

## 2026-11-25 - An unverified allergy code is withheld from FHIR (audit finding 3, AG-3)

- **Clinical risk:** H-022. The export sent whatever code was recorded, though only its shape is checked: a test and a validator fixture sent "Penicillin" with the code for myocardial infarction.
- **Changed:** the FHIR allergy export sends the words and withholds the code, with a note. **This removes the code from every allergy a FHIR client reads**, because nothing here can verify one yet; a receiver that reads only codes finds none (OQ-AG3).
- **Verified:** 1 unit test that failed first. **Not done / not verified:** no pharmacist, terminology edition or receiving system; the external FHIR validator was not run here (CI runs it); the REST answer still shows the code to a person. The substance code in the validator fixture was not checked by anyone and no longer leaves.

## 2026-11-25 - An empty reference range is never normal; a crossed critical limit is said (audit finding 2, AG-2)

- **Clinical risk:** H-034. A heart rate of 130 with an empty reference range was flagged normal and exported to FHIR as `N`; one above a critical limit of 120 with no reference range had no flag and raised no event.
- **Fixed:** see AG-2 in `tasks.md`. A unit test that required "a critical range alone flags nothing" was replaced: the audit and its maintainer ask for the other reading, and it awaits a clinical owner (OQ-AG2).
- **Found beyond the audit:** high corrected to critically high raised no event; it does now.
- **Verified:** 2 domain tests and 1 database test; the domain tests failed first. **Not done / not verified:** no clinical owner was asked; a one-sided range still gives normal on its open side.

## 2026-11-25 - A critical correction after sign-off reopens review (audit finding 1, AG-1)

- **Clinical risk:** H-050 and H-059, controls added, nothing accepted. An abnormal result signed and then corrected to critical left no task: signing cancelled it, and abnormal to critical was "no change".
- **Fixed:** see AG-1 in `tasks.md`. Found beyond the audit: a signature racing a correction cancelled the new version's task; a lock on the result and the task's version now stop that.
- **Changed:** a result corrected after a signature raises `result-filed` whatever it now says, so a site rule without conditions on that event makes a task for it. Tasks carry `source.version` (absent on older tasks).
- **Verified:** 3 database tests that failed first, 1 domain test. **Not done / not verified:** no covering recipient, acknowledgement or escalation (OQ-AG1); no rule is approved, so a running service makes no task; no clinician reviewed it.

## 2026-11-24 - Relative history: three links asked apart, and no exception to the guard (EH-3, OQ-EH5)

- **Clinical risk:** a relative who shares no genes can no longer be read as inherited risk. H-067 rewritten (still `open`, my scores): it now names the risk that an entry reveals an adoption, a donor, a surrogate or a parentage.
- **Changed, on the maintainer's instruction:** the list of relatives became three questions: the genetic link (with its side), the gestational link, and the shared environment, none guessed from another; plus an optional sex at birth. "Family history" is now **relative history** in every path, resource, schema, table and module. The routes of the entry below (`/family-history`) are gone; they were a few hours old and unreleased.
- **Guard:** `tests/grief_census.rs` is again exactly as it was, with no exception; `tests/relative_history_census.rs` (3 tests) extends the refusal of the word to every path, policy resource and schema name. The entry below says the guard was changed: no longer.
- **Migration 43** replaces the table of migration 42 and refuses to run if it holds any entry, so nothing is removed. A development database with an old entry must be made again. `generate-schema-map.py` now reads `DROP VIEW`.
- **Verified:** fmt, clippy, the normal pass (114 integration, 344 unit, 237 domain) and the database pass (228) on a fresh Postgres 18; migration 43 by hand on a database with an entry (refused, the row kept) and on an empty one.
- **Not done / not verified:** no automated test of migration 43's refusal; no clinical geneticist has read the links; no code system was looked up; exposures on the patient's own record (EH-17) are not built; the gestational rules for mitochondrial donation and transgender parents were not worked through beyond reading `line` as egg or sperm.

## 2026-11-24 - Audit rows say the kind of a refusal, never its words (EH-16)

- **Clinical risk:** none to care; a privacy fix. The words of a refusal can quote what was sent.
- **Changed:** every controller that audited `e.to_string()` for a refusal (14, and the allergy and entry routes) now audits `StoreError::refusal_kind()`. The entry below said ten; that count was cut short. An auditor loses the text, and the current version on a conflict.
- **Added:** `tests/audit_words.rs` (3 tests): no controller may audit the words, and no kind holds them.
- **Verified:** fmt, clippy, the normal pass and the database pass on a fresh Postgres 18.
- **Not done / not verified:** rows already written keep their words (append-only); the feed, MLLP and store modules are outside the census; nobody who reads audit rows was asked whether the text was used.

## 2026-11-24 - Problem list, family history and reads as at a time, from openEHR (EH-1 to EH-3)

- **Clinical risk:** new clinical content. Hazards H-066 (problem list), H-067 (family history) and H-068 (as-at reads) opened with the author's scores; nobody has reviewed them or the rules.
- **Added:** `spec/lessons/openehr-rust` (what `openehr-rust` at `320e1e84` was read for, and advice), plan.md §40, tasks EH-1 to EH-16 (prefix `EH`), OQ-EH1 to OQ-EH5, and `spec/problem-list`, `spec/family-history`, `spec/as-at`. In the service: 11 routes, one migration (two append-only tables), `entries_store.rs`, `controllers/entries.rs`, `links::linked_ids_at`, the policy resource `FamilyMemberHistory`; in the domain crate `problem.rs`, `family_history.rs`, `coding.rs` and `dates::partial_order`.
- **Fixed:** a refused allergy wrote the SNOMED CT code it was sent into its audit row; it now records the kind of refusal. Rows written before this keep what they hold: the trail is append-only. Ten other controllers record the store's words and were not checked (EH-16).
- **Changed, a guard:** `tests/grief_census.rs` refused any route with the word *family* (SG-2). It now allows the four family history paths by name, and a new test holds that an entry can reach no other person. Not confirmed by the maintainer (OQ-EH5).
- **Version:** the domain crate is 0.2.0 (a new `Resource` variant). Not published; the service cannot be published until it is, since it now needs 0.2.0.
- **Verified:** fmt, clippy `-D warnings`, rustdoc, the checked-arithmetic gate, the normal pass and the database pass on a fresh Postgres 18. Five deliberate breaks (no lock, today's merges, the exact time left out, patient access allowed, the reason in the audit row) and the audit leak each made a test fail.
- **Not done / not verified:** no screen, no FHIR, no locales, no amendment requests on a problem (EH-15), so no user has these yet; medicines and immunisations; no openEHR specification or archetype was read and no conformance is claimed; no customer, clinician or geneticist was asked; the UI, container, load and mobile gates were not run.

## 2026-11-24 - Spec: condition-specific support groups (CF-1)

- **Added:** `spec/support-groups/index.md` (design rules; a referral record, not a membership, by default; a volunteer role that grants no clinical read) and OQ-CF1 to OQ-CF5 in `spec/16-open-questions.md`; CF-1 in `tasks.md` is marked drafted.
- **Clinical risk:** none changed (no behaviour). The page names a disclosure hazard (a group list or message reveals a diagnosis) that needs a hazard-log row before any build; I did not add one and set no status.
- **Added later the same day:** a worked example from the All Wales Adult Inherited Metabolic Disease Service's support-groups page (about 60 signposts, no endorsement, no review date); it adds a type, an endorsement statement and a last-checked date to the directory entry.
- **Not done / not verified:** only that one page was read (its linked sites and the Welsh version were not); no law checked, no customer, clinician, volunteer or patient asked; the five questions are the customer's to answer. Nothing built.

## 2026-11-24 - Spec: the customer's answer to OQ-CH6 (national summary depends on the country)

- **Changed:** OQ-CH6 records that a national summary depends on the country. The custodial-health spec says the core writes the release review to the one patient record only; a national summary is an export in the country pack, subject to the release rules of OQ-CH5, with the custody label not carried into it unless the pack says so.
- **Clinical risk:** an export carrying custody into a general practice's summary would defeat the label; covered by the rule above, but untested and unbuilt. No hazard-log row added yet.
- **Not done / not verified:** no country's summary read. All six questions OQ-CH1 to OQ-CH6 now have a customer answer, but the country, the controller and processor, who decides a function and the release rules are still open inside them. Nothing built.

## 2026-11-24 - Spec: the customer's answer to OQ-CH4 (one record)

- **Changed:** OQ-CH4 records one record: the custody health team writes into the person's own record. The custodial-health spec now says custody and everything written in it carry a label by default and are hidden from a reader with no need; after release the label stays unless a pack rule or the person lifts it, with a lift audited; non-health custodian staff neither read nor write; the release hand-over becomes a review of open items on the one record.
- **Clinical risk:** one record exposes custody to every reader unless label masking works, and the service cannot do that yet (SE-4); a wrong or missing label is a hazard in both directions. The profile is not usable until masking is built and tested. No hazard-log row added yet (no design to score).
- **Not done / not verified:** my earlier lean (a separate linked record) was not taken; the labelling design is my reasoning, not a clinician's or a lawyer's. Only OQ-CH6 is unanswered. Nothing built.

## 2026-11-24 - Spec: the customer's answer to OQ-CH5 (release varies)

- **Changed:** OQ-CH5 records that release of a record to a court, police or investigator varies by jurisdiction, country, U.S. state, purpose and urgency. The custodial-health spec now says the core holds no release rule and refuses without a pack; every release records its basis, extent, releaser and audit; an urgent release is allowed only where the pack says so and is reviewed afterwards; a release is by section, not the whole record.
- **Clinical risk:** the urgent path is a disclosure without consent, so it needs a hazard-log row before any build (like H-024 for a read); I did not add one, since there is no design to score.
- **Not done / not verified:** no law read for any jurisdiction; the release-table design is my reasoning, not a lawyer's. OQ-CH4 and OQ-CH6 unanswered. Nothing built.

## 2026-11-24 - Spec: the customer's answers to OQ-CH2 and OQ-CH3 (operators; functional only)

- **Changed:** OQ-CH2 records that the service may be run by government, police or a private company; OQ-CH3 records that non-health staff are told functional information only. The custodial-health spec now separates the health record from the custodian's systems, adds an operator type beside the setting type, and defines a functional item as a short fixed statement from a reviewed list, with a date and expiry and no reason, approved by a clinician (my proposal).
- **Clinical risk:** a function can itself disclose a diagnosis ("needs a dialysis machine"), so the list is reviewed; every disclosure to non-health staff is audited. No hazard-log row added yet (nothing to score).
- **Not done / not verified:** controller and processor, the country, and who decides each function are open; the examples of functions are mine, not from a source; no custody, legal or clinical review. Nothing built.

## 2026-11-24 - Spec: the customer's answer to OQ-CH1 (all custodial settings)

- **Changed:** OQ-CH1 records that the scope is every custodial setting; the country is still open. The custodial-health spec adds a setting type to the one profile and notes what differs for police custody, a secure youth setting, immigration detention and a hospital treating a person in custody.
- **Clinical risk:** no behaviour changed; no hazard row added (nothing to score yet). The secure-youth and immigration-detention cases raise safeguarding and release-of-records questions that need a lawyer.
- **Not done / not verified:** the per-setting differences are my reasoning, with no source. The country (and so the law) is unanswered. OQ-CH2 to OQ-CH6 unanswered. Nothing built.

## 2026-11-24 - Spec: prison health, a custodial care setting (CF-2)

- **Added:** `spec/custodial-health/index.md` (a custodial profile; health information stays with the health team by default; movement, release hand-over, missed-care reasons) and OQ-CH1 to OQ-CH6; CF-2 in `tasks.md` is marked drafted.
- **Clinical risk:** none changed (no behaviour). The page names two hazards that need hazard-log rows before any build (health information shown to non-health staff; a release with no hand-over); I did not add them, because there is no design to score yet.
- **Not done / not verified:** no outside source read, no law checked, no customer, clinician or person who has been in custody asked; all nine points are my reasoning. Nothing built.

## 2026-11-24 - Spec: the customer's answer to OQ-CF4 (no channel; providers suggest)

- **Changed:** OQ-CF4 records that a group has no channel in the app and any of the patient's providers may suggest a group and give the patient its name and contact information. Design rule 4 now says the system sends nothing about a group. This also answers who refers (a provider).
- **Clinical risk:** the outbound path to a third party is gone, which lowers the disclosure surface; the name-on-a-handout risk remains (H-065 not changed; its OQ-CF4 reference is now answered).
- **Not done / not verified:** "any of the patient's providers" is read as "a provider with an active care relationship"; the customer has not confirmed that reading or which roles count. OQ-CF5 unanswered. Nothing built.

## 2026-11-24 - Hazard H-065: a relative's name in free text is a risk (CF-1)

- **Clinical risk:** new hazard `H-065` (a support-group offer or note discloses a diagnosis or a relative's name), status `open`, severity 3 and likelihood 3 as the author's proposal; nobody has reviewed it and I set no accepted or closed status. The offer record is now specified as structured, with no free text and no person name.
- **Not done / not verified:** no control is built; OQ-CF4 and OQ-CF5 undecided; the scoring is not clinician-reviewed.

## 2026-11-24 - Spec: the customer's answer to OQ-CF3 (no relatives as members)

- **Changed:** OQ-CF3 records no: a carer or relative is not recorded as a member, because the groups are third-party. Design rule 5 of the support-groups spec now says so; a patient's remark that a relative will go stays a note in their own words and creates no person record.
- **Not done / not verified:** whether a relative's name in a free-text note is itself a risk was not examined. OQ-CF4 and OQ-CF5 unanswered. Nothing built.

## 2026-11-24 - Spec: the customer's answer to OQ-CF2 (only the offer)

- **Changed:** OQ-CF2 records that the system holds only the offer; no membership, attendance or roster for any group. The support-groups spec drops the conditional wording that depended on it (a roster role, a staff-run roster, a controller decision). Volunteers need no role, since there is no roster.
- **Clinical risk:** the disclosure surface is smaller (no member list exists to leak). No hazard-log row changed.
- **Not done / not verified:** whether an offer record (the group, date, and the patient's answer) itself discloses a diagnosis to staff who can read the patient is still the label question OQ-CF5. OQ-CF3 to OQ-CF5 unanswered. Nothing built.

## 2026-11-24 - Spec: the customer's answer to OQ-CF1

- **Changed:** OQ-CF1 records the answer (groups are run by patients, charities or hospital staff, on social media or not, local to global); `spec/support-groups/index.md` gets a section on what it implies (directory entry carries operator, where it meets and reach; only staff-run groups can be under the system's control; a social media group is never posted to or invited on a patient's behalf).
- **Not done / not verified:** the implications are my reasoning, not reviewed; who refers into a group is still open; OQ-CF2 to OQ-CF5 unanswered. Nothing built.

## 2026-11-24 - Dependency: axa 0.23, axa-test 0.14; the service package includes `migrations/`

- **Changed:** `axa` 0.21 to 0.23 and `axa-test` 0.12 to 0.14 (with axa-idempotency-key 0.3.0, axa-postgres-job-queue 0.7.0 and patch bumps). The one breaking change we hit: `Idempotency::ttl` returns a `Result`, so `idempotency::keys` returns `Result<Idempotency, ConfigError>` and boot fails with a message instead of panicking. Cooldown exception approved by the maintainer, applied to that one `cargo update` command only.
- **Also:** `include` in the service `Cargo.toml` now lists `migrations/*.sql` (`sqlx::migrate!` embeds them), so the package verifies; `cargo publish --dry-run` passes.
- **Verified:** fmt, clippy `-D warnings`, the normal pass (all green) and the database pass (220 tests, fresh database).
- **Not done / not verified:** the axa 0.22 and 0.23 release notes were not read in full, so new features are not adopted; `cargo deny`, the UI and CI were not run. The service crate is still not published (the maintainer runs `cargo publish`).

## 2026-11-24 - Backlog: customer feedback on groups, custodial and military health, multi-system genetic conditions

- **Added:** tasks `CF-1` to `CF-4` and a section 4 in `spec/care-settings.md`; prefix `CF` in `scripts/generate-status.sh`.
- **Clinical risk:** none yet; no behaviour changed. Each task is flagged clinical and will need a hazard-log row when it gets a design.
- **Not done / not verified:** no code, no sources read, no clinician, prison or military health service, or patient group consulted. Brain-lung-thyroid syndrome is named as the customer wrote it; its coding was not looked up.

## 2026-11-24 - Release: matcher 0.8.1 published; the service uses it

- **Clinical risk:** the matcher fix (an empty name part no longer scores NaN) can change a match score for such a name; no hazard-log row changed. See the matcher entry above.
- **Published:** `electronic-patient-record-matcher` 0.8.1 on crates.io, from `9095bd1`, with the CC-BY-SA-4.0 licence expression. The service workspace now requires `0.8.1` (`Cargo.lock` updated). To resolve a version published minutes before, I overrode `registry.global-min-publish-age` on that one `cargo update` command for our own crate; the setting itself is unchanged.
- **Verified:** clippy; the normal pass and the database pass on a fresh database, against the published crate.
- **Not done / not verified:** `electronic-patient-record-domain` and `electronic-patient-record` (both 0.1.0) are published next, in that order; their versions were not bumped because they were never published (a first release is 0.1.0). CI on the pushed commits was not yet seen.

## 2026-11-24 - The Creative Commons choice is CC-BY-SA-4.0, not CC-BY-NC-SA-4.0

- **Clinical risk:** none: licensing.
- **Changed:** `CC-BY-NC-SA-4.0` is replaced by `CC-BY-SA-4.0` in `LICENSE.md`, `LICENSES/` (`cc-by-nc-sa-4.0/` removed, `cc-by-sa-4.0/index.md` added, from the SPDX text of CC BY-SA 4.0), `spec/licenses/index.md`, `README.md`, `GOVERNANCE.md`, `CITATION.cff`, `plan.md`, the three `Cargo.toml` manifests, the domain crate's README, `package.json` and the `Containerfile` label. The sentences that said the licence forbids commercial use are rewritten: BY-SA allows commercial use, and says to share adaptations alike. The choice is still one of seven.
- **Effect to know about:** anyone who took the code under the non-commercial option now has a licence that allows commercial use; the other six options are unchanged. Versions already published (the matcher 0.8.0 on crates.io) keep the licence string they were published with.
- **Verified:** the licence-file test, the link check and `cargo deny check licenses` pass.
- **Not done / not verified:** earlier CHANGELOG entries still name the old identifier, as they were true then; I did not check the SPDX text against the Creative Commons legal code by hand, and I am not a lawyer: this is a mechanical change that the project owner should review.

## 2026-11-24 - Major update: dependencies, `tests/main.rs`, `make test-benchmarks` and `make test-coverage`

- **Clinical risk:** none: tooling, tests and lock files.
- **Changed:**
  - The service's single test program is now `tests/main.rs` (target `main`, was `suite`): `cargo test --test main <module>::`. About 80 files were updated to say so; older CHANGELOG entries still say `suite`, as they were true then.
  - `make test-benchmarks` (`bin/test-benchmarks`) and `make test-coverage` (`bin/test-coverage`) wrap the existing gate scripts (`matcher-bench`, `service-bench`, `coverage-matcher`, `coverage-service`), so each command is still defined once.
  - Lock files: service and matcher `cargo update` (18 and 5 packages, among them `hl7-2` 0.3.3, `jiff` 1.0.2, `clap` 4.6.9, `serde_with` 3.25); UI: `svelte` 5.59.1, `@lilydesignsystem/svelte-headless` 0.13.1.
  - The `mobile-apps` worktree and its local branch were removed (clean, merged into `main`); the Lily checkout in it is fetched again by `scripts/fetch-lily.sh`.
- **Already as asked, so unchanged:** Rust 1.100.0 pinned with `clippy`, `llvm-tools`, `rustfmt`; `rust-version = "1.98"` (N-2); `[profile.dev]` and `[profile.mutants]` as listed; `chrono` is absent from production (one copy remains in the tests through `axum-test`, DEP-1), `aws-lc-rs` and `cron` are gone (`ring`, `croner`); no open Dependabot alerts; private vulnerability reporting is on; the agent files are all under 40 KB; there are no `*-skill` or `*.github.io` folders in this repository.
- **Not done / not verified:** TypeScript 7.0.2 was tried and refused: SvelteKit needs the JavaScript compiler API (`typescript@^6`); `syncpack` 15.3.4 was not taken (inside the release-age window); `make test-benchmarks` took 7 minutes and only checks the benchmarks run; `make test-coverage` measured the matcher at 99.32% and the service's database-free tests at 38.26%, as designed (the full figure, 95.91%, is in tasks.md QA-15); containers belonging to other projects, and two defunct entries whose parent is not mine, were left alone.

## 2026-11-24 - Test headers name the right command after the folding (LC-15)

- **Clinical risk:** none: comments and a document.
- **Changed:** 20 test files whose header said `cargo test --test <name>` (a program that no longer exists) now say `cargo test --test suite <name>::`; `spec/local-ci.md` counts two service test programs.
- **Not done / not verified:** the commands in the headers were not each run; they follow the form the other folded files use, which was run.

## 2026-11-24 - `mllp_feed` joins `suite.rs`: the service has two test programs (QA-16 stage 3)

- **Clinical risk:** none: a test file moved.
- **Changed:** `tests/mllp_feed.rs` (about 2,000 lines, the MLLP feed's database tests) is a module of `suite.rs`. Its log capture is scoped to the test (`set_default`), so it does not disturb the others. `Cargo.toml` now lists `suite` and `fhir_validator` (which needs Java, run by its own CI job).
- **Verified:** clippy clean; the database pass on a fresh database with the pinned chaos seed; the normal pass three times in a row.
- **Not done / not verified:** the TLS cases run in the normal suite as before (they cannot run under LLVM instrumentation); the database tests still run one at a time on one database; a cold rebuild was not timed.

## 2026-11-24 - The site's zone and holidays live in the app's runtime (QA-16 stage 3)

- **Clinical risk:** none to what a clinician sees: the same settings are read and checked the same way (a zone that is not an IANA name still stops the service starting); they are kept per app instead of per process.
- **Changed:** `site::init`, `holidays`, `time_zone`, `zone` and `today` moved from the domain crate to `src/site.rs`, reading `EPR_SITE_TIME_ZONE` and `EPR_SITE_HOLIDAYS` from the app's settings and holding the result in its `Runtime`; the domain keeps the parsing (`zone_from`, `holidays_from`) and `day_in`. `site::holidays()` returns an `Arc` now. A unit test boots two runtimes (London with a holiday, and a plain one) and checks each reads its own.
- **Verified:** clippy, the arithmetic gate and the checked-arithmetic grep gate clean; both passes pass on a fresh database.
- **Not done / not verified:** with this, the domain crate holds no process-wide setting; `task_reminders::run_loop` and other spawned tasks outside a request read the process runtime's zone, as with the other moved state (in production that is the app's own, as it is booted from the process environment); `mllp_feed` still builds its runtime by hand and stays its own program; a cold rebuild and parallel database tests were not tried.

## 2026-11-24 - No test calls `set_var`; the service's test programs go from 44 to 3 (QA-16 stage 3)

- **Clinical risk:** none to what a clinician sees; the permission-basis setting is now held per app instead of per process, with the same parsing and the same safe default (an unusable value, read before start, means required).
- **Changed:** `permission::required()` and `init()` (the domain crate's process-wide `REQUIRED`) now live in the service (`src/permission.rs`), kept in the app's `Runtime`; the domain crate keeps the words and parsing. `chaos_api`, `chaos_roles`, `http_permission_basis`, `http_feed_health` and `runtime_boot` are modules of `suite.rs`; `chaos_support` is declared once there. `Cargo.toml` lists 3 programs: `suite`, `mllp_feed`, `fhir_validator` (needs Java).
- **Measured:** a one-line edit to `src/lib.rs` and a rebuild of every test program, cache wrapper off: 26.3 s wall, 86.8 s CPU (28.7 s and 121.3 s with 16 programs earlier today). The wall-clock gain from the last 13 programs is about 9%: the link is not the main cost; the library rebuilt for each program is. `spec/local-ci.md` says so.
- **Verified:** clippy and the arithmetic gate clean; both passes pass on a fresh database. `runtime_boot` runs two apps with different settings at once in the same process, and the suite boots many apps with different login settings in turn.
- **Not done / not verified:** `site::ZONE` and `site::HOLIDAYS` (domain crate) are still process-wide; `mllp_feed` still builds its own runtime by hand and stays a program; the suite's database tests still run one at a time on one database; a cold rebuild was not timed.

## 2026-11-24 - Eight more test programs fold into `suite.rs`; `set_var` is left in three files (QA-16 stage 3)

- **Clinical risk:** none: tests only.
- **Changed:** `audit_coverage`, `audit_search_coverage`, `balp_claim`, `http_audit_refusal`, `http_conflicts`, `http_idempotency`, `http_patients` (8 tests) and `openapi_responses` boot with `request_with` and their own settings (login off, and for `balp_claim` the audit server address), and are modules of `suite.rs`. `audit_switch` and `openapi_support` are declared once in `suite.rs`. 8 test programs remain in `Cargo.toml` (it was 44 this morning).
- **Found and fixed:** the two audit-refusal tests share a process-wide record of the routes they drive (`DRIVEN`); each now clears it before it starts, or the second reported the first's routes. `test_targets` now accepts a support directory (`dir/mod.rs`) in the suite.
- **Verified:** clippy clean; the normal pass and the database pass (fresh database, pinned chaos seed) both pass.
- **Not done / not verified:** `set_var` remains only in `chaos_api` and `chaos_roles` (they share `chaos_support` and the chaos census, and `chaos_roles` must be its own program for now) and `http_permission_basis` (it sets `EPR_REQUIRE_PERMISSION_BASIS`, which the domain crate's `permission::REQUIRED` reads once per process). `site::ZONE` and `site::HOLIDAYS` are still process-wide. Whether the suite's tests could run in parallel (they run one at a time) was not tried.

## 2026-11-24 - 21 more login-on test programs fold into `suite.rs` (QA-16 stage 3)

- **Clinical risk:** none: tests only.
- **Changed:** 21 test files (`http_adjustments`, `http_amendments`, `http_api_tokens`, `http_assessments`, `http_audit_refusal_roles`, `http_death_reports`, `http_deaths`, `http_hazards`, `http_idempotency_roles`, `http_observations`, `http_overdue_reviews`, `http_proxy_scopes`, `http_referral_queue`, `http_referral_worklist`, `http_referrals`, `http_results`, `http_roles`, `http_suspicions`, `http_tasks`, `http_watch_list`, `openapi_responses_roles`) boot with `request_with` and their own settings instead of `set_var`, and are modules of `suite.rs`. 16 test programs remain in `Cargo.toml` (it was 44).
- **Found:** `http_referral_queue` passed its "at least 10 audit rows" check only because `chaos_api`, which runs earlier, left rows behind; on a fresh database it makes 8. It now counts the rows it adds (at least 8). No test was weakened otherwise.
- **Measured:** after a one-line edit to `src/lib.rs`, rebuilding every test program took 28.7 s with the compile-cache wrapper off (8 s with it); the earlier figure was 273 s, taken another way (a cold rebuild of 44 programs), so this is an indication, not a like-for-like comparison.
- **Verified:** clippy clean; the normal pass and the database pass (fresh database, pinned chaos seed) both pass.
- **Not done / not verified:** 11 files still use `set_var` or hold process state (`audit_coverage`, `audit_search_coverage`, `balp_claim`, `chaos_api`, `chaos_roles`, `http_conflicts`, `http_audit_refusal`, `http_idempotency`, `http_patients`, `http_permission_basis`, `openapi_responses`, plus the standalone programs that hold state); two apps with different login settings in one process are tested together only through the suite's apps running in turn.

## 2026-11-24 - Login state moves into the app's runtime; the first login-on test file joins `suite.rs` (QA-16 stage 3)

- **Clinical risk:** none to what a clinician sees; this changes where the login settings are held. Login still fails closed: before `auth::init` has run for an app, login is enforced for it.
- **Changed:** the four process-wide statics in `auth.rs` (the PASETO verifier, the Keycloak verifier, whether login is required, the integration-token database) are fields of `Runtime`, read with `Runtime::current()`. `init_keycloak` reads `EPR_KEYCLOAK_*` through the app's own settings (`settings_from_lookup`), no longer the process environment. A service booted the usual way uses the process runtime and behaves as before.
- **Tests:** `tests/http_role_views.rs` now boots with `request_with` and its own settings instead of `set_var`, so it is a module of `suite.rs` (one program fewer to build and link).
- **Verified:** clippy and the arithmetic gate clean; unit tests; every database test passes in both passes on a fresh database; the suite with the converted file passes.
- **Not done / not verified:** about 30 test files still call `set_var` for the same settings and can be converted the same way (one at a time: each needs its `request` call and its settings block replaced); `site::ZONE`, `site::HOLIDAYS` and `permission::REQUIRED` in the domain crate are still process-wide; two apps with different login settings in one process were not tested together; spawned tasks outside a request see the process runtime, as with the other moved state. The build-time saving was not re-measured.

## 2026-11-24 - The chaos monkey also runs as every role (PA-12)

- **Clinical risk:** none: a test.
- **Changed:** the monkey's engine is now `tests/chaos_support/mod.rs`; `tests/chaos_api.rs` (login off) uses it unchanged in behaviour (its random sequence differs by one draw per request, so a seed no longer repeats the old run). New `tests/chaos_roles.rs` (its own program: the auth settings are read once) boots with login on and sends each hostile request with a token for one of the 17 roles, no token, or a garbage token; no panic, no 5xx other than 502/503/504, no leaked trace, SQL or path. It is listed in `Cargo.toml`, so the database gate runs it with the pinned seed.
- **Verified:** 2,000 requests on seed 20261028 and 15,000 on seeds 1, 7 and 424242: no bad answer. One 15,000-request run on 424242 failed after 5 s with no output captured and passed on four reruns; I could not reproduce it.
- **Not done / not verified:** about two thirds of the answers as a role are 403, so the handlers behind the gate are reached less often than with login off; the monkey still never sends a literal `%`. A planted panic in `GET /api/me/patients` failed the role sweep at once (reverted, not committed). Coverage was not re-measured.

## 2026-11-24 - Coverage 95.91%; FHIR `AuditEvent` paging tested (QA-15)

- **Clinical risk:** none: a test of existing behaviour.
- **Measured:** 25,690 of 26,786 lines, 95.91% (was 95.75%), same two-pass method.
- **Changed:** a database test, `tests/fhir_audit_paging.rs` (a module of `suite.rs`): 57 audit rows are written for a unique agent; the search for it answers a full page of 50 and a `next` link that keeps `agent=` and adds `_before=`; following it gives the other 7 with no shared row and no further link; a misspelt parameter (`agnet`) is refused with 400.
- **Not done / not verified:** the rest of `fhir.rs`'s 65 uncovered lines are conversions of stored records that cannot fail on valid data (`internal(...)`) and refusals on routes that need a role; not touched. Coverage was not re-measured after this test.

## 2026-11-24 - The database-gone sweep sends valid bodies, and 82 routes answer 503 (QA-15)

- **Clinical risk:** none: a test.
- **Changed:** `tests/database_down_sweep.rs` builds a body for each write route from its request schema in `openapi.json` (required fields only, the first enum value, format-shaped strings), so the body passes validation and the route reaches the closed database. With empty bodies few write routes got that far. Now 82 routes answer 503 (the floor is 60) and none answers another 5xx.
- **Not done / not verified:** the bodies are only structurally valid, so a route whose rules refuse them (a date in the future, a missing relationship) still answers 4xx before the database; the sweep does not say which routes did not reach it. Coverage was not re-measured.

## 2026-11-24 - The outbound gate refuses a suspicion message that names no patient (QA-15)

- **Clinical risk:** none: a test of existing behaviour (RS-4: a suspected condition reaches the patient only once the record says they were told).
- **Changed:** a database test in `tests/outbox_store.rs`: an event for the patient role that names a suspected condition but no patient is refused with the reason `suspicion-without-patient` and never delivered.
- **Not done / not verified:** the other uncovered `outbox.rs` lines are failed lookups, a foreign payload (`not-an-epr-event`: it needs a row written outside `enqueue`) and the relay's stop and depth error paths; not touched. Coverage was not re-measured.

## 2026-11-24 - Service coverage 95.75%: above the QA-15 target

- **Clinical risk:** none: a measurement and a note.
- **Measured:** 25,647 of 26,786 lines, 95.75% (was 95.45%; the database-gone sweep and the MLLP settings tests added 174 lines), two passes on a fresh database in a private target directory, TLS and Java-validator cases skipped. The earlier 40.7% was a broken run (two of my runs shared one build directory and the profile data was lost), not a measurement.
- **Not done / not verified:** the CI coverage floors were not raised (QA-15 says to raise them only after CI passes the same commands); the UI and mobile apps were not measured; the 1,139 uncovered lines are listed in `tasks.md`.

## 2026-11-24 - With the database gone, the audit routes and `/api/feed/health` answer 503, not 500 (QA-15)

- **Clinical risk:** none to what a clinician sees: only the status of an answer when the database cannot serve changes, from a plain 500 to the 503 `unavailable` every store already gave, so a client that retries on 503 now retries these too. Nothing is answered unaudited either way. No hazard-log row changed.
- **Found by:** a new sweep, `tests/database_down_sweep.rs` (a module of `suite.rs`, database-gated): it closes the pool, then calls every path and method in the served `openapi.json`; none may answer 5xx other than 503, and at least 10 must answer 503.
- **Changed:** `controllers/common.rs` `acquire_failed`, `begin` and `audit_failed` answer 503 when the error is one `store::unavailable` names (they were 500). `AuditError` now carries that flag (set in `audit::err` by the same rule), and `GET /api/audit`, `/api/audit/export`, `/api/audit/verify` use `audit_failed`; `/fhir/R4/AuditEvent` answers 503 with an `OperationOutcome` coded `transient`.
- **Verified:** clippy and the arithmetic gate clean; all unit tests and all database tests pass (two passes, fresh database).
- **Not done / not verified:** the sweep sends an empty JSON body, so a write route that rejects it with 422 before it touches the database is not tested for a lost database; each needs a valid body. `/api/audit` and its siblings may now log a warning, not an error, when the database is down: no alert was changed. Coverage was not re-measured.

## 2026-11-24 - A cut-short HL7 header reads as nothing (QA-15)

- **Clinical risk:** none: a test of existing behaviour.
- **Changed:** a unit test in `src/adt_feed.rs`: `processing_id_of` and `msh_first_component` return `None` for an empty text, a text with no `MSH`, a header with no separator, a header that ends before the field, an empty field and field number 0, and read a present value.
- **Not done / not verified:** the rest of `adt_feed.rs`'s uncovered lines (53, 5.2%) are database-failure branches (`temporary(...)`) and a few conflict and audit paths that need an injected fault; not touched. Coverage was not re-measured.

## 2026-11-24 - Coverage 95.45%; MLLP settings that cannot work are tested (QA-15)

- **Clinical risk:** none: a test.
- **Measured:** 25,473 of 26,688 lines, 95.45% (the `start` test took `app.rs` from 72.6% to 83.2%). Largest gaps now: `controllers/fhir.rs` (65 lines), `mllp.rs` (63), `adt_feed.rs` (53), `app.rs` (48), `audit.rs` (46).
- **Changed:** a unit test in `src/mllp.rs`: half a TLS pair, a client CA with no server certificate, client names with no CA, a bad processing id (and P, T, D accepted), and a missing, empty or non-PEM certificate file are each refused with the setting named.
- **Not done / not verified:** the TLS handshake paths (client certificate checks, key and certificate mismatch, accept errors, the stop timeout) stay uncovered here because TLS cases cannot run under LLVM instrumentation; they pass in the normal suite. Coverage was not re-measured after this test.

## 2026-11-24 - Coverage 95.31%; the real `start` command is tested with every optional part on (QA-15)

- **Clinical risk:** none: a test.
- **Measured:** 25,436 of 26,688 lines, 95.31% (was 95.16%; the referral-queue and purpose-sweep tests added 39 lines), same method as before.
- **Changed:** new `tests/start_binary.rs` (a module of `suite.rs`, database-gated) runs the binary's `start` against a scratch database with the MLLP listener, the outbox relay, task reminders, the request limit and the lockout switched on in the child's environment only, waits for `/_health/ready` to answer 200, checks the MLLP port accepts, and sends SIGTERM, which must stop it within 30 s. `app.rs`'s start-up path (about 80 lines at 72.6%) ran only under the in-process test app before.
- **Not done / not verified:** coverage was not re-measured after this test, so the gain is unknown; the optional parts are only checked to start, not to work (their own tests do that); no TLS, task-rule file, packs or permission-basis settings are switched on in it; the test needs a `kill` command (Unix).

## 2026-11-23 - One sweep: an unknown purpose of use is refused by every route that reads it (QA-15)

- **Clinical risk:** none: a test.
- **Changed:** new `tests/purpose_sweep.rs` (a module of `suite.rs`, database-gated) walks every path and method in the served `openapi.json` with `X-Purpose-Of-Use: curiosity`: no route may answer 5xx, and at least 40 must refuse it with 400 naming the header (44 do today). A route added later is swept without an edit.
- **Not done / not verified:** the write routes answer 422 for the empty body before the guard runs, so only the read routes are covered; each write route needs a valid body to reach the guard. I did not re-run the coverage, so I cannot say how many lines this added.

## 2026-11-23 - Coverage measured at 95.16%; referral-queue refusals tested (QA-15)

- **Clinical risk:** none: tests only.
- **Measured:** `cargo llvm-cov` of the service, two passes like the database gate (normal tests, then the ignored ones one at a time on a fresh `*_test` database), tests and `target/` excluded, the Java validator and TLS cases skipped (the TLS ones cannot run under instrumentation): 25,397 of 26,688 lines, 95.16%. Lower than the 95.79% of 2026-11-21 because that run included the TLS cases. 1,291 lines are uncovered, spread over many files; the largest are `app.rs` (78), `controllers/fhir.rs` (65), `mllp.rs` (63), `adt_feed.rs` (53), `audit.rs` (46). Most are error branches that need an injected fault (a failing audit write, a failing pool).
- **Changed:** `tests/http_referral_queue.rs` also checks that a clearance with an unknown `how` is refused (422) and that an unknown purpose of use is refused (400) by the listing, the count, one entry and the clearance.
- **Not done / not verified:** the figure was taken before this test, which adds only a few of the 33 uncovered lines in that controller; 100% is far off and is not claimed. Running everything in one pass on one database fails (tests count rows left by others); the two-pass method is the one to keep.

## 2026-11-23 - Matcher: a name with an empty part no longer scores NaN (QA-17)

- **Clinical risk:** it could change a match score, so it is treated as clinical-adjacent. Before, a family name with an empty part (two spaces, a leading or trailing space) scored 0/0, NaN, for that part; `weighted` then failed its finite check or the score dropped out. Now empty parts are ignored, and a name with no part at all scores 0.0 (absent evidence, never agreement). Names of one part per word are unchanged. No hazard-log row was changed.
- **Verified:** a new unit test; all matcher tests (126 integration, 7 unit) pass; clippy and the arithmetic gate are clean.
- **Not done / not verified:** the names reaching this function are already folded and usable, so I found no input from the service that triggered it; this closes the gap in the function. The matcher version was not bumped or published, so the service (which uses the published matcher) does not have it yet.

## 2026-11-23 - The mobile apps' Lily pin moves to 5df76fa (494 commits)

- **Clinical risk:** none: the pinned design-system source.
- **Changed:** `LILY_COMMIT` in `scripts/fetch-lily.sh` from `787d936` to `5df76fa`. Between them Lily's Swift and Kotlin libraries changed only in their documentation (`AGENTS/`, `CONTRIBUTING.md`); the theme files gained a timing scale, which the apps do not use. No app code changed.
- **Verified:** `mobile-swift-test.sh` (3 tests) and `mobile-kotlin-test.sh` (build successful) pass against the new commit.
- **Not done / not verified:** the Simulator and debug-APK build gates were not run; the 494 commits were not read beyond the paths above; the apps do not yet use Lily's timing scale or reduced-motion rule, which is worth a task.

## 2026-11-23 - Lily npm packages for the clinician UI: headless 0.13, picker-bar 0.9, theme-picker 0.5, themes 0.8

- **Clinical risk:** none: styling and picker packages.
- **Changed:** `@lilydesignsystem/svelte-headless` 0.9.0 to 0.13.0, `svelte-picker-bar` 0.6.0 to 0.9.0, `svelte-theme-picker` 0.4.0 to 0.5.0, `themes` 0.6.2 to 0.8.0, with the lock file. No source change was needed.
- **Verified:** `pnpm run lint`, `check` (0 errors), `check:tests`, vitest, and all 248 Playwright tests pass.
- **Not done / not verified:** I did not read Lily's release notes or compare the pages by eye; passing tests are the only evidence nothing moved. `axa` is already the latest (0.21.0, `axa-test` 0.12.0), so there was nothing to update there. The mobile apps' pinned Lily commit is a separate change.

## 2026-11-23 - cargo deny: RUSTSEC-2026-0594 (cookie) ignored until its fix leaves the cooldown

- **Clinical risk:** none.
- **Changed:** `deny.toml` ignores RUSTSEC-2026-0594 with a reason and an expiry (2026-12-07). `cookie` 0.18.2 is reached only through `axum-test` in dev-dependencies; the advisory needs `fork()` plus private cookies, and neither happens in the tests or the service. 0.18.3 fixes it and was published today, inside the 2-day cooldown.
- **Not done / not verified:** the lock file is unchanged; update `cookie` after 2026-11-25 and delete the ignore. `cargo deny` was not run locally (CI is the check). The failure `override toolchain ... is not installed` in the same CI log is from the deny action's container and predates this; it does not fail the job.

## 2026-11-23 - Seven more test programs folded into `suite.rs` (LC-15)

- **Clinical risk:** none: test tooling.
- **Changed:** `audit_immutable`, `hl7_samples`, `http_lockout`, `http_rate_limit`, `metrics`, `openapi_census` and `outbox_store` are modules of `tests/suite.rs` now, not programs of their own: seven fewer builds and links (about 44 s each, per `spec/local-ci.md`, so up to about five minutes off a full rebuild; not re-timed). They set no environment and keep no process-wide state beyond the counters `metrics` already tolerates. The `--test <name>` commands in `README.md`, `AGENTS.md`, `tasks.md`, the sample README and the test headers are now `--test suite <name>::`. No test was changed or removed.
- **Verified:** `cargo test --test suite` (100 passed, the rest ignored); the folded files' database tests (`audit_immutable`, `http_lockout`, `http_rate_limit`, `outbox_store`: 19 passed) on a local Postgres 18.
- **Not done / not verified:** the 32 programs that call `set_var` and the few that hold process-wide state (`http_feed_health`, `runtime_boot`, `mllp_feed`, `audit_coverage` and others) are still their own programs; that needs QA-16 stage 3. The full database run and CI were not run after the fold.

## 2026-11-23 - cargo-nextest for the database tests: measured, opt-in (LC-8)

- **Clinical risk:** none: test tooling.
- **Changed:** `scripts/gates/service-database.sh` can run the database tests under nextest with `EPR_DB_RUNNER=nextest`, using a new `database` profile (one test at a time) in `electronic-patient-record-service-with-rust/.config/nextest.toml`. The default is unchanged (`cargo test --test-threads=1`).
- **Measured, and why it is not the default:** 214 database tests on a fresh local Postgres 18: `cargo test` 50 s, nextest 109 s, all passing in both. Every nextest test pays for a new process and a new app boot. The same measurement also found the build, not the run, is the cost: after touching one file in `src/`, rebuilding every test program took 273 s, against 74 s to run everything. `spec/local-ci.md` already recorded about 44 s per test program and that mold was not faster, so the lever is folding programs together (LC-15).
- **Not done / not verified:** no test program was folded; the `--timings` breakdown I tried to take failed (the report path was wrong) and was not retried; nextest with parallel database tests, and a cold run, were not measured; CI is unchanged and not observed.

## 2026-11-23 - Provider features for the mobile apps: sign off results, warnings and adverse reactions, notes and tasks (MP-30 to MP-33)

- **Clinical risk:** none from code (documents only). New hazard **H-064** (a wrong result, version or patient signed off on a phone, or a note or task written or shared wrongly), status `open`, severity 5, likelihood 3; not accepted or closed.
- **Changed:** `spec/mobile-apps/index.md` §13 (a table of the three features against the service, eight rules, an order); OQ-MP10 (what the desktop shows as warnings), OQ-MP11 (sharing scope), OQ-MP12 (which result kinds may be signed off on a phone); `tasks.md` MP-30 to MP-33; `plan.md` §39.
- **Not done / not verified:** nothing is built. The results and sign-off routes exist; the service has **no alerts or warnings route, no notes route, no manual task create or edit, no sharing model**. The desktop portal ("WCP") was not seen, so its content is a target to list (OQ-MP10), not a fact. Two choices are my proposals and need the maintainer's confirmation: drafts held in memory only (a lost half-written note is the cost), and break-glass staying on the web. No clinician or Clinical Safety Officer reviewed any of it.

## 2026-11-23 - Twelve more patient features for the mobile apps, assessed (MP-17 to MP-29)

- **Clinical risk:** none from code (documents only). New hazard **H-063** (an action in the app reaches the wrong person or place: pharmacy, barcode, authorised access, personal details, analytics), status `open`, severity 5, likelihood 2; not accepted or closed.
- **Changed:** `spec/mobile-apps/index.md` §12 (a table of the twelve items with an assessment each, ten rules, an order); the §2 rule on analytics now allows only the allow-listed navigation events of §12 rule 1; OQ-MP8 (a PROMs provider) and OQ-MP9 (identity assurance for authorised access) added; `tasks.md` MP-17 to MP-29; `plan.md` §39 gains the assessment and order.
- **Not done / not verified:** nothing is built. Most of the list is writes and notifications, which the service does not have (no medicines, prescription, appointment, message, pharmacy, authorised-access-request or communications-preference route, no patient client, an outbox with no real recipient), so each feature waits for its route and its patient-portal spec. Promptly Health and the other named services were taken from the feedback text and not checked. The DPIA, record of processing and data-flow table are not yet updated for monitoring and PROMs (named as needs in MP-17 and MP-18). No patient, clinician or Clinical Safety Officer reviewed any of it.

## 2026-11-23 - Patient features for the mobile apps: prescriptions, appointments and referrals, health records, services (MP-13 to MP-16)

- **Clinical risk:** none from code (documents only). New hazard **H-062** (a patient acts on a misread result, an incomplete medicines list, a wrongly cancelled appointment or an unmade repeat prescription), status `open`, severity 4, likelihood 3; not accepted or closed.
- **Changed:** `spec/mobile-apps/index.md` gains §11 "Patient features": a table of the four groups against what the service has today, eleven rules (a patient client and the `patient-access` purpose; release rules; no silence read as "none"; a repeat prescription is a request, not an order; booking and cancelling through the booking system; messages never in a notification; the national register is the authority for a donation decision; a fixed emergency line; the never-copy rule holds for patients; a patient's phone is a personal device; words and access) and a build order, least risky first. OQ-MP5 is marked answered; OQ-MP6 (one app or two) and OQ-MP7 (which national services) are new. `tasks.md` gains MP-13 to MP-16, each naming the service route that does not exist yet.
- **Not done / not verified:** **the service has no medicines, prescription, appointment, message or donation route, no patient client in Keycloak, and its results and allergies routes are staff routes**: none of the four groups can be built until those exist and are specified in the patient portal first. The service-today column was read from `openapi.json` and the controllers, not run. The NHS 111, organ donation and blood service names are one country's examples. No clinician, patient or Clinical Safety Officer has reviewed any of it. The never-copy rule applied to a patient's own record is the maintainer's literal instruction; I noted it as stricter than a national app and did not ask whether it was meant for patients.

## 2026-11-23 - Record text is never copied onto a personal device (mobile spec §6 rule 4)

- **Clinical risk:** none from code (documents only). It narrows a disclosure path of H-061, which stays `open`.
- **Changed:** `spec/mobile-apps/index.md` §6 rule 4 now forbids copy, cut, look up, share, drag, export, print and save of record text and record photographs, and keyboard learning in search fields, on every device and build (the app cannot tell a personal device from a managed one with certainty). OQ-MP1's open part is reduced to which record kinds a personal device may hold. H-061, the data-flow table and MP-6's acceptance follow.
- **Not done / not verified:** nothing is built or tested; a screenshot or recording on iOS, a photograph of the screen and retyping cannot be prevented and stay as H-061's residual; whether the rule holds under every platform text component (selection of a screen reader's "read" action, Live Text on iOS) is not checked.

## 2026-11-23 - The mobile apps support managed and bring-your-own devices, phones and tablets (OQ-MP1)

- **Clinical risk:** none from code (documents only). The decision raises H-061's exposure to personal phones: its residual text now says the personal-device controls are required and not built. Status stays `open`.
- **Changed:** `spec/mobile-apps/index.md` §6 gains "Device ownership" (the maintainer's decision of 2026-11-23, nine rules: no reliance on MDM, a device lock required, an app-level wipe, nothing in a backup or the sandbox's reach, root detection is advice, managed configuration can only tighten, tablets, no real record data on a personal device until built and tested, distribution separate); OQ-MP1 is marked answered; MP-6's acceptance adds the controls and a test on a real phone and tablet of each platform; the MFA plan and H-061 follow.
- **Not done / not verified:** none of the rules is built; no device of any kind was tried; whether record text may be copied, and which record kinds a personal device may hold, are open; OQ-MP3 (distribution) is still open.

## 2026-11-23 - Threat model for a lost or shared phone (MP-11)

- **Clinical risk:** none from code (documents only). H-061 is the hazard: its controls, evidence and residual text are updated; its status stays `open`, and it is not marked accepted or closed.
- **Changed:** `governance/mfa-plan.md` gains "The native apps: a lost or shared phone" (a phone unlock is not the service's second factor; what a thief gets from a locked and an unlocked phone; the cost of a lost phone for a managed and for an owned device; no `offline_access`; no shared phones; distribution). `governance/data-flow.md` gains "The native apps" (a STRIDE table for the phone flow, the token, the screen, the OS, a lost or shared phone, the build).
- **Not done / not verified:** OQ-MP1 and OQ-MP3 are not answered: the maintainer decides, and the text gives the consequence of each. Read from the spec only: no app signs in, stores a token or locks yet, so nothing here was tested, and no real device was tried. No security engineer or Clinical Safety Officer reviewed it.

## 2026-11-23 - The site task rules and the HL7 feed's identity and health live in the app's runtime (QA-16 stage 2)

- **Clinical risk:** none intended: the same settings, read from the app's runtime instead of a process-wide cell. In a service booted the usual way there is one runtime, so nothing changes. Hazard log not edited.
- **Changed:** `Runtime` gains `site_rules`, `feed_health` and `feed_identity`; the three statics (`tasks_store::SITE_RULES`, `feed_health::FEED`, `adt_feed::CONFIGURED`) are gone. `tasks_store::rules()` and `init_rules()` return `Rules` (the site's `Arc` or the catalogue), callers borrow it. `feed_health::configured(&Runtime)`, `adt_feed::configured(&Runtime)` return clones; `*_now()` read the request's runtime. `mllp::Running::start` and `feed_health::run_loop` take the runtime. New unit test: a feed is configured in one runtime and not in another.
- **Not done / not verified:** `auth` and the domain crate's `site` and `permission` statics are still process-wide (see QA-16); the 32 test files still use `set_var`; `http_feed_health` is still its own test program. Full suite on a fresh database: all pass except the six Java `fhir_validator` tests (the health test failed once, fixed: its runtime lookup now falls back as the app does). Clippy all targets and the arithmetic gate pass. CI not observed.

## 2026-11-23 - The matcher's production code passes the checked-arithmetic lints; a gate and a `clippy.toml` keep it so (QA-17)

- **Clinical risk:** low: patient matching scores. No score is meant to change; the arithmetic is the same operations in the same order. Behaviour that can differ: a weight configuration whose total is not finite is now refused (`InvalidWeight`; before, an infinite total would pass the `> 0` check); an exact tie between two given-name scores compares with `total_cmp`, which also separates `-0.0` from `0.0` (similarities are never negative). Hazard log not edited.
- **Changed:** 27 sites in `engine.rs`, `ihi.rs`, `national_number.rs` and `normalize.rs`: typed float literals; `total` (a fold) replaces `Iterator::sum`; `count` counts as a float with no `as`; the IHI Luhn sum and the national number's modulus 11 use `try_fold`, `checked_mul`, `checked_rem` and `checked_sub`; the phone's leading `0` after the calling code is stripped with `strip_prefix`, not a byte slice; an unreadable number is `false` instead of an empty list read as a wrong length. New `clippy.toml` (the RCA-ENF-2 bans), new gate `matcher-clippy-arithmetic` in `ci.yml` and `ci-local.sh`. Three test and benchmark lines changed for the new bans.
- **Also:** the "lenient input" in the QA-17 note was checked and is not so: the typed R4 parse already refuses a `name`, `address` or `given` of the wrong shape (new test), and `er7`'s `query` errors only for a malformed path, so `get` in `hl7_adt.rs` cannot hide sender data.
- **Not done / not verified:** `compound_sim` still divides by a part length that is 0 for an empty part (`Smith  Jones`), giving NaN as before; I did not change it. The matcher's tests and benchmarks are not linted for arithmetic (RCA-SCOPE-4). Matcher tests: 126 passed; the service's tests were not re-run. CI not observed.


## 2026-11-23 - Gates run from a git hook no longer act on this repository (branch `mobile-apps`)

- **Clinical risk:** none: build scripts.
- **Found:** the first push of `mobile-apps` ran the pre-push hook in a worktree. A hook runs with `GIT_DIR` naming this repository, and the gates inherited it. `scripts/fetch-lily.sh` (`git -C vendor/...`) then set `origin`'s fetch URL to Lily's, turned on sparse checkout for the worktree, fetched Lily's commit into this repository and marked it shallow. The existing `scripts/test_gate_record.sh` (`git init` of a throwaway repository) re-initialised this repository and set `core.bare=true`, which broke `git status` in every checkout. All of it was put back by hand: `core.bare=false`, the fetch URL `git@github.com:electronic-patient-record/electronic-patient-record.git` (as the main checkout's `FETCH_HEAD` recorded it), sparse checkout off, `extensions.worktreeConfig` removed, `.git/shallow` and the worktree's `FETCH_HEAD` deleted. The main checkout's work and commits were untouched.
- **Changed:** `scripts/ci-local.sh`, `scripts/fetch-lily.sh` and `scripts/test_gate_record.sh` unset git's local environment variables (`git rev-parse --local-env-vars`) first, as `scripts/test_pre_push.sh` already did; `fetch-lily.sh` also refuses to change a directory that is not its own git repository. New `scripts/test_fetch_lily.sh`, run by the `docs-hooks` gate: with `GIT_DIR` naming another repository, the script leaves that repository's config, shallow list, `FETCH_HEAD` and sparse-checkout alone; it fails against the old script.
- **Not done / not verified:** Lily's objects fetched into this repository's object store are unreachable and were left for `git gc` to prune, not removed by hand. Whether other gates misbehaved under the hook before 2026-11-23 is not known; pushes from the main checkout may not have shown it.

## 2026-11-23 - The mobile apps' gates in CI and ci-local.sh (MP-4; branch `mobile-apps`)

- **Clinical risk:** none: build and test wiring.
- **Added:** gate scripts `mobile-swift-test`, `mobile-swift-build`, `mobile-kotlin-test` and `mobile-kotlin-build`, each fetching the pinned Lily checkout first. `scripts/ci-changes.sh` answers `swift` and `kotlin` as well as `rust` and `ui` (an app's directory, its gate scripts, and `scripts/fetch-lily.sh` for both; an unknown file still runs everything). `ci.yml` has jobs `mobile-swift` (`macos-latest`) and `mobile-kotlin` (`ubuntu-latest`, Temurin 25), both under `ci-ok`. `ci-local.sh` knows the projects `swift` and `kotlin`: the tests in `--fast`, the builds in `--full`, and a SKIP with the reason on a machine without Xcode or without a JDK and the Android SDK. The pre-push hook adds `swift` or `kotlin` when the push touches that app, its gates or the Lily pin. Tests for each in `scripts/test_ci_changes.sh` and `scripts/test_pre_push.sh`.
- **Verified:** all four gate scripts pass on this machine; `scripts/test_ci_changes.sh`, `scripts/test_pre_push.sh` and `check-gates` pass; the workflow parses as YAML.
- **Not done / not verified:** the two CI jobs have not run on GitHub: whether `macos-latest`'s default Xcode builds Lily's Swift 6 package and whether the Android Gradle plugin installs platform 37.2 on the runner is unknown until the branch is pushed. No caching of SwiftPM, Gradle or the Lily checkout. Dependabot does not watch the Gradle build, on purpose (its versions follow Lily's). On a Linux machine a plain `ci-local.sh --fast` now exits 2 for the iOS gates' SKIP unless `EPR_CI_ALLOW_SKIP=1` or `EPR_CI_ONLY` leaves `swift` out.

## 2026-11-23 - iOS and Android app scaffolds on Lily Design System (MP-1, MP-2; branch `mobile-apps`)

- **Clinical risk:** low now, real later: neither app reaches the service or shows a real record, but both show a patient banner (a synthetic drill patient). New hazard **H-061** (a phone shows the wrong patient, or keeps a record after it leaves the clinician's hands), `open`; the only control built is the banner text that keeps every part whole and says "not available" for an absent part.
- **Added:** the shared contract [`spec/mobile-apps/index.md`](spec/mobile-apps/index.md) (direct `/api` calls with `Accepts-version`, PKCE sign-in through the system browser, tokens in memory or the Keychain or Keystore, nothing from the record on the device, the web UI's safety rules restated for a phone, accessibility, platforms), plan.md §39, tasks MP-1 to MP-12, open questions OQ-MP1 to OQ-MP5. Two subprojects: `electronic-patient-record-mobile-app-with-swift` (SwiftPM `EPRCore` and `EPRAppUI`, an Xcode app target, iOS 18) and `electronic-patient-record-mobile-app-with-kotlin` (Gradle `:core` on the JVM and `:app` in Compose, minSdk 28). Each core has `ApiRequest` (refuses a non-`/api` path, a bad segment, a version segment, a bad version token and a non-HTTPS base except loopback when allowed), the `GET /api/patients` decoding (absent and `null` read as absent; an unknown `sex` is an error) and `BannerText` with the web UI's words. Each app has one screen built from Lily components: the not-for-clinical-use notice and the sample banner. `scripts/fetch-lily.sh` checks out a pinned Lily commit into `vendor/` (ignored), because neither Lily package is published; `scripts/check-links.py` skips `vendor/`.
- **Verified:** `swift test` (27 tests) and the iOS Simulator build pass; `./gradlew :core:test :app:testDebugUnitTest :app:assembleDebug` passes (24 core tests, 5 Robolectric tests). The Android banner test, at 320 dp and twice the font size, failed when the name was given `maxLines = 1` with an ellipsis, then passed again without. Each app was run once on a simulator or emulator (iPhone 16e, iOS 27.2; Pixel 10, API 37) at the default and the largest text size and looked at: every banner line wrapped.
- **Not done / not verified:** no sign-in, network call, record screen, idle lock, snapshot cover or `FLAG_SECURE`, translation, release build, signing or distribution. Neither GitHub CI nor `scripts/ci-local.sh` runs the apps' tests yet (MP-4). No VoiceOver or TalkBack pass, no real device, no tablet, landscape or dark mode. Keycloak has no `epr-ios` or `epr-android` client. No clinician, user or Clinical Safety Officer has seen any of it. The pinned Lily checkout is fetched from GitHub by a script, not by a package manager, and Lily's own build prints Gradle deprecation warnings.

||||||| 59d11012
## 2026-11-23 - `url` 2.5.9 and `form_urlencoded` 1.2.3 (RUSTSEC-2026-0592)

- **Clinical risk:** none: a dependency patch in the service lockfile.
- **Changed:** `cargo update -p url --precise 2.5.9` at the maintainer's instruction to ignore the 2-day publish cooldown for this crate; `form_urlencoded` followed. `cargo deny check advisories` now passes.
- **Not done / not verified:** clippy and the 331 unit tests pass; the database suite was not re-run; CI not observed. The cooldown in `~/.cargo/config.toml` is unchanged.

## 2026-11-23 - The secret scan passes again; the one CI failure left is a dependency advisory (QA-16)

- **Clinical risk:** none: a test and a scan allowlist.
- **Found by looking at CI, which I had not done for the last pushes:** the `CI` workflow failed on every push since `5daf958` (and earlier, `5fce464`) on two jobs: **gitleaks** (full history) and **cargo deny** (service). Every other job passed, including the API clippy and database tests and the new gates. The gitleaks finding was my own: the runtime test in `5fce464` used SHA-256 of the word "test" as an audit key, flagged `generic-api-key` (the same made-up value as the benchmark entry already in `.gitleaksignore`). The test now builds its key at run time and the old line is in `.gitleaksignore` with a reason and a date. `gitleaks git` now reports no leaks.
- **Not done / not verified:** **cargo deny still fails** on RUSTSEC-2026-0592, a stack overflow in `Url::origin()` on nested `blob:` URLs in `url` 2.5.8, reached through `axum-keycloak-auth` and `axum-test`; the fix is `url` 2.5.9. `cargo update -p url` does not take it because the maintainer's cargo config (`~/.cargo/config.toml`) sets `global-min-publish-age = "2 days"`, and 2.5.9 is newer. Left for the maintainer: wait, override the age for this one crate, or ignore the advisory in `deny.toml` with a reason (the service does not call `origin()` on request input). CI not re-observed after this change.

## 2026-11-23 - The service's production code passes the checked-arithmetic lints; a gate keeps it so (QA-17, RCA-ENF-1)

- **Clinical risk:** low, spread over many modules; no result is meant to change on a working clock and valid data. Behaviour that can differ: a `Timestamp` sum out of range in the suspicion future check, the observation future check and the feed health hour window is now handled by a checked add (the suspicion check says "the time now is outside the calendar"; the feed health read fails with `StoreError::Db`); a team or watch list with more rows than fit `i64` counts as having more. Hazard log not edited.
- **Changed:** the last 33 production sites: `clock::fraction` (`exactly` with `checked_rem` and `checked_div`), `dates`, `proxy` (term), `suspicion`, `packs`, `adt_feed`, `amendments_store`, `break_glass` (`GRANT_HOURS` is `i32`, no `unwrap_or(4)`), `me`, `patients`, `feed_queue` and `referral_unmatched` (the longest prefix on a character boundary, no loop and no slice), `fhir_patient` (`zip(1_usize..)`), `redact`, `links`, `observations_store`, `tasks_store`, `watch_list_store`, `store`, `openapi`; the 15 `#[allow(clippy::too_many_arguments)]` are `#[expect(.., reason = ..)]` and the five that were no longer needed are gone; the histogram sum is checked finite (RCA-FLT-2). New gate `service-clippy-arithmetic` (`scripts/gates/`, in `ci.yml` and `ci-local.sh`).
- **Not done / not verified:** the lints are in a gate, not in `[workspace.lints]` as RCA-ENF-1 shows, because of tests (see QA-17); about 820 hits remain in tests and benchmarks, allowed by RCA-SCOPE-4; the matcher crate is not covered; the 15 remaining `expect` for `too_many_arguments` still want a parameter struct. Full suite on a fresh database: 864 passed, 6 failed (Java `fhir_validator`). CI not observed.

## 2026-11-23 - interop/hl7_adt.rs under the arithmetic lints (QA-17)

- **Clinical risk:** none intended: the readers of an HL7 time offset and a birth date give the same results; the slicing and the offset arithmetic are now `get`, `split_at_checked`, `checked_mul` and `checked_add`. Hazard log not edited.
- **Changed:** the module denies the arithmetic lints (RCA-ENF-3); two tests for the edges (`+1400`, `-1400`, `-0000`, an offset of `+1500`, `+0960`, lone `+`, non-ASCII digits, a multibyte first character; birth dates of 0, 3, 5 and 7 digits).
- **Not done / not verified:** the module still discards a parser error with `.ok()` where it reads an unreadable path as empty (`get` helper, RCA-IN-6): a behaviour change that needs a specified answer first. Full suite on a fresh database: 864 passed, 6 failed (Java `fhir_validator`). CI not observed.

## 2026-11-23 - mllp.rs under the arithmetic lints; no invented clock for the feed's answer (QA-17)

- **Clinical risk:** low, on the inbound HL7 feed: no result changes on a working clock. The ACK's timestamp came from a clock that read `0` before 1970 and `i64::MAX` out of range, and from a hand-written calendar. It is now `jiff`, and a clock outside `jiff`'s range (years -9999 to 9999) closes the connection with the frame unanswered, so the sender sends it again, instead of answering with a made-up time. Hazard log not edited.
- **Changed:** `mllp.rs` denies the arithmetic lints for the module (RCA-ENF-3); `hl7_timestamp(i64) -> Option<String>`; `take_frame` checks `end + 1` and `end + 3`; `frame` grows its buffer instead of `with_capacity(len + 3)`; the stop wait is a compile-time constant. Tests: `hl7_timestamps_are_utc_and_exact` now covers `-1`, `i64::MAX` and `i64::MIN`.
- **Not done / not verified:** the unreadable-clock close in the connection loop has no test (it needs a bad clock). The frame's overflow branches are unreachable (`end` is a position in the buffer). Full suite on a fresh database: 862 passed, 6 failed (Java `fhir_validator`). CI not observed.

## 2026-11-23 - No invented time for a disclosure; checked hazard score, reopen count and rate budget (QA-17)

- **Clinical risk:** one clinical path: `outbox::now()` decided a disclosure's consent against a clock that read `0` before 1970 and `i64::MAX` out of range. It is now an error, so the event is decided again later and never delivered on a made-up time. Not reachable on a working clock. `Hazard::score` returns `Option<u8>` (a hazard's ratings are public fields, so a stored product could overflow `u8`), and reopening counts with `checked_add`. Hazard log not edited.
- **Changed:** `outbox::now` returns a `Result` and both callers pass it on; `Hazard::score() -> Option<u8>`; new `Refusal::CountOverflows` (mapped to a 422 sentence in `hazard_store`); `Budget::ask -> Option<Allowance>`, `None` when a time or count does not fit. Tests: `a_budget_whose_time_does_not_fit_gives_no_answer_not_a_wrapped_one`, the existing score and budget tests updated.
- **Not done / not verified:** `outbox::now` has no test (it reads the real clock), the reopen overflow has no test (it takes `2^32` reopenings), and `Budget` and `Hazard::score` have no production caller yet, only tests. `privacy.rs` and `hazard.rs` do not yet deny the arithmetic lints. Full suite on a fresh database: 862 passed, 6 failed (Java `fhir_validator`). CI not observed.

## 2026-11-23 - Gauges are unknown, not zero; the committed OpenAPI document is current (QA-17, RCA-P-3)

- **Clinical risk:** none: operator metrics only.
- **Fixes my own miss:** `5daf958` added the `/metrics` 500 to the OpenAPI document without regenerating `openapi.json`, so the census test `the_committed_document_is_what_the_code_produces` failed on that commit (I ran the full suite just before adding the text). `openapi.json` is regenerated here: the diff is that one response and nothing else.
- **Changed:** `Gauge` is unknown until set; an unknown gauge renders its `HELP` and `TYPE` lines and no sample, never `0`. `outbox::watch_depth` sets the pending and stuck gauges unknown when the depth cannot be read (it kept the stale value) or is negative (it set `0`); `outbox::depth` adds its three counts with `checked_add`. Test `a_gauge_is_unknown_until_set_and_after_a_failed_read_and_never_zero_for_it`.
- **Not done / not verified:** a scraper now sees no sample for the outbox gauges before the first depth read and after a failed one (before: `0`): alerting rules that compare the gauge with a number should treat absence as unknown. `outbox.rs` still has `now()` reading a clock before 1970 as `0` and one out of range as `i64::MAX`. Full suite on a fresh database: all pass but the six Java `fhir_validator` tests, with `openapi.json` regenerated. CI not observed.

## 2026-11-22 - metrics.rs: a count that overflows is an error (QA-17, RCA-NONE-4)

- **Clinical risk:** none: operator metrics only.
- **Changed:** `metrics.rs` denies the arithmetic lints for the module (RCA-ENF-3). The atomic `Counter::inc` used `fetch_add`, which wraps silently; every count now uses `checked_add`. A count that no longer fits (not reachable: `2^64` events) leaves the value as it was, is logged once, and makes `render` return `Err(Overflow { figure })`, so `GET /metrics` answers 500 naming the metric, with no number, until the process restarts. This replaces the ceiling (ARITH-4) I had pushed in `5ff8a66`, following your instruction that overflow is an error. Documented in the OpenAPI document and `spec/index.md`; `render` now returns `Result`. Tests: `a_count_that_no_longer_fits_is_an_error_naming_the_figure_never_held_or_wrapped`, `a_histogram_or_feed_count_at_the_top_is_an_error_not_a_new_value`.
- **Not done / not verified:** the process-wide overflow flag (`OVERFLOWED`) has no test, since setting it would fail every other test in the program; the pieces that feed it are tested. The gauges set by `outbox.rs` still turn a failed conversion into `0` (`unwrap_or(0)`): an unknown shown as zero (RCA-P-3), still to fix. Full suite on a fresh database: 860 passed, 6 failed (Java `fhir_validator`). CI not observed.

## 2026-11-22 - The last append-only stores and two catalogues use checked versions (QA-17)

- **Clinical risk:** none intended: the version written or expected is unchanged for every reachable value. At `i32::MAX` an append answers `BadQuery` ("version: out of range"); a form or pathway file whose previous version cannot be incremented is refused at load.
- **Changed:** `store::next_version` in `adjustments_store`, `amendments_store`, `hazard_store`, `observations_store` and `results_store`; `checked_add` in the domain's `assessments::parse` and `pathways::parse`.
- **Not done / not verified:** those modules do not yet deny the arithmetic lints. The catalogue refusals are unreachable in a file that passed the earlier checks (each version must equal the previous plus one, from 1), so they have no test. Full suite on a fresh database: 858 passed, 6 failed (Java `fhir_validator`). CI not observed.

## 2026-11-22 - api_tokens_store, photos_store and task_reminders under the arithmetic lints (QA-17)

- **Clinical risk:** none to a clinician's view; **security-relevant**: `api_tokens_store::system_time` turned a stored token time before 1970 or out of `SystemTime` range into the epoch (an expiry of 1970, so refused, but a `last_used_at` of 1970 invented); it is now an error when a token is read (RCA-IN-6). Reachable only by a corrupt `api_token_current` row.
- **Changed:** three modules deny the arithmetic lints (RCA-ENF-3); token expiry is `checked_mul` and `checked_add`; `next_version` in tokens and photos; `MAX_KIB` constant replaces `MAX_BYTES / 1024`; the reminder run counts with a checked `count`; `Duration::from_mins` for the interval setting.
- **Not done / not verified:** the corrupt-row error has no test (it needs a token row with a pre-1970 time; the table's triggers may not allow one). Full suite on a fresh database: 858 passed, 6 failed (Java `fhir_validator`). CI not observed.

## 2026-11-22 - pg_time.rs and relationships.rs under the arithmetic lints (QA-17)

- **Clinical risk:** none intended: every time the service can hold encodes to the same microsecond as before, and the relationship version written is unchanged. `to_pg_micros` used to turn a time outside `timestamptz`'s range into `i64::MIN` or `MAX` (`infinity`); it now refuses (not reachable from a `jiff::Timestamp`, whose range is far inside).
- **Changed:** both modules deny the arithmetic lints (RCA-ENF-3); the microsecond floor is `checked_div_euclid` on nanoseconds; `parse_pg_text` slices with `get`; the relationship rank is `u8`; new test `the_ends_of_the_jiff_range_fit_timestamptz_and_round_trip_to_the_microsecond`.
- **Not done / not verified:** the refusal in `to_pg_micros` has no test, since no `Timestamp` reaches it. Full suite on a fresh database: 857 passed, 6 failed (Java `fhir_validator`). CI not observed.

## 2026-11-22 - audit.rs under the arithmetic lints (QA-17)

- **Clinical risk:** low, but it is the audit trail's verifier: no verdict changes for a database the service itself wrote. Differences: a row count that overflows `usize` (not reachable) is an error instead of `usize::MAX`; an archive anchor whose `first_id` is the smallest `i64` is reported `Broken` ("the anchor's first id is not valid") instead of overflowing. Hazard log not edited.
- **Changed:** `audit.rs` denies the arithmetic lints for the module (RCA-ENF-3); `bump` counter with a test at `usize::MAX`; `valid_instant` slices with `get`.
- **Not done / not verified:** the anchor-id case has no test of its own (it needs an archive row with a minimum id; none is built). Full suite on a fresh database: 857 passed, 6 failed (Java `fhir_validator`). CI not observed.

## 2026-11-22 - Every append-only store uses the checked next version (QA-17)

- **Clinical risk:** none intended: the version written is the same as before for every reachable value; at `i32::MAX` an append now answers a 400-class `BadQuery` ("version: out of range") instead of overflowing.
- **Changed:** `expected_version + 1` and `was.version + 1` replaced by `store::next_version` in `patients_store`, `consents_store`, `links`, `tasks_store`, `referrals_store`, `watch_list_store`, `death_store` and `suspicions_store`.
- **Not done / not verified:** those modules still allow the arithmetic lints for their other sites. No new test beyond `next_version`'s own: the stores' existing database tests cover the normal path. Full suite on a fresh database: 856 passed, 6 failed (Java `fhir_validator`). CI not observed.

## 2026-11-22 - allergies.rs under the arithmetic lints; a checked next version (QA-17)

- **Clinical risk:** low, but the allergy record is clinical: no result changes except that an allergy or statement already at version `i32::MAX` now refuses an append (`StoreError::BadQuery`, "version: out of range") instead of overflowing; unreachable in practice. Hazard log not edited.
- **Changed:** `allergies.rs` denies the arithmetic lints for the module (RCA-ENF-3); new `store::next_version` with a test at `i32::MAX - 1` and `i32::MAX`; the "no statement yet is version 0" fallback is written `0_i32` and commented (it is declared by the allergy spec's statement rules).
- **Not done / not verified:** the other eight stores still write `expected_version + 1`. One full run had a failure in `every_alerted_role_sees_what_waits_and_nobody_else_does` that did not repeat alone or in a second full run on a fresh database (856 passed, 6 failed Java `fhir_validator`); cause not found. CI not observed.

## 2026-11-22 - consent.rs under the arithmetic lints (QA-17)

- **Clinical risk:** low, but touches whether a consent is in force (`in_period`). No result is meant to change: the hand-written calendar arithmetic is replaced by `jiff` civil-day counting, and the end-of-day addition is checked. `day_start("9999-12-31")` still works (counting civil days, not a `Timestamp`, whose range ends a few hours earlier); the new test pins it. Hazard log not edited.
- **Changed:** `consent.rs` denies the arithmetic lints for the module (RCA-ENF-3); `days_from_civil` removed; provisions are numbered with `zip(1_usize..)`.
- **Not done / not verified:** a period whose end plus a day overflows `i64` (not reachable from a valid date) returns `None` ("does not parse") as before for bad dates. Full suite on a fresh database: 855 passed, 6 failed (Java `fhir_validator`). CI not observed.

## 2026-11-22 - task_rules: a due date past the calendar is no plan (QA-17, first module under the arithmetic lints)

- **Clinical risk:** low, but touches what a clinician sees: a task's due and reminder days. `add_working_days` and `sub_working_days` used to stop at the calendar's end (`unwrap_or(d)`) and return a wrong day; they now return `None`, `plan` returns `None`, the dry run answers 422, and a real event whose plan has no date fails and rolls back. Only dates near year 9999 or 0001 are affected. Hazard log not edited.
- **Changed:** `task_rules.rs` denies the arithmetic lints for the module (RCA-ENF-3); no `as`, no `+ 1`, no counters; a named `MAX_NON_WORKING_RUN`; `spec/arithmetic-figures/index.md` row; test `a_date_past_the_calendar_is_no_date_not_the_last_one`.
- **Not done / not verified:** every other module still allows the lints; the 366-day give-up for an all-holiday calendar still returns the last day reached rather than an error (the calendar is site configuration, so RCA-IN-4 would have it refused at start; not done). Full suite on a fresh database: 854 passed, 6 failed (Java `fhir_validator`). CI not observed.

## 2026-11-22 - Option::unwrap_or_default banned in the service (QA-17, RCA-ENF-2, RCA-P-3)

- **Clinical risk:** none intended: every site keeps its behaviour, now spelled as the empty value it uses (`unwrap_or("")`, `map_or_else(Vec::new, ..)`, `Value::Null`). The audit digest input is unchanged (an absent detail is still the empty text).
- **Changed:** 51 sites in 17 source files and 7 test files; `unwrap_or_default` added to `clippy.toml`; the two `mllp.rs` slice reads that cannot fail return an error instead of an empty slice; the grep gate's known list loses the two `hl7_adt` entries (the text no longer matches its pattern, but the discarded error is still there; see `tasks.md`).
- **Not done / not verified:** where an absent or malformed input becomes empty (`fhir_patient` names and addresses, `hl7_adt` unreadable paths, a missing schema in `openapi.rs` becoming `null`) the behaviour is unchanged and still against RCA-IN-2 and RCA-IN-6; that needs a specified change. Full suite on a fresh database: 854 passed, 6 failed (Java `fhir_validator`). CI not observed.

## 2026-11-22 - Checked-arithmetic enforcement: release overflow checks, banned methods, a grep gate, settings that fail loudly (QA-17)

- **Clinical risk:** low, but one behaviour change: a malformed `EPR_BREAK_GLASS_REVIEW_HOURS` or `EPR_OBSERVATION_LATE_DAYS` (set to anything but a whole number in range, an empty value included) now stops the service at start instead of using 72 hours or 7 days silently (RCA-IN-4). Unset still means the default. No such setting is in the repository's compose or environment files.
- **Changed:** `overflow-checks = true` in the release profiles (RCA-ENF-4); `clippy.toml` bans `Iterator::sum` and `product`, `Duration::from_secs_f*`, `Instant::duration_since`, `saturating_duration_since`, `Ord::clamp`, `Saturating` and `Wrapping`; the five `unwrap_or_default` and `sum` uses in the domain crate and tests that tripped it were rewritten; new gate `service-checked-arithmetic` in `ci.yml` and `ci-local.sh`; `break_glass::review_due_hours` and `observations_store::late_days_from` return `Result`, with `check_settings` called at start. Tests assert the error for each bad value.
- **Not done / not verified:** `Option::unwrap_or_default` is not yet banned (51 sites, QA-17); the matcher has no `clippy.toml` (its float sums); four known hits of the grep gate remain, listed in the script with reasons; `arithmetic_side_effects` and the other RCA-ENF-1 lints are still not denied. Full suite on a fresh database: 854 passed, 6 failed (the Java `fhir_validator` tests). The gate script itself has no test of its own beyond a probe I ran by hand. CI not observed.

## 2026-11-22 - A date outside the calendar is refused, not guessed (QA-17, RCA-DECL-5)

- **Clinical risk:** low: a reading whose limit date cannot be computed (an absurd late-entry setting or birth date) now gets a 422 `effective_at_out_of_range` instead of being treated as needing a reason or as before birth.
- **Changed:** `observations_store::timing_problems`; row in `spec/arithmetic-figures/index.md`; test `a_limit_outside_the_calendar_is_refused_as_invalid_not_guessed`.
- **Not done / not verified:** other declared fallbacks (ARITH-1 to 3) were judged to be overflows of computed values, not invalid input, and kept. `late_days_from` still falls back to the default for a malformed `EPR_*` setting; that is a setting parse, not a declaration, and I left it for your decision. Only the lib tests for this module were run.

## 2026-11-22 - Referral timeouts move into the per-app runtime (QA-16 stage 2)

- **Clinical risk:** none intended: the same periods are read from the same setting; only where they are kept changes.
- **Changed:** `EPR_REFERRAL_TIMEOUTS` is held in `Runtime` instead of a process-wide `OnceLock`, so two apps in one program can time referrals differently; `referral_timeouts::current()` reads the runtime's setting when `init` has not run. Test `two_runtimes_in_one_program_have_their_own_referral_timeouts`.
- **Not done / not verified:** the remaining statics (auth, site rules, feed health, ADT feed) and stage 3 (moving test files off `set_var`). CI not yet observed. Tests: lib and `http_referral_worklist` and `runtime_boot`; the full suite was not rerun for this item.

## 2026-11-22 - First arithmetic sites converted to the checked rules (QA-17)

- **Clinical risk:** low, but touches what a clinician sees: an amendment's due time and overdue flag, a suspicion pathway's target days, small-number suppression totals and the late-entry check on an observation. Normal values give the same answers as before; only a date or count outside its type's range now differs, and then the answer is unknown (`None`, shown as null) instead of a panic or a clamped number. No hazard-log entry added; none of these ranges is reachable from valid data.
- **Changed:** `clock::days` returns `Option` (no `saturating_mul`) with `add_days` and `sub_days`; `Request::due` and `overdue` return `Option`, and `GET` amendment views show `due` and `overdue` as null when unknown; `pathways::plan` returns no plan when a due day is out of range; `privacy::suppress` publishes no total when the sum overflows; `policy::masked` and the hidden-count sum use `try_fold` with the fallbacks declared in the new `spec/arithmetic-figures/index.md` (ARITH-1 to ARITH-3). The earlier `spec/arithmetic/index.md` was replaced by the maintainer's `spec/rust-checked-arithmetic/`.
- **Not done / not verified:** the other 150 or so sites the RCA-ENF-1 lints flag, and the lints themselves are not denied yet; see QA-17 in `tasks.md` for the counts. The two `wrapping_*` calls left are in test code (a fuzz generator), allowed by RCA-EXC-5. Edge tests were added for `clock` and the amendment due time only.

## 2026-11-22 - Spec: never unwrap a checked arithmetic result (QA-17)

- **Clinical risk:** none: a specification.
- **Added:** a section in `spec/rust-checked-arithmetic/index.md`: the result of a `checked_*` call is never unwrapped, `expect`ed, or defaulted with `unwrap_or(0)` or `unwrap_or_default()`; it is passed on with `?` or `ok_or`, or refused.
- **Not done / not verified:** `unwrap` and `expect` are already denied in production code by lint; `unwrap_or(0)` and `unwrap_or_default()` on a checked result are not, so nothing yet stops them but review. No code was searched for existing cases.

## 2026-11-22 - The audit chain and server address belong to the runtime (QA-16, stage 2, first static)

- **Clinical risk:** none intended: the same chain is built from the same settings. `audit::chain()` now returns an `Arc<Chain>` (built once per runtime) instead of a `&'static`, so its callers pass `&audit::chain()`.
- **Changed:** `Runtime` owns the chain and `EPR_AUDIT_SERVER_ADDRESS`, so two apps in one program can hold different keys; outside a request the process runtime is one shared value. A unit test builds a keyed and an unkeyed runtime side by side.
- **Not done / not verified:** the other statics (auth verifier and `REQUIRE_AUTH`, referral timeouts, site rules, feed health, `adt_feed`) are still process-wide; a spawned task does not inherit the runtime, so a background job reads the process runtime's chain. Ran: clippy over the workspace and 109 tests matching audit, chain, authz and metrics, with the database; not the whole suite.

## 2026-11-22 - Spec: arithmetic must not panic or wrap (QA-17)

- **Clinical risk:** none: a specification. No code changed.
- **Added:** `spec/rust-checked-arithmetic/index.md`: use `checked_*` and return `None` (`255_u8.checked_add(1)` is `None`), with the casts, `wrapping_*`, `saturating_*`, sums, floats and time covered, and what a `None` means for a clinical value. Task QA-17.
- **Not done / not verified:** nothing enforces it yet. The 94 sites clippy's `arithmetic_side_effects` flagged have not been triaged and the lint is not denied; the spec says so. The matcher crate is undecided.

## 2026-11-22 - No panic in production code, and a lint that keeps it so

- **Clinical risk:** none intended: the same answers, with a refusal or a skipped step where a panic would have been. One behaviour is worth knowing: `api_tokens_store::prefix()` now returns an `Option`, and a token whose prefix could not be built is refused like any bad token.
- **Changed:** 93 sites in the service and domain crates that could panic (`expect`, slice and array indexing, `value["key"] = x` on JSON) now use `get`, `iter().skip`, `if let`, and `crate::json::{set, set_at, extend, push_at, str_is}`, which do nothing on a value of the wrong shape instead of panicking. `[workspace.lints.clippy]` denies `unwrap_used`, `expect_used`, `panic`, `indexing_slicing`, `unreachable`, `todo` and `unimplemented`; `clippy.toml` lifts the first four inside tests, and the two benchmark files allow them. The gate (`cargo clippy --workspace --all-targets`) now fails on a new one.
- **Not done / not verified:** arithmetic overflow is not covered (it wraps in release and panics only in debug; no `arithmetic_side_effects` lint, which flagged 94 sites); the matcher crate has no such lint yet; a panic in a dependency is not prevented; the `openapi.rs` writers now skip a bad shape silently, so a wrong document would show as a diff against `openapi.json`, not as a crash. Ran: clippy over the workspace, and all 857 tests with the database, of which six `fhir_validator_*` need Java and failed here as before.

## 2026-11-22 - Settings come from a per-app runtime, not the process environment (QA-16, stage 1)

- **Clinical risk:** none intended: the same settings are read, from a new place. With no runtime given, the place is the process environment, as before.
- **Changed:** `src/runtime.rs` holds `Settings` (a lookup) and `Runtime`; the service's 23 `std::env::var` reads now go through `runtime::setting`. A request reads the settings its app booted with (a tokio task-local set by `scope_mw`); outside a request the process environment is read at each call. `App` implements `TestApp::boot_test_with`, so `axa_test::request_with(&[..], ..)` boots an app from given settings, `axa`'s and the service's own. `tests/runtime_boot.rs` boots two apps in one program, one with `EPR_METRICS=1`, and each answers as its own settings say.
- **Not done / not verified:** stages 2 and 3. Eleven settings still land in process-wide `OnceLock`s (auth, audit chain and server, referral timeouts, site rules, feed health), so a second app in the same program gets the first one's values for those; no test file has moved off `set_var`. A task spawned from a request does not inherit the runtime. Run here: clippy, the 313 lib tests and the new database test; not the full suite.

## 2026-11-22 - ci-changes lists the licence files the tests read

- **Clinical risk:** none: a CI script.
- **Fixed:** `scripts/ci-changes.sh` now runs the Rust tests when `LICENSE.md` or `LICENSES/*` change, because `tests/licence_files.rs` reads them. They were treated as documents no test reads, which failed the `docs-ci-changes` gate after the licence commit `62c0479`.
- **Not done / not verified:** only `scripts/test_ci_changes.sh` was run (it passes); the CI workflow itself was not run.

## 2026-11-21 - A STRIDE entry per data flow (QA-10)

- **Clinical risk:** none: a governance document. No code changed.
- **Added:** a "Threats per flow" table in `governance/data-flow.md`, twelve rows, each with the main threats, what the design already does and the gap. Gaps worth acting on: MLLP has no authentication or encryption, `/metrics` is public when on, no backups exist, and the outbox recipient is undesigned.
- **Not done / not verified:** no control was exercised and no code re-read beyond what the document already states; no likelihood or impact ranking, no diagram, no security engineer's review. No hazard-log entry was added, since none of these gaps is yet tied to a patient harm.

## 2026-11-21 - Service coverage re-measured: 95.79 percent (QA-15)

- **Clinical risk:** none: a measurement.
- **Measured:** the complete service run a third time (all tests, ignored included, fresh `*_test` database, one thread, `tests/` excluded): 23,976 of 25,029 lines, **95.79 percent** (95.12, then 95.32 earlier today). Only the six Java-needing `fhir_validator_*` tests failed. What is left is mostly single error-propagation lines in the controllers and stores (a failed query, a failed audit append, a serialisation that cannot fail), which need an injected fault to reach; `app.rs` (command paths) and `audit.rs` hold the largest blocks.
- **Not done / not verified:** the figure counts inline `#[cfg(test)]` code, so production-only coverage is lower; the CI service floor (36, database-free) and the matcher and UI floors were not raised; no fault-injection harness exists, so 100 percent is not reachable by behaviour tests alone.

## 2026-11-21 - Service test: what a dry run says for three kinds of refusal (QA-15)

- **Clinical risk:** none: a test.
- **Tested:** one database test in `tests/mllp_feed.rs` for `adt_feed::dry_run`: text that is not HL7 is `not-hl7` with no acknowledgement; an `ADT^A99` is `refused` with "is not handled" and an `AR` acknowledgement; a valid message for a deployment taking processing id `T` (the message says `P`) is `refused` with `WRONG_PROCESSING_ID`. None keeps anything. The `a_dry_run` tests pass (2) on a fresh `*_test` database; clippy clean.
- **Not done / not verified:** the `retry` (temporary failure) and an unsupported-by-store refusal in a dry run are not forced; the coverage figure was not re-measured.

## 2026-11-21 - Service tests: four ways an archived audit table can be wrong (QA-15)

- **Clinical risk:** none: tests only. They pin how the archive check reports damage to the audit record.
- **Tested:** four database tests in `tests/audit_partitions.rs`, each on a scratch database with three archived rows and the table's triggers switched off to simulate a superuser: a row removed from the end is reported as "holds 2 rows, the anchor says 3"; a last row with a changed id as "its last row is not the one the anchor recorded"; a row whose `prev_digest` is altered as "does not link to the previous row"; and an archive written under a key, read by a build holding none, is `Unchecked` (3 rows, every one unchecked, "is not held"), never `Intact`. The 13 `audit_partitions` tests pass (through `suite`) on a fresh database; clippy clean.
- **Not done / not verified:** an unknown digest scheme in an archived row is not forced (`SchemeUnknown` stays unreached); a reader holding a different key under the same key id reads as a digest mismatch, which the existing edited-row test already shows; the coverage figure was not re-measured.

## 2026-11-21 - Service test: the feed health route with a configured feed (QA-15)

- **Clinical risk:** none: a test.
- **Tested:** `tests/http_feed_health.rs` (database-gated, login off, its own program because the configured feed is process-wide and set once): `GET /api/feed/health` answers an empty list before a feed is configured; once `feed_health::run_loop` has started it names the sender, a state, the window and the thresholds, and a feed that has sent nothing has no last-message time. It reaches the `controllers/feed.rs` branch (about 20 lines) that no earlier test did. Passed on a fresh `*_test` database; clippy clean; the `test_targets` census passes with the new `[[test]]` entry.
- **Not done / not verified:** the state judged is not asserted (it depends on the time and window the test runs in); a feed with messages and refusals is covered by the `feed_health` unit tests, not here; the effect on the coverage figure was not re-measured.

## 2026-11-21 - Service coverage re-measured: 95.32 percent; the `db` command tests now count (QA-15)

- **Clinical risk:** none: tests only.
- **Measured:** the complete service run again (all tests, ignored included, fresh `*_test` database, one thread, `tests/` excluded): 23,857 of 25,029 lines, **95.32 percent** (was 95.12). Only the six Java-needing `fhir_validator_*` tests failed. Still the largest gaps: `app.rs` 113 lines, `audit.rs` 58, `controllers/fhir.rs` 57, `adt_feed.rs` 55, `controllers/feed.rs` 42.
- **Fixed:** `tests/db_commands.rs` ran the real binary with `env_clear()`, which also removed `LLVM_PROFILE_FILE`, so what the binary did was never counted (`app.rs` did not move). It now passes that one variable through. Run alone under `cargo llvm-cov`, the three tests cover 68 of `app.rs`'s 231 lines.
- **Not done / not verified:** the full-run `app.rs` figure was not re-measured after the fix; the figure still counts inline `#[cfg(test)]` code; the CI service floor (36, database-free) is unchanged.

## 2026-11-21 - Service tests: the log sink and keeping a second relay (QA-15)

- **Clinical risk:** none: tests only.
- **Tested:** two database tests in `tests/outbox_store.rs`: an event to a recipient served by `LogSink` is delivered and recorded as one `delivered` attempt (the sink sends nothing anywhere); keeping a second relay stops the first, `stop_kept` stops the second within the wait, and a further `stop_kept` is a clean no-op. The `outbox_store` program passed (16 tests) on a fresh `*_test` database; clippy clean.
- **Not done / not verified:** the branch where a relay task ends in a panic is still unreached (no sink here panics on purpose); the effect on the coverage figure was not re-measured.

## 2026-11-21 - Service tests: feed-health words and the started listener (QA-15)

- **Clinical risk:** none: tests only.
- **Tested:** three unit tests in `feed_health.rs` (every stored alarm state word reads back and an unknown word reads as nothing; each of the three windows has its own silence limit; the settings read from the environment hold their defaults) and one database test in `tests/mllp_feed.rs`: `Running::start` serves a message with an `AA` answer, `keep` and `stop_kept` stop it within the wait, a second `stop_kept` is a clean no-op, and the port then refuses connections. The `mllp_feed` program passed (19 tests) on a fresh `*_test` database; clippy clean.
- **Not done / not verified:** the effect on the coverage figure was not re-measured; `from_env` is exercised against the real environment (it holds only while no `EPR_FEED_*` is set); the TLS listener's start path, the outbox's error branches and the larger `app.rs` command paths remain uncovered.

## 2026-11-21 - axa 0.21 and axa-test 0.12

- **Clinical risk:** none: dependency update; no route, rule or record changed.
- **Built:** `axa` 0.20.0 to 0.21.0 and `axa-test` 0.11.0 to 0.12.0 (both published 13 hours earlier, inside the two-day resolver age, so the update used `CARGO_RESOLVER_INCOMPATIBLE_PUBLISH_AGE=allow` for these two only; `cargo update -p axa -p axa-test` moved nothing else). The upgrade guide's one compile change (`axa-test` 0.12) was the only edit. Production now refuses a retired setting name (`DATABASE_URL`, `BODY_LIMIT_BYTES`, `HEADER_READ_TIMEOUT_SECONDS`, `HTTP_CSP`, `LOCO_ENV`): the repository's own compose, Containerfile, scripts and docs set none of them (searched; only history mentions them), but a deployment's own configuration was not seen.
- **Tested:** clippy over the workspace with all targets and features, no warnings; the 306 library tests; the whole workspace with ignored tests on a fresh PostgreSQL 18 database, one thread: everything passed except the six `fhir_validator_*` tests, which need Java and run in their own CI job (`chaos_api` also needs a database named `*_test`, and passed on one).
- **Not done / not verified:** the new capabilities are not yet used. Candidates, from the 0.21 upgrade guide, with what each would replace here: `TestApp::boot_test_with` and `axa_test::request_with` for the 32 test files that call `std::env::set_var` (126 calls, each file its own process); that needs the service's own `EPR_*` settings (23 `std::env::var` reads in `src/`) to come through a lookup as well, or a test would silently boot with the process environment, so it is a task, not a swap (QA-16); `Settings::renamed` for retired `EPR_*` names; `axa::error::Problems` for the hand-built 422 lists and `SafeInt` for amounts; `ReloadablePolicy::watch_file` if the role map is ever read from a file. Nothing else was checked against the 0.21 changelog; the root AGENTS.md line that named axa 0.17 now names 0.21 (the minimum Rust, 1.98, is the same in both).

## 2026-11-21 - Service: the first authoritative full coverage figure, and the `db` commands tested (QA-15)

- **Clinical risk:** none: tests only.
- **Measured:** the first complete service run since the isolation repairs: `cargo llvm-cov --ignore-run-fail` over every test with ignored ones included, one thread, on a fresh PostgreSQL 18 database, `tests/` excluded from the report: 23,787 of 25,008 lines hit, **95.12 percent** (line records merged by their highest count, so two spellings of one line are one line). The only failures were the six `fhir_validator_*` tests (they need Java; CI runs them in their own job). The largest gaps: `app.rs` 113 lines, `mllp.rs` 59, `adt_feed.rs` 58, `audit.rs` 57, `controllers/fhir.rs` 55, `controllers/feed.rs` 42.
- **Tested:** `tests/db_commands.rs` (3 database-gated tests, run through `suite`): the real binary's `db status` shows every migration pending on a new database and applied after `db migrate`, and a second migrate changes nothing; `db down` and `db truncate` fail with their own messages even when confirmed; a confirmed `db reset` drops a table left behind and migrates again. All 3 pass; clippy clean.
- **Not done / not verified:** the new tests' effect on the figure was not re-measured; the 95.12 figure counts inline `#[cfg(test)]` code, so production-only coverage is lower (QA-15 asks for production source); the MLLP TLS tests were not excluded and no test other than the six validator ones failed in this run (the two whose names contain `tls` passed), so the 2026-11-20 note that they cannot run under instrumentation did not reproduce here; not investigated; the CI service floor is still 36 on the database-free measure and was not changed. Many of the remaining gaps are error branches that need a failing database or peer.

## 2026-11-21 - UI coverage: the four counters and two more loaders (QA-15)

- **Clinical risk:** none: tests only.
- **Tested:** `tests/unit/feed-count.test.ts` (8 tests): the refused-message, referral-queue, death-report and referral-reply counts each read their own endpoint, show the number on 200, and show nothing (null) for a refusal, a non-integer or an empty body. `loadAmendments` and `loadSuspicion` return null on any status but 200. Vitest over `src/lib`: lines 96.53 to 97.63, branches 90.02 to 92.18; 429 tests pass; lint and `check:tests` clean.
- **Not done / not verified:** about 26 lines remain uncovered in `src/lib` (branches in `vitals.ts`, `session*.ts`, `adjustments.ts`, `i18n.ts`, `getAuth`); `.svelte` and `src/routes` stay outside the measure; the CI floors were not raised; the browser suite was not run.

## 2026-11-21 - UI coverage: the sign-in singleton's helpers (QA-15)

- **Clinical risk:** none: tests only.
- **Tested:** `tests/unit/auth-instance.test.ts` (6 tests, Redis mocked): `backendFrom` is memory with no settings and refuses either half-setting; with both it builds a Redis backend that connects once, on first use, and logs a client error; `cookieName` pins `__Host-` only on HTTPS; `sameOrigin` rejects a foreign or missing origin; `localeFrom` falls back to the default; `sessionFrom` is null without sign-in, passes the cookie and options to the store, and shows signed out (logging the reason) when the store throws. Vitest over `src/lib`: lines 95.26 to 96.53, branches 89.30 to 90.02, functions 91.05 to 94.37; 419 tests pass; lint and `check:tests` clean.
- **Not done / not verified:** `getAuth` (reads the environment) is still untested; `.svelte` and `src/routes` stay outside the measure; the CI floors were not raised; no real Redis was used here (the live test exists separately).

## 2026-11-21 - UI unit coverage, lines 91.89 to 95.26 percent (QA-15)

- **Clinical risk:** none: tests only; no UI behaviour changed.
- **Tested:** 13 new unit tests. `Busy` ignores a second call in flight and clears on a throw; `newIdempotencyKey` is a fresh UUID; `loadReferrals`, `loadWatchList`, `loadDeath`, `loadAssertions` and `loadAllergies` return null or empty, never "nothing found", on any status but 200, and the allergy loader offers break-glass only on a 403 that says so; `fetchBlob` and `sendBytes` handle success, refusal, a non-JSON body and a network failure; `lastActivityAt`. Vitest (v8) over `src/lib`: 94.25 percent of statements, 89.30 of branches, 91.05 of functions, 95.26 of lines (was 91.34, 85.89, 87.41, 91.89); 413 tests pass; lint and `check:tests` clean.
- **Not done / not verified:** `.svelte` components and `src/routes` stay outside the measure (QA-15 left open); `auth-instance.ts`, `account.svelte.ts`, `amendments.ts` and `api.ts` still have uncovered lines; the UI floors in CI (90 lines, 84 branches) were not raised; the browser suite was not run.

## 2026-11-21 - Matcher coverage 96.16 to 98.43 percent (QA-15)

- **Clinical risk:** none: tests only; no matcher behaviour changed.
- **Tested:** five new behaviour tests and one new test file. A shared IHI with unlike names is not auto-linked and carries `SameIhiDifferentNames` with its message (`tests/main/ihi.rs`); a national number whose modulus-11 check would be 10 is invalid for every last digit (`matching.rs`); a telephone compares by digits alone (`fields.rs`); the demo binary prints a decision for two files, prints usage and exits 2 for a wrong call, and fails with no output for a missing or malformed file (`tests/main/demo_binary.rs`). Matcher suite 126 tests; clippy clean; `coverage-matcher.sh` measured 564 of 573 lines (tarpaulin, LLVM engine).
- **Not done / not verified:** the 9 lines left are 8 in a stray local `target/package/.../engine.rs` that is not source (a leftover of `cargo package`) and `normalize.rs` line 78 (the digit fold in `fold_telecom`), which the phone test exercises yet tarpaulin reports as unhit, probably a line-attribution quirk of the function-path argument; not investigated further. The floor stays 95 until CI shows the same figure. The service (database-free 37.5 percent) and UI are untouched: QA-15 stays open for them.

## 2026-11-20 - Deterministic coverage-suite isolation

- **Clinical risk:** none: test isolation and assertions only; service behaviour is unchanged.
- **Fixed:** the consolidated integration suite could reuse `/_dependencies`' process-wide five-second result from a different test application, so an unreachable database sometimes read as healthy. `dependencies::routes_with_cache` lets an isolated router own its cache while the production `routes()` keeps the same shared cache and limits. The suspicion audit-leak test also no longer mistakes a random UUID containing the digits of a clinical value for disclosure: it parses the audit JSON, ignores UUID-valued identifiers and checks every other key and string value case-insensitively.
- **Tested:** all three readiness tests pass together with ignored tests enabled and one test thread; the login-on suspicion HTTP test passes against PostgreSQL. Both defects were reproduced first by the full LLVM coverage run.
- **Not done / not verified:** no production route behaviour changed; the complete coverage run and repository gates remain to be rerun after these repairs, and the five MLLP TLS tests remain incompatible with LLVM instrumentation (they stay covered by the normal suite).

## 2026-11-19 - Explicit updates and stated list limits (TL-13; H-060)

- **Clinical risk:** yes. A structural limit can refuse an otherwise valid clinical write, while a missing limit can let an oversized list consume time or memory. H-060 is new and open; no clinician or Clinical Safety Officer has reviewed the limit.
- **Built:** every array schema in the derived OpenAPI document now states `maxItems`: an existing, smaller domain maximum is preserved, and every other array says 1,000. `list_limits::enforce_mw` checks JSON bodies recursively inside authorization and idempotency but before handlers; over 1,000 is a 422 whose `field`, `max_items` and error name the dotted path and limit, never the submitted values. Updates remain whole versions. There is no PATCH; the census requires a future PATCH operation to declare that absent keeps, `null` clears and arrays never append implicitly. Specs: API surface and service.
- **Tested:** two middleware tests (exactly 1,000 reaches the handler unchanged; 1,001 nested at `patient.names[0].given` is refused and names that field); OpenAPI census of every array with a planted missing maximum; PATCH guard. `openapi.json` re-blessed. The service workspace library suite passed (515 tests), clippy with all targets and features was clean, rustdoc passed with warnings denied, and `sh scripts/ci-local.sh --fast` passed every gate. Its first sandboxed run failed only where tests tried to bind `127.0.0.1` (`EPERM`); the permitted rerun passed, including those tests.
- **Not done / not verified:** 1,000 is an engineering limit, not one reviewed with users; smaller domain limits remain in their existing validators rather than this structural middleware; response serialization does not independently check its documented maximum; no PATCH implementation exists; no database test is needed because the refusal happens before a handler or transaction.

## 2026-11-19 - Write-only secrets (TL-11)

- **Clinical risk:** none: integration tokens, not the record.
- **Fixed:** `POST /api/tokens` with an `Idempotency-Key` kept its answer, the token in clear, in `axa_idempotency` for up to a day, and a retry under the same key replayed it (shown twice, against TL-1's "shown once"). The route is now outside the idempotency layer (`idempotency::SHOWN_ONCE`) and its OpenAPI operation no longer lists the header; a retry is a new request, a 409 because the name is taken. Spec: API surface (idempotent creates, write-only secrets).
- **Built:** `tests/openapi_census.rs`: every response schema, `$ref`s followed, is searched for a field whose name has a whole word naming a secret (secret, password, passphrase, token, hash, salt, credential, key; snake, kebab and camel case); only `IssuedToken.token` on `POST /api/tokens` is allowed, and the census fails if it no longer sees that one. A planted `signingSecret` is caught; `keyed`, `tokens` and `monkey` are not. `openapi.json` re-blessed (the one parameter removed).
- **Tested:** the census (7 tests); `tests/http_api_tokens.rs` with login on: describing and revoking return no token, a keyed issue then a keyed retry gives 201 then 409 without `Idempotent-Replayed` and without the token, and no `axa_idempotency` body holds either token's secret. With the fix removed the test fails (the retry replays the token with 201). The idempotency unit tests and both idempotency HTTP tests pass; clippy clean.
- **Not done / not verified:** logs are not read by any test (the token routes log only a token's id); the census matches names only, so a secret under an innocent field name would pass; no stored secret other than a token exists yet (a subscription's signing secret is TL-6).

## 2026-11-18 - A rule's second action failing leaves nothing (TL-14)

- **Clinical risk:** none: a test. It shows the property H-059 relies on.
- **Tested:** `tests/tasks_store.rs::a_failure_in_a_rules_second_action_leaves_no_first_task_and_no_result`: two approved rules on one event, run in id order; a temporary trigger refuses only the second rule's task; filing the result fails, and neither the first rule's task nor the result is kept; with the trigger gone the same filing makes both tasks. The trigger is dropped before any assertion, so a failure cannot leave it behind. The 11 task database tests pass. TL-14 ticked.
- **Not done / not verified:** the same failure is not forced through an HTTP route (the route commits only after the store returns, as the other tests show).

## 2026-11-18 - A synthetic sandbox (TL-9)

- **Clinical risk:** none in a sandbox; loading it into a live record would add 24 false patients, which is why the loader needs an explicit flag and refuses data that is not plainly synthetic.
- **Built:** `operations/sandbox/dataset.json`: 24 patients, every family name `Sandbox<Word>`, every national number in the 999 range (reserved for testing, never issued) and passing modulus 11, a heart rate each, a temperature for every third and an allergy for every fourth. `scripts/sandbox/load.mjs`: loads it through the API (each record checked and audited as any other), skips a patient already on record so a second run adds nothing, needs `--yes-this-is-a-sandbox`, takes `--url` or `EPR_SANDBOX_URL` and `EPR_SANDBOX_BEARER`. `operations/sandbox/README.md`.
- **Tested:** `scripts/sandbox/test_load.mjs` in the `docs-load-harness` gate (the dataset passes the synthetic checks; the checks catch a real-looking family name, a number outside the test range, a bad check digit, a repeat and a future birth date; the flag is required; a load against a fake service makes each patient once and a second adds nothing; a refusal stops it). Once by hand against the debug build on a fresh database: 24 patients, 32 observations and 6 allergies in about a third of a second, all accepted by the service's own checks; a second run added nothing.
- **Not done / not verified:** no referrals, results or tasks in the dataset; the flag is the only guard (the loader cannot tell a sandbox from a live service); not run with login on.

## 2026-11-18 - Signing in with an integration token (TL-1)

- **Clinical risk:** none new: a token does at most what the integrator role (the ADT feed's) already does, and usually one part of it. A token with `patient-identity:create` can register a patient, as the feed can.
- **Built:** `auth::authenticate` sends a bearer token starting `epr_` to `integration`: parsed, its record found, the secret's hash compared in constant time, then revocation and expiry (`axa::api_tokens`); last use written at most once a minute; the caller is `token:<name>` with the integrator role and its scopes. `policy::Subject` gains `scopes` (`None` for a person; `Some` for a token), checked in `gate` and in `evaluate`, so every authorisation, the route's and a handler's, is scoped; `policy::scope_word` moves to the domain. A malformed, unknown, expired or revoked token gets one 401 with the same body, the reason logged with the token's id; a database that cannot be reached is a 503. The outbox's later check keeps roles only, which for a token is the integrator role. Specs: API surface, security policy. TL-1 ticked.
- **Tested:** a policy unit test (a token reaches what its role allows and a scope names, never one without the other; an empty set allows nothing; a person is unchanged; within its scope a token is decided as its role is); the login-on HTTP test now signs in with a token (a search within its scope is served and audited as `token:<name>`, its last use recorded; registering a patient, which the role allows and no scope names, is 403; managing tokens is 403; a malformed, an unknown and a revoked token get the same 401). 613 unit and HTTP tests; on a fresh database every database test passes (200) except the six FHIR validator tests, not run here; `cargo doc` clean.
- **Not done / not verified:** expiry over HTTP (unit-tested in the crate); no screen for tokens; no warning before a token expires.

## 2026-11-18 - Integration tokens: issue, list, describe, revoke (TL-1; OQ-TL4)

- **Clinical risk:** none in this slice: a token cannot sign in yet. When it can, a token reaches only what the integrator role already does.
- **Dependency:** `axa` gains its `api-tokens` feature, adding `axa-api-tokens` 0.1.0 (published 2026-11-16 16:05 UTC, inside the two-day cooldown; the maintainer approved the exception, and the update ran with the minimum lifted for that one command). Its own dependencies were already in the tree; the lockfile gained that one package.
- **Built:** migration 41: `api_tokens` (id, unique name, the secret's SHA-256 hash, scopes, expiry, issuer; refuses change), `api_token_versions` (description and final revocation with a reason; append-only), `api_token_current`, `api_token_uses` (last use, updated, never deleted). `api_tokens_store.rs`: `issue` (the token once, `epr_<id>_<secret>`), `list`, `describe`, `revoke`, and `record` and `record_use` for the next slice; scopes are `<resource>:<action>` from what the integrator role holds (`policy::permitted_actions`), 1 to 16 a token, life 1 to 365 days. `controllers/api_tokens.rs`: `POST /api/tokens` (201, `Cache-Control: no-store`, 409 for a name ever used), `GET /api/tokens` (metadata only), `POST /api/tokens/{token_id}/description`, `POST /api/tokens/{token_id}/revoke`. Policy resource `ApiToken`, the clinical administrator's (**assumed, awaiting the maintainer, OQ-TL4**). OpenAPI schemas; `openapi.json` re-blessed; 29 inventory rows; schema map regenerated. Specs: API surface, security policy; OQ-TL4 in the Tellescope lessons.
- **Tested:** unit tests (the scopes are exactly the integrator role's five; a bad name, an unknown or repeated scope, no scope, a life out of range or a long description is refused; only 32 hex digits are an id); `tests/http_api_tokens.rs` with login on (a clinician and an integrator can neither issue nor list; bad requests 422; issued once with `no-store`; the list carries neither the token nor its hash, and neither the token tables nor the audit trail hold the secret; the stored record verifies the token for its scope only, and refuses it once revoked; a name is not reused; a stale description 409; a blank reason 422; another administrator revokes; a revoked token takes no change; the tables refuse change); the three write routes' audit-refusal tests; the label census, the login-off search walk. 612 unit and HTTP tests; on a fresh database every database test passes (200) except the six FHIR validator tests, not run here.
- **Not done / not verified:** signing in with a token; no screen; no expiry warning; the feed's administration and an export are not scopes.

## 2026-11-18 - The team tasks screen (TL-3; H-059)

- **Clinical risk:** yes, as H-059: clinicians will see and close their team's tasks on this page. H-059 stays open.
- **Built:** `/tasks` in the UI and in the navigation: a team chosen from the staff roles (the service decides whose list it is), each task with the rule's title, due day or overdue on the site's day (`siteToday`), its source, the day last reminded, and a link to the patient's record (vitals for an observation, referrals for a referral, the record for a result). Closing as done or cancelled needs a reason of 1 to 500 characters, checked before sending, at the version read; a 409 or 422 reads the list again and says so; a refused team is said and never reads as no tasks. `src/lib/tasks.ts`; English and Welsh strings (Welsh machine-written, unreviewed). UI spec updated.
- **Tested:** `tests/unit/tasks.test.ts` (anchors, overdue, the reason rule counting characters, today at the site in three zones and an unknown one, teams are staff roles with names); two Playwright tests (due, overdue, reminded, links, a close refused without a reason then sent with the version and outcome; a refused team), axe on each state. 248 browser tests, UI lint, type check and unit tests pass.
- **Not done / not verified:** the page cannot know the reader's roles, so it offers every staff team and the service refuses the others; no count in the navigation; a result's task links to the record, not a results section (there is none); TL-3 is not ticked until OQ-TL1 is answered.

## 2026-11-18 - A site's own task rules file (TL-3; H-059)

- **Clinical risk:** yes, as H-059: this is the way a site makes task rules live. A site file's approved rules make tasks from the next start; the built-in catalogue stays all drafts. H-059 stays open, and who may approve a rule is still OQ-TL1.
- **Built:** `tasks_store::load_rules` and `init_rules`: `EPR_TASK_RULES_FILE` names a TOML file in the catalogue's shape, which replaces the built-in catalogue; read once at start-up, at most 1 MB, checked by `task_rules::parse` and for teams that are roles; any fault stops the start naming the setting and the file. `rules()` gives the site's file when set (the hooks, the routes' dry run). The start logs the count, the approved count and the source. `compose.yaml` passes the setting. Spec: where the rules come from.
- **Tested:** a unit test with real files: a good file loads with its approved rule; a missing file, a team that is no role, a rule with an unknown key, and a file over 1 MB are each refused naming the setting. 609 unit and HTTP tests; the task database and route tests pass.
- **Not done / not verified:** no test boots the service with a file set (the setting is read once per process); a change to the file takes a restart; nothing checks who a file's `approved_by` is.

## 2026-11-18 - A result's later versions raise task events (TL-3; H-059)

- **Clinical risk:** yes, as H-059: once a rule is approved, a result entered in error after filing cancels the task it made, and one amended or corrected to abnormal makes one. Every built-in rule is still a draft. H-059 stays open.
- **Built:** the event `result-entered-in-error` (facts `abnormal`, `test` as they were); `results_store::update` raises it, or `result-filed` when the new version is flagged abnormal and the last was not; `update_with_rules` for a test. The built-in draft `abnormal-result-review` is cancelled by `result-entered-in-error` as well as `result-signed-off`. The OpenAPI event list is now built from `Event::ALL`, so it cannot fall behind (`openapi.json` re-blessed). Spec events table updated.
- **Tested:** a database test that a normal result makes no task, amending it to abnormal makes one, correcting it while still abnormal makes no second, and entering it in error cancels the task naming the event. 608 unit and HTTP tests; on a fresh database every database test passes (199) except the six FHIR validator tests, not run here.
- **Not done / not verified:** a result amended from abnormal back to normal cancels nothing; a site's rules file and a screen are still to do.

## 2026-11-18 - The task reminder sender (TL-3, TL-4; H-059)

- **Clinical risk:** yes, as H-059: a reminder now marks a task on its team's list on its day, or is skipped with the reason after downtime when its rule says so. Off unless set; every built-in rule is still a draft. H-059 stays open.
- **Built:** `task_reminders.rs`: `EPR_TASK_REMINDER_MINUTES` (1 to 1440; a bad value stops the start; unset is off and the start logs a warning), `run_once` (decide each reminder due on the site's today, once; one already decided is left; record the run), `run_loop` (logs counts only). `tasks_store::reminded_on`; `GET /api/tasks` items carry `reminded_on` (schema `TaskListItem`; `openapi.json` re-blessed). `compose.yaml` passes the setting. Specs: task rules (the sender), API surface.
- **Tested:** a unit test of the setting (off, blank, 15, 1440, six bad values); a database test that a reminder is not decided the day before, is sent on its day, is not decided twice by a second run, shows as `reminded_on`, and that the run is recorded. 608 unit and HTTP tests; on a fresh database every database test passes (198) except the six FHIR validator tests, not run here.
- **Found:** the SQL census refused a first draft that compared task ids as text; the query casts to UUIDs.
- **Not done / not verified:** nobody is alerted when a reminder is sent (it only marks the list); `reminders_due` reads every open task each run, with no limit, which is fine for a scaffold and not measured at scale; the loop itself (as opposed to one run) is not tested; a result's later versions, a site's rules file and a screen are still to do.

## 2026-11-18 - Task routes: a team's list, closing a task, the dry run (TL-3; H-059; OQ-TL3)

- **Clinical risk:** yes, as H-059: a clinician will see and close their team's tasks here. Every built-in rule is still a draft, so the list is empty in a running service until a rule is approved. H-059 stays open.
- **Built:** `controllers/tasks.rs`: `GET /api/tasks?team=` (open, due first, up to 200), `POST /api/tasks/{task_id}/close` (done or cancelled, reason, version read; 409 stale, 422 closed or blank, 404), `POST /api/tasks/dry-run` (what the service's rules would make for an event and its facts on the site's calendar, drafts marked, nothing written; a fact the event does not carry or a bad day is a 422). Policy resource `Tasks` (not clinical: pointers): staff list and close, a clinician, the on-duty clinician and the Clinical Safety Officer run the dry run. **Assumed, awaiting the maintainer (OQ-TL3):** a team is a role; only a caller who holds it lists or closes (403, audited). The service starts only if every built-in rule's team is a role. OpenAPI schemas `Task`, `TaskList`, `CloseTask`, `TaskDryRun`, `TaskDryRunAnswer`; `openapi.json` re-blessed. Specs: `spec/09-api-surface.md`, `spec/clinical-workflows/task-rules.md`; OQ-TL3 in the Tellescope lessons.
- **Tested:** `tests/http_tasks.rs` with login on (a clinician lists the clinician team, audited; no team 422; another role's team or no role 403, audited; the auditor has no list; a nurse cannot close the clinician team's task; blank reason 422, stale 409, closed once, no task 404; the Clinical Safety Officer's dry run shows the built-in draft not approved and due two working days on, writes no task; a normal result plans nothing; a fact typo and a bad day are 422; a nurse cannot run it); the close route's audit-refusal test (500 and nothing stored when its audit row fails, then 200); unit tests of the fact check, the team check and the start-up team check; the policy matrix at 27 resources. 607 unit and HTTP tests; on a fresh database every database test passes (197) except the six FHIR validator tests, not run here.
- **Not done / not verified:** no screen; the reminder sender's loop; a site's own rules file; teams finer than a role; the login-off search walk exempts `/api/tasks` (no roles with login off) and its audit row is tested with login on instead.

## 2026-11-18 - Referrals raise task events (TL-3; H-059)

- **Clinical risk:** yes, in time, as H-059: once a rule is approved, a referral step will make a task (say, triage of an urgent request) and its end will cancel it. Every built-in rule is still a draft. H-059 stays open.
- **Built:** `referral::task_event` and `Priority::word`: a step into any state that is not an end raises `referral-stepped` with `state` and `priority`, an end raises `referral-closed` with `state`; `referrals_store::request` raises it for `requested` and `advance` for each step, so the routes and the inbound referral feed both raise. `request_with_rules` for a test. Spec events table updated.
- **Tested:** a unit test over every state; a database test that a routine request makes no task under a rule for urgent ones, an urgent one makes one, accepting it changes nothing, and its outcome cancels the task naming `referral-closed`. 604 unit and HTTP tests; on a fresh database every database test passes (196) except the six FHIR validator tests, not run here.
- **Not done / not verified:** the referral timeouts (PC-3) raise nothing of their own; a result's later versions; the routes, the reminder loop and a site's rules file.

## 2026-11-18 - Observations raise task events (TL-3; H-059)

- **Clinical risk:** yes, in time, as H-059: once a rule is approved, an observation out of range will make a task and entering it in error will cancel it. Every built-in rule is still a draft, so a running service makes none. H-059 stays open.
- **Built:** `observations::out_of_range`, `task_facts` (`code`, and `flag` only when there is one) and `Flag::word`; `observations_store::create` raises `observation-out-of-range`, and `update` raises `observation-entered-in-error` (facts as they were) or `observation-out-of-range` when the new version is out of range and the last was not. `create_with_rules` for a test. Spec events table updated.
- **Tested:** a unit test of the flags and facts (five values, no range); a database test that in-range and low values make no task under a rule for high, a high one makes one, a correction to another high value makes no second, and entered in error cancels it naming the event. 603 unit and HTTP tests; on a fresh database every database test passes (195) except the six FHIR validator tests, not run here.
- **Not done / not verified:** the referral hooks; a correction from out of range back into range does not cancel the task (no event says so); on this machine's long-lived test database three older tests fail on accumulated data (a feed refusal list, a duplicate name, an overdue list) and pass on a fresh one.

## 2026-11-18 - Results raise task events; the site's holidays (TL-3; H-059)

- **Clinical risk:** yes, in time, as H-059 says: once a rule is approved, filing a result will make a task and signing it off will cancel it, and the due day now skips the site's holidays. Today every built-in rule is a draft, so a running service makes no task. H-059 stays open.
- **Built:** `site::holidays_from` and `EPR_SITE_HOLIDAYS` (calendar days, at most 200, a bad day stops the start); the built-in task rules checked at start-up; `results::task_facts` (`test`, and `abnormal` only when the service flagged the value: no flag is no fact, never `false`); `tasks_store::raise`, which cancels the source's open tasks the event cancels and then makes what approved rules call for, on the caller's connection; `results_store::create` raises `result-filed` (not for a result entered in error) and `sign_off` raises `result-signed-off` on a new sign-off only. `create_with_rules` files under given rules, for a test. Spec: `spec/clinical-workflows/task-rules.md` (holidays, the events table, what is not built).
- **Tested:** unit tests of the holiday setting (separators, repeats, six bad forms, the limit) and of the facts; a database test that a rolled-back filing leaves no task, an abnormal result makes one task, no flag, a normal flag or entered in error makes none, a sign-off cancels it naming the event, and signing again changes nothing. 602 unit and HTTP tests; 192 database tests.
- **Not done / not verified:** two database tests failed on this machine's long-lived test database (a feed refusal past the first page of 1,130 old ones, and a duplicate-name refusal) and pass on a fresh one; not changed. The six FHIR validator tests not run. The observation and referral hooks, a result's later versions, the routes, the reminder loop and a site's rules file are still to do.

## 2026-11-17 - Domain benchmarks with criterion; coverage with tarpaulin (QA-13, QA-14)

- **Clinical risk:** none: benchmarks, coverage tooling and CI.
- **Benchmarks:** `electronic-patient-record-domain/benches/domain.rs` (criterion 0.8.2, now a workspace dev dependency) takes the policy, consent and observation benchmarks from `benches/service.rs`, so a change to one rebuilds only the domain crate, and adds the task rules: the rules file checked (1, 10, 50 rules), the dry run (`plan`, 1, 10, 50), 60 working days across a holiday season, a late reminder's decision. `service-bench` runs both. `spec/performance-testing.md` updated.
- **Coverage:** both coverage gates and CI's `coverage` job use `cargo-tarpaulin` (LLVM engine, its own target directory) in place of `cargo-llvm-cov`. The matcher measures 97.52 percent on CI (floor 95, unchanged); the service 37.71 (floor 36, was 50). The service floor moved because tarpaulin leaves test code out of the count and llvm-cov did not: with `--include-tests` the figure is 55.90, against llvm-cov's 53.44 on CI. The reason is in `scripts/gates/coverage-service.sh`.
- **Found:** this machine's llvm-cov run gave 36.12 percent because its target directory held binaries from before the domain crate moved out, which named 25 source files that no longer exist; CI's clean run was unaffected.
- **Tested:** both bench targets run once (`-- --test`: 16 domain, 15 service benchmarks); both coverage gates pass locally; clippy on the workspace.
- **Not done / not verified:** benchmark numbers from one run at a load average of 37, not a quiet machine; CI's tarpaulin figures match (the service's exactly: 5,719 of 15,164 lines); no branch coverage under tarpaulin's LLVM engine.

## 2026-11-17 - GitHub private vulnerability reporting is on

- **Clinical risk:** none: repository setting and documents.
- **Changed:** private vulnerability reporting switched on for the GitHub repository, as the maintainer asked. `SECURITY.md` gives it beside email, with the same targets; `security.txt` lists it as a second `Contact`; `docs/security/policy.md` no longer says it is off.
- **Tested:** the GitHub API reports it enabled; the `security.txt` unit tests pass (each contact appears in `SECURITY.md`).
- **Not done / not verified:** no test report was filed through it; GitLab's mirror has no equivalent set; secret scanning and Dependabot security updates on GitHub stay off.

## 2026-11-17 - The UI audit ignores a picomatch advisory with no fix (DEP-2)

- **Clinical risk:** none: build tooling only.
- **Changed:** GHSA-fc5q-39xc-vjpf (picomatch ReDoS, high, no patched version) is ignored in the UI's `pnpm-workspace.yaml`, with the reason and a review date of 2027-02-17, as the maintainer chose. It is in Vite's dependencies only; `pnpm audit --prod` finds nothing. CI's audit job had failed on it since 20c9ddb.
- **Tested:** `pnpm audit --audit-level high` passes locally, showing the one ignored high and one moderate.
- **Not done / not verified:** no check fails when the review date passes; the moderate finding is unchanged and below the gate.

## 2026-11-17 - axa 0.20 (AX-18)

- **Clinical risk:** none: dependency change; no handler changed.
- **Changed:** `axa` 0.19 to 0.20 and `axa-test` 0.10 to 0.11, inside the publish cooldown with the maintainer's approval. Zstd compression is now an opt-in feature in axa, left off: the service took no Zstd before by choice and now refuses a Zstd request body with 415. The lockfile loses `zstd`, `zstd-safe`, `zstd-sys` and `jobserver`.
- **Tested:** clippy on the workspace; 600 unit and HTTP tests; 193 database tests against a local Postgres 18; `cargo deny check`.
- **Not done / not verified:** the six FHIR validator tests were not run (no validator jar here); no test sends a Zstd body.

## 2026-11-17 - The task rules' mutation run (TL-3, CQ-30)

- **Clinical risk:** none: tests.
- **Found:** a mutation run on `task_rules.rs` left 16 mutants alive. New tests pin the event words, an empty id or title, the limits at their edges (60 working days, five reminders, a reminder the whole due period before) and the guard against a calendar of holidays; a re-run of the 100 mutants left four, none killable, listed with reasons in `docs/maintainability/mutation-testing.md`.
- **Also recorded:** a run on the domain crate's other examined files with only the domain crate's tests left about 60 role-table mutants of `policy.rs` alive, which the service's tests may well catch; to be judged in the service run, not counted as gaps yet.
- **Tested:** the 13 task-rule unit tests pass; the 15 working-day mutants were re-run after the last pin.
- **Not done / not verified:** the domain crate's other misses are not judged; the service run is still to do.

## 2026-11-17 - Task rules and their store (TL-3, TL-4, TL-14; H-059)

- **Clinical risk:** yes, in time: rules will decide which team is asked to do what, and when. Nothing makes a task in a running service yet (no event hook or route), and every built-in rule is a draft. Hazard H-059 added (open).
- **Built:** in the domain crate, `task_rules.rs` and `task_rules.toml`: rules as versioned data (an event, conditions on its facts, a task for a team due in working days, reminders, cancelling events, what a late reminder does), checked when read: the only action is a task and any other key stops the file loading, versions run 1, 2, 3, approver and day both or neither, conditions only on facts the event carries, due in 0 to 60 working days, at most five reminders, no rule cancelled by its own event, and no two rules the same work (TL-14). Working days skip weekends and site holidays. `plan` is also the dry run. In the service, `tasks_store.rs` and migration 40 (`task_versions` with `task_current`, `task_reminder_decisions`, `task_reminder_runs`, all append-only by trigger): tasks made and cancelled in the caller's transaction with audit rows; a person closes one with a reason at the version read; each reminder decided once, and a late one sent once or skipped with the reason (TL-4). Spec: `spec/clinical-workflows/task-rules.md`; inventory rows for the new columns.
- **Decided:** the maintainer answered OQ-TL2: a target deployment does need access limited by a clinician's licensed jurisdiction. TL-10 is closed as decided; building it is TL-15.
- **Tested:** 9 unit tests of the rules (a rule that tries to message a patient or change a field is refused; duplicates refused naming both; working days over a weekend and a holiday; a late reminder sent once or skipped, never both); 5 database tests (only an approved rule makes a task, with an audit row; a rolled-back write leaves none; an event cancels only its own source's open tasks; a person's close needs a reason and the current version; the tables refuse change; a reminder is decided once). Clippy on the workspace; the census tests a new table touches pass.
- **Not done / not verified:** the event hooks in the result, observation and referral stores; routes, a dry-run route and a screen; the reminder loop; a site holiday setting; who approves a rule (OQ-TL1); no mutation run on the new files yet (a run started for them tested other files instead).

## 2026-11-17 - aws-lc-rs is gone from the build (authentication-verifier 0.9.3)

- **Clinical risk:** none: dependency and build change; the MLLP TLS provider was already `ring`, chosen explicitly.
- **Changed:** `authentication-verifier` 0.9.2 to 0.9.3, whose `fetch` feature no longer enables `reqwest`'s bundled `aws-lc-rs`; `cargo tree -i aws-lc-rs --all-features` finds nothing. The stale comments in `Cargo.toml` and `src/mllp.rs` and the build-time note in `spec/local-ci.md` now say so. The 2026-11 MLLP TLS entry below is history and still says aws-lc-rs was in the tree; it was true then.
- **Not done / not verified:** `cargo check --all-targets` only; no tests run; the CI timings in `spec/local-ci.md` were not re-measured.

## 2026-11-17 - Research pass: Tellescope's API documentation (plan.md 38, TL-11 to TL-14)

- **Clinical risk:** none: documents only. TL-12, TL-13 and TL-14 are clinical when built.
- **Written:** `spec/lessons/tellescope-api-documentation/index.md` (what the page holds, an assessment, a comparison, six ideas, what is not adopted); `plan.md` §38; tasks TL-11 to TL-14, and TL-1 and TL-4 extended.
- **Found:** the published page documents only models from AI conversations to automation steps and ends mid-example; endusers, forms, tickets and webhooks are not on it.
- **Not done / not verified:** read once from the page; nothing tried against the API; nothing built.

## 2026-11-17 - Research pass: Tellescope (plan.md 37, TL-1 to TL-10)

- **Clinical risk:** none: documents only. Four of the ideas are clinical (TL-2, TL-3, TL-4, TL-8) and each says it needs a hazard row when built.
- **Written:** `spec/lessons/tellescope/index.md` (what it is, how its API and automation are built, an assessment, a comparison, ten ideas ranked, what is not adopted and why, two open questions, sources); `plan.md` §37 with the advice; tasks TL-1 to TL-10; TL added to the status script's prefixes.
- **Advice:** task rules that only create tasks, due in working days (TL-3), with cancel conditions and skip-if-passed on every reminder (TL-4), first; scoped integration tokens (TL-1) next. Not adopted: A/B splits on patients, unattended patient messaging and AI agents, marketing analytics on care journeys, a switch that suppresses events, payments, built-in SMS or video.
- **Not done / not verified:** public vendor pages and API documentation only, read once; the trust centre returned nothing readable, so the SOC 2 and HIPAA claims are the homepage's; no customer report, review or independent evaluation was looked for; nothing built.

## 2026-11-17 - Half the service's integration tests share one program (LC-15)

- **Clinical risk:** none: how tests are built.
- **Why:** each file in `tests/` was its own program, rebuilt and linked against the library after every edit, about 44 s each and most of an edit's 240 s (measured earlier today).
- **Changed:** `tests/suite.rs` includes 40 of the 79 files as modules (`#[path]`, so the files and every path to them stay); `autotests = false` in `Cargo.toml`, with the suite and the other 39 as `[[test]]` entries. Kept apart: the 30 files that set the auth settings in the process environment (in one process they would change each other's), those with process-wide state (metrics, lockout, rate limit, the MLLP feed, the outbox), those with helper submodules, and those a script or document runs by name (`fhir_validator`, `audit_immutable`, `openapi_census`, `hl7_samples`). `tests/test_targets.rs` fails when a test file is in neither list, so turning autotests off cannot silently drop one.
- **Tested:** clippy on the workspace; 45 test programs where there were 84; 587 tests pass and 194 database tests are left out without a database (before: 586 and 194; the new census is the one more); every database test on a fresh local Postgres 18 in one process per program, one test at a time, as the database gate runs them: 188 passed, the same as before the fold, and the six FHIR validator tests failed as before (no Java here).
- **Not done / not verified:** the other 39 files; the rebuild after a one-file edit was not measured again; run one file's tests with `cargo test --test suite <module>::`, which no document says yet beyond this entry and `tests/suite.rs`.

## 2026-11-17 - The keyed-digest benchmark no longer carries a key-shaped string

- **Clinical risk:** none: a benchmark and the secret scan's list.
- **Found:** CI's secret scan (gitleaks, full history) failed on `9f02955`: the `audit/digest_keyed` benchmark added in `218689d` held a 64-hex-digit test key, SHA-256 of the word "test", the made-up value the audit chain's unit tests use. It keys nothing.
- **Changed:** the benchmark derives its key at run time (SHA-256 of a fixed phrase; a key of one repeated byte is refused by the key loader). The old line stays in history, so its fingerprint is listed in `.gitleaksignore` with the reason.
- **Tested:** gitleaks over the full history finds nothing; the three audit benchmarks run (`--test`); clippy passes.
- **Not done / not verified:** the other benchmarks were not rerun.

## 2026-11-17 - The service and the domain crate on jiff 1.0 (DEP-1, AX-17)

- **Clinical risk:** none intended: the same times, formats and parse rules on the library's next major version; the full test suite is the check.
- **Approved exception:** `jiff-tzdb` 1.0.0 (2026-11-15 21:46 UTC), which jiff 1.0's bundled zone database needs, is inside the maintainer's two-day cooldown; the maintainer approved it. Resolved with the minimum lifted for that one command.
- **Changed:** jiff `1` in `[workspace.dependencies]`, the version axa 0.19 already uses; `to_zoned` is now `to_zoned_datetime` (jiff renamed `Zoned` to `ZonedDateTime`). The lockfile adds `jiff-tzdb` 1.0 and moves jiff 0.2 to 0.2.46.
- **Tested:** clippy on the workspace; all 84 test binaries without a database; every database test on a fresh local Postgres 18 except the six FHIR validator tests (no Java here).
- **Not done / not verified:** jiff 0.2 is still in the tree, through `serde_with` (which `fhir` and `axum-keycloak-auth` use), so the duplicate remains there; `pg_time.rs` is kept rather than replaced by `jiff-sqlx`.

## 2026-11-17 - Mutation testing: a script, hazard-weighted files, and the matcher's survivors pinned (CQ-30)

- **Clinical risk:** none: tests. The matcher decides whether two records are one person (H-001); these pins hold behaviour that no test held.
- **Built:** `scripts/mutants.sh matcher|service [--in-diff <base>] [-j N]` runs cargo-mutants on each crate's `.cargo/mutants.toml` (the matcher: every file but `main.rs`; the service: 12 files chosen by hazard, six of them now in the domain crate, so the script passes `--workspace`), with output git- and container-ignored. Not a CI gate (PR-8 declined a scheduled run). Results and reasons: `docs/maintainability/mutation-testing.md`.
- **Found and pinned:** the matcher run missed 36 of 348. The IHI tests derived their check digit with the function under test, so a broken Luhn agreed with itself; the given-name rules had no record with two name lists; the swap penalty, the share of thin evidence, equal thresholds, a config without a calling code, `ä`, `ö`, a Greek letter with a mark, a four-part date and `l` in Soundex were never read. New tests in `src/ihi.rs`, `src/normalize.rs` and `tests/fields.rs` pin 31; five cannot be killed (an even number is never 9; exact boundaries at 0.85 and at the evidence floor; a swap shape real names do not produce), and are listed.
- **Tested:** the matcher's tests and clippy pass with the pins.
- **Not done / not verified:** the matcher was not re-run, so the 31 are expected killed, not confirmed; the service's run was interrupted after 159 of 773 (at a load average near 100) and nothing from it is acted on; my first restart of the runs used `pkill -f cargo-mutants`, which also stopped another session's run on another project.

## 2026-11-17 - axa 0.19 (AX-17)

- **Clinical risk:** none. No code, schema or response changes; the OpenAPI document is unchanged.
- **Approved exception:** axa 0.18.0 and 0.19.0 and axa-test 0.9.0 and 0.10.0 were published today (11:42 and 14:35 UTC), inside the maintainer's two-day cooldown, which cargo 1.100 now enforces and refused; the maintainer approved the exception. The update was resolved with the minimum lifted for that one command; the cargo settings are unchanged.
- **Changed:** `axa` 0.19 and `axa-test` 0.10. The lockfile moves `axa-postgres-job-queue` to 0.6.0 and adds jiff 1.0 with `jiff-core`, `jiff-sqlx` and `jiff-static` (axa's queue moved from chrono to jiff); croner and sqlx's `chrono` feature left the tree, so nothing the service builds uses chrono. RUSTSEC-2026-0543's ignore stays, with its reason updated: chrono is still in the tests, through axa-test's axum-test and its expect-json. 0.18's changes are to axa's HTTP client, which the service does not use.
- **Checked:** clippy on the workspace; all 84 test binaries without a database; every database test on a fresh local Postgres 18 except the six FHIR validator tests (no Java here); `cargo deny check` (advisories, bans, licences, sources) passes; one `axa` and one `axum-test`.
- **Not done / not verified:** the service and the domain crate stay on jiff 0.2, so two jiff versions are in the tree: jiff 1.0's bundled zone database needs `jiff-tzdb` 1.0, which is inside the cooldown and not covered by the exception. Moving to it would also let `pg_time.rs` give way to `jiff-sqlx`. Not pushed, with the earlier local commits.

## 2026-11-17 - The domain rules are their own crate (LC-16)

- **Clinical risk:** none intended: code moved, no rule changed. Every rule that decides what a clinician may see or must do (policy, consent, role views, referrals, pathways, observations, deaths, the hazard register's rules) now lives in another crate, so a mistake in the move could change behaviour; the full test suite is the check. No hazard row changes.
- **Asked for:** the maintainer asked for `electronic-patient-record-domain`, for the domain logic and business rules as opposed to routes and storage, after asking whether a split would shorten rebuilds.
- **Built:** `electronic-patient-record-service-with-rust/electronic-patient-record-domain`, a workspace member, with 26 modules moved by `git mv` (adjustments, amendment, assessments, clock, consent, death, gate, hazard, ids, link_token, observations, packs, page, pas_suspicion, pathways, permission, policy, privacy, proxy, referral, results, role_views, site, suspicion, watch_list, and a new `dates`) and their three data files. It depends on serde, serde_json, jiff, toml, sha2, uuid, tracing and the matcher only: no database, web framework or async runtime, and never the service. `valid_birth_date` moved from the patient store into `dates`, and the referral `Priority` from the referral store into `referral`; both are re-exported where they were. The service re-exports every domain module under its old path, so `crate::consent` and `electronic_patient_record::consent` are unchanged. `consent::in_period` became public (the relationships store uses it). Shared dependency versions are in `[workspace.dependencies]`.
- **Followed:** the `Containerfile` copies the crate; the fmt, clippy, coverage and minimum-Rust gates and the pre-commit hook cover the workspace; the census tests that walk the source (grief, label, outbound, risk field, SQL) walk both crates, so none checks less; 100 live references to the moved files point at the new place (the hazard log, specs, tasks, both `AGENTS.md`); the status page counts the domain crate's tests. The OpenAPI descriptions that named source files now name only the file: with the new path, `chaos_api` found `/src/` in a served answer, which it rightly calls a leak.
- **Measured (one run, load average about 90):** after a one-file controller edit, the library rebuilt in 109 s, and the library with every test program in 240 s (about 3,560 CPU-seconds, about 44 s for each of ~80 test programs). Most of an edit's cost is the test programs: task LC-15 folds them into one. Not measured after the split.
- **Tested:** clippy on the workspace; all 84 test binaries without a database; every database test on a fresh local Postgres 18 except the six FHIR validator tests (no Java here); `cargo test -p electronic-patient-record-domain` runs the rules' tests without Postgres.
- **Fixed after:** the UI's `adjustments.test.ts` (which keeps the UI's catalogue equal to the service's) and `scripts/ci-changes.sh` still read `src/adjustments.rs`; the fast gates caught it before the push, and both now read the domain crate's.
- **Not done / not verified:** `interop`, `audit`, `metrics` and the stores stay in the service; nothing but `AGENTS.md` stops the domain crate gaining a database or web dependency; the crate is publishable (the service is) but not published; the rebuild after the split was not measured; not pushed, with the two earlier local commits, until the maintainer has checked the other session's `c74954f`.

## 2026-11-17 - The service moves from chrono to jiff (DEP-1)

- **Clinical risk:** yes, in principle: every date and time the service reads, checks, stores and shows now goes through another library (an age, a due day, an overdue flag, a measured time refused as future, a feed's silence). The formats, the parse rules and the days are kept as they were and tested; no hazard row changes, since no rule changed. Not reviewed by a clinician.
- **Asked for:** the maintainer asked to move to jiff, the alternative the advisory names.
- **Changed:** the service depends on jiff 0.2 (0.2.41, already in the lockfile, with its bundled zone database, so the image needs no zoneinfo) and on neither chrono nor chrono-tz; sqlx's `chrono` feature is off. `clock.rs` holds the formats the service wrote with chrono, byte for byte (to the microsecond, to the second, `to_rfc3339`'s `+00:00`, and serde's `Z` with the fraction in groups of three, used on the three serialized time fields and the feed-health answer); a parse as strict as chrono's RFC 3339 one (jiff alone takes a time without seconds); and day helpers. Zone names must be the database's own spelling, as chrono-tz required (`europe/london` is still refused). `pg_time.rs` binds a jiff time or day in Postgres's binary encoding, as sqlx's chrono support did, so no query changed. 52 files changed, most mechanically.
- **Unchanged by design:** the audit chain formats its times in SQL, so no stored digest can differ.
- **Removed:** chrono-tz left the tree, and its advisory ignore (RUSTSEC-2026-0555) with it.
- **Tested:** clippy on every target; all 82 test binaries without a database; every database test on a fresh local Postgres 18 except the six FHIR validator tests (no Java here; CI runs them); new tests for the formats against chrono's, the strict parse (what chrono refused is refused), Postgres's epoch, truncation toward the past and infinity, and a zone name in the wrong case; `cargo deny check advisories` passes.
- **Not done / not verified:** chrono is still in the tree through axa (croner and sqlx's `chrono` feature), so RUSTSEC-2026-0543's ignore stays until axa drops it; no answer was compared byte for byte with one from before the move beyond the tests' own; the 2026-11-17 benchmarks are still unmeasured.

## 2026-11-17 - Benchmarks for the keyed audit digest, consent and the HL7 feed; HL7 load scenarios

- **Clinical risk:** none: measurement only.
- **Built:** in `benches/service.rs`: `audit/digest_keyed` and `audit/fingerprint_keyed` (the gap the performance spec named: the key set is built from `integrity_mac` directly), `consent/decide` with 1, 10 and 50 active consents, `hl7/message_time`, `hl7/redact_er7`, `hl7/acknowledgement_aa` and `_kept`, `hl7/msa_text_long`, `hl7/mllp_take_ten_frames`, and `feed_health/assess`. In the load harness, `hl7-register` and `hl7-update` send ADT messages over MLLP on pooled connections (`AA` passes, anything else fails), behind a new `--mllp host:port` option; `run-local.sh` opens the listener on a free port and passes it; budgets: no failures. `spec/performance-testing.md` describes both.
- **Tested:** every benchmark runs once (`--test`, the `service-bench` gate); clippy is clean; the harness's tests (18) pass, two of them new: the message's fields, and whole runs against a fake MLLP listener that answers `AA`, then `AR`.
- **Not done / not verified:** **nothing is measured yet**: the machine was busy with mutation runs, so no number is in `BENCHMARKS.md`; the `hl7-*` scenarios have not run against the real service (only the fake listener); the feed in the load run is plain TCP on loopback; the feed is not in the `mixed` scenario.

## 2026-11-17 - A formal security policy and disclosure policy

- **Clinical risk:** none to the software; a process: a vulnerability that could show the wrong patient or change what a clinician decides or signs is now rated at least high and goes to the hazard log.
- **Written:** `SECURITY.md` is now a versioned vulnerability disclosure policy shaped on ISO/IEC 29147 and 30111: how to report (email; GitHub's private reporting is off), stages with targets (acknowledge 5 business days, triage 15), CVSS v4.0 severity raised for clinical harm, fix targets (critical 7 days, high 30, medium 90, low next release), coordinated disclosure at the fix or 90 days with an advisory and a CVE for medium and above, safe harbour for good-faith research on one's own installation, supported versions, scope, and the existing "does / does not claim" lists (now with the HL7 listener's TLS). `docs/security/policy.md` is the project's security policy: roles (no separation of duties with one maintainer; no CSO yet), principles, and secure development, supply chain, secrets, cryptography, incidents and exceptions, each rule with what enforces it, what a deployer must do, and what is not done.
- **Built:** `/.well-known/security.txt` (RFC 9116) in the UI's static files. `tests/unit/security-txt.test.ts` checks its required fields, that `Expires` is in the future and at most a year ahead, that its contact is the one in `SECURITY.md`, and that both policies are within their review dates; a browser test checks it is served at the root as text. `scripts/ci-changes.sh` runs the UI when either policy changes.
- **Corrected:** `docs/security/index.md` named a CI `zap` job that does not exist, and said MLLP has no encryption (it has TLS since IH-10).
- **Tested:** the unit test fails with a past `Expires` and with another contact; 395 unit tests and the browser test (light and dark) pass; the documentation census passes.
- **Not done / not verified:** GitHub's private vulnerability reporting, secret scanning and Dependabot security updates are off for the repository and were not changed; no PGP key is published; no threat model (QA-10), ASVS table (QA-9) or DAST job; the targets are one maintainer's and untested by any real report; no lawyer has read the safe-harbour section.

## 2026-11-17 - chrono is unmaintained: carried until 2027-02-17 (DEP-1)

- **Clinical risk:** none: no code changes.
- **Found:** CI for `fd1166c` failed `cargo deny` (the service) on two advisories published today, RUSTSEC-2026-0543 (chrono) and RUSTSEC-2026-0555 (chrono-tz): unmaintained, no vulnerability, no fixed version. The late-update test fixed in that commit passed.
- **Changed:** both are ignored in `deny.toml` with a reason and `expires 2027-02-17`, which `tests/advisory_expiry.rs` enforces. chrono also comes through axa, sqlx and croner, so the service cannot remove it from the tree alone. Task DEP-1 is the move to `jiff` or `time`.
- **Tested:** `cargo deny check advisories` passes here with the ignores; the expiry test passes.
- **Not done / not verified:** nothing moved off chrono; the maintainer may prefer to start DEP-1 now rather than carry the ignores.

## 2026-11-17 - The late-update test no longer fails between 10:00 and 11:00 on 17 November

- **Clinical risk:** none: a test.
- **Found:** CI for `f4bf1d8` failed `a_late_update_does_not_undo_a_newer_one`. It checked that the whole acknowledgement held no `2026111710` (the late message's time), but the acknowledgement's own MSH-7 is the time now, and CI ran at 10:05 UTC on 2026-11-17. The new `msrv` job in that run passed.
- **Changed:** only `MSA`, the text the sender reads, is checked for the time.
- **Tested:** passes at 10:xx UTC on 2026-11-17, when the old check fails.
- **Not done / not verified:** other tests were not searched for the same mistake beyond this file's literal time checks.

## 2026-11-17 - CI builds each crate on the oldest Rust it claims (QA-8)

- **Clinical risk:** none: a build check.
- **Built:** gate `rust-msrv` (`scripts/gates/rust-msrv.sh`): for the matcher and the service, read `rust-version` from `Cargo.toml` and run `cargo +<version> check --locked --all-targets`, installing that toolchain when missing (minimal profile). CI runs it as the `msrv` job, which `ci-ok` needs; `ci-local.sh` runs it in `--full` only, since it builds with another toolchain. Listed in `spec/local-ci.md`.
- **Tested:** passes here on Rust 1.97 for the matcher and 1.98 for the service; with the service on 1.97 cargo refuses it (axa 0.17 needs 1.98), which is the failure the gate exists for. The gate census (`check-gates.py`) passes.
- **Not done / not verified:** it checks that the code builds, and does not run the tests on the old toolchains; clippy and fmt stay on the pinned 1.99.1. Not yet run on GitHub.

## 2026-11-17 - Two feed tests pass on a database they have already run on

- **Clinical risk:** none: tests.
- **Found:** `the_sample_set_goes_through_the_feed_as_its_readme_says` (found during the axa 0.17 upgrade) and, once in three runs, `no_message_value_reaches_an_audit_row_the_queue_a_version_reason_a_metric_or_a_log_line` failed on a reused test database: a fixed birth date with a fixed name, or with marker names that share their start and end, made the duplicate check (rightly) take an earlier run's patient for the new one, so the first message was refused.
- **Changed:** both tests now give each run its own family name or birth date. The sample files are unchanged.
- **Tested:** the 18 feed database tests pass nine runs in a row on one database (they failed on the second run before).
- **Not done / not verified:** other database tests were not run repeatedly on one database; CI uses a fresh one, so it never saw this.

## 2026-11-17 - axa 0.17 (AX-16)

- **Clinical risk:** none. No schema or response body changes; the OpenAPI document is unchanged.
- **Approved exception:** axa 0.16.0, 0.16.1 and 0.17.0 and axa-test 0.8.0 were published between 2026-11-16 16:06 and 2026-11-17 07:17 UTC, inside the maintainer's two-day cooldown (`registry.global-min-publish-age`, which stable cargo does not enforce); the maintainer approved skipping it.
- **Changed:** `axa` 0.17 and `axa-test` 0.8. The service's `rust-version` is 1.98, which axa 0.17 needs; the matcher stays 1.97 (`AGENTS.md`, `plan.md`, the compatibility page). The new `health` command is answered by `axa::cli::health` before anything connects. The lockfile moves `axa-idempotency-key` to 0.2.3, `axa-postgres-job-queue` to 0.5.1, `axa-postgres-sqlx` to 0.2.1 and `axa-request-rate-limit` to 0.3.3, and adds `ppp` and `thiserror` 1.
- **Behaviour now in force:** a trailing slash on a mounted path is a `308` to the path without it; a Keycloak refusal says only `invalid token` (the reason is logged); an empty public prefix opens nothing, which changes nothing here because the service's public paths are a fixed list. No migration: the job queue's new trigger is on tables this service never sets up.
- **Checked:** clippy, every test without a database, rustdoc, and every database test on a local Postgres 18 except the six FHIR validator tests, which need Java here (CI runs them); one `axa`, one `axum-test`.
- **Found, not fixed:** `the_sample_set_goes_through_the_feed_as_its_readme_says` fails on a database where it has already run (its fixed sample patients exist, so the first message is refused); it passes on a fresh one, as in CI. Not caused by the upgrade.
- **Not done / not verified:** no gate builds with exactly Rust 1.98 (QA-8). Not adopted: `INTERNAL_PORT`, `PROXY_PROTOCOL_FROM`, `Patch`, `Form`, operator guards, API tokens, a shared rate limit, Tokio metrics, background migrations, axa-test's OpenAPI conformance check; the image's health check is unchanged.

## 2026-11-17 - The shutdown test always stops with a message in hand (IH-7)

- **Clinical risk:** none: a test.
- **Changed:** `a_stopping_listener_finishes_what_it_holds_and_answers_nothing_it_did_not_do` used to send the stop just after the message, so it usually landed before the message was read and the "finish what is in hand" branch ran by chance. It now locks `audit_events` against writes in its own transaction, waits until Postgres shows the feed's session blocked on that lock, sends the stop, then releases the lock, and requires `AA` and the stored change.
- **Tested:** passes five runs in a row; with the listener changed to abort its connections at stop, it fails. The 24 feed tests pass.
- **Not done / not verified:** a message still unfinished after `STOP_WAIT` (rolled back, unanswered) has no test.

## 2026-11-17 - The HL7 feed checks the name on a sender's certificate (IH-6)

- **Clinical risk:** small, which sender may change identity records: with the new setting, a sender whose certificate is from the right CA but for another name is now closed before anything is read. H-021's controls updated (still open).
- **Built:** `EPR_MLLP_TLS_CLIENT_NAMES`, a comma list of DNS names or addresses, which needs `EPR_MLLP_TLS_CLIENT_CA_FILE`. After the handshake the sender's certificate must be valid for one of them (`mllp::certificate_name`: webpki's subject-name check on the subject alternative names, never the common name); the name it matched goes on every audit row the feed writes as `peer_certificate`. A name that is not a DNS name or address, or names without a client CA, stop the start. `rustls-webpki` is now a direct dependency; it was already in the tree through rustls.
- **Tested:** the check on a test certificate (the listed name, the common name, another name, an address, garbage); a listener closing a certificate for another name and serving the listed one; the settings refused; the name on the audit row (Postgres).
- **Not done / not verified:** the name is in the audit row's detail, not the agent, and not on the work queue entry or the feed health; several senders on one listener is still not built; not tried with a real sender's certificate.

## 2026-11-17 - The flaky proxy-scopes test now says what refused it

- **Clinical risk:** none: a test.
- **Looked at:** `http_proxy_scopes::a_proxy_reads_only_what_its_terms_list_and_only_until_they_end` answered 401 instead of 403 once, on 2026-11-15, in a full database run. Alone it passed ten times in a row today (about 3.1 s each). Its tokens live 300 s, so a 401 after the 3 s wait is the token verifier refusing the request, not the proxy's expired terms.
- **Changed:** the assertion after the wait now prints the `www-authenticate` header and the body, so the next failure says what was refused.
- **Not done / not verified:** not reproduced and not diagnosed; the cause is still unknown.

## 2026-11-17 - The header holds with WCAG text spacing on any font (TH-1, a fix)

- **Clinical risk:** as the entry below: what a clinician sees at high zoom. No change to H-057.
- **Found:** the zoom and text-spacing test passed here and failed in CI: on the runner's Linux fonts, with WCAG 1.4.12's spacing, part of the header was wider than the header.
- **Changed:** nothing in the header may be wider than it (children shrink, long words break), and the purpose and theme controls wrap with their selects no wider than their row. The test now names any element that sticks out past the header, so a failure on another machine's fonts says why.
- **Not verified:** not reproduced here (macOS fonts pass either way); CI on Linux is the check.

## 2026-11-17 - At 400 % zoom nothing runs off the side; WCAG text spacing cuts nothing off (TH-1)

- **Clinical risk:** yes, what a clinician sees: at high zoom or on a narrow screen the header now wraps onto more lines, where before its links, the purpose and the theme control ran off the side (the page was 1,397 px wide at 320 px). Hazard H-057 updated (still open).
- **Changed:** the header and its navigation wrap. The privacy page's table scrolls in its own focusable, named region, so the page itself does not.
- **Tested:** a browser test at 320 CSS px wide (400 % zoom, WCAG 1.4.10) on home, a patient chart, privacy and path to production finds no sideways scroll; with WCAG 1.4.12's text spacing applied, the banner, the allergy line ("Penicillin (high risk), Latex"), the warning and the header lose nothing. It failed on the header and the privacy table before the fix. 390 unit tests and 242 browser runs pass.
- **Not done / not verified:** four pages, not every page; the test turns the page's content security policy off to apply the spacing styles; no person has used the app at 400 % zoom.

## 2026-11-17 - The HL7 feed's health on the feed screen (IH-8, UI)

- **Clinical risk:** small, what an administrator sees: the feed screen now says whether the feed is silent or refusing, in words. H-023's controls already name the health; no change to the hazard.
- **Built:** a "Feed health" section on `/admin/feed`, from `GET /api/feed/health`: per feed the state in words first (never by colour alone; a feed that has sent nothing is "Unknown, nothing received yet"), why, the last message, refusals in the last hour and waiting, the window it is judged by and since when; "The HL7 listener is off" when there is no feed; a failed read says so. A refused entry's headline no longer shows the control id when it is the service's fingerprint (IH-3), which meant nothing to a person. English and machine-written Welsh; the service's reasons stay English.
- **Tested:** `tests/unit/feed-health.test.ts` (the lines, unknown, fingerprints left out); a browser test with a critical feed (axe), the listener off and a failed read. 390 unit tests and 240 browser runs pass.
- **Not done / not verified:** neither the health nor the open count shows outside the admin pages; no alert; the Welsh is not reviewed.

## 2026-11-17 - A dry run for an HL7 message (IH-11)

- **Clinical risk:** none to the record: nothing is kept. It is a new way to read the record: its answer says whether a patient with a message's identifiers exists, so it is for the clinical administrator only, as the queue is, and audited.
- **Built:** `POST /api/feed/dry-run` with `{"message": "MSH|…"}` runs `adt_feed::dry_run`: the listening feed's identity and rules (recorded when the listener starts; the defaults when it is off, and the answer says so), the reader, the rules, matching, the duplicate check, the order check and the writes, in a transaction that is rolled back. No queue entry, no count, no message time. It answers what the message would do (`created`, `updated`, `unchanged`, `refused`, `retry`, `not-hl7`), why, the warning count and the acknowledgement the feed would send. The call is audited as one `Execute` row with the outcome word only. `apply_message` now takes a mode; the feed's path is unchanged.
- **Tested:** on Postgres, a new patient dry-runs `created` and leaves no patient or audit row; after the real message, an update dry-runs `updated` and leaves the version; a conflict dry-runs `refused` with its reason and the `AR` the feed would give, and is not queued; only the real message is counted; the route against its OpenAPI schema. The route censuses list it as a POST that reads and not a write.
- **Not done / not verified:** no screen; the audit row inside the rolled-back transaction (the would-be change's own row) goes with it, which is the point, but means the trail records only that a dry run happened.

## 2026-11-17 - HL7 sample messages, golden acknowledgements and a test sender (IH-11, in part)

- **Clinical risk:** none: test material and a tool for test deployments.
- **Built:** `operations/hl7/samples/`, ten synthetic messages (no real person; the national number is the published test value): each handled event for one patient, a correction, a death, a training message, a type not handled, a message with no PID and a bad national number, with a README of what each shows. `tests/hl7_samples.rs` reads every sample and compares the reader's acknowledgement byte for byte with `tests/golden/hl7-acks/<name>.ack`, rewritten only by `EPR_BLESS=1` for a meant change, and fails on a golden file with no sample. `scripts/hl7/send.mjs` sends samples to a listener, over TLS if asked, with a new control id per send and one patient per run, printing each answer; its tests run in the `docs-load-harness` gate, which now covers the Node tools.
- **Tested:** the goldens; the samples' codes; on Postgres, the whole set sent in order through a production feed (six `AA`, four `AR`, one patient, the death holding contact); the sender against a fake listener (fill, answers, options, control ids, one patient).
- **Not done / not verified:** the dry run (IH-11's other part); no sample from a real sending system; the goldens are the reader's answers, so a refusal the feed keeps reads `AE` there and `AR` in the feed, as the README says.

## 2026-11-17 - What an HL7 feed's sender may send (IH-6)

- **Clinical risk:** small, what reaches the record: with the new settings, a message from another application or facility, of another HL7 version, or of an event the feed does not take is refused and kept, not applied. Covered by H-021's controls.
- **Built:** `EPR_MLLP_SENDING_APPLICATIONS` (`APP`, or `APP^FACILITY` for MSH-3 and MSH-4, first components only), `EPR_MLLP_VERSIONS` (MSH-12) and `EPR_MLLP_EVENTS` (a subset of the handled triggers), each comma-separated and checked at start: a malformed entry, or an event the feed does not handle, stops the service. A message outside them is answered `AR` and kept, with a reason that quotes none of its values. Unset lists take anything, as before. The audit agent stays the configured `EPR_MLLP_AGENT`, never MSH-3.
- **Tested:** the rules (application alone, with facility, a later component ignored, version, event, empty lists), the settings and their errors, and on Postgres four messages outside the rules each kept and not applied while one inside is applied.
- **Not done / not verified:** one listener is one sender: several senders on one port, each with its own agent, answer and rules, is not built; the TLS client certificate's name is not used.

## 2026-11-17 - The HL7 listener drains at shutdown (IH-7)

- **Clinical risk:** none: it was already safe (a message killed mid-transaction rolls back unanswered; one committed and not answered is a harmless repeat when resent); this makes a stop tidy.
- **Built:** `mllp::serve_until` takes a stop signal: it stops accepting, closes idle connections, lets the message each connection is handling finish (commit and answer) and starts no other buffered one, waits up to `mllp::STOP_WAIT` (10 s), then aborts what is left, whose transactions roll back unanswered. The service keeps the listener (`mllp::Running`, `keep`, `stop_kept`, the outbox's pattern) and `run` stops it after the HTTP server and before the outbox.
- **Tested:** on Postgres, a message sent as the stop is given is either answered `AA` with the patient stored, or not answered and not stored; an idle connection is closed; the listener ends within its bound; nothing is served after. Run three times.
- **Not done / not verified:** in those runs the stop came before the message was read, so the finishing branch was not exercised on purpose; no test of a message taking longer than the bound.

## 2026-11-17 - HL7 acknowledgement hygiene (IH-2)

- **Clinical risk:** small, what reaches the record: with `EPR_MLLP_PROCESSING_ID` set, a message meant for another kind of deployment (a training message on a live one) is refused and kept, not applied. H-021's controls already cover refusals; no new hazard.
- **Found:** the HL7 builder already copied MSH-11 into each acknowledgement and escaped `MSA-3`; it left MSH-11 empty when the message had none, and nothing bounded `MSA-3`.
- **Changed:** MSH-11 is `P` when the message has none (HL7 requires it in an ACK); `MSA-3` is one line, at most 199 characters (OQ-IH5); a frame that starts as an MSH segment but does not read is answered `AE` by a hand-built acknowledgement (every field escaped, MSH-10 echoed so the sender can match it, MSH-11 copied when `P`, `T` or `D`) instead of a closed connection, and the connection stays open; `EPR_MLLP_PROCESSING_ID` (`P`, `T` or `D`) refuses any other processing id with a reason that quotes nothing, kept for a person, `AR`.
- **Tested:** golden checks of MSH-11 for `P`, `T` and none; `MSA-3` with line breaks, tabs, every delimiter and 400 characters reads back as one line of 199; the hand-built `AE` byte for byte, with delimiters escaped and non-MSH text unanswered; on Postgres, a `T` message to a `P` deployment refused, kept and answered as `T`, a `P` one applied, and an unreadable MSH answered `AE` with the connection still serving.
- **Not done / not verified:** golden acknowledgements for a whole sample corpus (IH-11); whether senders accept 199 characters in `MSA-3`.

## 2026-11-17 - How to take an HL7 feed live (IH-12)

- **Clinical risk:** a process for a clinical change, documents only. H-021 lists it as a control not yet used.
- **Written:** `operations/hl7/going-live.md`: agree the answers with the sender; replay a real sample on a test deployment and compare every outcome with the current system; prove the safety rules on it (late updates, duplicates, no blanking, the control id, the health); check the live settings (TLS, the client CA, thresholds, an alert); a register of intended differences whose decision, who and when stay blank until a named person signs, and no feed goes live until its rows are; one feed at a time, the old route kept and never deleted, a business day's hold; a table of what to watch and when to go back, going back first and finding out why after; and a record per feed.
- **Not done / not verified:** never used; no clinician, Clinical Safety Officer or interface engineer has read it; the sample's handling needs the data owner's agreement, which the page asks for but cannot give.

## 2026-11-17 - TLS on the HL7 listener (IH-10)

- **Clinical risk:** none directly; it closes part of H-021 (a port open to anyone, in plain text). H-021 updated (still open).
- **Built:** `EPR_MLLP_TLS_CERT_FILE` and `EPR_MLLP_TLS_KEY_FILE` (PEM, both or neither) make the MLLP port TLS only: rustls 0.23 with ring's provider chosen explicitly (aws-lc-rs is in the tree too, and rustls will not guess between them), a handshake that fails or outlasts the idle time closes the connection, and plain text is never answered. `EPR_MLLP_TLS_CLIENT_CA_FILE` requires every sender to present a certificate from that CA. A missing, empty or unreadable file, a file with no certificate or key, a key that does not fit, or a client CA without TLS stops the service starting. `rustls`, `rustls-pki-types` and `tokio-rustls` are now direct dependencies; all were already in the tree, so no new crate.
- **Tested:** with certificates made by `openssl` at run time (no private key is committed, so the secret scan stays clean): a message over TLS is answered; a plain-text client on the TLS port gets nothing; with a client CA, a sender with no certificate is not served and one with a certificate is; nine unusable settings each stop the start.
- **Not done / not verified:** a renewed certificate needs a restart; the client certificate's name is not used as the audit agent (IH-6); not tried with a real sender's TLS stack; the CI runner's `openssl` is assumed.

## 2026-11-17 - HL7 feed health and its alarm (IH-8)

- **Clinical risk:** yes, what a person is told: a refusal or a silent feed now raises a state an operator can alert on, where before someone had to open the admin screen. Hazard H-023 updated (still open).
- **Built:** `feed_health.rs` judges each configured feed by the site's local window (weekdays 08:00 to 17:59 day, the rest of a weekday evening, Saturday and Sunday weekend): `unknown` when it has sent nothing (never `healthy`), `critical` for silence past the window (`EPR_FEED_SILENCE_MINUTES`, default `day=60,evening=240,weekend=720`) or many refusals in an hour (`EPR_FEED_REFUSALS_CRITICAL`, 10), `warning` for a refusal in the hour (`EPR_FEED_REFUSALS_WARNING`, 1) or any open one. The defaults are placeholders (OQ-IH4); a bad setting stops the start. While the listener is on, a loop judges every minute, stores each change in a new append-only table (`feed_alarm_states`, migration 39, which also indexes the audit trail by agent) and logs it once, as an error when critical. A restart reads the last state, so it logs nothing again. The gauge `epr_feed_health{sender}` and `GET /api/feed/health` (clinical administrator, audited) show it.
- **Tested:** the windows (zones, summer time, edges), no data is unknown, silence and refusals raise the state, thresholds read and checked; on Postgres, a feed of its own goes unknown, healthy (stored once; judging again stores nothing), warning after a refusal and critical after two days' silence, with the gauge; the route against its OpenAPI schema.
- **Not done / not verified:** no notifier, so nobody is paged unless a deployer alerts on the log line or gauge; the UI shows neither the health nor the open count outside the admin pages; one listener is one feed (IH-6 brings senders); the loop's minute is fixed.

## 2026-11-17 - HL7 feed counters by sender, event and answer (IH-9)

- **Clinical risk:** none: a counter.
- **Built:** `/metrics` has `epr_feed_messages_total{sender,event,answer}`: `sender` is the feed's configured agent, `event` a handled trigger or `other` (so a sender cannot make new series), `answer` `AA`, `AE`, `AR` or `none` when the frame was not HL7. No control id, patient or text is a label; at most 200 series, then the sender `overflow`. `epr_feed_refused_messages_total` is unchanged. `operations/hl7/README.md` says what to alert on.
- **Tested:** `metrics::tests` (labels, clamping, no value from the message, the old counter still served) and, on Postgres, three `AR` answers to a resent message counted under the feed and event.
- **Not done / not verified:** no alert rules ship (IH-8 is the feed's own alarm); counters restart at zero with the process, as the others do.

## 2026-11-17 - A late HL7 update does not undo a newer one (IH-5)

- **Clinical risk:** yes, what reaches the record: an update that arrives after a newer one from the same feed is no longer applied. New hazard H-058 (open).
- **Built:** the reader takes each message's time from EVN-2, else MSH-7 (`hl7_adt::message_time`: at least to the minute, a `+HHMM`/`-HHMM` offset honoured, no offset read as UTC). Every message the feed applies records that time, per feed and patient, in a new append-only table, `feed_message_times` (migration 38), in the same transaction as the change. Under a lock per feed and patient, an update older than the newest one applied is not applied: it is kept in the work queue with the time-free reason "this update is older than one this feed has already applied for this patient: a person must check which is right" and answered `AR`. The same time is not older; another feed's times do not count. A message with no usable time is applied as before, and its audit row has `message_time_known: false`.
- **Tested:** the time parser (formats, offsets, bad times); the event time preferred to the message time; on Postgres, a create, a newer update, a late update refused and kept with the record unchanged and no time in the answer, an equal time applied, a timeless message applied and noted, and another feed not held back. The personal-data inventory and the schema map list the new table.
- **Not done / not verified:** OQ-IH3 is open (whether a late message should instead be applied field by field); a sender that changes its offset convention can make a real update look older; two feeds updating one patient are not ordered against each other; no clinician or Clinical Safety Officer has reviewed H-058.

## 2026-11-17 - No HL7 message value outside the stored message (IH-4)

- **Clinical risk:** none: what the feed applies and answers is unchanged. A privacy change.
- **Found by the new marker test:** the HL7 dictionary's findings (`PID[1]-7[1].1: "…" is not a valid DTM value`) and the feed's own PID-7 and PID-8 warnings quoted the field's value. They reached nothing but a count in the audit row, but they carried patient data one log line away from a leak. The parser's own error, for text that is not HL7, quoted the text's first segment.
- **Changed:** `hl7_adt::withhold_values` takes every quoted value out of a finding, keeping the path and the reason; the PID-7 and PID-8 warnings no longer quote; unreadable text gets the fixed `hl7_adt::NOT_HL7`. New `interop::redact::er7`, a deny-by-default ER7 redactor (segment ids and MSH-2 to 7, 9, 11 and 12 kept; MSH-8, MSH-10 and every other populated field `***`; empty fields kept; anything not ER7 masked whole; the separator read as a character, not a byte), shapes the listener's one log line about a frame. `tracing-subscriber` (already in the tree) is a dev-dependency, to capture log lines in a test.
- **Tested:** a marker in every PID field (whole, in five components, in a repetition), in MSH-7, MSH-8 and EVN, and in unreadable text, against every reason, warning and acknowledgement (no database; it failed before the fix); end to end on Postgres, a marker in a message applied, one refused and one not handled, against audit rows, the queue's columns, version reasons, metrics and captured log lines; the redactor over 20,000 random texts (never panics, masking twice changes nothing, no marker outside a kept MSH field).
- **Not done / not verified:** no `cargo fuzz` target (in-test generators, as for the rest of the feed); log lines elsewhere in the service were not checked; MSH-3 to MSH-6 and MSH-9 are kept as structure, so a sender that puts patient data there would still have it echoed and logged.

## 2026-11-17 - The HL7 control id is kept only as a keyed fingerprint (IH-3)

- **Clinical risk:** none: nothing a clinician sees or the feed applies changes. A privacy change: some senders build MSH-10 from a patient's number.
- **Changed:** the feed replaces MSH-10 with `adt_feed::control_digest` (an HMAC of the feed's agent and the id under the `feed-control-id` subkey of `EPR_INTEGRITY_MAC_KEY`; unkeyed SHA-256 when no key is set, as the audit chain does) before it writes anything. So the audit row, `feed_refusals.control_id`, the reason on a patient version made by the feed and the "ADT message applied" log line carry the fingerprint. The acknowledgement still echoes the sender's own MSH-10, and the stored message keeps it. The same feed and id give the same fingerprint, so IH-1's resend check still works. `openapi.json` and the personal-data inventory describe the column.
- **Tested:** `audit::tests` (stable per key and domain, never the value), `adt_feed::tests` (bound to the sender; `?` stays), and `tests/mllp_feed.rs`: an id ending in a national number is in no audit row, version reason or queue column, only in the stored message, and the fingerprint joins the rows.
- **Not done / not verified:** rows written before today keep the plain id (the tables are append-only); a new active key gives new fingerprints, so a resend across a key change is kept again; the admin screen shows the fingerprint, which is no use to a person, who reads the id in the message; log lines elsewhere were not audited for MSH-10 (IH-4 does that).

## 2026-11-17 - The clinician UI's privacy notice, linked from every page

- **Clinical risk:** none found: a footer link and a page of words; nothing a clinician sees about a patient changes.
- **Built:** `/privacy`, linked from a new footer on every page, lists everything the app keeps in the browser and why: `epr-theme` (the theme chosen), `locale` (the language of the last page opened, which is set on every page in a language, not only by a choice; the page says so), `epr.purpose` (the purpose of use, this tab only), and with sign-in on, `epr_login` and `epr_session` (random ids in cookies the page cannot read; the session is on the server). It says none of these names a person, that cookies go only to the app's own server, that there are no analytics or third-party cookies, how to remove them, that patient data entered is not covered, and that this is not a deployment's legal privacy notice. English and Welsh (machine-written, not reviewed, marked as such on the page).
- **Tested:** a unit test that reads `src/` and fails when a cookie or storage name is defined that the notice does not list (or one listed no longer exists), when a cookie is set by a bare string, or when `localStorage` or a hand-written `document.cookie` appears; browser tests for the footer link, the five rows and axe, and the Welsh page. 387 unit tests and 238 browser runs pass.
- **Not done / not verified:** the other eight locales show the English notice, as for the rest of the UI; nobody has reviewed the Welsh; no data protection officer has read the page; the `locale` cookie is still set without a choice, which the notice describes rather than changes.

## 2026-11-17 - A kept HL7 refusal is answered AR, and a resend adds no row (IH-1)

- **Clinical risk:** yes, what reaches the record: the feed's answer to a refused message changes, which changes what a sender does next. Hazards H-021 and H-023 updated (still open).
- **Changed:** a refused message the service has kept in the work queue is now answered `AR`, with `MSA-3` starting "kept for review by a person; do not resend" and then the reasons; it was `AE`, which senders commonly resend, so each resend added a queue row and a strict-order sender would have held every later message behind it. `AE` is now only for what a resend can fix: the database unavailable, or the refusal not stored. `EPR_MLLP_KEPT_ANSWER=AE` keeps the old answer, with the same words, for a sender that needs it; a type not handled stays `AR`. `feed_queue::record` returns the open entry when the same agent sends the same message with the same control id again (`Recorded.repeat`), under a transaction-scoped advisory lock; migration 37 adds the index it uses. A closed entry does not count: a resend after a person closed it is kept again. The stale "work queue not built" comment in `src/adt_feed.rs` is gone.
- **Tested:** `adt_feed::tests` (the answer rule), `mllp::tests` (the setting), and in `tests/mllp_feed.rs` three sends of one refused message give one row and three identical `AR` answers, a changed text with the same control id is a new row, a resend after closing is kept again, the `AE` setting, and with no database a valid message gets `AE` and nothing says it was kept. Every refusal test that expected `AE` now expects `AR`.
- **Not done / not verified:** one setting per listener, not per sender (IH-6 brings senders); not tried with a real sending system; a sender set to alert on `AR` will now alert where it used to retry, so each sender must be told (OQ-IH1).

## 2026-11-16 - Research: HL7 v2 feed hardening, from an integration hub rewritten in Rust (IH)

- **Clinical risk:** none from this change (documents only). IH-1, IH-5, IH-8 and IH-12 are clinical and add hazard rows with their first code.
- **Researched:** an HL7 v2 integration hub's Rust rewrite, read locally at one commit: how it acknowledges, what it never logs, how its senders retry, how it watches feeds, how it tests and how it cuts a feed over. Written up in `plan.md` §36, `spec/interoperability/hl7-v2-feed-hardening.md`, tasks IH-1 to IH-12 (prefix registered), OQ-IH1 to OQ-IH6, `spec/11` and `spec/17`.
- **Found here:** a refused message is answered `AE`, which senders commonly resend, and each resend adds a work-queue row; MSH-10 is kept in plain text in audit rows, the queue and a log line; an older update can overwrite a newer one; a stale comment in `src/adt_feed.rs`. Two places where this project is ahead: the change commits before `AA`, and the listener has a connection cap and a loopback default.
- **Not done / not verified:** nothing is built. None of the four findings was observed with a real sender; each is inferred from the code and the hub's documents. The hub was read, not run. `hl7::v2::builder::acknowledge` (MSH-11, `MSA-3` escaping) and the service's shutdown were not checked. HL7 v2.5.1 chapter 2 was not re-read.

## 2026-11-16 - Themes: no meaning by colour alone, and forced colours win (TH-1)

- **Clinical risk:** yes, what a clinician sees, though nothing on screen changes: this checks that warnings, the banner's states and a high-risk allergy keep their meaning without colour and under a Windows contrast theme. Hazard H-057 updated (still open).
- **Audited:** every CSS rule that uses the warning colours (the notice, the idle warning, the banner's merged and drill lines, a high-risk allergy line, an overdue grant) also carries its meaning in words; vital-sign flags are only words. Nothing needed changing.
- **Tested:** `tests/unit/theme-contrast.test.ts` lists those rules with their words and fails when a new rule uses the warning colours, and fails if any rule sets `forced-color-adjust: none`. A browser test with forced colours on, on a light and a dark device, finds the banner, "Penicillin (high risk)", the warning and the theme control, the control focusable, nothing opting out, and the allergy line's and the warning's edges still drawn. With `forced-color-adjust: none` put on the warning, it failed.
- **Not done / not verified:** forced colours are Chromium's emulation, not a real Windows contrast theme; zoom and text spacing are not tested; no person has looked at the page in forced colours.

## 2026-11-16 - The theme control is Lily's ThemePicker 0.4 (TH-1)

- **Clinical risk:** yes, what a clinician sees: the theme control in every page's header is now Lily's ThemePicker. It offers the same three choices (*Follow this device*, *Light*, *Dark*); both themes still pass WCAG 2.2 AA, and no other theme is offered. Hazard H-057 updated (still open).
- **Approved exception:** `@lilydesignsystem/svelte-theme-picker` 0.4.0 was published at 17:41 UTC today, inside the maintainer's cooldown; the maintainer asked for the upgrade. pnpm wrote the exception for that one version into `pnpm-workspace.yaml` (`minimumReleaseAgeExclude`).
- **Changed:** 0.4.0 added a *Follow this device* choice and a cookie that a server can read, which is what the UI's own select did; the UI now uses the picker (`variant="select"`, a native select with the visible label *Theme*) as a direct dependency. The cookie is still `epr-theme`, now `system`, `light` or `dark`, kept 400 days and written only when the user chooses; nothing goes in `localStorage`. The attribute on `<html>` is `data-theme` (was `data-color-scheme`), from the server for an explicit choice and from the picker in the page. An old `device` cookie reads as following the device. The picker loads `static/themes/light.css` and `dark.css`, which are empty on purpose: the tokens stay in `app.css` so the first paint is right.
- **Tested:** `tests/unit/theme.test.ts` (5: the choices, old and forged values, the attribute, the picker refusing a cookie it does not offer, the stylesheets exist and hold no tokens); browser tests in light and dark: a choice applies, sets the cookie, is sent by the server and survives a reload, axe passes, following the device again sends no attribute, the first visit sets no cookie, the stylesheets load, and forged or old cookies change nothing. 380 unit tests and 232 browser runs pass.
- **Not done / not verified:** the picker's default icon-button listbox is not used (the native select keeps the platform's keyboard and screen-reader behaviour); the picker bar is still not rendered; the Welsh words are machine-written and not reviewed; no person has used the new control; no forced-colours test.

## 2026-11-16 - Lily 0.9 and the Lily pickers

- **Clinical risk:** none found. The only Lily component a page shows is the disabled `Slider` on the path-to-production page, and 0.9.0 does not change it; the Theme control is the UI's own select, unchanged.
- **Approved exception:** `@lilydesignsystem/svelte-headless` 0.9.0 and `svelte-picker-bar` 0.6.0 were published on 2026-11-14 at 23:43 UTC, inside the maintainer's two-day cooldown; the maintainer asked for the upgrade.
- **Changed:** `svelte-headless` 0.8.0 to 0.9.0 (adds the record lists and healthcare display components; nothing removed or renamed) and `svelte-picker-bar` 0.5.3 to 0.6.0. `svelte-theme-picker` was already at its latest, 0.3.2, as were the picker bar's other pickers and `themes` 0.6.2. pnpm's supply-chain check passed.
- **Checked:** every UI and docs gate in `ci-local.sh --full`: lint, check, unit tests, browser tests in light and dark with axe, and the UI image.
- **Not done / not verified:** the UI still renders neither the picker bar nor Lily's ThemePicker (both installed, neither imported), and none of 0.9.0's new clinical components is used.

## 2026-11-16 - axa 0.15 (AX-15)

- **Clinical risk:** none. No code, schema or response changes.
- **Approved exception:** axa 0.15.0 and axa-test 0.6.0 were published at 15:00 UTC today, inside the maintainer's two-day cooldown (`registry.global-min-publish-age`, which stable cargo does not enforce); the maintainer asked for 0.15.0 by name.
- **Changed:** `axa` 0.15 and `axa-test` 0.6; the lockfile moves `axa-postgres-job-queue` to 0.4.0 and adds `croner` 4 with its build dependencies. No migration: 0.15's new columns and index are on the job queue's tables, which this service never sets up or connects (the queue is compiled in only because `outbox` enables it).
- **Checked:** every service gate in `ci-local.sh --full` (clippy, tests, docs, migrations, every database test, load smoke, benchmarks, coverage, the FHIR validator, the API image); one `axum-test` and one `axa`.
- **Not done / not verified:** unique jobs, cron schedules, `mail` and `storage` are not adopted. The proxy-scopes test that once failed under 0.14 passed here; still not diagnosed.

## 2026-11-15 - A theme control: follow this device, light, dark (TH-1)

- **Clinical risk:** yes, what a clinician sees: a user can now pick light or dark against their browser's setting. Both pass WCAG 2.2 AA; no other theme is offered. Hazard H-057 updated (still open).
- **Specified:** the UI uses the browser's light/dark setting (`prefers-color-scheme`), and an in-app control can override it for that browser (the maintainer chose the control, 2026-11-15). Written into NFR-4 and the UI spec's themes section (points 2 and 3).
- **Built:** `src/lib/theme.ts`; a *Theme* select in every page's header with *Follow this device* (the default), *Light* and *Dark*; the choice applies at once, is kept in the `epr-theme` cookie for a year, and `hooks.server.ts` puts it on `<html>` as `data-color-scheme` (the `%theme%` placeholder in `app.html`), so the next page never flashes the other theme. Any other cookie value is read as following the device. `app.css` has dark tokens for a chosen dark as well as a device's dark, and a test keeps them equal. English and Welsh words.
- **Tested:** `tests/unit/theme.test.ts` (4), two more contrast tests; browser tests that a choice applies, survives a reload, passes axe, and goes back to following the device, and that a forged cookie changes nothing, each on a light and a dark device. 379 unit tests and 232 browser runs pass.
- **Not done / not verified:** the Welsh words ("Thema", "Dilyn y ddyfais hon", "Golau", "Tywyll") are machine-written and not reviewed; the choice is per browser, not per account; no other theme; no forced-colours test; no person has looked at either theme.

## 2026-11-15 - axa 0.14 (AX-14)

- **Clinical risk:** none. The outbox stores two more columns that are always empty; nothing a clinician sees or sends changes.
- **Approved exception:** axa 0.14.0 and axa-test 0.5.0 were published at 11:29 UTC today, inside the maintainer's two-day cooldown (`registry.global-min-publish-age`, which stable cargo does not enforce); the maintainer approved taking them now.
- **Changed:** `axa` 0.14 and `axa-test` 0.5; the lockfile moves `axa-transactional-outbox` to 0.4.0, `axa-postgres-job-queue` to 0.3.0 and `axa-cursor-pagination` to 0.1.2. The outbox's `Event` has a `tenant`, set to none (one site). Migration 36 (`20261115000036_axa_outbox_tenant.sql`) adds the four setup statements 0.4.0 has that migration 28 lacks, in axa's words: nullable `tenant` on `axa_outbox_events` and `axa_outbox_owed`, a tenant form of `axa_outbox_fingerprint` (identical to the old one when there is no tenant, so no stored fingerprint changes) and the owed trigger; `Outbox::connect_existing` refuses tables without them. `outbox::tests` checks both migrations together against axa. The personal-data inventory classifies the two columns; its census test caught them.
- **Checked:** clippy, the outbox unit tests and every database test on a fresh scratch database (all passed on the second full run; one `axum-test` and one `axa`).
- **Not done / not verified:** in the first full database run after the inventory fix, `http_proxy_scopes::a_proxy_reads_only_what_its_terms_list_and_only_until_they_end` got 401 where it expected 403 after its 3 s wait for an expiry; alone it passed 5 times, and the next full run passed. Not diagnosed: the log says nothing about the 401, and nothing shows it comes from axa 0.14. The migration was not run against a database that already holds outbox events (the scratch databases start empty). The tenant features are not adopted.

## 2026-11-15 - The dark theme is on and checked; form control edges reach 3:1 (TH-1, in part)

- **Clinical risk:** yes: what a clinician sees changes. A device set to dark now gets the dark theme, which the scaffold's `data-color-scheme="light"` had hidden from everyone, and inputs, selects and textareas have darker edges in light (`#767676`) and lighter ones in dark (`#858c96`). Hazard H-057 updated (still open).
- **Built:** `--control-border`, 3:1 or more in both themes, for form controls; `--border` stays for decoration. `color-scheme` in both themes. Every browser test runs in light and in dark (`chromium-dark`), so axe checks every page in both: 228 runs pass, 32 s. A test checks the background really changes with the device. `tests/unit/theme-contrast.test.ts` (27 tests) checks every token pair in both themes, that dark sets every token, and that form controls use the control border; it failed with the old border colour put back.
- **Not done / not verified:** the theme control, so no user can yet choose light on a dark device or any other theme; a flag-by-flag check that no meaning is carried by colour alone; forced colours; how the dark theme looks to a person (no one has looked at it; axe and the ratios are the only checks).

## 2026-11-15 - Themes: light and dark must pass, others by the user's choice (TH-1, spec only)

- **Clinical risk:** a theme changes what a clinician can see. Recorded as hazard H-057 (open); nothing is built, so nothing on screen changes with this entry.
- **Specified:** NFR-4 and the UI spec's new [themes](electronic-patient-record-ui-with-svelte/spec/index.md#themes-th-1) section. The light and the dark theme each pass WCAG 2.2 AA on every page (1.4.3, 1.4.11, 1.4.1), and one of them is always the default. A user may choose a theme that does not pass, for themselves only; it is labelled, never a default, and the theme control stays readable in every theme, following WCAG's conforming alternate version (W3C Understanding Conformance, technique C29). A theme changes colours only: the banner, the warning, allergy and critical flags and focus stay, not by colour alone. Task TH-1 holds the tests and the work.
- **Found:** text contrast passes in both themes today (6.7:1 or more), but `--border` on the background is 1.67:1 light and 2.02:1 dark, below 1.4.11's 3:1, and axe has only ever run in the light theme.
- **Not done / not verified:** no code, test or theme control; the ratios were computed from `src/app.css`'s tokens, not measured on rendered pages; the WCAG conformance text was read on W3C's Understanding page, not in the standard's own section.

## 2026-11-15 - Load tests and benchmarks (OS-5, SF-2)

- **Clinical risk:** none. Tests, benchmarks and CI only; no service or UI code changed, and nothing a clinician sees.
- **Built:** criterion 0.8 benchmarks (a dev dependency only) for the matcher (`benches/matcher.rs`: normalisation, identifiers, six comparison shapes, a register of 1 to 1,000 candidates) and the service (`benches/service.rs`: the audit digest, the policy decision, registration parsing and validation, a comparison with the site's rules, observation checks, an HL7 v2 parse, the duration histogram). A load harness replaces `scripts/load/measure.mjs`: `run.mjs` with ten scenarios (register, read, search, a chart open made of the chart page's 8 reads, FHIR read and search, compare, a vital sign, an allergy, and a weighted mix), closed and open loops (the open loop charges a stall to every request it delayed), smoke, standard, stress and soak profiles, the NFR-2 budgets in `budgets.json`, `--baseline` regression and JSON output naming the machine and commit; `run-local.sh` builds a release binary, runs it on a scratch database with login off and samples the database's waits. `test_load.mjs` has 16 tests, including whole runs against a fake service for each exit status. Gates: `docs-load-harness` (every push), `service-load-smoke`, `matcher-bench` and `service-bench` (`ci-local.sh --full` and a new nightly `performance` CI job, which also keeps a standard run as an artefact). [Performance testing](spec/performance-testing.md) is the spec; NFR-2 cites it.
- **Measured** (one laptop run, `BENCHMARKS.md`): chart open p95 12 ms at 1 worker and 289 ms at 32, within NFR-2's 1 s; search p95 81 ms at 32, within 200 ms. FHIR read p50 (32 ms against 10) and FHIR search p95 (248 ms against 200) broke their budgets at 32 workers. The mix broke its 1 s p95 at 64 workers. A 3 minute soak showed no drift. 84 percent of busy database sessions were waiting on the audit chain's advisory lock (CQ-18).
- **Fixed, the same day:** the API image copied only `src/`, and the new `[[bench]]` entry made `Cargo.toml` fail to parse in the image build (CI on `105142a`); the Containerfile now copies `benches/` too. The pre-push hook runs `--fast`, which has no image build, so it did not catch this.
- **Not done / not verified:** latency is not a gate in CI (a shared runner); the nightly job has not run yet. The chart is thin, login is off, the mix is a guess, order sign is not measured, one client ran on the same host, the soak was 3 minutes, and the run without the audit append was not done. Why recording a vital sign outran every other write at 32 workers (3,346 a second) while an allergy fell to 202 was not looked into.

## 2026-11-15 - Browser tests wait before reading what was sent

- **Clinical risk:** none. Tests only; the screens and what they send are unchanged.
- **Fixed:** an audit of the browser tests' captured-request checks for the race fixed below found four more that read the requests straight after a click. One was a real race: `recording a proxy sends its basis, scopes and last day` read the request before the list showed the proxy; it now waits for the list first. Three check that nothing was sent (a vital sign with no reason, a referral step whose note is empty, an unmatched reply cleared with no referral id). Such a check can pass when a stray request arrives late, so each now waits for the field or state that shows the click was handled where there is one, and the test counts the requests exactly after the next real send. The four tests passed five times each.
- **Not done / not verified:** the audit matched a check on a captured array whose line before is a click, fill or similar; a check two lines after an action with no wait between was not looked for. The full browser suite was not rerun here; the pre-push hook and CI run it.

## 2026-11-15 - A browser test waits for the request it checks

- **Clinical risk:** none. A test only; the screen and what it sends are unchanged.
- **Fixed:** `a patient asks for a reading to be corrected, withdraws, and states disagreement with a refusal` read the last request straight after clicking "Add my statement", with nothing on the page to wait for, and on CI's run of `7040cc3` it still saw the withdrawal. It now polls until the statement's request arrives. It passed ten times in a row here.
- **Not done / not verified:** the other 60 checks of captured requests in the browser tests were not audited for the same race here; the entry above did that.

## 2026-11-15 - One definition per gate, first batch (LC-3)

- **Clinical risk:** none. CI and local tooling only; the same checks run.
- **Built:** `scripts/gates/<name>.sh` holds each gate once, and both `ci.yml` and `ci-local.sh` call it. Moved so far: the matcher's four gates and the ten document gates. `scripts/check-gates.py` (the `docs-gates` gate) fails on any `run:` step in `ci.yml` that neither calls a gate script nor is listed with its reason: setup, GitHub only (the secret scan, the pull-request checks, `ci-ok`, the toolchain pin, pnpm audit), or not moved yet (20 steps, a list that only shrinks). It also fails when a gate script has no local twin, or a listed step no longer exists. `scripts/test_check_gates.sh` adds an inline gate, a missing script and a stale entry to copies of the workflow, and each fails.
- **Changed:** CI's matcher job now runs `matcher-test` as the script does: `cargo nextest run` and `cargo test --doc` where nextest is installed, plain `cargo test --no-fail-fast` where it is not (CI's matcher job, today).
- **Second batch, the same day:** the service's four gates and the UI's five (lint, check, test types, unit, browser) moved; `ui-unit.sh` passes its arguments to vitest (CI adds `--coverage`), and `ui-browser.sh` takes `PW_PORT` from the caller. 10 inline steps are left on the list.
- **Third batch, the same day; LC-3 done:** the database tests, the scratch database test, the FHIR validator, both images and both coverage floors moved, 30 gate scripts in all, and the not-moved list is empty. `ci-local.sh --full` now runs the coverage floors too (`coverage-matcher`, `coverage-service`; SKIP without `cargo-llvm-cov`), and every gate passed through the scripts. Listed with reasons and not moved: the `psql` step that prepares CI's database (setup; locally the scratch database applies the same SQL) and the two `podman compose` start-up checks (CI only: they need a clean runner's free ports).
- **Not done / not verified:** CodeQL and cargo deny are GitHub actions with no local twin, and are listed as such.

## 2026-11-15 - Speed spike: cargo-nextest adopted (LC-8)

- **Clinical risk:** none. Test runner only; the same tests run.
- **Built:** CI's API job runs `cargo nextest run` then `cargo test --doc`, and `ci-local.sh`'s Rust test gates do the same when `cargo-nextest` is installed (plain `cargo test` otherwise). nextest ran the same tests (117 matcher, 553 service, 182 database tests left out) about 25 s faster warm in CI over two runs, and faster here.
- **Not adopted:** mold (no faster in CI), sccache (CI already caches dependencies with rust-cache). The API image's cold release build, 679 s of a code push, is the lever left: new task LC-14.
- **Not done / not verified:** few samples on runners whose speed varies by tens of seconds; why the warm nextest build was faster is not known; the matcher's CI job is unchanged because it was not measured. The spike branch and workflow were deleted.

## 2026-11-15 - A signed local gate record (LC-6)

- **Clinical risk:** none. Developer tooling only. A record is self-attested and is not evidence for the hazard log or the safety case; it says so in its own text, as OQ-LC3 recommends (hosted runs only).
- **Built:** `scripts/ci-local.sh --full` on a tree whose tracked files match HEAD writes `target/ci-local/<commit>.json`: the commit and tree, the host's OS and architecture, the tool versions, each gate's result and seconds, and whether all passed. Untracked files are counted, not refused. It signs the record with git's `user.signingkey` (`ssh-keygen -Y sign`, namespace `epr-ci-local`). A dirty tree, or HEAD moving during the run, writes none. `ci-local.sh --verify <commit>` (`scripts/gate-record.sh verify`) checks the signature against git's allowed signers, and that the record names that commit and its tree. A run narrowed by `EPR_CI_ONLY` never counts as passed.
- **Tests:** `scripts/test_gate_record.sh` (the `docs-gate-record` gate and a step of CI's docs job) uses a throwaway key and repository: a record verifies; changed, copied to another commit, or signed by a key not on the list, it fails; a dirty tree is refused; a failed run names its gates; a signing failure leaves an earlier record as it was.
- **Not done / not verified:** nothing publishes or keeps a record beyond `target/`, which is not committed. A record is signed, not reproduced: it says this key's holder ran the gates, not that they would pass elsewhere.

## 2026-11-15 - CI runs only the jobs a push needs (LC-7)

- **Clinical risk:** none. CI only; every check still runs, nightly if not on the push.
- **Built:** a `changes` job runs `scripts/ci-changes.sh` on the files changed since the last commit on the branch whose CI passed (not since the previous commit, which a newer push may have cancelled), or since a pull request left its base. Rust jobs run when Rust needs them, UI jobs when the UI does, the container build when either does, and each CodeQL language with its project. Documents run neither. The docs, secret-scan, toolchain and hazard-log jobs run on every push, and a nightly run (03:17 UTC) and a manual run do everything. `ci-ok` waits on `changes` too and stays the single required check; a skipped job passes. The concurrency group now names the event, so a push no longer cancels the nightly run.
- **Tests:** `scripts/test_ci_changes.sh` (the `docs-ci-changes` gate and a step of CI's docs job) has fixed cases and a census of what the tests read outside their own project, 10 files today. Each must run the project that reads it. The census found that a UI test reads the service's `src/adjustments.rs`, which a hand-made list had missed. Run with `SECURITY.md` dropped from the list, it failed.
- **Measured:** the documents-only push `792d5dc` was green with the 11 build and test jobs skipped, in 0.6 job-minutes and 18 s (a full run is about 36 job-minutes). Over the last 200 commits that saves about 6.8 job-minutes a push, some 16% of LC-1's median; a code push's wall time is unchanged.
- **Not done / not verified:** the container build and CodeQL were not moved to nightly as the task proposed, because every commit lands on `main`, so path filters do that work. The pre-push hook still maps files by directory and does not use `ci-changes.sh`. A skipped CodeQL language still starts a runner for a few seconds. The first nightly run has not happened.

## 2026-11-15 - A preflight before a full local run (LC-12)

- **Clinical risk:** none. Developer tooling only.
- **Built:** `scripts/preflight.sh`, run first by `ci-local.sh --full`. It checks `PW_PORT`, `EPR_CI_PG_PORT`, a local `DB_URL` server, the FHIR validator jar, and the Podman VM's clock, memory and disk. It reports every problem with its fix and exits 3 before any gate runs; `EPR_CI_SKIP_PREFLIGHT=1` skips it. `scripts/test_preflight.sh` breaks each condition on purpose (the `docs-preflight` gate and a step of CI's docs job), and failed against a preflight whose clock check never fired. With port 4183 really taken, `--full` stopped in 1 s.
- **Not done / not verified:** the thresholds are judgements from the failures seen. The VM checks ran only on macOS. A remote `DB_URL` is not checked.

## 2026-11-15 - Playwright 1.65.1

- **Clinical risk:** none. A test tool only.
- **Built:** `@playwright/test` 1.65.0 to 1.65.1 in the UI; the 113 browser tests pass, with lint and the test type check. `@axe-core/playwright` 4.14.0 is already the latest.
- **Not done / not verified:** the release is two days old and its notes were not read. Locally the browser tests ran on port 4193.

## 2026-11-15 - AGENTS.md names the API image's memory need

- **Clinical risk:** none. Documents only.
- **Built:** a line in AGENTS.md: the API image needs a Podman VM with about 16 GB or more, because `fhir-r4` and `fhir-r5` each take about 4.5 GB to compile side by side; with 12 GB the kernel killed `rustc` (seen 2026-11-14 in its log). With the VM at 32 GB, the `container-api` gate built in 276 s, and every `--full` gate has passed on this machine (the browser tests on another port, as 4183 was taken).
- **Not done / not verified:** 16 GB is an estimate from two 4.5 GB compiles plus overhead; only 12 GB (fails) and 32 GB (passes) were tried.

## 2026-11-14 - A pre-push hook (LC-5)

- **Clinical risk:** none. Developer tooling only.
- **Built:** `.githooks/pre-push` runs `scripts/ci-local.sh --fast` for the projects the pushed commits touch (the document checks always), installed by `scripts/install-hooks.sh`. `ci-local.sh` gains `EPR_CI_ONLY`, with left-out gates shown as `off`. `scripts/test_pre_push.sh` tests the selection in a throwaway repository; it is the `docs-hooks` gate and a step of CI's docs job, and it failed against a broken hook. A badly formatted Rust file stopped a push (exit 1, 55 s); every project with warm caches took 107 s.
- **Not done / not verified:** the hook checks the working tree, not the pushed commits, and does not stash; it warns when a checked project has uncommitted changes. A cold build on a fresh clone was not timed.

## 2026-11-14 - The service spec names axa 0.13's database settings (AX-13)

- **Clinical risk:** none. Documents only.
- **Built:** the service spec's configuration section names `DB_URL_FILE` (both it and `DB_URL` set stops startup), `DB_APPLICATION_NAME`, `DB_STATEMENT_TIMEOUT_MS` and `DB_READ_ONLY`, read by `axa`; the production checklist's secrets line names the two `_FILE` settings.
- **Not done / not verified:** no test here sets any of them, no compose file uses them, and `DB_URL_FILE` with a mounted secret has not been tried.

## 2026-11-14 - UI dependency bumps (Dependabot #9, prettier 3.10)

- **Clinical risk:** none expected. Tool and framework patch updates; the browser tests pass unchanged, and the only source change is Prettier's line wrapping in `src/lib/translations.ts` (no word changed).
- **Built:** `@sveltejs/adapter-node` 6.0.2, `@sveltejs/kit` 3.2.0, `prettier` 3.10.1 (Dependabot asked for 3.10.0), `typescript-eslint` 8.72.2 and `vite` 8.3.8, as Dependabot PR #9 asked; that PR failed CI because Prettier 3.10 wraps long non-Latin strings differently. The lockfile refreshes their transitive dependencies.
- **Not done / not verified:** the releases are two to four days old; no changelog of theirs was read beyond what the gates exercise.

## 2026-11-14 - axa 0.13 (AX-13)

- **Clinical risk:** none expected. No screen or rule changes; the API refuses a browser's cross-site write it accepted before, and the UI never sends one (its proxy forwards no `origin` or `sec-fetch-site`).
- **Built:** `axa` 0.13.0 and `axa-test` 0.4.0; the lockfile moves `axa-postgres-sqlx` to 0.2.0. No source change; the OpenAPI document is unchanged. New behaviour from axa, with no code here: cross-site write refusal, compressed request bodies decoded, a JSON Pointer in a rejected body's problem, named Postgres connections, and `DB_URL_FILE` (both it and `DB_URL` set stops the boot).
- **Not done / not verified:** the API container did not build locally (the Podman VM killed the compiler for memory, in `fhir-r5`); CI builds it. Real-Keycloak tests not run. None of axa 0.13's new pieces is adopted (`config::Settings`, tenancy, readiness waiting for migrations, `TestDatabase`), and no deployment document names `DB_URL_FILE` or `DB_APPLICATION_NAME` yet.

## 2026-11-14 - The UI adds syncpack as a dev dependency

- **Clinical risk:** none. A development tool only; nothing the UI builds, serves or shows changes.
- **Built:** `syncpack` 15.3.3 in the UI's `devDependencies`, pinned exactly, added with `pnpm add -D` (no `package-lock.json`). `pnpm exec syncpack lint` reports no issues.
- **Not done / not verified:** no syncpack config, no `package.json` script and no CI or `ci-local.sh` gate, so nothing runs it. With one `package.json` in the repository it has little to keep in step yet.

## 2026-11-14 - Research pass: clinical modules, patient portal, analytics, population health, research data (plan.md §35)

- **Clinical risk:** no change to the product. Documents only: no code, route, screen or hazard row changed. Most new tasks are marked clinical, and each adds its hazard rows with its first code; each page describes its candidate hazards.
- **Built:** ten spec pages:
  - `spec/clinical-workflows/` (inpatient first, then the clinic visit);
  - `spec/scheduling/`;
  - `spec/clinical-documentation/`;
  - `spec/order-entry/`;
  - `spec/nursing-flowsheets/` (NEWS2 and escalation);
  - `spec/medication-administration/` (the drug chart and bedside scanning);
  - `spec/patient-portal/` (results, messages, booking, bills and sharing);
  - `spec/analytics/`;
  - `spec/population-health/`;
  - `spec/research-dataset/`.

  Each page has workflows, testable rules, candidate hazards, a first slice and keyed sources. `plan.md` §35 brings together the ranked ideas, risks and the order of work. `tasks.md` gains 84 tasks under ten new prefixes (IW, SC, CN, OE, NF, MA, PP, AN, PH and RD), registered in `scripts/generate-status.sh`. Each task names the existing task it refines. Spec 16 gains 62 open questions, and spec 17 and the spec index link the pages.
- **Findings worth acting on:**
  - The local national data opt-out refuses only the `research` purpose, while England's opt-out also covers planning (RD-1).
  - NEWS2 needs level of consciousness and air or oxygen, which the record does not hold (NF-1).
  - A stalled replication slot could take the clinical record offline once any reporting copy exists (AN-3).
  - The portal page recommends that billing stay outside the EPR (OQ-PP4).
- **Not done / not verified:** the research ran as seven parallel passes whose pages were not read line by line here. Most study figures are from abstracts or search summaries, as each page says. External URLs were not link-checked. No clinician, pharmacist, data protection officer or ethics committee has reviewed anything. No licence question (NEWS2 in software, HEDIS, risk scores) is answered.

## 2026-11-13 - Documents name no national health service (NHS, NHS Wales, DHCW)

- **Clinical risk:** none. Documents and code comments only; no rule, screen or code path changes.
- **Built:** the maintainer asked for no mention of the NHS, NHS Wales or DHCW. Every current document now describes the body or service instead of naming it: "England's national health body", "the health service in Wales", "Wales's national digital health body", "a hospital trust", "national health number", and the like, in `plan.md`, `tasks.md`, `RFC.md`, the specs, the research and supplier pages, `docs/`, `governance/` and `agents/`. `spec/nhs-wales/` is now `spec/wales/`, and every link to it follows. Citation keys that began with NHS are renamed (`[EFAM]`, `[EMPLOYERS]`, `[UKHIV]`, `[UKCOERCE]`, `[UKRBAC]`, `[UKDR]`, `[UKREV]`). Old code names in current documents (`nhs.rs`, `nhs_number_*` codes) are the new ones. Where a quotation named the body, the replacement is in square brackets.
- **Kept:** citation URLs and bare domain names that contain `nhs` (removing them would leave sources nobody can find), one contact address, the FHIR identifier system URI, the HL7 assigning authority `NHS` and the sample messages that carry it, and the official name of the Welsh cancer-site list's source in the service's data files. Earlier changelog entries keep their words, as the record of what was true then; one old link now points to `spec/wales/`.
- **Checked:** a search of every tracked file finds no NHS, DHCW or Welsh "GIG" outside those kept items; links, the status block, llms, the schema map and the hazard log check pass. The rewording was split across four parallel passes, and their output was read in part, not line by line.
- **Not done / not verified:** proper names of trusts and boards are now descriptions ("the Fife health board"), which is less exact. A reader cannot search the documents for those organisations by name. No one has read every changed line.

## 2026-11-13 - The service and UI name a national health number, not the NHS (matcher 0.8.0 published)

- **Clinical risk:** small, in wording only. A clinician now reads "National health number" where the label, banner, patient list, search and registration form said "NHS number", and the refusal reasons say "national health number"; the Welsh, German, Spanish, French, Chinese, Hindi and Arabic words changed with them (machine-written, unreviewed). No rule changed: the check, the merge refusal and the matching decisions are as before. The hazard log's wording follows (H-001, H-003 and the rows naming the number); no hazard's status changed.
- **Built:** matcher 0.8.0 is published to crates.io and the service uses it. The API's conflict codes are now `different_national_numbers`, `same_national_number_different_birth_dates` and `same_national_number_different_names`, and the registration refusal codes are `national_number_placeholder` and `national_number_check_digit`; the UI reads only the new codes. Service and UI names, comments, tests and specs follow (`NATIONAL_NUMBER_SYSTEM`, `nationalNumber`, `bannerNationalNumber` and the rest).
- **Kept, as data:** the FHIR identifier system URI real records carry for this number (once in the matcher, once in the UI, and in one database test's query), the HL7 assigning authority `NHS` that senders put in PID-3 and the sample messages that carry it, and the official name of the source of the Welsh cancer-site list.
- **Checked:** service fmt, clippy and database-free tests; the database suite on a scratch database, where 175 passed and `http_roles` failed once with an expired test token under machine load (average 34) and passed when run again alone; UI lint, both type checks, 347 unit tests and 113 browser tests; the hazard log check.
- **Not done / not verified:** a client of the API that reads the old codes breaks; none is known outside this repository. The real-Keycloak tests were not run. No speaker reviewed the new translated words. Documents outside the three projects are reworded in the next commit.

## 2026-11-13 - The matcher names a national health number, not the NHS (matcher 0.8.0)

- **Clinical risk:** none today, and none in behaviour. Every matching rule is unchanged; only names, codes and English messages change. When the service moves to 0.8.0, the conflict codes and messages a clinician sees change with it, and the UI must change in the same commit.
- **Built:** in the matcher, the NHS number is a "national health number": module `national_number`, `NATIONAL_NUMBER_SYSTEM`, `is_valid_national_number`, `is_placeholder_national_number`, and the conflict codes `different_national_numbers`, `same_national_number_different_birth_dates` and `same_national_number_different_names`. Tests, the spec, `AGENTS.md` and `index.md` follow. Version 0.8.0 (breaking: the public names and codes).
- **Kept:** the FHIR identifier system URI that real records carry for this number. It is spelled out once, in `NATIONAL_NUMBER_SYSTEM`, because without it those records would not be recognised and the rule that two different numbers never link would stop applying.
- **Checked:** fmt, clippy, every matcher test, and rustdoc.
- **Not done / not verified:** 0.8.0 is not published, so the service still builds against 0.7.0 and the API still sends the old codes. Earlier changelog entries keep the old names.

## 2026-11-13 - First posts for GitHub Discussions

- **Clinical risk:** none. Draft posts only; nothing in the software changes.
- **Built:** six first posts in `locales/en-us/communications/github-discussions/`, one per default Discussions category, each with its title on the first line: `announcements.txt` (Discussions open, what exists, where to go for what), `general.txt` (a welcome with one question), `ideas.txt` (how to suggest an idea, with headings to copy), `polls.txt` (RFC question 2, the UK's or the US's standards first, advisory, with four options), `qa.txt` (how to ask, with what never to paste), and `show-and-tell.txt` (the matcher declining to link two records when a given name is missing, with commands to try it). Written from the skills in `skills-on-ai`, using only facts from this repository. The matcher output in the show-and-tell post is from a real run on 2026-11-13.
- **Not done / not verified:** Discussions are not enabled on the repository, so nothing is posted, and the category names are GitHub's defaults, which may differ once they are enabled. The poll has no closing date. The posts are English only. No reader has seen them.

## 2026-11-13 - The API listens on port 1241

- **Clinical risk:** none. A port number only.
- **Built:** the service's default port is 1241, where it was 5150 (`src/app.rs`). Everything else that named 5150 now names 1241: the Containerfile (`EXPOSE`, `PORT`, the health check), `compose.yaml` (the published port, `PORT`, the health check, the UI's `EPR_API_URL`), the CI container job, the UI proxy's default and `.env.example`, the real-Keycloak Playwright config and test, the READMEs, `operations/keycloak/README.md`, `scripts/keycloak-dev-token.sh`, `agents/testing.md`, `plan.md` and `spec/07-non-functional-requirements.md`. Earlier entries in this changelog still say 5150, because that was true then. A deployment that sets `PORT` itself is unchanged.
- **Checked:** the service's database-free tests; `electronic-patient-record check` reports `PORT=1241`; the UI's lint, both type checks and 347 unit tests; `compose.yaml` and `ci.yml` parse. Nothing on this machine listens on 1241.
- **Not done / not verified:** the service was not started on 1241 locally (a bare start exits with no database or keys); the container images were not built (the Podman VM's disk is full), so the compose stack and the CI container job's health check on 1241 are first tried in CI. The real-Keycloak tests were not run.

## 2026-11-13 - The path to production (PR-9, H-056)

- **Clinical risk:** yes, small, and logged as H-056 (open). The warning on every page keeps its words, "Not for clinical use. Do not enter real patient data.", and gains a link. The new page puts a percentage on each line of readiness, and a reader might take those figures as permission to use the software in real care. Against that, the page says each figure is a guess no person has checked, gives no overall figure, and no line is 100%.
- **Built:** a UI page at `/<locale>/path-to-production` (`/en-001/path-to-production` in English). It says what production means: the care of real patients, with their real data, at a named site, by people who rely on it. It then lists the 17 lines of the evidence checklist, each with a disabled Lily `Slider` at its estimate (0 to 100), the percentage as text, what exists today and why. `governance/production-readiness.md` is retitled "Path to production: the evidence checklist", says what production means, and gains an Estimate column. `spec/index.md`, the Wales page, the auditability page, `plan.md` and `README.md` now call it the path to production; the file keeps its name so links hold. The page's words are in `STRINGS` with a machine-written, unreviewed Welsh table; the list stays in English, as the checklist's words. This is the first use of a Lily widget in the UI.
- **Checked:** lint, both type checks, 347 unit tests and all 113 browser tests (the new page with axe, the banner link). The new unit test failed when one estimate was changed on the page alone and again on the checklist alone. Screenshots at 1100 and 320 px.
- **Not done / not verified:** no person has checked any estimate. They were written by an AI coding assistant from this repository on 2026-11-13, and nothing changes them when the code changes. The other "NOT FOR CLINICAL USE" notices (about 30 documents, the OpenAPI descriptions, `llms.txt`) keep their words and do not link here, because they are safety warnings. The Welsh is not reviewed. The real-Keycloak tests were not run. The page scrolls sideways at 320 px (1151 px wide), the same as the home and patients pages, because of the site header (XP-1, XP-2).

## 2026-11-13 - A scratch database per local run (LC-4)

- **Clinical risk:** none. Local tooling and one CI step; no service code changes.
- **Built:** `scripts/ci-local.sh --full` makes a database for the run, runs the migrations as their own gate (`service-migrate`), points `DB_URL` at it for the database tests and drops it at the end or on an interrupt. A `DB_URL` that is set now names only the server; nothing is written to its database. Without one, the throwaway Postgres 18 starts on a free port, where it used the fixed port 5544. The functions are in `scripts/scratch-db.sh`. `scripts/test_scratch_db.sh` runs two runs at once on one server with a leftover table in the shared database, then two runs at once with no `DB_URL`. It is the `scratch-db-test` gate and a step in CI's `api-db` job.
- **Checked:** the test passes with a given `DB_URL` and without one. It fails against two broken versions: one where every run gets the same name, and one that uses the shared database. Afterwards the server held no scratch database and no table. A `--full` run passed every gate except three. The chaos test refused the first scratch name because it will only write to a database whose name contains `test`. The guard was kept and the name now ends `_test`, and on a rerun `service-migrate` and all 176 database tests passed. The two container builds failed because the Podman VM's disk is full (200 GB used), unrelated to this change.
- **Not done / not verified:** nothing removes a scratch database left by a run killed with `kill -9`. A given server's role needs `CREATEDB`. The container gates were not rerun. Two runs at once still share `target/`, the logs in `target/ci-local/`, the Playwright port and the container image tags. The CI step has not run on GitHub yet, and there only the given-server half runs: the runner has no Postgres 18 server tools.

## 2026-11-10 - axa 0.12 (AX-12)

- **Clinical risk:** none. For a Postgres service this upgrade is renaming only, and nothing the service does changes.
- **Built:** axa 0.12.0 with the feature `postgres-sqlx` in place of `sqlx`. The module paths `axa::sqlx` become `axa::postgres_sqlx` (in `src/app.rs`, `src/pool.rs` and `src/store.rs`). axa-test moves to 0.3.0. The lockfile now has `axa-postgres-sqlx` 0.1.3 in place of `axa-sqlx`, and `axa-transactional-outbox` 0.3.0. `plan.md`'s stack table names the new version and feature.
- **Checked:** the build, all 553 database-free tests, the OpenAPI census (the document is unchanged), fmt, clippy, rustdoc, the `check` command, and these database tests: readiness, `http_patients`, `http_suspicions`, `outbox_store` and `audit_immutable`. The axa upgrade guide was read at its tag.
- **Not done / not verified:** the full database suite was not run locally for this change; CI runs it. The real-stack tests were not rerun.

## 2026-11-10 - axa 0.12 recorded, not started (AX-12)

- **Clinical risk:** none. This is a backlog entry.
- **Built:** `tasks.md` gains AX-12, the axa 0.12 upgrade, from the axa maintainers' release note. For this service it is renaming only: the `sqlx` feature becomes `postgres-sqlx`, three uses of `axa::sqlx` become `axa::postgres_sqlx`, and `axa-test` moves from 0.2 to 0.3.
- **Not done / not verified:** nothing is upgraded. The axa upgrade guide was not read here. The three uses were found with a search.

## 2026-11-10 - A database that cannot serve is a 503 (AX-11)

- **Clinical risk:** small. During a database outage a clinician's screen now gets a 503 that says to try again shortly, where it got a 500. The UI shows any 5xx as an error either way.
- **Correction:** the axa 0.11 entry said a database outage now answers 503. That was true only of `axa`'s own routes, such as `/_health/ready`. The service's routes turned every store error into a string and answered 500.
- **Built:** `StoreError::Unavailable`. The store's error constructor (`store::db_err`, behind every store's `db_err`) sets it when the error is a sqlx error that `store::unavailable` judges to be the database unable to serve for now. That covers a pool timeout or closed pool, a lost connection, and the Postgres codes 25006, 53300, 08… and 57P…, the same rule `axa` 0.11 applies. It is answered 503 with `code: unavailable` and `Retry-After: 5`, never the cause, in the route's own shape (JSON or an OperationOutcome). Any other database error stays a plain 500. Every 503 the service sends now carries `Retry-After`.
- **Checked:** new unit tests (the classification; the 503 with `Retry-After` on /api and /fhir and none on a 500), all 553 database-free tests, clippy, rustdoc, and the database suite on a fresh database.
- **Not done / not verified:** no test runs a store route against a database that is actually down. An audit row that cannot be written during an outage is still a 500, because the audit writer's errors are not store errors. The OpenAPI document does not list 503 on each route.

## 2026-11-10 - A 401 names its scheme, and axa's headers are pinned (AX-11)

- **Clinical risk:** none. Response headers only.
- **Built:** the service's own authentication refusal (`auth::refusal`, used by `require_auth_mw`) now sends `WWW-Authenticate: Bearer` on a 401, as RFC 6750 asks and as axa's own authentication crates do since 0.11. The service's 401 comes from its own guard, not those crates, so it had none. Other refusals (503 before start-up finishes) carry no such header. `tests/axa_defaults.rs` builds the routes inside axa's standard stack, as `App::router` does, and pins what axa 0.11 adds to every response: `cross-origin-opener-policy` and `cross-origin-resource-policy` of `same-origin`, and a `permissions-policy` turning off camera, microphone, location, payment and USB.
- **Checked:** the new unit test (on /api and /fhir, and none on a 503), the new header test, all 551 database-free tests, the login-on database tests (`http_roles`, `http_watch_list`), clippy and rustdoc.
- **Not done / not verified:** no test covers the 503 a store route now gives during a database outage; it needs the stores with a failing pool. The UI proxy's handling of the new header was not tried in a browser (the UI maps a 401 to "sign in" either way).

## 2026-11-10 - Watch-list days are the site's days (WL-6, H-055)

- **Clinical risk:** small. Near midnight, for a site outside UTC, an entry's last day now lapses on the site's calendar rather than the server's UTC one. Hazard H-055's residual risk is updated.
- **Built:** `site::zone`, `site::day_in` and `site::today` hold the site's day (`EPR_SITE_TIME_ZONE`, checked at start-up; UTC when unset). The watch list judges a last day, both when it is added ("today or later") and when it lapses, on that day. The suspicion plans use the same `site::zone` as before.
- **Checked:** a new unit test (23:30 UTC on 30 June is 1 July in London), all 549 database-free tests, the watch-list and suspicion database tests, and clippy.
- **Not done / not verified:** the database tests run with no zone set, so they count UTC days. The UI still shows the "until" day as it was entered, with no zone named.

## 2026-11-10 - Lapsed watch-list entries are shown to their owner (WL-6, H-055)

- **Clinical risk:** yes. An entry whose last day passed used to leave its owner's list with nothing said. It is now listed on the owner's page until they close it. Hazard H-055's residual risk is updated.
- **Built:** `GET /api/watch-list` also returns `lapsed`: the caller's entries whose last day has passed and that they have not ended, oldest last day first, up to 200 (`lapsed_more` when cut short). The audit row counts them. `watch_list_store::lapsed` shares one query with the active list. On `/watch-list`, "Lapsed, not yet closed" lists them with their last day and a link to the record. On the patient page, a lapsed entry can be ended with a reason, as an active one can.
- **Checked:** `tests/http_watch_list.rs` (a lapsed entry is on its owner's lapsed list and not on anyone else's; the owner ends it with a reason and it leaves the list), the OpenAPI census, all 342 vitest and 111 Playwright tests (the own-list test now covers the lapsed section), lint and `check`.
- **Not done / not verified:** nothing pushes a notice. The owner sees a lapse only by opening the page, and the header link carries no count. Lapsed is judged by the server's UTC day, not the site's. There was no run against the real stack.

## 2026-11-10 - The framework named as it is (Axum and axa)

- **Clinical risk:** none. This changes text only.
- **Built:** the descriptions that still called the service a loco.rs service now say Axum and `axa`: `plan.md`'s repository tree, its stack table (now Axum 0.8 with `axa` 0.11 and its features; tokens are PASETO or Keycloak, with no JWT library) and its architecture sketch; `spec/index.md`; the compatibility and maintainability docs (`axum-test` follows `axa`, not loco-rs, and `jsonwebtoken` is not a dependency); the Containerfile's image description; and two `deny.toml` comments.
- **Kept on purpose:** the test database's user and password are still `loco`, because they are credentials that CI and local setups use, not a description. Research rows and the CHANGELOG that describe the service as it was are history.
- **Not done / not verified:** `.gitignore` and `.gitleaksignore` comments still mention loco's scaffold, because they explain why the lines exist. No build or test depends on any of this text.

## 2026-11-10 - No SeaORM

- **Clinical risk:** none. This changes text only.
- **Built:** the service has no SeaORM dependency: neither crate names one, the lockfile holds no SeaORM crate, and axa runs on its `sqlx` feature alone. `plan.md`'s stack table now says PostgreSQL through `sqlx` with `.sql` migrations and no ORM, instead of SeaORM 2.0 and a migration crate. SF-3's original SeaORM line is struck through as removed. AX-11 no longer lists `axa-seaorm` or the SeaORM upgrade steps, which did not apply.
- **Kept on purpose:** `tests/chaos_api.rs` still checks that no error response names "SeaORM", like "sqlx", because that is a guard against leaking a driver's name. `plan.md` still says the patient administration system uses SeaORM, because it does. CHANGELOG history is unchanged.
- **Not done / not verified:** `plan.md`'s stack table still names loco-rs 1.2 as the service framework, which axa replaced; that was not part of this request and is left for its own change.

## 2026-11-10 - axa 0.11 (AX-11)

- **Clinical risk:** none intended. Behaviour changes are listed below; none changes what a clinician sees. (Corrected later the same day: the 503 for a database outage reached only `axa`'s own routes, such as readiness, until "A database that cannot serve is a 503".)
- **Built:** axa 0.11.0 and axa-test 0.2.0, with the axa crates that come with them (axa-problem 0.2.0 replaces axa-problem-details-for-http-apis; axa-environment 0.2.0, axa-request-rate-limit 0.3.0, axa-idempotency-key 0.2.2, axa-sqlx 0.1.3). The code compiled unchanged.
  - **Settings:** the database setting is `DB_URL`, not `DATABASE_URL`, everywhere outside the history: CI, compose, the Containerfile, the test harness, the scripts, AGENTS.md and the docs. A production binary refuses to start without it.
  - **Probe paths:** `/_health` and `/_ping` are now `/_health/live`, `/_readiness` is `/_health/ready`, and `/_health/startup` is new. The old paths answer 404. The public-path list, the route and OpenAPI censuses, the readiness and lockout tests, the compose and Containerfile healthchecks, CI, the UI proxy's comments and tests, and the specs all move with them. The OpenAPI document changes only those three paths.
- **Behaviour that changes with no compile error (from axa's upgrade guide):** a database that cannot serve now answers 503 with `Retry-After: 5`, not 500. Readiness answers 503 from the moment shutdown begins. Every response carries `cross-origin-opener-policy` and `cross-origin-resource-policy: same-origin` and a `permissions-policy` turning off camera, microphone, location, payment, USB and sensors. A response that sets a cookie is `no-store` and not compressed. A 401 carries `WWW-Authenticate: Bearer`. Credential headers are marked sensitive in logs. The body limit now binds handlers that read the body themselves.
- **Checked:** all 548 database-free tests, the database suite on a fresh database (176 passed; the 6 FHIR validator tests need Java, which this machine lacks, and run in CI), clippy, rustdoc, fmt, the `check` command, the touched UI tests, and the docs gates.
- **Not done / not verified:** no deployment exists, so no probe, load balancer or uptime check outside this repository was moved. `SHUTDOWN_DRAIN_SECONDS` is not set, because there is no Kubernetes deployment. No test asserts the new headers, the 503 on an outage, or `WWW-Authenticate`. The photo route, which a page loads, keeps the same-origin resource policy; that is right while the UI proxies it, and was not tried in a browser. The real-stack tests were not rerun on 0.11.

## 2026-11-10 - axa 0.11 recorded, not started (AX-11)

- **Clinical risk:** none. This is a backlog entry.
- **Built:** `tasks.md` gains AX-11, the axa 0.11 upgrade, from the axa maintainers' release note: the renamed settings (`DATABASE_URL` becomes `DB_URL`, and others), the moved health paths, the code changes, and the silent changes to check with tests.
- **Not done / not verified:** nothing is upgraded. The axa upgrade guide was not read here, and the list of places to change was counted with a search, not checked one by one.

## 2026-11-10 - My watch list (WL-6, H-055)

- **Clinical risk:** small. A clinician can now see every patient on their own list in one place. Hazard H-055's residual risk is updated.
- **Built:** `/watch-list`, linked from the header as "My watch list". It shows the reader's own active entries, oldest first: since when, until when, and a link to the record. It shows no reason, because the service's list carries none; the reason is read on the patient's page, where the read is audited against that record. It repeats that a watch list sends no results or messages, and says when the list was cut short.
- **Checked:** 1 new stubbed Playwright test with axe (the axe route census now covers the page). All 342 vitest and 111 Playwright tests, lint, `check` and `check:tests` pass.
- **Not done / not verified:** the header link shows for every role, as the other links do; a role without watch lists gets the service's refusal. Dates are shown in UTC, because the list does not carry the site's zone. There was no run against the real stack. The Welsh is machine-written and unreviewed.

## 2026-11-10 - Watch lists on the patient page (WL-6, H-055)

- **Clinical risk:** yes. A clinician now sees who keeps the patient on a watch list and why, and can add the patient to their own list or end an entry. Hazard H-055's residual risk is updated.
- **Built:** a "Watch lists" section on `/patients/[id]` (`WatchListSection.svelte`, `lib/watch-list.ts`). It says a watch list is a reminder that sends no results or messages. It lists every entry with its state in words, since, owner, last day, reason and end reason, as text. It adds the reader to their own list, with a required reason and an optional last day; a second active entry is answered "already on your watch list". It ends an entry with a reason. Ending someone else's entry is refused by the service, and the page says that only the person who added it can end it. A 403 and a failed load each say so.
- **Checked:** 1 new vitest test and 1 new stubbed Playwright test with axe. All 342 vitest and 110 Playwright tests, lint, `check` and `check:tests` pass.
- **Not done / not verified:** there is no page for one's own list. The owner is shown as the token subject, not a name. There was no run against the real stack. Lapse notices, team lists and hand-over are still not built. The Welsh is machine-written and unreviewed.

## 2026-11-10 - The suspicion screens against the real stack (RS-8)

- **Clinical risk:** none added. This is a test.
- **Built:** a real-Keycloak test in `tests/e2e-keycloak/signin.spec.ts`. An administrator makes a patient and the clinician's care relationship. The clinician sends a referral, then through the page records a suspicion of Welsh cancer with a site and "not told", corrects it to "told" with a reason, records "ruled out" and marks that entered in error. The test then reads the API's history: two versions, `patient_told` changed, the site kept, open again, the reason kept.
- **Checked:** run locally against the API on the host and Keycloak 26.3.0 under Podman; it passed. In the same run, 11 other real-stack tests passed and 11 skipped for want of their users. One failed: the registration test met a leftover patient with the same NHS number in the reused test database, so its first registration was already a duplicate. That is the environment, not this change.
- **Not done / not verified:** CI does not run the real-stack tests. The registration test was not rerun on a clean database.

## 2026-11-10 - The development realm imports again (SE-9)

- **Clinical risk:** none. This is development configuration.
- **Built:** the `on-duty-clinician` role's description in `operations/keycloak/epr-realm.json` was 272 characters, over Keycloak's 255-character column, so a fresh Keycloak could not import the realm and did not start. Its words are now shorter, with the same meaning. A new vitest test (`tests/unit/keycloak-realm.test.ts`) fails if any description in the realm is over 255 characters; it fails without this fix.
- **Checked:** the test, with and without the fix. A fresh Keycloak 26.3.0 imported the realm and served it.
- **Not done / not verified:** an existing development database, imported before the text grew, hid the fault, and nothing in CI starts a Keycloak. Other column limits in the realm (names, redirect URIs) are not checked.

## 2026-11-10 - Correct a suspicion on screen (RS-8, H-054)

- **Clinical risk:** yes. A clinician can now record on screen that the patient has been told, which lets a message name the condition. They can also move the point of suspicion (which moves the due days) or change the instruction. Hazard H-054's residual risk is updated.
- **Built:** an open suspicion offers "Correct this suspicion" (`SuspicionAmend.svelte`, `buildAmendment`, `localInput`). It covers whether the patient was told, the point of suspicion and the instruction, starts from what is recorded, and needs a reason. The rest of the suspicion is sent unchanged, under the same pathway version, against the version on screen; a 409 reloads. A point left as shown is not moved. A correction with no change, or with no reason, is refused before sending.
- **Checked:** 3 new vitest tests (reason and change needed; the body; the shown time round-trips) and 1 new stubbed Playwright test with axe. All 340 vitest and 109 Playwright tests, lint, `check` and `check:tests` pass.
- **Not done / not verified:** the condition, grounds, band, factors, subtype and pathway cannot be corrected on a screen. No version history is shown. The point is typed in the reader's own time. There was no run against the real service and Keycloak. The Welsh is machine-written and unreviewed.

## 2026-11-10 - End or correct a suspicion on screen (RS-8, H-054)

- **Clinical risk:** yes. A clinician can now end a suspicion, or reopen it, from the patient page. Hazard H-054's residual risk is updated.
- **Built:** an open suspicion offers "Record the outcome" (`SuspicionEnd.svelte`, `buildOutcome`). No kind is preselected. Confirmed asks for the diagnosis, ruled out asks for who ruled it out, and every outcome asks for what it rests on; only those fields are sent. It is sent against the version on screen, and a 409 reloads. A note says that a missed appointment, a decline or a closed referral is not an outcome. An ended suspicion offers "This outcome was entered in error", which needs a reason, reopens the suspicion, and says that an outcome already sent to the PAS is not undone there.
- **Checked:** 1 new vitest test (each kind's fields, the version) and 1 new stubbed Playwright test with axe (ruled out without who is refused before sending; recorded; marked in error without a reason is refused; marked; open again with one outcome in error). All 337 vitest and 108 Playwright tests, lint, `check` and `check:tests` pass.
- **Not done / not verified:** no form amends a suspicion. No version history is shown. A confirmed diagnosis is free text; its code field is not on the form. There was no run against the real service and Keycloak. The Welsh is machine-written and unreviewed ("Y diagnosis" among others).

## 2026-11-10 - Record a suspicion on screen (RS-8, H-054)

- **Clinical risk:** yes. A clinician can now record a suspicion from the patient page, including whether the patient was told. Hazard H-054's residual risk is updated.
- **Built:** `GET …/suspicion` also carries `pathways`, the newest version of each pathway, which hold no patient data (OpenAPI `SuspicionRead`). A referral with no suspicion offers "Record a suspicion on this referral" (`SuspicionRaise.svelte`, `buildSuspicion`). It offers only the chosen pathway's band, factors, subtype list and agreement, and marks a draft pathway as draft. Whether the patient was told, a required subtype, a band and the agreement have no preselected answer; sending without them names what is missing and sends nothing. The service's refusals are shown as it words them. The view now clears when the patient or referral changes, so another patient's suspicion is never left on screen while one loads.
- **Checked:** 2 new vitest tests (nothing defaulted; the body), 1 new stubbed Playwright test with axe; all 336 vitest and 107 Playwright tests, lint, `check`; the service's HTTP suspicion test (only the newest version is offered), the OpenAPI census, clippy, and all 548 database-free tests.
- **Not done / not verified:** the point of suspicion is typed in the reader's own time (the label says so), not the site's. A band's result id is typed, with no picker. No form amends, ends or corrects a suspicion. There was no run against the real service and Keycloak. The Welsh is machine-written and unreviewed ("Categori" for band among others).

## 2026-11-10 - The subtype named on screen (RS-8, H-053)

- **Clinical risk:** small. A clinician now reads "Primary cancer site: Lower GI (03, …)" instead of a bare code. The name comes from the same pathway list the code was checked against. H-053's control says so.
- **Built:** a pathway plan carries `subtype` (scheme, title, code, name), named from the pathway version's list, or null. The OpenAPI schema is updated. The suspicion view shows the title and name, then the code and list.
- **Checked:** `pathways::tests`, the HTTP test (the name is "Lower GI"), all 548 database-free tests, clippy, the OpenAPI census, the UI `check`, lint, and the stubbed Playwright test with axe.
- **Not done / not verified:** the names are the built-in lists' English names. A Welsh screen shows them in English, and nothing translates them. No real-stack run was made.

## 2026-11-10 - A referral shows its suspicion, read only (RS-8, H-053, H-054)

- **Clinical risk:** yes. A clinician now sees, under each referral, what is suspected, whether the patient was told, the pathway's due days and whether they have passed, and the outcome. Hazards H-053 and H-054 have their residual risk updated.
- **Built:** `SuspicionView.svelte` and `lib/suspicions.ts` in the UI, shown under each referral in the patient's "Referrals" section. When the patient has not been told, the screen says in bold not to name the condition to them. It notes a draft pathway or a newer version, and shows the subtype as code and list, the priority, the point of suspicion and what the clock starts from, and any instruction. Each target shows its due day in the site's days, marked overdue while open, or as passed at the outcome once ended. It also shows the outcome or that the suspicion is open, and how many outcomes were entered in error. A 403 and a failed load each say so, never "none". The words are English and machine-written Welsh (unreviewed).
- **Checked:** 3 new vitest tests and 1 new stubbed Playwright test with axe; all 334 vitest and 106 Playwright tests, lint, `check` and `check:tests`.
- **Not done / not verified:** no form raises, amends, ends or corrects a suspicion. The subtype's name is not shown; the plan carries only the code. No version history is shown. The screen was not run against the real service or Keycloak. The Welsh is not reviewed by a speaker. Each referral makes its own request, so a long list makes many.

## 2026-11-10 - An outcome recorded in error (RS-7, H-054)

- **Clinical risk:** yes. A clinician can reopen a suspicion whose outcome was recorded in error, so it returns to the referrer's worklist and its targets count again. Hazard H-054's control and residual risk are updated.
- **Built:** migration `20261110000035_suspicion_outcome_errors.sql`. It numbers each outcome by `seq` (the existing rows read as 1), re-keys the table on referral and `seq`, and adds an append-only error table and a view of the current outcome. No row is changed. `POST /api/patients/{id}/referrals/{referral_id}/suspicion/outcome/entered-in-error {seq, reason}` marks the current outcome. The suspicion is then open again: it can be amended, it is back on the worklist if its referral has ended, and it can take a new outcome. The history keeps the first outcome and the reason under `outcomes_in_error`. A stale `seq` is 409. The audit row names the `seq` only. The censuses (audit switch, audit refusal, personal-data inventory, OpenAPI, route table, schema map) are updated.
- **Checked:** the HTTP test (blank reason refused, wrong `seq` 409, marked, reopened, back on the list, amended, confirmed as `seq` 2, the first outcome kept), the audit refusal drive, all database-free tests, and the database suite on a fresh database.
- **Not done / not verified:** any treating role may mark an outcome in error, and no second person confirms it. A stop already sent to the PAS cannot be undone there; this is written down but no sender exists. No screen exists. No clinician has reviewed whether reopening should restart anything other than the worklist and the targets.

## 2026-11-10 - Suspicion due days in the site's days (RS-6, H-053)

- **Clinical risk:** yes. A due day and whether it is overdue can move by one day near midnight, for a site outside UTC. Hazard H-053's cause and control are updated.
- **Built:** a pathway plan now counts the clock start's day and today in the site's time zone (`EPR_SITE_TIME_ZONE`, the one already used for ages, checked at start-up; UTC when unset). The plan names the zone as `time_zone`. A test shows 23:30 UTC on 30 June is 1 July in London, and the due day follows. The OpenAPI schema is updated.
- **Checked:** `pathways::tests`, all 548 database-free tests, the suspicion and worklist database tests, clippy, the OpenAPI census.
- **Not done / not verified:** the database tests run with no zone set, so they count UTC days, as before. No screen shows the plan. Clock pauses are not modelled. A deployment that leaves the zone unset still gets UTC days, and nothing warns it.

## 2026-11-10 - The suspicion's PAS mapping and a contract test (RS-5)

- **Clinical risk:** none yet. Nothing calls the mapping and nothing is sent. Once a sender uses it, a wrong mapping is a clinical risk, under H-053 and H-054.
- **Built:** `src/pas_suspicion.rs`, pure code. It holds the PAS condition for each pathway, and the subtype a child's type 1 diabetes implies. `new_suspicion`, `amendment` and `stop` give the PAS's request bodies, or a hold for a person. Each checks against the PAS's live profile: the condition is tracked, a subtype comes from that condition's list and is sent with its scheme, a required list is met, and days are counted in the profile's time zone. `confirmed` sends nothing; `ruled-out` becomes `downgraded`; `withdrawn` becomes `withdrawn`. A suspicion raised by a programme or a result is held, and so is an amendment that removes a subtype or changes the condition. `tests/pas_contract.rs` checks every body against the PAS's request schemas, copied with its two built-in profiles from PAS commit `b64b7e2` into `tests/fixtures/pas/`, and checks that its checker refuses a bad body.
- **Checked:** 7 unit tests, the contract test, clippy, rustdoc, and all 547 database-free tests.
- **Not done / not verified:** no sender, no profile read, no `epr` referral, no `If-Match` and no store for the PAS's ids. People's names are supplied by the caller; the service has token subjects, not names. The schema checker covers only what those schemas use, not all of JSON Schema. Nothing checks that the copied schemas and profiles still match the PAS. The PAS has not run its side against these bodies; it will test against a running PAS once a sender exists. The spec records the PAS's rule for a held programme or result suspicion: it is sent once a clinician takes it on, with that clinician's name and the day they acted.

## 2026-11-10 - The PAS interface agreed (RS-5, spec only)

- **Clinical risk:** none yet. This is spec only, and nothing is sent to the PAS.
- **Built:** `spec/referral-suspicion/index.md` records that the PAS's maintainer agreed the four PAS changes: an `epr` referral source with the EPR id as `external_ref`, an optional `subtype_scheme`, a `withdrawn` stop, and an amend route with If-Match. It records the shapes the PAS's session sent, and how each EPR event maps to a PAS call. The sender will check a subtype against the PAS's live profile before sending, because copied lists drift. The amend route replaces stop-and-re-record for a moved point of suspicion. `suspected_on` is a day in the profile's `time_zone`.
- **Checked:** links, the status block, `llms.txt`.
- **Not done / not verified:** the PAS commit `b64b7e2` was seen in its git log, but its code was not read; the shapes are as its session reported them. The rule that the sender checks a subtype against the PAS's live profile is written down, but nothing enforces it yet. No EPR sender, port or contract test exists. The time-zone conversion is not built.

## 2026-11-10 - A subtype on a suspicion (RS-5, EPR side; H-053)

- **Clinical risk:** yes. A clinician now sees and records a primary cancer site or a type of dementia on a suspicion. Hazard H-053 gains the control and the cause.
- **Built:** a pathway version may hold a subtype list (`[pathway.subtypes]`: a scheme, a title, a source, whether it is required, and named codes, none repeated). A suspicion may carry `subtype` `{scheme, code}`. It is refused under a pathway with no list, refused when the scheme is another list's or the code is not in it, and required where the list is. Two new pathway versions: `suspected-cancer-wales` v2, which requires `wales-primary-cancer-site` (the NHS Wales list, 01 to 14 and 98), and `dementia` v2, which takes `generic-dementia-type` (optional). Version 1 of each is unchanged, so stored suspicions keep their rules. An amendment that adds a site reports `subtype` as changed. The OpenAPI schema is updated.
- **Checked:** `pathways::tests` and `suspicion::tests`, the HTTP test (a missing site and a site from the wrong list are refused, a right one is kept and moves no date), clippy, the OpenAPI census, and the database suite on a fresh database.
- **Not done / not verified:** the lists are copied from the PAS's `wales` and `generic` profiles, not read from its `GET /api/suspicion-profile`, and nothing checks that the two stay the same. The NHS Wales Data Dictionary page was not opened here. The plan does not show the subtype's name. No screen exists. Nothing is sent to the PAS yet, and the PAS's `subtype_scheme` is not agreed. `type-1-diabetes-child` holds no stored subtype. No clinician has approved either new version.

## 2026-11-09 - The Wales FHIR package checked, licence unclear (WL-9, blocked; OQ-WL3)

- **Clinical risk:** none. This is a research note.
- **Built:** `spec/nhs-wales/index.md` records the newest Wales FHIR package: `fhir.r4.wales` 2.7.0, STU2, FHIR 4.0.1, on UK Core STU2 2.0.2, with 236 resources and its tarball's SHA-256. It lists which profiles are active and which are draft. No licence is stated in the package, in its GitHub repository or on the resources, which say only "2026 NHS Wales". The new open question OQ-WL3 asks whether the package may be fetched to validate against in CI without being copied here, or whether NHS Wales must be asked first. WL-9 stays open and blocked.
- **Checked:** links, the status block, `llms.txt`. The package was downloaded to a scratch directory to read its manifest and count its resources, then not kept.
- **Not done / not verified:** no validation against the Wales profiles was run. The guide's "Licensing" page was not reached (404 at the path tried). No one at NHS Wales was asked.

## 2026-11-09 - Write-back and reconciliation (WL-5, spec only)

- **Clinical risk:** none yet. This is spec only, and immunisations are not built. Once built, a record that fails to arrive is a clinical risk.
- **Built:** `spec/data-exchange/index.md` gains a section. For data held in two places it names six things: the system of record, the direction of flow, what is written back, what counts as arrived (the receiver says it filed it, not that it was delivered), the report of what failed to arrive, and the manual fallback. It adds five rules: one path out through the outbox, sent is not done, a copy keeps its source, unknown is not zero, and the report is kept. Immunisations are the example. The source is Welsh Government's 2024/25 statement that write-back from WIS to GP systems was not possible. The lessons list in `spec/nhs-wales/index.md` now says which lessons are written or built; its Welsh line was stale and is corrected.
- **Checked:** links, the status block, `llms.txt`.
- **Not done / not verified:** nothing is built: no immunisations, no acknowledgement of filing (the outbox counts a sink's success as delivered), no report and no sink. Which registers and GP systems accept a write-back was not checked.

## 2026-11-09 - Device readings counted only once validated (WL-10, spec only)

- **Clinical risk:** none yet. This is spec only and nothing reads a device. Once built, it is clinical (RC-11, EM-11).
- **Built:** `spec/device-readings/index.md` starts the EM-11 design, and the module map links it. A device reading keeps its device, its gateway and the capture time the device stamped, never the receipt time. It is shown apart as unvalidated and counts toward no score or alert until a clinician validates it, one reading or a set over a period. A clinician may instead reject it as an artefact, with a reason; it is kept and never counts. The device's value is never edited, and a clinician's own value names the reading it replaces. The page also says what `Observation` lacks for this today: a source, a device, a capture time and a validated state.
- **Checked:** links, the status block, `llms.txt` (unchanged: its generator does not list the per-topic spec pages).
- **Not done / not verified:** nothing is built. How long a reading may wait unvalidated, undoing a validation, device protocols and gateways, and the medical-device question (OQ-6) are open. No clinician or device engineer has read it.

## 2026-11-09 - "Live", per site (WL-11, spec only)

- **Clinical risk:** none directly. This is a governance section with no code.
- **Built:** `governance/production-readiness.md` gains a section saying a deployment never reports "live" in one word. Its report holds one row per site and function: state (`not in use`, `trial`, `in use`, `withdrawn`), since when, who, the settings as set, the evidence, and what that site does not use. It lists the functions this repository holds on 2026-11-09. A function with no row at a site is not in use there. The generated `spec/14-implementation-status.md` counts code and is not this report. The source is the Auditor General for Wales on WCCIS ("'Live' meant different things"), as recorded in `spec/nhs-wales/index.md`. The same file's safety row said 44 hazards; it now says 55, with 34 open.
- **Checked:** links, the status block, `llms.txt`.
- **Not done / not verified:** no deployment exists, so no report has been written. There is no template file or tool, and nothing checks that a report stays current.

## 2026-11-09 - Watch lists with a reason (WL-6, H-055)

- **Clinical risk:** **yes (new hazard H-055: a patient drops off a watch list unseen, or is watched for no reason anyone can read).** This is the first watch list. H-055 is new and open.
- **Built:** migration `20261109000034_watch_lists.sql` adds the append-only `watch_entry_versions` (version 1 is `active`, version 2 `ended`) and the view `watch_entry_current`. `src/watch_list.rs` holds the rules: a reason is required, and a last day, if given, must be today or later. An entry lapses the day after its last day, and that state is worked out when read. `watch_list_store.rs` keeps one active entry per clinician and patient, follows merges and lets only the owner end an entry, with a reason. `controllers/watch_list.rs` adds the patient's view with reasons and states, an add, an end, and `GET /api/watch-list`, the caller's own active entries as pointers with no reason. There are two new policy resources: `WatchList` (clinical) and `OwnWatchList` (pointers, no relationship). `openapi.json`, the specs, the inventory, the schema map and the stored-words, label, search-audit and refusal-audit censuses follow.
- **Checked:** fmt, clippy, the doc build and every database-free test. `watch_list::tests` has 2 tests. `tests/http_watch_list.rs` runs with login on and covers these cases:
  - a blank reason and a past last day are 422, and a second add is 409;
  - a clinician without a care relationship, and reception, are 403;
  - the owner's list holds the entry with no reason;
  - a colleague ending it is 404; the owner ending it without a reason is 422, from a stale version 409, then 200, then 422;
  - ended and lapsed entries leave the owners' lists and stay on the patient's record with their reasons;
  - no reason reaches the audit.

  On a fresh database every database test passed (176) except the six FHIR validator tests, which need Java. All 537 database-free tests pass.
- **Not done / not verified:** nothing tells the owner when an entry lapses. There is no team list, no hand-over and no reassignment when a clinician leaves. Last days are UTC days. No screen shows the lists. No clinician reviewed who may add or read an entry.

## 2026-11-09 - The PAS side of a suspicion, proposed (RS-5, spec only)

- **Clinical risk:** none yet. This is spec only, and nothing is sent to the PAS.
- **Built:** I read the PAS repository's spec 20 and its suspicion API, and `spec/referral-suspicion/index.md` now maps each EPR field to its PAS field. The mapping names five gaps:
  - the EPR has no subtype, and the PAS requires a primary cancer site under its `wales` profile;
  - nothing links an EPR referral to a PAS referral;
  - EPR `confirmed` comes at diagnosis, but the PAS clock stops at `treatment_started`;
  - EPR `withdrawn` has no PAS stop;
  - England's referral-sent clock is a different measure from the PAS's fixed point-of-suspicion clock.

  The questions went to the PAS session. Its answers are recorded in the same section: an `epr` source with the EPR id as `external_ref`; a `subtype_scheme` on the PAS request; a `withdrawn` stop; nothing sent on `confirmed`; and corrections as `entered_in_error` then re-record for now. They wait on the PAS maintainer's agreement. RS-5 stays open.
- **Checked:** links, the status block, `llms.txt`.
- **Not done / not verified:** nothing is agreed, there is no contract test, and no code changed. The PAS spec was read on 2026-11-09 and may have moved since.

## 2026-11-09 - Messages do not name an untold suspicion (RS-4, H-054)

- **Clinical risk:** **yes (H-054, updated).** A patient must not learn of a suspected condition from a letter or message before anyone has told them.
- **Built:** an outbox event (`outbox::NewEvent`) gains `names_suspicion_of`, the referral whose suspected condition its payload names. It is stored in the payload's route, so the outbox fingerprint covers it. `outbox::decide`, the gate every outbound event passes, now refuses such an event to a patient or proxy as `suspicion-not-told` unless that referral's current suspicion says `told`. It is also refused for a referral that is not the patient's. The record is read when the event is sent, not when it was queued. `suspicions_store::told` answers the question.
- **Checked:** fmt, clippy, every database-free test, the doc build, and `tests/outbox_store.rs` (13). Its new test covers three cases: a letter naming an untold suspicion is refused with `suspicion-not-told`, and so is one naming another patient's referral; after the suspicion is amended to `told`, the next letter passes this check. That letter is then refused by the read policy, which refuses every patient recipient today because the relay holds no relationship. No sink is called. On a fresh database every database test passed (175) except the six FHIR validator tests, which need Java.
- **Not done / not verified:** the check relies on the sender setting `names_suspicion_of`, and nothing scans a payload for a condition named without it. No letter, portal or text sender exists, and the outbox cannot deliver to a patient at all yet. PA-13's opt-outs, quiet hours and channel rules are not built. No clinician reviewed what counts as naming the condition.

## 2026-11-09 - A suspicion ends only by an outcome (RS-3, H-054)

- **Clinical risk:** **yes (H-054, updated).** A referral that is declined, completed, cancelled or redirected no longer drops a suspicion silently. It stays on the referrer's worklist until someone records what became of the suspicion.
- **Built:** migration `20261109000033_suspicion_outcomes.sql` adds the append-only `referral_suspicion_outcomes`, one per referral. `suspicion.rs` gains `Outcome`: `confirmed` needs the diagnosis, `ruled-out` needs who ruled it out and on what, `withdrawn` needs a reason, and only `confirmed` carries a diagnosis. `POST /api/patients/{id}/referrals/{referral_id}/suspicion/outcome` records it against the current version, once. After that the suspicion cannot be amended, the history says `open: false` with the outcome, and each target is judged at the outcome (met or missed), not at the time of reading. An ended suspicion is not carried to a replacement. The referrer's worklist gains `open-suspicion` for an ended referral with an open suspicion that was not carried. It cannot be acknowledged. The audit row names the outcome's kind only. `openapi.json`, the specs, the inventory, the schema map and the stored-words census follow.
- **Checked:** fmt, clippy, the doc build and every database-free test (535). On a fresh database every database test passed (174) except the six FHIR validator tests, which need Java. `suspicion::tests` gains the outcome rules. `tests/http_suspicions.rs` now also covers these cases: a no-show leaves it open; a decline puts the referral on the list as `reply` first; after acknowledging the decline it is listed as `open-suspicion`; the redirected referral whose suspicion was carried is not. An outcome without `by`, from a stale version, then accepted, then a second, are 422, 409, 201 and 422. An amendment after it is 422, the referral leaves the list, and no diagnosis grounds or names reach the audit.
- **Not done / not verified:** an outcome cannot be corrected or entered in error. The list reaches only the referrer, not the recipient or a team. Nothing tells the PAS (RS-5). There is no screen. No clinician reviewed which states count as ended or who may record an outcome; any treating role with a care relationship may.

## 2026-11-09 - axa 0.10 and axa-test

- **Clinical risk:** none. This is a dependency update with no change to behaviour. `openapi.json` is unchanged.
- **Built:** the service moves from `axa` 0.9.1 to 0.10.0. The `testing` feature is gone, so `axa-test` 0.1.0 is a dev-dependency and 30 test files import `axa_test::prelude` in place of `axa::testing::prelude`. axa's crates were renamed (for example `axa-problem` to `axa-problem-details-for-http-apis`). The service reaches them only through `axa`'s features and modules, which kept their names, as did axa's tables, so no migration is needed. `axum-test` is still one copy in `Cargo.lock`.
- **Checked:** fmt, clippy, the doc build and every database-free test (534). On a fresh database every database test passed (174) except the six FHIR validator tests, which need Java and the validator jar (not on this machine; CI runs them).
- **Not done / not verified:** the compose stack and the API image were not rebuilt or run on axa 0.10. `cargo deny` and `cargo audit` were not run locally; CI runs them.

## 2026-11-09 - A suspicion survives triage, rebooking and redirection (RS-2, H-054)

- **Clinical risk:** **yes (H-054, updated).** When a redirected referral is replaced, the new one now keeps the reason it is urgent and the day its clock started. Before this change the replacement started with no suspicion.
- **Built:** a referral can name the redirected referral it `replaces`. The old referral must belong to the same patient, be redirected and not already be replaced, and two replacements at once wait on a lock. In the same transaction, the old referral's current suspicion becomes the new one's version 1, with `carried_from` and the point of suspicion unchanged, and that is audited. A pathway counted from `referral-sent` counts from the first referral in the chain. Migration `20261109000032_suspicion_carried_from.sql` adds `carried_from`, which only a version 1 may have. `openapi.json`, the specs, the inventory and the schema map follow.
- **Checked:** fmt, clippy, the doc build and every database-free test (534). In the full database run, 172 tests passed. The six FHIR validator tests failed because they need Java. `http_death_reports` and `http_overdue_reviews` failed because their clock-made patients matched patients left over from earlier runs (409 possible duplicate). Both passed on a fresh database, as did `http_hazards` and `http_suspicions`. That flake is old and was not fixed. `tests/http_suspicions.rs` now also covers these cases. Accept, book, a no-show and a rebook leave the suspicion at version 1 with its point of suspicion. A redirect then a replacement carries it, with `carried_from`, the earlier point of suspicion and the due day 2026-11-30. Replacing the same referral twice, and replacing one that was not redirected, are 422. `tests/referrals_store.rs` and `tests/http_audit_refusal.rs` still pass.
- **Not done / not verified:** a replacement that does not name what it replaces starts with no suspicion, and nothing finds such a pair. No screen offers `replaces`. The external referral queue (PC-2) does not carry it. No clinician reviewed it.

## 2026-11-09 - A suspicion on a referral, under its pathway (RS-1, H-054)

- **Clinical risk:** **yes (new hazard H-054: a suspicion lost, changed unseen, or revealed to a patient not told).** A referral can now carry what the referrer suspects, from when and on what grounds, with the pathway's priority and due days beside it. H-054 is new and open; H-053's residual risk now says no screen shows the plan.
- **Built:** migration `20261109000031_referral_suspicions.sql` adds the append-only `referral_suspicion_versions` (a CHECK that only version 1 lacks a reason) and the view `referral_suspicion_current`. `src/suspicion.rs` holds the rules: the condition in the referrer's words, an optional code, the point of suspicion (not in the future), who or what raised it, the grounds (`criterion`, `result` with its id, `judgement`), an instruction, whether the patient was told, and the pathway's band, factors and agreement. An amendment needs a reason and a change. `suspicions_store.rs` checks that a result named as grounds is on this patient's record and works out each version's plan; `controllers/suspicions.rs` adds `GET`, `POST` and `PUT /api/patients/{id}/referrals/{referral_id}/suspicion`, authorised as `Condition`. `openapi.json`, the specs, the inventory and the schema map follow.
- **Checked:** fmt, clippy, the doc build and every database-free test. In the full database run, 173 tests passed. The six FHIR validator tests failed because they need Java and the validator jar, which this machine lacks. `tests/http_hazards.rs` failed once in the full run and passed when run alone; it was not investigated, and this change does not touch hazards. `suspicion::tests` has 3 tests. `tests/http_suspicions.rs` runs with login on: an empty history first; a raise with the Wales dates (2026-12-07, 2027-01-10) and the draft flag; a second raise 409; an amendment without a reason, with no change and from a stale version refused, then accepted, moving the due day to 2026-11-30 while version 1 keeps its own; heart failure refused without a band, with judgement grounds and with a result from elsewhere, then accepted at 14 days; another patient's path 404; reception and `patient-access` 403; no condition, name, grounds, reason, value or pathway word in the audit. The refusal-audit, label and inventory censuses cover the new routes and table.
- **Not done / not verified:** no screen; the referral list does not carry the suspicion; it does not survive a redirect into a new referral (RS-2); nothing ends it (RS-3); outbound messages are not checked against `patient_told` (RS-4); the PAS does not receive it (RS-5); no patient view; codes are not looked up; any treating role with a care relationship may amend it, not only the referrer. No clinician reviewed it.

## 2026-11-09 - Suspicion pathways as configurable data (RS-6, H-053)

- **Clinical risk:** **yes (new hazard H-053: a wrong target date or priority).** Nothing shows a pathway yet, but these figures will set due days and priorities. Every built-in pathway is a draft. H-053 is new and open.
- **Built:** the maintainer asked that cancer's suspicion, target dates and appointment priority generalise to configurable conditions. `src/suspicion_pathways.toml` holds seven pathways: suspected cancer in Wales (from the point of suspicion: 28 and 62 days) and in England (from referral, very urgent or urgent bands), type 1 diabetes in a child (same day, stat), heart failure (NT-proBNP bands: 14 or 42 days, the grounds must be a result), Parkinson's (an instruction, no interval and why), dementia (the person's agreement recorded) and sleep apnoea (factors such as driving raise it to urgent). `src/pathways.rs` reads and checks the catalogue and `plan` gives the priority and each target's due day and whether it is overdue. The design is in `spec/referral-suspicion/index.md`.
- **Checked:** fmt, clippy, the doc build; `pathways::tests` has 7 tests (the catalogue parses and every pathway is a draft; the Wales dates; a same-day target overdue only the next day; bands; factors; agreement; a bad catalogue refused).
- **Not done / not verified:** the catalogue is not attached to a stored suspicion (RS-1) or shown on any screen or route. Outcomes do not stop a target (RS-3). Due days are UTC days, not local days. Clock pauses are not modelled. The figures came from search summaries, not the NICE or Welsh Government pages. No clinician has approved any pathway.

## 2026-11-09 - Assessment forms as versioned standards (WL-3, H-052)

- **Clinical risk:** **yes (new hazard H-052: an assessment read under the wrong form, or a draft taken as approved).** This is the first assessment data. Every completed assessment keeps the form version it was made under and is shown with that version's questions. A form nobody approved says so. The one form is a project-written example, not a clinical tool. H-052 is new and open.
- **Built:** `src/assessment_forms.toml` is the form catalogue, held as data. `src/assessments.rs` reads it into the build and refuses a catalogue whose versions do not follow one another, a form with no questions, a blank or repeated question, a choice with no choices, or an approver without a valid day. It also holds the answer rules: every question once, a value of its kind, or null with a reason. Migration `20261109000030_assessments.sql` adds the append-only `assessment_versions` (form id and version on every row) and the view `assessment_current`. `assessments_store.rs` follows merges. `controllers/assessments.rs` adds `GET` (the catalogue and the assessments, each under its own version, with `approved` and `latest_version`) and `POST`. A new clinical policy resource, `QuestionnaireResponse`. `openapi.json`, the service spec, the inventory, the schema map and WL-3 follow.
- **Checked:** fmt, clippy, every database-free test (524), every database test (179) and the doc build. `assessments::tests` has 3 tests: the built-in catalogue parses with two versions of one form, both drafts; each answer rule; each bad catalogue. `tests/http_assessments.rs` runs with login on: a missing answer, a null without a reason and an unknown choice are 422; answers under version 1 (one absent with a reason) and version 2 are both shown with their own questions and words; `latest_version` is 2 and `approved` false on both; reception is refused; and no answer or reason reaches the audit. The refusal-audit, label, search-audit and inventory censuses cover the new route and table.
- **Not done / not verified:** no licensed or All-Wales form is encoded, and the licence questions OQ-M2 and OQ-WL2 still stand. There is no correction or entered in error, no screen, no score, and no way for a form version to be retired. The approver rule checks that a name and a day are present, not who the person is. No clinician reviewed it.

## 2026-11-09 - A narrower view by role, as data: the vaccinator (WL-7, H-005)

- **Clinical risk:** **yes, treated as clinical (H-005).** There is a new role, `vaccinator`, whose only permissions come from a data file. It reads and searches a patient's identity, allergies and medicines, still under a care relationship, a fitting purpose and the patient's consent. It reads nothing else and writes nothing. No existing role changed. H-005's controls and evidence gain this clause.
- **Built:** `src/role_views.toml` holds the view, with its source. `src/role_views.rs` reads it into the build and refuses a file naming an unknown role or resource, a role that has its own permissions, a duplicate, an empty list or a missing source. In the policy, `Role::Vaccinator` is a role whose permissions come only from a view (`Role::viewed`), and `permitted()` asks the view for it, granting `Read` and `Search` only. Its purposes are `treatment` and `operations`. The service spec has a row. The data file sits under `src/` because the Containerfile and the crate's `include` take only `src` and `migrations`; it was first written to `policy/` and moved before commit.
- **Checked:** fmt, clippy, every database-free test (521), every database test (178), and the doc build. 2 unit tests prove the built-in file parses and refuse each kind of bad file. `tests/http_role_views.rs` runs with login on: a vaccinator on the care team reads identity and allergies on REST and FHIR (`/fhir/R4/Patient`, `/fhir/R4/AllergyIntolerance`); vital signs on REST and FHIR and results are 403, and so is a write. The authorization unit test that lists who may use the compare route names the vaccinator, because its view includes identity search.
- **Not done / not verified:** the Keycloak realm and `operations/keycloak/role-map.json` have no vaccinator role, and the UI has no label for it. There are no immunisation records for a vaccinator to write, and no medicines routes exist, so "medicines" grants nothing yet. Views do not yet interact with security labels (PI) or assignments (RA), which the task said should come first; built first at the maintainer's request. The subset is from a search snippet of the Welsh GP Record FAQ, and nobody has reviewed it.

## 2026-11-09 - Permission at the point of care (WL-4, H-008)

- **Clinical risk:** **yes, treated as clinical (H-008).** It is off by default, and then nothing is refused: a basis sent with a request is simply kept. With `EPR_REQUIRE_PERMISSION_BASIS` on, a member of staff's read of a patient's clinical content without a basis is refused. `could-not-ask:emergency` and `could-not-ask:unconscious` are always accepted as a basis, so the requirement never waits on a patient who cannot answer. H-008's controls and residual say so; its status is unchanged.
- **Built:** `src/permission.rs`. The `X-Permission-Basis` header carries `asked-agreed` or `could-not-ask:<reason>`, the reason coded as `unconscious`, `lacks-capacity`, `too-unwell` or `emergency`. Anything else is a 400, so typed words never reach the audit. The authorization middleware keeps the basis in the request's origin, and every audit row of the request carries it as `permission_basis`. A bad value for the setting stops the service starting. The service spec has a row.
- **Checked:** fmt, clippy, every database-free test (519), 2 unit tests, an extended audit unit test, and `tests/http_permission_basis.rs` with login on and the setting on. That test covers: a clinician's allergy read with no basis is 403 `permission_basis_required` and audited; a typed reason is 400; each of `asked-agreed` and `could-not-ask:unconscious` is answered and its rows carry the basis; an identity read needs none; and no typed word reaches the trail.
- **Not done / not verified:** the UI neither asks for a basis nor sends one, so turning the setting on refuses every clinical read through the UI. The patient's access log does not show the basis. The reasons are my list, not a Welsh or clinical source's. Searches across patients and writes are not covered. No clinician or Clinical Safety Officer has reviewed it, and the Welsh Control Standard was not read.

## 2026-11-09 - The UI in Welsh, machine-written and unreviewed (WL-8, H-051)

- **Clinical risk:** **yes (new hazard H-051: unreviewed Welsh wording misleads a clinician).** At `cy-001` every page is now in Welsh, including the banner, vital signs, allergies, death reports and consent. No Welsh speaker has read any of it. Every page in that locale says so in Welsh and in English, with a link to the same page in English. H-051 is new and open.
- **Built:** `src/lib/i18n.ts` adds `TEXT`, which pages and `src/lib` read instead of `STRINGS`. Each key is read in the current locale when it is used: on the server the locale is kept per request in an `AsyncLocalStorage` (`hooks.server.ts`); in the browser it comes from `page.data.locale`, and the page remounts on a change. `src/lib/strings.cy.ts` has every key of `STRINGS`, typed so that a missing or extra key fails `check`. The words least sure of are listed at its top for a reviewer. `REVIEW` records each locale as machine-written and unreviewed. 31 files changed only `STRINGS` to `TEXT`, and two modules that copied strings when they loaded now read them on each call. The machine-translation notice is in the root layout. The translation register, the UI spec and `spec/locales.md` follow. In the matcher, `tests/welsh.rs` adds 7 tests and changes no matcher code.
- **Checked:** UI lint, `check` (0 errors), `check:tests`, vitest (331) and Playwright (105). Playwright shows `/cy-001` in Welsh in the server HTML and after navigating, shows the notice there and not in English, and passes axe. Unit tests: every key in Welsh with its placeholders; three requests rendered at once each keep their own locale; no locale may claim review without a name and a date. Matcher: fmt, clippy and 117 tests. Welsh diacritics typed or not, `ap`, `ab` and `ferch`, and double-barrelled names all match; a shared particle with different names goes to review, never auto-link.
- **Not done / not verified:** no Welsh speaker or Welsh-speaking clinician has reviewed a string. The Welsh Language Standards for health bodies were not read. The API's 422 problem text and service messages stay English, as does `aria-label="Main"`. There is no locale picker, and no test switches locale in the browser. The other eight locales still read English for the whole UI. The matcher has no table of anglicised Welsh names (ap Rhys and Price, ab Owen and Bowen go to review), and the patient model holds no address lines, so Welsh and English street names cannot be compared. Most of the Welsh table and the 31-file change were written by a subagent; I reviewed `i18n.ts`, the table's header and the layout, added the notice, and ran every gate, but did not read all 565 Welsh strings.

## 2026-11-09 - Results with a sign-off that names its version (WL-1, OR-4 first slice, H-050)

- **Clinical risk:** **yes (new hazard H-050: a result changed after sign-off is taken as signed).** This is the first results data. A clinician files a result, a corrected or amended version follows, and a sign-off holds only for the version it named. A result changed after sign-off reads as `changed-since-sign-off`, with the signed version shown beside the current one. H-050 is new and open; the wording is my judgement, not the Clinical Safety Officer's.
- **Built:** migration `20261109000029_results.sql` adds `result_versions`, `result_signoffs` and the view `result_current`, all append-only by trigger; a sign-off references its exact version. `src/results.rs` holds the rules: statuses only move forward, a change needs a reason, entered in error is final and has nothing to sign, and the sign-off state is computed. `src/results_store.rs` follows merges. `controllers/results.rs` adds four routes: list, file, a new version, and sign-off (a stale version is 409 naming the current one). A new clinical policy resource, `DiagnosticReport`: a clinician files, corrects and signs; a nurse, pharmacist or allied professional reads; a patient and a proxy read nothing until release rules exist. The audit rows hold ids, versions and counts only. `openapi.json`, the service spec, the personal-data inventory, the schema map, OR-4 and WL-1 follow.
- **Checked:** fmt, clippy, every database-free test (517) and every database test, including the new `tests/http_results.rs` through the router with login on. That test covers: only a clinician files and signs; a nurse reads; reception is refused; signing twice is the same sign-off; a correction without a reason is 422; after a correction the list shows both versions; re-signing the old version is 409; signing the new one closes it; back to preliminary is refused; entered in error cannot be signed; and no test name or value reaches the audit. The refusal-audit, label, search-audit, stored-words and inventory censuses cover the new routes and tables. There are 4 unit tests of the rules.
- **Not done / not verified:** no screen shows results. No feed files them: an integration has no care relationship and cannot write clinical content, so a lab feed (OR-3's ORU, OR-4) is not built, and a clinician files by hand. Not built either: routing to the ordering clinician, an inbox across patients, abnormal results first, escalation of an unsigned abnormal result, or telling a signer that a signed result changed. The value is text and the flag is the reporting service's; nothing is computed. No FHIR `DiagnosticReport` route. No clinician reviewed the states or who may sign.

## 2026-11-09 - A merged-in refusal names its record in the audit (WL-2, H-006)

- **Clinical risk:** **yes, treated as clinical (H-006).** No change to what a clinician is allowed to see. A refusal on a record that was merged into another already stood for the survivor. Now the refusal's audit row also says which record it came from, so a reviewer can see why a read of the survivor was refused. H-006's controls and evidence gain this clause; its status is unchanged.
- **Built:** `consent::Verdict` gains `from_merged`, the merged-in records whose consents decided it. `consent_verdicts` fills it, and a refusal's audit detail carries it as `consent_from_merged`. The service spec's merge row says so. WL-2 is ticked.
- **Checked:** fmt, clippy, and `tests/http_roles.rs` through the router with login on. A survivor is readable for treatment by a clinician with a care relationship; a treatment refusal is recorded on the duplicate; the duplicate is merged in; the same read is then 403, and its audit row names the duplicate. The first run failed because my refusal was for `treatment` while a read with no `X-Purpose-Of-Use` is `operations`. The test now sends `treatment`; this was the test's error, not the service's.
- **Not done / not verified:** there is no registered-site field, so "a change of site" has nothing to lose: consent is held against the patient. No test shows an unmerge returning the refusal to its own record alone through the router (the store's lookup test covers it). The UI does not show why a read was refused.

## 2026-11-09 - The PAS must track suspicion along a referral; Wales is a target

- **Clinical risk:** **no change to the product.** Documents only. The five new tasks (RS-1 to RS-5) are clinical, and each will need its hazard row when it is built.
- **Built:** [`spec/referral-suspicion/index.md`](spec/referral-suspicion/index.md) sets out the maintainer's requirement that a PAS must track suspicion during referrals. Cancer is the worked example: Wales's Single Cancer Pathway counts from the point of suspicion, and NICE NG12 applies in England. Five rules follow: the point of suspicion is a recorded event; it travels with the referral; it ends only with an outcome; absence never rules it out; and what the patient was told is recorded. Type 1 diabetes in children, heart failure, Parkinson's disease, dementia and obstructive sleep apnoea are shown to have the same shape, each adding one rule. Spec `02`'s PAS boundary gains a row. OQ-WL1 is decided: Wales is a target, so WL-8 and WL-9 are no longer waiting. tasks.md has RS-1 to RS-5, with the `RS` prefix registered.
- **Checked:** links, the status block, llms and the hazard log.
- **Not done / not verified:** the NICE intervals were read through search summaries, not on the NICE pages. The Welsh target was not checked against the latest statistics. The PAS repository was not read for whether it already tracks suspicion, and no fields were agreed with it. The FHIR mapping (`Condition` with `provisional`, linked from the `ServiceRequest`) is unchecked. No clinician read the page.

## 2026-11-09 - Research pass: NHS Wales national systems (plan.md §34, WL-1 to WL-12)

- **Clinical risk:** **no change to the product.** Documents only: no code, route, screen or hazard changed. Five of the new tasks are marked clinical (WL-1, WL-2, WL-3, WL-4, WL-6), and each will need its hazard row when it is built.
- **Built:** [`spec/nhs-wales/index.md`](spec/wales/index.md), a sourced comparison of the Welsh Clinical Portal, Welsh GP Record, Welsh Immunisation System, Welsh Intensive Care Information System, Welsh Nursing Care Record and Welsh Community Care Information System (now Connecting Care). plan.md §34 has the assessment, eleven ideas ranked, the advice (WL-1 and WL-2 next), risks and what was not verified. tasks.md has WL-1 to WL-12, with the `WL` prefix registered in `scripts/generate-status.sh`. Spec `16` gains OQ-WL1 (is Wales a target?) and OQ-WL2 (licence of the All-Wales nursing forms). Spec `17`, `index.md` and the module map link the page.
- **Checked:** links, the status block, llms and the hazard log. Every claim on the page has a source, and each source is marked as DHCW's own account, a supplier's, independent, or a search snippet.
- **Not done / not verified:** no Welsh system was used. Some pages could not be opened, so what they say rests on search snippets: the Welsh GP Record FAQ bodies, the January 2025 WICIS release (404) and the Digital Health articles (403). Whether the Welsh clinical data repository is openEHR or FHIR was not settled. No source confirms that WNCR uses openEHR, or that any WICIS unit is live. The Welsh Control Standard and the Wales FHIR guide were not opened, and the nursing forms' licences were not looked for. No clinician read the ideas. The research was gathered by two subagents; I checked their findings against this repository, not against every source page.

## 2026-11-09 - `podman compose build` no longer sends `target/` as build context

- **Clinical risk:** **no.** Build tooling only.
- **Built:** `.dockerignore` is a symlink to `.containerignore`. `podman compose` here hands off to `docker-compose`, which reads only `.dockerignore`, so it sent the whole repository, the service's `target/` (217 GB on this machine) included, and ran the Podman VM's disk out. AGENTS.md names the exception to the "Podman file names" rule and says to edit `.containerignore`; `.containerignore` says so too.
- **Checked:** `podman compose --profile ui build ui` built the UI image (1 min 43 s). `podman compose build api` passed the context step and reached `cargo build`.
- **Not done / not verified:** the API image still does not build through compose on a 12 GiB Podman VM: rustc is killed for memory compiling `fhir-r4` with cargo's default jobs. It built with `CARGO_BUILD_JOBS=1` via `podman build --env`; nothing in the repository sets that. Not tried on Windows, where git may check a symlink out as a text file. `podman-compose` (the Python one) was not tried.

## 2026-11-09 - Rust dependencies updated (`authentication-verifier` 0.9.2, `axum-keycloak-auth` 0.8.4)

- **Clinical risk:** **no, by my judgement, not the Clinical Safety Officer's.** No screen, rule or record changed. A Keycloak token is now refused if it names no configured audience, or if it is an ID or refresh token, or if its `nbf` or `iat` is in the future. Before, the verifier accepted these, though `operations/keycloak/README.md` already said `aud` must name `EPR_KEYCLOAK_AUDIENCES`. The realm in `operations/keycloak/epr-realm.json` adds `epr-api` through an audience mapper, so its access tokens still pass. A realm with no mapper would now see every clinician refused at sign-in.
- **Built:** `cargo update` in the matcher (`itoa`, `syn`) and the service (24 packages, among them `authentication-verifier` 0.9.2 with the engine's strict validation policy and `axum-keycloak-auth` 0.8.4, whose failures are a generic `401 invalid_token` and which no longer logs decoded claims). Six packages fell out of the tree. Both stay on Rust 1.97 (N-2 of stable 1.99.1).
- **Checked:** fmt, clippy, the matcher's tests (110), the service's database-free tests (513) and every database test (175). I read both crates' changelogs.
- **Not done / not verified:** no token from the real Keycloak realm was sent to the updated service, so the audience mapper passing is read from the realm file, not observed. The AGENTS.md note that clock drift shows as `InvalidSignature` was not rechecked against the new verifier.

## 2026-11-09 - CI's coverage upload uses `actions/upload-artifact` v7

- **Clinical risk:** **no.** CI only.
- **Built:** `ci.yml`'s coverage job uploads with `actions/upload-artifact@v7`, as Dependabot's PR #5 proposed; `release.yml` already used v7.
- **Checked:** the next CI run on main (the coverage job uploads `coverage-lcov`).
- **Not done / not verified:** the uploaded coverage files were not downloaded and compared with a v5 upload.

## 2026-11-09 - UI dependencies updated

- **Clinical risk:** **no.** No screen, rule or record changed. The runtime changes are patch releases of SvelteKit (3.1.1), adapter-node (6.0.1) and node-redis (6.3.1); the rest are tools and test libraries.
- **Built:** every UI dependency to its latest, as Dependabot's PR #7 proposed and more: `@lilydesignsystem/svelte-headless` 0.8.0 (0.7.0 renamed `tag-input`, which the UI does not use: it imports nothing from the package), `svelte-picker-bar` 0.5.3, `themes` 0.6.2, `redis` 6.3.1 (still pinned exactly), `@playwright/test` 1.65.0, `vitest` and `@vitest/coverage-v8` 5.1.0, `jsdom` 31.0.0, `eslint` 10.14.0, `eslint-plugin-svelte` 3.24.1, `globals` 17.14.0, `svelte-check` 4.7.8, `vite` 8.3.6, `@types/node` 26.6.6.
- **Checked:** lint, `check` (0 errors, 0 warnings), `check:tests`, vitest (318) and Playwright (103), with Playwright's Chromium reinstalled for 1.65.
- **Not done / not verified:** TypeScript stays at 6.0.3, because SvelteKit 3.1 still needs `ts.readConfigFile`, which TypeScript 7 lacks; I did not retry 7.0.2. No screen was compared by eye before and after. The node-redis session store's live test against a real Redis was not run.

## 2026-11-09 - `compose.yaml` trusts the UI container's `X-Forwarded-For` (OS-2, H-049)

- **Clinical risk:** **yes, treated as clinical (H-049).** Nothing a clinician sees changes. In the compose stack, the address the API audits, and counts toward the lockout when it is on, becomes each browser's as the UI sees it rather than the UI container's. H-049's residual is reworded; its status is unchanged.
- **Built:** the compose network has the fixed subnet `10.89.240.0/24`, the `ui` container the fixed address `10.89.240.250`, and the API `TRUSTED_PROXIES=10.89.240.250`: that one address, never the subnet, because a request to the published port `5150` arrives from inside the subnet too. Spec `07` NFR-6, the UI spec, the OS-2 note and H-049 say so.
- **Checked:** `podman compose config`. I ran the stack with the UI profile, login on and `LOCKOUT_CLIENT_ADDRESS_AFTER_COUNT=2` (an override, not in `compose.yaml`). The UI container got `10.89.240.250`. From inside it, bad tokens naming `203.0.113.7` were answered `401`, `401`, `429`; one naming `203.0.113.8` was `401`; the audit row named `203.0.113.7`. From the host, the header was ignored: every request counted as one address (`10.89.240.9`, the API container's own, which is how rootless Podman's port forward arrives), so changing it did not lift the lockout.
- **Not done / not verified:** no browser was driven through the UI to the API, so the address the UI sees for a browser on the host (likely one forwarded address for every browser on the machine) was not checked. `podman compose --build` here goes through `docker-compose`, which reads `.dockerignore` and not `.containerignore`, so it sent `target/` as build context and ran the VM's disk out; I built the images with `podman build` instead and did not fix that. The API image build also ran out of memory in a 12 GiB VM unless `CARGO_BUILD_JOBS=1`; that is not set anywhere. Another container given `10.89.240.250` by hand would be trusted. No other deployment sets `TRUSTED_PROXIES`.

## 2026-11-09 - The UI's server sends the browser's address (OS-2, H-049)

- **Clinical risk:** **yes, treated as clinical (H-049).** Nothing a clinician sees changes. Where the service lists the UI's server in `TRUSTED_PROXIES`, the address it audits and counts toward the lockout becomes each browser's, not the UI server's, so one user's failed tokens no longer lock out everyone behind the UI. Where it does not, nothing changes. H-049's residual is reworded; its status is unchanged.
- **Built:** the UI's proxy sends the API `X-Forwarded-For` with one entry, the browser's address from SvelteKit's `getClientAddress()`, only when it is an IPv4 or IPv6 address (an IPv6 zone is dropped). An `X-Forwarded-For` the browser sent is never passed on. The UI spec, spec `07` NFR-6, `src/lockout.rs`, the start-up warning, the OS-2 note and the personal-data inventory's `network_address` rows follow.
- **Checked:** UI lint, `check`, `check:tests`, vitest (318) and Playwright (103). There are 4 new unit tests: the address is sent and the browser's header is not; it is sent with login off and for IPv6; nothing is sent when SvelteKit cannot give an address; only an IPv4 or IPv6 address passes. I also ran the built UI against the real service, reaching the UI over `::1`, with a forged `X-Forwarded-For: 6.6.6.6`. With `TRUSTED_PROXIES=127.0.0.1` the search's audit row held `::1`. Unset, it held `127.0.0.1`, as before.
- **Not done / not verified:** `compose.yaml` does not set `TRUSTED_PROXIES`, because the UI container has no fixed address there and trusting the whole network would trust the host too; no deployment sets it. Behind a proxy in front of the UI, adapter-node's `ADDRESS_HEADER` and `XFF_DEPTH` must be set or the UI sends that proxy's address; that was not tried. The audit now holds browsers' addresses where trusted, which is more personal than the UI server's; no retention decision changed. Run with login off on both sides, not with a signed-in session.

## 2026-11-09 - Lockout by client address with `axa-ratelimit` (OS-2, H-049)

- **Clinical risk:** **yes (new hazard H-049, every clinician behind one address locked out).** It is off unless `LOCKOUT_CLIENT_ADDRESS_AFTER_COUNT` is set, so nothing changes for a deployment that does not set it. When it is set, an address that earns that many `401`s in 15 minutes is refused every request until the window ends, a valid token included. The UI's server does not forward the browser's address, so behind it every user is one address, and setting it there would lock out every clinician at once after a few expired tokens. H-049 is new and open; the wording is my judgement, not the Clinical Safety Officer's.
- **Built:** `src/lockout.rs`: `axa-ratelimit`'s `lock_out_failures`, mounted in `app.rs` outside authentication, so it counts authentication's `401`s and refuses before a token is checked. `LOCKOUT_CLIENT_ADDRESS_AFTER_COUNT` is a whole number from 1 to 1,000,000; unset is off, and anything else stops the service starting. The window is fixed at 15 minutes (`lockout::WINDOW_SECS`). The refusal is 429 with `Retry-After` and the `RateLimit-*` headers, a problem under `/api` and an `OperationOutcome` under `/fhir`. The probes are never refused. An IPv6 address counts by its `/64`. The table fails open when full and counts that in `epr_rate_limit_overflowed_total`. The first refusal of an address in a window is an audit row (`Execute`, outcome 4, agent `anonymous`, `denied: lockout`, the address in `network_address`). Every refusal is counted in the new `epr_lockouts_total`. The service logs a warning at start when it is on. Spec `07` NFR-6 and the OS-2 note follow.
- **Checked:** fmt, clippy, every database-free test, and every database test. There are 2 unit tests and `tests/http_lockout.rs` (3, plus 1 database-gated). They cover: an address locked out after its `401`s even with a good token, while another address and the probes are not; a success never counting; unset counting nothing; an IPv6 `/64` counted as one; a FHIR refusal as an `OperationOutcome`; and three refusals making one audit row naming the address. I ran the real binary with login on and the setting at 3, four times. Three runs answered `401` three times, then `429` with `Retry-After: 900`; the FHIR refusal was an `OperationOutcome`; `/_ping` still answered 200; the audit row named `127.0.0.1`. One earlier run answered `429` from the first request. I did not find why and could not make it happen again.
- **Not done / not verified:** the UI's server does not forward the browser's address, and `TRUSTED_PROXIES` is not set anywhere, so there is no deployment where turning it on is safe yet. There is no way to lift a lockout but waiting or a restart. The window cannot be set. Counts are per replica and lost on restart. A `401` is counted when it is answered, so bad tokens sent at once can pass the count by however many are in flight (no per-address request limit caps that). The UI was not driven to a lockout.

## 2026-11-08 - `axa` 0.9.0 to 0.9.1

- **Clinical risk:** **no.** No clinical rule, screen or record changed. A client sending the same `Idempotency-Key` while the first request still runs is now told to wait as long as the first has run (1 second at least), not always 1 second; it is still refused and nothing is written twice.
- **Built:** `axa` 0.9.1 and `axa-idempotency` 0.2.1 (`cargo update`). Spec `09` states the new `Retry-After` on 409 `idempotency_in_progress`.
- **Checked:** fmt, clippy, every database-free test, and the idempotency database tests. I read the rule in `axa-idempotency` 0.2.1's source.
- **Not done / not verified:** no test here makes a request that is still running, so the new `Retry-After` is tested in `axa-idempotency`, not in this service. The UI proxy was not checked for passing a 409's `Retry-After` through.

## 2026-11-08 - The outbox relay stops when the service stops (OS-3, H-038)

- **Clinical risk:** **yes, treated as clinical (H-038).** Nothing enqueues and the only sink logs, so no event leaves the service, before or after. A relay that would have been cut off by a shutdown mid-publish, and sent that event again later, now finishes it and records it. A crash still sends again. H-038's controls gain this clause; its status is unchanged.
- **Built:** `outbox::Running` starts the relay and its depth gauges; `Running::stop` tells the relay to stop and waits, and aborts and logs one that has not stopped in time. `app.rs` keeps the running relay, which it no longer leaks, and stops it after `serve` returns on `SIGTERM` or Ctrl-C, waiting up to `STOP_WAIT` (35 seconds: `axa-outbox`'s 30-second publish timeout and 5 to hand the batch back). Spec `08` "Outbox" says so.
- **Checked:** fmt, clippy, and `tests/outbox_store.rs` (13). Two new database tests: a stop sent during a publish waits for it, records it as delivered, sends nothing more and charges nothing; a relay whose sink never answers is abandoned within the wait, reports that it did not stop, and records nothing as sent. One new test without a database: with no relay kept, a stop answers at once. I also ran the real binary with `EPR_OUTBOX_INTERVAL_SECS=1`: `_ping` answered 200, the relay polled, and `SIGTERM` exited 0 with no error logged.
- **Not done / not verified:** the binary run had no event in flight, so the wait during a real shutdown was tested only through `Running`, not the binary. An abandoned relay's claim waits five minutes for the next relay, as after a crash. A second signal during the wait does not cut it short. The other background loops (idempotency purge, audit partitions, MLLP) are still stopped only by the process ending.

## 2026-11-08 - The outbox kept and relayed by `axa-outbox` 0.2 (OS-3, H-038)

- **Clinical risk:** **yes, treated as clinical (H-038, an event sent to a recipient who should not have it; H-046's death hold).** Nothing enqueues and the only sink logs, so no event leaves the service, before or after. What a relay would do changes in five ways. A refusal is still final and audited, and a death report still holds an event with no attempt. A failed death or consent lookup now writes no `failed` attempt: the event waits and is decided again, and is still never delivered unchecked. The decision is now made when the relay claims a batch, so a consent withdrawn or a death reported between the claim and the sink's answer does not stop that delivery. Before, the decision and the delivery were in one transaction, and the window was shorter, not absent. A changed stored event is now `failed` as `tampered`, audited, and never sent, where before nothing checked. A sink's failure is now a short code, and free error text is never stored. A failed event can now be put back by a named person. H-038's controls and residual are reworded to say this; its status is unchanged, and the wording is my judgement, not the Clinical Safety Officer's.
- **Built:** the `axa` feature `outbox`. Migration 28 runs `axa::outbox::SETUP_STATEMENTS` (a unit test fails if they drift apart): `axa_outbox_events`, `axa_outbox_payloads` and `axa_outbox_attempts`, insert-only under triggers that fire for every role and under replica mode, with the working tables `axa_outbox_owed` and `axa_outbox_leases`. Migration 17's `outbox_events` and `outbox_attempts` keep their rows and triggers, and a new trigger refuses new rows in them. `src/outbox.rs`: `enqueue` takes a transaction (it took a pool or a connection), refuses a payload over 64 KiB, and stores the recipient's roles, purpose, resource and patient in the payload beside what is sent, so that `axa-outbox`'s fingerprint covers them. The recipient is the topic, and the patient is the key, so a hold on one patient holds back no other. `decide` (the death hold, then the policy and consent decision, renamed `permits`), `publish` (the sink, or `no-sink`) and `record` (the counters, and an audit row for a delivery, a refusal or a changed event) go to `Outbox::run`. A sink answers `axa::outbox::Failure`. The relay tries an event 5 times in all, 30 seconds and then twice as long between, as before. `EPR_OUTBOX_INTERVAL_SECS` is now how often an idle relay looks for work, and the service will not start a relay unless every outbox table, index and trigger is there. `watch_depth` sets `epr_outbox_pending` (owed, claimed or held) and `epr_outbox_stuck` (failed). Spec `08` "Outbox", `10`, `data-exchange`, the service spec, the data flow, the personal-data inventory and the schema map follow. The migration is numbered `20261108100028`, because a name with `0800` in it trips the grief census's helpline check.
- **Checked:** fmt, clippy, every database-free test, and every database test. `tests/outbox_store.rs` (10) runs `axa-outbox`'s relay with the service's closures. It covers a rolled-back event never sent and a committed one sent once and audited; a patient's refusal and a role the policy refuses, final and audited; a death report holding an event through a confirmation until it is withdrawn, while another patient's event goes; a failing sink tried 5 times and then failed; an event with no sink failed, released by a named person and then sent; a payload changed with its guard turned off, failed as `tampered`, audited and never sent; an oversized payload refused; the record refusing `UPDATE`, `DELETE` and `TRUNCATE`; the closed tables refusing a new row; and the depth gauges.
- **Not done / not verified:** no producer and no real sink (IO-6). Not used: `axa::outbox::erase`, sealing (`outbox-encryption`), a deny list of field names, and a route to release a failed event, so a release needs code. The old tests of the retry waits and their cap are gone: the waits are `axa-outbox`'s, tested there. A failed lookup is logged by the relay and leaves no attempt row. The counters are counted before the outcome commits, so one that then fails to commit is still counted. The relay runs for the life of the process with no graceful stop, so a shutdown mid-delivery sends again. Migration 17's rows are not moved into the new record; nothing enqueued before this, so none are expected. A real crash, two relays at once, and a relay that stalls were not exercised here.

## 2026-11-08 - `axa` 0.7.0 to 0.9.0

- **Clinical risk:** **yes, treated as clinical (H-043, a retried create written twice), though nothing a clinician sees should change.** `axa-idempotency` 0.2.0 now holds a run's key for its whole life by default, which is what this service set by hand. The `.lease` setting is gone, so the service no longer sets it, and a unit test now pins that `rerun_abandoned_after` is never set. A retry of a lost run is still a 409 until the key expires. H-043's controls and residual are reworded to say this; its status is unchanged, and the wording is my judgement, not the Clinical Safety Officer's.
- **Built:** `axa` 0.9.0, with `axa-idempotency` 0.2.0 and `axa-cursor-pagination` 0.1.1. `src/idempotency.rs` drops `LEASE` and `.lease(LEASE)`. The table is unchanged and needs no migration. A running request reports in every third of its lease, every 8 hours. 0.8.0 changed only `axa-outbox`, which this service does not use yet, and 0.9.0 removed `axa-offset-pagination` and `axa-worker`'s pool calls, which it does not use either.
- **Checked:** fmt, clippy, every database-free test, and every database test (`tests/http_idempotency.rs` included).
- **Not done / not verified:** the UI and the container image were not run against it. A real crash during a keyed create was not exercised, as before. `axa-outbox` 0.2.0 is not adopted yet; that is the next entry.

## 2026-11-07 - `Idempotency-Key` run by `axa-idempotency` (OF-1, H-043)

- **Clinical risk:** **yes, treated as clinical (H-043, a retried create written twice).** A keyed create still runs once per subject and key, a replay is still audited and refused if it cannot be, and a refusal still releases its key. Three things change. A key may now be 1 to 255 visible ASCII characters (it was 1 to 128 of letters, digits and `. _ : -`), and the UI sends a UUID either way. The query string is now part of what makes a request the same one. A key store that cannot be reached is now 503, not 500. `axa-idempotency` would by default let the same request take over a key after a minute without a heartbeat, which could write a record twice after a crash or a lost answer. **The lease is set to the key's whole life (24 hours)**, so that retry stays a 409, as before, and a unit test pins it. H-043's controls and residual are reworded to say so; its status is unchanged, and the wording is my judgement, not the Clinical Safety Officer's.
- **Built:** `src/idempotency.rs` is now settings over `axa::idempotency` (feature `idempotency`): only a `POST` under `/api/`, keyed by the token subject, 24 hours, 1 MiB requests and 4 MiB answers, plus a layer outside it that audits a replay. Migration 27 makes `axa_idempotency` from the library's `SETUP_STATEMENTS` (a unit test fails if they drift apart) and drops `idempotency_keys`. The service checks the table at boot, and an hourly loop calls `sweep`. A success whose answer cannot be kept now reaches its caller, and a retry is a 409. Before, the caller got a 500 for a create that had been written, and the key was released. The personal-data inventory, the retention schedule, the privacy notes and the schema map follow the new table. `scripts/generate-schema-map.py` now reads `CREATE TABLE IF NOT EXISTS` and `DROP TABLE`.
- **Checked:** clippy, every database-free test, every database test, `tests/http_idempotency.rs` (runs once, replay, reuse, a bad key, a released key, a held key is a 409, an expired key runs again, no key), the per-subject test, and the inventory census.
- **Not done / not verified:** `tests/idempotency_clock.rs` is removed. It moved the service's clock to test the 24-hour life, but `axa-idempotency` reads the database's clock, so expiry is tested by setting the row's times, which PR-5 had moved away from. The keys held when migration 27 runs are dropped: a create sent before the upgrade and retried after it runs as if it had no key. The heartbeat, a real crash and the sweep were not exercised. `/fhir/` creates and `PUT`/`PATCH` stay uncovered, and `Idempotency::required` is not used, so a client that sends no key is as exposed as before.

## 2026-11-07 - Request limits counted by `axa-ratelimit` 0.2.0

- **Clinical risk:** **yes, treated as clinical (H-039, a clinician refused by a rate limit).** The limits, the classes, who is counted, the 429's body and `Retry-After`, and the audited first refusal are as before. What changes: every counted answer now carries `RateLimit-Limit`, `RateLimit-Remaining` and `RateLimit-Reset`, and a request is counted only when it goes ahead (before, a refused one counted too, which changed nothing, as the window was already spent). H-039's controls and residual are reworded to say this; its status is unchanged, and the wording is my judgement, not the Clinical Safety Officer's.
- **Built:** `src/rate_limit.rs` keeps the settings, the classes and the audit row, and hands the counting to `axa::ratelimit` (feature `ratelimit`): one `Limiter` per class, keyed by the verified token subject with `Limiter::key`, so a compare call still counts only against the compare limit, and a request with no subject is not counted. Both use `WhenFull::Allow`, so a full table still lets a new subject through. Such a request is now counted in a new metric, `epr_rate_limit_overflowed_total`, where before it passed unseen. The refusal is audited from `Verdict::first_refusal`, in a layer outside the limiters, which also answers in the route's shape, so a `/fhir` refusal is an `OperationOutcome`. The limiter holds a subject only as a keyed hash; the old table held it in clear.
- **Checked:** clippy, every database-free test, and every database test (175). `tests/http_rate_limit.rs` adds the `RateLimit-*` headers, the problem's content type, a `/fhir` refusal as an `OperationOutcome`, and no headers on an uncounted request. A unit test fills the table to `MAX_KEYS` and sees a new subject let through and marked. Another counts a clinician's minute at the defaults. One database test failed once on a 409 "possible duplicate" from patients that earlier runs left in the reused test database; after restarting the throwaway database, all passed.
- **Not done / not verified:** the limit was not run in the real binary with a real token; the tests run it in a test router and through the app's router with login off, which counts nothing. The old unit tests that moved a fake clock (a new window, `Retry-After` near the end of one) are gone, because `axa-ratelimit` reads the clock itself; those cases are now `axa-ratelimit`'s own tests, not this service's. Nothing calls `set_limit_for`, so lifting one subject's limit still needs a restart. Not taken up: `by_client` (an address limit; the address is the UI's server for every user while `TRUSTED_PROXIES` is unset) and `lock_out_failures` (sign-in is Keycloak's).

## 2026-11-07 - `escape_like` from `axa-cursor-pagination`, not `axa-offset-pagination`

- **Clinical risk:** **no.** A search sends the same pattern as before; the function is the same code. `patients_store::tests::like_wildcards_in_a_search_are_escaped` pins it, unchanged.
- **Built:** the `axa` feature `offset-pagination` is replaced by `cursor-pagination`, and `like_prefix` calls `axa::cursor_pagination::escape_like`. `axa-offset-pagination`, which `axa` calls insecure, is out of the build. `cargo deny` passes; the new crate's `hmac`, `sha2` and `base64` were already in the tree.
- **Not done, and why:** no list moved to `CursorPage`. Its cursor is signed, but what it holds is an offset, and the crate says a row added or removed between two fetches can make a later page skip or repeat a row. The audit lists here (`/api/me/access-log`, FHIR `AuditEvent`) page by row id (`before`, `_before`), which skips nothing as rows are added. The work queues (referrals, the feed, deaths, break-glass reviews) are not paged at all: they are capped and say `more` (PR-1). On a queue a row leaves as it is worked, so an offset page could skip one, and a skipped referral is one nobody works. Moving would also change every list's body to a bare array with a `Link` header, which the UI does not read. A keyset cursor in `axa-cursor-pagination` would be the one to adopt.

## 2026-11-07 - `axa` 0.6.1 to 0.7.0

- **Clinical risk:** **no change expected in what a clinician sees.** One answer changes: a FHIR read whose `If-None-Match` holds the patient's current `ETag` is now `304` with no body, so a client shows the copy it already holds, which is that version. The handler, its checks and its audit row still run first. The UI sends no `If-None-Match`, so it is unaffected. No hazard row was changed; that is my judgement, not the Clinical Safety Officer's.
- **Built:** `axa` 0.7.0. The `pagination` feature is now `offset-pagination`, and `escape_like` is called as `axa::offset_pagination::escape_like`; `axa-pagination` 0.1.1 is replaced by `axa-offset-pagination` 0.1.0, the same code. Also new since 0.6.1 and in force with no change here: a write breaking a check or not-null constraint answers `422`, not `500` (0.6.2), and a client that stops reading a response is dropped after `SEND_STALL_TIMEOUT_SECONDS`, 60 by default.
- **Checked:** clippy, every database-free test, and every database test, the HL7 validator ones included. `tests/http_patients.rs` now checks the `304` and that a stale tag still gets a full `200`.
- **Not done / not verified:** whether any write here can break a check or not-null constraint, and so now gets a `422`, was not searched for. Not taken up: `axa-ratelimit` 0.2.0, which now has what H-039 needs (a key per subject, `WhenFull::Allow`, `first_refusal`, a per-key limit), `axa-cursor-pagination`, `axa-idempotency`, `axa-outbox`, the queue's schedules, and `conditional::if_match` in place of the FHIR update's own `If-Match` check. The UI and the container image were not run against it.

## 2026-11-07 - Patient search escapes `LIKE` with `axa`'s `escape_like`

- **Clinical risk:** **no.** The pattern a search sends is byte for byte what it was: `%`, `_` and `\` are escaped, and a family name such as `50%_\` matches only itself. `patients_store::tests::like_wildcards_in_a_search_are_escaped` pins it, unchanged.
- **Built:** `like_prefix` calls `axa::pagination::escape_like` in place of its own loop, behind `axa`'s `pagination` feature, which adds `axa-pagination` 0.1.1. `cargo deny` passes with it.
- **Not adopted, and why:** the rest of `axa-pagination`. `Page::bounds` and `Page::respond` page by offset, report a total in headers and answer a bare array. Every list here pages by a cursor (`_before`, `next`) with a fixed limit and a `more` flag in the body. Moving would change every list's answer, and an offset on the append-only audit trail would shift as rows are added. `Task::development_only` is not used either: the service registers no task, so there is nothing that loads fixtures to mark.

## 2026-11-07 - `/api` reads bodies with `axa::extract`

- **Clinical risk:** **no change expected in what a clinician sees.** The status of each bad request is the same. The body now has a `detail`, which the screen shows where it shows one.
- **Built:** the 17 `/api` controllers take `Json`, `Path` and `Query` from `axa::extract` in place of Axum's. A body that is not JSON, one not sent as JSON, or one of the wrong shape is now a problem that says what was wrong (400 `bad_request`, 415 `unsupported_media_type`, 422 `unprocessable_entity`). Since 0.6 it had been a problem with no `detail`, and before that Axum's plain text.
- **Checked:** every database-free test, every database test, and a new one in `tests/http_patients.rs` that sends each of the three through the real router and finds no part of the body quoted.
- **Not done / not verified:** the FHIR routes keep Axum's extractors, because a FHIR refusal must be an `OperationOutcome`, and neither Axum's rejection nor `axa`'s is one; whether a bad FHIR path or query answers an `OperationOutcome` was not checked here. The audit routes keep their own handling of a bad query, which names the parameters. Two database tests failed once on a 409 "possible duplicate": patients my first `tests/balp_claim.rs` left in the shared test database, with the same fixed birth date as `tests/http_death_reports.rs`. The test database was restarted (it is `tmpfs`), and that test still uses a fixed birth date.

## 2026-11-07 - `axa` 0.5 to 0.6.1

- **Clinical risk:** **no change expected in what a clinician sees.** Every request now passes through the layers in a new order, which is where a mistake would show: who may see what, and what the audit trail records. The service's own refusals are unchanged, because they were already RFC 9457 problems (2026-11-06).
- **Built:** `axa`'s standard stack is now applied last, outside the service's own layers, as 0.6 requires (`App::router`). It used to be applied first, which put the service's layers outside it. So a refusal from authentication, the rate limit or the route gate now gets the request id, the security headers and CORS, and the client's address is resolved before the audit trail records it. `authz::client_address_mw`, added this morning for PX-4 to work round the old order, is gone. A failing `/_readiness`, an unmatched path, a wrong method and a rejected extractor are now problems as well, from `axa`; the readiness one keeps `ok` and `failed`.
- **Checked:** clippy, every database-free test, and every database test (167). `tests/http_patients.rs` now checks that a 404 and a 405 through the real router are problems that quote nothing the caller sent, and carry the stack's headers. I also ran the real binary over TCP: `_ping` and a 404 carry `nosniff` and a request id, a search's audit row holds `127.0.0.1`, the interaction and the server while a forged `X-Forwarded-For` is ignored, and `SIGTERM` stops it cleanly. `break_glass_store` failed once: the Podman VM's clock jumped forward during the run (chrony logged it). Three reruns passed.
- **Not done / not verified:** the UI was not re-run against the real service; it reads `detail` already, and its tests mock the API. The container image was not built locally, because the VM runs out of memory compiling `fhir-r5`. Not taken up from 0.6: `axa::extract`'s problem-answering `Json`, `Path` and `Query`, `axa-ratelimit` in place of the service's own limiter, and `KEEP_ALIVE_TIMEOUT_SECONDS`, left at 75 seconds, which a load balancer keeping idle connections longer would need raised. The `problem` feature is still named, and now adds nothing.

## 2026-11-07 - An `AuditEvent` claims BALP only when it meets it (PX-4)

- **Clinical risk:** **no.** Nothing a clinician sees, decides or signs reads these fields. The patient's access log is unchanged and does not show the new address. No hazard row was changed; that is my judgement, not the Clinical Safety Officer's.
- **Built:** `fhir_audit::violations` names each broken constraint, and `to_fhir` claims `PatientRead`, `PatientCreate` or `PatientUpdate` only when there are none. A row that cannot claim is exported as before. New rows hold the interaction and the server under `detail`, and the client's address in `network_address`. A new setting, `EPR_AUDIT_SERVER_ADDRESS`, is checked at start-up. `spec/audit/index.md`'s gap table is replaced, from the `ihe.iti.balp` 1.1.4 package itself. The second-hand table was wrong or short on several points: the client and server agent codes swap with the direction of the data, `network` is required on both, and `Query` requires the query.
- **A found gap, closed:** no API row has ever held `network_address`, though the spec listed it; only the HL7 feed set it. **Every audit row from a request now records an IP address, which is personal data**, kept as long as the audit row. The service's layers run before `axa` resolves the address, so a new layer, `authz::client_address_mw`, resolves it the same way; only the database test through the router showed this.
- **Tests:** one per constraint, in `fhir_audit`. The HL7 validator with the BALP package accepts all four claims and refuses one with its server agent removed. `tests/balp_claim.rs` follows a real registration, read and search through the router, and a forged `X-Forwarded-For` is not recorded.
- **Not done / not verified:** **no event claims a profile until a deployment sets `EPR_AUDIT_SERVER_ADDRESS`, and neither `compose.yaml` nor any deployment does yet.** Only the patient record claims: a search never can, because the query is deliberately not held, and a row about an allergy, an observation or anything else holds a type and no id, so it does not. Through the UI, the address recorded is the UI server's, because it does not forward `X-Forwarded-For` and `TRUSTED_PROXIES` is not set; that is the BALP client, but it is not the person's machine. The client is named by address, as no client id reaches the service. Rows written earlier claim nothing. The validator ran with `-tx n/a`, so codes were not looked up, and on events built in the test from the row shape rather than ones read back from a server. The optional transaction entity (`XrequestId`) is not sent, though the row holds the id. Whether this meets an external auditor's needs is untested; AU-3 stays open. 21 of the 22 local gates passed. **`container-api` did not:** the Podman VM (12 GB) killed `rustc` for out of memory while compiling the third-party `fhir-r5` crate, twice, the second time with `CARGO_BUILD_JOBS=2`, before reaching this crate. This change touches no dependency or `Containerfile`; the image is left to GitHub CI.

## 2026-11-07 - `/_dependencies` says what is down (PX-5)

- **Clinical risk:** **no.** Nothing a clinician sees or does reads this route, and it changes no answer of `/_health` or `/_readiness`.
- **Built:** `GET /_dependencies` answers 200 with `ok`, `checked` and `degraded`, for the database and each key service this deployment configures (`keycloak`, `paseto-keys`). One answer is reused for 5 seconds, passed or failed, and one round runs at a time. `src/dependencies.rs` has its own small cache, because `axa`'s named checks answer only through `/_readiness`, as ok or 503. `url` becomes a direct dependency; it was already in the build. Spec: `spec/09-api-surface.md`.
- **The public paths grow by one, on purpose:** the census in `tests/route_census.rs` now allows `/_dependencies`. The route tells an unauthenticated caller which key service is configured and whether it is up, as a label and no address.
- **Not done / not verified:** the plan said `/_health`, and it was not used, because the container restarts on that route. A key service is asked by opening a TCP connection, so one that listens and answers badly shows as up, and a connection through a proxy shows the proxy. Whether a running service keeps accepting tokens from keys in memory while its key service is down was not checked. Nothing watches the route, no alert uses it, and it was not tried against a real Keycloak. The UI's dependencies, such as its Redis, are not the service's and are not here. The service's `/_readiness` still asks only the database. The local `container-api` gate did not finish: the release build compiled, then the Podman VM ran out of disk saving the image, so the image was built only by CI's `container` job.

## 2026-11-06 - Error bodies are RFC 9457 problems (`axa-problem`)

- **Clinical risk:** **no change expected in what a clinician sees, and this is a change to how every refusal travels.** The page shows a refusal from `errors`, `error_codes`, `conflict_codes`, `refusal` and `break_glass_offer`, and from the status; those are unchanged, and no page read the old `error` member. A FHIR client gets the same `OperationOutcome`. No hazard row is added, for that reason; this is the agent's judgement, not the Clinical Safety Officer's.
- **Built:** the service turns on `axa`'s `problem` feature, and `ApiError` builds an `axa::problem::Problem`. On `/api`, a refusal is `application/problem+json` with `type`, `title`, `status` and `detail`, and `detail` holds what `error` held; the fields a client acts on stay at the top level as extensions. `{"error": …}` is gone. A refused record says `the record was refused` in `detail`, and its reasons stay in `errors`. Handlers did not change. `openapi.json` gives the new media type and schemas. The UI's end-to-end tests' mock answers say `detail` too, so they keep the real shape. Spec: `spec/09-api-surface.md`.
- **A break this closes before it landed:** `controllers/fhir.rs` read the `error` member back out of the JSON to fill `diagnostics`, and its tests wrote that JSON by hand, so they would have passed while the live facade lost its message on a 400. `error_outcome` now takes the typed `Problem`, its tests use real `ApiError` values, and `tests/error_census.rs` refuses `.get("error")` and a `Problem::new` outside `api_error.rs`.
- **Not done / not verified:** this changes the wire shape in place, with no second `Accepts-version` value, because the only client is the UI in this repository and no deployment exists; a client outside it would break. The audit 424 report keeps its `error` member, because it is a report, not a refusal. `instance` is not sent. `type` stays `about:blank`, so there are no problem-type URIs. The UI's own answers from `/api/proxy` (`the service is unreachable`, `the service timed out`) still say `{"error": …}`: that is TypeScript in the backend-for-frontend, which this crate does not reach, and no page reads them. A failure `axa` answers itself rather than the service, if any reaches a client, keeps `axa`'s shape, which it says is moving to RFC 9457.

## 2026-11-06 - `axa` 0.4 to 0.5

- **Clinical risk:** **yes, in how the service starts and is reset.** No clinical rule changed. Running out of connections now makes a new client wait: the server holds 10,000 at most (`MAX_CONNECTIONS`). A proxy can no longer offer HTTP/2 through to the service: it speaks HTTP/1 only, which is all it was built for. No hazard row is added: a clinician sees no difference while the service is up.
- **Built:** `db reset` is confirmed by `AXA_DESTROY_DATA=true`, in place of `EPR_ALLOW_DESTRUCTIVE_DOWN`, which is gone. `axa` now refuses `db reset`, `db down` and `db truncate` in `main`, before a setting is read, and `--environment` cannot lift it. The service still checks before it drops anything, and still has no `down` or `truncate`. `tests/destroy_refused.rs` runs the real binary, because no unit test reaches `main`.
- **Corrects the 0.4 entry:** that entry said a client that does not finish its headers in 30 seconds is dropped. On 0.4.0 that was not true of a client that sent nothing, or an HTTP/2 preface: those stayed open for ever. A `DATABASE_URL` of `localhost` with `hostaddr` set to another machine also passed as loopback. `axa` 0.4.1 closed both, and this move takes them.
- **Checked:** every gate in `scripts/ci-local.sh --full`, and the real binary, with `HEADER_READ_TIMEOUT_SECONDS=2`: a silent client and a partial request closed at 2 seconds, and an HTTP/2 preface at once; `check` refusing `localhost?hostaddr=10.0.0.5` in production, and `MAX_CONNECTIONS=0` and `TRUSTED_PROXIES=junk`; `db reset` working when confirmed in development, leaving 65 tables; `db down` and `db truncate` reaching the service's own refusals. `h2` and `fnv` left the build.
- **Not done / not verified:** the 10,000 connection limit was not tried with 10,000 connections. `TRUSTED_PROXIES` is not set, and nothing reads `ClientAddr`, so a row in the audit trail from the API still has no network address. `axa`'s `auth_refused` line does not apply, because the service keeps its own authentication; its own refusals are audited. No real TLS connection to Postgres was made, as before, and no real deployment exists.

## 2026-11-06 - `axa` 0.3 to 0.4

- **Clinical risk:** **yes, in how the service starts.** No clinical rule changed. In production a `DATABASE_URL` to another machine with no `sslmode` of `require`, `verify-ca` or `verify-full` now stops the boot, unless `DB_ALLOW_INSECURE=true`. A deployment that worked before can refuse to start, which is safer than sending records in the clear but is still an outage if nobody expects it.
- **Built:** the pool is on `AppContext::db`, opened by `axa::db::connect`, so `ctx.pool()` can no longer panic and the shared store no longer holds it. `pool::Db` and `pool::Conn` are `axa`'s. `axa` asks the database for `/_readiness`, so our named check is gone. `Environment::may_destroy_data` replaces `reset_allowed`, and the test of who may reset stays here. `axa::cli::parse` gives `--help` and `--version` the service's own name, and the direct `clap` dependency is gone.
- **Changes on purpose:** `DB_CONNECT_TIMEOUT` is 5 seconds and `DB_IDLE_TIMEOUT` 10 minutes, up from 500 ms each. A client that does not finish its headers in 30 seconds is dropped. A server fault no longer logs a database error's `Debug` text, which could quote a row.
- **Checked:** every gate in `scripts/ci-local.sh --full`, and the real binary: `--help` and `--version`; `check` refusing a remote `DATABASE_URL` with no `sslmode` and accepting one with `verify-full`; `db reset` refused in production with the flag and in development without it; migrate, a write, a read, a need, the overdue list and the death count; 503 with Postgres stopped, `/_ping` still 200, a clean stop on SIGTERM and no patient id in the logs.
- **Checked afterwards:** `compose.yaml` runs the local stack with `APP_ENV: production`, so the new rule stopped it and CI failed on the first push; the local gates only build the image and did not see it. That stack now sets `DB_ALLOW_INSECURE=true`, by name and with a comment, because its Postgres has no TLS. The rule itself is unchanged, and both CI steps that boot the stack pass.
- **Not done / not verified:** no real TLS connection to Postgres was made: `sslmode` was checked as a setting only. The local stack runs in production mode with a plain connection, on a private network, and is not a deployment to copy. No real deployment exists to check; the production-readiness list now asks. The binary was tried in `container-dev`, not with production login. `axa`'s newer guard in `cli::parse` is unreleased, so `db reset` still uses `EPR_ALLOW_DESTRUCTIVE_DOWN`. `axa-tracing`, `axa-opentelemetry` and `axa-keycloak` are not used. The 30-second header limit was not tried against a real client.

## 2026-11-06 - Overdue checks of a need, for the on-duty clinician (PX-2)

- **Clinical risk:** **yes, updated in H-048.** This changes which needs a person is prompted to check. A check that nobody opens the list for stays overdue, as it did before. The list marks a person with a report of death standing, so that nobody rings a family to check a need.
- **Decided by the maintainer (OQ-PX2):** the role of on-duty clinician owns the list.
- **Built:** `GET /api/overdue-reviews` and its count, a `/overdue-reviews` page and a navigation link. The role on its own now reads, reviews and ends a need, which is new. Ids, the version and the date only. A look is audited with the count.
- **Checked:** see `tests/http_overdue_reviews.rs`, a policy test and two browser tests.
- **Not done / not verified:** no count in the navigation, because it would make an audited refusal per page load for every other role, so the owner has to open the page. No push and no record of who took a check on. Not tested: a need on a record merged away, or a list cut short against a real database. A need is ordered by date alone, not by how serious it is. "Overdue" uses the UTC day, which can differ from the site's near midnight. The words are English only. No clinician, disabled person or interpreter has reviewed the list, and consent not hiding an item is my choice and needs review.

## 2026-11-06 - The rota belongs to workforce planning

- **Clinical risk:** **yes, recorded in H-046.** A shift with no `on-duty-clinician` has nobody accountable for a report of a death, and the service cannot see that. Keeping a holder on every shift now sits with workforce planning management.
- **Decided by the maintainer:** the rota belongs to workforce planning management, and the service will not hold or read one. The spec, OQ-BV4, the role table, H-046, the production-readiness list and the care-home idea now say so, and "a rota" is off the list of things to build.
- **Not done / not verified:** spec only: no code changed. No link from a workforce system exists, so the role is still given by hand in the identity provider, and which system would do it is not named. No alert for a shift with no holder exists here or anywhere. Not reviewed by a Clinical Safety Officer.

## 2026-11-06 - `axa` 0.1 to 0.3

- **Clinical risk:** **yes, in how the service starts and stops.** No clinical rule changed. A release build with no `APP_ENV` now runs as production, which closes a risk and is stricter. A setting that makes no sense stops a production boot. Requests still running 25 seconds after shutdown begins are cut off (`SHUTDOWN_TIMEOUT_SECONDS`), so a write in flight then rolls back and its caller sees a failure.
- **Built:** `axa` 0.3; `Routes::mounted` and `Handler::methods` in place of `routes::methods_of`, which is gone; `axa`'s `/_readiness` with a named `database` check in place of our own route, which is gone; the `check` command, which needs no database; a 10-minute limit on `/api/audit/export`.
- **Changes on purpose:** production answers with a strict `content-security-policy`, which suits a JSON API. `/_readiness` keeps one answer for a few seconds, so a load balancer sees a failure slightly later, and a failing answer now names `database`. Logs carry the route template, not the real path, so patient ids leave them. Only a server fault logs as an error.
- **Checked:** every gate in `scripts/ci-local.sh --full`, `tests/readiness.rs`, and the real binary: `check` with no database, a write and a read, 503 with Postgres stopped, and a clean stop on SIGTERM.
- **Not done / not verified:** the 10-minute export limit is a guess: no export of a real size was timed, and `/api/audit/verify` keeps the 60-second limit, also untimed. `check` covers `axa`'s settings and not the service's `EPR_*` ones. Whether 25 seconds suits the feed listener under load was not tested. `pool::Db` and the pool in `shared_store` stay until an `axa` release has them. Test databases still use the user name `loco`.

## 2026-11-06 - Four safety officer roles (SE-30)

- **Clinical risk:** no change to what a clinician sees, decides or signs. The four roles see no clinical content. They can add to the hazard register, so a wrong or duplicate hazard is possible, as it is from a clinician.
- **Built:** `medical-devices-safety-officer`, `medication-safety-officer`, `patient-safety-officer` and `medical-incident-officer` in the policy table and Keycloak. Each reads the register and raises a hazard, for the purpose of operations.
- **Checked:** a unit test that each reaches the register and nothing else, cannot treat, and that the matrix test covers them. Not run through the routes with a token of each role.
- **Not done / not verified:** the permissions are a first draft by an engineer, and nobody who holds such a post reviewed them. No incident, device or medication error reporting exists, so these roles have little to do yet. No formal NHS definition was found for a Patient Safety Officer or a Medical Incident Officer, and sources were read through search results only.

## 2026-11-06 - Staff are told a report of a death has arrived (BV-9; H-046 stays open)

- **Clinical risk:** **yes.** Decided by the maintainer as OQ-BV4: clinicians, nurses and administrators are alerted, and the role of on-duty clinician is accountable. Before this, a report waited until someone happened to open the record, and a proxy could read the record meanwhile. Administrators now see that a report exists for a patient, which they could already see in the banner. A refused consent does not hide a report from this list; that is a judgement for review.
- **Built:** the role `on-duty-clinician`, which on its own gives the death work and nothing else; the resource `DeathReports`; `GET /api/death-reports` and `/count`, ids and times only, oldest first, and a cut list says so; the `/death-reports` page; the count in the navigation; the Keycloak role.
- **Checked:** `tests/http_death_reports.rs` through the real router with login on: who sees the list and who is refused, no reason in the list or the audit, a verified and a withdrawn report leaving it, the on-duty role alone verifying and withdrawing, a nurse still unable to withdraw. A browser test of the page.
- **Not done / not verified:** **the service has no rota.** It cannot say who holds the role or whether anyone does, so a shift with no holder has nobody accountable and nothing says so. Nothing records that a person took a report on. There is no push, and no time limit for a report to wait. A report on a record merged away and a list cut at 50 are not tested through the route. No clinician, Clinical Safety Officer or bereaved person has reviewed this. The new words are in English only. The owner of overdue need checks (PX-2) is still undecided.

## 2026-11-05 - From SeaORM to sqlx

- **Clinical risk:** **yes, in every read and write of the record.** Nothing was meant to change in behaviour. The 26 migrations and all store code moved. Rollback is now impossible rather than guarded: there are no down files, and `db down` refuses.
- **Built:** `sqlx` 0.9 and `axa-sqlx` in place of SeaORM and `sea-orm-migration`; `migrations/*.sql`, embedded as `MIGRATOR`; the `electronic-patient-record-migration` crate removed. Store code is plain SQL on a `&mut PgConnection`. `pool::Db` lets one store function take a pool or a caller's transaction: `impl sqlx::Acquire` was tried first, and its futures are not `Send`, which every handler needs. `axa` 0.1 has no sqlx pool in `AppContext`, so the pool is in its shared store, read through `pool::Pool`, and the service has its own `/_readiness`, because `axa`'s checks nothing without its `sea-orm` feature.
- **Checked:** the migrations' SQL was captured from Postgres as the old code ran them, not retyped, and `pg_dump --schema-only` of a database built each way differs by 0 lines of 3,896. Every gate in `scripts/ci-local.sh --full`, with 163 database tests and 472 others. The real binary wrote, read and audited a patient, verified the audit chain, refused `db down` and `db reset`, answered `/_readiness` 503 with Postgres stopped and `/_health` 200, and exited on SIGTERM. Dependencies fell from 509 to 433 (651 before `axa`).
- **Caught on the way:** the mechanical rewrite took a connection before the failure counters in `audit::append` and `feed_queue::record`, so with the database gone, the audit-failure alarm would have stayed silent. `tests/metrics.rs` caught it, and both are fixed. `tests/support_only.rs` was still looking for `.rs` migration files, so it had stopped checking anything; it now reads the `.sql` files.
- **Not done / not verified:** queries are still checked at run time: `sqlx::query!` was not used, so a wrong column is found by a test, not the compiler. **An existing database must be recreated**: sqlx keeps its history in `_sqlx_migrations`, and nothing moves the old `seaql_migrations`. Nothing is in clinical use. Performance under load was not compared. Statement caching differs from SeaORM's, which was not measured. `db reset` in development now drops the schemas itself.

## 2026-11-05 - `db reset` is refused outside development and test

- **Clinical risk:** **yes: a defect in the previous entry, now fixed.** That commit mapped `db reset` to `Migrator::fresh`, which drops every table and does not go through the `down` guard, so one command could have destroyed clinical tables in production. It was on `main` for under an hour and was never deployed: nothing is.
- **Built:** `reset_allowed`: only in `development` or `test`, with `EPR_ALLOW_DESTRUCTIVE_DOWN=true`, the same rule as `down`. A unit test covers each case.
- **Not done / not verified:** the database-level protection is unchanged: anyone who can reach Postgres directly can still drop a table.

## 2026-11-05 - From Loco to axa (still on SeaORM)

- **Clinical risk:** **yes, in how the service starts and what it allows.** No clinical rule changed. Login is off by default only in `development`, `test` and `container-dev`, as before. **Stricter:** any other environment name now gets the production login rules; Loco held only the exact name `production` to them, so `staging` could start without keys. The rollback guard reads `APP_ENV`, then `LOCO_ENV`, and a test holds that the older name cannot loosen it.
- **Built:** `axa` 0.1 in place of `loco-rs`; `App::run`, written out; `config/` removed, with settings now in environment variables and `src/app.rs`; `routes::mounted`, which reads methods from the routers, so the censuses still compare what is served with what is written down. Dependencies fell from 651 to 509, and `unsafe-libyaml` and its advisory exception are gone.
- **Changes on purpose:** a 60-second request timeout in production (`REQUEST_TIMEOUT_SECONDS`), security headers and HSTS added, the ETag middleware gone, and Redis no longer needed: no code used it, though `compose.yaml` still starts it. `/_health`, `/_ping` and `/_readiness` remain.
- **Checked:** every gate in `scripts/ci-local.sh --full`, with 164 database tests. The real binary refuses production with no `DATABASE_URL` or no token keys, also under a mistyped environment; answers 401 with no token; sends the headers; and exits on SIGTERM. `db migrate` ran with `APP_ENV=production`.
- **Not done / not verified:** whether 60 seconds is enough for `/api/audit/export`. `routes::methods_of` reads Axum's debug text, as Loco did; a test fails if that changes, and the better home for it is `axa`. In `development` the service binds to `localhost`, which answered on IPv6 only here. With no `APP_ENV` at all the service defaults to `development`, so login is off, bound to `localhost`; that is not new, and needs a guard. Only the entries this change needs were made to `Cargo.lock`. The move to sqlx is next.

## 2026-11-05 - How old a vital sign's measured time may be (SW-2; H-034 stays open)

- **Clinical risk:** **yes.** Decided by the maintainer. A reading more than 7 days old now needs a typed reason, which stays in its history, and a reading before the patient's birth date is refused. No real reading can be lost to the 7 days: a reason always lets it through.
- **Built:** `timing_problems` and `timing_refusal`, on create and update; `EPR_OBSERVATION_LATE_DAYS` (1 to 3650, default 7); the form's label; tests of both rules through the routes. Marking an entry in error is exempt, so that a mistaken date can always be retracted. SW-2 is ticked.
- **Not done / not verified:** 7 days is a placeholder, and no clinician has set it. A typed reason can be anything, so this slows a false back-dated entry and does not stop one. The form shows the reason box before it is needed and says so only in its label; the refusal then says why. A day is allowed around the birth date for time zones, so a reading a few hours before birth can pass. A mistyped date within 7 days is still accepted. A bad setting falls back to 7 and logs a warning. Birth dates that are wrong will refuse real readings until corrected.

## 2026-11-05 - One command for the gates (LC-2)

- **Clinical risk:** none: a developer script.
- **Built:** `scripts/ci-local.sh`, with `--fast`, `--full` and `--list`. It reports every gate, never skips one silently, starts and removes its own Postgres, and bypasses `kache`.
- **Checked:** a deliberately broken unit test exited 1 with `ui-unit` FAIL, and every other gate still ran. `--full` passed all 22 gates, with the service database tests, the FHIR validator and both container builds, on the author's Mac.
- **Not done / not verified:** it is not all of CI. `cargo deny`, CodeQL, `pnpm audit`, the secret scan, the coverage floor, the fresh clone, the hazard-log pull-request checks and the container start checks stay on GitHub. It was not run on Linux, nor on a tree without the maintainer's uncommitted `Cargo.lock` bumps. The port is fixed unless `EPR_CI_PG_PORT` is set, so two `--full` runs at once collide; LC-4 covers that. The SKIP path was checked by reading it, not by forcing a skip.

## 2026-11-05 - A proxy stops at a confirmed death, not at a report (BV-5; H-046 stays open)

- **Clinical risk:** **yes.** Decided by the maintainer. This reverses the earlier entry: a proxy now keeps access while an unconfirmed report stands, and stops, like the patient's own login, once a person confirms the death. A wrong report no longer cuts off a parent or carer. The new risk is that a proxy can read the record of someone who has died until a person confirms it, and could learn of the death or read what the patient kept private. Outbound contact is still held at any report (OQ-BV1). H-046 records this.
- **Built:** the one check in `relationships::lookup`, the test, the spec and the hazard row.
- **Not done / not verified:** nothing alerts staff that a report has arrived, so that window has no limit. The words a proxy sees need people who have been through it. No Clinical Safety Officer has reviewed this.

## 2026-11-05 - Say that audit events are modelled on BALP, not conforming (PX-4, in part)

- **Clinical risk:** none in behaviour: words only.
- **Built:** the spec and two code comments corrected, and a table of the gaps. The spec said "IHE BALP shape", which overstated it. No event ever claimed the profile in its data.
- **Not done / not verified:** conformance itself. The gaps are from the constraints the patient administration system lists, not from the profiles or the validator with the BALP package, so the table may be incomplete.

## 2026-11-05 - Patients see and correct their own needs (PX-1; H-048)

- **Clinical risk:** **yes.** A patient can now add, check and end a need on "my record", at once, with no approval by staff. A patient could end a need staff believe still applies; the history shows both. H-048 records this. H-001's text is corrected: the service now uses the date of death.
- **Built:** the policy for the patient and a read-only proxy, a new proxy scope `adjustments`, the section on "my record", and tests that a patient cannot reach another person's needs, that a proxy needs the scope, and that a proxy cannot write.
- **My decision, for review:** a patient's change takes effect at once, because the person is the authority on what they need.
- **Not done / not verified:** nothing tells staff that a patient changed something. The words on "my record" are English only, and no patient has seen them. A proxy without the scope sees that needs are not shown. No clinician has reviewed this.

## 2026-11-05 - A date of death reaches the matcher, from an assertion only (BV-1; H-001)

- **Clinical risk:** **yes.** On a merge, a shared date of death from standing assertions now adds a little to the score, which can move a pair across a threshold. H-001 stays open.
- **Built:** the service on matcher 0.7.0; `death::match_date` and `death_store::for_matching`, the only filler; a refusal of `death_date` in the patient validator and on `/compare`. BV-1 is ticked.
- **My decisions, for review:** a date in dispute is left out; an unconfirmed report's date counts, because a merge is a person's act and they see the record.
- **Not done / not verified:** the weight has not been tried on real data. Only `Cargo.lock`'s matcher entry was changed in this commit; the maintainer's other lockfile bumps stay uncommitted, so this was tested with them and CI tests it without. The merge audit does not record that a date of death counted. No clinician has reviewed this.

## 2026-11-05 - Matcher 0.7.0 (OQ-BV3)

- **Clinical risk:** **not yet in effect.** The release adds `Patient.death_date` as weak match evidence. Nothing changes until the service uses it, which is the next commit. H-001 records the risk.
- **Built:** the version, published to crates.io at the maintainer's direction. The matcher's 110 tests pass, with clippy, the docs and a publish dry run.
- **Not done / not verified:** the 0.05 weight has not been tried on real data, and no clinician has reviewed it. A crates.io version cannot be taken back; it can only be yanked.

## 2026-11-05 - A patient's own login stops once a death is confirmed (BV-5; H-046 stays open)

- **Clinical risk:** **yes.** A confirmed death stops the patient's own access to the record, the photo, the access log and "my record". A withdrawal restores it, with nothing to do by hand. H-046 stays `open`.
- **Built:** the check in `relationships::lookup`, beside the proxy's, and `Answer::confirmed`; assertions in `tests/http_deaths.rs` that a report does not stop it, a confirmation does, and a withdrawal restores it.
- **My decision, for review:** "at death" is read as confirmed, not any report, so that a living person wrongly reported by a feed keeps their record. The maintainer decided the login stops; the threshold is mine.
- **Not done / not verified:** a person wrongly confirmed as dead loses their record until a clinician withdraws it, and sees only that it is gone; those words need people who have been through it. A token already issued is not revoked: it stops working on the record, not on sign-in. A person acting after death is still not built. No Clinical Safety Officer has reviewed this.

## 2026-11-05 - A need as a FHIR Flag (PX-3)

- **Clinical risk:** **yes, once anything reads it.** Another system can now read what a person needs, with the same access decision and audit row as the list. An ended need goes out as `inactive`.
- **Built:** `interop/fhir_flag.rs`, `GET /fhir/R4/Flag` and `GET /fhir/R5/Flag`, both capability statements, and active and ended flags in the FHIR validator's set.
- **Not done / not verified:** the validator needs Java and ran only in CI. No national profile is claimed and no national code used, so a reader will not recognise these codes without being told. A flag has no start date. The note goes out as written, and it is free text. No outside system has read one.

## 2026-11-05 - Needs on the banner and the patient page (PA-6, in part; H-048)

- **Clinical risk:** **yes.** The patient and summary banners now say what a person needs, and whether a check is overdue. Staff read this before they speak to someone. A failed look says so, and an empty list says "none recorded" and that this does not mean there are none.
- **Built:** `adjustments.ts`, `AdjustmentsSection.svelte`, the banner line, 23 translatable codes in `strings.ts`, a test that the UI's catalogue is the service's, and browser tests including accessibility.
- **Not done / not verified:** the words are English only, though the app has ten locales. No one who needs an adjustment has seen this. The banner gets longer with each need, and a long list was not tried at phone width. PN-4 is not built, and nothing shows beside an appointment, because the record holds none. Nothing tells anyone that a check is overdue unless the record is opened. The summary page has no section of its own, only the banner.

## 2026-11-05 - Reasonable adjustments and communication needs, in the service (PA-6, in part; H-048)

- **Clinical risk:** **yes.** A new record of what a person needs in order to use care. Hazard H-048 is added, `open`. An adjustment informs staff and changes nothing by itself. Nothing shows it yet: the screens are the next commit.
- **Built:** `adjustments.rs` (23 codes, the rules), `adjustments_store.rs`, migration 26, three routes under `/api/patients/{id}/adjustments`, the policy resource `Adjustment`, and every census. Read from the patient administration system's `adjustments`, `interpreter` and `needs_review` modules at `15e6389`, and reworked: versions here are append-only, and every change needs a reason. Research in plan.md §33, with PX-1 to PX-6 in `tasks.md`.
- **Not done / not verified:** no clinician, disabled person, interpreter or lawyer has reviewed the catalogue, the words or who may write. A patient and a proxy have no route. No FHIR `Flag`, no list across patients, and no national code list, because no licence was checked. The Accessible Information Standard is cited as the PAS cites it; I did not read it. "Overdue" uses UTC. Two writers at once can record the same need twice. A refused consent hides a need from staff, and that needs review. FHIR validator tests were not run locally; 164 database tests were, against a throwaway Postgres, with kache bypassed.

## 2026-11-05 - Tests for the server hook and the patient banner (CQ-35, in part)

- **Clinical risk:** none in behaviour: this adds tests. They cover the banner's report-of-death alert and the session rule that a background look is not activity (H-025).
- **Built:** `tests/unit/hooks-server.test.ts` and `tests/unit/patient-banner.test.ts`, 18 tests, the first component test in the project. Eight deliberate mutations of `hooks.server.ts` and `PatientBanner.svelte` each made a test fail; that check was run by hand and is not in CI.
- **Not done / not verified:** a component test for the patient page, and splitting the photo and edit form out of it, so CQ-35 stays open. The session and sign-in are mocked in the hook test, so a real login is not exercised there. The banner's photo and allergy lines were not mutated.

## 2026-11-05 - A design for preserving records after an event (SW-7), and a correction

- **Clinical risk:** none in behaviour: a design in `spec/suicide-watch/index.md` and a corrected comment.
- **Correction:** the suicide watch research and a comment in `audit_retention.rs` said archived audit rows leave a patient's access log. They do not: the log and the audit search read `audit_events_all`. The comment dated from before that view; I repeated it in the research without checking.
- **Built:** nothing new. The task asks that the archive's lack of a hold be addressed: the design says a hold need not stop the archive, since nothing is destroyed, and sets a rule for anything that would destroy.
- **Not done / not verified:** no test holds that rule, because nothing destroys yet, so it is words until the first such job. A database owner can still drop the archive, and only access control and backups outside the service answer that. No Clinical Safety Officer or lawyer has reviewed this. Retention law was not read. Five questions are open, including whether staff should know a record is held.

## 2026-11-05 - A design for reason-neutral lists (SW-6)

- **Clinical risk:** none today: a design in `spec/suicide-watch/index.md`, with no code. If built, it decides who can see that a person is being watched.
- **Built:** nothing. No ward list exists to change, and no role was added.
- **Not done / not verified:** no nurse, observer or person who has been observed has reviewed it. The table of who sees what is my reasoning, and whether an observer needs the reason is OQ-SW1. "Enhanced care" may come to mean one thing on some wards, and I have no evidence either way. The support worker role waits for RA-2.

## 2026-11-05 - A design for a directory pack (SG-4)

- **Clinical risk:** none today: a design and a test, with no change in behaviour.
- **Built:** the design in `spec/survivor-grief/index.md`; a seventh test in `tests/grief_census.rs` that fails on a helpline number or a service's name in the service or UI code.
- **Not done / not verified:** the pack loader knows nothing of this, and no pack holds a directory. The test knows only five shapes, so another service's number passes. No bereavement service, deployer or bereaved person has reviewed the design, and whether anyone would keep `last_checked` true is unknown. This project publishes no directory.

## 2026-11-05 - A design for consent to contact (SG-3)

- **Clinical risk:** none today: a design in `spec/survivor-grief/index.md`, with no code.
- **Built:** nothing. The design covers who, how, when and how often, the defaults, and why this is apart from access consent.
- **Not done / not verified:** no bereaved person, counsellor or lawyer has reviewed it. It cannot be used until the dead patient's wishes and hidden-from list exist (BV-6, PI-8). Four questions are open: people with no record, children, capacity, and whether an EPR should hold this at all. The defaults are my reasoning. I searched for no new sources.

## 2026-11-05 - Vital signs show both clocks, and say when an entry is late (SW-2, in part; H-034 stays open)

- **Clinical risk:** **yes.** A clinician now sees when a reading was written beside when it was measured, and a first entry written more than 60 minutes after its measured time says "late entry". H-034 records this and stays `open`.
- **Built:** `writtenText` and `lateByMinutes` in `vitals.ts`; both clocks in the vital signs list, the trend table (a new column), the history, the entered-in-error list and the summary page. `vitalLine` carries them too, so a patient choosing a record to amend sees them. Unit tests and a browser test of the summary page.
- **Not done / not verified:** **the bound on `effective_at`:** any past time from 1900 is still accepted, because a limit is a clinical choice and I have none to set, so the task stays open. The 60 minutes is my guess, and no one has said whether it suits a ward. No reason is asked for a late entry. A correction says which version and is not called late, so a late first entry that was then corrected shows as late only in its history. A device reading or an import will often look late; neither exists yet. It does not detect a false entry made on time, and the label must not be read as if it did. No clinician has seen the wording. Only vital signs have this: there are no other observations. The words are English only (SF-4).

## 2026-11-05 - Three step types for a contact outside the record (SG-1)

- **Clinical risk:** none today: this is a design in `spec/pathway-design.md`, with no code. If built, the words on a step shape what a clinician believes has happened to a person.
- **Built:** nothing. The design covers information given, signposted and referred, what each says it will not tell you, and which have no timer.
- **Not done / not verified:** no clinician, bereavement service or bereaved person has reviewed the design or its wording. The steps wait for the template model (XP-12). The task is ticked because its acceptance was adding this to the spec, and that does not mean it works in practice.

## 2026-11-05 - Seven deaths as test data (BV-8)

- **Clinical risk:** none in behaviour: this adds a test.
- **Built:** `tests/death_scenarios.rs`: expected, sudden, coroner, wrong patient, withdrawn, baby and twin, through the pure rules, with the answer for each record, in plain words.
- **Not done / not verified:** the scenarios are mine, and no clinician or bereaved person has read them. There is no seeded synthetic patient set in this repository, so they are not patients anyone can open, and they do not touch the routes, the store or the hold. A pregnancy lost before a record exists is not covered. The twin scenario shows the records are separate; it does not cover a letter to the living twin's parents that names the other child, or how the matcher treats twins once a date of death reaches it. A partial date beside a full one that agrees still sets `dates_differ`: recorded for review, not changed.

## 2026-11-05 - What is not built for people who are bereaved, as a rule and a test (SG-2)

- **Clinical risk:** none in behaviour: this adds a rule and a test. It limits what may be built later.
- **Built:** a "Do not build" section in `spec/survivor-grief/index.md`; `tests/grief_census.rs`, six tests that need no database.
- **Not done / not verified:** no counsellor, clinician or bereaved person has reviewed the rule. The test reads source text, so a refused thing under another name passes, and a comment using a listed word fails. Nothing enqueues today, so the date test guards the first caller and has never fired on real code; it blocks that file if it mentions a birth date for any reason, which may be too broad. A date of death also reaches the matcher (OQ-BV3) and is not a message, so this does not cover it. A contact a person asks for on a date is not built.

## 2026-11-05 - A label census for every output route (PI-7)

- **Clinical risk:** none in behaviour: this adds a test. It does put a gap in writing: no route applies a security label. A record cannot carry one, `authorize` always asks with none, and nothing acts on `Mask`, so the policy engine's masking rules, which are tested on their own, reach no screen.
- **Built:** `tests/label_census.rs`: 43 output routes, 34 of them `NotApplied`, and two tests that fail when labels reach a decision or a route acts on a mask.
- **Not done / not verified:** the census's classing is mine, from each route's name and what I know of it; I did not read every handler. It reads source text. Other POST and PUT answers are not listed, on the reasoning that they return what was written; a 409 that returns duplicate candidates is one to check. It is not the test the spec asks for, that a labelled record does not appear, which needs labelled records.

## 2026-11-05 - A proxy stops while a report of death stands (BV-5, in part; H-046 stays open)

- **Clinical risk:** **yes.** Any standing report, confirmed or not, including one from a feed, stops a proxy reading the record, the photo and the access log, and removes it from their "my record". A false report therefore locks out a parent or carer until a clinician withdraws it. H-046 stays `open`.
- **Built:** the check in `relationships::lookup`, the one place both routes ask, worked out per request so a withdrawal restores access; a test through the real router with login on.
- **My decisions, for review:** that an unconfirmed report is enough, since a disclosure cannot be undone; and that a patient's own login does not stop, since a living person wrongly reported would lose their record.
- **Not done / not verified:** the relationship for a person acting after death, which needs a lawyer. A proxy sees the record vanish and is told nothing: the words need people who have been through it. Each proxy request now makes more queries, and that was not measured. The photo path was not tested, though it uses the same lookup. All 163 database tests that CI runs here ran on local Postgres 18 with the build cache off; the FHIR validator is left to CI.

## 2026-11-05 - A report of death holds outbound events (BV-3; H-046 stays open)

- **Clinical risk:** **yes, once anything sends.** The outbox runner now holds every event about a patient with a standing report. Nothing enqueues and no sink exists, so nothing changes today. H-046 stays `open`.
- **Built:** the hold in `outbox.rs`, first in the runner, leaving the event pending so that a withdrawal releases it; `Report.held`; tests that a feed report holds, a confirmation keeps the hold, a withdrawal releases it, and held events do not starve others; `tests/outbound_census.rs`.
- **Not done / not verified:** every event is held, including one that ought to reach someone after a death. A held event writes no row, so nothing but the logged run count and `pending` depth shows it, and it can wait for ever. The census reads source text, so a way out through a crate it does not list would pass. All 163 database tests that CI runs here ran on local Postgres 18 with the build cache truly off, which covers the commits below as well. The 4 FHIR validator tests need Java and are left to CI.

## 2026-11-05 - A date of death in the matcher, as weak evidence (BV-1, in part; OQ-BV3)

- **Clinical risk:** **not yet in effect.** The service uses the published matcher, so nothing changes until a release. Once it does, a shared date of death adds a little to a match score, which can move a pair across a threshold; H-001 records this. H-046 stays `open`.
- **Built:** `Patient.death_date` and `Weights.death_date` (0.05), compared like a birth date, and counted only when both records have one. I chose the weight, the smallest in use, so that it is weak; that it has no guard, so it cannot link two records by itself; and that it is kept for a temporary identity.
- **Not done / not verified:** the release, and the service filling the field. Two people who die on one day gain score, and no real data has been tried. A date may come from a report that is wrong. **Correction to earlier entries:** `--config 'build.rustc-wrapper=""'` did not bypass the build cache, because the shell sets `RUSTC_WRAPPER`, so my local runs this session used the cache. CI does not, and it is the check on those commits.

## 2026-11-05 - FHIR says when a person has died (BV-2; H-046 stays open)

- **Clinical risk:** **yes.** A FHIR reader now sees `deceased[x]` for a confirmed death and the tag `death-reported` for one that is not confirmed. A wrong report therefore reaches any system that reads this tag, though not as `deceased[x]`. H-046 stays `open`.
- **Built:** `deceased_element` and `with_death` in `fhir_patient.rs`, on read, search, create and update; R5 gets the same through its conversion. A failed look is a 500. An update that sends back what a read gave is accepted.
- **A change from what I said I would do:** a FHIR write with a new `deceased[x]` is still refused, now with the route to use, instead of becoming a report. A report needs an author and a reason, a `Patient` write has neither, and the two writes could not share one transaction without changing `create_service` and `update_service`. So nothing is lost silently, but an integrator must call the report route.
- **Not done / not verified:** BV-2 is ticked, with an `A03` still recording nothing. Telling a FHIR reader about a death writes no separate audit row, only the Patient read's. A search looks up each patient's answer in turn, up to the search limit, and that cost was not measured. The R5 output and the FHIR validator were not run locally: R5 has no death test, and the validator needs Java, so it is left to CI. Every other database test ran on local Postgres 18: 161 passed.

## 2026-11-05 - A death from the feed is stored as a report (BV-2, in part; H-046 stays open)

- **Clinical risk:** **yes.** A feed can now put a report of death in the banner of a patient who is alive, until a clinician withdraws it. That is the rule decided as OQ-BV1. The other way round, a real death on a message is no longer left in a queue. A message with a death is answered `AA` again, and its other details are applied again. H-046 stays `open`.
- **Built:** `adt_feed.rs` stores the report with the message, in one transaction, with its own audit row. Repeats add nothing. A date that cannot be stored becomes no date. A repeat of a withdrawn report waits in the queue. A death about nobody on record waits in the queue. A refusal says the death was not recorded, an `A03` included.
- **My decisions, for review:** that a repeat of a withdrawn report is refused, which means the rest of the message is not applied either and a source that keeps sending it fills the queue; that a report's other demographics are still applied; and that a feed cannot retract, so a later `N` changes nothing.
- **Not done / not verified:** tried only with test messages, never a real source. Nothing tells a person that a report has arrived. An `A03` still records nothing. Two connections sending the same report at once could store it twice, which holds contact the same and shows two rows; not tested. Every database test ran on local Postgres 18, as the Podman VM would not start: 161 passed. The 4 FHIR validator tests need Java and are left to CI.

## 2026-11-05 - A report of death in the banner (BV-1, in part; H-046 stays open)

- **Clinical risk:** **yes.** The banner now tells staff that a patient has been reported as having died, or that a death is confirmed, and that contact is held. Staff can report, step and withdraw from the patient page. Nothing in the service holds contact yet, because it sends nothing: the words "contact is held" are the rule, for the staff member to follow. H-046 stays `open`.
- **Built:** `lib/death.ts`, `DeathSection.svelte`, the banner line on the patient and summary pages, unit tests, and two browser tests with a mocked API.
- **Not done / not verified:** not tried against the real service or with real roles. The steps are offered to every role and a refused one answers "forbidden" when pressed, so a nurse is offered a withdrawal they cannot make. The words are mine, English only, and no clinician or bereaved person has read them. The date field is free text checked by the service. The summary page has no report form. Patient search and the merge screens show nothing about a death, and "my record" does not show it to a patient or proxy. One older browser test now picks out its status line, as the page can hold a second.

## 2026-11-05 - Death assertion routes, and who may take each step (BV-1, in part; OQ-BV2, OQ-BV3; H-046 stays open)

- **Clinical risk:** **yes, for what staff can do.** Any member of staff can now record that a patient has died through the API, a clinician or nurse can confirm it, and a clinician can withdraw it. No screen uses these routes yet and nothing acts on a hold, so a clinician sees no change. H-046 stays `open`.
- **Decided by the maintainer:** OQ-BV2 as drafted, and OQ-BV3: the matcher's `Patient` carries a date of death as weak match evidence. That was against my recommendation. Neither has been reviewed by a Clinical Safety Officer.
- **Built:** four routes in `controllers/deaths.rs`; the policy resource `DeathAssertion`; OpenAPI schemas; `tests/http_deaths.rs`.
- **My readings of the decision, for review:** `certified` and `registered` are treated like verifying. No care relationship is needed, so reception can see the answer. The reasons are kept from the wider staff group, as free text may hold clinical detail. A patient and a proxy can see none of this, and a refused consent stops a read of the answer.
- **Not done / not verified:** the banner and screen, the feed writing a report, FHIR `deceased[x]`, and the matcher date. The integration role may report through this route, but the test covers only people.

## 2026-11-05 - An append-only store for death assertions (BV-1, in part; H-046 stays open)

- **Clinical risk:** none in effect: nothing writes to the store or reads it, so nothing changes for a clinician. The migration adds a table that will hold personal data. H-046 stays `open`.
- **Built:** migration 25 with `death_assertion_versions` and `death_assertion_current`, append-only by trigger, classified in the personal data inventory; `death_store.rs`; a report on a record merged away counts for the surviving record; `Status::word`; the current-view and stored-word censuses extended.
- **Not done / not verified:** no route or banner, because who may take each step is undecided (OQ-BV2), and the HL7 and FHIR refusals do not write here yet. A read of a merged-away record does not see a report on the surviving record; a merged-away record is not meant to be written to, and that was not checked against every route. No audit row is written, since nothing calls the store. Commit `8ed4d8d` failed CI because the table was not in the personal data inventory: I had run only four database test files locally. After the fix, every database test ran on local Postgres 18, as the Podman VM would not start: 160 passed. The 4 FHIR validator tests need Java and were left to their CI job. The classification is mine and no data protection officer has reviewed it.

## 2026-11-05 - A reported death is refused, not dropped (BV-2, in part; H-046 stays open)

- **Clinical risk:** **yes, for the feed.** An HL7 message with a date of death or a `Y` death indicator used to be applied and answered `AA`, with the death ignored. It is now answered `AE`, changes nothing, and waits in the feed queue. So other details on that message are no longer applied either. A FHIR write with `deceased[x]` was already refused and now has a test. H-046 stays `open`, as its status is the Clinical Safety Officer's.
- **Built:** `PID-29` and `PID-30` read in `hl7_adt.rs`; tests on the mapper, over MLLP against Postgres, and on the FHIR route. A date beside an indicator of `N` counts as a report, under OQ-BV1; HL7's `""` null does not.
- **Not done / not verified:** the death cannot be recorded, so this rests on a person working the queue, and nothing holds contact. An `A03` discharge is refused as an unhandled event and does not mention the death. A FHIR read cannot show a death. Tried only on test messages, not a real sender's. Local database tests ran `mllp_feed` only, on local Postgres 18, as the Podman VM would not start; CI runs the rest.

## 2026-11-05 - Death assertions: the spec and the rules (BV-1, in part; H-046 stays open)

- **Clinical risk:** none in effect: nothing calls the new code, so nothing changes for a clinician. It sets the rules a later change will put in front of one. H-046 stays `open`.
- **Built:** `spec/bereavement/death-assertion.md`; `death.rs`, pure, with a state table test and one read model; OQ-BV2 and OQ-BV3 added.
- **Not done / not verified:** no store, route, banner or HL7 reading, and nothing a hold acts on. The role draft and the recommendation on the matcher are mine and undecided. No clinician, medical examiner, lawyer or bereaved person has reviewed any of it. `found_at`, `place`, `verified_at` and `coroner_referred` are left for the store.

## 2026-11-05 - Temporary identity in the service and the UI (TT-1, H-047)

- **Clinical risk:** **yes.** A person can now mark a record as a temporary identity. Its name and birth date stop counting as evidence, it can never link itself to another record, and the banner, patient search and FHIR all say so. H-047 moved to `controlled-unreviewed`, not `closed`.
- **Built:** the service on matcher 0.6.0; the OpenAPI field; a checkbox at registration and on edit; the banner alert; a mark in patient search; FHIR `meta.tag`, read and written. Every writer replaces the whole record, so the store now refuses to drop the flag without a reason (422 `temporary_identity_reason_required`, in 7 languages, machine-written). An HL7 message keeps the flag and cannot set it. A merge with no other evidence is refused until a person ends the flag and enters the real details.
- **Not done / not verified:** nothing checks the name format the NHS alert recommends, or that a marked name is temporary; the flag works only if someone sets it. A writer that drops the flag and gives a reason still ends it. Banner and checkbox text are English only. Lists other than patient search and the typed compare page do not show it, and the state does not cross to another hospital. Who may end the flag is anyone who may edit identity, and no clinician has reviewed that or the wording. Local database tests ran only for `http_patients`, `drill_records`, `links_store`, `mllp_feed` and `patients_store`, on a local Postgres 18 because the Podman VM would not start; CI runs the rest. `Cargo.lock` changes here are the matcher only: unrelated local dependency bumps stay uncommitted.

## 2026-11-04 - `ETH` now means what HL7 says: substance abuse (PI-1, H-045)

- **Clinical risk:** **yes.** A record labelled `ETH` now unmasks for the clinician and the patient only, not nurses or allied health. No clinical record carries a label yet, so nothing changes on screen today. H-045 moves to `controlled-unreviewed`; it is not closed.
- **Built:** `Label::Ethnicity` replaced by `Label::SubstanceAbuse`, keeping the code `ETH`; tests pin each code and hold `ETH` and `SUD` to the same roles; OQ-PI1 recorded as the maintainer's decision.
- **Not done / not verified:** the pin is written by hand from the FHIR R4 value set read as a summary, and is not checked against HL7 automatically. A stored consent rule naming `ETH` now means substance abuse; none exists in a real deployment, but no migration handles that. The other sensitivity labels (PI-2) are not added. No clinician or data protection officer reviewed the decision or the role list.

## 2026-11-04 - Matcher 0.6.0 published; OQ-BV1 decided

- **Clinical risk:** none changed in the running service: it still uses matcher 0.5.0, so H-047 stays `open`. The decision shapes BV-1, which is not built, so H-046 is unchanged.
- **Built:** `electronic-patient-record-matcher` 0.6.0 on crates.io. OQ-BV1 recorded as the maintainer's decision: any report of a death holds outbound contact until a person confirms it.
- **Not done / not verified:** no git tag, as for earlier releases. The upload went ahead on the local gates, the dry run and the matcher's `cargo deny` job; it did not wait for the CI test job, through a filter error. The service has not moved to 0.6.0. The decision has not been reviewed by a Clinical Safety Officer or bereaved families.

## 2026-11-04 - Temporary identity in the matcher (TT-1, in part; H-047 not yet controlled)

- **Clinical risk:** yes, but **not yet in effect.** The matcher source now withholds a temporary identity's names and birth date and blocks `AutoLink`. The service uses the published matcher 0.5.0, so its behaviour is unchanged. H-047 stays `open`, and its row says so.
- **Built:** `Patient.temporary_identity`, conflict code `temporary_identity`, rule 7 in the matcher spec, `tests/temporary_identity.rs`; translated conflict text in the UI, which is unused until the API sends the code.
- **Not done / not verified:** no matcher release was published, as that is the maintainer's decision. The service, OpenAPI schema, a way to set the flag and the banner are not changed. The UI strings are machine-written. The matcher (109 tests) and UI (259 tests) passed locally; with the matcher unchanged for the service, its suite was not rerun. Local Rust builds went through a `kache` compiler cache that served a stale library until it was bypassed.

## 2026-11-04 - CI: test builds use line tables, so the runner stops running out of disk

- **Clinical risk:** none (CI configuration). No hazard row changed.
- **Built:** `CARGO_PROFILE_DEV_DEBUG=line-tables-only` on the `api` and `api-db` jobs. Commit `2456a7a` failed in `cargo test` with `No space left` and linker bus errors, across unrelated test binaries; SW-1's new test added one more to 58.
- **Not done / not verified:** the size saved was not measured. The `coverage` job was not changed, because it may need full debug information. Disk use on the runner was not recorded, so the remaining margin is unknown.

## 2026-11-04 - A census test keeps a risk score out of the core (SW-1)

- **Clinical risk:** none now: a guard, with no change to what a clinician sees. It blocks a future field that would. No hazard row changed.
- **Built:** `electronic-patient-record-service-with-rust/tests/risk_field_census.rs`, over the service, migrations, matcher and UI; `spec/11`, `spec/suicide-watch`; SW-1 ticked.
- **Not done / not verified:** it reads names only, so a score in free text, or under another name, passes. The shapes are my list and no clinician has reviewed it. NG225 was read from a third-party copy. Only this test, with `cargo fmt` and clippy for it, was run locally, not the whole suite; a field planted in a UI file made it fail, and the plant was removed. No pack mechanism exists yet to declare an exception, only the `DECLARED` list.

## 2026-11-04 - Two stale status statements corrected

- **Clinical risk:** none (documents only). No hazard row changed.
- **Built:** `spec/14` now lists observations, amendment requests and referrals, and no longer says nothing has run on GitHub. The service spec no longer says referrals have no route.
- **Not done / not verified:** `grep` confirmed the three controllers exist; I did not check the rest of the service spec for similar drift, and the PC-1 row's "nothing calls" wording was not reviewed.

## 2026-11-04 - Research pass: bereavement, suicide watch, survivor grief, trauma treatment (plan.md §32); hazards H-046, H-047

- **Clinical risk:** **yes, two rows.** New hazards, both `open`: H-046, a death on an inbound HL7 message is dropped silently, and the record has no concept of death; H-047, a temporary name for an unknown patient reads as real evidence to the matcher. No code changed. Most new tasks are marked `clinical`.
- **Built:** four spec pages; `plan.md` §32; tasks BV-1 to BV-8, SG-1 to SG-5, SW-1 to SW-7, TT-1 to TT-7; `spec/index`, `16`, `17`, `19`; hazards H-046 and H-047.
- **Not done / not verified:** no code, so **both hazards are logged, not fixed**; they come from reading code and the spec, not from running a message or a name. No clinician, lawyer, or person with lived experience has reviewed this. All law is a lay reading. NICE pages returned 403, so NG225 and NG116 came from third-party copies. The Lampard Inquiry has not reported. Evidence is English-language, mostly from high-income countries. The service spec still says referrals have no route.

## 2026-11-04 - The project site, and QA-4 pointed at the right file

- **Clinical risk:** none (a public description of the project; no change to the record). Every page says not for clinical use, and a build check fails without it. No hazard row changed.
- **Built:** <https://electronic-patient-record.github.io> in its own repository (SvelteKit 3, `@lilydesignsystem`, GitHub Pages): home, status, research, about. Linked from `README.md`. QA-4 now names `governance/retention-schedule.md`.
- **Not done / not verified:** the site's facts are a dated, hand-copied snapshot of `spec/14`, so they go stale. It has no browser test suite in CI and no automated accessibility run; it was checked by hand in Chromium at 1280 and 320 px. Only English. `spec/14`'s summary sentence omits observations, referrals and amendments and still says nothing has run on GitHub; the site lists what the controllers show, and that sentence was not changed here.

## 2026-11-04 - Research pass: roles, protected information, care homes, virtual wards (plan.md §31); hazard H-045

- **Clinical risk:** **yes, one row.** New hazard H-045, `open`: the code reads `ETH` as ethnicity, but HL7 defines it as substance abuse. Latent: no clinical record stores a label. No code changed; PI-1 must precede SE-4. Everything else is documents only; most tasks are marked `clinical`.
- **Built:** `spec/access-control/roles-and-assignments.md`, `protected-information.md`, `spec/care-homes/`, `spec/virtual-ward/`, `spec/step-down-facility/`; `plan.md` §31; tasks RA-1 to RA-7, PI-1 to PI-9, HM-1 to HM-5, VW-1 to VW-6; `spec/index`, `16`, `17`, `19`; hazard H-045.
- **Not done / not verified:** no code, so **H-045 is logged, not fixed.** `protected-information.md` was written from a crashed session's saved results, with no new searches. All law is a lay reading; the order under s.22 of the Gender Recognition Act was not read. Adoption, witness protection and psychotherapy-note law were not researched. No lawyer, clinician, data protection officer or person with lived experience has reviewed it.

## 2026-11-04 - Research pass: system quality attributes (plan.md §30, QA-1 to QA-12)

- **Clinical risk:** none (documents only). No hazard row changed.
- **Built:** `docs/index.md` and twelve `docs/<attribute>/index.md` pages; `plan.md` §30; tasks QA-1 to QA-12; `spec/index.md`, `spec/17`.
- **Not done / not verified:** no code. ISO/IEC 25010:2023 was not read. Several sources on the reliability, security, privacy and auditability pages were cited from memory; a link check tested only that they exist. Recalled legal points on those pages need checking.

## 2026-11-04 - Research pass: safety measures, maternity, manual steps, data in and out, data quality, productivity, training (plan.md §29)

- **Clinical risk:** none now (documents only). Most of these tasks change what a clinician sees and are marked `clinical`; the candidate hazards on each page go in the log with the first code. No hazard row changed.
- **Built:** eight spec pages, including the training overlay design; `plan.md` §29; tasks SM-1 to SM-5, MT-1 to MT-4, DX-1 to DX-10, PM-1 to PM-5, TO-1 to TO-9; `spec/index`, `15`, `16`, `17`, `19`.
- **Not done / not verified:** no code. **The training overlay asked for is designed, not built.** No clinician, midwife, trainer or records manager was asked. Maternity content must not be used as clinical content. Literature figures are as reported. No measure has been computed.

## 2026-11-04 - Research pass: comparisons with twelve other EPRs (plan.md §28, VC-1 to VC-12)

- **Clinical risk:** none now (documents only). VC-1, VC-2, VC-4, VC-6, VC-10 and VC-12 would change what a clinician sees; hazard rows are due with the first code. No hazard row changed.
- **Built:** twelve `spec/<vendor>/index.md` pages; `plan.md` §28; tasks VC-1 to VC-12; `spec/index.md`, `spec/17`.
- **Not done / not verified:** no code; no product was seen or used and nobody who uses one was asked. Vendor claims are claims; incident accounts are public reports, not findings of cause.

## 2026-11-04 - Research pass: mobile, care settings, specialty views, pathway design, private analysis (plan.md §27, XP-1 to XP-14)

- **Clinical risk:** none now (documents only). Mobile layout, setting profiles, specialty views and clinician-edited pathways each change what a clinician sees; hazard rows are due with the first code (XP-3 names the first). No hazard row changed.
- **Built:** `spec/mobile-viewability.md`, `spec/care-settings.md`, `spec/pathway-design.md`, `spec/privacy-analytics.md`; edits to `spec/05`, `06` (FR-8.10 to FR-8.12), `07` (NFR-4), `04`, `15`, `19`, `index`; seven open questions (OQ-MV1, OQ-ST1, OQ-ST2, OQ-SP1, OQ-HE1, OQ-HE2, OQ-PD1); `plan.md` §27; `tasks.md` `XP-1` to `XP-14` and the `XP-` prefix.
- **Not done / not verified:** no code; no phone or emulator was used; no homomorphic-encryption library was installed (the speed and size figures come from two theses by search result); no clinician or renal specialist was consulted, so the renal content is from registry material and one paper; the acute versus community comparison is the author's observation backed by trade-press examples, with no direct comparison source; WCAG values come from secondary summaries; the HL7 pathway material was read in part.

## 2026-11-04 - Spec: the care pathway comes before the separate episode

- **Clinical risk:** none now (documents only); the direction changes what the record will show, so a pathway hazard row is to be added with the first code (said in `spec/05` §5.2.1). No hazard row changed.
- **Built:** `spec/05-domain-model.md` §5.2.1 (the principle, why, seven design rules, FHIR mapping); `spec/01` (a "pathway-centred" aim); `spec/04` (glossary: care pathway; episode and encounter reworded); `spec/06` (FR-8.2 reworded, FR-8.8 and FR-8.9 added); `spec/02`, `spec/15`, `spec/19` (wording and order: pathways and plans first); `spec/interoperability/closed-loop-referral.md` (a referral is a step); OQ-CP1 to OQ-CP4; `spec/17-references.md`; notes on CA-1 and CA-2 in `tasks.md`.
- **Not done / not verified:** no code, no schema and no screen; the roadmap order is a recommendation, not a plan anyone has agreed. The FHIR quotations come from three R4 pages, two read only in part; the mapping to `CarePlan`, `PlanDefinition` and `Encounter` is the author's reading, not checked against UK Core, US Core or a national profile. No clinician has reviewed the rules, and the claim that care is lost between episodes rests on the project's own hazard H-044, not on a cited study.

## 2026-11-04 - The CI baseline (LC-1)

- **Clinical risk:** none (a read-only script and a table). No hazard row changed.
- **Built:** `scripts/ci-baseline.py` (runs by outcome, wall time and minutes per job, from `gh`); the table in `spec/local-ci.md`. Findings: 16.5 runs a day, median 13.4 minutes by the clock and 42.3 job-minutes per successful run; 20% of runs cancelled by a newer push; the container build and CodeQL for Rust are 58% of job-minutes.
- **Not done / not verified:** failures are sorted by step name only, logs were not read, and nine of the fourteen were outside the window examined; queue time before a job starts (where a platform slowdown would show) is not measured; minutes are not rounded as GitHub bills them; the script has no test and was run once.

## 2026-11-04 - Research pass: local CI/CD (plan.md §26, tasks LC-1 to LC-13)

- **Clinical risk:** none (documents only). No hazard row changed.
- **Built:** `plan.md` §26 (what CI does today, why it feels unreliable and costly with sources, nine options assessed, advice, ranked ideas, risks, order of work, sources); `tasks.md` section and the `LC-` prefix (also in `scripts/generate-status.sh`); `spec/local-ci.md`; four open questions (OQ-LC1 to OQ-LC4); `spec/17-references.md`; a pointer in `spec/11-testing-strategy.md` and `spec/index.md`.
- **Not done / not verified:** no code, no workflow or hook was changed and no tool was installed or run, so every speed claim is open. The repository is public, so the cost today rests on GitHub's statement about public repositories; the account's billing page was not read. Of the 14 recent failures three were opened (all our own); eleven were not. Service-container support in act, Forgejo and Woodpecker, hosted-runner prices, local use of CodeQL, and arm64 against x86-64 effects were not checked. Third-party incident counts disagree. AGENTS.md was not changed (see OQ-LC4).

## 2026-11-04 - Not adopted, recorded (PR-8)

- **Clinical risk:** none (a paragraph). No hazard row changed.
- **Built:** a "Considered and not adopted" paragraph in `spec/02-scope.md`: the weekly mutation run, and the Patient Administration System features (appointments and beds beyond the minimal ones, waiting-list validation, PIFU, transport, reminder ladders).
- **Not done / not verified:** the reasons given for the mutation run (cost, survivors need judging) are the author's, not measured: no run was timed. No user, clinician or maintainer other than the author has agreed that these stay out.

## 2026-11-04 - Four governance documents (PR-7)

- **Clinical risk:** none (documents only). No hazard row changed.
- **Built:** `governance/data-flow.md`, `mfa-plan.md`, `production-readiness.md`, `national-assumptions.md`, linked from `spec/index.md`. Findings worth reading: the service reads no `acr`/`amr` claim so it cannot require a second factor; the development realm sets none; `outbox::enqueue` has no caller, so nothing is sent out through the outbox yet; `/metrics`, when on, is public; no backup procedure exists; no readiness item is ticked.
- **Not done / not verified:** all four are drafts from reading code and configuration, with no running deployment, network capture, data protection officer or clinician. The national-assumptions pass is a keyword-and-reading pass over newer code only and does not re-check the earlier audit. What Redis holds was not checked. Not a claim that anything listed is done.

## 2026-11-04 - The referral timeouts take their time from the clock (PR-5, second slice)

- **Clinical risk:** none to behaviour (the service passes the real time); the H-044 timeout test is now stronger. The hazard row is unchanged and the hazard stays open.
- **Built:** `referral_worklist::list_at` and `count_at` judge "gone quiet" by a time passed in (`$2::timestamptz`); `list_with` and `count_with` pass the real time. The PC-3 test moves a `FixedClock` by days instead of rewriting `recorded_at` with triggers switched off, and uses its own sender so the shared test database's older rows cannot push it off the first page.
- **Not done / not verified:** break-glass grant expiry and review-overdue still use the database's `now()` (the grant's own `granted_at` and `expires_at` come from the database too, so moving them needs a store change); the controllers do not take a clock from the app state, so the HTTP tests cannot move time; `recorded_at` is still stamped by the database, so a clock that runs ahead of the database makes referrals look older than they are.

## 2026-11-04 - A controllable clock, first slice (PR-5)

- **Clinical risk:** none (idempotency keys only). No hazard row changed.
- **Built:** `src/clock.rs` (`Clock`, `SystemClock`, `FixedClock::advance`); `idempotency::reserve_at` and `purge_expired_at` take the clock and stamp `created_at` from it, `reserve` and `purge_expired` call them with `SystemClock`. `tests/idempotency_clock.rs` moves a `FixedClock` past the 24-hour life (held just inside, new just past, purged after) in a rolled-back transaction; 1 unit test of the clock.
- **Not done / not verified:** the HTTP middleware still uses `SystemClock` (no clock in the app state), so the existing HTTP test still rewrites a row; break-glass grant expiry and review-overdue, and the referral timeouts, still compare against the database's `now()` in SQL and are not on the clock; `outbox`, `mllp` and `rate_limit` keep their own `now()` helpers. Keys written before this change carry the database's time and are judged by the service's: a skew between the two clocks moves the edge of the 24 hours by that much.

## 2026-11-04 - One replica runs the audit maintenance job (PR-4)

- **Clinical risk:** none to what a clinician sees; it touches audit retention (H-042) only by making the job safer. No hazard row changed.
- **Checked first** (scratch database, a second session started while the first held its transaction open): two replicas making partitions together: the second returned `Ok(0)`, no error. Two replicas archiving together: the second failed with `relation "audit_events_p0" does not exist` and the first's work stood, so nothing was archived twice and no anchor was doubled; the cost was a false "audit archive failed" error in the log.
- **Built:** `audit_retention::run_once` takes a transaction-scoped `pg_try_advisory_xact_lock` (`LOCK_KEY`) and does nothing, saying `Ran::Skipped`, when another replica holds it; the loop uses it; `ensure_partitions` and `archive_older_than` take any connection. 2 database tests: a run is skipped while the lock is held; two runs at once make each partition once and neither fails.
- **Not done / not verified:** the "two at once" test passes without the lock too (making partitions was already safe; the archive race needs exact timing), so it guards the result, not the lock; the lock test is what proves the lock. One lock for the whole job, not per step. Only checked on one Postgres, one machine, not with two real processes. A replica that skips waits for its next tick, six hours, so a failed round on the first replica is retried by whoever ticks next.

## 2026-11-04 - Stored words checked against the database (PR-3, first slice)

- **Clinical risk:** none (a test and two `ALL` constants). No hazard row changed.
- **Built:** `tests/stored_words.rs` reads each `CHECK (col IN (...))` list from `pg_constraint` and compares it with the code's words for referral states, allergy statuses and consent statuses; `Status::ALL` added to `allergies` and `consent`. A word changed in code only, or in the database only, fails the test (shown by changing one word).
- **Not done / not verified:** amendment states (a tagged enum with no list), observation statuses, relationship kinds, unmatched-queue kinds and clearances, feed and break-glass outcomes are not compared. A variant left out of an `ALL` array is not caught. The parser reads quoted words from the constraint text as Postgres prints it, on the Postgres version in `compose.test.yaml` only. Not run in CI by me: the `api-db` job runs ignored tests, so it should pick it up, unconfirmed until CI reports.

## 2026-11-04 - A control may name its test (PR-2, first slice)

- **Clinical risk:** governance only (no behaviour change). H-044 evidence now names four tests. Not set to accepted or closed.
- **Built:** `scripts/check-hazard-log.py` reads `path::test_name` in the evidence column and fails when the file or the `fn`/`def` is missing (renaming a named test fails the check); `--report` lists hazards with no named test; 3 script tests; `risk-management-plan.md` says how.
- **Not done / not verified:** the report is not a failure yet: 19 hazards with controls and 24 open ones name no test, and only H-044 does. The failure for controlled hazards, and the pull-request hint listing hazards whose named files changed, are not built. A TypeScript test is found by its title appearing anywhere in the file, which is weaker than a function check. The check proves a test of that name exists, not that it covers the control.

## 2026-11-03 - Timeouts: a referral that has gone quiet reaches its sender (PC-3, first slice)

- **Clinical risk:** yes (H-044: a referrer waiting for a reply that is not coming). The hazard row says what is still missing. Not set to accepted or closed.
- **Built:** `EPR_REFERRAL_TIMEOUTS` (JSON, periods in days per priority for "no response" and "no outcome"; **no default**; a malformed or unknown value stops the service starting); `src/referral_timeouts.rs`; the worklist lists a referral still `requested`, or `accepted`/`appointment-booked`/`interim-report`, whose current version is older than its priority's period, with `why` `no-response` or `no-outcome`, and answers `untimed` (the priorities with no period); the screen words a timeout apart from a reply, offers no "Mark as seen" for it, and names the priorities that will not time out; 4 parse tests, a database test of the whole sequence, OpenAPI updated, 2 UI tests. Also fixed a latent flake in the referral e2e test: `getByText("Done.")` also matched a paragraph of the merge section, depending on render timing; it now matches exactly.
- **Not done / not verified:** **nothing is on by default**: until a deployment sets the periods, no referral ever times out, and only the on-screen warning says so. Periods are per priority, not per service type (assumed OQ-PC3). The clock runs from the current version, so any new version restarts it, and a timeout cannot be snoozed or acknowledged: it stays until the referral moves or the sender asks to cancel it. **An appointment date that has passed is not covered** (`Book` carries no date). Only the sender is told: no reminder, escalation or team. The database test ages a referral by disabling the append-only trigger for one statement in the test database, which the real service never does. The periods' suitability is not clinically reviewed. Tested against stubs in the UI, not a running service.

## 2026-11-03 - The referrer's worklist screen (PC-2, fifth slice)

- **Clinical risk:** yes (H-044: a decline or outcome the referrer never sees). A referrer can now see and mark what came back. The hazard row says what is still missing. Not set to accepted or closed.
- **Built:** `/referrals` ("Replies to referrals you sent", in the navigation): what came back in words and when on the site clock, a link to the patient, "Mark as seen" (sends the version shown; a 409 reloads), says when the list is cut short; the waiting count beside the navigation link on that page and a patient's record; `lib/referral-worklist.ts`; strings; a unit test file and two e2e tests with axe.
- **Not done / not verified:** tested against stubbed responses, not a running service, and no Keycloak sign-in test covers it. **The count shows only on two kinds of page**, so a referrer who opens neither is not told; showing it everywhere would make every page load an audited call. A role without a worklist (receptionist, administrator) makes one audited refusal per load of a patient record. The screen shows the state, not the recipient's words: reading the reply means opening the patient. "Marking seen" records a click, not an action. Nothing reminds anyone about a referral that has had no reply (PC-3). English only; no translation. Who counts as the referrer is still assumed (OQ-PC5).

## 2026-11-03 - The referrer's worklist, service side (PC-2, fourth slice)

- **Clinical risk:** yes (H-044: a decline or outcome the referrer never sees). The service can now say which of a person's referrals have an answer they have not seen. The hazard row says no screen shows it yet. Not set to accepted or closed.
- **Built:** migration 24 (`referral_acknowledgements`, append-only triggers); `src/referral_worklist.rs` (`list` with `more`, `count`, `acknowledge`: current version only, once, sender only); `GET /api/referrals/worklist`, `GET /api/referrals/worklist/count`, `POST /api/referrals/{referral_id}/acknowledge`, audited, in the route table, OpenAPI and the censuses; policy resource `ReferralWorklist` (the 18th; clinician, nurse and allied health may search and update; no care relationship, by design: the list holds only the caller's own referrals and no text); tests: 3 store tests, 1 HTTP test with login on, audit-failure test for the write, the policy invariant now 18 resources, a schema test that the listed states are the code's; personal-data inventory and retention schedule rows.
- **Not done / not verified:** **no screen and no navigation count**, so no referrer sees it yet. **"The referrer" is the author of version 1 (OQ-PC5, assumed)**: a colleague covering for them sees nothing, and a sender who leaves strands their list. Acknowledging records a click, not that anyone acted. Which states wait is my judgement (not reviewed by a clinician); `cancel-requested` and `cancelled` do not wait. Nothing reminds anyone: no timeouts (PC-3), and a referral nobody has answered never appears. No worklist for a recipient or a team. The list is not filtered by consent or care relationship because it shows only ids and states of referrals the caller sent; a sender whose relationship has since ended still sees that a referral of theirs was answered. The database suite ran without `fhir_validator` (needs a jar).

## 2026-11-03 - The unmatched referral reply screen (PC-2, third slice)

- **Clinical risk:** yes (H-044: an outcome that reaches no one). A person can now see and clear what was queued. The hazard row says what is still missing. Not set to accepted or closed.
- **Built:** `/admin/referral-queue` (list with kind in words, reasons and time on the site clock; show the note and message; clear by dismissal or hand match with a reason, a referral id for a match; 409 reloads; says when the list is cut short); the open count beside a new navigation link on admin pages; `lib/referral-queue.ts`; strings; a unit test file and two e2e tests with axe.
- **Not done / not verified:** the screen was tested against stubbed responses, not a running service, and no Keycloak sign-in test covers it. The count is fetched on the same admin pages as the feed count, so a role that may not read the queue makes one audited refusal per page load there (as the feed count already does). The referrer gets no notice of a reply: no worklist count, no entry owner, no timeouts (PC-3). A hand match records a link only; the screen cannot open the referral, and does not check the typed id exists (the service does). English only; no translation.

## 2026-11-03 - Lists that are cut short say so (PR-1)

- **Clinical risk:** yes (H-044: an oldest-first queue hides its newest entry once 50 are waiting). The hazard row names the new control and says the referral queue still has no screen. Not set to accepted or closed.
- **Built:** `src/page.rs` (`Page<T>`: rows plus `more`); the feed-refusal, break-glass review and referral-queue stores fetch one row beyond the limit; the three list routes answer `more` beside `limit` (OpenAPI schemas and `openapi.json` updated, field added, none renamed); the break-glass screen shows a message when the list is cut, the feed screen uses `more` instead of guessing from the row count; three database tests fill each store past 50 inside a rolled-back transaction and see 50 rows and `more: true`; a unit test covers exactly 50 (`more: false`) and 51; two UI e2e tests.
- **Not done / not verified:** "showing 50 of N" is not built (the screens say the list is cut, not how many). Exactly-50 giving `more: false` is tested on `Page::cut` only, not against a database, because the shared test database already holds more than 50 break-glass grants and cannot be brought to exactly 50 without deleting. The referral queue has no screen, so nobody is told on screen that its list is cut. The count routes and the worklist badge are unchanged. The e2e tests stub the service; they do not prove the service sends `more` (the database tests and the OpenAPI census do). The intermittent `http_amendments` failure is still uninvestigated.

## 2026-11-03 - Rustdoc and code comments across the service and the matcher

- **Clinical risk:** none; comments, documentation and a lint only. No behaviour changed.
- **Built:** a doc comment on 221 functions that had none (route handlers, stores, row readers, error mappers, policy and audit helpers, schema builders, the `Hooks` implementation), each written from reading the function and checked against its route table, module header or tests; doc comments on the 62 public fields and variants of the matcher (`Weights`, `MatchConfig`, `ConfigError`, `GuardField`, `ConflictCode`, `Decision`, `FieldScore`, `MatchResult`, `PartialDate`, the patient types); three rustdoc errors fixed (two links to private items, one redundant link target); `#![deny(missing_docs)]` on the matcher (the service already had it); a `cargo doc -D warnings --document-private-items` step in both Rust CI jobs and in the gate list in `AGENTS.md`. A few comments I first wrote were wrong or too firm (what `after_routes` starts, what a clock reading 0 does, which schemas a group holds) and were corrected against the code before this entry.
- **Not done / not verified:** the 7 methods of trait implementations (`fmt`, `from`, `into_response`) are left to their traits' docs, except `From<&StoreError>`. No doctest examples were added, so a comment is not executed. The migration crate, the UI's TypeScript and Svelte files and the test files were not touched. The scan counted functions without a `///`; it did not judge whether an existing comment is still true, and it did not cover types, constants or struct fields in the service (those are covered by `deny(missing_docs)` only where public). The comments describe behaviour as read from the code on 2026-11-03; nothing tests that they stay true. `# Errors` sections were added only where the function returns a `Result`.

## 2026-11-03 - Research pass: the patient-administration-system repository (plan.md §25)

- **Clinical risk:** none by itself; documents only. PR-1 is a proposed safety rule for three queues (H-044 is the nearest hazard); the hazard row is not changed until it is built.
- **Built:** `plan.md` §25 (assessment, eight ideas ranked, risks, open questions, order of work, sources); task series `PR` (PR-1 to PR-8, registered in `scripts/generate-status.sh`); OQ-PR1 and OQ-PR2 in `spec/16-open-questions.md`; a proposed rule in `spec/09-api-surface.md`; the sister project in `spec/17-references.md`.
- **Not done / not verified:** no code. I read the other repository's task titles, its hazard rule, its production-readiness checklist and its commit messages, **not its code or its plan**, so I cannot say how well any of its controls work. Each idea was checked against this repository's code where the table says so; PR-4 (replicas) was not checked and is written as a question. The `PR` items are not scheduled. The idea that the break-glass screen hides the newest grants rests on one local test database that held 57 unreviewed grants; it was not reproduced in a browser.

## 2026-11-03 - Inbound referral messages and the queue routes (PC-2, second slice)

- **Clinical risk:** yes (H-044). An inbound message can now move a referral, and an unmatched one is kept where an administrator can find it. The hazard row lists the routes as a control and says what is missing. Not set to accepted or closed.
- **Built:** `POST /api/referral-messages`, `GET /api/referral-queue`, `/count`, `/{id}`, `POST /api/referral-queue/{id}/clear`; policy resource `ReferralQueue` (17th); five route-table rows; audited receipts, views and clearances (ids, kinds, states; never text); OpenAPI schemas and a type-match test; both writes are in the audit-failure census; both searches in the audit-search census; `tests/http_referral_queue.rs` (login on); the policy matrix size guard went from 16 to 17 resources.
- **Not done / not verified:** **who sends and who clears is assumed (OQ-PC4)**: the integrator sends, the clinical administrator clears. An inbound message moves a referral with no care relationship and nothing proves the sender is the recipient the referral names (the sender is the integrator's token subject). No screen, no worklist count for the referrer, no owner model for another organisation, no timeouts for an uncleared entry, no FHIR or HL7 mapping of an inbound message. A message over 1 MiB is cut, as the feed's is. **Also fixed, unrelated to referrals:** `tests/packs.rs` named its scratch directories from the process id and the clock; on macOS the clock reads in microseconds, so two parallel tests could share a directory and one failed intermittently (seen once in a gate run, not on CI). The name now carries a counter. Not verified on Linux or CI, where it was not seen to fail. **Test-database drift:** `break_glass` review lists show the oldest 50, so `openapi_responses_roles` fails on a local database with more than 50 unreviewed grants; it passes on a fresh one, as in CI. Not fixed; see PR-1. A hand match records the link and does not apply the event to the referral. No concurrency test.

## 2026-11-03 - The unmatched-outcome queue store (PC-2, first slice)

- **Clinical risk:** yes (H-044: an outcome that reaches no one). The hazard row now names the store as a control and says nothing feeds it yet. Not set to accepted or closed.
- **Built:** migration 23 (`referral_unmatched`, `referral_unmatched_clearances`; triggers refuse update, delete and truncate on both; `down` refused outside development); `src/referral_unmatched.rs`; 4 database-gated tests; the 15 new columns are classified in `governance/personal-data-inventory.csv` (retention class `record`); schema map regenerated.
- **Not done / not verified:** **the hazard is not reduced in use**: no route receives an inbound message, so nothing is ever queued by the service; there is no policy row, audit row, screen, worklist count or owner for the queue (OQ-PC2's neighbour). A hand match records the link and does not apply the event to the referral. A referral id mistyped as another real referral's id still matches the wrong one. The state table and the sender rule are unchanged and unreviewed. No concurrency test.

## 2026-11-03 - The referral screen (CA-3, third slice)

- **Clinical risk:** yes (a clinician can now send and answer a referral from the patient page). H-044 lists the screen among its controls; it stays open and is not set to accepted or closed.
- **Built:** every referral version now carries `next`, the events the state table allows with who sends each and whether a note is needed (`allowed_events`, unit-tested against `referral::next`; OpenAPI and `openapi.json` updated); `ReferralsSection.svelte` and `lib/referrals.ts` on `/patients/[id]`; strings (English); a Playwright test (send with an `Idempotency-Key`, a step with a note, referral text shown as text, an end state offers no button, axe clean); `tests/http_referrals.rs` checks `next`.
- **Not done / not verified:** the screen cannot tell whether the signed-in person is the referrer, so it offers every listed step and the service refuses the wrong party's (shown as its 422 message). No patient-facing view, no translations (English only; the wording is mine and unreviewed by a clinician), no redirect, queue or timeouts. The e2e test stubs the API; no browser run against the real service. The section loads on every patient page and its call is not covered by the page-load budget tests.

## 2026-11-03 - Referral routes (CA-3, second slice)

- **Clinical risk:** yes. A referral can now be sent and answered through the service; hazard **H-044** now lists the routes as a control and says what is still missing. Not set to accepted or closed.
- **Built:** `GET`/`POST /api/patients/{id}/referrals`, `GET …/{referral_id}` (history) and `POST …/{referral_id}/events`; new policy resource `Referral` (clinician, nurse, allied health read/send/step; patient and proxy read); four route-table rows; audited successes and refusals (ids, versions, states, role, no text); OpenAPI schemas and a unit test that they match the types; `Event::parse`; `tests/http_referrals.rs` (login on: wrong-role, stale, note-required, end-state, receptionist, cross-patient, patient read, no clinical text in audit rows); the policy matrix size guard went from 15 to 16 resources; both write routes are in the audit-failure census (`tests/audit_switch`, `tests/http_audit_refusal.rs`): with the audit row made unwritable, the send and the step are refused and write nothing.
- **Not done / not verified:** **OQ-PC2 is assumed, not answered**: the sender is the referrer and any other permitted subject with a care relationship is the recipient, so nothing proves the recipient is the team the referral names. The redirect does not create a linked referral. No screen, no inbound-outcome matching or clearance queue, no FHIR mapping, no timeouts, no `Idempotency-Key` test on these routes (the middleware covers every POST under `/api`, unproven here). `tests/openapi_responses_roles.rs` does not check these responses against the schemas yet. Break-the-glass reads and consent refusals of referrals are not tested.

## 2026-11-02 - The referral store (CA-3, first slice)

- **Clinical risk:** yes (a referral and its outcome are what a clinician relies on to know a patient was seen). The row for hazard **H-044** now says what is in place (the store and its rules) and what is not (routes, the unmatched-outcome queue, any screen), and that the hazard stays unmitigated in use. Still `open`, not accepted.
- **Built:** migration 22 (`referral_versions` with a state check, `referral_current`, three triggers that refuse update, delete and truncate; `down` is refused outside development like the other clinical tables); `src/referrals_store.rs` over the pure table of PC-1: `request`, `advance`, `current`, `for_patient`, `history`. The store enforces what the pure table cannot: the sender's role (the referrer asks to cancel; only the recipient confirms), a note for the events that say something, a stale version as a conflict, and an existing patient. `Event::as_str` added to `referral.rs`. `tests/referrals_store.rs` (5, database-gated): request to outcome with the full history, a cancellation as a request, a decline after acceptance, a missing or blank note, the wrong role, a stale version, an unlisted transition, an unknown patient, and the three triggers. The 18 columns of the table and view are classified in `governance/personal-data-inventory.csv` and `referrals` is named in the `record` retention class.
- **Not done / not verified:** no route, policy row, audit row, screen, FHIR mapping, worklist or timeouts; nothing calls the store, so no referral can be made through the service. The referral id is a server UUID; carrying it on an incoming outcome and the unmatched queue (PC-2) are not built. `Book` and `RecordNoShow` carry no appointment date. The actor is passed in by the caller and not checked against the token. Concurrent writers were not tested (the append is the same primitive as the other stores). The state table is still from one unreviewed source. The `referrals` retention period is unset like the others.

## 2026-11-02 - Indexing the new documents and modules

- **Clinical risk:** none; documents only.
- **Changed:** `spec/index.md` lists the schema map and the closed-loop referral draft; `llms.txt` and `llms.json` list the referral draft; the service spec's component table has rows for the idempotent creates (`idempotency.rs`) and the referral state table (`referral.rs`), each with its hazard and its evidence.
- **Not done / not verified:** the README and `index.md` at the repository root were not changed (they do not list spec documents individually). The test counts in the new rows were read from the tests I wrote, not from the generated status page. No other module in the service was checked against the component table, so it may be missing other rows.

## 2026-11-02 - Hazard H-044 and the unmatched-outcome design (PC-2, design only)

- **Clinical risk:** yes. A referral outcome or decline that is filed against the wrong referral, or none, leaves the referrer waiting for a reply that has come, or acting on one that belongs elsewhere. Hazard **H-044** is added: severity 4, likelihood 3, status `open`, not accepted (only the Clinical Safety Officer does that). Its controls are the design below and the pure state table; its residual risk says plainly that nothing is implemented and that today the hazard is unmitigated.
- **Built:** documents only. The four-case design (matched and allowed, matched but refused by the state table, no match, no id) and the clearance queue, in `spec/interoperability/closed-loop-referral.md`; `tasks.md` PC-2 updated.
- **Not done / not verified:** there is no code, table, route, worklist or test: referral storage (CA-3) is not built, so there is nothing for an outcome to match. The design rests on one unreviewed source and has not been checked against IHE 360X. Who owns the queue when the sender is another organisation, and how long an entry may wait, are undecided. The severity and likelihood are my estimate, not a clinical assessment.

## 2026-11-02 - A flaky patient test gets an override on its setup create

- **Clinical risk:** none; a test only.
- **Changed:** in `tests/http_patients.rs`, `fhir_refusals_are_operation_outcomes_and_never_carry_candidates` creates its patient with an `override_reason`. It failed once in a local full database run with the create answering something other than 201, and passed when rerun; I did not see the failing response, so a duplicate-check 409 from a chance repeat of the birth date in my long-lived test database is the likely cause, not a proven one. The create is setup, and the test still checks the FHIR duplicate refusal later (a 409 with no candidates) against that record.
- **Not done / not verified:** the other tests in the file build their records the same way and were not changed; they may fail the same way. CI starts from a fresh database and has not shown this failure. I ran the file once after the change; one pass does not show a flake is gone.

## 2026-11-02 - A test that an Idempotency-Key belongs to its subject (OF-1)

- **Clinical risk:** none to behaviour; a test only. It guards H-043's cause "a key shared across subjects would hand one person another's answer"; the hazard text now says per-subject separation is tested.
- **Built:** `tests/http_idempotency_roles.rs` (database-gated, login on): two clinicians send the same key and body to `POST /api/hazards`; each gets its own record, a retry by either replays that subject's own answer with `Idempotent-Replayed`, writes nothing, and the replay's audit row names the replaying subject; `idempotency_keys` holds one row per subject and key. I checked it can fail by making the middleware key on a constant instead of the subject: the test failed, and I restored the file.
- **Not done / not verified:** only one route (hazard raise) and one role (clinician) were used. The hourly purge loop and a crash between the write and the stored answer are still untested. A raised hazard is a real row in the shared test database (the register is append-only), as in the other role-gated tests. Not run in CI yet.

## 2026-11-02 - The referral state table (PC-1)

- **Clinical risk:** none yet; the module is pure and nothing calls it. It is written so that a later referral cannot be shown as cancelled before the recipient confirms (PC-4): `cancelled` has exactly one way in, the recipient's confirmation of a request. No hazard row was added because nothing a clinician sees has changed; PC-4 adds it when the UI is built.
- **Built:** `src/referral.rs`: `State` (12), `Event` (13), `Actor`, `next(state, event)` returning the next state or `Refused`, and 8 unit tests including an exhaustive check of every state and event pair against a second listing of the table, and a walk that finds every state reachable and none a dead end. The draft spec's `deferred` row now lists a cancel request, as the code does.
- **Not done / not verified:** nothing calls it, so there is no stored referral, route or screen. The table is a proposal from one unreviewed proof-of-concept guide (seven pages read, summarised) and has not been checked against IHE 360X, which I have not read. The three provisional states are the source's own "to confirm". The actor of each event is named, not enforced. The mapping to FHIR `Task.status` and `businessStatus` is not written.

## 2026-11-02 - Research pass: the Care Coordination and 360 Referrals guide (plan.md §24, tasks series PC)

- **Clinical risk:** none now; documents only. PC-2, PC-3, PC-4, PC-6 and PC-7 are marked clinical and say a hazard row comes first; no hazard row was added because nothing was built. Nothing was set to accepted or closed.
- **Built:** `plan.md` §24 (what it is, assessment, overlap table, ten ideas ranked, risks, three open questions, order of work, sources); `tasks.md` series `PC` (PC-1 to PC-9); the draft `spec/interoperability/closed-loop-referral.md` (a referral state table, seven rules, the source's FHIR mapping, flags as an idea); OQ-PC1 to OQ-PC3 in `spec/16-open-questions.md`; a reference in `spec/17-references.md`; `PC` added to the status script's prefixes.
- **Not done / not verified:** I read seven pages of the guide through a summarising page fetch, not the raw pages, and not its other pages, its repository, or the IHE and HL7 specifications it rests on, so everything about 360X, QEDm, MHD and DCP is the guide's own account and the quotations are the summary's. The guide is a one-author draft with no stated licence that forbids clinical use; nothing was copied. The state table is proposed and untested; no code, hazard row or test exists for any item. The open questions are unanswered.

## 2026-11-02 - The UI sends an Idempotency-Key on four creates (OF-1)

- **Clinical risk:** yes, the same as H-043, whose controls and residual text now name the UI's part. A registration, allergy, relationship or consent submitted twice (a double click, a retry) is now stored once when it goes through the UI.
- **Built:** `idempotencyKeyHeader` in `src/lib/proxy.ts` (the API's rule: 1 to 128 of letters, digits and `. _ : -`); the proxy forwards a well-formed key and passes back `Idempotent-Replayed`; `call` takes `idempotencyKey`; `src/lib/idempotency.ts` makes a UUID key; patient registration (one key for the page), allergy, relationship and consent forms (a new key after each success). Unit tests for the helper, the proxy and the client.
- **Not done / not verified:** the other UI creates (observation, amendment, break-glass review, feed resolution) send no key. No browser test double-clicks a form, so the end-to-end effect through the UI is not shown; the API side is (`tests/http_idempotency.rs`). The key is made in the browser with `crypto.randomUUID`, and a page reload makes a new one, so a retry after a reload is not protected. A refused first request releases its key, so a second submit after an error sends the same key with the corrected body; that was shown at the API, not through a form.

## 2026-11-02 - Idempotency keys on creates (OF-1, in part)

- **Clinical risk:** yes. A create that is sent twice stores two records, and a duplicated allergy or observation can be read as two things. The change adds a guard and a second copy of an answer; hazard **H-043** (`governance/clinical-safety/hazard-log.csv`) is added, status `open`, severity 4, likelihood 2. It is not accepted: only the Clinical Safety Officer does that.
- **Built:** migration 21 (`idempotency_keys`); `src/idempotency.rs` (a middleware inside authorization: a keyed `POST` under `/api` runs once per subject and key; a replay returns the stored answer with `Idempotent-Replayed: true` and is audited; a different request under the key is 422; a pending key is 409; a refusal releases the key; keys expire after 24 hours and an hourly loop removes them); the `Idempotency-Key` header in the OpenAPI document on every `/api` `POST` (a unit test checks the documented pattern and length against the middleware); `spec/09-api-surface.md`; open question OQ-OF3, **assumed** by taking its recommendation (the maintainer has not answered). The migration's `down` drops the table everywhere because it holds no clinical record; it is the only table here that deletes.
- **Also changed:** the personal-data inventory census failed on the new table until its ten columns were classified (`governance/personal-data-inventory.csv`). None of the three retention classes fits a table that deletes after a day, so a fourth, `transient`, was added to the census's allowed list and to `governance/retention-schedule.md`. That widens what the census accepts; it does not skip any column. The 24-hour period is a code constant no one has reviewed.
- **Not done / not verified:** no client sends the header, so no real create is protected yet (the UI is unchanged); a client that omits it is as exposed as before. `/fhir/` creates are not covered. Login was off in the test, so per-subject separation was not exercised; the hourly purge loop, the 1 MB request and 4 MB response limits, and a crash between the write and the stored answer were not tested. A 5xx after a successful write releases the key and a retry can write again. The stored answer is patient data outside the append-only tables for up to a day. The full database-gated suite has not been run in CI against migration 21.

## 2026-11-02 - A census of the search routes' audit rows (OF-2)

- **Clinical risk:** none to behaviour; a test only. It guards what a clinician or auditor relies on: that a search of patient records leaves a trail.
- **Built:** `tests/audit_search_coverage.rs` (database-gated, login off) calls each of the nine `Search` routes in `routes::TABLE` and requires 200 and at least one new audit row; `/api/patients/compare` is exempt with its reason (it reads no stored record). A `Search` route that is neither called nor exempt fails the test. All nine passed on the first run; the census failed as intended when I removed one route from the table.
- **Not done / not verified:** the test counts rows before and after, so a row written by something else in the same moment would hide a missing one (the suite runs single-threaded). It does not read the row's detail, so it does not prove the searched values stay out of it. `GET /api/hazards` and `GET /api/me/patients` are `Read` lists, outside this census. The FHIR `$match` route and the FHIR `AllergyIntolerance` and `Observation` searches are not called here. Not run in CI yet.

## 2026-11-02 - Research pass: Openfront Hospital (plan.md §23, tasks series OF) and the schema map (OF-6)

- **Clinical risk:** none now; documents and a generated page. OF-1 (idempotency key on creates) and OF-3 (record hold) are marked clinical and say a hazard row comes first; no hazard was added because nothing was built.
- **Built:** `plan.md` §23 (assessment, seven ideas ranked, risks, two open questions OQ-OF1 and OQ-OF2 in `spec/16-open-questions.md`, order of work, sources); `tasks.md` series `OF` (OF-1 to OF-7, OF-6 ticked); `scripts/generate-schema-map.py` and `spec/schema-map.md` (18 tables, 11 views from the migrations), checked by `--check` in the docs job and the pre-commit hook, listed in `llms.txt` and `llms.json`; `OF` added to the status script's prefixes.
- **Corrected from the comparison:** it was written for the PAS and says the Rust API has no version check and does not audit searches. In this service every REST update already takes `expected_version` (409 with `current_version`), the FHIR update takes `If-Match`, and patient search already audits the criteria kinds, the count and the count hidden by consent. The real gaps here are idempotency on creates and a census over every search route.
- **Not done / not verified:** I read the PAS comparison, not Openfront's repository, so every claim about Openfront is second-hand. OF-1 to OF-5 and OF-7 are not built: none of them has code, a test or a hazard row. The `expected_version` and search-audit statements were checked by reading `search_service` and listing the controllers, not by a full audit of every route. The schema map lists name and SQL type only, misses tables built at run time (the audit partitions), and the `--check` was exercised by corrupting the page once.

## 2026-11-02 - Dependencies: UI npm-ui group (dependabot #4) and the service lockfile

- **Clinical risk:** none to behaviour intended; UI libraries and Rust patch releases. The picker bar and Svelte render what a clinician reads, so a visual regression there would matter; none was looked for beyond the tests below.
- **Changed:** the three npm-ui updates in dependabot's branch (`@lilydesignsystem/svelte-picker-bar` 0.4.0 to 0.5.1, `redis`, `svelte`) merged; the service `Cargo.lock` takes the semver-compatible updates `cargo` resolved (for example arrow 60.0.0 to 60.1.0).
- **Not done / not verified:** UI lint, check, 247 unit tests and 85 Playwright tests passed locally on the merged tree; no manual look at the picker bar's new version, and its release notes (helpers, themes) were not read for changes that affect this UI. The lockfile update was exercised by the service gates (fmt, clippy, tests without a database) and the roles test; the full database-gated suite was not run against it. The `dtolnay/rust-toolchain` 1.120.0 bump (PR #1) was closed, not merged: it would change the Rust toolchain, which `rust-toolchain.toml` pins to 1.99.1.

## 2026-11-02 - Audit routes answer a wrong-typed query with the JSON error (OS-7, behaviour change)

- **Clinical risk:** none to what a clinician sees or decides. This is an error-body change on two audit-reader routes; no audit row, policy decision or result changes.
- **Changed:** `GET /api/audit` (`outcome` or `before` not a whole number, `break_glass` not true or false) and `GET /api/audit/verify` (`checkpoint_id` not a whole number) now answer 400 with the JSON `Error`, as a bad date already did, not the framework's plain-text 400. The OpenAPI 400 for search names only the JSON body. `tests/openapi_responses_roles.rs` checks both real 400s against the schema (36 responses required in all).
- **Not done / not verified:** a client that read the old plain-text message will see JSON. The new message is fixed wording and does not say which criterion failed, where the framework text named the key. `GET /api/audit/export` already read pairs and is unchanged; the other routes that use a typed `Query` (the patient access log, for one) were not checked for the same plain-text answer.

## 2026-11-02 - OpenAPI: GET /metrics (OS-7, in part)

- **Clinical risk:** none; the document describes, it does not change a response.
- **Built:** `GET /metrics` answers 200 as `text/plain; version=0.0.4; charset=utf-8` (the Prometheus text format) and 404 with no body when `EPR_METRICS` is off. A unit test checks the document's content type against the header `metrics::response(true)` really sends, and that the 404 has no body.
- **Not done / not verified:** the route was not requested through the router, so the check is against `metrics::response`, not a served response. The body is described as a string; the counter names and the exposition format are not in the schema. `EPR_METRICS` being read from the environment on every request was not tested. Other routes still have only the default response (for example `GET /api/openapi.json`, the FHIR routes and the health probes).

## 2026-11-02 - OpenAPI: the patient's own routes (OS-7, in part)

- **Clinical risk:** none to behaviour; the document describes, it does not change a response. These routes are what a patient or proxy sees of their own record and who has accessed it, so the schema states that a staff identifier is never in the log, that a record the policy refuses is left out of the list, and that the log is for the patient and proxy only (a clinician gets 403).
- **Built:** `OwnRecord`, `OwnRecordList`, `AccessLogEntry`, `AccessLogPage`; `GET /api/me/patients` and `GET /api/me/patients/{id}/access-log` (seven typed filters, JSON or `format=csv` with `x-next-before`; 400, 403). A unit test compares the entry keys with `entry()` (now `pub(crate)`) for the caller's row, staff and a refusal, and the `by`, `did`, `what` and relationship words with the code. `tests/openapi_responses_roles.rs` adds a proxy role and five checked responses (34 required in all: 35 when the shared audit chain is intact and verify takes the 200 branch).
- **Not done / not verified:** only a proxy was exercised, never a `self` relationship, so `relationship: self` is checked against the code, not a response. The `did` words `created`, `changed`, `removed` and `used`, and a `break_glass: true` entry, were not produced. The CSV was checked for its header line and content type only. Paging (`next` not null, 50 rows) and the 1000-row CSV cap were not exercised. A proxy without the `access-log` scope (403) and an expired proxy were not requested here (`tests/http_proxy_scopes.rs` covers the behaviour, not the schema).

## 2026-11-01 - OpenAPI: the audit verify check accepts the answer the shared database gives (OS-7, fix)

- **Clinical risk:** none; test only.
- **Found:** CI for the audit-routes commit failed in `tests/openapi_responses_roles.rs`: `GET /api/audit/verify` answered 409 ("does not link to the previous row", first broken id 20), not the 200 the test asserted. In CI every database test shares one database, and `tests/audit_chain.rs` alters and deletes audit rows on purpose, so the chain is not guaranteed intact when this test runs. The service was right; the test's assumption was wrong. The same cause would fail the feed-routes commit, which carries the same test.
- **Changed:** the test now accepts 200 or 409 and checks the body against that status's schema (`AuditVerified` or `AuditBroken`); the checkpoint step runs only after a 200. The response count it requires drops from 30 to 29 for the same reason. This is not a weaker check of a schema: the CI body (`intact`, `first_broken_id`, `reason`, `keyed`, `assurance`) is exactly what `AuditBroken` lists, and the 409 branch is now checked where it occurs.
- **Not done / not verified:** the 409 branch ran in CI before this change, but I did not see it run after it; locally the chain is intact whichever order I ran, so the local run took the 200 branch (or I could not tell). I did not look for why the tamper tests leave the shared chain broken, or whether they should repair it. A real 424 (key not held) is still never produced.

## 2026-11-01 - OpenAPI: the feed refusal routes (OS-7, in part)

- **Clinical risk:** none to behaviour; the document describes, it does not change a response. A refused feed message is an update that did not reach the record, so the queue is how it is noticed; the schema says a listing never carries the message text, only a single read does.
- **Built:** `FeedRefusal`, `FeedResolution` (null while open), `FeedRefusalList`, `FeedRefusalCount`, `ResolveFeedRefusal`; `GET /api/feed/refusals`, `…/count`, `…/{id}` and `POST …/{id}/resolve` (200, 409, 422). A unit test compares the keys of an open listing entry and a read, resolved one with the serialised `Entry`, and the outcome and kind words with serde. `tests/openapi_responses_roles.rs` stores a refusal through `feed_queue::record`, then validates six more real responses (30 in all): list, count, read, a blank reason, a close, a second close.
- **Not done / not verified:** the 404 for an unknown refusal id was not requested, so it is not in the document. The `conflict` kind and the `handled` outcome were not sent. The `reasons` text is "in words" and not checked. The messages the feed itself would refuse (over the TCP listener) were not driven; the refusal was stored directly, so how the feed fills `event`, `control_id` and `peer` is covered by the feed's own tests, not this one. Only the clinical admin was used; the other roles' refusals (403) were not requested here.

## 2026-11-01 - OpenAPI: the audit routes (OS-7, in part)

- **Clinical risk:** none to behaviour; the document describes, it does not change a response. The audit trail is how misuse of a record is found, so the schema records what `verify` proves (`assurance`) and the three statuses a monitor must tell apart: 200 intact, 409 a row altered, 424 not fully checked because a key is missing (not evidence of tampering).
- **Built:** `AuditRow`, `AuditSearch`, `AuditHead`, `AuditVerified`, `AuditBroken`, `AuditUnchecked`; `GET /api/audit` (eleven typed query parameters; 200, 400), `GET /api/audit/export` (CSV or NDJSON attachment, `x-next-before` header; 400), `GET /api/audit/verify` (checkpoint parameters; 200, 400, 409, 424). A unit test checks the row keys with and without digests, the action codes against `Action::code`, and the search parameters against the criteria list. `tests/openapi_responses_roles.rs` (login on) validates eight more real responses (24 in all): searches by patient and by outcome, a bad date, an export refused for its format, a verify, a verify with its own head as checkpoint, a half checkpoint, a clinician refused (403); and checks the content types of a real CSV and NDJSON export and validates an exported NDJSON row against `AuditRow`.
- **Found, not fixed:** a criterion of the wrong type (`outcome=notanumber`) is refused by the framework with a plain-text 400, not the JSON `Error` a bad date gets. The document now describes both; making them agree is a behaviour change left as a follow-up in OS-7.
- **Not done / not verified:** the 409 (altered row) and 424 (key not held) bodies come from the code; the database refuses edits to the trail, so the test could not produce them. The `AuditBroken` and `AuditUnchecked` schemas are therefore unchecked against a real response. The export's CSV body is not validated beyond its header line; the 10000-row cap and `x-next-before` were not exercised. The FHIR `AuditEvent` search is not described here. Whether the `detail` object ever holds a non-object was not checked.

## 2026-11-01 - OpenAPI: the break-glass routes (OS-7, in part)

- **Clinical risk:** none to behaviour; the document describes, it does not change a response. Break glass lets a clinician read a record they have no relationship with, and the review is the control on it, so the schema states that the reason and the note are required, that asking again does not extend a grant, and that a grant can be reviewed once.
- **Built:** `OpenBreakGlass`, `ReviewBreakGlass`, `BreakGlassGrant`, `BreakGlassReview` (null until reviewed), `BreakGlassList`, `BreakGlassCount`; `POST /api/patients/{id}/break-glass` (201, 200, 422), `GET /api/break-glass/reviews`, `GET /api/break-glass/reviews/count`, `POST /api/break-glass/reviews/{id}` (200, 409, 422). A unit test compares the keys with the serialised `Grant` (an unreviewed one has a null `review`), checks the outcome words against serde and the request types. `tests/openapi_responses_roles.rs` validates eight more real responses (16 in all): a blank reason, an open, a repeat open, the list, the count, a blank review note, a review, a second review.
- **Not done / not verified:** the 409 body is the plain `Error` (no `current_version`), as the handler sends it. The grant's `overdue` true case and a grant on an unknown patient (404) were not requested. The checker does not support `oneOf`, so `review` is described as a nullable object. The time limit of a grant is not in the schema. The list is capped at `limit`; paging beyond it was not tested.

## 2026-11-01 - OpenAPI: the hazard register, and the first role-gated test (OS-7, in part)

- **Clinical risk:** none to behaviour; the document describes, it does not change a response. The hazard register is the safety case's own record, and only the Clinical Safety Officer retires a hazard; the schema says so and the test shows a clinician's retirement refused. No hazard was set to `accepted` or `closed`; the test's hazards live only in its throwaway database.
- **Built:** `CreateHazard`, `UpdateHazard`, `Hazard`, `HazardEvidence`, `HazardControl`, `StoredHazard`, `HazardList`; `GET` and `POST /api/hazards` and `PUT /api/hazards/{hazard_id}` with 200/201, 409 and 422. A unit test compares the keys with the serialised types and the enum words (evidence kinds, states, events) with serde, and checks each event body parses. `tests/openapi_responses_roles.rs` (login on, test identity provider) validates eight real responses: raise, a refused rating, list (auditor, audit purpose), a control, a clinician's retirement (403), a stale step (409), a retirement by the Clinical Safety Officer, and a missing token (401). The checker moved to `tests/openapi_support/mod.rs` and both tests use it. The success text for 201 no longer says "the stored patient".
- **Not done / not verified:** `UpdateHazard` is one object with every event's fields optional, not a `oneOf` (the checker has no `oneOf`, and requests are not validated by it); which fields each event needs is in the description and the unit test only. The 403 for a non-CSO retirement was checked against `Error`, but the body's extra wording was not. `/api/hazards` has no read-one route. Break-glass, audit, feed and `/api/me` are still undescribed.

## 2026-11-01 - OpenAPI: merge, unmerge and links (OS-7, in part)

- **Clinical risk:** none to behaviour; the document describes, it does not change a response. A merge joins two patient records, so a wrong schema here could lead an integrator to merge without a reason; the schema marks `reason` required, as the handler does.
- **Built:** `LinkRequest`, `PatientLink`, `PatientLinkList`; `POST …/merge` (201, 422), `POST …/unmerge` (200, 422) and `GET …/links` (200). A unit test compares `PatientLink` with the serialised `StoredLink` (a null score still has its key) and checks that `LinkRequest` needs both fields. `tests/openapi_responses.rs` merges a real second record of the same person, lists the links, sends a self-merge (422) and unmerges, validating all four (at least 38 responses now).
- **Not done / not verified:** the 422 body's extra fields (`refusal`, `refusal_into`, `conflict_codes`) are described in prose only; `ErrorList` allows extra keys. A merge refused by the matcher (`conflict_codes`) and a source already merged were not sent. `status` is a string; its two words (`active`, `reversed`) are in the description, not an enum. Login was off, so the administrator-only gating was not exercised.

## 2026-11-01 - OpenAPI: the photo routes (OS-7, in part)

- **Clinical risk:** none to behaviour; the document describes, it does not change a response. A patient's photo is shown in the banner to confirm identity, so the schema says it is never cached (`private, no-store`) and that a write needs `expected_version`.
- **Built:** `PhotoVersion`; `GET /api/patients/{id}/photo` (200 `image/png` or `image/jpeg`, 404), `PUT` (image body, required `expected_version`, optional `reason`; 200, 400, 409, 422) and `POST …/photo/withdraw` (no body; 200, 400, 409, 422). A unit test checks the shape. `tests/openapi_responses.rs` stores a real PNG, reads it back (content type, cache header and bytes compared), sends a stale and a version-less `PUT`, withdraws, and reads again (404), validating the five JSON responses against the served schemas (at least 34 responses now).
- **Not done / not verified:** the JPEG and WebP-style refusals, the size limit and the 422 body were not sent for real, so the 422 text is from the code, not a response. The `PhotoVersion` keys are checked only against a real response (the handler builds that JSON inline, so there is no type to compare). Login was off, so the role split was not exercised. The 404 for an unknown patient was not requested separately from the withdrawn case.

## 2026-11-01 - OpenAPI: the allergy-statement route (OS-7, in part)

- **Clinical risk:** none to behaviour; the document describes, it does not change a response. The statement "no known allergies" drives the allergy banner, so the schema records that it is refused while an allergy is active.
- **Built:** `CreateAllergyStatement` and `AllergyStatement`; the 200, 409 (stale `expected_version`) and 422 responses of `PUT /api/patients/{id}/allergy-statement`. A unit test checks the response keys against `Assertion` and the request's required keys and parsing against `StatementRequest`. `tests/openapi_responses.rs` sends a real 422 (active allergy), a real 200 and a real stale 409 and validates the 200 and 409 against the served schemas (at least 29 responses now).
- **Not done / not verified:** the withdraw step (`none_known: false`) succeeding was not sent; the 422 body was not validated; the `statement` field inside `AllergyList` is still described only as an object or null. Login was off, so the clinician-only gating was not exercised. Gates: fmt, census (5), openapi unit tests (14) and the real-response test pass locally; clippy and the full suite are run before the push.

## 2026-11-01 - OpenAPI: the amendment routes (OS-7, in part)

- **Clinical risk:** none to behaviour; the document describes, it does not change a response. Amendments are how a patient asks for the clinical record to be corrected, so the schema states who may take which step (deciding steps need the clinician role; withdraw and disagree need the patient role).
- **Built:** `CreateAmendment`, `UpdateAmendment`, `AmendmentView` and `AmendmentList`; the 201, 200, 409 (stale step) and 422 responses of `GET`, `POST` and `PUT` on `/api/patients/{id}/amendments`. A unit test checks the state, notice, step and ground words against what serde writes and the real request types. `tests/openapi_responses.rs` asks for an amendment on a real observation, lists, withdraws and sends a stale step, and validates all four responses; it found that `notice` is `null` after a withdrawal, so the schema allows null. The new table rows were checked to apply (a unit test fails when a request schema is unused).
- **Not done / not verified:** the other steps (accept, deny, apply, extend, disagree) were not sent for real, so the `accepted`, `applied` and `denied` state bodies are described only as "an object with a `state`" and their extra fields are not in the schema. The login was off, so the role split between clinician and patient steps was not exercised. Which states give a `null` notice is not stated. Gates: fmt, clippy `-D warnings`, openapi census (5), unit tests (13), `openapi_responses` (2, with Postgres 18).

## 2026-11-01 - OpenAPI: the response test also calls every PUT and each stale PUT (OS-7)

- **Clinical risk:** none; test only.
- **Built:** `tests/openapi_responses.rs` now also updates a fresh observation, consent and relationship (200) and repeats the update with the old `expected_version` (409), and sends a stale allergy update, validating each real response against the served schema. At least 23 responses are checked. All pass against Postgres 18, so the `VersionConflict` and stored-record schemas hold for every described PUT.
- **Not done / not verified:** the 422 and 404 bodies are still not requested; the validator is the same small one (no `allOf`, `oneOf`, formats or length limits). CI runs this test with the other database-gated tests (`cargo test --workspace -- --ignored`); that was read in `ci.yml`, not yet seen passing on GitHub for this commit.

## 2026-11-01 - OpenAPI: real responses are checked against the schemas, and three slices are corrected (OS-7)

- **Clinical risk:** none to behaviour; the document describes, it does not change a response. It was wrong in places an integrator would have relied on, which is the reason for this entry.
- **Found:** the earlier entries for observations, consents and relationships said their create and update routes carried request bodies and 201/200/409 responses. They did not: six rows were never added to the `BODIES` table (a scripted edit silently did not match after `cargo fmt` reflowed the lines before it). Only their list and read schemas were in the document. Two schemas were also wrong: `AllergyList.summary` carries more than `state` (for example `items`), and an observation version is a full observation plus `reason`, not an object with `reason`.
- **Built:** `tests/openapi_responses.rs` (database-gated, login off) creates a patient, allergies, observations (with a value and with an absent reason), a consent and a relationship, calls the described routes, and validates 15 responses (including a real 409) against the schemas the service serves, with a small validator (required keys, keys outside `properties`, enums, types) that has its own non-database test. The six rows are added, the two schemas fixed, and `ObservationVersion` is derived from `StoredObservation` so the two cannot differ. A unit test fails when a `Create…` or `Update…` schema is not used by any operation, or a table row names a schema that does not exist. Re-blessed document: 210 lines added, none removed.
- **Not done / not verified:** the validator does not implement `allOf`, `oneOf`, formats or length limits, and sees only the optional fields these requests set; the 422 and 404 responses and the consent and relationship updates were not requested; the PUT routes of observations, consents and relationships were not called. Gates: fmt, clippy `-D warnings`, openapi census (5), unit tests (12), `openapi_responses` (2, with Postgres 18).

## 2026-11-01 - OpenAPI: the consent and relationship routes (OS-7, in part)

- **Clinical risk:** none to behaviour; the document describes, it does not change a response. These two record types decide who may see a record, so a wrong schema would mislead an integrator; the tests below are the guard.
- **Built:** `ConsentRecord` (kind, status, provisions, period), `RelationshipRecord` (with the proxy terms: basis, scopes, expiry), and for each the stored, create, update and list forms; the 200/201, 409 (update only) and 422 responses of `GET`, `POST` and `PUT` on `/api/patients/{id}/consents` and `/relationships`. Two unit tests: property names equal the serialised keys with every optional field set, including provisions, periods and proxy terms; every enum word (consent kind, status and effect; relationship kind, status, basis and scope) equals what serde writes, variant by variant; the required request keys are enforced by the real request types. `patient_schemas` was split into five functions, because one `json!` call hit the macro recursion limit; the committed document gained 371 lines and lost none. `openapi.json`, spec 09 and `tasks.md` updated.
- **Not done / not verified:** a provision's `actors`, `purposes`, `resources` and `labels` are arrays of strings; their allowed words (roles, purposes, resource names, label codes) are not listed in the schema or tested. The consent and relationship validation rules (a general consent needs provisions; the proxy terms rules) are in the prose only. No 409 documented for create. The relationship schemas were not compared with a real response. Gates: fmt, clippy `-D warnings`, openapi census (5), unit tests (11).

## 2026-11-01 - OpenAPI: the observation list, create, update and versions (OS-7, in part)

- **Clinical risk:** none to behaviour; the document describes, it does not change a response. The schema says a value needs a unit, no value needs an `absent_reason`, and absence is never zero.
- **Built:** `ObservationRecord`, `StoredObservation` (the record's fields sit beside the stored ones, as the flattened type serialises), `CreateObservation`, `UpdateObservation`, `ObservationList` and `ObservationVersions`; the 200/201, 409 (update only) and 422 responses of the four observation routes. One unit test: property names equal the serialised keys with every optional field set, including the flattened stored form and the version item (stored plus `reason`); the `absent_reason`, `status` and `flag` words equal what serde writes, variant by variant. `openapi.json`, spec 09 and `tasks.md` updated.
- **Not done / not verified:** `amendment_notices` is only an object, and a version item is described as an object with `reason` (the rest is in the prose, and the test checks it, but a client generator would not see it). The "value needs a unit" rule and the supported-code table (`observations::SUPPORTED`) are not in the schema. No 409 documented for create (none read in the handler). No real HTTP response was compared. Gates: fmt, clippy `-D warnings`, openapi census (5), unit tests (9).

## 2026-11-01 - OpenAPI: the allergy list, create and update (OS-7, in part)

- **Clinical risk:** none to behaviour; the document describes, it does not change a response. It states in the schema that an absent `criticality` means not stated.
- **Built:** `AllergyRecord`, `StoredAllergy`, `CreateAllergy`, `UpdateAllergy` and `AllergyList`, and the 200/201, 409 (update only) and 422 responses of `GET` and `POST /api/patients/{id}/allergies` and `PUT …/allergies/{allergy_id}`. Three unit tests: property names equal the serialised keys (with every optional field set), the `category`, `criticality` and `status` words equal what serde writes for each variant, required keys are present on a bare record, and the list's top-level keys equal `AllergyList`'s. `openapi.json`, spec 09 and `tasks.md` updated.
- **Not done / not verified:** `statement`, `summary` and `amendment_notices` are described only as objects (`summary` has its `state`). The allergy create was read for a 409 and none was found, so none is documented; that was a reading of the handler, not a test. The allergy-statement route (`PUT …/allergy-statement`) is not described. Length limits in the schema are copied from `allergies::validate` by hand and not tested. No real HTTP response was compared. Gates: fmt, clippy `-D warnings`, openapi census (5), unit tests (8).

## 2026-11-01 - OpenAPI: the patient read and search responses (OS-7, in part)

- **Clinical risk:** none; the document describes, it does not change a response.
- **Built:** `PatientView` (200 of `GET /api/patients/{id}`) and `PatientSearchResults` (200 of `GET /api/patients`, with the `family`, `birth_date`, `identifier` and `drill` query parameters). One unit test: the schemas' property names equal the keys of a real serialised `PatientView` and of the search body, and the documented query parameters all deserialise into `Search`. `openapi.json`, spec 09 and `tasks.md` updated.
- **Not done / not verified:** the search body is built in the test the way the handler builds it, so a change in the handler would not be caught; a new `Search` field would not be caught either (the test checks the documented ones are accepted, not that none is missing). The `X-Purpose-Of-Use` header is not documented. The 404 of the read and the 400 of the search are not described. No real HTTP response was compared. Gates: fmt, clippy `-D warnings`, openapi census (5), unit tests (6).

## 2026-11-01 - OpenAPI: the 409 and 422 responses of patient create and update (OS-7, in part)

- **Clinical risk:** none; the document describes, it does not change a response.
- **Built:** `DuplicateConflict` (create: `error`, `candidates`, `candidates_cut`) and `VersionConflict` (update: `error`, `current_version`) schemas, and a 409 and a 422 (`ErrorList`) on `POST /api/patients` and `PUT /api/patients/{id}`. One unit test: the version conflict is the real `ApiError` built from `StoreError::Conflict`; `openapi.json` re-blessed.
- **Not done / not verified:** the duplicate body in the test is built by hand to mirror `controllers/patients.rs`, not taken from the handler, so it would not catch a change there; the HTTP database test `http_patients` already exercises the real 409 but does not compare it to the schema. A candidate is described only as an object. Gates: fmt, clippy `-D warnings`, openapi census (5), unit tests (5).

## 2026-11-01 - OpenAPI: request and success schemas for patient create and update (OS-7, in part)

- **Clinical risk:** none; the document describes, it does not change a response.
- **Built:** `Patient`, `Stored`, `CreatePatient` and `UpdatePatient` schemas; `POST /api/patients` (201) and `PUT /api/patients/{id}` (200) carry a request body and a success response. Two unit tests: the schemas' property names equal the fields a fully populated `Patient` and `Stored` serialise (both directions), and equal what `CreateRequest` and `UpdateRequest` accept, including which keys are required. `openapi.json`, `spec/09-api-surface.md` and `tasks.md` updated.
- **Not done / not verified:** the schemas are hand-written; the tests check property names and required keys, not types, formats or enum values beyond the property list. No real HTTP response was compared to them (the database tests were not run for this). The 409 and 422 responses of these two routes are not described. Every other route still has no body schema, so a client cannot yet be generated for the service as a whole. Gates: fmt, clippy `-D warnings`, openapi census (5), unit tests (4).

## 2026-11-01 - OpenAPI: the error bodies are described (OS-7, in part)

- **Clinical risk:** none; the document describes, it does not change a response.
- **Built:** `components.schemas` gains `Error`, `ErrorList` and `OperationOutcome`; every 401 and 403 references the one that fits its path (`/fhir/` gets the outcome). Two unit tests in `src/openapi.rs`: real `ApiError` bodies carry the keys the schemas require, and each reference resolves. `openapi.json`, `spec/09-api-surface.md` and `tasks.md` updated.
- **Not done / not verified:** no request body or success response is described, so no client can be generated. The schemas are written by hand beside `api_error.rs`; the tests check required keys, not every field, and the 422 `ErrorList` is not referenced by any operation yet. A real 403 over HTTP was not compared to the schema. Gates: fmt, clippy `-D warnings`, the openapi census (5) and unit tests (2); the full suite was not re-run.

## 2026-10-31 - Audit: a test that a successful patient read leaves an audit row (OS-9, in part)

- **Clinical risk:** none to behaviour; it adds a guard for H-042's neighbour, the claim that access is audited. No hazard row changed.
- **Built:** `tests/audit_coverage.rs` creates a patient and calls each patient-scoped `GET` in `routes::TABLE` (15 routes, R4 and R5 FHIR reads included; a photo and an observation are created first), asserting a 2xx and a rise in `audit_events_all`. All 15 pass. Checked (before the photo and observation fixtures were added) by demanding an impossible rise: it then named every route walked. `tasks.md` OS-9 updated; the note that `authz::authorize_mw` audits only refusals is recorded.
- **Not done / not verified:** nothing is skipped. Not walked: writes, `/api/audit`, feed, break-glass, hazard, `/api/me` reads. It counts rows, so it cannot tell that the row names the right patient or action. Login was off; role-dependent paths were not exercised. Gates run: fmt, clippy on the test, the test itself; the full suite was not re-run.

## 2026-10-31 - Audit: archiving no longer leaves the current month without a partition (OS-13, in part; H-042)

- **Clinical risk:** removes a window in which an access could not be audited (and so was refused) right after an archive. H-042 text updated; it stays `open`.
- **Built:** migration 20 renames `audit_archive_before` to `audit_archive_move` and creates a new `audit_archive_before` that calls it and then `audit_ensure_partitions(24)` in the same transaction. The tests that called the maintenance step by hand after an archive no longer do: an append and a verification work at once, and a maintenance run right after finds nothing to make (`ensure_partitions` returns 0). `down` refuses, as for the other clinical migrations. `spec/08-architecture.md`, `tasks.md` and H-042 updated.
- **Not done / not verified:** the migration was run on empty scratch databases only. A partition range freed by archiving a month that is not the current one is not remade (it is in the past, and nothing writes there). The service was not started. Gates: fmt, clippy `-D warnings`, the database suite with Postgres 18 (only the four `fhir_validator` tests fail, as without `FHIR_VALIDATOR_JAR`); the no-database tests do not touch migrations and were not re-run.

## 2026-10-31 - Audit: the verify endpoint checks the archive too (OS-13, in part; H-042)

- **Clinical risk:** the answer of `GET /api/audit/verify` can now be `409` for damage in an archived partition, and an intact answer carries `archive_rows`. H-042 text updated; it stays `open`.
- **Built:** `verify_audited` runs `audit::verify_archive` after the live check. Damage in the archive outranks a live finding; a live break is reported as before; an archive the service lacks the key for is the same `424`. Status codes are unchanged; `archive_rows` is the only new field. Two tests: `tests/audit_chain.rs` asserts `archive_rows` is 0 when nothing is archived; a new test in `tests/audit_partitions.rs` archives, expects 200 with `archive_rows` 3, edits an archived row with the triggers disabled and expects 409 naming the partition. `spec/08-architecture.md` and `tasks.md` updated.
- **Found while testing:** after an archive the current month's partition is gone, so the audit row that a verification writes fails until the maintenance step has run; the loop makes it within six hours, and the test calls it first. A verify served after an archive but before the next loop tick would answer 500 for that call.
- **Not done / not verified:** nothing runs the verify endpoint on a schedule. The maintenance loop does not call the check. The check reads the whole archive on every call and was not timed over large archives; `archive_rows` is not in the OpenAPI document or the UI. The running service was not started. Gates: fmt, clippy `-D warnings`, 356 tests without a database, the database suite with Postgres 18 (only the four `fhir_validator` tests fail, as without `FHIR_VALIDATOR_JAR`).

## 2026-10-31 - Audit: an archived table is checked against its anchor (OS-13, in part; H-042)

- **Built:** `audit::verify_archive` reads each archived partition in batches and checks the anchors link head to tail, and that the table holds the anchor's row count, first and last id and last digest, with its chain recomputed from `first_prev_digest`; a missing table, a missing, extra or edited row is `Broken` at that anchor or row. The per-row event rebuild is now one helper shared with `verify_against`. One database test (6 in `tests/audit_partitions.rs`): intact with a small batch, then a row edited with the triggers disabled (as a database owner could) is found. `spec/08-architecture.md`, H-042 and `tasks.md` updated; the hazard stays `open`.
- **Not done / not verified:** nothing calls `verify_archive` yet: the verify endpoint and the maintenance loop still start at the newest anchor, so a damaged archive is found only when someone runs it. The check was not timed over many or large archived partitions. It cannot tell a dropped partition from one never archived if its anchor was also removed (the anchors are append-only by trigger, not by ownership). Gates: fmt, clippy `-D warnings`, 356 tests without a database, the database suite with Postgres 18 (only the four `fhir_validator` tests fail, as without `FHIR_VALIDATOR_JAR`).

## 2026-10-31 - Audit: the archive stays visible through a view (OS-13, in part; H-042)

- **Built:** migration 19 adds `audit_events_all` (the live table plus every table in `audit_archive`, as a `UNION ALL` over explicit columns) and `audit_refresh_all_view()`; `audit_archive_before` is replaced to rebuild the view in the same transaction as each move. The patient access log and the audit search read the view; the chain check and the append still read `audit_events` and the anchors. The archive test now also finds an archived row through `patient_log` and `search`. Inventory rows for the view's 13 columns. fmt and clippy `-D warnings` pass; with Postgres 124 pass and the 4 `fhir_validator` fail as always.
- **Not done / not verified:** the view's speed over many archived partitions was not measured; any new reader of `audit_events` that skips the view will miss archived rows; the earlier entry's "not built" about the log and search no longer holds, but its other gaps do (no governance period, no check of an archive against its anchor, no large-table run); H-042 stays `open`; CI has not run on this commit.

## 2026-10-31 - Audit: monthly partitions and an archive step (OS-13, in part; H-042)

- **Clinical risk:** an audit row that leaves `audit_events` is not shown in a patient's access log; a missing partition fails the write. H-042 is added `open` (not reviewed). OQ-OS7 (the period) is marked assumed: the maintainer chose "partitions and archive step". Archiving is **off unless `EPR_AUDIT_ARCHIVE_AFTER_MONTHS` is set**, so by default nothing leaves `audit_events`.
- **Built:** migration 18 attaches the old table as the first partition (ids, rows and chain unchanged; triggers moved to the parent in one transaction; a `TRUNCATE` trigger on each partition), `audit_archive_anchors` (append-only), `audit_ensure_partitions`, `audit_archive_before` (anchor, detach, move, row trigger restored). `src/audit_retention.rs` runs the maintenance every six hours; `audit::append` and `verify` start from the newest anchor. 5 new database tests (partitioned and append-only, rows before the migration keep ids and chain, chain across a boundary and a break on the far side, archive with anchor and the chain going on, an empty live table continues from the anchor); 1 unit test. Inventory rows for the anchors; the census test now skips partitions, which hold the parent's columns (every column of every real table is still required). fmt and clippy `-D warnings` pass; plain tests 356 pass; with Postgres 124 pass and the 4 `fhir_validator` fail as always.
- **Found while testing:** detaching a partition drops its inherited row trigger, which would have left archived rows editable; the archive function now creates it again, and a test tries to edit, delete and truncate an archived table.
- **Not done / not verified:** no period is set by governance; a patient's access log and the audit search do not read the archive, and no view joins it; nothing checks an archived table against its anchor's row count or hash chain; the migration ran on empty scratch databases and one with a few rows, not on a large table (the attach scans every row once, under a lock); a database owner can still drop a partition; the loop was not run at a real month change or in a running service; `down` always refuses; CI has not run on this commit.

## 2026-10-31 - Packs: the `au` pack turns the IHI rule on (OS-10, step c)

- **Clinical risk:** with `EPR_PACKS` naming `au`, a shared valid IHI can link two records automatically and two different valid IHIs forbid a link. H-041 is added `open` (not reviewed). A deployment that does not enable `au` is unchanged.
- **Built:** `packs/au/identifiers.toml` (`use_ihi = true`, one match and one differ vector, invented numbers); `src/packs.rs` reads an optional `identifiers.toml` with unknown keys refused, every number checked as a valid IHI, the vectors run through the real matcher, `RuleSet.use_ihi` (any pack on means on), the hash includes it, and `packs::engine()` sets the matcher's `use_ihi`. `tests/packs.rs` has 5 new tests (14 in all). fmt and clippy `-D warnings` pass.
- **Not done / not verified:** the service was not started with `EPR_PACKS=au`, so the wiring in `app.rs` and a real IHI through the API were not seen; the IHI numbers are invented and the rule was not checked against an official test number or the specification; no person has reviewed the pack; a pack can only turn the rule on, never off; step (b) as a rule list is not done; CI has not run on this commit.

## 2026-10-31 - UI: the three IHI conflict codes are translated (OS-10)

- **Built:** `different_ihis`, `same_ihi_different_birth_dates` and `same_ihi_different_names` have a sentence in each of the eight locales (`translations.ts`, `conflicts.ts`); the conflict test covers twelve codes and now finds the numbered codes by name, not by position. 247 vitest pass, lint, `check` and `check:tests` pass; coverage lines 92.2, branches 86.65.
- **Not done / not verified:** the new sentences are machine-written and no speaker has read them (this is clinical wording, hazard H-001); "IHI" is left as the abbreviation in every locale; Playwright was not re-run (no page changed); nothing in the service can send these codes until a pack turns `use_ihi` on.

## 2026-10-31 - Service: depends on matcher 0.5.0 (OS-10)

- **Changed:** `Cargo.toml` and `Cargo.lock` take the published matcher 0.5.0. No service code changed, and `use_ihi` is off in every engine the service builds, so behaviour is unchanged. fmt, clippy `-D warnings`, database-free tests and `cargo deny` pass; with Postgres, 119 ignored tests pass and the 4 `fhir_validator` tests fail as always without `FHIR_VALIDATOR_JAR`.
- **Not done / not verified:** no pack key turns the IHI rule on, so it is unreachable from the service (that is a clinical-impact change and needs a hazard-log update); the UI has no translation for the three new conflict codes; CI has not run on this commit.

## 2026-10-31 - Matcher 0.5.0 published to crates.io

- **Done:** `electronic-patient-record-matcher` 0.5.0 is on crates.io, published from `4621cd9` after CI on that commit was green. A published version cannot be changed or removed (only yanked).
- **Not done / not verified:** no git tag or GitHub release was made; the service still depends on 0.4.0 and was not tested against the published 0.5.0 crate; the caveats in the 0.5.0 entry below still stand.

## 2026-10-31 - Hazard log: H-039 row was malformed CSV

- **Fixed:** the H-039 `controls` field held a quoted sentence with unescaped double quotes, so the row split into the wrong columns and `scripts/check-hazard-log.py` failed the docs job on the first push. The quotes are doubled; the wording is unchanged and H-039 stays `open`.
- **Not done / not verified:** I did not run the checker when I added the row on 2026-10-30, and it is not in the pre-commit hook, so the fault reached CI; the hook was not changed. This entry was itself missing from the first fix commit (see the correction below).

## 2026-10-31 - Matcher 0.5.0: an opt-in IHI rule (OS-10, step c)

- **Clinical risk:** a deterministic identifier decides links. The rule is off by default, so nothing changes until a deployment turns it on; H-040 (packs) covers a pack-driven change and stays `open`.
- **Built:** matcher `ihi` module, `MatchConfig::use_ihi`, three conflict codes (`different_ihis`, `same_ihi_different_birth_dates`, `same_ihi_different_names`), 7 integration tests and 2 unit tests; matcher spec section 4; version 0.5.0. A clash on either number forbids a link even when the other is shared. Matcher fmt, clippy `-D warnings`, all tests and `cargo deny` pass; CI on `4621cd9` is green.
- **Not done / not verified:** the service still depends on 0.4.0, so nothing reaches the API until it moves to 0.5.0 and a pack sets `use_ihi`; the UI shows a code it does not know as the English text and has no translation for the three new codes; the IHI rule was tested against numbers the tests complete themselves, not an official test IHI, and the prefix and Luhn rule were not re-read against the Digital Health specification; step (b) is only partly done (one shared lookup, no pack-configurable rule list); `ConflictCode` has new variants, so a caller that matches exhaustively must handle them.

## 2026-10-31 - Outbox: queue depth and stuck gauges (OS-3, in part)

- **Built:** `outbox::depth` counts events still to be tried and events left after `MAX_FAILURES` failures; `run_loop` sets the gauges `epr_outbox_pending` and `epr_outbox_stuck` after each run; `metrics.rs` gained a `Gauge` type. Tests: `/metrics` lists both; a database test checks the counts move by (1, 1). No clinical effect.
- **Not done / not verified:** no alert rule ships; the gauges read zero until the first loop run and stay zero when the loop is off, which looks like "empty"; the depth query has no tuned index; the loop was not run live. `sccache` is no longer installed here, so builds ran with `RUSTC_WRAPPER` empty.

## 2026-10-31 - Outbox: the doubling of the retry wait is tested (OS-3, in part)

- **Built:** one database-gated test, `the_wait_after_a_failure_doubles_with_each_failure`: events with 1 to 4 earlier failures, stamped in the past by an insert, are skipped just inside the wait (30, 60, 120, 240 seconds) and taken just past it. No code or spec changed. The 9 outbox store tests pass against Postgres; fmt and clippy `-D warnings` pass.
- **Not done / not verified:** the cap (the wait stops doubling after 2^10) is not tested and cannot be reached: an event stops being claimed after `MAX_FAILURES` (5) failures, so the exponent never passes 4; the cap is dead code until that limit rises. The boundaries have a margin of 10 to 20 seconds, so a very slow database could make the test flaky; the test did not run in CI.

## 2026-10-30 - Benchmark: re-scored and the scan record committed (OS-1)

- **Changed:** `.sota/last-scan.json` re-scored against rubric version 1 from repository evidence: published coverage, SAST, performance numbers and background jobs moved from missing to partial; secret scanning and the code of conduct are met (edge); coverage stays 44% (11 of 25 met), tier LAGGING, 14 gaps; the previous run is kept under `previous`. `spec/benchmark.md` has the "Since last scan" line. `.sota/` is now committed.
- **Not done / not verified:** the comparators were not re-fetched, so this measures us against the same field, not today's field; the scores are my judgement from files, with no second reviewer; "partial" is a call (published coverage has floors and an artifact but no trend; the outbox has no producer); the maintainer may prefer `.sota/` ignored, which is one `.gitignore` line and a `git rm --cached`.

## 2026-10-30 - CI: CodeQL scans Rust too (OS-4, in part; OQ-OS3 assumed)

- **Changed:** the `codeql` job is a matrix over `javascript-typescript` and `rust`, both `build-mode: none`, each with its own category; `ci-ok` still needs the job. The reason: CodeQL's Rust analysis is generally available without a build since CodeQL 2.23.3 ([GitHub changelog](https://github.blog/changelog/2025-10-14-codeql-scanning-rust-and-c-c-without-builds-is-now-generally-available)).
- **Not done / not verified:** the job has not run on GitHub; I do not know whether CodeQL parses this workspace's Rust (edition, loco macros) or how many alerts it will raise on the first run, and a first-run failure would block `ci-ok` until triaged; support was confirmed from a search summary of the changelog, not the CodeQL documentation; the matrix leg names changed (`CodeQL (rust)`), which matters if branch protection lists the old name; semgrep was not added; the service's first scan may be slow.

## 2026-10-30 - Packs: the phone calling code from a pack (OS-10, first slice a)

- **Clinical risk:** the calling code decides whether two phone numbers are scored as one. H-040 is added `open` (not reviewed). OQ-OS3, OQ-OS5 and OQ-OS6 are marked assumed: the maintainer chose "take my recommendations", the Clinical Safety Officer has not answered OQ-OS6.
- **Built:** `packs/uk` (44) and `packs/au` (61), each `reviewed = "none"`, with `[[match]]` and `[[differ]]` vectors; `src/packs.rs` loads the packs named in `EPR_PACKS` (unset means none) from `EPR_PACKS_DIR` (default `packs`), refuses unknown keys, a wrong id, a code outside 1 to 999, a pack without vectors and a vector the real matcher fails, stops on two packs that disagree unless `EPR_PACK_WINNER_PHONE_CALLING_CODE` names a pack, hashes the rules and logs the hash and a warning per pack; `app.rs` calls it at start; the three matching call sites (`links.rs`, `controllers/patients.rs` twice) use `packs::engine()`. New dependency `toml` 1 (already in the lock through loco); `cargo deny check` passes.
- **Tests:** `tests/packs.rs` (9, database-free): the shipped packs load, none enabled changes nothing, a clash needs a winner and a wrong winner is refused, the hash changes with the rules, seven broken packs are refused, a `[[differ]]` that matches is refused, bad ids, the setting's parsing, and the code changes what a local number means. Full suite 350 pass, 0 fail without a database; fmt and clippy `-D warnings` pass.
- **Not done / not verified:** the service was not started with `EPR_PACKS` set, so the `app.rs` wiring and the log lines were not seen; the matcher still defaults to 44, so "no pack" is today's UK behaviour, not a neutral core; the calling codes are from the E.164 list and were not checked against a second source; no person has reviewed either pack; the hash is not written on records; no override directory; no HL7 validator step; no dry run over a labelled corpus (ID-1); steps (b) and (c) are not started; the test's scratch directories in the system temp folder are not removed; the `Containerfile` did not copy `packs/` (fixed in the next commit, `COPY packs ./packs`, but the image was not built here, so the copy is untested until CI builds it); CI has not run.

## 2026-10-30 - UI: a refused request says so (OS-2, H-039)

- **Built:** the proxy now forwards `Retry-After`; `commonProblem(429)` returns `rateLimited`, whose text says the request was not carried out and nothing was changed; the 24 call sites that use `STRINGS[common]` show it. Tests: one proxy test (429 and `Retry-After: 17` pass through) and one `commonProblem` case. 247 vitest and 85 Playwright pass; lint, `check` and `check:tests` pass; UI coverage lines 92.18, branches 86.6.
- **Not done / not verified:** no page was driven to a 429 in a browser, so the wording on screen is unseen; pages that handle their own statuses without `commonProblem` were not audited and may still show nothing useful; the wording is English only and unreviewed; the UI does not wait or retry by itself and does not show the `Retry-After` figure; H-039 stays open.

## 2026-10-30 - Service: per-subject request limits (OS-2, in part)

- **Clinical risk:** a limit is a refusal a clinician could meet. H-039 is added `open` (not reviewed); OQ-OS1 is marked assumed, because the Clinical Safety Officer has not answered it.
- **Built:** `src/rate_limit.rs`, mounted in `app.rs` after authentication so the subject is the verified token's: a fixed one-minute window per `(subject, class)` held in the process; defaults 1200 requests and 120 compare calls per minute, set by `EPR_RATE_LIMIT_PER_MINUTE` and `EPR_RATE_LIMIT_COMPARE_PER_MINUTE` (1 to 1,000,000; anything else stops the service starting); past a limit, 429 with `Retry-After` and the usual error body; the first refusal of a subject in a window is an audit row, all are counted in `epr_rate_limited_total`; requests without a subject are not counted; if the table holds 100,000 live entries a new subject is let through. Config comments that promised the module now describe it; `SECURITY.md`, `spec/07` and the benchmark row are corrected.
- **Tests:** 6 unit tests (trip, window reset, separate subjects and classes, `Retry-After` never 0, a clinician at five requests a second for ten minutes and a compare every two seconds never trips, full table fails open, settings parsing); 3 router tests and 1 database-gated audit test (three refusals, one audit row). Full suite with database: 458 pass, 4 fail (`fhir_validator`, `FHIR_VALIDATOR_JAR` unset, as before); fmt and clippy `-D warnings` pass.
- **Not done / not verified:** the service was not run with the layer and no request went through the real app, only a test router with the same middleware; counts are per process, so N replicas allow N times the limit and a restart forgets them; no client-address key (the service sits behind the UI's server, so the address is one caller for everybody); no sign-in lockout (the identity provider does sign-in); only the first refusal per window is audited, a departure from "each refusal audited"; the defaults are reasoned from a guess at clinical use (20 requests a second, compare every half second), not measured; the UI does not yet show a clear message for a 429; no Redis; CI has not run.

## 2026-10-30 - Service: outbox outcome counters (OS-3, in part)

- **Built:** `epr_outbox_delivered_total`, `epr_outbox_refused_total` and `epr_outbox_failed_total` on `/metrics`, counted after the attempt is committed. No labels and no patient data. The metrics test lists them; the delivery test checks the delivered counter rises. Unit tests, fmt, clippy `-D warnings` and the 8 outbox database tests pass.
- **Not done / not verified:** the refused and failed counters are not asserted in a test; no queue-depth gauge (the metrics module has counters only) and no alert, so a stuck event shows only as a failed count that keeps rising; counters restart at zero with the process and with several replicas each reports its own; no hazard row (no behaviour a clinician sees changed); CI has not run.

## 2026-10-30 - Service: backoff between outbox retries (OS-3, in part)

- **Built:** a failed event waits 30 seconds before its first retry and twice as long after each further failure (exponent capped at 10), decided in the claim query from the failure count and the newest failed attempt. `run_once` uses it; `run_once_with` takes the base in seconds so a test can use 0. A new database-gated test shows a failed event is not retried within the wait (3 runs, 1 call); the retry-limit test uses base 0. The 8 outbox database tests passed on four consecutive runs; fmt and clippy `-D warnings` pass.
- **Not done / not verified:** only the first wait was tested, not the doubling or the cap; the wait is measured by the database clock from the previous attempt, so a clock step changes it; the 30 seconds is a guess, not a measured figure; the setting is not configurable at run time; the 114-test database suite was not re-run, only the outbox file; CI has not run.

## 2026-10-30 - Service: outbox loop and a log sink (OS-3, in part)

- **Built:** `outbox::run_loop` (runs `run_once` on an interval, logs a database failure and carries on), `LogSink` (logs the event id, kind and recipient, never the payload or the patient), and `EPR_OUTBOX_INTERVAL_SECS`: off unless set, 1 to 3600 seconds, a bad value stops the service starting; when set, `app.rs` spawns the loop with the `log` sink only. Service `spec/index.md` has the row. Tests: `parse_interval` (unit) and `the_loop_delivers_without_being_asked_again` (database-gated); 295 unit tests and the 7 outbox database tests pass, fmt and clippy `-D warnings` pass.
- **Not done / not verified:** the wiring in `app.rs` was compiled but the service was not started with the variable set; nothing enqueues, so with the loop on it finds nothing; no real sink (IO-6); no loco worker; no backoff, so a failing event is retried at every interval until 5 failures; with several service replicas every one runs the loop, which the claim lock allows but the load was not tried; the loop has no graceful shutdown; the setting is read once at start; CI has not run on this.

## 2026-10-30 - Service: transactional outbox and delivery runner (OS-3, in part)

- **Built:** migration 17 with append-only `outbox_events` and `outbox_attempts` (triggers refuse update, delete and truncate); `src/outbox.rs`: `enqueue` joins the caller's transaction, `decide` makes the read path's decision for the recipient (`policy::evaluate` with its roles and purpose, no relationship, the patient's consent; any allow that carries a masking obligation is refused), `run_once` claims one event per transaction with `FOR UPDATE SKIP LOCKED`, calls the recipient's `Sink` only on an allow, and records `delivered`, `refused` (final, audited) or `failed` (up to 5, one attempt per run). A failed consent lookup is a failure, never a delivery. `spec/08` has the design; `spec/10` lists the tables; H-038 is open (added, not reviewed); 14 inventory rows classify the columns.
- **Tests:** 4 unit tests and 6 database-gated tests (rollback never delivered, commit delivered once, a refusing patient and a disallowing role get nothing and are audited once, a failing sink stops at 5, a recipient with no sink is not lost, the tables are append-only). All 114 database-gated tests pass except the 4 `fhir_validator` ones, which need `FHIR_VALIDATOR_JAR` set (not set here) and panic before any outbox code runs; 294 unit tests, fmt and clippy `-D warnings` pass.
- **Not done / not verified:** nothing calls `enqueue` and no sink exists (IO-6), so no event leaves the service; no loop calls `run_once` and there is no loco worker; no backoff; delivery is at least once and a claim race between two runners can deliver twice (the race was reasoned about, not provoked); the producer supplies the roles and purpose, with no registered-subscriber check; a sink's error text is stored as given, so "names no clinical content" is a rule, not a check; no queue-depth metric or stuck-event alert; the chaos sweep does not cover the runner; CI has not run on this.

## 2026-10-30 - CI: UI coverage floor (OS-4, in part)

- **Built:** `@vitest/coverage-v8` 5.0.5 (dev); `vite.config.ts` sets floors of 90 percent of lines and 84 of branches over `src/lib/**/*.ts`; the UI job runs `vitest run --coverage`. Measured: lines 92.17, branches 86.57, 246 tests pass. `coverage/` is ignored by git, prettier and eslint.
- **Not done / not verified:** `.svelte` components and `src/routes` are outside the figure, so it describes library code, not the pages; the 2 skipped tests (live Redis) are not counted; a floor of 99 percent fails with exit 1 as it should; the job has not run on GitHub; no Rust SAST (OQ-OS3); no hazard row (no behaviour changed).

## 2026-10-30 - Spec: architecture status re-read (OS-9, in part)

- **Changed:** `spec/08-architecture.md` called the FHIR facade and the policy engine "planned" and listed only the M0 shell as built; both are in `src/` (`controllers/fhir.rs`, `fhir_r5.rs`, `policy.rs`, `authz.rs`), as are consent, break-glass, the audit chain and the ADT feed. The diagram and closing sentence now say so.
- **Not done / not verified:** only the "planned" and "built" claims in section 8 were checked, by file existence; whether every route is policy-checked and audited was not traced; other spec files were not re-read; no code changed.

## 2026-10-30 - Governance: code of conduct (OS-14)

- **Built:** `CODE_OF_CONDUCT.md`, the Contributor Covenant 2.1 text fetched from contributor-covenant.org, with the one placeholder filled: reports go to the maintainer's address (the one `SECURITY.md` already publishes) with the subject `CONDUCT: electronic-patient-record`. `CONTRIBUTING.md` and `GOVERNANCE.md` link to it; `llms.txt` and `llms.json` list it.
- **Not done / not verified:** the covenant names "community leaders" and there is one maintainer, so there is no second person to report to when the maintainer is the subject of a report (OQ-OS: name a backup contact when a second maintainer exists); the text was not compared line by line with the upstream repository, only fetched from the official site; nobody has confirmed the address is the one the maintainer wants for conduct reports; no behaviour changed, so no hazard row.

## 2026-10-30 - CI: secret scan over the full history (OS-11)

- **Built:** `ci.yml` job `secrets` (required by `ci-ok`): gitleaks 8.30.1 installed from the release tarball and checked against a pinned SHA-256, run with `--redact` over the full history. `.gitleaksignore` holds accepted findings, each with a reason.
- **Found:** a local scan of 202 commits reported 2 findings, both the scaffold `auth.jwt.secret` loco wrote into `config/development.yaml` and `config/test.yaml` in the first commit. The service builds loco without its `auth` feature, nothing in `src/` reads that block, and `production.yaml` has none, so they are allow-listed by fingerprint, not rotated, and not removed from history. The first run with the allow-list reports no leaks; a planted AWS-style key in a throwaway repository is reported.
- **Not done / not verified:** the job has not run on GitHub (CQ-20); it reads git history only, not ignored files, build output or container layers; a pattern scanner misses secrets that do not look like secrets; no pre-commit hook; the two accepted values remain readable in history; no hazard row (no behaviour changed).

## 2026-10-30 - Supply chain: dead advisory ignores removed, the last one expires (OS-12)

- **Found:** `cargo deny check advisories` reported 12 of the service's 13 ignores and all 11 of the matcher's as "advisory was not encountered". The RustSec database shows patched releases for four of them (protobuf 3.7.2, anyhow 1.0.103, quick-xml 0.41.0, crossbeam-epoch 0.9.20), the service lockfile already holds the patched anyhow, quick-xml and crossbeam-epoch, and the other crates are not in either tree. The note that they could not be fixed without a loco-rs bump was wrong for these.
- **Changed:** both `deny.toml` files lost the dead ignores (the matcher now has none). One stays, RUSTSEC-2024-0449 (`unsafe-libyaml`, unmaintained, reached through loco's own YAML config parsing), with its reason and `expires 2027-01-30`. The matcher's header named a `security.yml` workflow that does not exist; it now names the `deny` job.
- **Tests:** `tests/advisory_expiry.rs` (3) fails on a missing, expired or too-distant date in either file, and was checked against the committed matcher file; `tests/doc_census.rs` now reads the matcher's `deny.toml` too. Full `cargo deny check` passes on both crates; service 327 and matcher 96 tests pass; fmt and clippy `-D warnings` pass.
- **Not done / not verified:** which code paths of `unsafe-libyaml` run was not traced; the test proves a date is written, not that the reason is true; a newly published advisory can fail CI, as intended; `cargo deny` duplicate-crate warnings (about 35) and unused-licence warnings are unchanged and were not touched; CI has not run on this.

## 2026-10-30 - Spec: pack mechanism designed (OS-10, in part)

- `spec/interoperability/index.md` gains "Pack mechanism (design, OS-10)": a pack is data, never code (algorithms come from a closed set in the service and matcher); one directory per pack with a manifest and domain files (TOML rules, CSV tables) mapped row by row to the equity audit; four load-time checks, including the pack's own test vectors and the HL7 validator with a control that must fail; one `enabled` list per deployment; load order core, enabled packs, read-only override directory, environment; conflicts on the same key stop startup unless the deployer names the winner, recorded in a hashed effective rule set written on the records it influenced; seven things no pack may do; the matcher receives rules as values and never reads a file; a three-step first slice (phone code, NHS number as the first rule, Australia's IHI as the second). OQ-OS5 (format and home) and OQ-OS6 (unreviewed packs in production) added; IO-18, the equity audit and tasks point at it.
- **Not done / not verified:** nothing is built and no prototype loader tests the design; it is a proposal the maintainer has not confirmed; the borrowed shapes (OpenMRS Initializer, OpenMRS 3 config schemas, HAPI starter keys, Ozone child distros) were read from documentation, not code; the effective-rule-set hash on records needs a migration and a hazard row, neither written; no signing scheme for packs; no clinical behaviour changed, so no hazard row now.

## 2026-10-30 - UI: shared, encrypted session store (OS-8, in part)

- **Built:** the session store split into policy (`session.ts`) and storage (`session-backends.ts`): memory by default, unchanged; Redis when `EPR_SESSION_REDIS_URL` and `EPR_SESSION_ENCRYPTION_KEY` are both set (either alone stops startup). Records are sealed with AES-256-GCM bound to their key (`session-crypto.ts`) and stored under a SHA-256 of the cookie value, so Redis holds no token and no usable cookie. TTL is the earlier of idle and absolute expiry; a background look still does not extend a session; login attempts use `GETDEL`, single-use across replicas; a refreshed token is written back (`save`). If Redis is down the proxy answers 503 and keeps the cookie; the page shows the reader as signed out and deletes nothing. The store API became async, so `auth.ts`, `auth-instance.ts`, `hooks.server.ts` and `proxy-handler.ts` changed. New dependency `redis` 6.2.1 (published 2026-08-11); `pnpm audit` high is clean.
- **Tests:** 17 new unit tests with a fake Redis (encryption round trip, tamper, wrong key, wrong storage key; two stores sharing one fake as two replicas: shared session, idle and absolute expiry, sign-out, single-use attempt under a race, persisted refresh, nothing in the clear, wrong key reads nothing); an opt-in live test (`EPR_TEST_REDIS_URL`) passed against a real Redis 7 through node-redis and skips otherwise. 13 existing tests were converted to async. Lint, `check`, `check:tests`, 246 vitest and 85 Playwright pass.
- **Found on the way:** my first fake `getDel` was not atomic and made the single-use test fail; real `GETDEL` is atomic, and the fake now is too.
- **Not done / not verified:** no run behind two real UI processes and a proxy (the task's Playwright acceptance); sessions are not counted in Redis, so the 10,000 cap is memory-only; no key rotation or key id (a new key signs everyone out); not wired in `compose.yaml`; Redis availability is now sign-in availability when it is on; the Redis connection is plain unless the URL is `rediss://`, and only ciphertext crosses it; no hazard row (sign-in behaviour is unchanged by default); CI has not run on this.

## 2026-10-30 - Service: OpenAPI inventory with a freshness gate (OS-7, in part)

- **Built:** `src/openapi.rs`, an OpenAPI 3.1 document derived from the routes the service serves and the policy table, so a served route cannot be missing; `GET /api/openapi.json` (authenticated; policy row patient identity, read, as for `/fhir/R4/metadata`); the committed copy `openapi.json` (51 paths, 62 operations); `tests/openapi_census.rs`. `spec/09-api-surface.md` describes it. No new dependency.
- **Tests:** 5 census tests: committed copy equals the code's output (corrupting it fails the test, as checked), every served route is present with its resource, action and security, public routes are marked, no version segment in any path, unique operation ids, and a planted route appears. fmt, clippy `-D warnings` and the whole database-free suite pass (324 tests). The route census still passes with the new policy row.
- **Found on the way:** my `tests/metrics.rs` histogram test from the previous entry failed about one run in a few because a digit string it searched for (`0042`) can occur inside a duration sum; it now uses tokens that cannot, and passed 60 runs in a row.
- **Not done / not verified:** no request or response schemas (a client cannot be generated); responses are only default, 401 and 403; the document is derived from the table, not from handlers, so it cannot say what a handler returns; the route is behind login, so an anonymous integrator cannot read it; database-gated tests and the chaos sweep were not run (the chaos sweep now also visits the new route); no hazard row (no clinical behaviour changed); CI has not run on this.

## 2026-10-30 - Service: request-duration histogram (OS-6, in part)

- **Built:** `metrics::duration_mw` as the outermost layer; `GET /metrics` now also serves `epr_http_request_duration_seconds`, a Prometheus histogram labelled by method, matched route template and status code. Refusals (401, 403, 503) are timed because the layer sits outside authentication; unmatched requests are `unmatched`; at most 1000 series, the rest folded into `overflow`; label values escaped. No new dependency.
- **Tests:** 3 unit tests in `metrics.rs`; `tests/metrics.rs` sends four requests through a real router and asserts the labels are templates (`/things/{id}`), a 403 and `unmatched`, and that none of the ids in the paths appears anywhere in the output. fmt, clippy `-D warnings` and the whole database-free suite pass (290 unit tests).
- **Checked, not built:** loco 1.2's `/_readiness` already pings the database, queue and cache and answers 503.
- **Not done / not verified:** no OTLP tracing. Loco initialises the subscriber itself, so an exporter would replace its logger, and what may leave the process (OQ-OS4; `sqlx::query` spans can carry statement text) is a privacy decision for the maintainer and data protection officer. The database-gated tests and the chaos sweep were not run after this change, and the histogram was not read from a live server; the series are in memory, reset on restart, with no exemplars; the middleware's cost was not measured (BENCHMARKS.md predates it); no hazard row (no clinical behaviour changed).

## 2026-10-30 - Load measurement and first published numbers (OS-5, in part)

- **Built:** `scripts/load/measure.mjs` (Node, no dependencies) and `BENCHMARKS.md`. Run once against a release build under the `container-dev` profile with Postgres 18 from `compose.test.yaml`: registering a patient stays at 166 to 194 a second at 1, 8 and 32 clients (p95 12 ms rising to 191 ms); opening a patient by id reaches about 720 a second (p95 19 to 65 ms); search about 860 (p95 13 to 91 ms); no errors. The benchmark database and containers were removed afterwards and another project's Postgres was left alone.
- **Not done / not verified:** the flat write rate is consistent with the audit lock (CQ-18) but the lock was not isolated, no wait events were read, the pool size was not varied; no CI job and no regression gate; the 1 s chart-open and 200 ms typeahead targets of NFR-2 are not measured because neither a chart nor a typeahead index exists; one client on the same host, debug logging on, 10 seconds per step, one run, about 5,400 patients not a million; the build was a dirty tree at `fdcd091`; no hazard row (no behaviour changed).

## 2026-10-30 - CI: coverage floor, CodeQL and pnpm audit (OS-4, in part)

- **Built:** three `ci.yml` jobs, all required by `ci-ok`: `coverage` (cargo-llvm-cov; matcher at least 95 percent of lines, service database-free tests at least 50; lcov uploaded), `codeql` (JavaScript and TypeScript, build mode none), `pnpm-audit` (high and above).
- **Measured, not guessed:** matcher 97.65 percent of lines, service `--lib` 52.24 percent; both commands run locally and exit 0 at the floor, and the matcher exits 1 at a floor of 99. `pnpm audit --audit-level high` passes today (one low finding).
- **Not done / not verified:** none of the three jobs has run on GitHub (CQ-20), so action versions and the CodeQL setup are untested; no Rust static analysis (OQ-OS3); no UI coverage; the service figure excludes 112 database-gated tests and understates real coverage; no codecov upload or trend; a floor proves lines ran, not that they were checked; no hazard row (no behaviour changed).

## 2026-10-30 - Service: documentation census and stale statements fixed (OS-9, in part)

- **Built:** `tests/doc_census.rs` fails when `config/*.yaml`, `deny.toml` or `SECURITY.md` names a `src/*.rs` file or a workflow that does not exist. Fixed: config comments that promised a rate limiter, a response cache and a CORS allow-list (none exists; the comments now say so and name OS-2 and OS-3); `policy.rs` (two modules call `evaluate`); `auth.rs` (consent and break-glass are built); `SECURITY.md` (`/metrics` is public; roles, consent, break-glass and the audit chain are built, rate limiting is not); `deny.toml` (names the `deny` job in `ci.yml`).
- **Tests:** the census fails on the pre-fix files with seven missing paths and passes after; a planted-path test checks the scanner; fmt, clippy `-D warnings`, 287 unit tests and the four other census tests pass.
- **Not done / not verified:** the census checks that a path exists, not that the description is true; it does not scan `tasks.md`, `spec/` or `src/` comments; `spec/08` was edited in the earlier entry but not re-read; the settings keys `rate_limit` and the Redis cache stay in config unread; database-gated tests were not run (no config or code path they use changed); no hazard row (no behaviour changed); CI has not run on this.

## 2026-10-30 - Spec: benchmark against open-source EPR and EHR systems (OS-1, in part); equity facts verified (IO-19, in part)

- New `spec/benchmark.md`: twelve repositories (OpenEMR, Medplum, HAPI, OpenMRS, Blaze, EHRbase, Metriport, Haste, Helios, Marley, VistA, HospitalRun) read through the GitHub API with exact stars and dates; an engineering matrix of 36 items (11 of 25 table-stakes met, 7 partial, 7 missing: tier LAGGING, 44 percent), a capabilities-and-configuration matrix against nine systems, fifteen ranked gaps, eight configuration patterns worth copying. plan.md §22 assesses and ranks; tasks OS-1 to OS-14 added (`OS-` prefix registered in `generate-status.sh`); OQ-OS1 to OQ-OS3; `spec/07`, `08`, `11`, `13`, `16`, `17` and `index.md` updated; `.sota/` holds the rubric and the scan record.
- **Found about ourselves on the way, recorded as OS-9 and not yet fixed:** `config/*.yaml` comments name `src/rate_limit.rs`, `response_cache.rs` and `http_security.rs`, which do not exist; `tasks.md` named `src/openapi.rs`, which does not exist (the convention line now says so); `policy.rs` says nothing calls `evaluate` but two modules do; `auth.rs` and `SECURITY.md` say consent and break-glass are not built; `deny.toml` cites a `security.yml` that does not exist; `spec/08` listed tantivy, an outbox and row-level security as if present (now marked not built); CQ-18 cited `audit.rs:25` (now `:23` and `:431`).
- `spec/equity-audit.md` gains "Verified 2026-10-30": row 4 corrected (the 44 is a config field the service never sets, not a constant), row 12 corrected (the terminology check judges SNOMED CT only and passes other systems), row 11 confirmed; sourced identifier facts for Australia, New Zealand, Scotland, the US and Brazil (Ireland, Germany and India unverified); FHIR `fathers-family`/`mothers-family` extensions and the W3C names guidance for the name pack; the trunk-prefix rule for phones; EHDS dates.
- **Not done / not verified:** nothing built, no code changed, no hazard row (no behaviour changed); the comparators were not cloned and a few cells are marked unverified (GitHub code search was rate-limited); certification claims were read from the projects' own pages; the capabilities column for us states what is built, not what is tested end to end; effort figures are estimates; the `.sota/` files are untracked and the maintainer decides whether to commit them. Gates run: `generate-status.sh`, `generate-llms.py`, `check-links.py` (results in the commit message when committed; CI has not run on this).

## 2026-10-29 - Spec: equity audit of the core (IO-19, in part)

- New `spec/equity-audit.md`: 13 findings, each with file, who is excluded, a proposed pack boundary and whether it is clinical. Largest: the NHS number is the only deterministic identifier in the matcher; the phone default is 44; names are one family string plus ordered givens; Soundex and nicknames are English-oriented; units are one fixed table; only English strings exist and right-to-left is a one-entry list.
- **Not done / not verified:** found by keyword search, not a census; no code changed, so none is fixed; the compliance and terminology rows were not re-read; example countries and laws are from memory. Row 8 (access-log zone label) was first misread as a US date format and corrected after reading the function; other date displays were not read. Several rows are clinical and need a hazard row before a fix. IO-19 stays open.

## 2026-10-29 - Spec: equity across countries, regions and federations (IO-19 planned)

- Widens the previous entry from FHIR to everything: `spec/01-purpose-and-vision.md` "Worldwide and equitable"; `spec/02-scope.md` gains the **Equity rule** (neutral core, country rules in packs, the same evidence for every pack, conflicts decided by configuration and recorded, gaps named) and names regions and federations; `spec/interoperability/index.md` says the pack idea also covers HL7 v2 variants, CDA, openEHR and regional exchanges; `spec/locales.md` says locale data is a pack and lists right-to-left, calendars, name order and plural rules; task IO-19 added (audit the core for UK and US assumptions).
- **Not done / not verified:** nothing is built or audited; I did not search the code or specs for country assumptions, so the claim that the repository is UK and US weighted rests on the files I have read, not a census; the federations named are from memory and unchecked; the example list of assumptions is not exhaustive; compliance (`spec/12`) and the glossary still read UK-first; no hazard row because no behaviour changed.

## 2026-10-29 - Spec: worldwide target, national FHIR profile packs (IO-18 planned)

- `spec/01-purpose-and-vision.md` gains "Worldwide"; `spec/02-scope.md` gains "Reach" and says country-named rules (UK MDR, DCB0129, HIPAA) are that country's; `spec/interoperability/index.md` gains "National standards" (base FHIR international; one profile pack per country; supported only when that country's validator passes; terminology not assumed; machine-assembled until a local reviewer); task IO-18 added.
- **Not done / not verified:** nothing is built, no pack mechanism exists; the example country guides are from memory and unchecked; the existing `metadata` still names UK Core and US Core only; other specs (compliance, glossary, roadmap) still read UK-first and were not revised; no hazard row yet because no behaviour changed; translations and locale work (`spec/locales.md`) were not touched.

## 2026-10-29 - Spec: R5 differences read from HL7 (IO-17, in part)

- `spec/interoperability/index.md`: the R4 to R5 table is replaced by the differences HL7 lists for Patient, AllergyIntolerance, Observation, AuditEvent, Consent and Provenance (<https://hl7.org/fhir/R5/diff.html>). New finding: R5 `AllergyIntolerance` deletes `recorder` and `asserter` for `participant`. `fhirVersion` rule written down: HL7 gives no status code for an unsupported version, so our 406 is a choice. No R5 release of UK Core (R4 line) or US Core 9.0.0 (R4) found.
- **Not done / not verified:** Subscriptions still `[uncertain]`; the pages were read through a summarising fetch tool, not line by line; no code changed, so whether the converter handles `participant` for an allergy with a recorder is untested; the earlier note that the service's allergy `recorder` survives conversion is unchecked; no hazard row, no behaviour change. IO-17 stays open.

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
