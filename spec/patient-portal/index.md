# Patient portal: results, messages, booking, bills and sharing

Public-source research read 2026-11-14 plus this repository's own files. A proposal and a reading, not clinical advice: nothing on this page is built, and no clinician has reviewed it.

Method: web searches (many answers are search snippets, not full documents; "Not verified" says which) and the project's specs: [scope](../02-scope.md) (the portal row of the PAS boundary, and billing out of scope for 1.0), [implementation status](../14-implementation-status.md), the [UI spec](../../electronic-patient-record-ui-with-svelte/spec/index.md) (`/my-record`, the access log, proxy terms, amendment requests), the [service spec](../../electronic-patient-record-service-with-rust/spec/index.md) (results and sign-off, consent, care relationships, proxy scopes, the `patient-access` purpose of use), and tasks CA-5, CA-6, SE-2, SE-6, IO-2, IO-5, IO-8, PA-5, RC-4 and OR-4 in [tasks.md](../../tasks.md). Style follows [virtual ward](../virtual-ward/index.md). Sources are keyed [S1] and so on, listed at the end. A claim marked *(snippet)* rests on a search summary only; *[uncertain]* means sources disagree or the claim could not be checked.

## What it is

A patient portal is the part of the record a patient, or someone acting for them (a proxy), uses directly. This page covers five things a patient asks of it: (1) see my results, (2) message my care team, (3) book an appointment, (4) pay a bill, and (5) share my record. It does not cover questionnaires, letters or a medicines list, which CA-5 also names.

The [scope](../02-scope.md) already decides part of the shape (PA-1, proposed): the portal owns **clinical content** (results, letters, messages, record download, proxy access); a PAS owns **booking, rebooking and reply links**, and the portal reads appointments through the scheduling port. Billing and claims are **out of scope for 1.0**.

**Built today:** patient and proxy sign-in through Keycloak; `/my-record`, which lists the records linked to the signed-in person, read under purpose `patient-access`; a patient access log with filters and a CSV download (SE-6); proxy terms with a basis, scopes (`identity`, `allergies`, `photo`, `access-log`) and an end date (PA-5); amendment requests by a patient for vital signs and allergies (RC-4). Results exist as append-only versions with a version-named sign-off (OR-4 first slice), but the policy gives a patient and a proxy **no** access to them "until release rules exist". **Not built:** a portal results view, messaging, booking, billing or sharing.

## 1. Results

### How others do it

- **US, immediate release.** Since the information-blocking rule of the 21st Century Cures Act, many US systems release results to the portal as soon as they are final. Federal guidance treats an intentional, unnecessary delay of electronic health information as likely interference, and gives holding lab results so a clinician can speak first as an example *(snippet, via FAQ summaries)* [S1][S2]. The Preventing Harm exception (45 CFR 171.201) covers a delay only on an individual determination, no broader than necessary, and in most patient-access cases only for a risk of **physical** harm [S3]. An association asked in 2025 for emotional harm to count; no change was found *(snippet)* [S2].
- **Patients' view (Steitz et al. 2023).** A survey of 8,139 portal users at four US academic centres (all on Epic MyChart): 95.7% preferred immediate release, 95.3% among those with non-normal results; 7.5% said reading results before speaking to a clinician increased worry, rising to 16.5% for abnormal results against 5.0% for normal ones; counselling before release was not linked to less worry *(snippet of the article and press release)* [S4][S5].
- **England.** For general practice, prospective record access has been switched on for most patients aged 16 and over with an online account from late 2023: new entries, test results included, appear in England's national patient app *(snippet)* [S6]. Results become visible to the patient when the practice **files** them, so the guidance says abnormal results should ideally be shared with the patient before filing [S7]. Historic results before 2023 may not show [S8].
- **Australia.** From 13 October 2025 most pathology appears in the national record as soon as it is uploaded; anatomical pathology, cytopathology and genetic tests keep a delay, cut from 7 to 5 days; most imaging moves to a 5-day delay in early 2026 (February or March: sources differ, *[uncertain]*) [S9][S10]. The national medical association opposed removing the delay *(snippet)* [S11].
- **Epic MyChart** releases results under each organisation's own rules; we did not find a public primary source for its default rule table *(null: no vendor document seen)*.

