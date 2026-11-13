# EMIS: comparison with this project

Public-source research read 2026-11-04. Vendor material is marketing; nothing here was verified by using the product; no private documentation was read. Direction only: nothing in this project was built from it.

## What it is

- EMIS Health, maker of EMIS Web (GP and community clinical system), EMIS-X (a cloud-based next-generation platform), EMIS-X Mobile and Patient Access (patient app). Sources: [EMIS EMIS-X page](https://www.emishealth.com/emis-web/emis-x), [Patient Access](https://www.emishealth.com/patient-access), [EMIS-X Mobile](https://www.emishealth.com/news-insights/emis-x-mobile-transforming-healthcare-anytime-anywhere).
- Ownership has changed twice. EMIS Group plc was bought by Optum (UnitedHealth Group) for about 1.2 billion pounds, completed 25 October 2023 after a Competition and Markets Authority review ([Management in Practice](https://managementinpractice.com/news/emis-1-2bn-takeover-by-us-healthcare-giant-has-completed/)). In 2026 UnitedHealth sold Optum UK, which holds EMIS, to the US private-equity firm TPG ([Digital Health, May 2026](https://www.digitalhealth.net/2026/05/gp-it-supplier-optum-acquired-by-us-equity-firm-tpg/); [Becker's Payer](https://www.beckerspayer.com/m-and-a/unitedhealth-completes-sale-of-uk-business-to-tpg/)). Sources disagree on the completion date (13 March or about 23 April 2026) and on the price; I could not reconcile them, and could not open the primary pages.
- Scale: EMIS Web is reported as used by more than half of English GP practices, about 60 per cent and about 3,700 practices in news of the July 2024 outage ([Digital Health, Nov 2024](https://digitalhealth.net/2024/11/gps-face-emis-it-outage-at-busiest-time-of-the-week); [ITV](https://itv.com/news/2024-07-19/thousands-of-gp-surgeries-in-england-hit-by-global-it-outage)). Treat the percentage as press estimate.
- Settings: primary care, community (EMIS Web Community, G-Cloud listing), a Community, Child and Mental Health variant, and independent providers ([G-Cloud entry](https://www.applytosupply.digitalmarketplace.service.gov.uk/g-cloud/services/260487589376653); [consultancy notes](https://jamesharleyconsultancy.freshdesk.com/support/solutions/articles/101000480201-emis-web-for-independent-healthcare-providers)).

## Capabilities worth noting

- Patient Access: a patient-facing app described by the vendor as a "digital front door" for practices on EMIS Web ([EMIS](https://www.emishealth.com/patient-access)).
- EMIS-X Mobile: a cloud-hosted app that syncs with EMIS Web; one trust case study reports halved sync times (vendor material, [EMIS](https://www.emishealth.com/news-insights/emis-x-mobile-transforming-healthcare-anytime-anywhere)).
- GP Connect: read-only record access, appointment management and structured medications and allergies ([EMIS GP Connect](https://www.emishealth.com/products/emis-web/gp-connect)).
- Data sharing agreements between practice and PCN hub for record and appointment-book sharing (consultancy guide: [EMIS Web data sharing agreements](https://jamesharleyconsultancy.freshdesk.com/support/solutions/articles/101000433916-emis-web-clinical-services-data-sharing-agreements)).
- Transfer-of-care messages, including community mental-health letters, sent to GPs ([EMIS interoperability overview, consultancy](https://jamesharleyconsultancy.freshdesk.com/support/solutions/articles/101000477044-emis-interoperability-overview)).

## Architecture and standards

- EMIS Web is a hosted system; EMIS-X is the cloud successor, and EMIS says it will progressively extend EMIS Web (vendor: [EMIS-X](https://www.emishealth.com/emis-web/emis-x)). Whether and when EMIS Web is retired: a search summary said "phased retirement"; I could not confirm it from a primary page.
- Record access requests use FHIR via GP Connect (consultancy overview above; [EMIS GP Connect](https://www.emishealth.com/products/emis-web/gp-connect)). Third-party access runs through the IM1 pairing of England's national health body ([England's national health body](https://digital.nhs.uk/developer/api-catalogue/interface-mechanism-1-standards)).
- Closed, proprietary product. Data model and public API detail: not found.
- Terminology (SNOMED CT or Read codes) in current versions: not verified.

## Fit across care settings

- Primary care: the core market.
- Community: a named product and G-Cloud listing; depth against community caseload and visit-round needs: not found.
- Mental health: variants and letters exist, but I found no independent source on Mental Health Act handling in EMIS products.
- Acute and ED: no evidence found in the sources read; I could not confirm any acute footprint.

## What independent sources report

Positive or neutral:
- The CMA examined the Optum acquisition in depth and cleared it ([Management in Practice](https://managementinpractice.com/news/emis-1-2bn-takeover-by-us-healthcare-giant-has-completed/)).
- Broad interoperability role through GP Connect and the PCN hub model (vendor and consultancy sources only).

Problems reported:
- 19 July 2024: a global IT outage reached EMIS Web ([ITV](https://itv.com/news/2024-07-19/thousands-of-gp-surgeries-in-england-hit-by-global-it-outage)).
- 4 November 2024: an EMIS outage removed appointment booking and record access across England; a GP quoted called it a patient-safety issue ([Digital Health](https://digitalhealth.net/2024/11/gps-face-emis-it-outage-at-busiest-time-of-the-week)).
- January 2026: a major IT failure in Kent and Medway left many surgeries without records or medication histories; whether EMIS was the cause: not stated in what I read ([Digital Health](https://www.digitalhealth.net/2026/01/major-it-failure-impacts-kent-and-medway-gp-practices/)).
- April 2026: a Merseyside trust logged a patient-safety incident when discharge information did not reach GP practices via EMIS ([Mersey Care](https://yourspace.merseycare.nhs.uk/clinical/community-news/qpa-emis-outbound-documents)).
- A 2021 coroner's report concerned an EMIS appointment-list fault ([EMIS response](https://www.judiciary.uk/wp-content/uploads/2021/03/2021-0061-Response-from-EMIS-Redacted.pdf)). EMIS also faced possible payments over GP reporting failures ([Pulse](https://www.pulsetoday.co.uk/news/technology/emis-may-have-to-pay-millions-due-to-gp-reporting-failures/)), where EMIS and England's former national health data body said no safety or data risk was found.

## Compared with this project

| Area | This product | This project today | What we could learn or should avoid |
|---|---|---|---|
| Ownership and openness | Private equity owner (2026), closed | Open spec and code | Spec-first openness lowers lock-in risk |
| Standards | FHIR via GP Connect, IM1 | FHIR R4 and R5 read facade for three resources | Learn: read-only first, then structured medications and allergies |
| Patient access | Patient Access app, mobile app | Patient requests corrections only; portal not built | Plan a portal on the existing amendment path |
| Sharing between sites | Data sharing agreements per PCN hub | Consent and care-relationship access, break-glass | Record the agreement a share relies on |
| Mobile | EMIS-X Mobile syncs with EMIS Web | No mobile layout | Sync-latency is the measurable claim; define ours |
| Resilience | Outages in 2024 and 2026 | Single scaffold, no operations record | Specify a degraded mode (read-only, paper fallback) |
| Referrals and letters | Letters to GPs; one logged non-delivery | Closed-loop referrals, unmatched-reply queue, timeouts | Our closed loop answers the non-delivery case; keep it |
| Mental health | Letters and variants; MHA not found | Not built | Same as other vendors: a pack |

## Ideas for this project

These are ideas, not decisions.
1. Document a downtime mode: what a clinician sees when the service is unreachable, tied to non-functional requirements (spec/07).
2. Extend the closed-loop referral model to discharge and transfer-of-care messages so a non-delivery becomes a visible queue item, as the Merseyside incident shows.
3. When a portal is specified, start from the existing amendment request path and keep GP Connect-style read-only first.
4. Specify a measurable sync target for any mobile queue (time to reconcile after reconnect).
5. Model record-sharing agreements as data attached to a consent decision, for the care-settings profiles.

## Not verified

- Exact TPG completion date and price; I could not open the Digital Health or Becker's pages (access refused) and relied on search summaries.
- Current practice count; the 60 per cent and 3,700 figures are press estimates.
- EMIS-X versus EMIS Web retirement, SNOMED use, acute or MHA capability.
- Whether the January 2026 Kent and Medway failure involved EMIS.
- Vendor pages were read only through search summaries.
- Link check 2026-11-04: these cited pages returned 404. They may have moved, but what rests on them is unconfirmed: https://www.applytosupply.digitalmarketplace.service.gov.uk/g-cloud/services/260487589376653

## Sources

- https://managementinpractice.com/news/emis-1-2bn-takeover-by-us-healthcare-giant-has-completed/ : Optum acquisition
- https://www.digitalhealth.net/2026/05/gp-it-supplier-optum-acquired-by-us-equity-firm-tpg/ : TPG purchase (title seen)
- https://www.beckerspayer.com/m-and-a/unitedhealth-completes-sale-of-uk-business-to-tpg/ : sale to TPG (title seen)
- https://digitalhealth.net/2024/11/gps-face-emis-it-outage-at-busiest-time-of-the-week : November 2024 outage
- https://itv.com/news/2024-07-19/thousands-of-gp-surgeries-in-england-hit-by-global-it-outage : July 2024 outage
- https://www.digitalhealth.net/2026/01/major-it-failure-impacts-kent-and-medway-gp-practices/ : Kent and Medway
- https://yourspace.merseycare.nhs.uk/clinical/community-news/qpa-emis-outbound-documents : Mersey Care incident
- https://www.judiciary.uk/wp-content/uploads/2021/03/2021-0061-Response-from-EMIS-Redacted.pdf : coroner response
- https://www.pulsetoday.co.uk/news/technology/emis-may-have-to-pay-millions-due-to-gp-reporting-failures/ : reporting failures
- https://www.emishealth.com/emis-web/emis-x : vendor EMIS-X
- https://www.emishealth.com/patient-access : vendor patient app
- https://www.emishealth.com/news-insights/emis-x-mobile-transforming-healthcare-anytime-anywhere : vendor mobile
- https://www.emishealth.com/products/emis-web/gp-connect : vendor GP Connect
- https://www.applytosupply.digitalmarketplace.service.gov.uk/g-cloud/services/260487589376653 : G-Cloud community listing
- https://jamesharleyconsultancy.freshdesk.com/support/solutions/articles/101000477044-emis-interoperability-overview : consultancy overview
- https://jamesharleyconsultancy.freshdesk.com/support/solutions/articles/101000433916-emis-web-clinical-services-data-sharing-agreements : consultancy on sharing agreements
- https://jamesharleyconsultancy.freshdesk.com/support/solutions/articles/101000480201-emis-web-for-independent-healthcare-providers : consultancy on variants
- https://digital.nhs.uk/developer/api-catalogue/interface-mechanism-1-standards : IM1, England's national health body
