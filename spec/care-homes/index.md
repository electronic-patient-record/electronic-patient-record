# Care homes: integration with this EPR

Public-source research read 2026-11-04 plus this repository's own files. A proposal and a reading, not clinical or legal advice: nothing here is built, and no care home, clinician or lawyer has reviewed it.

Method: web searches and the project's own specs ([care settings](../care-settings.md), [domain model 5.2.1](../05-domain-model.md), [interoperability](../interoperability/index.md), [access control](../access-control/index.md), [mobile viewability](../mobile-viewability.md), [data exchange](../data-exchange/index.md), [manual steps](../manual-steps/index.md)). Almost everything outside the repository rests on search snippets and secondary summaries, not on reading the full documents; "Not verified" says which. England is the worked example, as the project is UK-first; the United States and Australia are shown as examples, not as the rule ([equity rule](../02-scope.md)). Every statement about practice describes what a source says, never an instruction.

## Why care homes matter

1. **The NHS and the home hand over a person, not a patient.** A resident moves between the home, an ambulance, an emergency department, a ward and back, often several times a year. Care is lost between those settings: the point of [5.2.1](../05-domain-model.md).
2. **The people closest to the resident are mostly not on the NHS record system.** Care home staff are employed by a provider that is regulated but is not an NHS body. Most of what they know (appetite, mood, a change from usual) is written in their own record or on paper.
3. **Transfer is where medicines go wrong.** A systematic review of studies of medication errors in nursing homes reports transfer-related errors in 13 to 31 percent of residents in the studies it covers, mostly omissions from inaccurate medication histories and transcription (Pharmaceutical Journal summary, snippet; the review was not opened). It found no intervention that clearly improved the information passed between homes and hospitals.
4. **Both national programmes and regulators push towards a shared record.** See the next sections.
5. **An open-source EPR meets incumbents.** Care home software is its own market (see below), so integration, not replacement, is the realistic first step.

## What a care home is, and who works there

- **Kinds.** Care homes with personal care only (residential) and care homes with nursing; many specialise in dementia or learning disability, and some provide short-stay or intermediate care. In England the regulated activity is accommodation with nursing or personal care, registered with the Care Quality Commission (CQC) under the Health and Social Care Act 2008 and its 2014 Regulations (secondary summaries; the legislation text was not read, and sources disagree on how many fundamental standards there are). Other countries: US nursing homes certified for Medicare or Medicaid are federally regulated and must complete the Minimum Data Set assessment for every resident whatever the payer (CMS); Australian residential aged care follows a Royal Commission that urged connection to My Health Record (see Sources).
- **Staff.** A nursing home employs registered nurses; a residential home may have none on site, and relies on care assistants, some trained to give medicines. Visiting GPs, pharmacists, community nurses, allied health staff and specialist teams are guests with their own record systems. Agency staff are common (a general observation, no figure found).
- **Residents.** Often older, frail, with several conditions, and many lack mental capacity for some decisions. Capacity is decision-specific under the Mental Capacity Act 2005; a best-interests decision follows when it is absent. A health and welfare lasting power of attorney lets a chosen attorney decide when the person loses capacity; an attorney cannot demand a treatment (secondary legal summary).
- **Deprivation of liberty.** In England the Deprivation of Liberty Safeguards (DoLS) apply in care homes and hospitals. The Liberty Protection Safeguards were legislated in 2019 and then postponed; in 2023 the Department of Health and Social Care said implementation would be delayed beyond the life of that Parliament (law-firm summaries). I did not find the position after the 2024 election. A record needs a DoLS authorisation as a dated fact with an expiry, not a flag.
- **Emergency plans.** ReSPECT (Resuscitation Council UK, 2016) records a person's emergency care and treatment recommendations, built from a conversation with the person or, without capacity, with their welfare attorney or family. It is not an advance decision to refuse treatment (trust and ICB documents). Care home staff support the process; they do not write the clinical recommendations.

## What flows between a care home and the NHS

