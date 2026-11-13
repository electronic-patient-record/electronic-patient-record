# MEDITECH Expanse: comparison with this project

Public-source research read 2026-11-04. Vendor material is marketing; nothing here was verified by using the product; no private documentation was read. Direction only: nothing in this project was built from it.

## What it is

- Medical Information Technology, Inc. (MEDITECH) was founded in 1969 in Massachusetts and is privately held, based in Westwood, Massachusetts. Michelle O'Connor has been president and CEO since 2021 ([Wikipedia](https://en.wikipedia.org/wiki/Meditech)).
- Products: MAGIC (1982) and Client/Server (1994) are the older platforms; Expanse, a web-based EHR, launched in 2018 ([Wikipedia](https://en.wikipedia.org/wiki/Meditech)). MEDITECH as a Service (MaaS) is a hosted subscription option, and the cloud offering is built on Google Cloud ([MEDITECH](https://ehr.meditech.com/ehr-solutions/meditech-expanse); [Business Wire](https://www.businesswire.com/news/home/20201203005080/en/MEDITECH-Selects-Google-Cloud-to-Power-EHR-Solutions), vendor).
- Markets: mainly US community and mid-sized hospitals; offices are listed in Canada, the UK, Ireland, South Africa, Australia and elsewhere ([Wikipedia](https://en.wikipedia.org/wiki/Meditech)).
- Scale: KLAS (via trade press) put MEDITECH at 14.7% of US acute hospitals and 12.5% of beds in 2026 ([HIT Consultant](https://hitconsultant.net/2026/05/14/klas-2026-ehr-market-share-report/); read through a search summary). Reported 2019 revenue was $493.8 million (Wikipedia).

## Capabilities worth noting

- Care settings listed by MEDITECH: acute, ambulatory, critical care, emergency department, home care and hospice, labour and delivery, mental health, oncology, post-acute, surgical services, therapies and virtual care ([MEDITECH](https://ehr.meditech.com/ehr-solutions/meditech-expanse), vendor).
- Mobile-first claim: MEDITECH cites a customer saying over 75% of its nurses use EHR-integrated smartphones (vendor customer claim, same page).
- AI features claimed: ambient capture of conversations into documentation, generated hospital-course summaries and nurse handoffs; a customer is quoted cutting discharge documentation from 12.5 to 5.5 minutes per patient (vendor, same page, not independently checked).
- Expanse Navigator is described as built on Google Cloud, summarising structured and unstructured data ([Nirmitee guide](https://nirmitee.io/blog/meditech-expanse-guide/), secondary).
- A Greenfield Workspace for developers ([MEDITECH Greenfield](https://ehr.meditech.com/meditech-greenfield), vendor).

## Architecture and standards

- A third-party source says US Core FHIR R4 APIs with SMART standalone launch and OAuth 2.0 are offered, with a sandbox giving view-only patient-facing data ([Medblocks](https://medblocks.com/docs/patient-access/meditech), secondary). Another notes scheduling APIs are limited to Expanse customers ([Supergood](https://supergood.ai/docs/meditech-api), secondary).
- Hosting: customer-hosted or MaaS on Google Cloud (vendor).
- Data model: Wikipedia traces the original system to MUMPS-derived technology (MIIS, MAGIC). I did not find a primary statement of the current Expanse data model.
- Openness: closed source; developer access via Greenfield.

## Fit across care settings

- Acute and small or rural hospitals: the main footprint. MEDITECH says fifteen rural hospitals chose Expanse in 2025 (vendor, via a search summary).
- ED, mental health, home care, ambulatory: listed by MEDITECH. Named customers on its page include Silver Hill Hospital (psychiatric) and Willis Knighton (132 clinics) (vendor).
- Primary care outside hospital-owned clinics, and community services in the UK health service's sense: unknown from my sources.

## What independent sources report

Positive or neutral:
- KLAS ranked MEDITECH in the top two for US market-share growth in 2019 to 2021 in releases quoted by the vendor ([Business Wire, 2022](https://www.businesswire.com/news/home/20220426005144/en/KLAS-Ranks-MEDITECH-Among-the-Top-Two-EHR-Vendors-for-Market-Share-Growth)); this is a vendor press release about KLAS, not the KLAS report.
- MEDITECH announced a twelfth consecutive Best in KLAS in 2026 ([Business Wire](https://businesswire.com/news/home/20260204264955/en/MEDITECH-Secures-Best-in-KLAS-for-a-12th-Consecutive-Year)); the segment and basis were not read, so vendor-reported.

Problems or limits:
- A 2019 KLAS comparison reported Expanse lagging EpicCare Inpatient in customer satisfaction ([HIT Consultant, 2019](https://hitconsultant.net/2019/08/05/klas-cerner-meditech-expanse-lags-behind-epiccare)); dated, read through a search listing only.
- I found no independent usability study, inquiry report or safety finding specific to Expanse. That is an absence in my search, not evidence of absence.
- Market share trend: a 2026 summary says Epic gained and others lost hospitals; the MEDITECH 2025 net change was not retrieved.

## Compared with this project

| Area | This product | This project today | What we could learn or should avoid |
| --- | --- | --- | --- |
| Delivery model | Hosted subscription for smaller hospitals | Self-hosted containers | Small sites need an easy hosted path; record what a minimum deployment needs |
| Care settings | Acute through home care, vendor-listed | Scaffold; settings in spec | Our care-settings spec is the plan; do not claim fit before evidence |
| Mobile | Mobile-first claim, nurse smartphones | Mobile layout not built | Treat nurse mobile use as a requirement set, with measures (spec: mobile) |
| AI documentation | Ambient notes and summaries (vendor) | Notes not built | Any generated summary needs source links and clinician sign-off |
| APIs | US Core R4, SMART, sandbox | R4/R5 read facade, three resources | Publish a sandbox and conformance statement early |
| Audit and consent | Not found in public sources I read | Hash-chained audit, break-glass, consent | Strength to keep visible |

## Ideas for this project

These are ideas, not decisions.
1. Write a "smallest hospital" deployment profile (one node, hosted option) and its costs, as the market evidence suggests small sites are where simple systems win (spec: scope).
2. Define nurse-on-phone tasks (vitals, allergy check) as the first mobile layout, with a measured target (spec: mobile).
3. If summaries are ever added, require each sentence to link to its source record, and mark machine-written text, as the vendor claim is unverified (spec: notes).
4. Offer a public sandbox with synthetic patients and a conformance statement for the FHIR facade.
5. Use the vendor's care-setting list as a checklist for gaps in our care-settings spec, not as proof of fit.

## Not verified

- KLAS share and award claims are vendor releases or search summaries; I read no KLAS report.
- Customer stories (Silver Hill, Willis Knighton, Fraser Health, Bingham) are vendor claims.
- Sources on FHIR scope and Navigator are third-party blogs and directories.
- A claim in one guide that MEDITECH was named a Gartner leader in 2026 was not confirmed and is not used.
- No independent usability or safety study of Expanse was found.
- The 2019 KLAS comparison is dated.

## Sources

- https://en.wikipedia.org/wiki/Meditech : history, products, ownership
- https://ehr.meditech.com/ehr-solutions/meditech-expanse : Expanse overview (vendor)
- https://ehr.meditech.com/meditech-greenfield : developer workspace (vendor)
- https://www.businesswire.com/news/home/20201203005080/en/MEDITECH-Selects-Google-Cloud-to-Power-EHR-Solutions : Google Cloud (vendor)
- https://www.businesswire.com/news/home/20220426005144/en/KLAS-Ranks-MEDITECH-Among-the-Top-Two-EHR-Vendors-for-Market-Share-Growth : KLAS growth (vendor release)
- https://businesswire.com/news/home/20260204264955/en/MEDITECH-Secures-Best-in-KLAS-for-a-12th-Consecutive-Year : Best in KLAS (vendor release)
- https://hitconsultant.net/2026/05/14/klas-2026-ehr-market-share-report/ : KLAS 2026 share summary
- https://hitconsultant.net/2019/08/05/klas-cerner-meditech-expanse-lags-behind-epiccare : 2019 KLAS comparison
- https://medblocks.com/docs/patient-access/meditech : third-party FHIR notes
- https://supergood.ai/docs/meditech-api : third-party API notes
- https://nirmitee.io/blog/meditech-expanse-guide/ : third-party guide
