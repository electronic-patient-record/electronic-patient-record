# Prison health: a custodial care setting (CF-2)

A draft from the customer's request of 2026-11-24 ("add capabilities for prison health") and this repository's own files. A proposal: nothing is built, no outside source was read, no prison health clinician, prison officer, lawyer or person who has been in custody has reviewed it, and it is NOT FOR CLINICAL USE. "My reasoning" marks the author's.

## Related links

[Care settings](../care-settings.md) (a setting is a profile, not a fork; section 4), [Protected information](../access-control/protected-information.md), [Roles and assignments](../access-control/roles-and-assignments.md), [Bereavement](../bereavement/index.md) (a death in custody), [Support groups](../support-groups/index.md) (CF-1), [Open questions](../16-open-questions.md) (OQ-CH1 to OQ-CH6), [Scope and the equity rule](../02-scope.md).

## What was asked, and what is not known

A customer wants the system to support prison health. The request did not say which country or statute, or who runs the service. The customer later said the scope is all custodial settings (OQ-CH1, below); the country is still unknown.

## The customer's answer to OQ-CH1 (2026-11-24): all settings

The scope is every custodial setting: prison, police custody, immigration detention, secure youth settings, and a hospital that treats people in custody. The country is not yet given. My reasoning on what follows:

- **One profile with a setting type, not five profiles.** The nine points below apply to all, but the legal frame, the length of stay and the people held differ: police custody is hours, a prison is months or years, immigration detention has no fixed end, a secure youth setting holds children (consent, safeguarding and parental rights differ), and a hospital treating a person in custody is a visit with an escort. The profile carries a **setting type**, and rules that differ (consent, who may be told, how long a record is kept, who is the controller) attach to the type and the country pack.
- **Short stays make the hand-over rule heavier.** A person held for hours may leave with no hand-over at all; the record then needs to say what was started and not finished.
- **Children in secure youth settings** are a safeguarding case before they are a custody case; the child's record rules (consent, who may be told) come first, and no point on this page weakens them.
- **A hospital treating a person in custody** is the case where the custodian is in the room: what the escort may hear, see or be told is point 2 and OQ-CH3 in its sharpest form.
- **Immigration detention** is a place where a health record can be requested for an immigration decision. Whether it may be released is OQ-CH5 in its sharpest form, and the default is that it is not.

## The customer's answers to OQ-CH2 and OQ-CH3 (2026-11-24)

**OQ-CH2: various; government, police or private.** The service may be run by a government body, by the police, or by a private company. **OQ-CH3: functional only**: security and operational staff are told only what a person can or cannot do, never a diagnosis. My reasoning on what follows:

- **A police-run or prison-run service holds a health record under the same roof as the people who may use it against the person.** The health record is therefore **separated from the custodian's own systems**: a custodian's staff have no role that reads clinical data, and the custodian's own systems are never a data source for it. This is the most important rule of the profile (point 2). Where the operator is the police, it also follows that health staff are not police staff for the purpose of the record, and the record says so.
- **The operator decides the controller, and the country pack names it.** A government, a police force and a private contractor each have a different legal basis, a different oversight body and a different duty to the patient. The core holds an **operator type** (government, police, private) beside the setting type, and the core's rules (separation, audit, release of records) do not change with it; only the pack's law does.
- **A change of operator is a change of controller.** A private contract that ends or is moved leaves a record that must go to the next health team intact and audited: the record never belongs to the operator in the sense of being deleted or left behind.
- **Functional only means a list, not free text.** A function is a short fixed statement of what a person needs or cannot do ("needs a ground-floor cell", "cannot be handcuffed behind the back", "needs to carry an inhaler", "should not be moved without a nurse"). Each item is chosen from a reviewed list, has a date and an expiry, and carries no reason. A clinician approves each item (my proposal; OQ-CH3 still asks who decides). Those words are the **only** health content that leaves the health team, and a function that would hint at a diagnosis ("needs a dialysis machine") is reviewed before it goes on the list, because the function itself can disclose.
- **Every function shown to a non-health person is audited**, with who saw it and when, in the same way as any disclosure.

## The customer's answer to OQ-CH5 (2026-11-24): it varies

