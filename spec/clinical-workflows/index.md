# Clinical workflows: the inpatient stay first, then the clinic visit

Public-source research read 2026-11-14 plus this repository's own files. A proposal and a reading, not clinical advice: nothing here is built, and no clinician has reviewed it.

Method: web searches and fetches on 2026-11-14 (FHIR R4 pages read in full or in part; the papers and reports mostly through abstracts, press summaries or search snippets, and "Not verified" says which), and the project's specs: [domain model 5.2.1](../05-domain-model.md), [care settings](../care-settings.md), [pathway design](../pathway-design.md), [virtual ward](../virtual-ward/index.md), [step-down facility](../step-down-facility/index.md), [closed-loop referral](../interoperability/closed-loop-referral.md), [scope](../02-scope.md) (the PAS boundary), [implementation status](../14-implementation-status.md) and [open questions](../16-open-questions.md) (OQ-CP1 to OQ-CP4, OQ-ST1, OQ-PC1). Tasks this refines: CA-1, CA-2, CA-6, EM-5, EM-8, EM-13, XP-5 and XP-6 in [`tasks.md`](../../tasks.md). The companion for booking is [scheduling](../scheduling/index.md).

## What it is

A clinical workflow here is the sequence of decisions and hand-offs a clinician makes with a patient in one setting, and what the record must show at each one. This page covers two:

- **The inpatient stay**: a decision to admit, arrival on a ward and a bed, transfers between wards and teams, the daily round, shift and team handover, and discharge. It is the most hazardous stretch of acute care for information loss, because responsibility changes hands many times in a few days.
- **The clinic visit** (ambulatory): a booked or walk-in contact, check-in, the consultation, the outcome (follow-up, discharge, a test, a referral) and the letter.

Both are written **pathway first** ([5.2.1](../05-domain-model.md)): the admission and the clinic visit are `Encounter`s, dated events on one or more of the patient's care pathways. Discharge ends a ward stay; it never closes a pathway. No rule below depends on an `EpisodeOfCare` existing.

## Evidence that shapes the design

- **Handover.** In nine paediatric residency programmes (10,740 admissions), the I-PASS handover bundle was followed by a 23% fall in medical errors (24.5 to 18.8 per 100 admissions) and a 30% fall in preventable adverse events (4.7 to 3.3), with no change in non-preventable events [W6]. The bundle was training, a mnemonic and a printed or electronic handover tool together, not software alone. I-PASS is: **I**llness severity (stable, "watcher", unstable), **P**atient summary, **A**ction list (with owner and timing, and "none" said aloud), **S**ituation awareness and contingency plans ("if this, then that"), **S**ynthesis by the receiver [W7]. An observational study found the first S and the severity label were often left out in practice [W7, snippet].
- **Handover failure is a recognised sentinel-event cause.** The Joint Commission's Sentinel Event Alert 58 (2017) links inadequate hand-offs to delayed treatment, falls and medication errors, and recommends using the EHR to support hand-offs; it does not endorse one tool [W8, secondary summaries].
- **After discharge.** In 400 consecutive medical patients, 19% (95% CI 15 to 23%) had an adverse event in the weeks after discharge; two thirds were adverse drug events [W9, abstract]. A systematic review found a discharge summary was available at the first post-discharge visit in only 12 to 34% of cases, direct doctor-to-doctor contact happened in 3 to 20%, and test results were missing from 33 to 63% of summaries [W10, secondary summaries].
- **Follow-up intended at discharge and never booked.** England's healthcare safety investigation body investigated a patient discharged twice with a plan for clinic follow-up; neither appointment was made. Follow-up was free text spread across a four-page discharge notification, responsibility was split between hospital and GP, the clinic code was missing, and "there was no feedback loop ... to ensure that a follow-up appointment had been made"; a clerk said "the 'safety net' is the patient". The report found one trust whose EPR tracked every outpatient automatically [W11]. This is the strongest single reason for the discharge-to-booking loop in workflow 6 below.
- **Discharge planning.** NICE guidance for adults with social care needs asks for a named discharge coordinator from admission, a discharge plan agreed with the person, a contact name in the community team, and the discharge summary to the GP within 24 hours [W12, snippets of NG27 and QS136].
- **Discharge summary content.** England's professional record standards body publishes the eDischarge summary headings; the implementation guide marks six as mandatory (demographics, GP practice, person completing the record, diagnoses, clinical summary, allergies and adverse reactions) and ten as required, including "plan and requested actions", "information and advice given" and "safety alerts" [W13]. Which version is current (2.1 or a later 3.x) is [uncertain] [W14, snippet].
- **Bed occupancy.** A simulation model found risks appear when average acute bed occupancy passes about 85%, with regular shortages and periodic crises at 90% or more [W15]; a later paper warns that the 85% figure has been turned into a planning rule the model does not support [W16, snippet]. The board should show occupancy; it should not hard-code a target.
- **Patients on the wrong ward ("outliers", "boarders").** The evidence conflicts: a prospective Italian study found higher in-hospital mortality (OR 2.1) for medical outliers; a UK retrospective study found longer stays and no mortality difference; a systematic review could not pool the studies [W17, snippets]. Enough to show the outlier state on the board and in handover, not to claim harm.
- **Flow bundles.** Guidance used in the UK (the "SAFER" bundle) asks for a senior review before midday, an expected date of discharge with clinical criteria for discharge, early transfer from assessment units, and review of long stays, with "red" and "green" days [W18, snippets]. A single-centre audit found all patients had a senior review before midday but only 10% left before midday; delays were medicines, staffing, pharmacy and transport, and the authors note limited evidence for the bundle [W19].

