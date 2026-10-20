## 1. Purpose and Vision

Build open-source electronic patient record (EPR) software that is **safe, standards-native, auditable and usable**.

- **Safe**: an append-only longitudinal record; wrong-patient and missed-result hazards designed against, not patched.
- **Standards-native**: FHIR R4 is both the data model and the API; SNOMED CT, LOINC and UCUM for codes.
- **Auditable**: every read and write is recorded; patients can see who accessed their record.
- **Usable**: designed against documented causes of clinician burnout (documentation burden, alert fatigue, inbox
  load, latency). See [usability](usability/index.md).
- **Compliance-ready, not compliant**: it supports a deployer's HIPAA, UK GDPR, NHS DSPT/DTAC, DCB0129/0160 and ONC
  (g)(10) work with evidence. Compliance belongs to a deployment.

**The gap.** Open source offers complete applications on legacy stacks (OpenEMR, OpenMRS, OSCAR, VistA) or modern
headless FHIR platforms with no clinical UI (Medplum, HAPI FHIR, EHRbase). No widely adopted project combines a
FHIR-native core with a complete, safety-engineered, usability-tested clinical UI. Research: [`../plan.md`](../plan.md) §3.
