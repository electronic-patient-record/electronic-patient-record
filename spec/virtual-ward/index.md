# Virtual ward: discharge into it from this EPR

Public-source research read 2026-11-04 plus this repository's own files. A proposal and a reading, not clinical advice: nothing here is built, and no clinician has reviewed it.

Method: web searches (mostly search snippets, not full documents; "Not verified" says which) and the project's specs: [care settings](../care-settings.md), [domain model 5.2.1](../05-domain-model.md), [mobile viewability](../mobile-viewability.md), [closed-loop referral](../interoperability/closed-loop-referral.md), [open questions](../16-open-questions.md) (OQ-PC1, OQ-CP1 to OQ-CP4), [scope](../02-scope.md). Related: [care homes](../care-homes/index.md) (if written yet), [data exchange](../data-exchange/index.md). The companion for beds is [step-down facility](../step-down-facility/index.md); this file holds the full step list, that file the differences and the transfer-of-care handover.

## What it is

A virtual ward (England) gives acute-level monitoring and care to a patient at home or in a care home, run like a ward: a named clinical lead, a defined capacity, admission criteria, a daily review and an escalation route. Other names: hospital at home, in Australia "hospital in the home" (HITH), in the US the "Acute Hospital Care at Home" programme. It is a different thing from an ordinary community nursing caseload because the hospital (or a provider on its behalf) stays clinically responsible and the patient is counted as occupying a "bed".

- **Step-up** (avoids an admission, from the GP, ambulance or emergency department) and **step-down** (supports earlier discharge from a ward). This file's subject is step-down from an acute stay; step-up shares the same record needs.
- **Pathways named by England's national health body**: acute respiratory infection (including COPD) and frailty have guidance notes; a national heart-failure note was not found (see Not verified).
- A virtual ward is not a **step-down facility** (a bed in a building): see the [companion](../step-down-facility/index.md).

## Models and evidence

- **England.** England's national health body published guidance in 2022 asking integrated care systems to introduce or extend virtual wards, with a 2022/23 ambition of 40 to 50 "beds" per 100,000 population; a 2025 report says the per-capita target was shelved, and December 2024 data showed England at 20.1 per 100,000 with one integrated care board in range (secondary sources, snippets). An operational framework (August 2024, secondary report) and monthly capacity/occupancy statistics from a situation report exist. A virtual-wards minimum data set, DAPB4073, is listed as active from 4 February 2026 ([standard](https://standards.nhs.uk/published-standards/virtual-wards)).
- **Clinical leadership.** The acute respiratory infection note says the ward should be led by a named consultant (a nurse or allied health professional consultant counts) or a suitably trained GP, with specialist advice. The ICS leadership guide asks for an accountable clinical service lead reporting to the medical director and chief nurse of the organisation (snippets).
- **Evidence, admission avoidance.** The 2024 Cochrane update (Edgar et al., CD007491; five trials, 1,502 people) found probably little or no difference in mortality at six months, probably lower cost with variable method, higher satisfaction, and possibly a useful alternative for a selected group of older people (snippet of the review page).
- **Evidence, early discharge.** A Cochrane review of 2017 (Gonçalves-Bradley et al., CD000356) covers early discharge to hospital at home; only an older secondary summary was seen (satisfaction higher, readmission no clear difference after surgery, cost mixed).
- **Limits.** The trials are of selected patients in specific services and may not transfer to a national roll-out of mixed quality; the Cochrane qualitative synthesis (2024) is about implementation. A Bristol study found NEWS2 on a remote-monitored virtual ward a poor predictor of (re)admission; a Scottish Health Technologies Group item of 2026 says decisions should use several measures, and devices should be validated across skin tones (snippets). No national safety investigation of virtual-ward alerting was found.
- **Elsewhere.** The US Acute Hospital Care at Home waiver was extended to 30 September 2030 by the Consolidated Appropriations Act 2026 (trade press and a state memo, not the statute). In Australia, hospital in the home is described by a society position statement and applied through state guidelines under the national safety and quality standards (Queensland, Victoria); the national commission itself does not appear to define it. These are examples, not a rule: [equity rule](../02-scope.md).

## Who is responsible

- **Responsible clinician.** One named person or role for the patient while on the ward (consultant, trained GP, advanced practitioner) and one clinical lead for the ward. The patient's GP is told but is not by default the responsible clinician.
- **Ward owner.** A team that holds the list of patients, the daily round, the alert queue and the discharge decision. In the [pathway model](../05-domain-model.md) (5.2.1 rule 3) this is the owner of a step on the patient's pathway; the acute team's pathway step ends only when the virtual ward has accepted.
- **Relationship model.** Responsibility moves only on acceptance, never on the sending of a referral. The built care-relationship check can say who may see the record; it does not say who is responsible. OQ-CP1 asks who owns and closes a pathway.
- **Out-of-hours and escalation.** Who answers an alert at 02:00, and who decides to readmit, is stated per ward (not found as a national rule).

## The discharge workflow

The one numbered workflow both files share (idea 1 below). Each step names what the clinician needs on screen.

