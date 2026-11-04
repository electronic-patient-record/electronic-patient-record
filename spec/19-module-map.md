## 19. Module map

**Proposed 2026-10-24, not yet confirmed by the maintainer** (OQ-M1). The groups come from plan.md §17.2, whose sources
are search summaries and a pasted list, so details marked uncertain there stay uncertain. Nothing here is built. The
column "Slot" is a proposal: **1.0** means already in the 1.0 claim, **M7** means planned before the 1.0 candidate (M8),
**post-1.0** means named so nobody assumes otherwise. A module that is only a form still changes what a clinician sees,
so every module in 1.0 or M7 gets hazard rows (EM-16) before its code.

| Module group | Slot | Tasks | Notes |
|---|---|---|---|
| Clinical documentation | 1.0 | DO-1 to DO-8 | covered |
| Orders and results | 1.0 | OR-1 to OR-6 | covered |
| e-Obs and NEWS2 | 1.0 | DO-5, RC-11 | covered |
| Patient portal | M7 | CA-5 | covered |
| Mobile viewability | now (UI) | XP-1 to XP-4 | see [mobile viewability](mobile-viewability.md) |
| Setting profiles, specialty packs | after CA-1, CA-2 | XP-5 to XP-7, XP-10, XP-11 | OQ-ST1, OQ-SP1 |
| Clinician pathway design | after CA-2 | XP-12 to XP-14 | OQ-CP3 |
| Privacy-preserving analysis | post-1.0 | XP-8, XP-9 | OQ-HE1, DPO |
| Clinical safety measures | now (reports), after AS-1 | SM-1 to SM-5 | OQ-2 |
| Maternity | decision first | MT-1 to MT-4 | OQ-MT1 |
| Bulk import, `$export`, data quality | M2 onward | DX-1 to DX-10 | OQ-DE*, OQ-DQ*, OQ-MS* |
| Training environment and overlay | TO-1 before any demonstration | TO-1 to TO-9 | OQ-TR* |
| Care pathways and plans, referrals, scheduling, ADT | M7 | CA-1 to CA-4 | scheduling follows OQ-PA1 |
| eMAR (closed-loop administration) | M7 | EM-2 | OQ-M3 (build or integrate scanning) |
| Nursing assessments and risk tools | M7 | EM-3 | OQ-M2 (licences) |
| Discharge summary and clinic letters | M7 | EM-8 | PRSB standard uncertain |
| Fluid balance | post-1.0 | EM-4 | simple, but not required for the claim |
| Bed flow | post-1.0 | EM-5 | CA-1 keeps a minimal board |
| Handover list | post-1.0 | EM-13 | |
| Specimen collection and labelling | post-1.0 | EM-6 | wrong blood in tube is a never event; revisit first |
| Imaging link | post-1.0 | EM-7 | link out only, no images stored |
| Virtual consultations | post-1.0 | EM-9 | link out only |
| Perioperative | post-1.0 | EM-10 | |
| Critical care and device feeds | post-1.0 | EM-11 | medical-device territory (OQ-6) |
| Emergency department board and triage | post-1.0 | EM-12, TR-1 to TR-13 | OQ-T2 |
| Maternity, paediatrics, mental health, community | post-1.0 | EM-15 | scope note only |

Mapping to DCF and EMRAM is EM-14 and is buyer evidence, not a claim of compliance.
