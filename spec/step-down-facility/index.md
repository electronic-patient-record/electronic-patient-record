# Step-down facility: discharge into it from this EPR

Public-source research read 2026-11-04 plus this repository's own files. A proposal and a reading, not clinical advice: nothing here is built, and no clinician has reviewed it.

Method: web searches (mostly search snippets) and the project's specs: [care settings](../care-settings.md), [domain model 5.2.1](../05-domain-model.md), [closed-loop referral](../interoperability/closed-loop-referral.md), [open questions](../16-open-questions.md), [scope](../02-scope.md). Related: [virtual ward](../virtual-ward/index.md) (holds the full 12-step discharge workflow; this file gives only the differences for a bed), [care homes](../care-homes/index.md) (if written yet), [data exchange](../data-exchange/index.md).

## What it is

A bed outside the acute ward where a patient who no longer needs acute care continues to recover, be assessed or be rehabilitated before going home or to longer-term care.

| Name (UK unless stated) | Typical meaning (as described in sources) |
|---|---|
| Bed-based intermediate care | Time-limited recovery and rehabilitation in a community unit, a care home or a community hospital bed |
| Community hospital | Local inpatient beds with GP, nurse or therapy cover; often a step-down destination |
| Discharge-to-assess (D2A) bed | A temporary bed in which needs are assessed after discharge, not before; "home first" is the default and the bed is the alternative |
| Reablement unit | Short-term support to regain independence, at home or in a unit |
| Step-down ward / care home bed | A bed with nursing for a person not yet fit for home |
| Elsewhere | Sub-acute or skilled nursing facility, inpatient rehabilitation, transitional care (US and others; names and rules differ and are not sourced here) |

NICE NG74 distinguishes four kinds of intermediate care: crisis response, home-based, bed-based and reablement. It suggests considering bed-based care for people who are acutely ill but stable and not fit for safe transfer home (1.2.4), and starting within 2 days of an appropriate referral because delay increases the risk of deterioration (1.4.6) (snippets of the guideline page). Quality standard QS173 statement 2 repeats the 2 days and asks commissioners to monitor waiting times.

## Models and evidence

- **Discharge to assess.** National guidance in England makes "home first" the default (pathways 0 and 1) with alternatives for those who cannot go home, and says that, apart from exceptional circumstances, people should not move straight into a permanent care home without a chance to recover in a temporary placement (search summary; the pathway-by-number definitions, including which is bed-based, were not read).
- **Intermediate care evidence.** A summary of NICE NG74 says average wait from referral to bed-based care was 3 days and about 76 percent started within 2 days (HFMA summary, snippet). The underlying trials were not read.
- **Flow and delay.** The NHS action cards for transfer of care hubs describe the discharge process as a hub function (snippet; not read in full). Delayed discharge figures were not gathered.
- **Limits.** Evidence for bed-based care mixes services of different design; claims of benefit here are not the author's findings. Other countries differ; the pattern (assess in a lower-acuity bed, with named medical cover) is an example not "the" rule ([equity rule](../02-scope.md)).

## Who is responsible

- **On arrival.** Clinical responsibility passes at a stated point (acceptance and arrival, not the sending of a referral). Who is the responsible doctor varies: the facility's consultant, a visiting GP or a community-hospital GP, or a geriatrician. How this is stated in each service was not found; it is an open question.
- **Medical cover and nursing cover** are separate facts to record: who can be called, in what hours, how fast, and what the facility cannot do (for example give intravenous therapy, or monitor overnight).
- **Return to acute.** The facility needs return criteria and a phone route; the acute service says whether it will take the patient back and by what route (emergency department or direct to ward).
- **Pathway view.** The pathway stays with the pathway owner named at discharge; the facility owns the next step. The built care-relationship check grants access, not responsibility; see [virtual ward](../virtual-ward/index.md) "Who is responsible".

## The discharge workflow

Follow the 12 steps in the [virtual ward page](../virtual-ward/index.md) (idea 1 there). Differences for a bed:

1. **Eligibility** is the facility's acceptance criteria (stable, rehabilitation potential, within the cover available, no need for acute-only treatment). The service directory shows hours and exclusions.
2. **Consent and choice.** Record the patient's choice among offered places (a patient may be offered an interim bed while a preferred place is awaited; the law on choice, capacity and funding differs by country and is not summarised here).
3. **Capacity and best interests.** If the patient lacks capacity, record the decision-maker and basis (a pack per jurisdiction).
4. **Referral is the same loop** with `requested` to `accepted`; add **bed offered**, **transport booked** and **arrived** as steps on the pathway. The referral state table has no "arrived"; that is a gap.
5. **Transfer-of-care handover** (see below) goes with the patient and ahead of them, and the receiving nurse confirms receipt.
6. **Set expectations**: expected length of stay, goals, review date, who decides the next destination. A bed with no end date is a delayed discharge in waiting.
7. **Monitoring and escalation** use the facility's own observation routine; our record shows what is sent in and what is read back (for example the discharge summary from the facility).
8. **Close the acute stay** when the facility confirms arrival; keep the pathway open, now with the facility's step as the open one.

## What must travel

The England transfer-of-care standards from the Professional Record Standards Body (PRSB) set the shape for a discharge summary; the content below is the author's list to check against it.