## How others do it

- **Epic.** Grand Central is described as Epic's ADT and bed-management module (bed planning, discharge tracking, cleaning and transport tasks); Cadence is its outpatient scheduling module [W20, vendor blogs and a hospital poster, not Epic documentation].
- **Oracle Health.** A customer case study describes CareAware Capacity Management with Patient Flow, Transfer Center and a command-centre dashboard, with automated bed cleaning and transport dispatch [W21, vendor-sourced].
- **OpenMRS and Bahmni (open source).** The OpenMRS bed-management module holds beds, admission locations and a REST API, and Bahmni's bed-management app does admit, transfer and discharge to a bed [W22]. Older Bahmni pages list as unsupported: undoing a discharge, admission without a bed, and two patients in one bed [W22]; the first is a correction path this project must have (as an append-only cancel), the second is a real case (a patient admitted to a ward with no bed yet) and the third should stay refused. An "intended discharge" date can be configured [W22].
- **OpenEMR** is ambulatory-first; see [scheduling](../scheduling/index.md).

What none of the public material showed: how a vendor links a discharge plan's follow-up to a booked appointment and alarms when it is not booked. That loop is where this project can do better than "the patient is the safety net".

## Standards

- **FHIR R4 `Encounter`** [W1]. `status`: planned, arrived, triaged, in-progress, onleave, finished, cancelled, entered-in-error, unknown; "a patient IS admitted when the status is in-progress". `statusHistory` and `classHistory` keep the trail (an emergency encounter that becomes inpatient). `location[]` has a status (planned, active, reserved, completed) and a period, so a ward and bed move can be a new location entry; a move to another service may instead be a new encounter with `partOf`. `hospitalization` holds `admitSource`, `dischargeDisposition` and `destination`. `basedOn` points at the `ServiceRequest` that caused it, which is how this project links an encounter to its pathway (OQ-CP2). `Encounter` "is not to be used to store appointment information".
- **FHIR R4 `EpisodeOfCare`** [W2]: "a tracking resource, rather than a planning resource"; statuses planned, waitlist, active, onhold, finished, cancelled, entered-in-error. Kept administrative here (5.2.1 rule 4).
- **FHIR R4 `Location`** [W3]: a hierarchy through `partOf` (site, ward, room, bed); `physicalType` (bed, room, ward are example codes); `mode` instance or kind (a "kind" for "an isolation room" not yet chosen); `operationalStatus` bound to the v2 bed-status table: Closed, Housekeeping, Occupied, Unoccupied, Contaminated, Isolated [W4].
- **HL7 v2 ADT** [W5]. A01 admit, A02 transfer, A03 discharge, with A11, A12 and A13 cancelling each; message structures for the cancels differ between implementation guides [W5, snippets]. This project reads A01, A04, A08, A28 and A31 for identity today ([service spec](../../electronic-patient-record-service-with-rust/spec/index.md)); the movement events are not read.
- **Transfer of care.** The eDischarge headings above [W13]; HL7's `Composition` is the planned carrier (EM-8).

## The inpatient workflows

Each step names what the clinician needs on screen. "Data" lists the record items; items marked *(built)* exist today.

### 1. Decision to admit

