# Functional suitability: what it means here

Public-source research read 2026-11-04 plus this repository's own files. A proposal and a reading: targets here are not commitments, and nothing was measured by the author of this note unless it says so.

## Definition

ISO/IEC 25010:2023 defines functional suitability as the degree to which a product provides functions that meet stated and implied needs when used under specified conditions. Its sub-characteristics, as an iso25000.com summary lists them, are functional completeness (the functions cover the specified tasks and users' objectives), functional correctness (results are accurate for the intended users) and functional appropriateness (the functions help users accomplish the specified tasks and objectives) [1]. I saw that summary and a search snippet of the ISO catalogue page, not the standard's text; the ISO page returned HTTP 403 [2]. The 2011 edition used the same three names for functional suitability, as I recall, but I did not check this against the 2011 text. The 2023 edition added safety as a ninth characteristic and moved quality in use to ISO/IEC 25019 [3]; harm to people is therefore treated in [safety](../safety/index.md), not here.

The SEI approach (Bass, Clements, Kazman) treats functionality as the thing that is not a quality attribute: it is what the system does, and quality attributes decide how well and how safely it does it. Their six-part scenario (source, stimulus, artifact, environment, response, response measure) is still usable for a function whose correctness has a measurable response [4].

## Why it matters for an EPR

Functional gaps in an EPR are felt at the bedside. Public reports of EPR go-lives describe a trust declaring a critical incident after go-live at Nottingham University Hospitals [5], and patients complaining about disrupted outpatient bookings after a new EPR at Sheffield Teaching Hospitals [6]. I saw only search snippets of those reports, which do not separate missing function from training, data migration or workflow design, so they illustrate fit-to-workflow risk, not a verified cause.

Functional correctness for an EPR is mostly about identity and data: the right patient, the right allergy, no silent loss. A matcher that agrees on missing data would be incorrect in the most dangerous way, which is why this project's rule is that absent evidence is `null` with a reason and blocks an automatic link.

## What the project says today

- **Scope is a scaffold.** `spec/07-non-functional-requirements.md` opens by saying nothing is measured, and `spec/06-functional-requirements.md` says that at M0 none of FR-1 to FR-8 is met except FR-1.1 in part. The statement of what is built is in `spec/14-implementation-status.md` and `tasks.md`.
- **Built (per the specs, which I read but did not re-run):** the matcher with an explainable score (FR-1.1, NFR-9 determinism), a duplicate check at registration (ID-4), an audited patient register, hash-chained audit, policy checks, an HL7 ADT feed over MLLP (off unless configured), and a read-only FHIR R4 and R5 facade for Patient, AllergyIntolerance and Observation. No clinical record exists yet.
- **Planned:** record core (FR-3), documentation (FR-5), orders and results (FR-6), care pathways (FR-8.8 to FR-8.11) and the rest. Functional appropriateness is argued in `spec/pathway-design.md`, `spec/care-settings.md`, `spec/triage` and `spec/usability`, none built.
- **Benchmark view.** `spec/benchmark.md` (2026-10-30) says plainly that functionally the repository is an M0 to M1 scaffold while the comparator field is not, and treats that gap as the roadmap.
- **Guards that bear on correctness.** Census tests (routes, OpenAPI, policy, config), the HL7 FHIR validator as an oracle for what the service writes (`tests/fhir_validator.rs`, CI job `fhir-validator`), database-gated tests for the append-only rules, and NFR-12 (no computed score may reduce what a patient is offered; its tripwire reads names, not behaviour, so it is not proof).
- **A labelled matching corpus is planned, not built** (ID-1 acceptance asks for 200 or more pairs including twins and transposed dates).

## Measures

| Measure | How to measure | Suggested target or rule for setting one | Where in our data/CI |
|---|---|---|---|
| Requirement coverage (completeness) | Count FR-n.m with a passing acceptance test over all FR items for the current milestone | 100% of the milestone's FR items traced to a test before the milestone is called done; report partial items as partial | `spec/14-implementation-status.md`, `sh scripts/generate-status.sh --check` |
| Matching accuracy (correctness) | Precision, recall and false-link count on a labelled pair corpus, reported per country pack | False automatic link target is zero on the corpus; set recall by clinical-safety review, not by the author | ID-1 corpus (not built); matcher `cargo test` |
| Standards conformance (correctness) | HL7 validator errors on every resource the service writes; Inferno results once SMART exists | Zero validator errors on written resources, with a control that must fail | `tests/fhir_validator.rs`, CI `fhir-validator` |
| Absent-evidence handling | Tests that a missing field yields `null` with reason, never agreement or zero | Every field with a "missing" case has a test | matcher tests; hazard log entries |
| Task completion (appropriateness) | Summative test: completion rate and time on critical tasks with clinicians | NFR-10 sets SUS at least 75; set task budgets with clinicians | `spec/usability`; not run |
| Alert override rate | Share of alerts overridden, per alert type | Track, then review outliers; no a-priori target | NFR-10; CDS not built |
| Defects escaped | Defects found after release by severity and by whether a test existed | Each escaped defect gets a test before the fix | issues, `CHANGELOG.md` |
| Spec-to-code drift | Census: documented routes, fields and settings equal the mounted ones | Zero drift | census tests, `doc_census` (OS-9) |

## Trade-offs with other attributes

- **Safety and functionality.** More function means more hazards; each clinical function needs a hazard log row first (AS-1) ([safety](../safety/index.md)).
- **Security and privacy against completeness.** Consent and relationship checks deliberately withhold data; a "complete" chart is complete only for those allowed to see it ([security](../security/index.md), [privacy](../privacy/index.md)).
- **Maintainability.** Append-only and no-delete design costs functions users expect (true deletion); it is chosen for auditability ([auditability](../auditability/index.md)).
- **Interaction capability.** A function nobody can find is functionally absent; see [interaction capability](../interaction-capability/index.md).
- **Interoperability.** Country packs add function per country but multiply what must be reviewed ([interoperability](../interoperability/index.md)).

## Gaps and risks

1. No clinical record yet, so almost all functional claims are plans.
2. No labelled matching corpus, so matcher correctness is argued from unit tests only.
3. Conformance to UK Core and US Core is by validator, not by an external test kit (Inferno planned after IO-2).
4. NFR-12's tripwire reads column names and cannot prove a score does not reduce care.
5. Functional appropriateness has not been tested with a clinician; the pathway and usability specs are proposals.
6. Machine-assembled translations and country packs are unreviewed by speakers or experts; completeness in those locales is unknown.

## Ideas

1. Add a requirement-to-test trace table generated from FR ids (extends the status generator; ties to `scripts/generate-status.sh`).
2. Build the labelled matching corpus (ID-1) and publish false-link and false-miss counts per release.
3. Write each critical function as a six-part scenario with a response measure before building it [4].
4. Run clinician walk-throughs of the pathway view (FR-8.8) with paper prototypes before code; record what was and was not tested.
5. Add a "does not do" list per release to `CHANGELOG.md` (already the practice; make it a table).
6. Treat a hazard-log row as a precondition to merging a clinical function (AS-1, already a rule; add a CI check for the link).

## Open questions

- Which clinical settings come first, and so which FR items define "complete" for M2?
- Who signs off functional appropriateness: a clinician advisor, the Clinical Safety Officer, or both?
- How is completeness defined for a deployment that uses only some country packs?

## Not verified

- The text of ISO/IEC 25010:2023; only summaries and snippets were seen.
- The causes behind the Nottingham and Sheffield reports.
- That the specs' "built" statements match the code at this commit; I read the specs, not the code.
- Nothing in this note was measured.

## Sources

1. https://iso25000.com/index.php/en/iso-25000-standards/iso-25010 : sub-characteristic names and definitions (summary page).
2. https://www.iso.org/standard/78176.html : ISO catalogue page (could not be fetched, 403); search snippets only.
3. https://quality.arc42.org/articles/iso-25010-update-2023 : 2023 changes: safety added, renames.
4. https://people.ece.ubc.ca/matei/EECE417/BASS/ch04lev1sec3.html : Bass, Clements, Kazman, the six-part quality attribute scenario.
5. https://www.digitalhealth.net/2025/11/nottingham-hospitals-declares-critical-incident-after-epr-go-live : Nottingham critical incident (snippet only).
6. https://www.digitalhealth.net/2025/09/patients-complain-about-disruption-following-new-epr-in-sheffield/ : Sheffield EPR disruption (snippet only).
