# Bereavement: what an EPR must do when a patient dies

Public-source research read 2026-11-04 plus this repository's own files. A proposal, not clinical or legal advice: nothing here is built, no clinician or person with lived experience has reviewed it, and it is NOT FOR CLINICAL USE.

## Related links

[Survivor grief](../survivor-grief/index.md) (the companion page: care of the people left behind), [Domain model 5.2.1](../05-domain-model.md) (pathway first), [Pathway design](../pathway-design.md), [Protected information](../access-control/protected-information.md), [Roles and assignments](../access-control/roles-and-assignments.md), [Access control](../access-control/index.md), [Maternity](../maternity/index.md), [Care homes](../care-homes/index.md), [Virtual wards](../virtual-ward/index.md), [Manual steps](../manual-steps/index.md), [Patient identity](../patient-identity/index.md), [Closed-loop referral](../interoperability/closed-loop-referral.md), [Data exchange](../data-exchange/index.md), [Retention schedule](../../governance/retention-schedule.md), [Service spec](../../electronic-patient-record-service-with-rust/spec/index.md), [hazard log](../../governance/clinical-safety/hazard-log.csv).

England and Wales are the worked example because the project is UK-first. Scotland, the United States and Australia are shown as examples, never as "the rule" ([equity rule](../02-scope.md)). Law belongs in a country pack, not in the core. Tags such as [AHRA3] point to Sources. "My reasoning" means there is no source behind a statement.

## Why this page exists

**A record that does not know a person has died keeps treating them as alive.** It sends the reminder, offers the appointment, marks the missed visit, and counts a birthday. For a family, a letter addressed to the person they have just lost can be the worst thing that arrives that week. For a living person wrongly recorded as dead, the same machinery stops their care and benefits.