| Item | Why | Source in our record today |
|---|---|---|
| Diagnoses, procedures, reason for the stay | Receiving team knows what was treated | None (no notes or problem list) |
| Medicines on discharge, changes, stopped, and why | Avoid duplication or omission | None (RC-5 not built) |
| Allergies and adverse reactions | Safety | Built |
| Risks: falls, pressure, delirium, infection, safeguarding | Care plan on arrival | None |
| Resuscitation and treatment escalation (ReSPECT or equivalent) | Decision at the bedside | None |
| Mental capacity and legal status | Lawful care and restrictions | None (XP-7 is only the proposal) |
| Goals, expected length of stay, rehabilitation plan | Purpose of the bed | None (care plan not built) |
| Function, mobility, diet, swallowing, continence | Daily care | None |
| Latest observations and results pending | Baseline, follow-up owner | Observations built; results none |
| Who to call, in hours and out of hours, and return criteria | Escalate | None |
| Next of kin, carer and contacts | Family involvement | None |

## Compared with this project

| Need | This project today | Gap |
|---|---|---|
| Referral with accept/decline | State table, worklist and timeouts built | No bed, no capacity, no arrival or transport step |
| Pathway stays open | Designed only | No build; OQ-CP1 owner question open |
| Discharge summary out | None | No outbound message, no document builder, no PRSB mapping |
| Receive a facility's updates | Referral `interim-report` state exists on paper | No inbound clinical document |
| Responsibility on arrival | Access by care relationship | No responsible-clinician or location record |
| Setting profile | Designed in [care settings](../care-settings.md) | Not built; a community bed is neither acute ward nor home |
| Phone view for a visiting doctor | Requirements only | See [mobile viewability](../mobile-viewability.md) |

## Candidate hazards

For the Clinical Safety Officer; none accepted. For the hazard log (AS-1).

- S-1 Patient sent to a bed that never accepted, or accepted a different patient.
- S-2 Transfer of clinical responsibility unclear, so nobody reviews the patient for days.
- S-3 Medication list missing or stale on arrival; discharge list differs from the facility's.
- S-4 Resuscitation status not carried, or an old form carried.
- S-5 Facility cannot meet the patient's needs (cover mismatch) and is not told.
- S-6 No return-to-acute route; deterioration waits for an ambulance without context.
- S-7 Results pending at discharge have no owner (H-044 family).
- S-8 Patient lacks capacity and the move is made without a recorded decision.
- S-9 Length of stay and goals missing, so the bed becomes a permanent placement by default.
- S-10 The acute pathway is closed on transfer; the facility does not own the next step.

## Ideas

1. **Shared discharge-workflow template** with the virtual ward (see the [virtual ward](../virtual-ward/index.md) ideas); the destination type changes criteria and handover content (links CA-1, CA-3, PA-14).
2. **Handover as a structured document built from the record**, preview shown to the clinician, with every missing item shown as "missing" not blank; map to the PRSB eDischarge shape later (RC-5 for medicines).
3. **Add `arrived` and `transport-booked`** as pathway steps next to the referral loop, and show "accepted but not arrived" on the referrer's worklist.
4. **Record cover** on the destination: medical and nursing hours, exclusions, return route, so the picker warns about a mismatch.
5. **Expected length and review date** are mandatory fields with a worklist of overdue reviews.
6. **Return-to-acute referral** is the same loop in the other direction with the facility record attached.
7. **Facility updates** read in through the `interim-report` state, shown on the pathway.
8. **A setting profile for community beds**, so the user sees the bed-and-caseload view they need, not an acute ward board.

## Open questions

- Who is the responsible clinician at a D2A bed or a care home bed, and how is it recorded when it is a GP who visits?
- Is the facility a user of our EPR, a recipient of messages, or both? (Data exchange.)
- Which register of services and capacity do we read, and who keeps it current?
- How does a worldwide-equitable design describe "step-down" where no such facility exists?
- Does a care home bed need the care-home spec's resident record first?
- What is the legal frame for moving a patient lacking capacity (a pack per jurisdiction)?

## Not verified

All NICE, NHS and HFMA statements are from search snippets; the full NG74 text, the gov.uk D2A annexes, the transfer-of-care hub cards and the PRSB standards were not read. The ReSPECT link to transfer standards was not found. The mapping of pathway numbers to bed-based care was not confirmed. No clinician has reviewed the table of what must travel.
- Link check 2026-11-04: these cited pages returned 404. They may have moved, but what rests on them is unconfirmed: https://standards.nhs.uk/published-standards/edischarge-summary; https://hfma.org.uk/articles/nice-update-intermediate-care-can-improve-independence-and-cut-costs

## Sources

- NICE NG74 intermediate care including reablement: https://www.nice.org.uk/guidance/NG74/chapter/recommendations
- NICE QS173 statement 2: https://www.nice.org.uk/guidance/qs173/chapter/quality-statement-2-starting-bed-based-intermediate-care
- HFMA summary of NG74: https://hfma.org.uk/articles/nice-update-intermediate-care-can-improve-independence-and-cut-costs
- NHS England, Home First principle: https://www.england.nhs.uk/urgent-emergency-care/improving-hospital-discharge/reducing-long-term-stays/home-first/
- Gov.uk, hospital discharge and community support guidance: https://www.gov.uk/government/publications/hospital-discharge-and-community-support-guidance/hospital-discharge-and-community-support-guidance
- Transfer of Care Hub action cards: https://assets.publishing.service.gov.uk/media/630e26a78fa8f5536727f08d/Transfer_of_Care_Hub_-_Hospital_Discharge_Action_Cards_Revised.pdf
- PRSB transfer of care toolkit: https://theprsb.org/toolkits/transfersofcare/about-this-toolkit/
- NHS England standards, eDischarge summary: https://standards.nhs.uk/published-standards/edischarge-summary
- PRSB maintaining standards: https://theprsb.org/projects/maintainingourstandards/
