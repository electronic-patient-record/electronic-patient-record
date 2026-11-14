# Scheduling: schedules, slots, appointments, waiting lists and missed visits

Public-source research read 2026-11-14 plus this repository's own files. A proposal and a reading, not clinical advice: nothing here is built, and no clinician has reviewed it.

Method: web searches and fetches on 2026-11-14 (the FHIR R4 and IHE pages read in part; reviews and reports mostly through abstracts, review summaries or search snippets, and "Not verified" says which), and the project's specs: [scope](../02-scope.md) (the PAS boundary and the scheduling port, PA-1, OQ-PA1), [domain model 5.2.1](../05-domain-model.md), [pathway design](../pathway-design.md), [closed-loop referral](../interoperability/closed-loop-referral.md), [equity audit](../equity-audit.md), [implementation status](../14-implementation-status.md). Tasks this refines: CA-4 (scheduling), CA-5 (portal reads appointments through the port), CA-6 (worklists), EM-9 (virtual consultations) in [`tasks.md`](../../tasks.md). The clinical side of the clinic visit and of discharge follow-up is in [clinical workflows](../clinical-workflows/index.md).

## What it is

Scheduling decides when and where a patient will be seen and by whom. It has four parts:

- **Availability**: a clinician's, room's or service's bookable time (a clinic template: which session, which slot lengths, which slot types, for new or follow-up patients, for urgent or routine).
- **Booking**: putting a patient into a slot, by staff (this page) or by the patient (the portal, CA-5, out of scope here).
- **Waiting**: a patient who needs an appointment but has none yet (a waiting list or a follow-up due), with a clock and a clinical priority.
- **What happens after**: attended, cancelled by the patient or the provider, rescheduled, did not attend.

**Boundary.** Under the proposed PA-1 split ([scope](../02-scope.md)), a deployment with a patient administration system (PAS) keeps appointments, waiting lists, referral-to-treatment clocks, reminders and letters there; this EPR reads them through a **scheduling port** and provides a **minimal scheduler** only for a deployment without a PAS. The clinical parts stay in the EPR either way: why the appointment exists (the pathway step that asked for it), whether it happened, and what is still owed when it did not.

**Why it is clinical.** A missed or never-booked appointment is a clinical event. England's healthcare safety investigation body found intended follow-up after discharge that was never booked, with no feedback loop and the patient as the safety net [S9], and separately found patients with glaucoma losing sight while follow-up was delayed, with follow-up patients at the greatest risk of avoidable sight loss in ophthalmology [S10]. The scheduler is where those loops can close.

## Evidence that shapes the design

