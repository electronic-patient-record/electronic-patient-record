# Maternity: how it works and what it would need from this project

Public-source research read 2026-11-04. A proposal and a reading, not clinical guidance: nothing here is built, and no midwife or obstetrician has reviewed it.

Method: web searches and the project's own specs ([care settings](../care-settings.md), [domain model 5.2.1](../05-domain-model.md), [patient identity](../patient-identity/index.md), [access control](../access-control/index.md), [interoperability](../interoperability/index.md)). Much of what follows rests on search snippets and secondary summaries, not on reading the full reports; the list under "Not verified" says which. Every statement about clinical practice is a description of what a source says, never an instruction.

## Why maternity is different

1. **Two patients, one event.** From birth a mother and each baby are separate people with separate records, yet most of the care around the birth is joint. A shared event has to be written into two records.
2. **Mostly well people, short and intense peaks.** Most pregnancies need routine care over about nine months, with a labour that can turn urgent within minutes. A record must serve a calm schedule of contacts and a time-critical event.
3. **Many teams, many places.** A woman may be seen at home, in a clinic, in a GP surgery, in a scan department and on a labour ward; the midwife often leads. This is the pathway-first case of [5.2.1](../05-domain-model.md).
4. **Sensitive by default.** Domestic abuse, age, safeguarding and family circumstances are commonly met in this pathway.
5. **A national data set and specialist suppliers exist** (see below), so a general EPR meets an incumbent, not a gap.

## The pathway (antenatal, intrapartum, postnatal, neonatal)

Described from NICE and NHS England pages; the clinical content is for a midwife and an obstetrician to write.

**Antenatal.**
- *Booking.* NICE recommends the first booking appointment with a midwife by 10+0 weeks of pregnancy, and plans 10 routine appointments for a first pregnancy and 7 for later ones (NICE NG201, from a search snippet; the detailed schedule PDF was not opened).
- *Scans and screening.* Dating and anomaly scans, and screening offers (for example infectious disease and blood tests), happen at set points. Their content and timing are not sourced here.
- *Risk assessment.* Risk is assessed at booking and throughout; the Ockenden interim report lists risk assessment through pregnancy as a theme with actions on antenatal pathways (summary sources).
- *Shared care.* Midwife, GP, obstetrician and sometimes a specialist (diabetes, mental health) share care; who leads depends on risk. Which is lead carer, and when it changes, is the question a pathway owner answers ([5.2.1](../05-domain-model.md) rule 3).

**Intrapartum.** A labour record (progress, observations, decisions), a partogram (a chart of labour progress; its detail is not sourced here), and cardiotocography (CTG, the fetal heart and contractions trace) for monitoring. Handover between shifts and between midwife and obstetrician happens during labour. Ockenden's monitoring-of-fetal-wellbeing theme ties to Saving Babies' Lives and named fetal-monitoring leads (summary sources).

**Postnatal.** The mother and baby are both followed: the mother's recovery and mental health, infant feeding, the baby's checks. Care passes from hospital midwives to community midwives, health visitors and the GP.

**Neonatal and newborn screening.** A baby may need neonatal unit care. Newborn screening (the blood spot test, hearing, a physical examination) depends on the baby having a correct identifier; the NHS publishes failsafe processes for missed or rejected samples (gov.uk pages in Sources, not read in full).

## Two patients, one episode: identity

- **Births are notified to the national demographics service.** In England the Personal Demographics Service allocates the baby's NHS number through a birth-notification process, and the service for newborn numbers (NN4B) has been decommissioned into PDS (search snippets; the specification was not read). A maternity system can create the mother's record if she is unmatched.
- **Multiple births.** The newborn blood spot label specification prints birth order and number of births in the confinement (snippet), so twins are distinguished by more than name and date of birth. Whether each twin gets a number at once and how a mother's record is linked was not found.
- **Errors mix babies up.** An NHS England freedom-of-information document on babies switched at birth describes a baby's number and location assigned to the other twin, and twin records entered wrongly through labels, data entry or birth weight (snippet). An NHS case study covers a baby linked to the wrong mother. Newborn screening learning documents describe NHS numbers wrongly written on a card, forcing a repeat test.
- **Name changes.** A baby is often registered with a temporary name ("Baby of ...") and renamed later; a mother's surname may also change. This is a common practice in hospitals, not a sourced fact, and is listed as an open question.
- **This project.** The matcher never auto-links twins or multiples and requires evidence for a link ([patient identity](../patient-identity/index.md); the matcher spec). A mother and her baby are two `Patient` records joined by a related-person or `Patient.link`-style relationship that is **not** an identity link. A merge of mother and baby must be impossible.

