# Access Rio: comparison with this project

Public-source research read 2026-11-04. Vendor material is marketing; nothing here was verified by using the product; no private documentation was read. Direction only: nothing in this project was built from it.

## What it is

- Access Rio (previously Servelec Rio) is an electronic patient record for community, mental-health and child-health providers, sold by The Access Group ([Access Group, Rio page](https://www.theaccessgroup.com/en-gb/campaigns/hsc/rio)). Variants include Rio Evo for independent mental health ([Access Group](https://www.theaccessgroup.com/en-gb/health-social-care/healthcare-software/rio-evo-for-independent-mental-health/)).
- Ownership: Servelec (Sheffield) was taken private by Montagu in 2018 for 223.9 million pounds and bought by The Access Group in 2021; the date is given as August or 1 September 2021 ([Wikipedia, Servelec](https://en.wikipedia.org/wiki/Servelec); [Digital Health, 2021](https://www.digitalhealth.net/2021/09/servelec-acquired-the-access-group); [HTN](https://htn.co.uk/2021/09/01/acquisition-of-servelec-announced/)). Access is backed by private equity: Hg and TA Associates, with GIC as a minority investor, at a 9.2 billion pound valuation in 2022 ([Hg](https://hgcapital.com/insights/the-access-group-announces-substantial-investment)). I found no later ownership change; not verified.
- Servelec also made Oceano (patient administration) and Flow (bed management) (Wikipedia). Whether these are still sold under Access: not confirmed.
- Scale: the vendor says Rio is "used by almost half of all Mental Health Trusts in England" ([Access Group](https://theaccessgroup.com/en-gb/health-social-care/software/mental-health)); vendor claim, not independently checked. Also in Ireland (St Patrick's, 2017, per Wikipedia).

## Capabilities worth noting

- Clinical and administrative functions for community, mental and child health, with modules such as eObservations, order communications, ePMA, a Mood Diary app and an Integration Hub (vendor: [Rio page](https://www.theaccessgroup.com/en-gb/campaigns/hsc/rio)).
- Browser-based Rio Cloud, with community nurses recording in the field and syncing to the record, as in a Birmingham Community case study (vendor, same page).
- A mental-health module for inpatient and outpatient work that the vendor says complies with the Mental Health Act, and is listed on the Digital Marketplace ([Access Group](https://theaccessgroup.com/en-gb/health-social-care/software/mental-health), as relayed by search; the Rio page itself does not mention the Act).
- Merging separate instances into one has been done: Mersey Care consolidated its Rio instances ([Mersey Care](https://yourspace.merseycare.nhs.uk/clinical-system-harmonisation/rio-electronic-patient-record-epr-system)).

## Architecture and standards

- Cloud option via browser; the Integration Hub is a cloud platform for sharing data across settings (vendor).
- Data model, FHIR or HL7 support, and public API documentation: I could not find a primary statement; not verified.
- Hosting model and tenancy per trust: not found.
- Closed, proprietary product; no public source.

## Fit across care settings

- Mental health: the core market, with the Mental Health Act as a design driver.
- Community and child health: named scope, with field recording (vendor case studies).
- Primary care: not its market; integration with GP systems: not found.
- Acute and ED: not its core; Servelec's Oceano PAS was built with University Hospitals Birmingham, live August 2017 (Wikipedia), but I found no current evidence of acute use.

## What independent sources report

Positive or neutral:
- Trusts have chosen and gone live with Rio over many years, for example Lincolnshire in 2018 ([Building Better Healthcare](https://buildingbetterhealthcare.com/lincolnshire-nhs-trust-goes-live-with-rio-electronic-patient-record--148402)) and Oxford Health in 2023 (per a search summary; source page not opened).
- Mersey Care's consolidation of its instances (above).

Problems reported:
- 2017: board papers of South Staffordshire and Shropshire Healthcare cited a risk to patient safety and of illegal detention because Rio did not allow full monitoring of Mental Health Act status; the trust ran a manual system as a workaround ([Digital Health, 2017](https://www.digitalhealth.net/2017/03/patients-at-risk-of-illegal-detainment-at-mh-trust/)). This is one trust in one year; it may not reflect other versions or configurations.
- 2017: a coroner's concern that North West Boroughs' Rio and IAPTUS systems did not connect, so a clinician could not see that a person was open to another team; the trust proposed an alert flag in both ([trust response](https://www.judiciary.uk/wp-content/uploads/2017/12/2017-0319-Response-by-North-West-Boroughs-Healthcare-NHS-Trust.pdf)).
- Delayed go-lives, for example 5 Boroughs four months late ([Digital Health, 2015](https://www.digitalhealth.net/2015/02/5-boroughs-live-with-rio)).
- I found no CQC report, HSJ story or formal inquiry on Rio for 2024 to 2026.

## Compared with this project

| Area | This product | This project today | What we could learn or should avoid |
|---|---|---|---|
| Mental Health Act | Module claimed compliant (vendor); one trust reported gaps (2017) | Not built; a legal-status pack is planned (XP-7) | Legal status as dated record with authority; test that nothing depends on a manual workaround |
| Mobile and field work | Browser cloud, field recording, sync | No mobile layout | Define what a visit round needs offline |
| Cross-system visibility | Coroner concern about two unconnected systems | Matcher, consent, referral loop | Show "open to another team" as a flag from identity matching |
| Setting profiles | Community, mental and child health in one product | Care-settings profile idea | One record, several profiles: close to our plan |
| Integration | Integration Hub (vendor) | HL7 v2 ADT inbound; FHIR read facade | Publish contracts; keep the hub idea out of the core |
| Patient apps | Mood Diary app | Portal not built | Patient-reported outcomes could reuse the observation resource |
| Openness | Closed | Open | Keep an auditable spec |
| Audit | Not found | Hash-chained trail | Strength to keep |

## Ideas for this project

These are ideas, not decisions.
1. In the care-settings legal-status pack, include a report that lists everyone whose detention authority is expiring or missing, so no manual register is needed (spec/care-settings.md).
2. Add a "this person is open to another team" signal from the matcher and the care-relationship table, to be shown in the patient banner on small screens (spec/mobile-viewability.md).
3. Describe caseload and visit-round work units beside the bed board in the setting profile.
4. Model patient-reported mood or symptom diaries as observations with a source label, once a portal exists.
5. Add to the pathway design a rule that discharge from one team does not close the pathway (5.2.1), as Rio's long-running community use implies.

## Not verified

- Current Access ownership beyond 2022 reports; whether Rio is still sold under all variants.
- The "almost half of Mental Health Trusts" claim; the Mental Health Act compliance claim.
- Standards support (FHIR, HL7), hosting, offline behaviour.
- Oxford Health 2023 go-live; Somerset time-saving figures (vendor).
- The 2017 incidents are old; several Digital Health pieces are 2007 to 2017.
- Several vendor pages were read through search summaries.

## Sources

- https://www.theaccessgroup.com/en-gb/campaigns/hsc/rio : vendor Rio page
- https://theaccessgroup.com/en-gb/health-social-care/software/mental-health : vendor mental health EPR page
- https://www.theaccessgroup.com/en-gb/health-social-care/healthcare-software/rio-evo-for-independent-mental-health/ : vendor Rio Evo
- https://en.wikipedia.org/wiki/Servelec : Servelec history (secondary, may be dated)
- https://www.digitalhealth.net/2021/09/servelec-acquired-the-access-group : acquisition
- https://htn.co.uk/2021/09/01/acquisition-of-servelec-announced/ : acquisition
- https://hgcapital.com/insights/the-access-group-announces-substantial-investment : Access ownership 2022
- https://www.digitalhealth.net/2017/03/patients-at-risk-of-illegal-detainment-at-mh-trust/ : MHA risk report
- https://www.judiciary.uk/wp-content/uploads/2017/12/2017-0319-Response-by-North-West-Boroughs-Healthcare-NHS-Trust.pdf : coroner response
- https://www.digitalhealth.net/2015/02/5-boroughs-live-with-rio : delayed go-live
- https://buildingbetterhealthcare.com/lincolnshire-nhs-trust-goes-live-with-rio-electronic-patient-record--148402 : Lincolnshire go-live
- https://yourspace.merseycare.nhs.uk/clinical-system-harmonisation/rio-electronic-patient-record-epr-system : Mersey Care consolidation