1. **Open the patient's pathway** (not only the admission): current owner, goals, open steps, active alerts. Data: pathway, encounter, allergies, consent.
2. **Check eligibility** against the ward's published criteria: clinical stability, a diagnosis the ward takes, a score range (for example NEWS2), a monitored home with power, phone and signal, a carer where needed, and nothing that blocks (safeguarding concern, no capacity and no one to act for the patient, unsafe home). Data: latest observations, capacity, home and household assessment, safeguarding flags, resuscitation status.
3. **Discuss and record consent** of the patient (or the decision-maker under capacity law) and, where the home is shared, the household's agreement to equipment and visits; record the explanation given, the alternative (stay in hospital) and the patient's choice. Data: consent record with who, when, what was said.
4. **Choose the destination** from a list showing each ward's pathway, hours, current capacity and acceptance time. Data: service directory, capacity feed. A step-down bed is chosen on the [companion page](../step-down-facility/index.md).
5. **Send a structured referral** as a pathway step: reason, diagnoses, medicines and allergies, observations and trend, devices needed, thresholds, escalation plan, ReSPECT or equivalent, who to call, expected stay. Data: referral with attachments; the [referral state table](../interoperability/closed-loop-referral.md).
6. **Wait for acceptance** (state `accepted`, or `declined`, `information-requested`, `deferred`, `redirected`). The patient is not discharged on `requested`; a timeout puts an unanswered request on the referrer's worklist (H-044).
7. **Prepare the home**: equipment delivered and tested, patient taught, first contact booked, medicines supplied (a few days of supply and a plan for changes). Data: task list with owners.
8. **Hand over** clinician to clinician for anything unstable, with the structured summary; record who spoke to whom and when.
9. **Set monitoring and escalation**: which measures, how often, who looks at the data, the alert thresholds for this patient (not only the ward default, with the reason for any change), the readmission route and its phone number. Data: monitoring plan; observations feed.
10. **Close the acute stay** only when 6, 7 and 9 are done: the ward stay ends; the pathway stays open and owned by the virtual ward.
11. **Daily round** by the virtual ward; step-down from the virtual ward (to discharge, to the GP or community team, or back to hospital) is again a referral or a hand-back with its own loop.
12. **Close the pathway or hand it over** to a named owner with outcome recorded (5.2.1 rule 3).

## What must travel

| Item | Why | Source in our record today |
|---|---|---|
| Identity, national health number, contact details, address and access | Find the patient; visit; call back | Patient (matcher DTO); no contact or access detail model |
| Diagnoses and reason for the ward | Eligibility, thresholds | None (no problem list or notes) |
| Medicines, changes made, supply given | Safe continuation | None (not built; see RC-5) |
| Allergies and adverse reactions | Avoid harm | Built |
| Latest and baseline observations | Baseline for alerts | Built (vital-sign observations) |
| Alert thresholds and escalation plan | Who acts and when | None |
| Resuscitation and treatment escalation (ReSPECT) | Decision at home | None |
| Capacity and consent | Lawful care | Consent built (access); no treatment consent or capacity |
| Safeguarding and home risks | Staff and patient safety | None |
| Equipment and remote-monitoring devices | Monitoring works | None |
| Who is responsible; who to call; hours | Escalation | None (relationship model is access only) |

## Compared with this project

| Need | This project today | Gap |
|---|---|---|
| Referral with accept/decline and timeout | State table and worklist built | Not tied to a pathway, bed or capacity; no outbound message |
| Pathway stays open after discharge | Designed (5.2.1) | Not built |
| ADT, beds, discharge | HL7 v2 ADT inbound only | No discharge action |
| Remote observations in | Vital-sign observations stored | No device or app feed, no source or device identity, no alerts |
| Responsibility on acceptance | Care-relationship access | No responsible-clinician record |
| Medicines on transfer | None | Whole area |
| Mobile for visiting staff | Read-only needs written | See [mobile viewability](../mobile-viewability.md) |
| Capacity and occupancy report | None | Minimum data set DAPB4073 to read |

## Candidate hazards

For the Clinical Safety Officer; none is accepted or closed. To go to the hazard log (AS-1) with the first code.

- V-1 Patient discharged to a service that never accepted, or declined after acceptance (the loop closes late).
- V-2 Patient at home with no monitoring owner (nobody reads the alerts, or the owning team is unlisted).
- V-3 Alert thresholds set wrong, left at default, or changed without reason; NEWS2 trusted alone.
- V-4 Remote observations lost (device off, signal, sync failure) and read as "no change"; absent is not normal.
- V-5 Medication list not transferred, or supply runs out.
- V-6 Responsible clinician unclear, or two claim it.
- V-7 Pulse oximeter inaccuracy across skin tones read as reassurance.
- V-8 Home or household unsafe, or consent from the wrong person.
- V-9 Escalation phone unanswered out of hours.
- V-10 Patient recorded as "discharged" while the pathway has no owner.

## Ideas