1. **Open the patient**, then their open pathways, before the admission form: owner, current step, next step, overdue items. Data: pathways (CA-2), allergies *(built)*, latest observations *(built)*, watch-list entries *(built)*, consent and care relationships *(built)*.
2. **Record the decision to admit** as a request (`ServiceRequest`): the reason, the admitting team and named consultant, the specialty wanted, urgency, isolation need, and the pathway it belongs to (or "not on a pathway", said). The decision and the time are separate from arrival: a decided-but-not-arrived patient is a state with a clock, not an absence.
3. **Bed request** goes to the bed board with the specialty, sex or privacy need, isolation need and any equipment. Data: bed board (workflow 3).

### 2. Arrival and admission

1. **Arrival on the ward** creates the `Encounter` (class inpatient, status in-progress) with its first `location` (ward, then bed when known; ward with no bed is allowed and shown as such).
2. **Admission checks the record does not do for the clinician**, shown as a checklist with "not done" as a state, never a blank: identity confirmed and wristband (two identifiers), allergies reviewed (a "no known allergies" statement is a recorded fact, an empty list is "not recorded"), observations and an early warning score (NEWS2 is not built; see EM-1 to EM-3), medicines reconciliation started (RC-5, RC-10), risk assessments due (falls, pressure, VTE, delirium: assessments are built, the catalogue is a pack), resuscitation and treatment-escalation status, capacity and next of kin.
3. **Responsible clinician and team** are recorded as facts with a start time (not inferred from the care-relationship check, which governs access, not responsibility).
4. **Expected date of discharge and the criteria for it** are set early and shown on the board, with who set them; changing them keeps the old value (append-only) and a reason.

### 3. The bed board and bed management

1. **One row per bed** per ward: occupied, unoccupied, closed, cleaning, contaminated or isolated (the v2 table [W4]), the patient (name, age, sex, a short reason), the responsible team, NEWS2 when built, isolation, expected date of discharge, and the outlier flag (patient's specialty differs from the ward's).
2. **Pending lists beside the beds**: decided to admit and waiting (with time since decision), expected discharges today and tomorrow, transfers requested.
3. **Occupancy summary** per ward and site, shown as a number and a fraction, with no built-in threshold colour unless a deployment configures one with a source [W15, W16].
4. **An isolation flag is never filtered off** the board or the banner (EM-5 acceptance).
5. **Allocation is a person's decision**; the system may suggest, never moves a patient by itself.

### 4. Transfer (ward to ward, team to team)

1. **Request** (from the sending team or bed management): reason, receiving ward or team, urgency.
2. **Accept** by the receiving team: responsibility changes on acceptance and arrival, not on the request (the same rule as the [virtual ward](../virtual-ward/index.md) "Who is responsible").
3. **Hand over** with the structured handover note (workflow 5) for the patient, and outstanding tasks and pending results move to the receiving team's worklist by name, not by default.
4. **Move**: the old `location` entry gets an end time and status completed; the new one starts. A transfer that is undone is a cancel event with a reason (the A12 idea), not an edit.
5. **On screen for the receiver**: why the patient came, the action list, pending results with who ordered them, the escalation plan, isolation and alerts.

### 5. Daily round and handover

1. **Ward or team list** (EM-13): each patient with illness severity, early warning score and trend, outstanding tasks with owners and due times, pending and unacknowledged results (results with sign-off are built), new allergies, watch-list entries, and expected date of discharge.
2. **Handover note per patient**, structured on I-PASS headings [W7] (the choice between I-PASS and SBAR is OQ-IW2): the sender writes severity, summary, actions and contingencies; the receiver records a synthesis or at least an acknowledgement with name and time. The note quotes live data (observations, results) read-only and shows when each was taken; it never copies them as text that can go stale unnoticed.
3. **A patient cannot drop off the list** while they have an overdue task, a pending abnormal result or an unacknowledged handover (EM-13 acceptance). Leaving the list needs a reason (discharged, transferred and accepted, or died).
4. **Shift handover is between named people**, recorded; a list printed for handover shows its print time on every page.

### 6. Discharge

The detailed step list for discharge to a monitored service is the twelve steps of the [virtual ward](../virtual-ward/index.md), with the bed differences in [step-down facility](../step-down-facility/index.md). For an ordinary discharge home:

