## 11. Testing Strategy

Unit tests in each crate; database-gated tests (`#[ignore]`, need Postgres) for migrations and storage; Playwright with
axe for the UI; the official HL7 FHIR validator as an oracle for what the service writes (`tests/fhir_validator.rs`, run by hand, not yet in CI); planned: FHIR conformance (Inferno (g)(10), the validator against RC-2 search and UK Core), a labelled
patient-matching corpus (ID-1), fuzzing (matcher, HL7 v2 parser), load (1M patients), chaos, clinical safety test
cases traced to the hazard log, NISTIR 7804 summative usability testing. What each gate proves and does not prove:
[`../agents/testing.md`](../agents/testing.md).
