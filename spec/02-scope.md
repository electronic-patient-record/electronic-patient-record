## 2. Scope

**In scope for 1.0** (ambulatory first, inpatient core next): patient identity and registration; summary and banner;
problems, allergies, medications, immunisations, histories, vitals and flowsheets; encounters and basic ADT; notes,
templates and structured forms; orders and results with closed-loop follow-up; prescribing with interaction checks via
an external CDS service; care plans, teams, tasks, referrals; scheduling; patient portal; audit, consent,
break-the-glass; FHIR R4 API, SMART App Launch, CDS Hooks client, HL7 v2 inbound.

**Reach.** The product is for use worldwide. 1.0 ships the base FHIR R4 API plus the UK Core and US Core packs, because
those are the ones this project can test today; every other country is added as a pack (IO-18) and is not claimed until
its pack validates. Regulatory text that names one country (for example UK MDR 2002 below, DCB0129, HIPAA) is that
country's rule, not a universal one; a deployer in another country applies their own.

**Out of scope for 1.0**: billing and claims (X12, NHS PbR); a licensed drug knowledge base (we integrate one);
e-prescribing network certification (Surescripts, EPS) and EPCS; PACS (link via ImagingStudy/DICOMweb); LIS/RIS
internals; eMAR with barcode scanning (planned M7, EM-2; see the [module map](19-module-map.md)); ONC certification itself; eCQM/QRDA; public-health eCR;
offline-first data entry; emergency department triage (Manchester Triage System or another), decided post-1.0 on
2026-10-23 (OQ-T2). The data model stays compatible: triage would be append-only assessments on an Encounter. The
design takes a pluggable, versioned triage pack so that more than one system can be loaded, and only one is planned
(OQ-T4). No triage content is in the repository until its licence is settled (OQ-T1).

**Boundary with the Patient Administration System (PAS)** (OQ-PA1, **proposed 2026-10-23, awaiting the maintainer**). The
PAS is a separate, administrative system (appointments, waiting lists and RTT clocks, reminders and letters, referral
administration, transport) that holds only opaque person ids. The EPR must also run without it, so it keeps a minimal
scheduler behind a **scheduling port** that the PAS can implement.

| Area | EPR owns | PAS owns when present | Meeting point |
|---|---|---|---|
| Clinical record, `Encounter`, orders, results, notes | all | nothing | PAS check-in opens or updates the EPR `Encounter` (PA-8) |
| Appointments, slots, schedules | a minimal scheduler (CA-4) used only without a PAS | all of it | the scheduling port; the EPR shows the age of what it displays |
| Waiting lists, RTT clocks, clinical priority dates | display only (PA-15) | all | read through the port |
| Referrals | the clinical request and its closed loop (CA-3) | administrative status, triage outcome, redirects | external reference on both sides |
| Reminders, letters delivery, transport | none | all | the outbound choke point (PA-13) applies to the EPR's own messages |
| Patient portal | clinical content: results, letters, messages, record download, proxy access to the record (CA-5) | booking, rebooking and reply links | the portal reads appointments through the port |
| ADT and bed occupancy | the clinical admission and a minimal bed board (CA-1) | administrative ADT events where it runs | HL7 ADT inbound (OR-3) |
| Person identity | the matcher and MPI (ID-*) | an opaque `person_id` from `person-service` | the identity link (PA-2) |

Until the maintainer confirms, nothing here is built, and the CA items keep their text with a note.

**Medical device boundary.** Calculators and AI that make diagnostic or therapeutic decisions may be Software as a
Medical Device (UK MDR 2002). Such features stay in a separate module behind a flag, with a regulatory assessment
first (OQ-6).