## Sensitive information and access

- **Domestic abuse.** NICE's public guidance tells women that disclosures are recorded in a special confidential record, not the hand-held notes (NICE CG110 patient page). A Royal College of Midwives opinion piece describes a secret part of the record and a "partner present" mode that clears sensitive details from the woman's phone view. NHS England guidance for GP online access says it should not be obvious that information has been hidden.
- **Why it matters for design.** A partner may coerce a woman to open her record; the paper maternity book is kept at home (forum snippet, weak source).
- **Safeguarding and teenage parents.** Safeguarding concerns about the unborn baby, or the parents' age or capacity, affect who may see what and who is told. This project's proxy and adolescent confidentiality rules ([access control](../access-control/index.md)) apply, but a teenage mother is both a patient in her own right and, for the baby, a parent with responsibility; no source on the rules was read.
- **Partner as proxy.** A partner may be a legitimate proxy for the baby and not for the mother. The two records need separate proxy sets.

## The woman's own record

- England's Better Births programme promised patient-held digital maternity records: 100,000 women offered access by March 2020, all women by 2023/24, with 105,000 offered and 60% accessing by September 2019 (NHS England page, snippet). Wales planned digital access from March 2026 (trade commentary, snippet).
- Personalised care and support plans were promised to all women by 2021 (snippet; current status not found).
- The woman is the person who holds the whole pregnancy across places; a record she can see and add to fits this project's patient access and patient amendments, with the hidden-information limits above.
- Her record of the baby is a proxy view of a different patient, which the access model must express explicitly.

## Systems, standards and data sets