### Notes as well as results (OpenNotes)

In a year-long study at three US sites (105 primary care physicians, 13,564 patients with a note available), most patients read at least one note, few reported confusion, worry or offence, and up to 5% of doctors reported longer visits; author order and figures are from secondary sources *(snippet)* [S12]. A 2020 survey of more than 23,000 note readers found patient-reported errors most often in diagnoses, medical history and medications [S13]. This project has no notes yet; the lesson for results is that patients find errors, so a results view needs a way to report one (RC-4 amendment requests).

### Standards

- FHIR R4 `DiagnosticReport` (the report and its status) and `Observation` (each value, with interpretation and reference range) [S14][S15].
- SMART App Launch 2.2 `patient/` scopes, with granular scopes such as `patient/Observation.rs?category=…|laboratory` to limit an app to lab results [S16].
- US: 45 CFR 171 (information blocking) [S3]. England: the general practice guidance on filing [S7]. Australia: the 5-day delay categories [S9]. Each is a **deployment rule pack**, not code ([scope](../02-scope.md), equity rule).

### Patient workflow

1. The patient (or a proxy with a `results` scope) opens `/my-record` and chooses a record.
2. The page lists results that the release rule has released, newest first, each with name, date taken, date released, status (`preliminary`, `final`, `corrected`) and whether a clinician has signed off **this version** (the built `sign_off` field).
3. A result not yet reviewed says so in words: "A clinician has not reviewed this result yet. If you are worried, contact …". It never reads as normal.
4. The patient opens one result: value, unit, reference range, the lab's flag, and any comment a clinician added for the patient. A missing range shows "No reference range was sent", never a blank.
5. The patient may ask a question about it (section 2) or report an error (RC-4).
6. Each view is in the patient's access log as an access by the patient or proxy.

### Rules

- R-RES-1. A result is shown to a patient or proxy only when the deployment's release rule for its kind has released it; a result with no matching rule is held, with the reason "no release rule", and appears on a staff worklist.
- R-RES-2. The release rule is a table keyed by kind of result (and, where a pack says so, by sensitivity label) with one of: `immediate`, `after-sign-off`, `after-delay(N days)`, `never-online`. Each row names its source (law, guidance or local decision).
- R-RES-3. In a pack where immediate release is the law (US), a single result may be held only by a named clinician's recorded, per-patient decision with a reason and an end date; a blanket hold by kind of result is refused by the pack's checker.
- R-RES-4. A released result that has no sign-off for its current version shows "not yet reviewed" text; the patient view never infers "normal" from no flag.
- R-RES-5. A correction is a new version. The patient sees the latest version and a note that it changed; earlier versions stay in the record (append-only) and are visible on request.
- R-RES-6. An `entered-in-error` result disappears from the list and is replaced by "A result was withdrawn on DATE"; it is not silently removed.
- R-RES-7. A proxy sees a result only with a `results` scope in force at the time (PA-5), and never one labelled adolescent-confidential (R-PRX below).
- R-RES-8. Every view of a result by a patient or proxy writes an audit row under purpose `patient-access`.

## 2. Messaging

### How others do it