1. **One discharge-workflow definition** for any destination (ward, bed, home, care home) as a pathway pack template, with steps 1 to 12 above, shared by both pages; the destination changes the criteria, not the shape (links CA-1, CA-3, PA-14).
2. **Discharge is blocked until acceptance** is shown, with a visible override that asks for a reason and goes to audit; never a silent send.
3. **A "who is responsible" panel** on the first view of a pathway: owner team, named clinician, out-of-hours route, and since when.
4. **Monitoring plan record** with measure, frequency, threshold, reason, and an "owner reads at" time; missing data is shown as missing with a reason.
5. **Remote observations carry source and device** so that missing, late and out-of-range readings are distinguishable (task XP-7 is for legal status; observations ingestion needs its own task).
6. **Capacity feed** from each ward (free, used, accepted today) read by the destination picker; start with a manual figure.
7. **Read the virtual-wards minimum data set** DAPB4073 and decide which fields we can supply for reporting.
8. **Mobile first for the visiting clinician**: the pathway, the plan, the contacts and the alert history on a phone ([mobile viewability](../mobile-viewability.md)).
9. **Readmission route as a pathway step** that returns the patient to acute care with the virtual-ward data attached.
10. Revisit OQ-PC1 (hospital at home in scope for 1.0, currently "no") once pathways exist.

## Open questions

- Is the virtual ward a destination for the referral loop only, or does our EPR also run the ward's worklist (OQ-PC1)?
- Who owns the alert queue when devices come from a supplier's app (is that app the record)?
- How is a household's consent recorded when the patient lives with others?
- Which thresholds are defaults, who sets them, and who reviews a change (OQ-PD1)?
- Where is the line between an observation and a patient-reported measure?
- What does a worldwide-equitable ward look like without a mobile network or a device (phone call, community health worker)?

## Not verified

Most statements from England's national health body and Cochrane come from search snippets or secondary summaries, not full documents. Not seen: the operational framework text, the frailty note, any heart-failure note, the 2017 Cochrane full text, the DAPB4073 specification, the ReSPECT link to transfer standards, the US statute text, Australian national statements. Figures about bed targets are from trade press. No clinician or lawyer has reviewed anything.
- Link check 2026-11-04: these cited pages returned 404. They may have moved, but what rests on them is unconfirmed: https://standards.nhs.uk/future-standards/virtual-wards-minimum-data-set; https://www.nj.gov/health/healthfacilities/certificate-need/guidance/Extension-of-CMS-Acute-Care-Hospital-at; https://hithsociety.wildapricot.org/Definition

## Sources

- England's national health body, ARI virtual ward guidance note: https://england.nhs.uk/long-read/guidance-note-virtual-ward-care-for-people-with-acute-respiratory-infection-including-chronic-obstructive-pulmonary-disease/
- England's national health body, supporting clinical leadership in virtual wards: https://www.england.nhs.uk/long-read/supporting-clinical-leadership-in-virtual-wards-a-guide-for-integrated-care-system-clinical-leaders/
- England's national health body, virtual ward statistics: https://england.nhs.uk/statistics/statistical-work-areas/virtual-ward
- Public Technology, per-capita target shelved: https://www.publictechnology.net/2025/05/19/health-and-social-care/nhs-virtual-wards-add-1000-beds-in-the-past-year-as-per-capita-target-shelved/
- HTN on the 2022 guidance: https://htn.co.uk/2022/05/05/nhse-publishes-guidance-for-icss-to-introduce-virtual-ward-model/
- Standards of England's national health body, virtual wards minimum data set: https://standards.nhs.uk/published-standards/virtual-wards and https://standards.nhs.uk/future-standards/virtual-wards-minimum-data-set
- Cochrane, admission avoidance: https://cochranelibrary.com/cdsr/doi/10.1002/14651858.CD007491.pub3
- Cochrane, early discharge: https://cochranelibrary.com/cdsr/doi/10.1002/14651858.CD000356.pub3/information
- Cochrane news on implementation: https://www.cochrane.org/news/cochrane-examines-evidence-base-effectiveness-and-implementation-hospital-home-programmes
- Bristol, NEWS2 in remote monitoring: https://research-information.bris.ac.uk/en/publications/clinical-acuity-and-national-early-warning-scores-news2-of-remote/
- Scottish Health Technologies Group, virtual wards: https://admin.shtg.scot/media/monhbnev/2026-05-28-virtual-wards-rec-019.pdf
- US extension (state memo): https://www.nj.gov/health/healthfacilities/certificate-need/guidance/Extension-of-CMS-Acute-Care-Hospital-at Home-Program-until-2030.pdf
- US bill text H.R. 4313: https://www.govinfo.gov/content/pkg/BILLS-119hr4313eh/html/BILLS-119hr4313eh.htm
- Queensland hospital in the home: https://health.qld.gov.au/system-governance/policies-standards/guidelines/hospital-in-the-home
- Victoria HITH guidelines: https://www2.health.vic.gov.au/sites/default/files/migrated/files/collections/policies-and-guidelines/1/110702_doh-hith-guidelines-web---pdf.pdf
- HITH Society Australasia definition: https://hithsociety.wildapricot.org/Definition
- PRSB transfer of care toolkit: https://theprsb.org/toolkits/transfersofcare/about-this-toolkit/
