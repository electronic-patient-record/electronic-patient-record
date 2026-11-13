# Altera Digital Health: comparison with this project

Public-source research read 2026-11-04. Vendor material is marketing; nothing here was verified by using the product; no private documentation was read. Direction only: nothing in this project was built from it.

The user's spelling "Alteira" is taken to mean Altera Digital Health. It is unrelated to the semiconductor company Altera, which search results also surface.

## What it is

- **Owner.** Altera Digital Health is a business unit of Harris Healthcare. Harris (N. Harris Computer Corporation) is a wholly owned subsidiary of Constellation Software Inc. (TSX: CSU). Harris announced on 2022-05-06 that it had completed the purchase of Allscripts' "hospitals and large physician practices" segment, which then operated as Altera. The price is not stated in that release. (Vendor/owner release: [Altera newsroom](https://www.alterahealth.com/newsroom/harris-completes-purchase-of-allscripts-hospitals-and-large-physician-practices-business-segment/).)
- **Origin.** The trade press reported the rebrand of the Allscripts part as Altera in May 2022 ([Digital Health](https://digitalhealth.net/2022/05/allscripts-re-branded-altera-digital-health)). A directory page gives March 2022 as the acquisition date; Harris's own release says May. I use May.
- **Products named in sources.** Sunrise (hospital EHR, used in hospital trusts in England), Paragon (including "Paragon Denali", aimed at smaller and community hospitals in the US), and TouchWorks (ambulatory EHR). The Harris release does not list products; Altera's site lists them.
- **Main markets.** US, UK (acute hospital trusts), Canada and others. I found no sourced figure for total customers or patients.
- **Care settings.** Mainly acute and ambulatory. See "Fit across care settings".
- **2026 state.** Altera is still operating under Harris in 2026: press releases of February to March 2026 ([BusinessWire, HIMSS26](https://www.businesswire.com/news/home/20260303986338/en/Altera-Digital-Health-to-Showcase-Human-Centered-AI-and-EHR-Advancements-at-HIMSS26) was not readable by me; only its search snippet). Stockport and Tameside and Glossop foundation trusts awarded ten-year EPR contracts reported on 2026-03-24 ([Healthcare Management](https://www.healthcare-management.uk/tag/tameside-and-glossop-integrated-care-nhs-foundation-trust)). A WARN notice for 133 workers in Melville, New York, effective 2026-12-31, is listed by an aggregator ([WARN Tracker](https://www.warntracker.com/layoff/altera-digital-health-2026-12-31)); no company explanation found.

## Capabilities worth noting

- **Ambient scribe.** Sunrise Thread AI, described by Altera as a native ambient scribe and note-generation assistant, general availability announced 2025-11-18 ([BusinessWire](https://businesswire.com/news/home/20251118584360/en/Altera-Digital-Health-Introduces-Sunrise-Thread-AI-an-Ambient-Scribe-and-Note-Generation-Assistant)). Vendor claim; no independent evaluation found.
- **Contract renewals.** Altera says 14 clients renewed Sunrise in February 2026 ([BusinessWire](https://www.businesswire.com/news/home/20260224930858/en); see also [Becker's](https://www.beckershospitalreview.com/healthcare-information-technology/ehrs/14-health-systems-extend-ehr-contracts-with-altera-digital-health/)). Renewals are a vendor-reported signal, not a quality measure.
- **Cloud-native smaller-hospital EHR.** Paragon Denali is described as built on Azure with containerised services, to reduce downtime; KPC Health planned a rollout at seven Southern California hospitals ([BusinessWire](https://www.businesswire.com/news/home/20250211824072/en/), via search summary). Design claim only.
- **National exchange.** A partnership to bring Health Gorilla's network into Sunrise was announced in June 2025 ([BusinessWire](https://www.businesswire.com/news/home/20250610262689/en/Altera-Digital-Health-and-Health-Gorilla-Announce-Strategic-Partnership-to-Accelerate-Healthcare-Interoperability)).
- **UK spread.** Fordcombe Hospital was reported in 2025 as the 23rd public hospital in England on Sunrise ([Digital Health](https://www.digitalhealth.net/2025/02/kent-hospital-goes-live-with-altera-electronic-patient-record/)).

## Architecture and standards

- A third-party listing says Sunrise supports FHIR R4 and US Core on Microsoft Azure, and warns that exchange with non-Altera systems can be hard in practice ([EHR Source](https://ehrsource.com/vendors/altera-digital-health/)). This is a review site, not Altera documentation.
- I found no public Altera FHIR API specification, data model description or open-source component. Searches for dbMotion and Altera Cloud hosting details returned nothing usable.
- Openness: proprietary, closed source, as far as public pages show. Not confirmed from a licence or contract.

## Fit across care settings

- **Acute:** the core of Sunrise in hospital trusts (Worcestershire Acute, Salford Royal, Liverpool Heart and Chest in vendor case studies: [Digital Health](https://digitalhealth.net/2023/03/worcestershire-acute-hospitals-nhs-trust-selects-altera-digital-health), [case studies](https://www.casestudies.com/company/altera-digital-health)).
- **Emergency department, community, mental health, primary care:** I could not find a sourced statement for any of these in Altera's current product pages. Stockport and Tameside are integrated-care trusts, which suggests some community scope, but the sources I read do not say which services the contract covers. TouchWorks is an ambulatory (outpatient, US-style) product.

## What independent sources report

- **Positive or neutral.** Hospital trust choices and 2026 ten-year awards are public facts (see above). Trust board papers and case studies are largely supplier-authored or supplier-quoted.
- **Problems.** I found no independent inquiry, regulator finding, patient-safety study or KLAS report on Sunrise or Paragon. A KLAS page for Sunrise (over 400 beds) exists ([KLAS](https://klasresearch.com/comments/altera-digital-health-sunrise-large-over-400-beds/344260)), and a user comment mentions a change of ownership, but I could not read it and it is anecdotal. The Melville layoff notice (above) is a fact about headcount, not about product quality.
- Absence of findings here is a limit of my search, not evidence that none exist.

## Compared with this project

| Area | This product | This project today | What we could learn or should avoid |
|---|---|---|---|
| Scope | Full hospital EHR (Sunrise), ambulatory (TouchWorks), small-hospital (Paragon) | Scaffold: allergies, vitals, referrals, identity, consent, audit | Breadth took a vendor decades; stay narrow and say so |
| Identity | Not found in public sources | Matcher with explainable scores; no auto-link on missing evidence | Our explainability is a documented strength to keep |
| Access control | Not found | Consent, care relationship, break-glass | Keep break-glass reasons reviewable |
| Interoperability | FHIR R4 claimed by a review site; network partnerships | FHIR R4/R5 read facade (three resources); HL7 v2 ADT inbound | Partnerships do not replace a public API spec; publish ours |
| AI documentation | Ambient scribe on sale | Nothing; not planned as core | Treat as a clinical-risk feature needing hazard-log entries |
| Audit | Not found | Hash-chained audit trail with retention | Already a differentiator to state carefully |
| Mobile/community | Not found for Sunrise | Planned only | See ideas |

## Ideas for this project

These are ideas, not decisions.

1. **Integrated-care contracts as a test case.** Stockport and Tameside is a recent example of an acute system extended across a wider organisation. Use it as a scenario when drafting [care settings](../care-settings.md) profiles (idea only; contract scope unverified).
2. **Smaller-hospital profile.** Paragon Denali targets smaller hospitals; a lighter setting profile in our spec could echo that, without a second product.
3. **Ambient-scribe hazard note.** If notes are ever built, record an AI-drafted note as a distinct provenance state in the append-only record, with human sign-off, and add hazards before any build.
4. **Publish API conformance.** The vendor's FHIR support is claimed by third parties; ours could be checked in CI with a public capability statement.
5. **Closed-loop referral as a pathway building block.** Nothing found in Altera sources on referral closure; this is a gap we already cover and could extend in [5.2.1](../05-domain-model.md).

## Not verified

- Current ownership beyond the 2022 release: no 2026 filing read; the Harris chain is as of 2022.
- HIMSS26 and February 2026 BusinessWire pages returned HTTP 403; only search snippets informed those lines.
- Product scope of the Stockport and Tameside contract (Sunrise or other; services covered).
- Any FHIR API, dbMotion, hosting or openness detail from Altera directly.
- Any independent safety, usability or outage data. Nothing was verified by using the product.
- Customer counts and market share: not found.

## Sources

- https://www.alterahealth.com/newsroom/harris-completes-purchase-of-allscripts-hospitals-and-large-physician-practices-business-segment/ : Harris completion release, 2022-05-06
- https://digitalhealth.net/2022/05/allscripts-re-branded-altera-digital-health : trade press on the rebrand
- https://www.healthcare-management.uk/tag/tameside-and-glossop-integrated-care-nhs-foundation-trust : Stockport and Tameside contracts, 2026
- https://www.warntracker.com/layoff/altera-digital-health-2026-12-31 : WARN notice aggregator
- https://businesswire.com/news/home/20251118584360/en/Altera-Digital-Health-Introduces-Sunrise-Thread-AI-an-Ambient-Scribe-and-Note-Generation-Assistant : vendor, Thread AI
- https://www.beckershospitalreview.com/healthcare-information-technology/ehrs/14-health-systems-extend-ehr-contracts-with-altera-digital-health/ : trade press on renewals
- https://www.businesswire.com/news/home/20250610262689/en/Altera-Digital-Health-and-Health-Gorilla-Announce-Strategic-Partnership-to-Accelerate-Healthcare-Interoperability : vendor, partnership
- https://www.digitalhealth.net/2025/02/kent-hospital-goes-live-with-altera-electronic-patient-record/ : English hospital go-live
- https://digitalhealth.net/2023/03/worcestershire-acute-hospitals-nhs-trust-selects-altera-digital-health : English trust selection
- https://www.casestudies.com/company/altera-digital-health : vendor-linked case studies
- https://ehrsource.com/vendors/altera-digital-health/ : third-party review site (FHIR, Azure claims)
- https://klasresearch.com/comments/altera-digital-health-sunrise-large-over-400-beds/344260 : KLAS page, not read