- **Volume.** In Epic Cosmos data from 2020 to 2025, patient-authored portal messages rose from 0.99 to 2.50 per patient per year (153%), office visits rose 17% and telephone encounters fell 6%; growth in messages was not linked to fewer office visits; lower use in socially vulnerable neighbourhoods was noted (Long et al., JAMA, June 2026) *(snippet of press coverage)* [S17][S18].
- **Billing for messages (US).** At one academic centre, after clinicians could bill a message needing medical decision-making as an e-visit, billed e-visits rose from 0.26% to 1.40% of threads, and patient message threads fell by about 3,388 a week; a portal interface change happened at the same time (Holmgren et al., JAMA 2023) [S19]. A 2025 study reports that messages are unevenly spread across patients and physicians *(title only; full text refused)* [S20].
- **Emergencies.** In 7,322 patient messages at one US primary care practice, about 3.5% of a sample described potentially high-risk symptoms (chest pain, breathing, palpitations); screening subject lines for high-risk words had a sensitivity of 15%; many messages waited over a closed office; adverse outcomes linked to messages were rare (North et al., JAMIA 2013) [S21]. Health systems commonly tell patients not to use the portal for emergencies *(snippet)* [S22].
- **Adolescent accounts.** At three US children's hospitals, an estimated 64% to 76% of adolescent portal accounts with outbound messages were used by guardians (Ip et al., JAMA Netw Open 2021) [S23]. A message "from the patient" may not be from the patient.

### Standards

- FHIR R4 `Communication` (a message sent) and `CommunicationRequest` (a request that a message be sent) [S24][S25].
- The team inbox and triage are CA-6.

### Patient workflow

1. The patient (or a proxy with a `messages` scope) chooses "Message your care team" on a record.
2. Before typing, the page states, every time: "Do not use this for an emergency. Call [local emergency number] or [urgent line]." and the expected reply time for this team.
3. The patient chooses a topic (question about a result, medicines, appointment, other) and a team from the patient's care relationships; free choice of any clinician is not offered.
4. The patient writes the message and sends it. The page shows "Sent at TIME. Expect a reply by DATE." and a reference.
5. The message lands in the team's inbox (CA-6), not one person's, so leave and absence do not strand it.
6. A clinician or delegate replies or converts it (to a call, an appointment, an order). The thread is part of the record; nothing is deleted.
7. If no one has acted by the expected time, the message escalates on the team's worklist, and the patient sees "No reply yet. If you still need help, call …".

### Rules

- R-MSG-1. Every compose screen shows the not-for-emergencies text and the team's reply time; the text is configured per deployment and cannot be blank.
- R-MSG-2. A message goes to a team with a current care relationship to the patient; a message to a team without one is refused with a reason.
- R-MSG-3. Messages are append-only: no edit or delete by sender or reader; "sent in error" is a new entry.
- R-MSG-4. A message carries who sent it (patient or named proxy, from the token), never only the record it is about.
- R-MSG-5. An unacted message escalates after the team's configured time (fake-clock test); "unread" is never shown as "answered".
- R-MSG-6. No keyword screen is relied on for safety; if one is added, it can only add urgency, never lower it.
- R-MSG-7. A proxy may read and send messages only with a `messages` scope; a thread an adolescent started under confidentiality is never shown to a proxy.
- R-MSG-8. Whether a message is billable is not decided in the portal (section 4).

## 3. Booking

### How others do it

- **Self-scheduling evidence.** At an orthopaedic practice, online-booked visits had a no-show rate of 4.5% against 4.4% for staff-booked (no significant difference) [S26]. At Mayo Clinic, self-scheduling of well-child visits did not raise no-shows, and 29.5% of self-scheduling actions were out of business hours (North and Nelson, 2021) *(snippet)* [S27]. Larger reductions quoted on vendor sites come from non-peer-reviewed sources and are not used here.
- **Epic MyChart** offers scheduling where the organisation configures it; one Epic discussion says its `$find` support was not built for patient-facing use *(snippet)* [S28].

### Standards

- FHIR R4 `Appointment`, `Slot` and `Schedule` [S29]. `Appointment/$find`, `$hold` and `$book` come from the Argonaut Scheduling IG (STU3, 2018) [S30]; IHE ITI Scheduling moves them to FHIR R4 with changes [S31]. No R5 core `$find` was found *(null: not in the search; unverified)*.

