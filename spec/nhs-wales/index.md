# NHS Wales national systems: comparison with this project

Public-source research read 2026-11-09 (plan.md §34, tasks WL-1 to WL-12). Six systems run by Digital Health and Care
Wales (DHCW). DHCW pages describe DHCW's own work and are marked **DHCW**; supplier pages are marked **supplier**;
search-result snippets that could not be opened are marked **snippet**. Nothing here was verified by using a system, and
no private documentation was read. Direction only: nothing in this project was built from it.

DHCW has moved its pages: the addresses in the request now redirect to `/product-directory/our-digital-services/…`, and
the Welsh Immunisation System page under `/coronavirus/` returns 404. DHCW page bodies are drawn by script, so they were
read from the raw HTML.

## Welsh Clinical Portal (WCP)

- **What it is (DHCW).** One view of a patient's record from many sources, on desktop and an iOS and Android app; "39,000+
  users", "500,000+ patient records accessed per month", "1,000+ users of the WCP app", undated [WCP].
- **Capabilities (DHCW)** [WCP] [AR25M3]:
  - the GP record summary;
  - documents from anywhere in Wales (referrals, discharges, letters, clinical notes, care plans);
  - referral triage: by urgency, on hold pending tests, or a question back to the GP; advice between GP and specialist;
  - e-requesting of pathology (DHCW: "nearing… 90% electronic") and radiology, with test sets and bulk orders;
  - results from anywhere in Wales, and radiology images;
  - **results notification and electronic sign-off**: the clinician is told of a newly authorised result and signs it off;
  - **watch lists, which now record why a patient is on one**;
  - a medicines list pre-filled from the GP record;
  - since 2025: single log-on across health boards, palliative care e-forms, integration with ePMA.
- **Architecture.** Results come from and requests go to the Welsh Results Reports Service (WRRS), one national diagnostic
  repository. A Hywel Dda freedom-of-information disclosure (updated 21 April 2026, independent) lists HL7 ADT and ORU feeds
  into WCP and a patient-index (MPI) check before results are filed [HDFOI]. In 2021 DHCW chose Better's openEHR platform as
  its clinical data repository [HTN21]; an HL7 UK post of June 2026 calls the Care Data Repository "a persistent FHIR store"
  [HL7UK]. The two accounts do not agree, and which model is live was not found.
- **Independent findings.** Audit Wales (January 2018) found weak governance and benefits monitoring across NHS Wales
  informatics [AW18] (summary only). The Senedd Public Accounts Committee (November 2018) reported 21 outages in six
  months of 2018, and a patient at Velindre who "did not receive chemotherapy treatment, as blood results were
  unavailable"; it called progress reports "overly positive" [PAC18]. Nothing independent on WCP alone was found.

## Welsh GP Record (WGPR)

- **What it is (DHCW).** A summary of the GP record for those caring for a patient anywhere in Wales [WGPR]. The patient
  choice form (18 February 2025, read in full) names: contact details, current diagnoses, medications, allergies, recent
  test results [WGPROPT]. The FAQ (snippet) adds the registered GP, past problems and two years of prescribing.
- **Who sees it.** Extended in 2016 from emergency settings to hospital doctors and pharmacists for planned care [GOV16].
  Community pharmacists read it through Choose Pharmacy, accredited and only while serving that patient [CHOOSE] (DHCW).
  Vaccinators see only medicines and allergies (FAQ, snippet).
- **Permission.** The clinician asks the patient's permission at each consultation [GOV16]; a practice page says
  "unless the clinician is unable to ask you and there is a clinical reason" (snippet). A full opt-out is a form to the GP
  practice, which adds Read code `#9Ndo`; **the opt-out does not follow the patient to a new practice**, and the form states
  the clinical cost of opting out [WGPROPT].
- **Audit.** Every view is logged, and the National Intelligent Integrated Audit Solution (NIIAS) watches the logs
  [GOV16] [CHOOSE]. The Welsh Control Standard v5 covers NIIAS alerts [WCS] (not opened).

## Welsh Immunisation System (WIS)

