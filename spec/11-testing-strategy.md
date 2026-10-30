## 11. Testing Strategy

Unit tests in each crate; database-gated tests (`#[ignore]`, need Postgres) for migrations and storage; Playwright with
axe for the UI; the official HL7 FHIR validator as an oracle for what the service writes (`tests/fhir_validator.rs`, run by hand and in the `fhir-validator` CI job); planned: FHIR conformance (Inferno (g)(10), the validator against RC-2 search and UK Core), a labelled
patient-matching corpus (ID-1), fuzzing (matcher, HL7 v2 parser), load (1M patients), chaos, clinical safety test
cases traced to the hazard log, NISTIR 7804 summative usability testing. What each gate proves and does not prove:
[`../agents/testing.md`](../agents/testing.md).

Against the field ([benchmark](benchmark.md), 2026-10-30): the census tests, the seeded hostile-input sweep and the
hand-run mutation pass have no counterpart in any comparator. Missing here and usual there: coverage measured and
gated in CI (OS-4), a performance run on `main` with published numbers (OS-5), a documentation census (OS-9),
property tests from a library rather than by hand (CQ-30), and conformance test kits in CI (after IO-2 and IO-4).