### Patient workflow

1. The patient sees upcoming appointments for the record, read through the scheduling port, with the time the data was read ("as of HH:MM").
2. For a visit type the deployment allows, the patient asks for times; the PAS (or CA-4's minimal scheduler) returns offered slots.
3. The patient chooses one and confirms; the booking is made in the PAS, which answers booked or refused.
4. Cancel or rebook follows the same path.
5. Any reasonable adjustment on the record (PA-6) is shown beside the appointment.

### Rules

- R-BK-1. The portal shows appointments only through the scheduling port and says how old the data is; a failed read shows "Could not load appointments", never "No appointments".
- R-BK-2. Self-booking is offered only for visit types a deployment lists; a type that needs triage is a request, not a booking.
- R-BK-3. A proxy books only with an `appointments` scope.
- R-BK-4. A booking is confirmed to the patient only after the PAS confirms it.

## 4. Bills

### Does billing belong in an EPR?

**No, not in this one.** Billing is out of scope for 1.0 ([scope](../02-scope.md)), and it is country-specific in a way the clinical record is not. Our proposal: billing lives in a **separate billing or revenue system**; the portal at most shows a link to it, or a read-only statement through a billing port. The EPR may supply what a bill is made from (an encounter happened, a procedure was done) but never prices, balances or payments. Epic, by contrast, runs bills, estimates and payment plans inside MyChart [S32][S33], because Epic sells revenue-cycle software as one product.

### How rules differ by country

- **US.** Statements, card payment and payment plans are normal portal features [S32][S33]. Under the No Surprises Act, a provider must give an uninsured or self-pay patient a good-faith estimate on request, or when a service is booked three or more business days ahead; any talk about cost counts as a request; deadlines run from one to three business days by case [S34][S35]. Hospitals must publish standard charges and either list at least 300 shoppable services or offer an online price estimator (45 CFR 180.60) [S36].
- **England (and similar tax-funded systems).** Most care is free at the point of use, so most patients get no bill. Exceptions: hospital charges to overseas visitors not ordinarily resident, under 2015 regulations with exemptions (primary care is outside them) [S37][S38]; prescription charges (£9.90 an item in England for 2025/26, frozen for 2026/27; exemptions apply), paid at the pharmacy, not through a record [S39][S40]; private patients treated by a public hospital *(null: no primary source read)*.
- **Elsewhere.** Not researched *(null: outside this pass)*.

### Standards

FHIR R4 `Account`, `ChargeItem`, `Invoice` and `PaymentNotice` exist. `Invoice` is maturity level 0 (trial use), built from the `ChargeItem`s on an `Account` priced by `ChargeItemDefinition` [S41][S42]. Low maturity is a further reason not to build on it here.

### Patient workflow (if a deployment links a billing system)

1. The patient sees "Bills and payments" only when the deployment has configured a billing system.
2. The link opens that system, which does its own sign-in or receives a short-lived handoff; the EPR does not see card details.
3. Where the billing system offers a read-only statement through the port, the portal shows it with its age.

### Rules

- R-BILL-1. The EPR stores no price, balance, card or payment data.
- R-BILL-2. Bills are hidden for a deployment with no billing system; the portal never shows "£0" or "$0 owed" when it simply has no data.
- R-BILL-3. A proxy sees bills only with a `billing` scope; a bill never reveals an adolescent-confidential service to a proxy or to the account holder who is not the patient (US explanation-of-benefits leakage is a known issue; *[uncertain]*, not researched here).

## 5. Sharing

### How others do it

- **One-time share with a clinician.** Epic's Share Everywhere: the patient makes a code in MyChart; the clinician enters it with the patient's date of birth on a website; three wrong dates void the code. Sources disagree on validity (5 minutes or up to 60) *[uncertain]* [S43][S44].
- **Link sharing.** SMART Health Links let a patient share a link (or QR code) to FHIR data, optionally with a passcode (`P` flag) and an expiry [S45].
- **App access.** US payers must offer a Patient Access API with SMART on FHIR; certified US health IT must offer patient-facing APIs; most sources read are secondary *(snippet)* [S46]. SMART standalone launch with `launch/patient` lets an app ask the patient to pick a record [S16].
- **Summary and full export.** The International Patient Summary (IPS) 2.0.0 is a minimal, specialty-agnostic summary for unscheduled cross-border care [S47]. A full single-patient export is IO-8.

### Standards

SMART App Launch 2.2 [S16]; IPS 2.0.0 [S47]; SMART Health Links [S45]; FHIR R4 `Consent` [S48]; IO-2, IO-5, IO-8.

### Patient workflow

1. The patient chooses "Share my record".
2. The patient chooses what (summary as IPS, results only, everything), with whom (a person with a code, a link, an app) and until when (required end).
3. The page shows exactly what will be shared, including the warning that once shared, a copy cannot be taken back.
4. The patient confirms; the share is recorded as a `Consent` with the scope and end, and in the access log.
5. Each use of the share by the recipient is an entry in the access log ("Someone with your share code viewed your summary").
6. The patient can end the share at any time; ending stops future reads, not copies already taken.

### Rules

- R-SH-1. Every share has an end date and a scope; none is open-ended.
- R-SH-2. A share is checked against the patient's consent and labels at the time of each read, not only when made.
- R-SH-3. Every read through a share is in the patient's access log with the share it used.
- R-SH-4. A share code or link is single-person: after a number of failed checks (for example three) it is void.
- R-SH-5. A proxy may share only with a `share` scope, and never content labelled adolescent-confidential.
- R-SH-6. Download for the patient (IO-8) and share to others are different actions with different confirmations.

## Proxy and adolescent confidentiality (all five)

- US systems commonly cut a parent's proxy view at about age 12 to a limited set (appointments, immunisations, allergies, billing, messaging) and give the teen an own account; thresholds vary by state and system *(snippet of health-system pages)* [S49].
- In England, guidance names age 11 as a review point for parental proxy access to general practice records, with a competence assessment by a skilled clinician; local medical committees differ on independent access under 16 *(snippet)* [S50][S51].
- Guardians often use adolescents' own accounts [S23], so neither model is safe by itself.

Rules:

- R-PRX-1. Each capability has its own proxy scope: `results`, `messages`, `appointments`, `billing`, `share`, added to the built four. No scope implies another.
- R-PRX-2. At the deployment's review age, every parental proxy's scopes are cut to the pack's reduced set and a review task is raised; the cut does not wait for the review.
- R-PRX-3. Items with an `adolescent-confidential` label are never shown, counted, messaged or shared to a proxy.
- R-PRX-4. A proxy view always carries "This view does not show everything in the record", whether or not anything is hidden, so its absence never reveals a confidential item.
- R-PRX-5. The patient (when old enough under the pack) sees each proxy's scopes and can end one; the clinician records competence decisions with a reason.

## Hazards to add when built

Described here, not added to the hazard log. Each goes to `governance/clinical-safety/hazard-log.csv` (AS-1) with the first code that makes it real. None is accepted or closed; only the Clinical Safety Officer does that.

- PP-H1. A patient reads a serious result before any clinician has, alone and at night, and acts on it wrongly or in distress.
- PP-H2. A released result reads as normal because it has no flag or no range, or because "not reviewed" is not shown.
- PP-H3. A result is held by a rule nobody owns and never released or reviewed.
- PP-H4. A message describing an emergency waits in an inbox.
- PP-H5. A message sits with an absent clinician and is never answered.
- PP-H6. A proxy (or a guardian using the teen's account) sees or learns of adolescent-confidential care.
- PP-H7. A record is shared with the wrong person, or for longer or wider than meant.
- PP-H8. An appointment shown is stale or a booking shows as made when the PAS refused it.
- PP-H9. A bill or statement reveals confidential care to an account holder.
- PP-H10. Digital exclusion: patients without access, skills or language miss results or replies that others get, and nobody notices.

## Equity

In 2020, 56% of US adults said they had been offered portal access and about 40% had used one in the past year; Black and Hispanic adults were 5.3 and 5.4 percentage points less likely to be offered one, and among those offered a portal, access differences by race were not significant; encouragement by a clinician was linked to 21 points more use [S52]. An earlier analysis of the same survey (2014, 3,677 adults) found 92% thought online access important, but only 34% had been offered it and 28% had used it, with Black and Hispanic respondents less often offered access [S53]. Studies linking neighbourhood broadband to lower portal use were found only as search snippets and are not cited. A portal must never be the only route: every capability above has a phone or in-person path, and a deployment should measure use by group (see [equity audit](../equity-audit.md)).

## First slice per capability

- **Results:** a release-rule table (pack data, with a checker) and a read-only results list on `/my-record` for the patient only, showing sign-off status in words; proxies still refused. Fake-clock test for `after-delay`.
- **Messaging:** patient-to-team messages as append-only `Communication` rows into a CA-6 team inbox, with the emergency text, the reply-time promise and an escalation on a fake clock; no attachments.
- **Booking:** a read-only list of appointments through the scheduling port with "as of" time; no self-booking.
- **Bills:** nothing in code; a configured link out, hidden when unset. Record the boundary decision.
- **Sharing:** IPS download for the patient (IO-5) and a list of active shares with "end"; no codes or links yet.

## What this repo has and lacks

| Need | Has | Lacks |
|---|---|---|
| Patient sign-in | Keycloak patient and proxy roles; `/my-record` | Step-up authentication for sharing |
| Access log | SE-6 with filters and CSV | Entries for shares, messages and result views (none exist) |
| Proxy | PA-5 terms; scopes `identity`, `allergies`, `photo`, `access-log` | Scopes for results, messages, appointments, billing, share; adolescent rule |
| Results | OR-4 versions and sign-off; policy refuses patients | Release rules, a patient view |
| Errors in record | RC-4 amendment requests (vitals, allergies) | Results as a subject of a request |
| Messaging | None | Store, inbox (CA-6), escalation |
| Booking | Port proposed (PA-1) | Port, PAS adapter, CA-4 |
| Bills | Out of scope | (Intentionally) everything |
| Sharing | Consent store (SE-2) | FHIR `Consent`, SMART (IO-2), IPS (IO-5), EHI export (IO-8), share links |
| Equity | Equity audit page | Use-by-group measures for the portal |

## Not verified

Not read in full: the Steitz article (abstract and press only), the Long et al. 2026 article (press only), the 2025 Health Affairs article (refused), the Mayo self-scheduling paper, the eCFR text of 45 CFR 171.201 (redirected; the federal FAQ was read), the CMS good-faith-estimate rule text (CMS decision tree and summaries only), England's prospective-access and proxy guidance (summaries), the RCGP children's guidance (secondary only), Epic's own documentation for result release and Share Everywhere (help pages and FAQ only), SMART Health Links passcode rules, and the private-patient charging rules in England. The OpenNotes 2012 author order is from secondary sources *[uncertain]*. No clinician, lawyer or patient has reviewed this page.

## Sources

- [S1] ONC information blocking FAQ summary (delay as interference): https://www.healthcareittoday.com/2021/03/23/new-information-blocking-faqs-from-onc/
- [S2] AMGA letter on immediate resulting, 2025: https://www.amga.org/getmedia/e77e0dbd-fe17-4882-a703-9c9a8c7de52c/amga-immediate-resulting_10-23-2025-letter-final.pdf
- [S3] ASTP/ONC FAQ, Preventing Harm exception and type of harm: https://healthit.gov/faq/do-preventing-harm-exception-requirements-type-harm-align-hipaa-rules/ ; rule text: https://www.ecfr.gov/current/title-45/subtitle-A/subchapter-D/part-171/subpart-B/section-171.201
- [S4] Steitz BD et al., Perspectives of patients about immediate access to test results, JAMA Netw Open 2023;6(3):e233572: https://doi.org/10.1001/jamanetworkopen.2023.3572
- [S5] Press release on Steitz et al.: https://bidmc.org/news-stories/all-news-stories/news/2023/03/survey-shows-patients-prefer-immediate-access-to-test-results
- [S6] England's national health body, millions more given access to GP records online (2023): https://www.england.nhs.uk/2023/12/millions-more-people-given-access-to-their-gp-records-online/
- [S7] England's national health body, clinical messaging and test results: https://www.england.nhs.uk/long-read/clinical-messaging-and-test-results/
- [S8] England's national health website, view your test results: https://www.nhs.uk/nhs-services/gps/view-your-test-results/
- [S9] Australian Government, modernising My Health Record (access to results): https://www.health.gov.au/our-work/modernising-my-health-record-improved-access-to-health-information
- [S10] Medical Republic, more test results sooner: https://www.medicalrepublic.com.au/more-test-results-available-sooner-in-my-health-record/123961
- [S11] AMA (Australia), share-by-default reforms: https://www.ama.com.au/ama-rounds/17-october-2025/articles/share-default-reforms-my-health-record
- [S12] Kaiser Permanente Washington, OpenNotes 2012 study news: https://kpwashingtonresearch.org/news-and-events/recent-news/news-2012/patients-feel-more-control-their-health-when-doctors-share-notes/
- [S13] Bell SK et al., Frequency and types of patient-reported errors in EHR ambulatory care notes, JAMA Netw Open 2020: https://pubmed.ncbi.nlm.nih.gov/32515797
- [S14] FHIR R4 DiagnosticReport: https://hl7.org/fhir/R4/diagnosticreport.html
- [S15] FHIR R4 Observation: https://hl7.org/fhir/R4/observation.html
- [S16] SMART App Launch 2.2, scopes and launch context: https://hl7.org/fhir/smart-app-launch/scopes-and-launch-context.html
- [S17] Long JJ et al., Trends in patient portal messages, office visits and telephone encounters, JAMA 2026 (press): https://www.ajmc.com/view/patient-portal-messages-outpace-office-visits
- [S18] Healthcare Dive on the same study: https://www.healthcaredive.com/news/patient-portal-messages-clinicians-skyrocketed-jama/823785/
- [S19] Holmgren AJ et al., Billing patient portal messages as e-visits and messaging volume, JAMA 2023;329(4):339-342: https://pmc.ncbi.nlm.nih.gov/articles/PMC10408262
- [S20] The uneven distribution of patient portal messages, Health Affairs 2025: https://www.healthaffairs.org/doi/10.1377/hlthaff.2025.01416
- [S21] North F et al., Patient-generated secure messages and eVisits: are patients at risk? JAMIA 2013: https://pmc.ncbi.nlm.nih.gov/articles/PMC3822104
- [S22] UPMC, patient portal (not for emergencies): https://www.upmc.com/healthbeat/family-health/patient-portal
- [S23] Ip W et al., Prevalence of adolescent portal account access by guardians, JAMA Netw Open 2021: https://pmc.ncbi.nlm.nih.gov/articles/PMC8446820
- [S24] FHIR R4 Communication: https://hl7.org/fhir/R4/communication.html
- [S25] FHIR R4 CommunicationRequest: https://hl7.org/fhir/R4/communicationrequest.html
- [S26] Online self-scheduling in an orthopaedic practice (no-show 4.5% vs 4.4%): https://pmc.ncbi.nlm.nih.gov/articles/PMC10199359/
- [S27] North and Nelson, self-scheduling of well-child visits, JMIR Med Inform 2021;9(3):e23450: https://medinform.jmir.org/2021/3/e23450
- [S28] FHIR chat, Appointment $find vs Slot search: https://chat-archive.fhir.org/stream/179166-implementers/topic/Appointment.20.24find.20vs.20Slot.2Esearch.html
- [S29] FHIR R4 Appointment: https://hl7.org/fhir/R4/appointment.html
- [S30] Argonaut Scheduling IG, Appointment $find: https://fhir.org/guides/argonaut/scheduling/OperationDefinition-appointment-find.html
- [S31] IHE ITI Scheduling, volume 1: https://profiles.ihe.net/ITI/Scheduling/volume-1.html
- [S32] Epic, patient financial experience: https://epic.com/software/patient-financial-experience
- [S33] Cleveland Clinic, MyChart billing: https://my.clevelandclinic.org/resources/mychart-billing
- [S34] CMS, No Surprises Act good-faith estimate decision tree: https://www.cms.gov/files/document/nsa-gfe-decision-tree.pdf
- [S35] CMS, No Surprises Act key responsibilities for providers: https://www.cms.gov/files/document/nsa-key-responsibilities-providers.pdf
- [S36] CMS, hospital price transparency FAQ (45 CFR 180.60): https://www.cms.gov/files/document/hospital-price-transparency-frequently-asked-questions.pdf
- [S37] UK Government, charging overseas visitors in England, guidance for providers: https://gov.uk/government/publications/nhs-cost-recovery-overseas-visitors/charging-overseas-visitors-in-england-guidance-for-providers-of-nhs-services
- [S38] Charges to Overseas Visitors Regulations 2015 (SI 2015/238): https://legislation.gov.uk/uksi/2015/238/2022-03-17/data.html
- [S39] UK Government, prescription charges frozen (2025): https://www.gov.uk/government/news/cost-of-living-boost-for-millions-as-prescription-charges-frozen
- [S40] Community Pharmacy England, patient charges: https://cpe.org.uk/dispensing-and-supply/prescription-processing/receiving-a-prescription/patient-charges/
- [S41] FHIR R4 Invoice: https://hl7.org/fhir/R4/invoice.html
- [S42] FHIR R4 ChargeItem, Account and PaymentNotice: https://hl7.org/fhir/R4/chargeitem.html , https://hl7.org/fhir/R4/account.html , https://hl7.org/fhir/R4/paymentnotice.html
- [S43] Epic Share Everywhere FAQ: https://shareeverywhere.epic.com/FAQ
- [S44] Cleveland Clinic, MyChart record sharing: https://my.clevelandclinic.org/resources/mychart-record-sharing
- [S45] SMART Health Cards and Links IG, links specification: https://hl7.org/fhir/uv/smart-health-cards-and-links/links-specification.html
- [S46] CMS, interoperability and patient access policies: https://www.cms.gov/about-cms/health-informatics-and-interoperability-group/policies-and-technology-interoperability-and-burden-reduction
- [S47] HL7 International Patient Summary IG: https://hl7.org/fhir/uv/ips/
- [S48] FHIR R4 Consent: https://hl7.org/fhir/R4/consent.html
- [S49] CentraCare, MyChart proxy access for teens: https://www.centracare.com/articles-stories/mychart-proxy-access-for-teens/
- [S50] England's national health body, proxy access: https://www.england.nhs.uk/long-read/proxy-access/
- [S51] Pulse, Wessex LMCs guidance on online services for children: https://pulsetoday.co.uk/resource/pulse-intelligence/wessex-lmcs-guidance-on-online-services-for-children
- [S52] National Cancer Institute, HINTS Brief 52, disparities in patient portal communication, access and use (July 2023): https://hints.cancer.gov/docs/Briefs/HINTS_Brief_52.pdf
- [S53] Peacock S et al., Patient portals and personal health information online, JAMIA 2017;24(e1):e173-e177: https://pmc.ncbi.nlm.nih.gov/articles/PMC7651932