1. **Medically fit and criteria met**: the expected date and its criteria shown, with what is still outstanding (pharmacy, transport, equipment, social care, family).
2. **Discharge summary** (`Composition`, EM-8) drafted from the record: diagnoses, what was done, medicines with changes and why (RC-10), allergies, results and results still pending with who will act on them, information given to the patient, and **plan and requested actions** as structured items, each with an owner (hospital team, GP, community team or patient) [W13].
3. **Each follow-up action becomes a tracked step**, never free text only: "clinic in 6 weeks" makes a booking request on the scheduling side (see [scheduling](../scheduling/index.md), workflow 3) with a due window, and the step stays open on the pathway until an appointment is booked or a named person records why not [W11]. A step owned by the GP is sent and shown as "sent, not confirmed" until acknowledged.
4. **Pending results at discharge** stay owned by a named clinician after the patient leaves (a result arriving after discharge goes to that clinician's worklist, not to the ward's).
5. **Sign and send** the summary; an unsigned summary cannot be sent (EM-8). The time sent is shown against the NICE 24-hour expectation [W12] without the system claiming it.
6. **End the ward stay**: `Encounter` status finished, `dischargeDisposition` and `destination` set, the bed goes to cleaning. The pathway remains open with its next step and owner; a pathway with no next step and no owner is listed (5.2.1 rule 3).
7. **Cancel discharge** (the A13 idea) is an append-only event with a reason, for the patient who did not leave.

## The ambulatory workflow: the clinic visit

1. **Before the clinic**: the clinic list from the schedule ([scheduling](../scheduling/index.md)) with, per patient, the reason for the visit (the referral or the follow-up step that caused it), results since the last visit, and anything the patient sent.
2. **Check-in** sets the appointment to arrived and creates the `Encounter` (class ambulatory, or virtual for video and telephone: EM-9). Identity is confirmed at check-in.
3. **Consultation**: observations, assessments, the note (not built), orders and referrals as pathway steps.
4. **Outcome is mandatory**: one of follow-up (with a window and a reason), discharge from this clinic back to a named owner, test then decide, referral on, or patient-initiated follow-up (a PAS function under [scope](../02-scope.md)). A follow-up outcome makes a booking request exactly as in discharge step 3.
5. **Letter** (`Composition`, EM-8) to the GP and the patient, with the same "plan and requested actions" structure.
6. **Did not attend** is not an outcome the clinician leaves blank: the clinician decides rebook, contact, or discharge back to the referrer, and for a child "was not brought" with a safeguarding review (see [scheduling](../scheduling/index.md), rules).

## Rules

Each is a sentence a test can check.

1. An `Encounter` can be created with no `EpisodeOfCare`, and every screen and rule works without one.
2. Ending an inpatient `Encounter` (status finished) never changes the status of any care pathway.
3. An `Encounter` with status in-progress and class inpatient has exactly one `location` entry with status active at the ward level at any time; a bed is optional.
4. A transfer, discharge or admission that is reversed is a new cancel event with a reason; no stored row is updated or deleted.
5. Responsibility for an inpatient changes only on a recorded acceptance by the receiving team; a transfer request alone leaves the sending team responsible.
6. A bed in status occupied names exactly one current patient; assigning a second patient to it is refused.
7. An isolation status shows on the bed board and the patient banner under every filter.
8. A patient with an overdue task, a pending abnormal result or an unacknowledged handover note stays on the handover list.
9. A handover note quotes observations and results with their own timestamps and cannot be edited after signing; a correction is an addendum.
10. A discharge summary cannot be sent unsigned.
11. Every follow-up action in a signed discharge summary or clinic letter is a structured step with an owner and a due window; free text alone is refused or flagged as "not tracked".
12. A follow-up step with no booked appointment by the end of its window appears on the owning team's worklist, until booked or closed with a named person's reason.
13. A result arriving after discharge goes to the named clinician who owns it, never only to the ward.
14. A clinic visit cannot be closed without an outcome; a missing outcome is a listed item, never a default of "discharged".
15. The expected date of discharge keeps every earlier value with who changed it and why.
16. An unknown or unrecorded item (allergies, observations, resuscitation status) is shown as not recorded, never as none.

## Hazards to add when built

Described here, not added to the [hazard log](../../governance/clinical-safety/hazard-log.csv). The first code for each area adds its row in the same commit (AGENTS.md); none is accepted or closed by the project.