- **Reminders.** A Cochrane review (2013 update; 8 trials, 6,615 people) found text-message reminders raised attendance compared with no reminder (risk ratio 1.14, 95% CI 1.03 to 1.26; 7 trials, 5,841 people; moderate certainty) and worked about as well as phone-call reminders (RR 0.99, 0.95 to 1.02; 3 trials), at lower cost per attended appointment; no trial reported health outcomes, and failed delivery and privacy loss were not studied [S6]. So reminders are worth sending, they are not a safety measure on their own, and a failed send must be visible.
- **Who misses appointments.** In Scotland, a linkage study of 824,374 general-practice patients found missed appointments a marker of all-cause mortality with a dose response; press summaries report about three times the mortality for people with long-term physical conditions missing two or more a year, and about eight times for those with mental-health conditions only [S7; the multiples are from the university's summary, not the abstract]. A missed appointment is a signal about the patient, not only lost capacity.
- **Children who were not brought.** Safeguarding guidance across the UK replaces "did not attend" with "was not brought" for children, because a child cannot bring themselves and missed appointments can be early signs of neglect; policies say a child must not be discharged from a service for missed appointments alone without a safeguarding assessment [S8, policies seen in snippets].
- **No-shows and overbooking.** A systematic review of 105 studies found an average no-show rate of the order of 23% across specialties, with long lead time and prior no-shows the main predictors [S11, snippet]. A rapid review of trials using predictive models to target no-shows found too much heterogeneity to pool and raised concerns about overbooking's effect on patients [S12, snippet].
- **Overbooking can be unfair.** Scheduling patients predicted to miss into overbooked slots made Black patients in one specialty clinic wait about 30% longer than other patients; a method that minimised the longest-waiting group's wait removed the disparity at similar cost [S13, abstract]. Any no-show prediction is an equity question for the [equity audit](../equity-audit.md), not a tuning parameter.
- **Letters people cannot read.** The same safety body found appointment letters for diagnostic tests sent in English only, leading to missed appointments and loss to follow-up, and recommended written appointment information in other languages [S14, snippet].

## How others do it

- **Epic.** Cadence is described as Epic's central scheduling module: provider calendars, templates, room booking, routing rules and cancellation tracking [S15, vendor and consultancy blogs, not Epic documentation].
- **Oracle Health.** Millennium has a scheduling application; third parties integrate with it [S16, vendor marketing]. Its documentation was not found in public.
- **OpenEMR (open source).** Appointment statuses are a configurable list (an administrator adds codes and symbols); "no show" is one status among them; users report that one status for every missed visit made it impossible to tell no-shows from cancellations and reschedules [S17]. A separate **Recall Board** holds patients who need a follow-up booked; a recall disappears from the board once an appointment is booked within three months of the recall date [S17]. The lesson: missed-visit reasons need distinct codes, and a follow-up due list is a first-class list.
- **Bahmni (open source, on OpenMRS).** An appointments module books against a service and a provider, and a "waitlist" appointment can be saved with no date [S18].

## Standards

- **FHIR R4 `Schedule`** [S2]: "a container for slots of time that may be available for booking appointments"; `actor` (practitioner, role, location, healthcare service, device), `planningHorizon`, `serviceType`; it "does not provide any information about actual appointments".
- **FHIR R4 `Slot`** [S3]: `schedule` (required), `status` (busy, free, busy-unavailable, busy-tentative, entered-in-error), `overbooked` ("too many appointments allocated to it"); one slot may hold several appointments (a group session of five); a slot can be busy with no appointment (a blocked session).
- **FHIR R4 `Appointment`** [S1]: `status` proposed, pending, booked, arrived, fulfilled, cancelled, noshow, entered-in-error, checked-in, waitlist; `cancelationReason` "only used for appointments that have been cancelled, or no-show"; `slot[]`; `basedOn` the `ServiceRequest` "this appointment is allocated to assess" (the link to the pathway step); participants each with a status (accepted, declined, tentative, needs-action). An appointment usually leads to an `Encounter` "when the service starts, not when the patient arrives"; emergency care makes an `Encounter`, not an `Appointment`.
- **FHIR R4 `AppointmentResponse`** [S4]: a participant's reply (accepted, declined, tentative, needs-action); the appointment's status is updated when all have replied.
- **IHE ITI Scheduling** [S5]: built on FHIR R4 from the Argonaut scheduling guide; operations `$find` (potential appointments, required), `$hold` (optional) and `$book` (book, cancel or reschedule from a proposal `$find` returned; required). This is the natural shape for the **scheduling port**: the EPR calls `$find` and `$book` whether the other side is a PAS or the minimal scheduler.
- **HL7 v2 SIU** [S19]: S12 new booking, S13 reschedule, S14 modification, S15 cancellation, S26 no-show; not every system sends S26, and lost S13 or S15 messages leave stale slots [S19, implementer guides and snippets; chapter 10 of the standard not read]. An inbound SIU feed is how a PAS's appointments reach the EPR where FHIR is not offered.

## The workflows

Each step names what the person needs on screen.

### 1. Build a clinic template

1. **Choose the owner** of the schedule: a clinician, a team or service, or a room (`Schedule.actor`).
2. **Define sessions** (day, start, end, site, room, recurrence, start and end dates) and **slot types** within them: length, new or follow-up, urgent or routine, face to face, telephone or video (EM-9), group with a capacity.
3. **Preview** the generated slots for the next weeks before publishing; a changed template affects only slots with no booking, and a slot with a booking is never silently removed (a clash list goes to the booking team).
4. **Block** a session (leave, training) with a reason: booked patients in it are listed for rebooking, never cancelled by the system.

### 2. Book an appointment (staff)

1. **Start from the reason**: the referral, the follow-up step or the request it fulfils (`basedOn`), shown with its priority, its due window and its pathway. An appointment with no reason is allowed and says "not on a pathway" ([5.2.1](../05-domain-model.md) rule 2).
2. **Find** slots (`$find`) filtered by service, clinician, slot type, site, date range; the patient's own constraints (interpreter and language, access needs, transport, times they cannot come) are shown and the interpreter or adjustment is booked with the slot ([reasonable adjustments](../reasonable-adjustments/index.md)).
3. **Book** (`$book`): one patient per single slot. **Overbooking** a full slot is allowed only where the schedule permits it, asks for a reason, marks the slot overbooked and is audited.
4. **Confirm** to the patient (letter, message) in their recorded language where a translation exists; the confirmation's send status is stored.
5. **On screen after booking**: the appointment on the patient's pathway as the step's next date; the step is no longer "due, not booked".

### 3. Follow-up due and the waiting list

1. **A follow-up request** comes from a discharge summary or a clinic outcome ([clinical workflows](../clinical-workflows/index.md)) with a due window (for example "6 to 8 weeks"), a clinical priority and an owner; or a new referral is accepted onto a list.
2. **The waiting list** shows each patient with the reason, the priority, the date added, the due window and how far past it they are; sorting by clinical risk is possible, not only by date (risk-based follow-up is the eye-care lesson [S10]).
3. **Past due** puts the item on the owning team's worklist (CA-6) with the time overdue; it stays there until booked or closed with a named person's reason.
4. **Validation** (contacting patients on a long list to see whether they still need care) is a PAS function under [scope](../02-scope.md); the EPR records the clinical outcome of it.

### 4. Cancel or reschedule

1. **Who cancels** is recorded: patient or provider (clinic cancelled, clinician absent). The reason is coded, not free text only.
2. **A provider cancellation** puts the patient back on the list with their original due date and priority, not a new one, and is shown on the pathway.
3. **A reschedule** keeps the history (the original and the new time); a patient who has cancelled repeatedly is visible to the clinician.

### 5. Did not attend, was not brought

1. **The clinic marks the patient as not attended** after the session (status noshow), distinct from a late cancellation.
2. **The clinician decides** what happens next: rebook, contact the patient, discharge back to the referrer with a letter, or (for a child or an adult at risk) "was not brought" with the safeguarding steps of the deployment's policy [S8]. The appointment stays an open item on the clinic's worklist until that decision is recorded.
3. **The record shows the pattern**: missed appointments across services, with dates, visible on the patient's pathway, without a score that claims a risk [S7].
4. **No automatic discharge** after a set number of missed appointments.

### 6. Reminders

1. **A reminder schedule** per appointment type (for example 7 days and 2 days before) by text, email or letter, in the patient's language where translated; the content says the minimum (a time and place, not a specialty that discloses a diagnosis) [S6 notes privacy was not studied].
2. **Each send has a status** (queued, sent, failed, opted out); a failed reminder is visible on the clinic list.
3. **Opt-out and contact preferences** come from the patient's record; a patient with no usable contact detail is listed, not silently skipped.

### 7. The clinic list on the day

Per session: patients booked, with arrival state (not arrived, arrived, with clinician, seen), the reason, the interpreter booked, reminder status, and anything new since booking (results, a message). Check-in creates the `Encounter` ([clinical workflows](../clinical-workflows/index.md), the clinic visit).

## Rules

Each is a sentence a test can check.

1. A slot of capacity one accepts a second booked appointment only if its schedule allows overbooking and the request carries a reason; the slot is then marked overbooked and an audit row is written.
2. Booking the same patient twice into overlapping times is refused unless a reason is given.
3. An appointment's status moves only along the published state table; cancelled, noshow, fulfilled and entered-in-error are final, and a rebooking is a new appointment linked to the old one.
4. No stored appointment, slot or waiting-list row is updated in place or deleted; every change is a new version with an actor and a time.
5. `cancelationReason` is required for cancelled and noshow, and records who cancelled (patient or provider).
6. A provider cancellation returns the patient to the waiting list with the original due date and priority.
7. A follow-up request with a due window and no booked appointment by the end of the window appears on the owning team's worklist until booked or closed with a reason by a named person.
8. A did-not-attend appointment stays on the clinic's worklist until a clinician records the next step; there is no automatic discharge after N missed appointments.
9. For a patient under 18 (or an adult flagged at risk by the deployment), a missed appointment is recorded as "was not brought" where the deployment's policy says so, and the safeguarding step of that policy is offered.
10. Changing a clinic template never removes or moves a slot that holds a booking; the affected bookings are listed.
11. Every reminder has a delivery status; a failed or impossible reminder shows on the clinic list.
12. A reminder message holds no diagnosis or specialty beyond what the deployment configures as safe.
13. An appointment read through the scheduling port shows the time it was last read from the PAS; data older than the configured limit is shown as stale, never as current.
14. A missing contact detail, language or interpreter need is shown as not recorded, never as "none needed".
15. Any no-show prediction used to overbook is off by default and, when switched on, reports waiting time by the groups the equity audit names.

## Hazards to add when built

Described here, not added to the [hazard log](../../governance/clinical-safety/hazard-log.csv). The first code adds its row in the same commit (AGENTS.md); the project never sets one accepted or closed.

- **SC-H1 Intended appointment never booked** (follow-up lost between the plan and the booking) [S9].
- **SC-H2 Patient waits past a safe interval** on a follow-up list sorted by date alone [S10].
- **SC-H3 Missed appointment treated as administrative**: discharged after DNAs with no clinical decision; a child not brought, missed as a safeguarding sign [S7, S8].
- **SC-H4 Double booking or unsafe overbooking**: two patients for one clinician's slot, rushed consultations, long waits that fall on one group [S13].
- **SC-H5 Wrong patient or wrong clinic booked** (identity or clinic code error; compare the missing clinic code in [S9]).
- **SC-H6 Stale appointments from the PAS** shown as current; a lost cancellation leaves a slot or a patient expected [S19].
- **SC-H7 Reminder discloses sensitive information** or goes to the wrong contact.
- **SC-H8 Reminder failure unseen**: the patient never knew and is recorded as a DNA [S14].
- **SC-H9 Template change silently drops booked patients**.

## First slice

The smallest buildable piece: **a follow-up-due list with a minimal booking**, because the never-booked follow-up is the evidenced harm.

- **Store**: append-only `schedule_versions`, `slot_versions`, `appointment_versions` (each with a current view, triggers refusing update, delete and truncate); `appointment_versions` holds the patient, slot, status, `basedOn` (the referral or a follow-up request), cancellation reason and who; a pure state table (`appointment.rs`) like `referral.rs`. A follow-up request is the referral loop's request with a due window, or a small `follow_up_versions` table if CA-2 is not ready (OQ-SC2).
- **Routes**: `GET`/`POST /api/schedules`, `GET /api/schedules/{id}/slots` (the `$find` shape), `POST /api/patients/{id}/appointments` (book, with an idempotency key), `PUT …/appointments/{appointment_id}` (an event: arrive, fulfil, cancel, no-show; with `expected_version`), `GET /api/worklists/follow-up-due`. No version segment ([09](../09-api-surface.md)); policy resource `Appointment`; audit rows in the same transaction.
- **Screen**: a follow-up-due list (patient, reason, due window, days overdue, owner) with a "book" action that opens free slots; the patient page shows the appointment as the step's next date.
- **Tests**: the state table; double-booking refused; overbooking with a reason allowed only where configured (the CA-4 acceptance); a follow-up past its window appears on the worklist (fake clock); a provider cancellation restores the original due date; append-only triggers; axe on the screens.
- **Not in the slice**: templates with recurrence, reminders (CA-4's email test is the next slice), SIU inbound, the PAS port, waiting-list validation, patient booking.

## What this repo has and lacks

| Need | This project today | Gap |
|---|---|---|
| Patient identity, contact details (telecom) | Built (matcher DTO) | No contact preferences, language or interpreter need |
| Referral loop with timeouts and a worklist | Built (PC-1, PC-3, CA-3 slices) | `Book` carries no date; no appointment |
| Watch lists, results sign-off worklists | Built | Not linked to appointments |
| Schedule, Slot, Appointment | None | Whole area (CA-4) |
| Scheduling port to a PAS | Proposed (PA-1) | Not designed in detail; IHE `$find`/`$book` is the candidate |
| SIU inbound | None | Only ADT identity events are read |
| Reminders | None | No message sending exists |
| Waiting list | None | Basic list only, per PA-1 |

## Not verified

- The Cochrane figures were read on the review summary page, not the full review; the 2013 update is not the newest search date and a later update was not looked for.
- The Scotland mortality multiples come from a university press release; the overbooking and no-show reviews were seen in snippets; the overbooking-bias paper was read as an abstract.
- The "was not brought" policies are local UK policies seen in search snippets; no national standard was read.
- HL7 v2 chapter 10 was not read; the SIU event meanings come from implementer guides.
- The IHE profile version (1.0.0 or a later build) to target is [uncertain].
- Epic and Oracle Health statements are vendor or third-party material. OpenEMR's Recall Board page is old and may not match the current release.
- No clinician, booking clerk or safeguarding lead has read this.

## Sources

- [S1] HL7 FHIR R4, Appointment: https://hl7.org/fhir/R4/appointment.html
- [S2] HL7 FHIR R4, Schedule: https://hl7.org/fhir/R4/schedule.html
- [S3] HL7 FHIR R4, Slot: https://hl7.org/fhir/R4/slot.html
- [S4] HL7 FHIR R4, AppointmentResponse: https://hl7.org/fhir/R4/appointmentresponse.html
- [S5] IHE ITI Scheduling, volume 1: https://profiles.ihe.net/ITI/Scheduling/volume-1.html ; `$book`: https://profiles.ihe.net/ITI/Scheduling/OperationDefinition-appointment-book.html
- [S6] Gurol-Urganci et al., Mobile phone messaging reminders for attendance at healthcare appointments, Cochrane CD007458 (2013): https://www.cochrane.org/evidence/CD007458_mobile-phone-messaging-reminders-attendance-healthcare-appointments
- [S7] McQueenie et al., Morbidity, mortality and missed appointments in healthcare, BMC Medicine 2019: https://doaj.org/article/8726f50dfd414d0e89238a59bd35dfdd ; university summary: https://www.abdn.ac.uk/news/12622/
- [S8] "Was not brought" guidance (examples): https://www.coventry.gov.uk/downloads/file/37446/was-not-brought ; https://remedy.bnssg.icb.nhs.uk/media/v4tbugcl/bnssg-childrens-wnb-policy-v3-jan25.pdf ; origin (REF impact case): https://results2021.ref.ac.uk/impact/a90af37a-f4d4-494e-8ebd-0e8b5ede70c9/pdf
- [S9] Healthcare safety investigation body (England), Outpatient appointments intended but not booked after inpatient stays (2021): https://www.hssib.org.uk/patient-safety-investigations/outpatient-appointments-intended-but-not-booked-after-inpatient-stays/investigation-report/
- [S10] Healthcare safety investigation body (England), Lack of timely monitoring of patients with glaucoma: https://www.hssib.org.uk/patient-safety-investigations/lack-of-timely-monitoring-of-patients-with-glaucoma/investigation-report/
- [S11] Dantas et al. (attribution [uncertain]), No-shows in appointment scheduling: a systematic literature review (snippet): https://ouci.dntb.gov.ua/works/4NodRevl
- [S12] Predictive-model interventions to reduce outpatient no-shows, rapid review (snippet): https://pmc.ncbi.nlm.nih.gov/articles/PMC9933067
- [S13] Samorani et al., Overbooked and overlooked: machine learning and racial bias in medical appointment scheduling, M&SOM 2022;24(6):2825-42: https://papers.ssrn.com/abstract=3467047
- [S14] Healthcare safety investigation body (England), Booking systems failures: written communications in community languages (2023): https://hssib.org.uk/patient-safety-investigations/clinical-investigation-booking-systems-failures-written-communications-in-community-languages/investigation-report
- [S15] Epic Cadence (secondary): https://www.mindbowser.com/epic-cadence-a-guide-for-healthcare-scheduling/
- [S16] Cerner Millennium scheduling integration (vendor marketing): https://www.notablehealth.com/lp/beyond-the-portal/beyond-the-portal-patient-access
- [S17] OpenEMR wiki, appointment statuses: https://www.open-emr.org/wiki/index.php/AppointmentStatuses-v7 ; Recall Board: https://www.open-emr.org/wiki/index.php/Recall_Board ; forum on no-show versus cancellation: https://community.open-emr.org/t/cancelled-appointments-and-no-shows/8683
- [S18] Bahmni, Appointments Scheduling Module: https://bahmni.atlassian.net/wiki/spaces/BAH/pages/117677537/Appointments+Scheduling+Module
- [S19] HL7 v2 SIU (implementer guides): https://www.hl7.eu/HL7v2x/v25/std25/ch10.html ; https://home.meditech.com/en/d/greenfield/otherfiles/schapptoutbound24.pdf ; https://rhapsody.health/resources/hl7-siu-message/
