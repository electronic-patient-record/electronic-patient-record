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

## What custody changes (my reasoning, to be checked with the customer)

1. **The person is held, so the setting is a profile**: the rule of [care settings](../care-settings.md) rule 1 applies. One patient, one record; a custodial profile chooses the screens, forms and rules. A person who enters custody is not a new patient: their existing record and pathways continue.
2. **Who may know is different, and more dangerous.** (The customer decided that non-health staff are told functional information only: OQ-CH3.) A person in custody cannot choose to avoid the people around them. Information about health, such as a diagnosis, a drug, an infection or a mental-health status, can be used against them by other prisoners or by staff. The default is **health information stays with the health team**; security and operational staff get only what a rule says they need (for example, that a person needs a wheelchair-accessible cell), never the diagnosis. This is the label-masking and protected-information design applied hard (FR-4.3); it is the same disclosure risk as H-065.
3. **Custody may limit consent and choice.** A person may not be able to choose their clinician, refuse a transfer, attend an appointment or keep a medicine. The system records the facts, such as an appointment missed because the person was not escorted, and never records a refusal the person did not make. A missed visit has a reason field that can say "not escorted".
4. **Movement.** People move between establishments, to court, to hospital and into release, often at short notice. A transfer, a hospital visit and a release are events the pathway must follow. The record travels with the person, and what the receiving health team needs (medicines, risks, open pathways) is a summary that is **complete or says it is not**, never a silent gap. Nothing about a transfer destination is shown outside the people who need it.
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
| OQ-CH4 | Does the health record stay with the person's own record when they leave custody, or is it a separate record that is linked? |
| OQ-CH5 | Which records are requested by courts, police or investigators, under what law, and who may release them? |
| OQ-CH6 | Is there a national summary the receiving community team already reads, which the release hand-over should write to? |

## Tasks and acceptance

`CF-2` is this page. A later build needs, before any code: answers to the country in OQ-CH1, the controller and processor in OQ-CH2, who decides a function in OQ-CH3, and OQ-CH4 to OQ-CH6; a hazard-log row for the disclosure of health information to non-health staff and for a release with no hand-over; a test that a custody role reads no clinical data beyond the listed functional needs; a review by a prison health clinician and by a person who has been in custody.

## Not done, not verified

No outside source was read. No law was checked (this is a country matter: a pack, not core). No customer, clinician, prison or person who has been in custody was asked. Nothing is built.