- **IW-H1 Patient in the wrong bed or two patients in one**: the board shows a stale or wrong location and a clinician treats the wrong person.
- **IW-H2 Responsibility gap at transfer**: neither team believes it is responsible between request and acceptance.
- **IW-H3 Lost handover item**: a task, pending result or contingency is not carried to the next shift or team, or the patient drops off the list.
- **IW-H4 Stale quoted data in a handover**: a value copied into a note is read as current.
- **IW-H5 Follow-up intended but never booked** [W11].
- **IW-H6 Result after discharge unowned** [W10].
- **IW-H7 Discharge summary late, missing or wrong** (medicine changes, pending results) [W9, W10].
- **IW-H8 Pathway closed by discharge**: the patient leaves and nothing more is owed in the record, though care is.
- **IW-H9 Isolation status hidden** and an infected patient placed beside a vulnerable one.
- **IW-H10 Outlier patient missed on the round** because the ward list belongs to another specialty [W17].
- **IW-H11 Decided-to-admit patient invisible** while waiting for a bed.

## First slice

The smallest buildable piece that is useful and safe to test: **a ward stay and a handover list**, with no bed allocation logic.

- **Store**: append-only `encounter_versions` with a current view (status, class, patient, period, `basedOn` request, pathway reference or the reason there is none), and `encounter_location_versions` for ward and bed with periods; `locations` (site, ward, bed, operational status); triggers refuse update, delete and truncate, as the referral store does. Events: admit, transfer, discharge, and the three cancels, each with an actor, time and reason where the event needs one (a pure state table first, like `referral.rs`).
- **Routes**: `GET`/`POST /api/patients/{id}/encounters`, `PUT …/encounters/{encounter_id}` with an event and `expected_version`; `GET /api/wards/{id}/patients` (the list). No version segment ([09](../09-api-surface.md)); a policy resource `Encounter`; audit rows in the same transaction.
- **Screen**: a ward list (patient, bed or "no bed", responsible team, observations and pending results from what is built, expected date of discharge) and an admit/transfer/discharge form on the patient page.
- **Tests**: the state table (every event from every state); append-only triggers; one bed one patient; a cancel restores the earlier state as a new row; ending an encounter does not touch pathways (once CA-2 exists, or a placeholder test now); the list keeps a patient with a pending abnormal result (fake clock); axe on the screen.
- **Not in the slice**: a bed board with cleaning and transport, a discharge summary, the handover note, ADT outbound, NEWS2.

## What this repo has and lacks

| Need | This project today | Gap |
|---|---|---|
| Patient identity, banner, merge | Built | None for this purpose |
| Allergies, observations, results with sign-off, assessments, watch lists | Built | No early warning score, no trend on a ward list |
| Consent, care relationships, break-the-glass | Built | Relationship is access, not responsibility |
| Referral loop and worklist | Built | Not linked to an encounter, bed or appointment |
| HL7 ADT inbound | Identity events only | A02, A03 and the cancels not read; no location |
| Encounter, Location, bed board | None | Whole area (CA-1, EM-5) |
| Care pathways | Designed (5.2.1) | Not built (CA-2) |
| Handover list and note | None | EM-13 |
| Discharge summary, clinic letter | None | EM-8; no notes, problems or medicines (RC-5) |
| Follow-up as a tracked step | None | Needs CA-2 and the scheduling port |

## Not verified

- The I-PASS and Joint Commission statements come from abstracts, institutional press and secondary summaries; the NEJM and alert full texts were not read. The Forster and Kripalani figures are from abstracts and secondary summaries. The NICE recommendations were seen in snippets; the exact NG27 wording was not read. The outlier and SAFER evidence is from snippets except the neurosurgery audit [W19], which was read.
- The safety investigation report [W11] was read in part (about the first 100,000 characters); the remaining text, including any later findings, was not.
- Vendor descriptions (Epic, Oracle Health) are from blogs and case studies, not product documentation. Bahmni statements are from pages marked as older releases.
- Which eDischarge version is current is [uncertain]. Whether a ward move should be a new `Encounter` or a new `location` entry in each profile in use (UK Core, US Core) is not settled (OQ-IW1).
- No clinician, ward manager or bed manager has read this.

## Sources

