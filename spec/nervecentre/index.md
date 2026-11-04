# Nervecentre: comparison with this project

Public-source research read 2026-11-04. Vendor material is marketing; nothing here was verified by using the product; no private documentation was read. Direction only: nothing in this project was built from it.

## What it is

- Nervecentre Software Ltd supplies the Nervecentre platform, an acute-hospital electronic patient record (EPR) that began as a mobile platform for clinical workflows, task management and electronic observations. A sector-advisory article (Nelson Advisors, an M&A advisory firm, not independent research) says it has grown into a full acute EPR covering patient administration, emergency care, clinical noting, electronic prescribing (ePMA), observations, bed management and discharge ([healthcare.digital](https://www.healthcare.digital/single-post/the-first-single-epr-for-primary-and-secondary-care-assessment-of-nervecentre-s-expansion-into-regi)).
- Owner: I could not find a sourced statement of the company's ownership.
- Main market: NHS England acute trusts. Trade press reports ten-year or long contracts at Northampton General, Derby and Burton with Chesterfield Royal, two Yorkshire trusts, East Sussex Healthcare, Mid Yorkshire, United Lincolnshire and Liverpool University Hospitals ([Digital Health, East Sussex](https://www.digitalhealth.net/2025/05/east-sussex-healthcare-signs-10-year-epr-deal-with-nervecentre/); [Digital Health, two Yorkshire trusts](https://www.digitalhealth.net/2024/08/two-yorkshire-trusts-select-nervecentre-as-preferred-epr-supplier); [Liverpool University Hospitals](https://uhliverpool.nhs.uk/about-us/our-stories/liverpool-university-hospitals-appoint-nervecentre-electronic-patient-record-supplier)).
- Scale (vendor or advisory claims only): "second-largest EPR provider by acute bed footprint in England" and "fastest-growing EPR" are claims, not measured here (same healthcare.digital article; [Digital Health, Mid Yorkshire](https://www.digitalhealth.net/2025/12/mid-yorkshire-teaching-nhs-trust-selects-nervecentre-epr/) as reported by search summary).

## Capabilities worth noting

- Electronic observations with automatic early-warning scores and escalation on handheld devices, evaluated at Nottingham ([JMIR Med Inform 2019](https://medinform.jmir.org/2019/1/e11678/PDF)).
- Real-time workflow engine: alerts and task assignment rather than documentation only (advisory article above).
- Clinical photography and electronic observations as an early phase at Mid Yorkshire from November 2025 ([Digital Health](https://www.digitalhealth.net/2025/12/mid-yorkshire-teaching-nhs-trust-selects-nervecentre-epr/)).
- Community use cases (nursing assessments, MUST screening, virtual wards, urgent community response) per the advisory article.
- Shared multi-tenant platform for a group of trusts, e.g. Derby and Burton with Chesterfield Royal ([Digital Health](https://www.digitalhealth.net/2023/11/two-derbyshire-trusts-pick-nervecentre-as-preferred-supplier-for-joint-epr)).

## Architecture and standards

- Described as cloud-native, multi-tenant SaaS with the data layer separated from the clinical applications, running in browsers and on mobile devices, with FHIR-based open APIs and alignment to GP Connect, BaRS and EPS ([healthcare.digital](https://www.healthcare.digital/single-post/the-first-single-epr-for-primary-and-secondary-care-assessment-of-nervecentre-s-expansion-into-regi)). This is an advisory firm's description; I found no public developer documentation for the APIs.
- Nottingham University Hospitals selected it for ePMA and FHIR-based interoperability (search summary of trust and trade-press items; no single primary URL confirmed).
- Data model, source openness and licence: not found. Assume closed commercial software.
- The vendor domain nervecentre.com redirected to a domain-for-sale page when fetched on 2026-11-04, so no vendor page was read.

## Fit across care settings

- Acute inpatient wards: strongest evidence (observations, handover, tasks).
- Emergency department: in the product scope per the advisory article; Nottingham's emergency department was reported as affected after go-live (see below).
- Community: used in integrated acute and community trusts (East Sussex, York and Scarborough per the advisory article); standalone community trusts often use other products, and Leicestershire Partnership was reported evaluating a replacement for Nervecentre point solutions (same article, unverified).
- Mental health: an audit on older-people mental health wards after Nervecentre ([BJPsych Open 2021](https://www.cambridge.org/core/journals/bjpsych-open/article/audit-of-documentation-of-observations-on-mental-health-services-for-older-people-mhsop-wards-following-implementation-of-nervecentre/1D765774A9AAE8A58B5C7A5CBE70C0D3)). Wider mental-health fit: unknown.
- Primary care: the advisory article rates displacing GP systems as moderate-to-low; the path is APIs. Not a GP system.

## What independent sources report

- Positive, peer-reviewed: at Nottingham, a mixed-methods study found unplanned critical-care admissions from electronic-observation wards fell by about 10% and early-warning-score incidents by about 50% on those wards; patient contact time rose (2.9% to 7.3%) ([JMIR Med Inform 2019](https://medinform.jmir.org/2019/1/e11678/PDF)). The study is observational with a short post-deployment window and other improvement work in parallel. Problems it lists: staff stress and workload during rollout, weak senior-doctor engagement, Wi-Fi blackspots, limited out-of-hours ICT support.
- Positive, audit: NEWS documented correctly 100% versus 87% before, on older-people mental health wards ([BJPsych Open 2021](https://www.cambridge.org/core/journals/bjpsych-open/article/audit-of-documentation-of-observations-on-mental-health-services-for-older-people-mhsop-wards-following-implementation-of-nervecentre/1D765774A9AAE8A58B5C7A5CBE70C0D3)); the author attributes this to Nervecentre but it is a single-site audit.
- Full-EPR go-lives (trade press, not the trusts' own papers): Nottingham University Hospitals declared a critical incident after the 3 November 2025 go-live, citing slower than expected performance, and later reported the EPR fully functional ([Digital Health, critical incident](https://www.digitalhealth.net/2025/11/nottingham-hospitals-declares-critical-incident-after-epr-go-live); [Digital Health, resolved](https://www.digitalhealth.net/?p=257787)). York and Scarborough delayed its go-live in 2026 ([Digital Health](https://www.digitalhealth.net/2026/03/york-and-scarborough-delays-nervecentre-epr-go-live/)); Sherwood Forest also delayed elements ([Digital Health](https://www.digitalhealth.net/2026/03/sherwood-forest-hospitals-delays-epr-launch/)). The reasons given are mixed (operational pressure, national approvals), so these are not findings about product quality.
- Regulator or inquiry findings (CQC, HSSIB): none found.

## Compared with this project

| Area | This product | This project today | What we could learn or should avoid |
| --- | --- | --- | --- |
| Observations and early-warning scores | Core strength, on handhelds | Vital-sign observations stored; no scoring or escalation | Escalation rules and task generation are the value, not the chart ([spec/pathway-design.md](../pathway-design.md)) |
| Prescribing, orders, results, notes | Reported in scope (ePMA, noting) | Not built | Nothing to copy yet |
| Beds, ADT, patient flow | Bed management, patient administration reported | Not built; HL7 v2 ADT inbound only | Flow is what trusts buy first |
| Mobile | Handheld first | Not built (planned) | Wi-Fi and device support caused problems at Nottingham ([spec/mobile-viewability.md](../mobile-viewability.md)) |
| Identity | Not found | Matcher with explainable scores; no auto-link on missing evidence | Our differentiator; keep |
| Consent, break-glass, audit | Not found publicly | Built; hash-chained audit | Not comparable on public evidence |
| Referrals | BaRS alignment claimed | Closed-loop referrals, unmatched-reply queue, timeouts | BaRS is a standard worth reading |
| FHIR | FHIR APIs claimed | R4/R5 read facade (three resources) | Public API documentation is something vendors lack and we have |
| Openness | Closed | Open spec and code | Keep spec-first |
| Go-live evidence | Large deployments, some incidents | Scaffold, no clinical record | Do not claim parity |

## Ideas for this project

These are ideas, not decisions.

1. Treat go-live performance as a requirement: write a non-functional entry for emergency-department peak load, drawing on the Nottingham report ([spec/07-non-functional-requirements.md](../07-non-functional-requirements.md)).
2. Make early-warning scoring a pathway-design example: score, threshold, task, escalation, with the clinician able to see why ([spec/pathway-design.md](../pathway-design.md)).
3. In the mobile spec, list the operational dependencies the Nottingham study found (Wi-Fi blackspots, out-of-hours support, permission friction) as acceptance criteria ([spec/mobile-viewability.md](../mobile-viewability.md)).
4. Add BaRS to the interoperability references for the closed-loop referral work.
5. Record in care-settings that one platform serving acute and community is a stated selling point, and note what that implies for episode models ([spec/care-settings.md](../care-settings.md)).

## Not verified

- Ownership, company size, revenue, and every bed or customer count: not found or vendor/advisory only.
- The "second-largest" and "fastest-growing" claims are vendor or advisory claims.
- The advisory article (Nelson Advisors, published July, year inferred as 2026) has unsourced figures.
- Several Digital Health pages returned HTTP 403 when fetched; their content is from search summaries only, and the Nottingham incident details were not read in full.
- Whether the 2019 Nottingham study and trust items concern today's product version: unknown.
- No FHIR conformance statement, API documentation, safety case or DCB0129 evidence was found.

## Sources

- https://www.healthcare.digital/single-post/the-first-single-epr-for-primary-and-secondary-care-assessment-of-nervecentre-s-expansion-into-regi (advisory commentary, scope and architecture)
- https://medinform.jmir.org/2019/1/e11678/PDF (Nottingham mixed-methods evaluation)
- https://www.cambridge.org/core/journals/bjpsych-open/article/audit-of-documentation-of-observations-on-mental-health-services-for-older-people-mhsop-wards-following-implementation-of-nervecentre/1D765774A9AAE8A58B5C7A5CBE70C0D3 (mental-health ward audit)
- https://www.digitalhealth.net/2025/11/nottingham-hospitals-declares-critical-incident-after-epr-go-live (go-live incident)
- https://www.digitalhealth.net/?p=257787 (Nottingham EPR fully functioning)
- https://www.digitalhealth.net/2026/03/york-and-scarborough-delays-nervecentre-epr-go-live/ (delay)
- https://www.digitalhealth.net/2026/03/sherwood-forest-hospitals-delays-epr-launch/ (delay)
- https://www.digitalhealth.net/2025/05/east-sussex-healthcare-signs-10-year-epr-deal-with-nervecentre/ (contract)
- https://www.digitalhealth.net/2024/08/two-yorkshire-trusts-select-nervecentre-as-preferred-epr-supplier (contract)
- https://www.digitalhealth.net/2023/11/two-derbyshire-trusts-pick-nervecentre-as-preferred-supplier-for-joint-epr (shared platform)
- https://www.digitalhealth.net/2025/12/mid-yorkshire-teaching-nhs-trust-selects-nervecentre-epr/ (contract)
- https://uhliverpool.nhs.uk/about-us/our-stories/liverpool-university-hospitals-appoint-nervecentre-electronic-patient-record-supplier (trust announcement)
