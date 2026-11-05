## 11. Testing Strategy

Unit tests in each crate; database-gated tests (`#[ignore]`, need Postgres) for migrations and storage; Playwright with
axe for the UI; the official HL7 FHIR validator as an oracle for what the service writes (`tests/fhir_validator.rs`, run by hand and in the `fhir-validator` CI job); planned: FHIR conformance (Inferno (g)(10), the validator against RC-2 search and UK Core), a labelled
patient-matching corpus (ID-1), fuzzing (matcher, HL7 v2 parser), load (1M patients), chaos, clinical safety test
cases traced to the hazard log, NISTIR 7804 summative usability testing. What each gate proves and does not prove:
[`../agents/testing.md`](../agents/testing.md).

Where the gates run, and a proposal to run more of them locally first with GitHub as the independent second check: [`local-ci.md`](local-ci.md) (LC, 2026-11-04; nothing built).

**Risk field census (SW-1).** `tests/risk_field_census.rs` fails if a field shaped like a suicide or self-harm risk score appears in the service, migrations, matcher or UI, because NICE NG225 advises against such tools ([suicide watch](suicide-watch/index.md)). It reads names only.

**Stored words (PR-3)** also cover adjustment statuses; `tests/http_adjustments.rs` runs PA-6 through the real router with login on.

**Grief census (SG-2).** `tests/grief_census.rs` fails on a name shaped like a family or household view, a list of people bereaved by suicide, an anniversary mailing or a grief score; on a route about a family, a household or grief; on a new caller of `outbox::enqueue` until it is listed, and on one that works from a date of death, a due date or a birth date; and if a death and a referral meet in code ([do not build](survivor-grief/index.md#do-not-build)). It reads source text, so it finds names and calls, not intent.

Against the field ([benchmark](benchmark.md), 2026-10-30): the census tests, the seeded hostile-input sweep and the
hand-run mutation pass have no counterpart in any comparator. Missing here and usual there: coverage measured and
gated in CI (OS-4), a performance run on `main` with published numbers (OS-5), a documentation census (OS-9),
property tests from a library rather than by hand (CQ-30), and conformance test kits in CI (after IO-2 and IO-4).