- **Maternity Services Data Set (MSDS)**, England. Information standard DCB1513, v2.0 (Amd 10/2018), covering activity from first booking to discharge of mother and baby; systems had to conform from 1 April 2019; trusts submit monthly; secondary use only (NHS England standards pages). It is a reporting extract, not a record design.
- **Specialist systems.** BadgerNet (System C) describes itself as a perinatal record covering maternity, neonatal and paediatric pathways (supplier listing). Euroking (Magentus) was widely used; Digital Health reported a 2009 liquidation risk for the original company and, in 2024, 13 trusts moving away after the alert below. K2 was not researched.
- **National Patient Safety Alert, 7 December 2023.** NHS England warned of potential serious risks with the Euroking maternity information system: overwritten and incorrectly displayed patient data; "no evidence of actual harm" was reported; users of all other maternity systems were advised to reassess clinical safety with a clinical safety officer (NHS England and trade coverage; the alert's full text was not read).
- **International.** A Dutch FHIR BirthCare implementation guide (Nictiz) models a pregnancy as a `Condition` on the mother with her care in an `EpisodeOfCare`, linking observations to events. A US HL7 guide for longitudinal maternal and infant health data (research use) exists, and the HL7 Europe Patient Summary maps an EHDS "current pregnancy" item to `Observation` profiles (still being revised). No openEHR pregnancy archetype was found in one search; that is not proof none exists. A pregnancy as `EpisodeOfCare` is the opposite of this project's pathway-first direction; see Ideas.

## What public reviews say about records and information

Secondary summaries only; the reports themselves were not read for records content.
- **Morecambe Bay (Kirkup, 2015).** Coverage reports accusations that some midwives destroyed clinical records, a probable "conscious suppression" of an internal report, and 44 recommendations. The lesson for a record is tamper-evidence: the append-only audit design of this project is relevant.
- **Ockenden final report (2022).** 15 immediate and essential actions under sub-headings (summary counts conflict across sources). Themes seen: risk assessment, fetal monitoring, informed consent with information in several formats, and consultant review with a documented plan in the maternity record. No electronic-record recommendation was found in a snippet search.
- **East Kent, "Reading the signals" (Kirkup, 2022).** Led to a national maternity outcomes signal system (MOSS, launched November 2025 per a search summary). It argues for reading outcome signals early, which needs structured, comparable data.
- **A later Nottingham review** (Ockenden, 2026) was seen only in a result title and is not used.

## Where general EPRs fall short

Evidence is thin; this is largely the author's inference, supported by the Euroking alert.
- A separate specialist system holds the maternity record, so the general EPR sees a summary or nothing, and clinicians re-key results (duplicated entry; unsourced as a rate).
- Mother-baby linkage is a data model concern; the Euroking alert shows data overwritten or wrongly displayed can be a system fault.
- CTG traces come from devices; capture of the trace into the record is a known integration question (not sourced here).
- Acute-shaped systems assume one admission and one patient; a birth makes a second patient inside the first patient's admission.

## Compared with this project

| Need | This project today | Gap |
|---|---|---|
| Pathway as the unit of care | Direction decided ([5.2.1](../05-domain-model.md)); not built | Pathway template for pregnancy, with owner changes |
| Specialty pack | Mechanism described ([care settings](../care-settings.md) s.3); not built | A maternity pack: value sets, forms, alerts, reviewed by clinicians |
| Twin and multiple identity | Matcher never auto-links twins; built | Birth-event creation of baby records, birth order, newborn number from a national service |
| Mother and baby linkage | Not modelled | A non-identity relationship, never mergeable |
| Proxy access | Proxy role and care relationships built for identity | Per-patient proxy sets; partner as proxy for baby only |
| Hidden information | Label masking planned (FR-4.3) | "Not obvious it is hidden", partner-present mode, paper and PDF outputs |
| Woman's own record | Patient role exists; no clinical record yet | A woman-held view; patient amendments; offline use |
| Closed-loop referral | Built for referrals | Scans, screening results, newborn failsafe |
| Device data (CTG) | Not planned | Waveform storage and a time-stamped link to the labour record |
| National data set | None | An MSDS extract; country packs under the equity rule |
| Clinical content | None | A midwife and obstetrician to write it |

## Candidate hazards

Candidates for the Clinical Safety Officer, in the hazard-log shape ([hazard log](../clinical-safety/)). None is accepted; the numbering is deliberately omitted.
1. A baby is linked to the wrong mother, or two babies' data are swapped (see the switched-baby document).
2. Twins are auto-merged or one twin's results are filed on the other.
3. A baby has no identifier, or a temporary one, and screening results are rejected or lost.
4. A hidden disclosure becomes visible to a partner through a proxy view, a printout or a letter.
5. Antenatal information (risk, screening results, plan) does not reach the labour record.
6. Postnatal follow-up for mother or baby is owed by a closed episode and nobody owns it (the H-044 shape).
7. CTG or other device data are not captured, or are shown against the wrong patient or time.
8. Handover during labour omits a documented plan or escalation.
9. Overwritten or wrongly displayed data in a specialist system (the Euroking alert shape) not detected by the integration.
10. A teenage mother's confidentiality and the baby's parental responsibility are confused.
11. A translation of a plan or consent is machine-written and unreviewed.

## Ideas

1. Write a maternity pathway template for [pathway design](../pathway-design.md) with a midwife and an obstetrician, leaving all clinical content as proposals.
2. Model the birth as an event that creates a new baby `Patient` and a mother-baby relationship that the matcher and merge refuse to treat as identity.
3. Add a census-style test: no two records created from one birth event may be merged, and twins are never auto-linked.
4. Allocate identifiers through a country pack (PDS birth notification in England); the core allows a temporary identifier with a reason and a visible "not yet numbered" state.
5. Give the baby its own proxy and consent set, separate from the mother's.
6. Design a hidden-information class with a rule that its existence is not shown to a proxy or the patient's shared view, and that outputs (letters, PDFs, exports) honour it.
7. Make the pregnancy a `CarePlan` pathway, with `EpisodeOfCare` only for reporting; map to Dutch BirthCare and Patient Summary profiles only where they do not force an episode-first model.
8. Plan device data (CTG) as an integration, not a core feature: store the trace as an attachment or waveform with patient, time and device identifiers.
9. Provide an MSDS extract as a reporting pack, derived, never the source of truth.
10. Consider a safety check against the Euroking failure mode: a read-back test that compares displayed data with stored data.
11. Offer a woman-held view first (read, add notes, amend), mobile and offline ([mobile viewability](../mobile-viewability.md)).
12. Decide whether to build a maternity system at all, or integrate with a specialist one and own only the pathway, identity and access layers.

## Open questions

1. Build, or integrate with BadgerNet or similar and own the layers around it?
2. How do temporary baby names and later renaming interact with the matcher and audit trail?
3. Does each twin or triplet receive a national number at birth, and when is the mother linked?
4. Who is the lead carer at each point and how is the handover shown on the pathway?
5. What may a partner or a teenage parent see, in the mother's record and in the baby's?
6. How are hidden disclosures kept from printouts and the paper or exported record?
7. How would CTG data be captured, stored and tied to the right patient?
8. What does a stillbirth, miscarriage or termination do to the baby record, the pathway and the woman's view?
9. Which countries' data sets and screening programmes become packs, under the equity rule?
10. Who clinically reviews it, and how is the review recorded?

## Not verified

- Only search snippets were seen for: NICE NG201, MSDS, Ockenden (final and interim), Kirkup (Morecambe Bay and East Kent), PDS birth notification, the Euroking alert, NHS England maternity digital pages, the Dutch, US and EU FHIR guides, and domestic-abuse recording guidance. No full report was read.
- Not researched: K2, Maternity Digital, openEHR archetypes (one search), CTG capture, partogram content, screening schedules, non-UK systems, legal rules on proxies and teenage parents.
- The "where general EPRs fall short" section is mostly inference.
- No clinician reviewed anything.
- Link check 2026-11-04: these cited pages returned 404. They may have moved, but what rests on them is unconfirmed: https://www.weightmans.com/insights/the-ockenden-review/

## Sources

- Maternity Services Data Set: https://standards.nhs.uk/published-standards/maternity-services-data-set and https://ijpds.org/article/view/3007
- NICE NG201 antenatal care: https://www.nice.org.uk/guidance/NG201/chapter/Recommendations
- Ockenden final report: https://www.gov.uk/government/publications/final-report-of-the-ockenden-review ; interim themes: https://www.weightmans.com/insights/the-ockenden-review/
- Morecambe Bay: https://www.gov.uk/government/news/morecambe-bay-investigation-report-published ; https://www.sands.org.uk/sites/default/files/Morecambe_bay_report.pdf
- East Kent: https://www.gov.uk/government/publications/maternity-and-neonatal-services-in-east-kent-reading-the-signals-report ; response: https://www.gov.uk/government/publications/maternity-and-neonatal-services-in-east-kent-report-government-response
- Euroking alert: https://www.england.nhs.uk/2023/12/identified-safety-risks-with-the-euroking-maternity-information-system/ ; https://digitalhealth.net/2024/04/euroking-patient-safety-alert-13-trusts-switch-supplier ; https://digitalhealth.net/2009/11/euroking-faces-liquidation
- Digital maternity programme: https://www.england.nhs.uk/mat-transformation/harnessing-digital-technology/ ; https://www.digitalhealth.net/tag/maternity-epr/
- Birth notification and newborn numbers: https://standards.nhs.uk/published-standards/birth-notifications ; https://www.gov.uk/government/publications/nhs-numbers-for-newborn-screening-specification-for-the-blood-spot-card-label/nhs-numbers-for-newborn-screening-output-based-specification-for-the-blood-spot-card-label
- Switched babies and twins: https://www.england.nhs.uk/wp-content/uploads/2019/10/FOI_Babies_switched_at_birth.pdf
- Newborn screening failsafe and incidents: https://www.gov.uk/government/publications/newborn-blood-spot-screening-failsafe-solution-user-guide ; https://www.gov.uk/government/publications/newborn-blood-spot-screening-pathway-requirements-specification/learning-from-screening-incidents
- Domestic abuse in records: https://pre.rcm.org.uk/news-views/rcm-opinion/2020/documenting-domestic-abuse ; https://www.nice.org.uk/guidance/cg110/ifp/chapter/Pregnant-women-who-have-a-violent-or-threatening-partner-or-family-member
- FHIR guides: https://informatiestandaarden.nictiz.nl/wiki/Gebz:V1.3_FHIR_IG ; https://build.fhir.org/ig/HL7/fhir-mmm-ig/artifacts.html ; https://build.fhir.org/ig/hl7-eu/eps/map-ehdscurrentpregnancy.html