| Flow | Direction | Typical content | Typical channel today | Failure mode |
|---|---|---|---|---|
| Emergency transfer to hospital | Home to NHS | Identity, reason, recent observations, medicines list, ReSPECT, capacity and attorney, contacts | Red bag with paper forms; ambulance crew; phone; some eRedBag (Health Innovation Network) | Missing or old forms, wrong resident's papers, ReSPECT not seen by the crew |
| Hospital discharge to home | NHS to home | Diagnoses, medicines changed and why, follow-up, equipment, mobility, diet, capacity | Paper discharge summary in the red bag; fax or NHSmail; phone | Medicine changes lost or transcribed wrongly; no follow-up owner; return late at night |
| Admission to the home | NHS or family to home | Pre-admission assessment, GP summary, medicines, allergies, ReSPECT | Paper, email, phone; GP Connect where available | A new resident's medicines list built by hand on day one (NICE advises a list on the day of arrival) |
| Medicines orders and changes | GP and pharmacy to home | Prescriptions, changes, stops | Paper or electronic prescription; supplier eMAR; phone | Mismatch between what is prescribed and what the home gives |
| Deterioration | Home to GP, 111 or ambulance | Observations, NEWS2, soft signs, structured call (RESTORE2) | Phone call; paper chart | Observations not sent, not trusted or not taken; a sign missed without a tool |
| Visits and reviews | Both ways | GP and community review notes, pharmacist review, therapy plans | Letters, notes in the visitor's own system | Findings do not reach the home's record; the home's concern does not reach the GP |
| Reading the GP record | NHS to home | Restricted GP summary, allergies, medicines, immunisations (GP Connect Access Record) | Care home software view | Access depends on a digital record, a data-security rating and activation; coverage uneven |
| Shared care record view | NHS to home | Regional or integrated care system record | Browser portal, separate login, or supplier integration | Another login to remember; staff share logins; not in the home's own system |
| Reporting and quality | Home to regulators | Incidents, safeguarding, notifications, MDS (US) | Regulator portals | Duplicate keying |

Notes. The NHS England red bag guidance lists what travels with the resident; the Sutton scheme began in 2015 and NHS England published a quick guide (the often-quoted national coverage and "three to four days shorter stay" are from secondary summaries, not checked). NHS England's Enhanced Health in Care Homes framework (2016, updated in 2020 as version 2) describes seven core elements, among them enhanced primary and community care support and falls prevention, reablement and rehabilitation (I saw only two named; the rest are for the PDF). RESTORE2 pairs early soft signs with NEWS2 measurements and a structured communication tool; the 2020 joint guidance of the CQC, DHSC, NHS England and PHE said the NHS supports tools such as RESTORE2 and NEWS2; a survey found 34.2 percent using NEWS2, 23.3 percent RESTORE2 and 21.9 percent neither, and formal evaluation is scarce (snippets). It is not mandated nationally, as far as I found.

## Records, assurance and standards