- **What it is.** An all-Wales vaccination register built in-house by DHCW during COVID-19; 10 millionth vaccination by
  2 July 2024 [WIS10M] (DHCW). Health boards plan, schedule and invite; vaccinators record; pharmacy staff record stock.
- **Capabilities** (HTN, largely DHCW's announcement, 1 October 2025) [HTN25]: ordering and stock, recording at the point
  of care, **other vaccines due, checked against stock**, invitations and reminders, near real-time reporting with
  denominators, eligibility from live demographics plus GP risk conditions, HL7 and FHIR interfaces.
- **Independent.** Public Health Wales: 1,059,998 people recorded with a flu vaccine in 2025/26, the first season WIS was
  the source, so not comparable with earlier GP-extracted figures [PHWFLU]. For the 2024/25 flu mop-up, Welsh Government
  said "timely write-back of vaccinations recorded in WIS" to GP systems "will not be possible"; health boards sent weekly
  summaries and practices keyed the codes in by hand [WHC24].

## Welsh Intensive Care Information System (WICIS)

- **What it is (DHCW).** A managed service to replace paper ICU charts, collecting data "automatically from the
  monitoring devices, pumps and respiratory equipment"; the Grange University Hospital "will be the first of all 13 Welsh
  adult critical care units" [WICIS]. Supplier: Ascom Digistat, £13m over 7 years, announced 14 September 2020, rollout
  "until 2023" [ASCOM] (supplier; it says 14 units).
- **Capabilities (DHCW, supplier).** Electronic assessments, bedside prescribing and administration, device-captured vital
  signs, fluid balance, calculated scores, national audit and research data [WICIS] [ASCOM].
- **Status.** Go-live was planned for summer 2022 [WICISNEWS] (DHCW); in March 2023 Cwm Taf Morgannwg had "no definite go
  live date" [CTMFOI]. A Welsh Government release of January 2025 asks "Is the current WICIS solution delivered to ABUHB
  safe to go live?" and mentions a "retrograde move to the core Digistat product" (snippet; the PDF returned 404)
  [ATISN]. **No source found confirms any unit live in November 2026.**

## Welsh Nursing Care Record (WNCR)

- **What it is (DHCW).** Standardised nursing documents completed on tablets at the bedside, in a "standardised nursing
  language", led by clinicians from every health board [WNCR]. Launched April 2021 on adult inpatient wards; 89% of wards,
  more than 421,000 patients and almost 20 million notes by 9 December 2024 [WNCRMS]; "live on all eligible inpatient
  wards" in 58 hospitals, undated [MEDIWALES].
- **Each form is a versioned national data standard** [WNCRDOCS]: Adult Inpatient Assessment v2.3, WAASP nutrition v2.1,
  Continence v1.2, Falls and Bone v1.2, Pain v2.5, Patient Handling v1.2, PURPOSE T pressure ulcer v2.2, Mouthcare v1.3,
  bowel, repositioning and skin charts v1.0, cannula v1.2, catheter v1, and the inpatient note. A new form follows a
  standard operating procedure for national data standardisation.
- **Governance.** Each standard was developed by a multidisciplinary group, approved by the Chief Nursing Officer and the
  Directors of Nursing, and issued as a Welsh Health Circular; every health board created nursing informatics roles [PRSB].
- **One national instance.** In November 2023 eight databases, one per organisation, were merged; before, "the fix would
  have to be applied eight times" [WNCRDB] (DHCW).
- **Not confirmed:** that WNCR uses openEHR. No source links it to openEHR or Better. Do not claim it.

## Welsh Community Care Information System (WCCIS), now Connecting Care

- **What it was.** One shared health and social care record for 22 local authorities and 7 health boards, contract
  signed March 2015, on CareDirector v5 [AW20] [CC].
- **Audit Wales, October 2020** [AW20]: 13 local authorities and 2 health boards live at 31 August 2020; about £30m spent
  or committed centrally; organisations deployed it differently; key functions "significantly delayed"; national data
  standards and benefits reporting unfinished.
- **Auditor General, July 2022** [AGL22]: "'Live' meant different things"; a June 2021 survey found "a more negative than
  positive impact on most users' ability to do their work"; full outages in an upgrade, October to November 2021;
  **Welsh-language support, mobile working and NHS interfaces still outstanding**; the choice framed as "single system"
  against "interoperable systems using the same standards".
- **Now.** Renamed Connecting Care on 1 March 2024; commercial products per organisation, an Integrated Care Record for
  sharing, and national data standards [CCNEW] [CC]. CareDirector reaches end of life in January 2026; councils are moving
  to other products, with migration "uncertain and challenging" and six-figure costs [VALE] [NATION].

## Standards in Wales

- Welsh Health Circular WHC/2023/018 (14 June 2023) introduced HL7 FHIR in all NHS Wales bodies [WHC23].
- The FHIR Standards Wales implementation guide is R4 and derived from UK Core; Simplifier shows 2.5.0 (STU2) (snippet),
  HL7 UK says 2.6.0 [SIMPLIFIER] [HL7UK]. Version and licence not checked.

## What this project can learn

Ranked in plan.md §34.2; tasks WL-1 to WL-12.

1. A result is signed off against the version it was shown (WCP) — extends OR-4.
2. A watch list needs a reason (WCP). **Built as WL-6** (2026-11-09): see the service spec.
3. Permission asked at the point of care, or a recorded reason it could not be (WGPR) — beside SE-2 and break-the-glass.
4. A patient's choice follows the person, not the site (WGPR's opt-out does not) — SE-2 and the matcher's merges.
5. A narrower view by role (WGPR's vaccinator view, snippet) — PI and RA.
6. An assessment form is a versioned standard with named sign-off, and each answer keeps its form version (WNCR) — EM-3.
7. A record kept in two places has one direction of flow and a reconciliation (WIS write-back) — DX and RC-5. **Written as WL-5** in [data exchange](../data-exchange/index.md) (2026-11-09); not built.
8. A device reading counts once a clinician validates it, with its source kept (WICIS) — EM-11. **Written as WL-10** in [device readings](../device-readings/index.md) (2026-11-09); not built.
9. "Live" is the functions in use, per site (WCCIS) — the generated status. **Written as WL-11** in [production readiness](../../governance/production-readiness.md) (2026-11-09).
10. Welsh from the start, reviewed by a speaker (WCCIS) — **built as WL-8** (2026-11-09): the UI in Welsh at `cy-001`, machine-written and not reviewed by a speaker.
11. The Wales FHIR guide as a national pack (WHC/2023/018) — IO-18.
12. Already held, recorded so as not to be asked again: an exit path (DX `$export`), one instance rather than copies per site,
    downtime (CA-7), monitored audit (RA-6), honest progress reports (AGENTS.md rule 3).

## Could not verify

- WCP: dates of DHCW's user figures; single sign-on (NADEX), smartcards, SNOMED CT, any public API; whether the repository
  is openEHR, FHIR or both.
- WGPR: the FAQ field list and vaccinator view (snippets); whether permission is still asked at every consultation.
- WIS: offline working; whether write-back to GP systems works now.
- WICIS: any unit live; 13 or 14 units; device protocols, charting intervals, HL7 or FHIR.
- WNCR: openEHR, SNOMED CT, FHIR, offline; the July 2025 completion date (Digital Health returned 403).
- WCCIS: the Integrated Care Record supplier; which organisations run replacements in 2026.
- The licence of the All-Wales nursing forms (PURPOSE T, WAASP) was not looked for.

## Sources

All read 2026-11-09.

- [WCP] https://dhcw.nhs.wales/product-directory/our-digital-services/welsh-clinical-portal/
- [AR25M3] https://dhcw.nhs.wales/annual-report-2025/performance-overview/mission-3/
- [HDFOI] https://hduhb.nhs.wales/about-us/governance-arrangements/freedom-of-information/disclosure-log/disclosure-log-appendices/1-integration-requests-to-digital-health-and-care-wales-dhcw-pdf-116kb/
- [HTN21] https://htn.co.uk/2021/07/07/digital-health-and-care-wales-awards-contract-for-clinical-data-repository/
- [HL7UK] https://www.hl7.org.uk/?p=3210
- [AW18] https://business.senedd.wales/documents/s73101/PAC5-08-18%20P3%20-%20AGW%20Report%20-%20Informatics%20Systems%20in%20NHS%20Wales.pdf (summary only)
- [PAC18] https://senedd.wales/senedd-now/news/500-million-programme-to-digitise-paper-patient-records-in-wales-outdated-and-potentially-putting-patients-at-risk
- [WGPR] https://dhcw.nhs.wales/product-directory/our-digital-services/welsh-gp-record/
- [WGPROPT] https://dhcw.nhs.wales/product-directory/our-digital-services/welsh-gp-record/documents/welsh-gp-record-opt-out-18-february-2025/
- [GOV16] https://www.gov.wales/gp-record-sharing-improve-care-wales
- [CHOOSE] https://dhcw.nhs.wales/news/in-depth/gpc-wales-approves-request-from-digital-health-and-care-wales-to-implement-access-to-welsh-gp-record-for-choose-pharmacy-services/
- [WCS] https://dhcw.nhs.wales/ig/ig-documents/welsh-control-standard-for-electronic-health-and-care-records-v5-pdf/ (not opened)
- [WIS10M] https://dhcw.nhs.wales/news/latest-news/10-millionth-covid-vaccination-recorded-in-wales/
- [HTN25] https://htn.co.uk/?p=76319
- [PHWFLU] https://icc.gig.cymru/app/uploads/sites/2/2026/07/Ffliw-tymhorol-yng-Nghymru-2025-i-2026-Saesneg-yn-unig.pdf
- [WHC24] https://www.gov.wales/change-influenza-vaccination-programme-2024-2025-whc2024046-html
- [WICIS] https://dhcw.nhs.wales/product-directory/our-digital-services/welsh-intensive-care-information-system/
- [WICISNEWS] https://dhcw.nhs.wales/news/latest-news/welsh-intensive-care-information-system-steps-closer-to-go-live/
- [ASCOM] https://www.ascom.com/uk/news/local-news/uk-news/ascom-wins-all-wales-nhs-intensive-care-contract-worth-13m/
- [CTMFOI] https://ctmuhb.nhs.wales/use-of-site/freedom-of-information/disclosure-log/20231/february/ctmuhb7923feb-critical-care-systempdf/
- [ATISN] https://gov.wales/sites/default/files/publications/2025-01/ATISN%2023148%20doc%201.pdf (404; snippet only)
- [WNCR] https://dhcw.nhs.wales/product-directory/our-digital-services/welsh-nursing-care-record-wncr/
- [WNCRMS] https://dhcw.nhs.wales/news/latest-news/welsh-nursing-care-record-reaches-major-milestone/
- [WNCRDB] https://dhcw.nhs.wales/news/latest-news/welsh-nursing-care-record-upgraded-to-one-national-database/
- [MEDIWALES] https://mediwales.com/news/the-welsh-nursing-care-record-is-now-live-across-all-eligible-wards-in-wales/
- [WNCRDOCS] https://dhcw.nhs.wales/data/information-standards/data-standards/nhs-wales-nursing-documentation/adult-nursing-documentation/
- [PRSB] https://theprsb.org/news/the-welsh-nursing-care-records-journey-to-success/
- [AW20] https://audit.wales/news/ambitious-vision-welsh-community-care-information-system-still-long-way-being-realised
- [AGL22] https://www.wao.gov.uk/sites/default/files/publications/WCCIS-Letter_to_MS_Chair_PAPAC.pdf
- [CC] https://dhcw.nhs.wales/product-directory/our-digital-services/connecting-care/
- [CCNEW] https://dhcw.nhs.wales/product-directory/our-digital-services/connecting-care/connecting-care-a-new-chapter-for-community-mental-health-and-social-care
- [VALE] https://www.valeofglamorgan.gov.uk/Documents/_Committee%20Reports/Scrutiny-New-2025/Scrutiny-Live-Well/2025/25-05-27/Social-Care-Records-System.pdf
- [NATION] https://nation.cymru/news/councils-move-to-new-social-care-data-system-hits-technical-problems
- [WHC23] https://www.gov.wales/sites/default/files/publications/2023-06/introduction-of-hl7-fhir-in-all-nhs-wales-bodies_1.pdf
- [SIMPLIFIER] https://simplifier.net/guide/fhir-standards-wales-implementation-guide (snippet)
