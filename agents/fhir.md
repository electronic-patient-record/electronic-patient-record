# FHIR

Decisions are in [`plan.md`](../plan.md) §4. In short: **R4 (4.0.1)** on the wire; UK Core and US Core 6.1.0 (tracking
8.x) profiles; IPS for summaries; **SMART App Launch 2.2**; **CDS Hooks 2.0**; **Bulk Data 2.0**; HL7 v2.5.1 inbound
through an adapter; SNOMED CT, LOINC, UCUM, ICD-10/11, RxNorm (US), dm+d (UK), CVX behind a `TerminologyService`.

Pitfalls: FHIR is an exchange model, so complex orders, flowsheets and billing may need side tables; search needs
extracted index tables, not JSONB scans; `entered-in-error` replaces deletion; an empty allergy list means "not
recorded", never "none known"; use the HAPI validator as a test oracle, not a runtime dependency.
Nothing in this file is built yet.