- **Digital social care records (England).** NHS England runs an Assured Supplier list for Digital Social Care Records, bought through a dynamic purchasing system; suppliers must meet more than 17 capabilities (snippets). Statements that over 80 percent of providers have a digital record come from an NHS England speaker at a supplier session in March 2026 (supplier page, secondary).
- **Minimum Operational Data Standard (MODS).** An information standard for the consistent baseline recording of direct-care data by CQC-registered adult social care providers (reference DAPB4102, version 1.0.0, published 15 April 2024); technical dependencies include SNOMED CT UK Edition and HL7 FHIR UK Core; implementing it is a contractual requirement for assured suppliers (NHS standards pages, snippet; I did not open the profiles). A claim of a 1 July 2026 compliance deadline comes from a blog and is unverified. Wales has a parallel procurement using FHIR R4.
- **Data Security and Protection Toolkit (DSPT).** Social care providers publish a self-assessment; "approaching standards" is reported as the level for NHSmail, and "standards met" for GP Connect, local shared care records and summary care records (secondary pages from 2019 to 2024; the current rules were not confirmed).
- **GP Connect Access Record.** NHS England states it is used by social care organisations providing regulated care; care homes see a restricted HTML summary (problems, medication, allergies, immunisations); nurses employed in homes may see more (supplier pages).
- **Medicines.** NICE SC1 (2014) and quality standard QS85 (2015): a list of medicines the day a person arrives, and a comprehensive medicines record sent with a person who transfers (NICE pages; recommendation numbers 1.3.2 to 1.3.4 and 1.7.1 and 1.7.3 as quoted by a secondary summary). Electronic medication administration records (eMAR) are referred to in CQC guidance that I did not open.
- **United States.** The federally mandated Minimum Data Set 3.0 (in force 1 October 2010) is completed by licensed staff and sent electronically to CMS (CMS pages). Long-term care vendors such as PointClickCare describe ONC-certified EHR status and automated MDS submission to iQIES (vendor pages; promotional, and the vendor's "about 80 percent of the market" claim is its own).
- **Australia.** Providers can register to view and upload to My Health Record; guidelines for residential aged care exist (version 3 of May 2024); an aged care transfer summary is being added; one source says 35 percent of homes were connected by February 2024 (Ageing Australia, ADHA, secondary).
- **FHIR.** The social-care profiles I found are the MODS ones above; I found nothing on a complete FHIR profile set for a care home resident. Open point.

## Consent, capacity and access

- **Non-NHS staff have no NHS smartcard.** They reach NHS data through a care-home system or a portal, under data-sharing agreements and the DSPT. Smartcard-free access means the identity of the person behind a login is weaker, so shared logins and phones passed between shifts are a risk, and a brief login that a member of staff reuses for several residents is a wrong-resident risk (an inference; no study found).
- **Who may see what.** In GP Connect the restricted view exists because a care assistant should not see everything a nurse does. A care-home profile should be role-based inside the home and bounded by purpose (care of this resident now).
- **Capacity changes who consents.** For a resident without capacity for a decision, the consenting party can be the attorney or, failing one, a best-interests process. A consent record must say who decided, on what basis and when it expires.
- **Family and resident access.** Families often ask for a view of care notes and observations; the legal basis depends on who holds the lasting power of attorney and on the resident's wishes. Residents may not be able to use a portal, so a proxy is the norm, which fits the project's [proxy model](../access-control/index.md) only if the proxy is typed (attorney, family, advocate) and dated.
- **Safeguarding.** A home may hold concerns about abuse by family, staff or other residents; access and disclosure rules must allow hiding information from a proxy. No source on England's specific duties was read here.
- **Bring your own device.** Staff use personal phones; see [mobile viewability](../mobile-viewability.md) items 6 and 7.

## Compared with this project

| Need | This project today | Gap |
|---|---|---|
| Pathway across home, GP and hospital | Pathway-first direction ([5.2.1](../05-domain-model.md)); not built | Pathways and a pathway owner who can be an organisation outside the NHS |
| Closed-loop referral | Built; a referral is answered or flagged | Home as sender and receiver; a "return to the home" task after discharge |
| Observations, NEWS2 | Vital-sign observations built; the vitals can be read as FHIR | Observations arriving from a home (device or hand-keyed), provenance, a score and its rule (an escalation threshold is a clinical decision) |
| Medicines | Not built (medications are not in the record) | Medicines at transfer is the central need; a medicines list is a prerequisite |
| Roles | clinician, nurse, pharmacist, allied-health, receptionist, clinical-admin, patient, proxy, auditor, integrator, CSO | No care assistant, senior carer, registered manager or visiting-professional role; no organisation holder |
| Relationships | care-team, active-encounter, self, proxy | A care relationship held by an organisation (the home) and its staff by rota; a time-boxed visitor relationship |
| Word "patient" | Patient used throughout | "Resident" label in a home profile; the data type stays the same |
| Offline and mobile | Direction in [mobile viewability](../mobile-viewability.md); not built | Writes queued offline with idempotency keys already exist; reading downtime snapshot on a home's weak Wi-Fi |
| Allergies, amendments, consent, audit | Built | Consent by an attorney; amendments from a home to a hospital record |
| HL7 v2 ADT in, FHIR R4/R5 read | Built | An ADT admit or discharge to a home; FHIR write of observations in |
| Minimum data set | None | An agreed set (MODS-aligned) |
| Outbound messages | Not built | Discharge handover to the home; notification of a ReSPECT change |

## Candidate hazards

For the Clinical Safety Officer to review. None is accepted or closed; ids are working labels, not hazard-log entries (the log is `governance/clinical-safety/hazard-log.csv`).

- CH-1 Wrong resident: a paper form, a red bag, a shared login or a phone passed between shifts mixes two residents' data.
- CH-2 Medicines changed in hospital are not applied at the home, or a stopped medicine is restarted.
- CH-3 A ReSPECT form or authorisation is out of date, missing or not visible to the ambulance or ward.
- CH-4 Observations from the home arrive late, without a time or a source, or as a number with the wrong unit, and a deterioration is not escalated.
- CH-5 A proxy sees information the resident or a safeguarding process says to hide, or a proxy without authority acts.
- CH-6 Information for a resident who lacks capacity is recorded as the resident's consent.
- CH-7 A discharge to a home has no named owner for follow-up (the H-044 pattern across organisations).
- CH-8 Staff without clinical registration are shown functions they should not use (recording a clinical decision).
- CH-9 An absent observation or missing allergy is displayed as normal or none; the project rule is `null` with a reason.
- CH-10 A transfer summary is sent to the wrong home or the wrong inbox (fax, email, portal).

## Ideas

Proposals only.

1. **An organisation as the holder of a care relationship.** The home is a care-relationship holder; individual staff inherit access by rota and purpose, not by the home being a person. Test the existing `CareRelationship` resource for what it lacks.
2. **Care-home roles as pack data, not core roles.** Add a "care-home staff" family (care assistant, senior carer, nurse in charge, registered manager) as a setting pack under the [equity rule](../02-scope.md); registration status (nurse, none) is a fact on the person, not inferred from the role.
3. **A "resident" label in a home profile.** Setting profile ([care settings](../care-settings.md)): the term changes, the type does not.
4. **A discharge-to-care-home handover.** A document or FHIR bundle that carries medicines (changed, stopped, new, with reasons), allergies, capacity and attorney, ReSPECT, mobility and diet, and a named owner for follow-up. Depends on medications being built.
5. **A return-to-home task on the pathway.** When a resident is discharged, a task for the home to acknowledge receipt, closing like a referral.
6. **Observations in from a home.** A write path for observations from a home or a device with source, observer, time and unit, and a displayed "last recorded" age; the score and its escalation rule belong to a clinician-reviewed pack.
7. **An agreed minimum data set.** Start from MODS rather than invent one, map to the project's fields, and list what is missing. Review with an assured supplier.
8. **A typed, dated proxy.** Attorney, relative, advocate, each with authority, start, end and hidden-from flags.
9. **Time-boxed visitor access for GPs and pharmacists** instead of a permanent relationship, with an audit entry for each view.
10. **A one-page resident view for a phone.** Banner with identity, allergies, ReSPECT, capacity and attorney, last observations; large touch targets ([mobile viewability](../mobile-viewability.md)).
11. **Offline-tolerant entry for a weak connection**, reusing the idempotency key already on creates; decide in OQ-5.
12. **Integrate before replace.** Read GP Connect and MODS-conformant suppliers' data rather than ask homes to adopt a second system; an assured supplier is the likely route.
13. **Do not adopt a nurse-led scoring tool by default.** Ship NEWS2 or RESTORE2 only as an optional, clinician-reviewed pack.

## Open questions

1. Who is the legal owner of the record for a resident: the home, the GP, the ICB? (Data-protection roles for a home and the NHS body differ by arrangement; a lawyer is needed.)
2. Will care homes want an open-source EPR, or only a feed into their existing supplier? Ask homes and an assured supplier.
3. Which of the seven EHCH elements does a record need to support? (The framework PDF was not read.)
4. What is the current status of the Liberty Protection Safeguards, and what must a record hold for a DoLS authorisation?
5. What does a care assistant need to see during a night shift, and what must be hidden?
6. Which standard carries a discharge summary into a home: FHIR bundle, HL7 CDA or a paper-compatible PDF?
7. How are agency staff identified, and for how long does their access last?
8. How should a resident without a stable NHS number (a new admission from abroad) be matched? See the [matcher](../patient-identity/index.md).
9. What are the equivalents outside England beyond the two sampled here (Scotland, Wales, Northern Ireland, the Netherlands, Germany)?
10. How do we involve residents and families in the design? The residents most affected cannot easily use a survey.

## Not verified

No care home, supplier or clinician was consulted. Nearly every source below is a search snippet or a secondary summary: the EHCH framework PDF, the red bag guide, the MODS specification, GP Connect access rules, DSPT thresholds, the NICE recommendation numbers, CQC regulation numbering, the Liberty Protection Safeguards position after 2024, the RESTORE2 adoption figures and the Australian uptake figure were not read in full. Vendor statements (PointClickCare, Person Centred Software, Birdie, Access) are promotional. The hazard ids are working labels. Nothing was tested in code.
- Link check 2026-11-04: these cited pages returned 404. They may have moved, but what rests on them is unconfirmed: https://www.nice.org.uk/news/article/using-a-red-bag-approach-to-improve-transition-of-care; https://standards.nhs.uk/published-standards/adult-social-care-digital-social-care-record-minimum-operational-data-specification; https://www.weightmans.com/insights/liberty-protection-safeguards-in-limbo/

## Sources

All read 2026-11-04, as search results only unless noted.

- EHCH framework: https://www.england.nhs.uk/publication/enhanced-health-in-care-homes-framework/ ; https://www.england.nhs.uk/wp-content/uploads/2016/09/ehch-framework-v2.pdf ; https://pharmaceutical-journal.com/article/news/greater-integration-could-bring-better-health-in-care-homes-kings-fund-says
- Red bag: https://www.england.nhs.uk/urgent-emergency-care/hospital-to-home/red-bag/ ; https://www.england.nhs.uk/wp-content/uploads/2018/06/quick-guide-redbag-hospital-transfer-v1.pdf ; https://www.nice.org.uk/news/article/using-a-red-bag-approach-to-improve-transition-of-care ; https://healthinnovationnetwork.com/wp-content/uploads/2024/03/eRedBag-Guide-for-Care-Home-Providers-March-2024.pdf
- RESTORE2 and NEWS2: https://www.hantsiow.icb.nhs.uk/your-health/schemes-and-projects/restore2 ; https://www.southwestlondonics.org.uk/our-work/enhanced-health-in-care-homes/restore2/
- Digital social care records and MODS: https://transform.england.nhs.uk/key-tools-and-info/procurement-frameworks/dynamic-purchasing-system-dps-for-digital-social-care-record-solutions ; https://standards.nhs.uk/published-standards/adult-social-care-digital-social-care-record-minimum-operational-data-standard-mods ; https://standards.nhs.uk/published-standards/adult-social-care-digital-social-care-record-minimum-operational-data-specification ; https://www.sell2wales.gov.wales/search/search_switch.aspx?ID=154390 ; https://www.learnsignal.com/blog/digital-social-care-records-mods-compliance-2026/ (unverified blog)
- GP Connect: https://england.nhs.uk/long-read/gp-connect ; https://personcentredsoftware.com/resources/non-clinical-access-to-residents-gp-records ; https://support.carelinelive.com/hc/en-us/articles/5947867743519-GP-Connect-Access-Levels ; https://help-accesscareplanning.theaccessgroup.com/en/articles/14046364-12th-march-2026-introducing-gp-connect-for-access-care-planning
- DSPT and NHSmail: https://www.dsptoolkit.nhs.uk/News/Access-to-shared-systems-DSPT ; https://www.digitalhealth.net/2019/02/entry-level-data-security-toolkit-care-providers-nhsmail/ ; https://www.ukauthority.com/articles/nhsmail-opened-to-social-care-providers/
- Medicines: https://www.nice.org.uk/guidance/qs85/chapter/Quality-statement-1-Record-keeping ; https://pharmaceutical-journal.com/article/news/drug-errors-occur-in-up-to-27-of-people-living-in-nursing-homes-study-shows
- ReSPECT and capacity: https://www.resus.org.uk/sites/default/files/2021-03/ReSPECT%20information%20for%20nursingresidential%20care%20home%20staff%20including%200321.pdf ; https://www.lblaw.co.uk/knowledge/blog/different-legal-documents-regarding-your-health-decisions-when-you-have-lost-capacity/
- DoLS and LPS: https://www.weightmans.com/insights/liberty-protection-safeguards-in-limbo/ ; https://www.blakemorgan.co.uk/delays-liberty-protection-safeguards
- CQC: https://www.cqc.org.uk/about-us/care-workers-guide ; https://legislation.gov.uk/uksi/2014/2936/note/data.pdf
- United States: https://www.cms.gov/medicare/quality/nursing-home-improvement/minimum-data-sets-swing-bed-providers ; https://pointclickcare.com/wp-content/uploads/2024/09/PointClickCare-EHR-for-Skilled-Nursing-Facilities-Solution-Sheet.pdf
- Australia: https://ageingaustralia.asn.au/aciitc-reports/my-health-record-guidelines-for-residential-aged-care ; https://www.agedcareengagement.health.gov.au/blog/better-connected-care-through-my-health-record/ ; https://theweeklysource.com.au/technology/australian-digital-health-agency-adding-hospital-aged-care-transfer-summary-to-my-health-record