- [W1] HL7 FHIR R4, Encounter: https://hl7.org/fhir/R4/encounter.html
- [W2] HL7 FHIR R4, EpisodeOfCare: https://hl7.org/fhir/R4/episodeofcare.html
- [W3] HL7 FHIR R4, Location: https://hl7.org/fhir/R4/location.html
- [W4] HL7 Terminology, v2 table 0116 bed status: https://terminology.hl7.org/5.0.0/CodeSystem-v2-0116.html
- [W5] HL7 Terminology, v2 table 0003 event types: https://terminology.hl7.org/1.0.0/ValueSet-v2-0003.html ; message structures (v2.3.1): https://www.hl7.eu/HL7v2x/v231/hl7v231msgstruct.htm
- [W6] Starmer et al., Changes in medical errors after implementation of a handoff program, NEJM 2014;371:1803-12 (via press release): https://www.sciencedaily.com/releases/2014/11/141105203533.htm ; PSNet: https://psnet.ahrq.gov/perspective/handoffs-and-transitions
- [W7] I-PASS elements: https://psnet.ahrq.gov/primer/handoffs ; https://medicine.okstate.edu/site-files/documents/ipass.pdf ; observational study (snippet): https://pmc.ncbi.nlm.nih.gov/articles/PMC9923540
- [W8] The Joint Commission, Sentinel Event Alert 58, inadequate hand-off communication (2017): https://digitalassets.jointcommission.org/api/public/content/a05e74ef89484e2084b6511189b73a99 ; PSNet summary: https://psnet.ahrq.gov/issue/inadequate-hand-communication
- [W9] Forster et al., The incidence and severity of adverse events affecting patients after discharge from the hospital, Ann Intern Med 2003;138:161-7: https://pubmed.ncbi.nlm.nih.gov/12558354/
- [W10] Kripalani et al., Deficits in communication and information transfer between hospital-based and primary care physicians, JAMA 2007;297:831-41 (PSNet summary): https://psnet.ahrq.gov/issue/deficits-communication-and-information-transfer-between-hospital-based-and-primary-care
- [W11] Healthcare safety investigation body (England), Outpatient appointments intended but not booked after inpatient stays (2021): https://www.hssib.org.uk/patient-safety-investigations/outpatient-appointments-intended-but-not-booked-after-inpatient-stays/investigation-report/
- [W12] NICE NG27 recommendations: https://www.nice.org.uk/guidance/ng27/chapter/Recommendations ; QS136 statement 3: https://www.nice.org.uk/guidance/QS136/chapter/quality-statement-3-coordinated-discharge
- [W13] eDischarge headings (FHIR implementation guide): https://nhsconnect.github.io/ITK-FHIR-eDischarge/explore_headings.html
- [W14] PRSB transfers of care standards: https://theprsb.org/projects/maintainingourstandards/
- [W15] Bagust, Place and Posnett, Dynamics of bed use in accommodating emergency admissions, BMJ 1999;319:155-8: https://pmc.ncbi.nlm.nih.gov/articles/PMC28163
- [W16] ANZAM 2012 paper on the 85% rule (snippet): https://anzam.org/wp-content/uploads/pdf-manager/295_ANZAM-2012-214.PDF
- [W17] Medical outliers: systematic review, J Gen Intern Med 2019 (snippet): https://link.springer.com/10.1007/s11606-019-05008-4 ; UK retrospective study (snippet): https://researchportal.bath.ac.uk/en/publications/are-medical-outliers-associated-with-worse-patient-outcomes-an-an/ ; SISIFO study (snippet): https://www.citedrive.com/en/discovery/clinical-and-safety-outcomes-of-patients-admitted-to-different-specialty-wards-the-prospective-multi-center-controlled-sisifo-study/
- [W18] SAFER bundle guides (snippets): https://sbuhb.nhs.wales/go/discharge-support/further-discharge-information/using-the-safer-principles-to-manage-hospital-flow/ ; https://www.england.nhs.uk/south/wp-content/uploads/sites/6/2016/12/rig-red-green-bed-days.pdf
- [W19] Irvine et al., Factors affecting patient flow in a neurosurgery department, Ann R Coll Surg Engl 2020: https://pmc.ncbi.nlm.nih.gov/articles/PMC6937602
- [W20] Epic Grand Central and Cadence (secondary): https://ehealth.connect-care.ca/epic-systems/epic-modules/grand-central ; https://www.mindbowser.com/epic-cadence-a-guide-for-healthcare-scheduling/
- [W21] Oracle Health customer case study, University of Tennessee Medical Center: https://www.oracle.com/customers/university-of-tennessee-medical-center/
- [W22] Bahmni, Bed Management module: https://bahmni.atlassian.net/wiki/spaces/BAH/pages/707854444/Bed+Management+Module ; Admit, discharge and transfer patients: https://bahmni.atlassian.net/wiki/spaces/BAH/pages/114923665/Admit+Discharge+and+Transfer+Patients
