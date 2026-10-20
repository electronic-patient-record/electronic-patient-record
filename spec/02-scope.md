## 2. Scope

**In scope for 1.0** (ambulatory first, inpatient core next): patient identity and registration; summary and banner;
problems, allergies, medications, immunisations, histories, vitals and flowsheets; encounters and basic ADT; notes,
templates and structured forms; orders and results with closed-loop follow-up; prescribing with interaction checks via
an external CDS service; care plans, teams, tasks, referrals; scheduling; patient portal; audit, consent,
break-the-glass; FHIR R4 API, SMART App Launch, CDS Hooks client, HL7 v2 inbound.

**Out of scope for 1.0**: billing and claims (X12, NHS PbR); a licensed drug knowledge base (we integrate one);
e-prescribing network certification (Surescripts, EPS) and EPCS; PACS (link via ImagingStudy/DICOMweb); LIS/RIS
internals; eMAR with barcode scanning (planned M7); ONC certification itself; eCQM/QRDA; public-health eCR;
offline-first data entry.

**Medical device boundary.** Calculators and AI that make diagnostic or therapeutic decisions may be Software as a
Medical Device (UK MDR 2002). Such features stay in a separate module behind a flag, with a regulatory assessment
first (OQ-6).
