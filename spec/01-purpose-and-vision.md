## 1. Purpose and Vision

Build open-source electronic patient record (EPR) software that is **safe, standards-native, auditable and usable**.

- **Safe**: an append-only longitudinal record; wrong-patient and missed-result hazards designed against, not patched.
- **Standards-native**: FHIR R4 is both the data model and the API; SNOMED CT, LOINC and UCUM for codes.
- **Worldwide and equitable**: the target is every country, region and federation, not the United States or the United
  Kingdom alone. UK and US material in this repository is the first worked example, not the boundary. Standards,
  protocols, conventions and locales get **equitable capability**: no country's way is the built-in default that others
  must work around, and each is supported through a pluggable pack with the same mechanism and the same evidence rule
  ([equity rule](02-scope.md), "Reach"). For FHIR that is a **national profile pack** per country
  ([interoperability](interoperability/index.md), "National standards"). A pack is supported when its own validator or
  test passes, and not before.
- **Auditable**: every read and write is recorded; patients can see who accessed their record.
- **Usable**: designed against documented causes of clinician burnout (documentation burden, alert fatigue, inbox
  load, latency). See [usability](usability/index.md).
- **Compliance-ready, not compliant**: it supports a deployer's HIPAA, UK GDPR, NHS DSPT/DTAC, DCB0129/0160 and ONC
  (g)(10) work with evidence. Compliance belongs to a deployment.

**The gap.** Open source offers complete applications on legacy stacks (OpenEMR, OpenMRS, OSCAR, VistA) or modern
headless FHIR platforms with no clinical UI (Medplum, HAPI FHIR, EHRbase). No widely adopted project combines a
FHIR-native core with a complete, safety-engineered, usability-tested clinical UI. Research: [`../plan.md`](../plan.md) §3.
