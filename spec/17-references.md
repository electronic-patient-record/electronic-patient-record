## 17. References

Full list with citations: [`../plan.md`](../plan.md) §13. Key standards: HL7 FHIR R4 (hl7.org/fhir), US Core, UK Core,
IPS, SMART App Launch 2.2 (hl7.org/fhir/smart-app-launch), CDS Hooks (cds-hooks.hl7.org), Bulk Data
(hl7.org/fhir/uv/bulkdata), SQL-on-FHIR (sql-on-fhir.org), IHE profiles (profiles.ihe.net), USCDI (healthit.gov/isa),
Inferno (inferno.healthit.gov). Evidence: Melnick 2020; Arndt 2017; Steinkamp 2022; ECRI Top 10 Hazards 2026; NISTIR
7804 / 7741 / 7865. Regulation: DCB0129/DCB0160, DTAC, HIPAA Security Rule, ONC HTI-1/HTI-5, SAFER Guides.

Open-source systems read for the [benchmark](benchmark.md) (2026-10-30, plan.md §22.6): OpenEMR, Medplum, HAPI FHIR
and its JPA starter, OpenMRS core, Blaze, EHRbase, Metriport, Haste Health and Helios (both Rust), Marley Health
(formerly Frappe Health), VistA, HospitalRun (archived); Bahmni, Ozone HIS and GNU Health for capabilities only.

A proof-of-concept FHIR guide read in part for the [closed-loop referral model](interoperability/closed-loop-referral.md) (2026-11-02, plan.md §24.8): *Care Coordination and 360 Referrals* v0.1.0, <https://kevinmayfield.github.io/PatientCareCoordination/en/index.html> (seven pages; one author; no licence stated; "must not be used for implementation or clinical purposes").

The sister project, read for what this record can learn (2026-11-03, plan.md §25.7): the patient administration system in `~/git/patient-administration-system/patient-administration-system` at commit `4f75348` (a local checkout, same author; task titles and first lines, `spec/hazard/index.md`, `governance/production-readiness.md`, file names and commit messages; its code was not read).
