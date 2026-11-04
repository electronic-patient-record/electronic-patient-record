# System C: comparison with this project

Public-source research read 2026-11-04. Vendor material is marketing; nothing here was verified by using the product; no private documentation was read. Direction only: nothing in this project was built from it.

## What it is

- System C Healthcare Ltd is a UK supplier of health and social care software. Wikipedia gives this ownership history: founded 1983; floated on AIM in 2005; acquired by McKesson in 2011; sold to Symphony Technology Group in 2014; acquired by CVC Capital Partners in 2021 ([Wikipedia](https://en.wikipedia.org/wiki/System_C); the 2011 McKesson deal is also in [Digital Health](https://www.digitalhealth.net/2011/05/mckesson-completes-purchase-of-system-c/), the 2021 CVC investment in [Digital Health](https://www.digitalhealth.net/2021/02/private-equity-firm-cvc-capital-partners-invests-in-system-c/)).
- Products today (vendor): CareFlow EPR, CareFlow Medicines Management (pharmacy, EPMA), ChemoCare, BadgerNet Maternity and Neonatal, CarePlus Child Health, Liquidlogic social care and education systems, and AI products ([systemc.com/about-us](https://www.systemc.com/about-us/)).
- Medway: System C's Medway EPR was rebranded CareFlow in 2022, as part of a stated five-year investment programme ([Digital Health, 2022](https://digitalhealth.net/2022/02/system-c-ics-epr)). Older trust and press material still says Medway.
- Graphnet: a separate UK company (shared care records, CareCentric), founded 1994, with CVC as an investor in both and a joint "System C & Graphnet Care Alliance" ([Wikipedia](https://en.wikipedia.org/wiki/Graphnet)). The 2014 Digital Health article describes them as separate companies with shared investors ([Digital Health, 2014](https://digitalhealth.net/2014/11/system-c-launches-carecentric-for-medway)). Whether Graphnet is now a subsidiary of System C: I could not confirm.
- Wellbeing Software: I could not establish any ownership link to System C. Search results showed it as a separate company with conflicting reports of its owner.
- Acquisitions listed: Care Records (2008), Liquidlogic (2009), Careflow Connect (2016), WellSky International (2021), Clevermed (2023), Oxford Computer Consultants (2023), CIS Oncology (2024), MYP Technologies (August 2025, [System C release](https://www.systemc.com/system-c-healthcare-acquires-myp-technologies-to-drive-international-growth-in-health-and-social-care/)).
- Sale process: Nelson Advisors (an M&A adviser, April 2026, updated September) reports, citing a Mergermarket item of April 2026, that CVC is selling System C with Arma Partners advising ([healthcare.digital](https://www.healthcare.digital/single-post/system-c-potential-acquirers)). No completed sale was found as of 2026-11-04. The same article's financial figures are weakly sourced and internally inconsistent.
- Scale (vendor): 25 deployed trusts and 20K inpatient beds on the CareFlow page; "80% of the UK oncology market" and "99.9% of UK neonatal sites" on the about page ([systemc.com/solutions/epr](https://www.systemc.com/solutions/epr/start); [about](https://www.systemc.com/about-us/)).

## Capabilities worth noting

- CareFlow Clinicals: ePMA, eObservations, order comms and results, clinical noting, plan of care. Patient administration: scheduling, capacity, waiting lists and RTT ([systemc.com](https://www.systemc.com/solutions/epr/start), vendor).
- Specialist depth: chemotherapy (ChemoCare), maternity and neonatal (BadgerNet), medicines management (vendor, same sources).
- Integrated health and social care under one supplier (Liquidlogic with CareFlow), which few competitors offer.
- CareFlow Connect for mobile use; the page says "device-agnostic" (vendor).

## Architecture and standards

- Vendor: "FHIR-compliant", fully cloud-hosted, NHS data standards (CDS 6.3, ECDS, SNOMED CT, dm+d, ICD-10, OPCS-4) ([systemc.com](https://www.systemc.com/solutions/epr/start)). A search summary of the same vendor material says HL7 v2 and FHIR are supported side by side and GP Connect is used for GP data; the page I read did not mention them. I found no conformance statement or public API documentation.
- G-Cloud listings exist ([Digital Marketplace](https://www.applytosupply.digitalmarketplace.service.gov.uk/g-cloud-14/services/687125563583525)); not read in full.
- Source openness: closed commercial software. Certifications listed by the vendor include ISO 27001 and ISO 9001 ([about](https://www.systemc.com/about-us/)).

## Fit across care settings

- Acute inpatient, ED, theatres, outpatients and pharmacy: vendor-stated (CareFlow page).
- Community and ICS shared care: vendor-stated; Graphnet is the care-record side.
- Maternity and neonatal: strongest, specialist products.
- Mental health: not found on the pages read. Unknown.
- Primary care: not a GP system; unknown beyond that.
- Social care: strong, via Liquidlogic (vendor: 60% of councils use its adults or children's solution).

## What independent sources report

- Royal Wolverhampton went live with CareFlow in September 2025; System C says on time and within budget, while the trust said processes were slower as staff familiarised themselves ([Digital Health, October 2025](https://www.digitalhealth.net/2025/10/royal-wolverhampton-goes-live-with-system-cs-careflow-epr); [HTN, December 2025](https://htn.co.uk/2025/12/11/royal-wolverhampton-shares-post-epr-go-live-update-and-phase-two-plans-for-2026/)). I read these through search summaries only.
- Robert Jones and Agnes Hunt (a System C EPR, reported as the Apollo product, not CareFlow) reported ongoing issues finding clinical documents a year after its May 2025 go-live; System C said it is working with the trust ([Digital Health, March 2026](https://www.digitalhealth.net/2026/03/robert-jones-and-agnes-hunt-experiences-issues-after-epr-go-live/)).
- Barking, Havering and Redbridge, on Medway, said it went live before the system was ready and paused further go-lives (older coverage, undated in my results; attribute to Medway, not CareFlow).
- HSJ commentary ([HSJ](https://www.hsj.co.uk/technology-and-innovation/why-you-cant-build-an-epr-on-sand/7037382.article), paywalled) says some organisations struggle for years after go-live; I did not read it fully.
- Regulator findings, peer-reviewed safety evaluation of CareFlow: none found.

## Compared with this project

| Area | This product | This project today | What we could learn or should avoid |
| --- | --- | --- | --- |
| Scope | Full acute EPR, pharmacy, maternity, social care | Allergies, observations, amendments, referrals | Breadth is years of work; we should stay narrow and correct |
| Medicines | EPMA, chemotherapy | Not built | Medicines safety needs its own hazard analysis |
| Orders and results | Order comms and results | Not built | Closed-loop pattern may transfer |
| Care records across organisations | Graphnet shared care records | Consent and care-relationship access, break-glass | Our access model suits shared records |
| Identity | Not found | Matcher with explainable scores | Differentiator |
| Audit | Not found publicly | Hash-chained, retention and archive | Differentiator |
| Standards | FHIR claimed; NHS data standards | FHIR R4/R5 read facade, HL7 v2 ADT | Publish a conformance statement; vendors do not |
| Mobile | Connect app, device-agnostic | Not built | See [spec/mobile-viewability.md](../mobile-viewability.md) |
| Ownership churn | Four owners since 2011, sale reported | Open, no owner lock-in | Continuity is a real buyer risk |

## Ideas for this project

These are ideas, not decisions.

1. Treat go-live as a phased programme: record in the roadmap that Wolverhampton's trust reported slower early processes and plan for a measured slow start ([spec/15-roadmap.md](../15-roadmap.md)).
2. For care-setting models, note that one supplier covers health and social care records; add an open question on whether social-care actors fit the consent model ([spec/care-settings.md](../care-settings.md)).
3. Specialty views: maternity, neonatal and oncology are separate products here; use that as evidence that specialty views need their own data shapes ([spec/pathway-design.md](../pathway-design.md)).
4. Document retrieval: the Robert Jones issue (finding documents) suggests testing document naming and search early.
5. Write a conformance statement for the FHIR facade, since none was found from this vendor.

## Not verified

- Current ownership of System C after the reported sale process; no completion found by 2026-11-04.
- Whether Graphnet is a subsidiary or only a sister company; the Wikipedia and 2014 sources are dated.
- Any link between System C and Wellbeing Software: none found.
- Vendor figures (25 trusts, 20K beds, market shares) are marketing and mix products.
- Digital Health pages returned HTTP 403, so several items rely on search summaries.
- The Medway to CareFlow rename is from a 2022 article; which trusts run CareFlow today was not checked.
- Wikipedia's article has inconsistent location details and some odd retrieval dates; treat it as a lead.
- Nothing on clinical safety evaluations, usability studies or DCB0129 evidence was found.
- Link check 2026-11-04: these cited pages returned 404. They may have moved, but what rests on them is unconfirmed: https://www.applytosupply.digitalmarketplace.service.gov.uk/g-cloud-14/services/687125563583525

## Sources

- https://en.wikipedia.org/wiki/System_C (ownership timeline, secondary)
- https://en.wikipedia.org/wiki/Graphnet (Graphnet, CVC, Care Alliance)
- https://www.systemc.com/about-us/ (vendor, products and acquisitions)
- https://www.systemc.com/solutions/epr/start (vendor, CareFlow modules and claims)
- https://www.systemc.com/system-c-healthcare-acquires-myp-technologies-to-drive-international-growth-in-health-and-social-care/ (vendor, August 2025)
- https://digitalhealth.net/2022/02/system-c-ics-epr (CareFlow launch, rebrand)
- https://digitalhealth.net/2014/11/system-c-launches-carecentric-for-medway (2014, Graphnet separate)
- https://www.digitalhealth.net/2021/02/private-equity-firm-cvc-capital-partners-invests-in-system-c/ (CVC, 2021)
- https://www.digitalhealth.net/2011/05/mckesson-completes-purchase-of-system-c/ (McKesson, 2011)
- https://www.healthcare.digital/single-post/system-c-potential-acquirers (advisory, sale process, April 2026)
- https://www.digitalhealth.net/2025/10/royal-wolverhampton-goes-live-with-system-cs-careflow-epr (go-live)
- https://htn.co.uk/2025/12/11/royal-wolverhampton-shares-post-epr-go-live-update-and-phase-two-plans-for-2026/ (trust update)
- https://www.digitalhealth.net/2026/03/robert-jones-and-agnes-hunt-experiences-issues-after-epr-go-live/ (post-go-live issues)
- https://www.hsj.co.uk/technology-and-innovation/why-you-cant-build-an-epr-on-sand/7037382.article (HSJ commentary, paywalled)
- https://www.applytosupply.digitalmarketplace.service.gov.uk/g-cloud-14/services/687125563583525 (G-Cloud listing)