Whether and how a record may be released to a court, the police or an investigator varies by jurisdiction, country, U.S. state, purpose and urgency, for example an imminent threat to wellbeing. My reasoning on what follows:

- **The core holds no release rule and no default.** The rule is a **release table in the country (or state) pack**, keyed by jurisdiction, requester type, purpose and urgency. With no pack, the core **refuses** and says why; it never guesses a jurisdiction's law, and a record is never released because no rule forbade it.
- **Every release has the same shape in the core, whatever the law:** who asked, under what basis (a named rule of the pack, or a named person's decision), what was released (the smallest part that meets the purpose), who released it, when, and that it was audited. A release with no basis recorded does not happen.
- **Urgency is a path with a review, not a bypass.** An imminent threat to wellbeing may allow a release that the ordinary rule would not, as break-glass does for a read ([H-024](../../governance/clinical-safety/hazard-log.csv)): it is allowed where the pack says so, limited to what the threat needs, and **reviewed afterwards by a named person**, with the review overdue shown. "Urgent" is the requester's claim until the releaser accepts it, and a claim that is not accepted is recorded too.
- **Purpose limits what is released.** A court order for one matter does not release the whole record; a safeguarding request does not release a mental-health note unless the pack says so. The core offers to release by section or label, not the whole, and says what it left out.
- **A different state is a different pack, and a person who moves is under the rule where the record is held, not where they are.** Which of the two applies is a legal question for the pack (OQ-CH5 stays open for it); the core records both places.
- **Immigration detention and police custody** are the settings where a request is likeliest to come from the party that holds the person (OQ-CH1 and the point above). The pack may forbid release to the custodian outright, and the core allows the pack to say so.

## The customer's answer to OQ-CH4 (2026-11-24): one record

The custody health team writes into the person's own record, not a separate linked one. (I had leaned towards a separate record; the customer chose otherwise, and this is the design.) My reasoning on what follows, because one record puts custody next to everything else:

- **Continuity is the gain**: a medicine, a result or a pathway started in custody is already in the record the community team reads on release, with no hand-over to lose. The release hand-over of point 5 becomes a **review of the open items on the one record**, which is complete by construction, and says so only for what is in it.
- **The cost is that custody becomes visible to every reader of the record.** A person's employer-facing, housing-facing or family-facing readers (a GP receptionist, a new clinic) could see that the person was in custody. So the **fact of custody and everything written in custody carry a label** by default, and the masking of [protected information](../access-control/protected-information.md) (FR-4.3) hides them from a reader who has no need: a reader sees a gap marked "restricted", not the custody setting, the establishment or the dates. This is a precondition of the profile, and the service cannot yet do it (no record carries a label yet, SE-4).
- **The custodian's staff still read no clinical data and write none.** Writing into the one record is for health staff; an operator-employed non-health staff member (a prison officer, a police officer) has no role that reads or writes the record, and the functional list of OQ-CH3 is the only thing that leaves it.
- **After release, what is shown about the custody period is the patient's and the clinician's choice with a default of restricted.** The custody label is lifted only by a rule of the pack or by the person's own decision, and a lift is audited. Which default the country wants (restricted, or open to treating clinicians) is a pack question.
- **An erroneous custody label, or a missing one, is a hazard in both directions**: a missing label shows custody to a reader who should not see it, and a label that hides a result a clinician needs (a pending result, a medicine) is a safety risk. A clinician who needs it opens it by break-glass, as in [H-024](../../governance/clinical-safety/hazard-log.csv), with the reason audited.
- **The record is append-only as everywhere**: a custody entry cannot be removed after release, only labelled and, where the law allows, amended by a visible correction.

## What custody changes (my reasoning, to be checked with the customer)

1. **The person is held, so the setting is a profile**: the rule of [care settings](../care-settings.md) rule 1 applies. One patient, one record; a custodial profile chooses the screens, forms and rules. A person who enters custody is not a new patient: their existing record and pathways continue.
2. **Who may know is different, and more dangerous.** (The customer decided that non-health staff are told functional information only: OQ-CH3.) A person in custody cannot choose to avoid the people around them. Information about health, such as a diagnosis, a drug, an infection or a mental-health status, can be used against them by other prisoners or by staff. The default is **health information stays with the health team**; security and operational staff get only what a rule says they need (for example, that a person needs a wheelchair-accessible cell), never the diagnosis. This is the label-masking and protected-information design applied hard (FR-4.3); it is the same disclosure risk as H-065.
3. **Custody may limit consent and choice.** A person may not be able to choose their clinician, refuse a transfer, attend an appointment or keep a medicine. The system records the facts, such as an appointment missed because the person was not escorted, and never records a refusal the person did not make. A missed visit has a reason field that can say "not escorted".
4. **Movement.** People move between establishments, to court, to hospital and into release, often at short notice. A transfer, a hospital visit and a release are events the pathway must follow. With one record (OQ-CH4) the record does not travel: the receiving health team reads the same record, and what it needs (medicines, risks, open pathways) is a summary that is **complete or says it is not**, never a silent gap. Nothing about a transfer destination is shown outside the people who need it.
5. **Release is a hand-over, not an end.** Continuity of care in the community (a medicine supply, a referral, an appointment, a named contact) is the highest-value moment and a known gap in many systems (my reasoning; no source read). A release with an open pathway and no hand-over is a hazard.
6. **Medicines are held in a different way**: in-possession, supervised or withheld (tradeable or dangerous drugs), by rule of the establishment. The medicine administration module needs a profile where administration is witnessed and a missed dose has a reason ([medication administration](../medication-administration/index.md)).
7. **A death in custody** is an investigated event. The [bereavement](../bereavement/index.md) page covers the record of a death; an independent investigation may need the record, a legal step that belongs to the country pack.
8. **Records requested by security staff, police or courts are a disclosure decision**, never a default and always audited. Which requests are lawful is the country's law (a pack, [equity rule](../02-scope.md)).
9. **Sensitive categories are likelier here**: substance use, blood-borne infections, self-harm and suicide risk, and mental health. They are the categories that [protected information](../access-control/protected-information.md) lists, and the masking has to work before this profile can be used.

## What a custodial profile could hold (not built)

| Piece | Holds | Does not hold |
|---|---|---|
| Custody status | That the person is in a custodial setting, the establishment, and the dates | The offence, the sentence or the court's reasoning |
| Movement events | Transfer, hospital visit, court, release: date, from, to | A destination shown to staff who do not need it |
| Health information for non-health staff | Only the functional needs a rule lists (for example a mobility need) | A diagnosis, a drug, a test, a note |
| Missed-care reasons | "Not escorted", "moved", "court" and "refused", each as a distinct reason | A refusal recorded when the person was not asked |
| Release hand-over | A summary: medicines, risks, open pathways, who to contact, with its gaps named | A hand-over shown as complete when it is not |

## Not decided: needs the customer

| Id | Question |
|---|---|
| OQ-CH1 | **Partly answered 2026-11-24 by the customer: all of the settings** (prison, police custody, immigration detention, secure youth settings, and a hospital that treats people in custody). **Still open:** which country and statute. |
| OQ-CH2 | **Partly answered 2026-11-24 by the customer: various; government, police or private.** **Still open:** who is the controller and who the processor, for each. |
| OQ-CH3 | **Partly answered 2026-11-24 by the customer: functional information only.** **Still open:** who decides each functional item (a clinician is my proposal). |
| OQ-CH4 | **Answered 2026-11-24 by the customer: one record.** The custody health team writes into the person's own record. |
| OQ-CH5 | **Partly answered 2026-11-24 by the customer: it varies by jurisdiction, country, U.S. state, purpose and urgency (for example an imminent threat to wellbeing).** **Still open:** which release rules each pack holds, and who may release. |
| OQ-CH6 | Is there a national summary the receiving community team already reads, which the release hand-over should write to? |

## Tasks and acceptance

`CF-2` is this page. A later build needs, before any code: answers to the country in OQ-CH1, the controller and processor in OQ-CH2, who decides a function in OQ-CH3, the release rules of each pack in OQ-CH5, and OQ-CH6; a hazard-log row for the disclosure of health information to non-health staff and for a release with no hand-over; a test that a custody role reads no clinical data beyond the listed functional needs; a review by a prison health clinician and by a person who has been in custody.

## Not done, not verified

No outside source was read. No law was checked (this is a country matter: a pack, not core). No customer, clinician, prison or person who has been in custody was asked. Nothing is built.