The code today has **no way to record a death at all** (see [Compared with this project](#compared-with-this-project)). Everything below is therefore design, and it is easier to do now than after letters, portals and pathways exist.

Three ideas run through the page. They are my reasoning from the sources:

1. **A death is an assertion with evidence, not a true-or-false field.** It has a source, a status and a date that may be approximate, and it may be wrong.
2. **Every effect of a death must be reversible and traceable to that assertion.** If the assertion is withdrawn, what it stopped must be found and restored.
3. **The record does not end at death.** It stays, it is still confidential, and new people need to read it for new reasons.

## Before the death, briefly

End-of-life records are a lead-in, not the topic of this page.

- ReSPECT records emergency care and treatment recommendations ([care homes](../care-homes/index.md)). Whether this project owns it is OQ-E1 in [open questions](../16-open-questions.md).
- Electronic Palliative Care Coordination Systems (EPaCCS) capture preferences such as a preferred place of death. One review cites only 18 to 26 percent of people having such a record at death in two UK cities [EPACCS].
- A Leeds study found a recorded preference for hospital was linked to a five-fold increase in the likelihood of dying in hospital [RESPECTLEEDS]. That is an association, not a cause.
- NICE NG142 (end of life care for adults: service delivery, 2019) and NG31 (the last days of life) are the English guidelines. I could not open the recommendations [NG142].

**What this project would need.** Preferred and actual place of death as two separate facts, so that the gap can be seen. A way to record, while the person is alive, what they want after death (see "Confidentiality and access after death").

## Recording a death

### What sources distinguish

- **Verification is not certification.** Verification establishes that the person has died. Certification gives the cause. In the UK an experienced, assessed-as-competent registered nurse may verify an expected adult death under Hospice UK guidance [HOSPICEVOD]. So the person who confirmed the death is often not a doctor, and often not the person who certifies.
- **The US and FHIR.** FHIR `Patient.deceased[x]` is 0..1, a boolean or a dateTime, mapped to HL7 v2 PID-30 and PID-29 [FHIRDEC]. Only one form can be present. A boolean alone is the case of "we know they died, we do not know when". USCDI v3 lists date of death, and an HL7 issue asks for `deceasedDateTime` to be Must Support in US Core [USCORE]. IHE publishes a Vital Records Death Reporting guide for sending a death to vital records [VRDR].
- **England's two levels.** The Personal Demographics Service (PDS) holds a death notification status: **informal** (1), a notice from a local NHS organisation such as a GP or trust; **formal** (2), from the Registrar of Deaths; and **U**, removed, because a record can be updated in error [PDSCODE] [NEMS].
- **The duty on speed.** A Data Provision Notice requires GP practices and acute trusts to report deaths to PDS "within one working day of the death occurring", from 10 January 2022 [DPN].

### What this project would need

The rules are now specified on [death assertions](death-assertion.md). A proposed `DeathAssertion`, append-only like the other version tables:

| Field | Why | If absent |
|---|---|---|
| `occurred_at`, with precision (`YYYY`, `YYYY-MM`, a date, an instant) and the site zone | People are found after days; a death near midnight has a date that depends on the zone | `null` with a reason ("date not known") |
| `found_at` | Not the same as when the person died | `null` with a reason |
| `place` (a type, then free text) | Hospital, home, care home, hospice, public place, abroad; preferred against actual | `null` with a reason |
| `source` | Registrar, national demographics, hospital, GP, care home, coroner, family member, other system | Required |
| `status` | Reported, verified, certified, registered, withdrawn in error | Required |
| `verified_by`, `verified_at` | The person who confirmed it (a nurse, a paramedic, a doctor) | `null` with a reason |
| `reported_by` | Who told us; see [manual steps](../manual-steps/index.md) on source against typist | Required when the status is reported |
| `coroner_referred` | The cause is not settled, so nothing should imply it is | `null` with a reason |
| `reason`, `author`, `recorded_at` | Every version in this project has these | Required |

**Reading it.** Absent evidence is never agreement. No death assertion means "not known to have died", not "alive". A screen should never show the word "alive". The cause of death is a separate, later fact with its own status, and it may be months away.

## Certification, medical examiners and the coroner

England and Wales, from 9 September 2024:

- All deaths must be independently scrutinised, by a coroner or by a medical examiner [NOTIF].
- A registered medical practitioner who attended the deceased in their lifetime may complete the medical certificate of cause of death (MCCD) and propose a cause. The old requirement to have seen the patient within 28 days has gone [NOTIF] [GOVOVERVIEW].
- The medical examiner's scrutiny includes a proportionate review of records, an interaction with the attending practitioner, and offering the deceased person's representatives the chance to ask questions and raise concerns [NOTIF].
- Registrars no longer refer deaths to the coroner: the attending practitioner or medical examiner decides. When the cause is notified to the registrar, the 5-day statutory period to register starts [GOVOVERVIEW].
- **The MCCD now records the existence of implantable medical devices** [NOTIF]. A record that knows about implants helps here. This project has no implant list.
- The Access to Health Records Act 1990 was amended on the same date, so that a medical examiner may apply for access to the record [AHRA3].
- The non-statutory scheme began in 2019 [SI493EM]. NHS England reports over half a million deaths registered under the reforms, and a 10 percent fall in deaths reported to coroners in 2024 against 2023 [MEREPORT]. The fall covers a year in which the reforms ran for only the final third.

**Coroner.** The guidance says a death should always be notified where there is reasonable cause to suspect certain causes, among them violence, neglect, self-harm, a medical treatment or procedure, or an employment-related disease; and where the cause or the person's identity is unknown, the death may have been in state detention, or no attending practitioner is available [NOTIF]. The passage of time since the death does not remove the duty [NOTIF]. Family or police may report too, and the doctor should still notify [NOTIF]. The full list is in the guidance and is not reproduced here.

**Elsewhere, as examples.**
- Scotland has no medical examiner: a random sample of MCCDs goes to the Death Certification Review Service, selected by National Records of Scotland when the MCCD is entered [SCOTDCRS]. Sources conflict on the sampling rate (8 and 12 percent are both quoted). Deaths are registered within eight days [NRS].
- United States: death registration is by state and I did not research it.

**What this project would need.**
- A **medical examiner role** and a **purpose of use** for death review. The purposes today are treatment, emergency, operations, payment, research, patient access and audit; none fits. The role list has none either ([roles](../access-control/roles-and-assignments.md)).
- Time-bound read access for the examiner's office, including weekends (see "Faith and culture"), audited.
- A summary view for the attending practitioner: the problems, last contacts, implants and any procedure, so that certifying is faster and less likely to be wrong. This helps a doctor who attended the person months ago, which the rule now permits.
- The MCCD is not this project's document. It should reference one, not reproduce it. A digital MCCD was planned [GOVOVERVIEW]; I do not know its current state.

## Stopping contact with a person who has died

**The evidence is thin and anecdotal.** I found no study measuring how often this happens or how much distress it causes.
- A flu reminder reached a widow addressed to her husband who had died in 2014. The provider said its records showed him as deceased and that several letters had gone out wrongly [NOTTS].
- Questionnaires went to more than 50 dead patients in Scotland; an advocate called them an insult to grieving relatives [SCOTSMAN].
- Sands' National Bereavement Care Pathway (NBCP) says primary care should be told promptly so that a woman "is not sent reminders for antenatal appointments and immunisation clinics" [NBCPSB].
- The child death guidance says providers should notify the Child Health Information System "to ensure that further clinic appointments are not arranged" [CDR].
- Ireland's stillbirth and first-trimester pathways list cancelling antenatal clinics, scans and classes [HSEPATH].

The fact that three national bodies write this into pathways shows that it is a recognised failure.

**The UK government's Tell Us Once service does not solve it.** It notifies government and councils, and one council says some organisations such as the NHS may not be covered [TELLUS]. I did not confirm this.

**How long it takes.** Three different clocks run (the first figure is a duty, the others are ONS statistics):

| Clock | Time | Source |
|---|---|---|
| Local organisation tells PDS (informal) | One working day, a duty | [DPN] |
| Registration, all deaths, England and Wales | Median 7 days in 2022 | [ONSREG] |
| Registration, coroner-certified | Median 26 days in 2022 | [ONSREG] |
| Registration, coroner with inquest | Median 149 days in 2020 | [ONSREG] |
| Registration, deaths by suicide | About 5 to 6 months on average | [ONSSUI] |

A registered death sets off the PDS update and a deduction from the GP list [PCSEDED]. Spine publishes a death notification event when the status changes, through the National Events Management Service, and a subscriber should treat the latest message by `meta.lastUpdated` as the truth [NEMS]. A supplier suggests rechecking PDS before sending an appointment confirmation [SIXB]; that is vendor guidance, not an NHS rule.

So a system that waits for the formal status can keep writing for months in exactly the hardest cases. A death abroad, or one known only to a family, may never reach a local organisation in a day (my reasoning).

**What this project would need.**
- A **send-time check**. Every outbound route (letter, text, email, portal notice, recall, survey, research invitation, bill) asks at the moment of sending, not at scheduling.
- **Any active assertion stops contact**, including a reported one. The cost of holding a letter to a living person for a day is smaller than the reverse (my reasoning; a clinician should test this).
- **A no-show cannot be recorded against a person who has died.** Today a referral can reach `no-show`, and a missed-appointment letter is the natural next build.
- **A list of what was stopped**, with who, when and why, so that it can be restored (see "A death recorded in error").
- **Screening and recall** belong to national programmes. I found no source on how they stop. A local copy of an invitation list is a hazard.
- Contact with the family about the death itself is deliberate and worded by a person. No template addresses the person who died.

## Pathways, referrals and results at death

Under [5.2.1](../05-domain-model.md) rule 3 a pathway ends only by a recorded outcome: completed, stopped with a reason, or handed over to a named owner. A pathway with no owner is a safety condition. Death tests this rule.

**Proposed reading (mine, for clinicians to change).**
- Death does not silently close anything. Each open pathway gets the recorded outcome "patient died" with the assertion that caused it. If the assertion is withdrawn, the pathway reopens as it was.
- **A team must own the closing work.** Some steps are simply ended (a follow-up clinic). Others are not, because the answer still matters to someone.
- **Results pending at death still arrive.** A result nobody owns is the H-044 pattern. It may matter to the medical examiner, the coroner or a relative: for example an infection, or a genetic finding. One summary of GMC-based guidance notes genetic risk can be considered familial information [GMCSUM]. A pending result stays on a worklist until a named person has seen it.
- **Referrals.** The state table has no event for a death. A cancellation is a request and only the recipient's confirmation reaches `cancelled`, so a dead patient's referral stays open until someone answers, and it appears on the sender's worklist as `no-response`. The other organisation needs telling, and the closed loop should carry that fact.
- **Medicines, devices and equipment** at home need stopping or collecting. Medicines are not yet in the record. I did not research the rest.
- **Learning from deaths.** The National Quality Board's 2017 guidance asks trusts for a policy on engaging bereaved families and giving them the chance to raise questions; NHS England published guidance on working with families in 2018; a 2019 CQC review found some trusts struggling with this [NQB] [NHSEFAM] [CQCLFD]. So the record of what happened at the end must stay readable and honest, and the append-only model helps.

## A death recorded in error

Errors happen in both directions, and undoing them is a design need, not an edge case.

**Evidence.**
- PCSE says a death notification "can be recorded against the wrong patient", which it calls extremely rare [PCSEDED].
- The National Back Office investigates and resets the death status on PDS, and reports the outcome to the user who raised it [NBO].
- At Scarborough in 2024 a woman arriving for a scan was told she was dead. The GP said it had received an electronic instruction from PCSE; PCSE said marking patients dead is the GP's responsibility; NHS England spoke of an incorrect civil registration later corrected. Her carer's allowance was briefly stopped, and a later report says she was still listed as dead elsewhere [SCARB].
- A Portsmouth cancer patient was told he was deceased by his surgery in 2024 [ITVDEAD].
- A local record can show a patient living while PDS shows them dead [APPT].
- **United States.** The Social Security Administration posted about 5.6 million deaths in 2025, of which 12,504 (0.22 percent) were later found wrong. An audit found that for 45 percent of corrected records in its sample, staff had not documented why the death was recorded or why it was removed. So the agency cannot identify root causes [SSAOIG]. This is the exact shape of the risk: a small rate, on large numbers, with reasons not kept.

**Three different mistakes (my reasoning).**
1. A living person is recorded as dead.
2. The right fact is on the wrong record: one person's death on another's record, because two people were confused. The person who died remains "alive" while a stranger is "dead".
3. A dead person is recorded as alive, or a formal status conflicts with a local one.

**What this project would need.**
- **Append-only withdrawal.** A death is never deleted. A new version with status `withdrawn in error`, a reason, an author and the evidence. The history remains, as for an allergy `entered-in-error`.
- **A restore list.** Each effect (a stopped recall, a cancelled appointment, an ended proxy, a closed pathway) is tied to the assertion that caused it. Withdrawal shows a person the list and asks them to act. Automatic restoration of a cancelled appointment is unsafe; the slot has gone (my reasoning).
- **A hard check at a read of a person who is present.** A patient at the desk, or an encounter opened, with an active death assertion is a stop-and-check, worded with care, because the person may be standing there.
- **A conflict state, not a winner.** Local and national disagreement is shown and worked by a named person, following PDS's own lesson that changes need checking [PDSLONG].
- **Merges.** Merging a record with a death assertion into a record with later activity must stop for review. The matcher scores identity today and does not know about death. A merge that follows a confusion is exactly how mistake 2 spreads.
- **A national code U.** A feed must handle removal as well as arrival. A subscriber that handles arrival only leaves a living person dead locally [NEMS].

## Who is told: next of kin, GP, teams and national services

- **Next of kin.** Sources say that next of kin have no automatic right of access to a deceased patient's record [AHRASEC]. I did not research who should be told, in what order, or the legal standing of the term, and a lawyer and clinicians must. This project has no contact person, next of kin or related-person type: a search of the service, matcher and UI code for these terms finds nothing.
- **An important trap.** A person named as next of kin may be estranged, unaware of the death, or the person the patient feared. See [protected information](../access-control/protected-information.md): domestic abuse, the proxy as abuser. A hidden-from list must outlive the patient (my reasoning).
- **GP and teams.** The child death guidance requires the GP to be told the fact and circumstances "so that the GP is able to support the family" [CDR]. A 1998 English survey found 56 percent of practices kept death registers and 40 percent identified bereaved relatives, with hospitals perceived as slower to notify than hospices [SOUTHTHAMES]. It is old, and I found nothing newer.
- **National demographics.** PDS in England. The project has no PDS link: the name does not appear in the service or UI source. Under the equity rule this is a pack: the core holds an assertion and a source; a pack supplies the national feed.
- **Who hears first.** A team learning from a dashboard, a porter from a list, or a relative from the portal are all failures (my reasoning). Staff need to learn of a death in a way that lets them act on it, without it reaching the family before a person has spoken to them.

**What this project would need.** A typed related-person record, with reason and dates, kept separate from a proxy: being told is not access. A notification list derived from the care team and open pathways, reviewed by a person.

## Confidentiality and access after death

### Law, as examples

- **Data protection law stops at death.** The ICO says personal information relates to a living person, so a subject access request cannot reach a dead person's information; access runs under the Access to Health Records Act 1990 or the Northern Ireland Order 1993 [ICOHEALTH].
- **The English Act.** Section 3(1)(f) gives the right to the patient's personal representative and to "any person who may have a claim arising out of the patient's death". Paragraph (g), added on 9 September 2024, covers a medical examiner [AHRA3]. Secondary sources say it reaches records from 1 November 1991 and that next of kin have no automatic right [AHRASEC].
- **The patient can refuse in advance.** Section 4(3): access shall not be given on an application under (1)(f) "if the record includes a note, made at the patient's request, that he did not wish access to be given on such an application" [AHRA4]. **So a note made while alive has legal force after death, and a record must be able to hold it and to find it.**
- **Professional duty.** The GMC's duty of confidentiality continues after death; what may be disclosed depends on the facts. Known objections should be respected. Disclosure may be required by law or under the duty of candour. Summaries cite the 2017 guidance, paragraphs 134 to 138 [GMCSUM] [GMCDIS]. I did not read the GMC text.
- **United States.** HIPAA protects a decedent's information for 50 years after death [HHSDEC]. A personal representative under state law acts for the estate. A covered entity may disclose relevant information to family and others involved in care or payment before death, unless that is inconsistent with a known prior expressed preference [HHSFAQ] [CFR510].
- **Australia.** Sources suggest My Health Record keeps a record for 30 years after death, or 130 years after birth if the date of death is unknown, with a 2018 change for cancelled records [MHRRET]. The evidence is weak: a forum quotation and news.

So access after death differs by country, by who is asking and by what the patient said. It is a pack, and it needs a date of death to compute anything.

### Retention

The [retention schedule](../../governance/retention-schedule.md) sets no periods. Secondary sources say GP records in England are kept 10 years after death under the NHS Code, and that since 1 August 2022 the last registered practice usually handles requests [RETGP] [PCSEACC]. I did not read the Code. A retention rule that counts from death is impossible with no death date, and the conflict between erasure and append-only (already open in the schedule) arrives here first.

### Proxy, portal and the patient's own access

- A Dutch hospital says a proxy loses access at death; a US form lists death among the ends of a proxy's authority; another US system makes the account inactive when told of the death [PROXYEND].
- One widow found her husband's MyChart disabled a day or so after he died, while trying to sort out a bill. A Toronto hospital network offers an "access to deceased patient's account" option that lasts one year [PORTALDEC].
- I found nothing on NHS App or GP online access after death.

**Today, a proxy survives death.** Terms are `{basis, scopes, expires_at}`, the expiry can be up to five years ahead, and nothing links them to a death. Basis `parental-responsibility` for a child who has died, or `power-of-attorney` (which my reasoning says ends at death; a lawyer must confirm), would carry on working. A `self` relationship has only an optional period, and nothing ends it at death: anyone holding the patient's login keeps reading the record and its access log.

**What this project would need.**
- Death ends the use of a proxy or patient login for active purposes, tied to the assertion so that a withdrawal restores it.
- A new, separate relationship for **a person acting after death**: a personal representative or claimant, with evidence, scope and an end date. Not a proxy continuing by default.
- Each decision on a request is recorded: who asked, on what basis, what was disclosed, what was withheld and why. Third-party content is reviewed by a person. The bulk route is the one to watch ([data exchange](../data-exchange/index.md)).
- **An after-death wishes record.** The section 4(3) note and "prior expressed preference" are both things a patient says while alive. They fit neither a security label nor today's consent provisions. They also need to survive a merge.
- An amendment asked for by someone other than the patient: the amendment workflow takes the patient role only.

## Child deaths and baby loss

- **England: the Child Death Review Statutory and Operational Guidance (2018).** Within 24 hours or the next working day the attending team should notify the Child Health Information System, the GP, other professionals such as health visitor and school nurse, the child death review partners through a Notification Form, and MBRRACE-UK for perinatal deaths (22+0 weeks to 28 days) [CDR]. The family gets a key worker as a single point of contact [CDR]. Data entry to the National Child Mortality Database is reported as mandatory from 1 April 2019 [NCMD]. eCDOP is widely used, so an EPR integrates rather than replaces.
- **The siblings' records** still run: their immunisations, their health visitor. A surviving twin needs special care. The NBCP mentions butterfly stickers on cots of a surviving baby [NBCPSB].
- **Stillbirth and miscarriage** answer part of [maternity](../maternity/index.md) open question 8.
  - The NBCP states rules that shape the record: for example, a baby born after 24 weeks who is known to have died before 24 weeks cannot be recorded or registered as a stillbirth [NBCPMC]. Terms depend on country.
  - Before a live birth there may be no baby `Patient`. A baby who lived, however briefly, is a patient who has died. The pathway, the woman's view and the reminders change in each case.
  - Parents name their babies, and a name matters to the family and to siblings [NBCPSB]. "Baby of ..." on a letter can hurt (my reasoning). The woman's own record carries the loss; see [survivor grief](../survivor-grief/index.md).
  - Parents' consent is not required for a post-mortem ordered by a coroner [NBCPSB]. A record of a declined hospital post-mortem should not imply that none took place.

**What this project would need.** A child and perinatal notification step on a template, a list of who was notified and when, and a stop on all the siblings' or mother's reminders that referred to the baby, reviewed by a person.

## Organ and tissue donation

- England moved to opt-out ("deemed consent") on 20 May 2020 [ODENG]. The Act allows consent to be deemed unless a person stated otherwise [ODACT]. A study aid lists exclusions (under 18, resident under 12 months, lacking capacity); that is a weak source. A 2024 article reports no discernible change in donation rates after three years [ODCAM].
- I did not find how the Organ Donor Register check and the family conversation work, or anything on US state registries.

**What this project would need.** The register is the truth, not a copy in the EPR. A stale local flag is a hazard in both directions. The moment that matters is before death, so a flag recorded only as a consequence of a death is too late (my reasoning). A record might show "see the register" and who checked. Nothing more should be built until a specialist nurse has been asked.

## Faith, culture and the delay of a certificate

- Muslim tradition ideally favours burial within 24 hours and at the latest within three days; a third-party guide says so [FARRA]. An NIHR evaluation notes the same emphasis across Jewish, Muslim and other faiths [NIHREVAL].
- Faith groups raised concern that medical examiner scrutiny would delay funerals, and the reforms were designed with this in mind. Rapid release can be requested [JEWISHNEWS].
- In the House of Lords in April 2025 a minister reported unpublished regional data that 88 percent of faith communities' urgent scrutiny requests were met [HANSARD]. I saw this in a search result only.
- Where a coroner is involved, burial waits for release, and a third-party guide cites Regulation 20 of the 2013 Regulations [FARRA]. Coroner delay is reported [JEWISHNEWS].
- ONS medians for registration rose in the period, and I do not know how much is the reform [ONSREG].

**What this project would need (my reasoning).**
- A family's request for urgent scrutiny is recorded as a request from the family, with the time, and it reaches a person at once. **It is never inferred from a recorded religion.**
- Requirements differ within each faith, so the core holds no rule for any faith. A family says what they need.
- The medical examiner's access works out of hours, because a two-day read delay is the failure this concerns.
- Preferences stated in life (care of the body, who may wash it, post-mortem objection) are worth holding as the patient's words. I found no standard for them. Scotland, the US and other countries were not researched here.

## The family's own care

A bereaved relative is usually a separate patient, perhaps in another organisation, perhaps not registered at all. So the family's care cannot be a section of the dead person's record.

- **Evidence that a flag is wanted.** The NBCP says it can help if the GP, midwife or health visitor marks bereaved parents' records "with an alert or sticker to notify other healthcare professionals to their bereavement" [NBCPSB]. Its neonatal guidance says a system should clearly signal it to staff [NBCPNN]. Ireland uses a bereavement symbol, and notes that some people may not want a visual identifier [HSESYM].
- **Evidence that consent governs it.** The NBCP says: if a woman declines to have her primary care team informed, "this decision must be respected", and a partner should be asked separately, with the response recorded [NBCPSB]. It also says hospitals should phone, because letters may take several days.
- **Bereaved people may have no other doctor to tell.** A person with no GP has nowhere for this flag to go (my reasoning).

**Proposed rules (mine; for review by people with lived experience).**
1. **A note on the survivor's own record, made by consent**, in that person's words where possible, dated, and reviewable. It says that they were bereaved, and only what they agree.
2. **No link between the records.** It is never a `Patient.link`, an identity link or a proxy. It gives no access to the dead person's record. It need not name them.
3. **No cause by default.** A survivor's note does not say how someone died unless they ask for it. A family member may not know, and a child may not have been told.
4. **Everyday visibility is a decision.** The banner is seen over a shoulder. A receptionist may need to know, so as not to ask after the person who died, but not why. The options and their leak routes follow the [protected information](../access-control/protected-information.md) table.
5. **Hidden from proxies** by default; a young person's own loss is also protected.
6. **It ends when the person says so.** Append-only means the note is ended, not deleted.
7. **Nothing about the survivor is written on the dead person's record.** That record can be disclosed to a personal representative or claimant, who may be another family member (my reasoning).

Bereavement support is on the [survivor grief](../survivor-grief/index.md) page.

## Staff

Staff lose patients too. They may have known a patient for years.

- NHS England, NHS Confederation, NHS Employers and Samaritans published a postvention toolkit for staff affected by suicide; a confidential bereavement line run by Hospice UK has been available to NHS staff [NHSEMP]. I did not confirm it still runs.
- The Royal College of Psychiatrists publishes guidance on supporting staff after a patient's suicide. Surrey-led research produced "Postvention guidance: supporting NHS staff after the death by suicide of a colleague" (2023). Primary care has less: a 2025 BJGP Life article describes the gap for GPs [RCPSTAFF] [SURREY] [BJGPLIFE].
- I found nothing on "second victim" support for other deaths, in this pass.

**What this project would need (my reasoning).**
- Telling the people who knew the patient, not only the responsible consultant. A community nurse should not arrive at a door to find the patient has died.
- No one should find out from an automated list or a cancelled visit. The notice should be a person's.
- Reviews after a death must not read as blame, and audit views of a dead patient's record are not automatically suspicious: staff often look back, which is a human need and also a snooping risk. The access rule needs thought, and a local policy.
- A learner's or a colleague's own relative may be the patient. See the staff row in [protected information](../access-control/protected-information.md).

## Compared with this project

| Need | Today | Gap | Verified against the code |
|---|---|---|---|
| Record a death | **Nothing.** No field, table, route or screen | The whole of "Recording a death" | Case-insensitive search for `death`, `deceased`, `coroner`, `bereave`, `grief` in `src` of the service, matcher and UI, plus the migrations and service tests: **no hits** |
| FHIR `Patient.deceased[x]` | On write, not in `MAPPED`, so a body carrying it is refused 422; on read, never present | Mapping, in both directions. A reader sees absence, which is not "alive" | `src/interop/fhir_patient.rs` (`MAPPED`), `src/controllers/fhir.rs` |
| HL7 PID-29, PID-30 | Not read. Only named PID fields are. A registration update with a death indicator is applied and accepted with no warning | Read them; do not acknowledge a death we ignored | `src/interop/hl7_adt.rs` (by reading, not run) |
| Discharge on death, ADT A03 | Refused `AR`; handled events are A01, A04, A08, A28, A31. So the feed cannot carry a discharge | Discharge and death events; A40 merges are also not built | `HANDLED_EVENTS`, the test `other_message_types_and_events_are_unsupported…`, `src/adt_feed.rs` header |
| National demographics | No PDS link | A pack, with arrival and removal (codes 1, 2, U) | Search of service and UI `src` for `PDS`: no hits |
| Stop outbound contact | Outbox is on only by setting; **nothing enqueues**, and only a policy-and-consent allow reaches the sink | A death check at send time in `outbox.rs`, before the first real sink | `src/outbox.rs`, [service spec](../../electronic-patient-record-service-with-rust/spec/index.md) |
| Close a referral | 13 events, none for death; `cancelled` needs the recipient to confirm; `no-show` is available; a timeout worklist would flag it | A death event; no no-show after death | `src/referral.rs`, `src/referral_timeouts.rs` |
| Pathways | Direction only ([5.2.1](../05-domain-model.md)); not built | "Patient died" as a recorded outcome | [5.2.1](../05-domain-model.md) rule 3 |
| Proxy ends at death | Ends only by revocation or `expires_at`, up to five years | Tie to the assertion; a separate after-death relationship | `src/proxy.rs` (`decide`), [service spec](../../electronic-patient-record-service-with-rust/spec/index.md) PA-5 |
| Portal and access log after death | `self` and `proxy` read `/api/me/...` while the relationship is in force | Rule for a dead patient's login; a named person who may read the access log | `controllers::me`, service spec |
| Medical examiner | Eleven roles, none for this; seven purposes, none for this | Role, purpose, time-bound access | `src/policy.rs` (`Purpose`, roles) |
| Next of kin, contact, related person | No type; relationship kinds are `care-team`, `active-encounter`, `self`, `proxy` | A related person who has no access | `src/relationships.rs`; search for `next_of_kin`, `emergency_contact`, `related_person`: no hits |
| After-death wishes (AHRA s.4(3)) | Consent provisions permit or deny by role, purpose, resource, label, period | A note made in life, with effect after death, surviving a merge | [service spec](../../electronic-patient-record-service-with-rust/spec/index.md) SE-2 |
| Age on the banner | `ageOn(birth_date, today, zone)` always counts to today | Age at death; a clear, gentle banner state | `src/lib/patient.ts`, `src/lib/PatientBanner.svelte` (UI) |
| Merge with a death | Matcher `Patient` holds names, birth date, sex, identifiers, addresses, telecom; `links::judge` cannot see a death | A merge stop; where the fact lives | `electronic-patient-record-matcher-with-rust/src/patient.rs` |
| Undo | Append-only versions, `entered-in-error` on observations and allergies | A status, a reason, a restore list | [service spec](../../electronic-patient-record-service-with-rust/spec/index.md) |
| Retention from death | Periods not set; "kept for ever by default" for audit | A date of death; the erasure question | [retention schedule](../../governance/retention-schedule.md) |
| Implants for the MCCD | No implant list | Part of a wider gap | Not searched beyond the absence of a record type in the service spec |
| Family flag | None; labels exist in `policy.rs` but no record carries one | Rules 1 to 7 above | `src/policy.rs`; SE-4 in [protected information](../access-control/protected-information.md) |

**What surprised me.** The feed today accepts a message that says a patient died, updates the name, and says "accepted". That is correct for what it does and quietly wrong for a clinician: the sender believes the death has been received. It stays latent while no staff member relies on the feed, and it should be fixed before someone does.

## Candidate hazards

For the Clinical Safety Officer to review. None is accepted or closed, and none is in the [hazard log](../../governance/clinical-safety/hazard-log.csv). The ids are working labels.

- BV-1 **A living person is recorded as dead**, so care, medicines, screening, a proxy or benefits stop, and they may be told so in person.
- BV-2 **One person's death is filed on another's record**, so one is wrongly dead and the other wrongly alive, and care continues for a person who died.
- BV-3 **A person who died stays alive in the record**: letters, recalls, reminders, surveys or no-show letters reach the family; a formal status arrives months later.
- BV-4 **A withdrawn death does not restore what it stopped**: a screening recall or a monitoring plan never restarts, and nobody knows (the SSA undocumented-reason shape).
- BV-5 **A result pending at death has no owner**, and something a relative or coroner needed is lost (H-044 pattern).
- BV-6 **Death closes a pathway silently**, so a safeguarding concern, a sibling's or relative's care, or an infection follow-up is lost.
- BV-7 **A proxy or a shared login continues to read a dead person's record**, or a family member discovers something the patient kept private. The disclosure cannot be undone.
- BV-8 **The patient's refusal of access after death is not found**, because it was not recorded or was lost in a merge, and records are disclosed contrary to it.
- BV-9 **Records are released to the wrong person**, or third-party information about a living relative goes out with them.
- BV-10 **Next of kin who should not be told is told**: an abuser, an estranged relative, or a person the patient asked to exclude.
- BV-11 **The fact of death reaches the family by a machine** (a portal, a cancelled appointment, a text) before a person has spoken to them.
- BV-12 **A cause is shown as settled when it is not**, for example a suspected suicide or an inference from a code before a coroner's conclusion.
- BV-13 **A merge carries a death onto a living person**, or removes it from a dead one.
- BV-14 **A stale organ donation flag** contradicts the register, in either direction.
- BV-15 **A request for urgent scrutiny for burial is lost or delayed**, or a family's faith is assumed from a field.
- BV-16 **A survivor's flag discloses a death or its cause** through a banner, a letter, a proxy or a shared screen.
- BV-17 **A survivor's information is written on the dead person's record** and later disclosed to another family member.
- BV-18 **A staff member learns of a patient's death by arriving at the home**, or a clinician continues a plan on old information.
- BV-19 **A child's sibling or a surviving twin is treated wrongly** because a reminder or record mixes up the two.
- BV-20 **A message carrying a death indicator is acknowledged and ignored** by the feed, so the sender believes it was received. Latent today.
- BV-21 **The stop-all rule on a reported assertion is abused or mistaken**, so a false report silently stops care. It needs a fast check by a person.
- BV-22 **Absence of a death assertion is read as alive**, in an export or a count.

## Ideas

Proposals only. The order is mine.

1. **Write a clinical-impact spec before any letter or portal route is built.** Death is cheaper to design in now.
2. **Add an append-only `death_assertions` store**, not a column on the identity record, because a death has evidence, status and conflict. A read model gives the one banner answer. Whether the matcher's `Patient` also carries a date of death, for matching and merge stops, is open question 3 below; the rule is to change the matcher and its spec, never to add an adapter.
3. **A route census for the living.** In the manner of the existing census tests: every outbound route has a test that an active death assertion stops it. A new route fails until it names its behaviour.
4. **An effects ledger.** Every automatic effect records the assertion that caused it. Withdrawal lists them for a person.
5. **Stop on report, confirm by a person (decided, OQ-BV1).** Any assertion holds outbound contact at once. A named worker confirms within a period that a clinician sets.
6. **Map FHIR and HL7.** `deceasedDateTime` when a date is known, `deceasedBoolean` otherwise, never both [FHIRDEC]; read PID-29 and PID-30; send a message that says when a death could not be applied.
7. **A pack for national feeds**: PDS events 1, 2 and U in England, latest by `meta.lastUpdated`, and other country packs.
8. **A pathway outcome of "patient died"**, and a template step for "what is still owed". Add a referral event for a death, with a spec, a table test and a change to the recipient's confirmation rule.
9. **A person-acting-after-death relationship**, separate from proxy, with basis, evidence, scope and end date. Under English law its bases come from section 3(1)(f) and (g); the US and other countries are a pack.
10. **An after-death wishes record**, in the person's words, including the section 4(3) note. It must survive merges and show on a request.
11. **A worklist for a death**: who to tell, what is open, what is stopped, who owns the results. A named owner, closed like a referral.
12. **A medical examiner view** and role, read-only, time-bound, audited, with a purpose code.
13. **A gentle banner state**: date of death, age at death, source and status. Tested with bereaved people and with staff. No red, no skull, no jokes in test data.
14. **Measure it.** Time from death to assertion, letters sent after an assertion (target none), withdrawals and their reasons, and restored items. A planted-failure test in the manner of [measuring clinical safety](../clinical-safety/measurement.md).
15. **Add death scenarios to the synthetic data**: expected, sudden, coroner, wrong patient, withdrawn, baby, twin.

## Open questions

1. Who may record each status, and may a receptionist record a report from a family member?
2. Decided 2026-11-04 by the maintainer (OQ-BV1): any report holds contact until a person confirms it. Still to settle: who confirms, and how soon a held contact that matters, such as an appointment for a living patient, is released.
3. Does a date of death belong on the matcher's `Patient`, and does it change a match?
4. What may a next of kin be told, in which country, when the patient said nothing?
5. Does a section 4(3) note need particular wording or form, and may a patient make it through a portal?
6. Who reads a dead patient's access log, and may a family ask who looked?
7. How long does a dead patient's care relationship last, for staff finishing work or looking back?
8. What is the right question about a recorded cause: does the EPR hold it at all, or only point to the certificate?
9. How should a record be marked after a stillbirth or a miscarriage, and how does it answer [maternity](../maternity/index.md) question 8? Parents must help decide.
10. Can a person with no record at all be told anything, or supported, by a service using this system?
11. Who must read this before it is built: bereaved families, parents, faith groups, a medical examiner, a coroner's officer, a lawyer?
12. Which of these belong in [open questions](../16-open-questions.md) and the task list? This pass changes neither.

## Not verified

- **All law is a lay reading.** I read sections 3 and 4 of the 1990 Act as fetched summaries of legislation.gov.uk, with text up to date to 3 November 2026 on that page. The page also lists a prospective change to section 3 that I did not follow. I did not read the Medical Examiners Regulations, the Medical Certificate of Cause of Death Regulations or the Notification of Deaths Regulations themselves; the guidance on the last was read as extracted PDF text. Northern Ireland was not researched beyond the ICO's mention of the Order.
- **Could not open.** NICE pages (403), HHS pages (403; the wording rests on search summaries), Hansard (403), the eCFR, and the National Medical Examiner's guidance itself (only its landing page, which does not mention faith or urgent release). Whether that guidance covers urgent release was seen only in a third-party guide. The PDS FHIR API specification needs JavaScript and was not read; the event guide on `nhsconnect.github.io` was, and may not reflect the current API.
- **Possibly out of date.** Whether a digital MCCD is live. ONS medians (2020 to 2022) and the reform's effect on them: I did not find figures after 2022. The Records Management Code's retention period, which I saw only second-hand. The NBCP documents are 2022. The Child Death Review guidance is 2018 with later organisational change. The Hospice UK guidance has several editions; I read none. The 45 percent SSA finding is from a press release and concerns its sample.
- **Weak sources, used because I found nothing better.** News reports for Nottingham, Scotland, Scarborough and Portsmouth; a commercial funeral guide; a US supplier page; forum and news for Australia; a study aid for organ donation exclusions; local forms for proxy endings. The Scotsman piece is about a decade old.
- **Searched and not found.** Any measure of how often letters reach dead patients. How screening programmes stop. NHS App after death. US state registries for donation. Who should be told first, as a matter of law or guidance. Any EPR vendor's design for undoing a death.
- **Not researched.** Funeral and mortuary systems, medical devices at home, billing and insurance after death, research data after death, deaths abroad, cross-border records, deaths in custody beyond the coroner rule, medicines disposal, advance decisions.
- **The code** was read and searched, not run, on 2026-11-04. The statement about PID-29 and PID-30 comes from reading the mapper and finding no reference to those fields.
- Nobody bereaved, no clinician, medical examiner, coroner's officer or lawyer has seen this page. The hazards are my reading, not a hazard analysis.
- Link check 2026-11-04: these cited pages returned 404. They may have moved, but what rests on them is unconfirmed: https://oig.ssa.gov/assets/uploads/2026-july-numident-death-entry.pdf; https://nrscotland.gov.uk/research/guides/statutory-registers/deaths; https://www.hse.ie/eng/about/who/acute-hospitals-division/woman-infants/bereavement-care/pathway-for-first-trimester-miscarriage.pdf; https://www.hse.ie/eng/about/who/acute-hospitals-division/woman-infants/bereavement-care/pathway-for-stillbirth.pdf; https://uhn.ca/PatientsFamilies/Patient_Services/Documents/myUHN-Portal-Access-Form.pdf

## Sources

All read 2026-11-04. (fetched) means I fetched the page or PDF and read a summary or extracted text of it; (search summary) means a search tool's summary of results; (snippet) means a short extract or title in search results.

- [NOTIF] Guidance for registered medical practitioners on the Notification of Deaths Regulations, September 2024: https://assets.publishing.service.gov.uk/media/66d044a059b0ec2e151f847e/Guidance_for_registered_medical_practitioners_on_the_Notification_of_Deaths_Regulations__web_.pdf (fetched, PDF text)
- [GOVOVERVIEW] GOV.UK, An overview of the death certification reforms: https://www.gov.uk/government/publications/changes-to-the-death-certification-process/an-overview-of-the-death-certification-reforms (fetched)
- [MEGUIDE] NHS England, National Medical Examiner's guidance, landing page, first published 30 July 2024: https://www.england.nhs.uk/publication/national-medical-examiners-guidance-for-england-and-wales/ (fetched; the guidance itself not read)
- [SI493EM] Explanatory Memorandum to the Medical Examiners (England) Regulations 2024: https://www.legislation.gov.uk/uksi/2024/493/pdfs/uksiem_20240493_en_001.pdf (search summary)
- [MEREPORT] NHS England, National Medical Examiner report 2024: https://www.england.nhs.uk/long-read/national-medical-examiner-report-2024/ (search summary)
- [AHRA3] Access to Health Records Act 1990, s.3: https://www.legislation.gov.uk/ukpga/1990/23/section/3 (fetched)
- [AHRA4] Access to Health Records Act 1990, s.4: https://www.legislation.gov.uk/ukpga/1990/23/section/4 (fetched)
- [AHRASEC] Trust and GP guidance on deceased patients: https://www.nbt.nhs.uk/about-us/information-governance/subject-access-requests-sar/rights-deceased ; https://gpnotebook.com/en-GB/pages/general-practice/access-to-health-records-of-deceased-patients ; https://www.lincslmc.co.uk/ahra-for-deceased-patients (search summary)
- [ICOHEALTH] ICO, right of access, health data: https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/individual-rights/right-of-access/health-data (fetched)
- [GMCSUM] MDU, Confidentiality and disclosing information after death: https://themdu.com/guidance-and-advice/guides/disclosure-after-death ; MDDUS, Patient confidentiality after death: https://mddus.com/resources/resource-library/risk-alerts/2020/september/patient-confidentiality-after-death ; FICM: https://ficm.ac.uk/media/1456 (search summary)
- [GMCDIS] GMC, Disclosing medical records after death: https://www.gmc-uk.org/professional-standards/learning-materials/disclosing-medical-records-after-death (snippet)
- [HHSDEC] HHS, Health information of deceased individuals: https://www.hhs.gov/hipaa/for-professionals/privacy/guidance/health-information-of-deceased-individuals/index.html (search summary; fetch returned 403)
- [HHSFAQ] HHS FAQ on disclosing about a decedent: https://www.hhs.gov/hipaa/for-professionals/faq/does-hipaa-permit-a-covered-entity-to-disclose-information-about-a-decedent/index.html (search summary; fetch returned 403)
- [CFR510] 45 CFR 164.510: https://www.law.cornell.edu/cfr/text/45/164.510 (search summary)
- [MHRRET] Australia, forum quotation of the Act: https://choice.community/t/myhealth-record-megathread/15714?page=15 ; Pulse+IT on the 2018 amendments: https://www.pulseit.news/australian-digital-health/feds-introduce-privacy-and-record-deletion-amendments-to-my-health-record-act ; Senate committee: https://aph.gov.au/Parliamentary_Business/Committees/Senate/Community_Affairs/MyHealthRecords/Report/c02 (search summary, weak)
- [RETGP] MDU, updated guidance on records management and retention: https://www.themdu.com/guidance-and-advice/latest-updates-and-advice/updated-guidance-on-medical-records-management-and-retention ; NHS England, keeping GP records: https://www.england.nhs.uk/long-read/keeping-gp-records/ (search summary)
- [PCSEACC] PCSE, access to medical records: https://pcse.england.nhs.uk/help/medical-records/access-medical-records-members-public (search summary)
- [PDSCODE] HL7 UK, UK Core death notification status value set: https://simplifier.net/hl7fhirukcorer4/valueset-ukcore-deathnotificationstatus/$download?format=xml ; NHS Data Dictionary archive: https://archive.datadictionary.nhs.uk/DD%20Release%20May%202020/data_dictionary/data_field_notes/p/pds/pds_death_notification_status_code_de.asp@shownav=0.html (search summary)
- [NEMS] NHS Digital, PDS death notification event: https://nhsconnect.github.io/Events-Management/pds_death_notification.html (fetched)
- [DPN] NHS England Digital, Mandating mortality data updates onto PDS, last edited 4 July 2023: https://digital.nhs.uk/data-and-information/keeping-data-safe-and-benefitting-the-public/how-we-look-after-your-health-and-care-information/directions-and-data-provision-notices/data-provision-notices-dpns/mortality-data-flows/ (fetched)
- [PDSLONG] NHS England, Personal demographic service: https://www.england.nhs.uk/long-read/personal-demographic-service-pds/ (fetched)
- [PCSEDED] PCSE, Deductions and amendments: https://pcse.england.nhs.uk/help/patient-registrations/deductions-and-amendments (fetched)
- [NBO] NHS Digital, National Back Office data quality incidents (mirror): https://nhsd-proxy.openprescribing.net/services/personal-demographics-service/national-back-office-for-the-personal-demographics-service/data-quality-incidents (fetched)
- [SIXB] 6B, use cases of the PDS API (supplier): https://6b.health/insight/use-cases-real-world-applications-of-nhs-pds-api-in-digital-health-product-development-referral-patient-portals/ (search summary)
- [APPT] appt-health, patient alerts (software help): https://help.appt-health.co.uk/tutorial/what-is-patient-alerts (search summary)
- [SCARB] GB News: https://www.gbnews.com/news/woman-told-dead-hospital-scan-scarborough ; Yahoo: https://au.news.yahoo.com/woman-puzzled-hospital-tells-her-162724476.html (search summary)
- [ITVDEAD] ITV News: https://itv.com/news/meridian/2024-08-07/cancer-patient-told-he-was-deceased-by-doctors-surgery (search summary)
- [SSAOIG] SSA OIG press release 2 July 2026: https://oig.ssa.gov/news-releases/2026-07-02-audit-finds-social-security-administration’s-employees-did-not-always-document-reasons-for-removing-beneficiaries’-incorrect-death-postings/ (fetched); report: https://oig.ssa.gov/assets/uploads/2026-july-numident-death-entry.pdf (search summary)
- [NOTTS] Nottingham Post, 2 November 2020: https://www.pressreader.com/uk/nottingham-post/20201102/281861531013280 (search summary)
- [SCOTSMAN] The Scotsman: https://www.scotsman.com/news/nhs-questionnaires-sent-to-dead-patients-a-disgrace-1-3849358 (search summary)
- [TELLUS] Nottinghamshire County Council, notify a death: https://www.nottinghamshire.gov.uk/media/tk0ggex4/notify-a-death.pdf (search summary)
- [ONSREG] ONS, Impact of registration delays on mortality statistics, 2022 and 2020: https://www.ons.gov.uk/peoplepopulationandcommunity/birthsdeathsandmarriages/deaths/articles/impactofregistrationdelaysonmortalitystatisticsinenglandandwales/2022 ; https://www.ons.gov.uk/peoplepopulationandcommunity/birthsdeathsandmarriages/deaths/articles/impactofregistrationdelaysonmortalitystatisticsinenglandandwales/2020 (search summary)
- [ONSSUI] ONS FOI: https://www.ons.gov.uk/aboutus/transparencyandgovernance/freedomofinformationfoi/thenumberofsuicidesintheukin2020andthenumberofsuicidesawaitingacoronersinquest (search summary)
- [HOSPICEVOD] Hospice UK, Care After Death: Registered Nurse Verification of Expected Adult Death: https://www.hospiceuk.org/care-after-death-rnvoead-guidance-6th-edition (search summary)
- [SCOTDCRS] Healthcare Improvement Scotland, DCRS annual report 2024-25: https://www.healthcareimprovementscotland.scot/wp-content/uploads/2025/10/DCRS-Annual-Report-2024-25.pdf ; letter to the Scottish Parliament: https://parliament.scot/-/media/files/committees/health-social-care-and-sport-committee/correspondence/2022/20220129_mphwhs_medical_certificates_of_cause_of_death.pdf (search summary)
- [NRS] National Records of Scotland, statutory register of deaths: https://nrscotland.gov.uk/research/guides/statutory-registers/deaths (search summary)
- [FHIRDEC] FHIR patient definitions: https://build.fhir.org/branches/master/patient-definitions.html ; HAPI thread: https://groups.google.com/g/hapi-fhir/c/me79sFZcDwI (search summary)
- [USCORE] HL7 Jira FHIR-40046: https://jira.hl7.org/browse/FHIR-40046 (snippet; I matched the summary to this issue by number and did not open it)
- [VRDR] IHE Vital Records Death Reporting: https://build.fhir.org/ig/IHE/QRPH.VRDR/StructureDefinition-vrdr-decedent-definitions.html (snippet)
- [NBCPSB] NBCP, Stillbirth Bereavement Care Pathway, England, July 2022: https://www.nbcpathway.org.uk/wp-content/uploads/2024/03/NBCP-Stillbirth-July-2022.pdf (fetched, PDF text)
- [NBCPMC] NBCP, Miscarriage, ectopic and molar pregnancy: https://www.nbcpathway.org.uk/wp-content/uploads/2024/03/NBCP-Miscarriage-short-guidance-July-2022.pdf (fetched, PDF text)
- [NBCPNN] NBCP, neonatal short guidance: https://www.nbcpathway.org.uk/wp-content/uploads/2024/03/NBCP-Neonatal-short-guidance-July-2022.pdf (search summary)
- [HSEPATH] HSE Ireland, pathway for stillbirth: https://www.hse.ie/eng/about/who/acute-hospitals-division/woman-infants/bereavement-care/pathway-for-stillbirth.pdf ; pathway for first trimester miscarriage: https://www.hse.ie/eng/about/who/acute-hospitals-division/woman-infants/bereavement-care/pathway-for-first-trimester-miscarriage.pdf (search summary)
- [HSESYM] HSE Ireland, bereavement symbol pathway: https://pregnancyandinfantloss.ie/wp-content/uploads/2025/04/BEREAVEMENT-SYMBOL-PATHWAY-NOV-2024.pdf (search summary)
- [CDR] Child Death Review Statutory and Operational Guidance (England): https://assets.publishing.service.gov.uk/government/uploads/system/uploads/attachment_data/file/859302/child-death-review-statutory-and-operational-guidance-england.pdf (fetched, PDF text)
- [NCMD] NCMD, notification guidance: https://www.ncmd.info/?p=7175 ; Manchester CDOP annual report: https://democracy.manchester.gov.uk/documents/s38127/2021-2022%20Manchester%20CDOP%20Annual%20Report.pdf (search summary)
- [ODENG] NHS Organ Donation, Organ donation law in England: https://www.organdonation.nhs.uk/uk-laws/organ-donation-law-in-england/ (search summary)
- [ODACT] Organ Donation (Deemed Consent) Act 2019: https://www.legislation.gov.uk/ukpga/2019/7/enacted (search summary)
- [ODCAM] Why did England change its law on deceased organ donation in 2019?: https://cambridge.org/core/services/aop-cambridge-core/content/view/622DDD66E330A6EDAF29E9FD31B713A0/S1744133124000112a.pdf/why-did-england-change-its-law-on-deceased-organ-donation-in-2019-the-dynamic-interplay-between-evidence-and-values.pdf (search summary)
- [JEWISHNEWS] Jewish News: https://www.jewishnews.co.uk/death-certification-changes-made-law/ ; https://www.jewishnews.co.uk/new-death-certificate-reforms-will-aid-jewish-community/ (search summary)
- [HANSARD] Lords, Funerals: Death Certificate Delays, 3 April 2025: https://hansard.parliament.uk/lords/2025-04-03/debates/D31B03E5-B1A6-4394-B3A0-401AB5D1A977/FuneralsDeathCertificateDelays (search summary; fetch returned 403)
- [NIHREVAL] NIHR, Rapid evaluation of the 2024 death certification reforms: https://njl-admin.nihr.ac.uk/document/download/2049530 (search summary)
- [FARRA] Farra, Muslim burial in the UK (commercial guide): https://withfarra.co.uk/guides/muslim-burial-requirements-uk (search summary, weak)
- [NG142] NICE NG142: https://www.nice.org.uk/guidance/ng142 (search summary; recommendations page returned 403)
- [EPACCS] Allsop et al., accepted manuscript: https://eprints.whiterose.ac.uk/180801/3/Main%20Document_accepted_Allsop%20et%20al%202021.pdf (search summary; I matched the claim to this document from the result list and it should be checked)
- [RESPECTLEEDS] ReSPECT manuscript: https://eprints.whiterose.ac.uk/209876/3/ReSPECT_manuscript_Jan24%20%281%29.pdf (search summary)
- [PROXYEND] Spaarne Gasthuis: https://www.spaarnegasthuis.nl/app/uploads/2021/09/dsg-1401-Giving-someone-else-access-to-your-medical-records.pdf ; HaysMed: https://www.haysmed.com/wp-content/uploads/2023/06/portal-adult.pdf ; Phoebe Health: https://WWW.PHOEBEHEALTH.COM/media/file/PaperworkRequirements_Proxy.pdf (search summary)
- [PORTALDEC] Becker's: https://www.beckershospitalreview.com/healthcare-information-technology/ehrs/what-happens-to-patients-mychart-accounts-when-they-die ; University Health Network: https://uhn.ca/PatientsFamilies/Patient_Services/Documents/myUHN-Portal-Access-Form.pdf (search summary)
- [NQB] National Quality Board, Learning from deaths: https://www.england.nhs.uk/wp-content/uploads/2017/03/nqb-national-guidance-learning-from-deaths.pdf (search summary)
- [NHSEFAM] NHS England, guidance on working with bereaved families: https://www.england.nhs.uk/2018/07/nhs-publishes-guidance-to-help-trusts-learn-from-deaths/ (search summary)
- [CQCLFD] MDDUS on the CQC review: https://www.mddus.com/resources/resource-library/news-digest/2019/march/nhs-trusts-struggling-to-engage-with-bereaved-families (search summary)
- [SOUTHTHAMES] Bereavement care in general practice: a survey in South Thames Health Region: https://core.ac.uk/works/427416 (search summary)
- [NHSEMP] NHS Employers, Suicide prevention and postvention: https://www.nhsemployers.org/articles/suicide-prevention-and-postvention (search summary)
- [RCPSTAFF] RCPsych: https://elearninghub.rcpsych.ac.uk/products/RCPsych_guidance_mental_health_organisations_staff_support_following_patient_suicide (search summary)
- [SURREY] Postvention guidance: https://keele-repository.worktribe.com/output/718833/postvention-guidance-supporting-nhs-staff-after-the-death-by-suicide-of-a-colleague (search summary)
- [BJGPLIFE] BJGP Life: https://bjgplife.com/the-need-to-support-colleagues-when-a-patient-dies-by-suicide/ (search summary)
- This repository: `electronic-patient-record-service-with-rust/src/` (`policy.rs`, `proxy.rs`, `relationships.rs`, `outbox.rs`, `referral.rs`, `referral_timeouts.rs`, `interop/fhir_patient.rs`, `interop/hl7_adt.rs`, `adt_feed.rs`, `controllers/fhir.rs`), `electronic-patient-record-matcher-with-rust/src/patient.rs`, `electronic-patient-record-ui-with-svelte/src/lib/patient.ts`, `PatientBanner.svelte`.
