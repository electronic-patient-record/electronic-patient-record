## 15. Roadmap

| M | Name | Exit criterion |
|---|---|---|
| M0 | Scaffold | Monorepo, CI green, chapters drafted (this milestone; partly done, see 14) |
| M1 | Identity | Matcher with corpus; patient CRUD/search/`$match`; merge/unmerge; audit on every access; registration UI |
| M2 | Record core | Append-only store; problems, allergies, medications, vitals; summary and banner; FHIR read/search/history |
| M3 | Security | Policy engine; break-the-glass; labels; access log; RLS; ZAP clean |
| M4 | Documentation | Notes, templates, addenda, copy-forward provenance, forms, flowsheets, NEWS2 |
| M5 | Orders and results | CPOE, order sets, HL7 v2 ORU, results inbox, CDS Hooks |
| M6 | Interop and apps | SMART 2.2, CapabilityStatement, Bulk, IPS, Inferno (g)(10) in CI |
| M7 | Care and access | Care pathways and plans first (CA-2), then encounters/ADT, referrals as pathway steps, scheduling, portal, downtime snapshot |
| M8 | Assurance | Hazard log complete; NISTIR 7804 test (SUS ≥ 75); DTAC/DPIA drafts; load test; 1.0 candidate |

**Directions of 2026-11-04 (XP-1 to XP-14, plan.md §27; not yet placed on a milestone).** Mobile viewability starts now because it touches screens already built; setting profiles and specialty packs follow CA-1 and CA-2; clinician pathway design follows the pathway model (M7); homomorphic-encryption analysis is post-1.0 and a spike first.

**After 1.0: emergency triage.** Manchester Triage System work (`TR-1` to `TR-13` in `tasks.md`, plan.md §15) is not
on a milestone. It needs the Encounter and vitals work of M2 and M7, and the pack interface may take more than one
triage system but only one is planned. Decided 2026-10-23 (OQ-T2, OQ-T4). TR-1 (licence) must finish before any
content is built.

