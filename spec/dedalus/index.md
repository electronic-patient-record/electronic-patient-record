# Dedalus: comparison with this project

Public-source research read 2026-11-04. Vendor material is marketing; nothing here was verified by using the product; no private documentation was read. Direction only: nothing in this project was built from it.

## What it is

- **Company.** Dedalus Group (Dedalus Healthcare), founded in Florence in 1982. Its own site says it has more than 7,700 staff in 40 countries, and a 2025 press release cites 7,500 healthcare organisations and 5,700 laboratory and diagnostic clients (figures found via search of [dedalus.com](https://www.dedalus.com/global/en/about-us/the-geographical-footprint/) and a [2025 release](https://www.dedalus.com/global/wp-content/uploads/sites/9/2025/07/Press-Release-Dedalus-and-ENTOMO-review-2-1-1.pdf); vendor claims).
- **Ownership.** Press and deal databases report that the private-equity firm Ardian took a majority stake in 2016, raised it later, and sold a minority to ADIA in 2021 ([Bluerating](https://www.bluerating.com/?p=145585), [BeBeez](https://bebeez.it/private-equity/adia-rileva-la-minoranza-di-dedalus-da-ardian-che-resta-azionista-di-maggioranza/)). Those are 2016 to 2023 records; I found no 2025 or 2026 ownership report.
- **DXC business.** Dedalus agreed in July 2020 to buy DXC Technology's health business for GBP 413 million (about US$525 million), covering ORBIS and integrated-care activities in Germany, Austria, Switzerland, France and Brazil, plus the NHS Lorenzo footprint; completion was reported in April 2021 ([Digital Health 2020](https://www.digitalhealth.net/2020/07/dedalus-buys-dxcs-health-business/), [Digital Health 2021](https://digitalhealth.net/2021/04/dxc-technology-completes-sale-of-health-business-to-dedalus)).
- **Products.** ORBIS and ORBIS U (hospital EHR, mainly DACH and now UK and Ireland), Lorenzo (originally CSC, built for the NHS National Programme for IT; the iSoft line came to DXC/CSC in 2011), and a newer open platform called DC4H ([dedalus.com FAQ](https://www.dedalus.com/global/en/faq/solutions)).
- **Lorenzo today.** In March 2022 Dedalus announced it would stop offering Lorenzo in the UK and Ireland and move customers to ORBIS U, keeping the Lorenzo Care Manager module ([Digital Health via Pulse IT](https://pulseit.news/?p=21317)). Procurement notices from 2025 and 2026 show Dedalus still being direct-awarded EPR support, so Lorenzo-era systems remain in use; I could not confirm which trusts ([Find a Tender 2025](https://www.find-tender.service.gov.uk/Notice/062374-2025/PDF)).

## Capabilities worth noting

- **NHS ORBIS U wins.** East and North Hertfordshire was reported as the first UK trust to adopt ORBIS U; NHS Humber Health Partnership selected ORBIS U in January 2026 (search summaries; I did not open the articles).
- **Cloud and microservices.** An AWS Marketplace listing describes a three-tier microservices architecture with HL7, FHIR, DICOM and SNOMED CT support ([AWS Marketplace](https://aws.amazon.com/marketplace/pp/prodview-ht5rgwtli4a52)); Dedalus announced AWS as a strategic cloud provider ([PR Newswire](https://prnewswire.com/news-releases/dedalus-expands-work-with-aws-as-a-strategic-cloud-provider-to-transform-the-digital-healthcare-ecosystem-globally-301702563.html)).
- **Beyond the EPR.** Laboratory, diagnostic imaging and integrated-care products sit in the same group, which is Dedalus's stated differentiator (company site, vendor claim).
- **Irish expansion.** A December 2025 announcement of jobs in Ireland ([ANSA](https://www.ansa.it/amp/english/news/news_from_embassies/2025/12/09/dedalus-creates-jobs-in-ireland-ambassador-and-premier-at-announcement_640aad2e-9415-4c61-be2b-2ed2cbac5e6a.html)).

## Architecture and standards

- Dedalus says ORBIS U supports HL7, FHIR and IHE and that it aims to be an open system ([dedalus.com FAQ](https://www.dedalus.com/global/en/faq/solutions)).
- An independent counterpoint: a 2025 KLAS user comment on ORBIS (Central Europe) says FHIR was missing and only HL7 was available ([KLAS](https://klasresearch.com/comments/dedalus-orbis-global-central-europe-europe/116239)). The two statements conflict and may refer to different versions or regions; I cannot resolve it.
- openEHR lists Dedalus as an industry partner ([openEHR](https://openehr.org/community/industry_partners_detail/dedalus)); how openEHR is used in products is not clear to me.
- Closed source, commercially licensed as far as public pages show.

## Fit across care settings

- **Acute:** the core (ORBIS in German-speaking hospitals; ORBIS U and Lorenzo in the NHS).
- **Community and mental health:** Lorenzo had a Care Manager module for community-style care, retained in the 2022 plan; how complete it is for mental-health law and caseloads, I could not find a source for.
- **Emergency department and primary care:** not found in sources read.
- Lorenzo was one of the systems that NHS trusts bought partly for breadth beyond acute, but I found no independent comparison with community-specific systems (consistent with the note in [care settings](../care-settings.md)).

## What independent sources report

- **Safety incident (Lorenzo, under DXC at the time).** East and North Hertfordshire NHS Trust went live on Lorenzo in September 2017. By July 2018, around 48,000 patients had been discharged, and the trust could not show that discharge summaries for 14,600 had been sent to GPs. It declared a serious incident; DXC attributed it to configuration; the fix was reported at GBP 7 million; officials said there was then no evidence of patient harm ([Digital Health](https://www.digitalhealth.net/2018/10/east-and-north-herts-lorenzo-it-issue/), [ITPro](https://itpro.com/it-infrastructure/31946/failed-nhs-it-system-will-cost-7m-to-stabilise)). This predates Dedalus ownership.
- **Other Lorenzo reports.** Warrington sent 650 discharge summaries to GPs without a primary diagnosis after go-live, with GP complaints; Sheffield had nearly 15,000 patients without letters ([Digital Health](https://digitalhealth.net/?p=56567)). The Sheffield system was not confirmed as the same cause.
- **Warwick coroner response.** A 2021 Prevention of Future Deaths response from Warwick Hospital appears in search results (judiciary.uk); I did not read it, so I make no claim.
- **Positive.** Continued NHS wins in 2025 to 2026 are public. I found no independent satisfaction measure for ORBIS U.

## Compared with this project

| Area | This product | This project today | What we could learn or should avoid |
|---|---|---|---|
| Referral and discharge letters | Lorenzo failures were unsent discharge summaries | Closed-loop referrals with timeouts and an unmatched-reply queue | Silent non-delivery is the failure to design against; extend closed-loop to letters |
| Scope | Lab, imaging, EHR, integrated care in one group | EPR scaffold only | Do not chase breadth; interoperate |
| Interoperability | HL7, FHIR, IHE claimed; user says FHIR missing | FHIR read facade for three resources; HL7 v2 ADT | State exactly which resources and operations exist, as we do |
| Product continuity | Lorenzo retired in UK and Ireland in favour of ORBIS U | Pre-1.0 scaffold | Plan migration paths and stable record formats; append-only FHIR-shaped data helps |
| Community and mental health | Care Manager module kept | Care settings planned only | See ideas |
| Identity, consent, audit | Not found in public sources | Matcher, consent, break-glass, hash-chained audit | Keep publishing these; they are rarely public |

## Ideas for this project

These are ideas, not decisions.

1. **Outbound letter closure.** Treat a discharge summary like a referral: a sent item expects a receipt, with a worklist and timeout. Relevant to hazard H-044 and [5.2.1](../05-domain-model.md).
2. **Configuration is a safety surface.** DXC attributed the Hertfordshire fault to configuration; add census tests for configuration that governs outbound communications.
3. **Setting profiles over modules.** Rather than a Care Manager add-on, a [care settings](../care-settings.md) profile; community and mental-health pack decisions remain open.
4. **Say what the facade supports.** The conflicting FHIR claims show the value of a plain capability statement; generate ours from the code.
5. **Migration promise.** Document how a record survives a product retirement (export in FHIR, append-only history) as a stated non-functional goal.

## Not verified

- Current ownership (latest reports 2023; one directory snippet of a 2026 co-CEO appointment is not a company source).
- Which UK trusts still run Lorenzo; whether ORBIS U is live at any UK trust at scale.
- Employee, country and customer numbers are Dedalus statements and vary by page and date.
- The Warwick coroner response and the Sheffield letters cause were not read or confirmed.
- ORBIS U FHIR support: vendor says yes, one KLAS user says no.
- I did not read the East and North Hertfordshire or Humber articles in full.
- Nothing was verified by using the product.
- Link check 2026-11-04: these cited pages returned 404. They may have moved, but what rests on them is unconfirmed: https://openehr.org/community/industry_partners_detail/dedalus

## Sources

- https://www.digitalhealth.net/2020/07/dedalus-buys-dxcs-health-business/ : acquisition of DXC health business
- https://digitalhealth.net/2021/04/dxc-technology-completes-sale-of-health-business-to-dedalus : completion
- https://pulseit.news/?p=21317 : Lorenzo to ORBIS decision (2022)
- https://www.dedalus.com/global/en/faq/solutions : vendor FAQ, standards
- https://www.dedalus.com/global/en/about-us/the-geographical-footprint/ : vendor footprint
- https://www.dedalus.com/global/wp-content/uploads/sites/9/2025/07/Press-Release-Dedalus-and-ENTOMO-review-2-1-1.pdf : 2025 vendor release
- https://www.bluerating.com/?p=145585 : Ardian stake (Italian press)
- https://bebeez.it/private-equity/adia-rileva-la-minoranza-di-dedalus-da-ardian-che-resta-azionista-di-maggioranza/ : ADIA minority
- https://aws.amazon.com/marketplace/pp/prodview-ht5rgwtli4a52 : architecture listing
- https://prnewswire.com/news-releases/dedalus-expands-work-with-aws-as-a-strategic-cloud-provider-to-transform-the-digital-healthcare-ecosystem-globally-301702563.html : AWS
- https://www.ansa.it/amp/english/news/news_from_embassies/2025/12/09/dedalus-creates-jobs-in-ireland-ambassador-and-premier-at-announcement_640aad2e-9415-4c61-be2b-2ed2cbac5e6a.html : Ireland jobs
- https://klasresearch.com/comments/dedalus-orbis-global-central-europe-europe/116239 : KLAS user comment
- https://openehr.org/community/industry_partners_detail/dedalus : openEHR partner page
- https://www.digitalhealth.net/2018/10/east-and-north-herts-lorenzo-it-issue/ : Hertfordshire incident
- https://itpro.com/it-infrastructure/31946/failed-nhs-it-system-will-cost-7m-to-stabilise : same, GBP 7m
- https://digitalhealth.net/?p=56567 : Sheffield letters
- https://www.find-tender.service.gov.uk/Notice/062374-2025/PDF : 2025 direct award notice
