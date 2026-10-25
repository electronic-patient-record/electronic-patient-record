# Patient identity

Explainable weighted matching, deterministic NHS-number rule, refusals for missing evidence, twins and differing verified numbers; registration-time duplicate prevention; reversible merge with `Patient.link` and provenance; FHIR `$match`. Tasks ID-1 to ID-7. **Built today: the matcher and the compare endpoint.** See [matcher spec](../../electronic-patient-record-matcher-with-rust/spec/index.md).

Planned; nothing here is built yet unless stated. Source: [`../../plan.md`](../../plan.md).

## A shared NHS number (CQ-37, decided 2026-10-25)

A shared valid NHS number no longer links two records automatically on its own. Missing evidence is not agreement: if the given name, the family name or the birth date is absent on either record, the pair goes to Review and the matcher says which field is missing. A pair with all three present, no full-birth-date disagreement and not both names disagreeing still links; a family name that differs (marriage) does not block it. The decision was the maintainer's, taken on the recommendation to apply the project-wide rule here too; the Clinical Safety Officer has not reviewed it (H-001 stays `controlled-unreviewed`).
