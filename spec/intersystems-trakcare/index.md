# InterSystems TrakCare: comparison with this project

Public-source research read 2026-11-04. Vendor material is marketing; nothing here was verified by using the product; no private documentation was read. Direction only: nothing in this project was built from it.

## What it is

- **Owner.** InterSystems, a privately held software company headquartered in Boston, founded in 1978; founder Terry Ragon is CEO. Wikipedia reports revenue over US$1 billion in 2023 and about 2,400 staff ([Wikipedia](https://en.wikipedia.org/wiki/InterSystems)); treat as secondary.
- **Products.** TrakCare, a hospital information system that came from the Australian vendor TrakHealth, bought in 2007 and sold outside the United States. HealthShare, an interoperability and health-informatics platform (now marketed as HealthShare Health Connect). InterSystems IRIS for Health, the data platform TrakCare is built on ([Wikipedia](https://en.wikipedia.org/wiki/InterSystems)).
- **Scale (vendor).** InterSystems says TrakCare serves over 100 million patients in 29 countries on five continents, in more than 600 hospitals ([InterSystems UK](https://www.intersystems.com/uk/trakcare/unified-healthcare-information-system)). Vendor claim, not independently counted.
- **Markets with public evidence.** Scotland (12 of 14 health boards by 2017 per a vendor release; a 2010 framework worth over GBP 44 million reported by a Czech press release), England (North Tees, Gloucestershire, Yeovil), Ireland (National Forensic Mental Health Service, 2023), Australia and Asia ([Lupa.cz](https://www.lupa.cz/tiskove-zpravy/trakcare-od-intersystems/), [Digital Health](https://www.digitalhealth.net/2015/10/english-hospital-live-with-intersystems), [Digital Health 2023](https://www.digitalhealth.net/2023/12/irelands-national-forensic-medical-health-svc-goes-live-with-trakcare/)).

## Capabilities worth noting

- **One system, one code base.** The vendor describes a single user interface, code base and data platform for the patient record ([InterSystems UK](https://www.intersystems.com/uk/trakcare/unified-healthcare-information-system)).
- **Search-driven access.** "TrakCare Assistant" gives text-driven access to patient data to cut clicks (same page, vendor claim).
- **Modules.** Medication management, laboratory, revenue cycle and departmental documentation as add-ons; telehealth integrates with Microsoft Teams, Google Meet and Attend Anywhere (same page).
- **AI kept separate.** The vendor states TrakCare itself includes no AI; generative AI is in a separate product, IntelliCare, not available everywhere (same page).
- **Mobile.** Web-based, described as mobile-first, with an extension for the Ascom Myco 3 smartphone (same page).
- **Mental health in the scope from the start in Scotland.** The Scottish contract covered mental-health patient administration, complex scheduling and order communications, per a NHS Fife report ([Digital Health 2017](https://www.digitalhealth.net/2017/08/mental-health-professionals-across-fife-obtain-crucial-data-via-trakcare)).

## Architecture and standards

- Built on InterSystems IRIS for Health; vendor says all TrakCare data is available through an API and supports HL7 FHIR and SMART on FHIR ([InterSystems UK](https://www.intersystems.com/uk/trakcare/unified-healthcare-information-system); [connectivity](https://intersystems.com/uk/trakcare/connectivity)).
- In 2019 InterSystems committed to making NHS HL7 FHIR profiles available to NHS customers at no extra cost, via Health Connect ([Digital Health](https://digitalhealth.net/?p=86593), [Healthcare IT News](https://www.healthcareitnews.com/news/europe/intersystems-makes-hl7-fhir-integration-components-free-nhs-customers)).
- A 2021 TrakCare release used the IRIS for Health FHIR repository to support SMART on FHIR apps ([IT Brief](https://itbrief.com.au/story/covid-19-drives-new-updates-to-intersystems-health-platform)).
- Closed source, commercially licensed as far as public pages show; the data model is not published in what I read.

## Fit across care settings

- **Acute and outpatient:** the core use; the vendor page speaks of hospitals, outpatients, wards and discharge into the community.
- **Mental health:** evidence from Scotland (Fife) and Ireland's national forensic mental health service, which the trade press says supports Mental Health Act reporting ([HTN](https://htn.co.uk/2023/11/29/irelands-national-forensic-mental-health-service-launches-new-electronic-medical-record/)).
- **Community:** the vendor page does not list it. Not found in sources read.
- **Primary care:** not mentioned on the vendor page; not found elsewhere.
- **Emergency department:** not described in the pages I read; I could not confirm it.

## What independent sources report

- **Delays and data problems in England.** North Tees and Hartlepool, first English trust to go live (October 2015), delayed phase two while dealing with data and reporting issues ([Digital Health](https://www.digitalhealth.net/2016/03/north-tees-delays-phase-2-of-trakcare-epr)). Gloucestershire went live in December 2016 and suspended referral-to-treatment reporting over data quality concerns after migration ([Digital Health](https://www.digitalhealth.net/2017/02/gloucestershire-hospitals-suspends-rtt-reporting/)); an earlier go-live there was postponed ([Digital Health](https://digitalhealth.net/2016/09/delayed-south-trakcare-project-sees-glocs-stop-go-live-again)).
- **Scotland.** NHS Ayrshire and Arran slowed its rollout in 2013 after clinician feedback on phase one ([Digital Health](https://www.digitalhealth.net/2013/02/ayrshire-goes-slowly-with-trakcare/)). Later boards went live; a vendor release claims 12 of 14 boards.
- **Positive.** Yeovil's 2016 go-live was described as a success ([Digital Health](https://www.digitalhealth.net/clinical_software/47780/yeovil-epr-trakcare-go-live-a-)); North Tees reported over 25,000 clinical hours saved (trade-press snippet, vendor-sourced).
- Coverage is mostly 2013 to 2017. I found no formal inquiry or audit and nothing from 2025 or 2026. A "Best in KLAS" banner for 2025 appears only on a vendor page; I could not read the detail.

## Compared with this project

| Area | This product | This project today | What we could learn or should avoid |
|---|---|---|---|
| Scope | Full hospital system incl. mental-health administration, scheduling, orders | Scaffold; orders, scheduling, ADT/beds not built | Scheduling and orders are where rollouts failed on data and reporting; test migration data early |
| Platform | IRIS for Health, one code base | Rust service, FHIR-shaped append-only record | Single data platform is a strength; our record is already single-store |
| Interoperability | FHIR, SMART on FHIR, NHS profiles free to customers | FHIR R4/R5 read facade; HL7 v2 ADT inbound | Publish profile conformance; consider NHS profiles as a future pack |
| Mental health | Mental Health Act reporting in Ireland; Scotland MH administration | Not built; legal status planned as a pack (XP-7) | Legal status as a record, not a flag, matches our design |
| Mobile | Mobile-first claim; one smartphone extension | Planned; mobile layout not built | Vendor claim only; our [mobile viewability](../mobile-viewability.md) spec should be testable |
| Access control | Not found in public sources | Consent, care relationship, break-glass | Our model is more explicit in public |
| Audit | Not found | Hash-chained, retention/archive | State this plainly |
| Identity | Not found | Matcher with explainable scores | Same |

## Ideas for this project

These are ideas, not decisions.

1. **Migration and reporting checks before go-live.** Two English go-lives stalled on data quality and waiting-time reporting. Add a migration-reconciliation check to the planned ADT/scheduling work.
2. **Mental-health legal status as a pack.** Use the Irish Mental Health Act reporting case as a worked example for [care settings](../care-settings.md) and XP-7 (statutes differ; the core ships none).
3. **Search-first view.** TrakCare Assistant suggests a text-search entry to the record; consider as a clinician-UI pattern in [pathway design](../index.md) work, after usability testing.
4. **Free NHS-profile policy.** Offering conformance profiles at no extra cost was a 2019 commitment; for an open project, publishing profiles is the default.
5. **Phased roll-out by setting.** Scotland's board-by-board rollout is a model for setting profiles; tie to the pathway-first rule in [5.2.1](../05-domain-model.md).

## Not verified

- All scale figures (patients, countries, hospitals) are InterSystems statements.
- Wikipedia revenue and staffing numbers are secondary.
- Current status of NHS England sites, and whether any are being replaced: not searched.
- Community, primary care and ED coverage: not found.
- The 2025 "Best in KLAS" banner: detail not read.
- Developer documentation for the FHIR API: not read.
- Independent evidence is mostly 2013 to 2017.

## Sources

- https://www.intersystems.com/uk/trakcare/unified-healthcare-information-system : vendor overview
- https://intersystems.com/uk/trakcare/connectivity : vendor connectivity
- https://en.wikipedia.org/wiki/InterSystems : company history, ownership
- https://digitalhealth.net/?p=86593 : FHIR tools free to NHS customers (2019)
- https://www.healthcareitnews.com/news/europe/intersystems-makes-hl7-fhir-integration-components-free-nhs-customers : same, trade press
- https://itbrief.com.au/story/covid-19-drives-new-updates-to-intersystems-health-platform : 2021 release, FHIR repository
- https://www.lupa.cz/tiskove-zpravy/trakcare-od-intersystems/ : Scottish framework (Czech, 2010)
- https://www.digitalhealth.net/2015/10/english-hospital-live-with-intersystems : North Tees go-live
- https://www.digitalhealth.net/2016/03/north-tees-delays-phase-2-of-trakcare-epr : delay
- https://www.digitalhealth.net/2017/02/gloucestershire-hospitals-suspends-rtt-reporting/ : RTT suspension
- https://digitalhealth.net/2016/09/delayed-south-trakcare-project-sees-glocs-stop-go-live-again : delayed go-live
- https://www.digitalhealth.net/2013/02/ayrshire-goes-slowly-with-trakcare/ : Ayrshire
- https://www.digitalhealth.net/clinical_software/47780/yeovil-epr-trakcare-go-live-a- : Yeovil
- https://www.digitalhealth.net/2017/08/mental-health-professionals-across-fife-obtain-crucial-data-via-trakcare : Fife mental health
- https://www.digitalhealth.net/2023/12/irelands-national-forensic-medical-health-svc-goes-live-with-trakcare/ : Ireland forensic mental health
- https://htn.co.uk/2023/11/29/irelands-national-forensic-mental-health-service-launches-new-electronic-medical-record/ : same, Mental Health Act reporting
