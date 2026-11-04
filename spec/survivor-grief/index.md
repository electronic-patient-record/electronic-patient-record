# Survivor grief: counselling and support for people bereaved

Public-source research read 2026-11-04 plus this repository's own files. A proposal, not clinical or legal advice: nothing here is built, no clinician or person with lived experience has reviewed it, and it is NOT FOR CLINICAL USE.

## Related links

[Bereavement](../bereavement/index.md) (the companion page: what the record does when a patient dies), [Domain model 5.2.1](../05-domain-model.md) (pathway first), [Pathway design](../pathway-design.md), [Closed-loop referral](../interoperability/closed-loop-referral.md), [Protected information](../access-control/protected-information.md), [Roles and assignments](../access-control/roles-and-assignments.md), [Access control](../access-control/index.md), [Maternity](../maternity/index.md), [Manual steps](../manual-steps/index.md), [Data exchange](../data-exchange/index.md), [Service spec](../../electronic-patient-record-service-with-rust/spec/index.md), [hazard log](../../governance/clinical-safety/hazard-log.csv).

"Survivor" here means a person bereaved by a death, including by suicide, traumatic death, or the loss of a child or baby. Some people dislike the word, and the term should be chosen with them. England is the worked example; Wales, the United States, Ireland and Australia are examples, never "the rule" ([equity rule](../02-scope.md)). Tags such as [NG105] point to Sources. "My reasoning" means there is no source behind a statement. Every statement about practice describes what a source says, never an instruction.

**Content rule.** This page is about records and services. It gives no detail about how anyone died.

## Why this page exists

Most grieving people never need a health service, and grief is not an illness. Some do need help, and it often fails to arrive: the UK Commission on Bereavement reported that over 40 percent of those who wanted formal bereavement support did not get any [UKCB]. A record cannot comfort anyone. What it can do is avoid harm, and hold what a person has agreed to, so that they do not have to tell their story again at each door.

Four things make this different from other referrals (my reasoning):

1. **The person who needs help may not be the patient the service knows.** The patient has died. The survivor may be a stranger to this system.
2. **Much of the help is outside the NHS**, from volunteers and charities, with no EPR and no reply.
3. **A record of grief speaks about someone else**: the person who died, and other living relatives.
4. **Timing and consent are unusually delicate.** A kind contact in one week is an intrusion in another.

## Models of support

### The three-component model

NICE set out a three-component model of bereavement support in its 2004 cancer guidance [TIERS]. The Welsh national framework uses it (read as a summary of the fetched page) [WALES]:

| Component | Who | What |
|---|---|---|
| 1, universal | "Available to all bereaved people" | Information and signposting: conversation, leaflets, online, knowing when to seek more help |
| 2, selective or targeted | "Available to some bereaved people" | Reflection in individual or group sessions, peer support, groups for a type of loss such as suicide |
| 3, indicated | "Available to a small number of bereaved people" | Specialist: mental health services, psychological support, specialist counselling |

The Welsh framework stresses that the components depend on each other: if one lacks capacity, the later ones are overwhelmed. Its provider measures are concrete: initial contact within five working days of an enquiry, a needs and risk assessment by a trained person, an estimated wait at registration, contact at least every four weeks while waiting, a review after twelve weeks, and outcome measures such as CORE-10 (not suitable for young children) [WALES]. Those are Welsh measures, shown as one example.

A public-health framing is often linked to this model [TIERS]. **I searched for the often-quoted proportions of people in each tier and did not find them in a source I could check, so none is given.**

### Evidence: help for those who need it, not for everyone

A 2008 meta-analysis of 61 studies found a small effect after treatment and none significant at follow-up; universal interventions did no better than the passage of time; those with more distress benefited more [CURRIER]. Critics argue that these reviews mix unlike approaches and populations, and that people who seek help within about three months do about as well as in psychotherapy generally [CRITIQUE]. A 2020 review found the evidence for bereavement groups weak and heterogeneous [GROUPS]. A systematic review of support after advanced illness exists; I did not read it [HARROP].

**What this means for a system (my reasoning).** Offering information to everyone is reasonable. Referring everyone automatically is not. An EPR should make an offer possible, not generate referrals by rule.

### Other countries, as examples

- **United States, hospice.** A hospice must "make bereavement services available to the family and other individuals in the bereavement plan of care up to 1 year following the death of the patient", under an organised programme supervised by a qualified professional [HOSPICE]. A trade body describes at least 13 months; the regulation is the authority. So a hospice must hold a plan for people who may never be its patients.
- **Ireland.** National standards and pathways for pregnancy loss and perinatal death [HSESTD].
- **Scotland, Wales, Northern Ireland.** The NBCP has Welsh and Northern Irish versions [NBCPW] [NBCPNI]. I did not read others.

### What this project would need

Bereavement support is a pathway under [5.2.1](../05-domain-model.md): owned, with a next step, or stated to have none. It belongs on **the survivor's own record**, not the dead person's (see [bereavement](../bereavement/index.md), rule 7). A template is a pack from a deployment, reviewed by named people ([pathway design](../pathway-design.md)). A person with no record is the hard case: see Open question 1.

## Prolonged grief disorder, and the risk of medicalising grief

- **The definitions differ.** ICD-11 6B42 requires that grief persists beyond six months and beyond what is normal for the person's culture and religion, with significant impairment [ICD]. DSM-5-TR requires 12 months [DURATION]. I read neither official text, only sources quoting them.
- **The difference matters in numbers.** One comparison gives a diagnostic rate of 76 percent under ICD-11 and 52 percent under DSM-5-TR in its own sample, which is not a population rate [PGDPREV]. Aligning both at a year substantially raises agreement [DURATION].
- **The cautions (Eisma, 2023)** [EISMA]:
  - There is "no consensus-based definition of normal grief", and any time threshold on a continuum "will be arbitrary".
  - Severe, lasting grief is common after some losses, such as the death of a child, so diagnosing it risks "labeling normal grief variations as a medical disorder".
  - Many countries require a diagnosis for reimbursed care, a dilemma in both directions.
  - Vignette studies suggest that the public stigmatises people with these diagnoses.
  - Most evidence comes from Western and East Asian samples, and standard tools may not capture grief in other groups, such as Indigenous Australian and New Zealand peoples.
  - A large trial found no benefit from citalopram for grief symptoms, which warrants caution over medication.
- **Culture.** A 2025 study across the UK, Ireland and Ontario found people did not see six months as normal, and about a quarter of those meeting the other requirements could not say whether their grief exceeded cultural norms [REDICAN].

**What this project would need (my reasoning).**
- **The system never proposes this diagnosis from a date.** No rule fires at six or twelve months after a death. That is a medicalising prompt on a clock that the sources call arbitrary.
- A diagnosis, if a clinician makes one, is an ordinary coded entry that names its code system and version, because the two criteria differ. A diagnosis under one is not a diagnosis under the other.
- A coded diagnosis travels: a diagnosis of a mental disorder in a shared record has consequences a note of bereavement does not (my reasoning). The category sits under the mental health row in [protected information](../access-control/protected-information.md). A diagnosis on an insurer's or employer's form is a real route.
- A problem list should allow "bereaved" as a life event, not a disorder, so that a clinician is not pushed to pick a diagnosis to record anything.

## Suicide bereavement and postvention

"Postvention" means support after a suicide. This section is about the people left behind.

- **Guidance, England.** NICE NG105 (2018) has a section on supporting people bereaved or affected by a suspected suicide, and quality standard QS189 statement 5 says they are "given information and offered tailored support". Those affected can include relatives, friends, classmates, colleagues, first responders and professionals. Partnerships are expected to gather intelligence to identify them, and to offer information as soon as possible and again later. NICE notes people affected are themselves at higher risk [NG105] [QS189].
- **Strategy.** The Suicide Prevention Strategy for England 2023 to 2028 includes improving support for people bereaved by suicide [STRATEGY]. The NHS Long Term Plan committed to suicide bereavement support in every area by 2023/24, from £57m [ROLLOUT]. I found no confirmation of current coverage or funding.
- **Support After Suicide Partnership (SASP)** is a UK network of over 112 organisations, hosted by the Royal College of Psychiatrists. It coordinates a hub commissioned by NHS England [SASP]. *Help is at Hand* is the widely used booklet for people bereaved by suicide or other unexplained death [HIAH].
- **Other services.** Cruse's Support After Suicide, Survivors of Bereavement by Suicide and The Compassionate Friends are named by one integrated care board; Samaritans co-wrote the NHS staff toolkit [HIOW] [NHSEMP].
- **Risk.** A study of 3,432 young bereaved adults and a later review found that people bereaved by suicide were more likely to attempt suicide than those bereaved by sudden natural causes [PITMAN]. Risk does not mean most people; the system should state neither "at risk" nor "fine" as a fact about a person.
- **Delay.** Much data about suicides is available only after an inquest, so support could be late [SAS2016]. Registration of these deaths runs to about 5 to 6 months [ONSSUI].
- **United States.** *Responding to Grief, Trauma, and Distress After a Suicide* (2015), from the National Action Alliance, is a national blueprint [USGUIDE].
- **Australia.** StandBy Support After Suicide is a national programme, running since 2002, delivered by local partners, with support for up to two years; a 2026 submission says some of its services are not available in the ACT [STANDBY].

### Who may be contacted after a suicide

This is the most delicate design point on this page.

- **The English model is not the EPR.** Real-time surveillance is usually run by police or coroners, who are at the start of the pathway. Consent "must be given by the people being supported to have their data shared". Anonymous data for spotting clusters needs no consent, provided that no one is identifiable. SASP's sample wording has police ask permission, then pass the name, contact details, who died and basic information only to the named provider [RTS] [SAS2016].
- **Practice.** In London, with informed consent, integrated care boards get contact details and make a proactive offer; one south-east London service says about 70 percent of its referrals come this way [THRIVE] [SELMIND].
- **Proactive outreach works differently from a leaflet.** The 2016 Public Health England guide describes services that contact a family within days, and a study describes a model with "persistent pro-active contact" [SAS2016] [POSTV]. The guide also says the trauma can cause tension and conflict within families [SAS2016].
- **Not found.** A national consent standard, how a refusal is recorded, how often people decline, what happens to a person who was not at the scene, or anything about using a health record to identify the bereaved.

**What this project would need (my reasoning).**
- **The EPR does not generate a list of people bereaved by suicide.** It does not infer who they are from a next-of-kin field, a household or a family link. The cause is unsettled for months, the data may be wrong, and the list would be highly sensitive.
- **A referral comes in, with its consent recorded.** The record names who took consent, when, what for, and the named provider. No consent, no referral.
- **A clinician who knows a family may offer support directly**, as a person. A patient of a mental health team who died means a family that team may know; the team should ask what is wanted, and with whom. A hidden-from list must be honoured (see [bereavement](../bereavement/index.md)).
- **A refusal is respected and recorded, with a date**, in the sense that the person is not asked again by the same route. An offer can be repeated later if the person agreed to that.
- **Suspected is not concluded.** A survivor's record does not state that a death was a suicide as a fact. It records what the person says, in their words. Coroner's conclusions arrive late, and may differ from what the family believes.
- **Safety is a clinician's**. Any concern for the survivor's own safety follows the ordinary clinical route, not a bereavement flag.

## Traumatic and sudden death

Sudden, violent or unexplained deaths bring a coroner, sometimes police, sometimes media, and sometimes a trial. I did not research homicide bereavement or major incidents; this pass searched for suicide guidance only. Eisma lists unnatural death among losses where severe grief is common [EISMA]. A witness or first responder may be affected too [NG105].

**What this project would need (my reasoning).** Do not assume that next of kin and the person who is grieving are the same. Treat a survivor who may also be a witness or a suspect with care: notes can be requested by a court. A lawyer should advise before any field is designed for this.

## Baby and pregnancy loss

- **The National Bereavement Care Pathway** covers miscarriage, ectopic and molar pregnancy, termination for fetal anomaly, stillbirth, neonatal death and sudden infant death up to 12 months [NBCPALL]. A Commons Library briefing reports all English trusts have signed up, and that seven-day services are being set up [COMMONS]. I did not check progress.
- **Standards.** Among them: a parent-led bereavement care plan providing continuity "between settings and into any subsequent pregnancies", and all bereaved parents being told about emotional and specialist support, and referred if they ask [NBCPSB].
- **Records.** Offer a pregnancy-loss form to be added to the notes "to alert staff in any future pregnancy" [NBCPMC]. Consider a clinical alert. In a later pregnancy, disclose the history on an ultrasound request, with consent, to avoid miscommunication [NBCPMC]. The stillbirth pathway says parents should be told what follow-up appointments are for, because an unexpected purpose can cause distress [NBCPSB].
- **Partners.** The partner is asked separately whether their GP may be told, and the response is recorded [NBCPSB]. Partners can be overlooked; the pathway says follow-up by GP or health visitor is essential for them [NBCPSB].
- **Ireland.** A bereavement symbol on the record, with permission; a separate symbol and a carried card for pregnancy after loss. Some people may not want a visual identifier [HSESTD] [HSESYM] [CUMH].
- **Outcome measures in this setting.** Small uncontrolled evaluations at Petals, a Cambridgeshire charity, used CORE-OM [PETALS].

**What this project would need.**
- The stillbirth pathway's "parent-led care plan" is a pathway that crosses settings and pregnancies: the case for [5.2.1](../05-domain-model.md). The maternity pack needs it.
- The alert lives on the woman's record, with her agreement, for her next pregnancy. The partner has a separate decision on a separate record, so two records with two consents come from one event.
- Stop every reminder tied to the pregnancy or baby (see [bereavement](../bereavement/index.md)).
- A carried card or a parent-held summary can spare someone from retelling.

## Child and young person bereavement

- **No official count.** The Childhood Bereavement Network says no annual data exists. Modelled estimates vary: almost 40,000 children facing a parent's death in a year in 2015, and later about 46,300. It also reports around 1 in 29 children aged 5 to 16 bereaved of a parent or sibling [CBN]. Charities produce these figures, and the numbers differ by source.
- **Services.** Winston's Wish, founded 1992, and others [WW]. An NSPA guide covers children bereaved by suicide; I saw only its title [NSPACYP].
- **Not found.** Anything on flagging a child's GP record, or on what a school is told.
- **Child death.** Parents get a key worker after a child's death. See [bereavement](../bereavement/index.md).

**What this project would need (my reasoning).**
- A bereaved child's parent is usually the proxy, and is grieving too. The child may not know how someone died. So a note on a child's record does not state a cause, and the proxy view is a route that must be considered.
- As a young person grows, the note becomes theirs to end or to keep, as with other [young people's confidentiality](../access-control/protected-information.md) points.
- A child's bereavement may surface years later. A dated life event is more useful than an alert.
- Outcome tools may not suit young children, as the Welsh framework says of CORE-10 [WALES].

## Referral to services outside the NHS

### What the sources say

- **Cruse.** "Cruse services are self-referral." Clients contact their local service; professionals can help them find it. Cruse aims to reply to messages within two working days, and clients usually reach voicemail. Volunteers are supervised and trained but "do not have psychiatric or psychological training". Funding often limits services to their own area [CRUSE]. One Welsh listing says third-party referrals are not accepted; some Northern Irish listings show GP referral as a method [CRUSEWALES] [CRUSENI]. So routes differ by nation, and sometimes by branch.
- **Closed loop assumed.** The [referral model](../interoperability/closed-loop-referral.md) expects a recipient that accepts, declines and sends an outcome. In the [service](../../electronic-patient-record-service-with-rust/spec/index.md) a referral with no answer shows as `no-response`.

### The mismatch (my reasoning)

A referral to a service that takes self-referrals, keeps its own confidential notes and has no EPR will never close. It will stay on the sender's worklist as an alarm that means nothing, or worse, teach staff to dismiss the list. Pretending that the loop closes is also a hazard: a GP may believe that someone is being supported when no one rang the number.

### What this project would need

Three step types, so the words do not lie (my reasoning, for clinicians to test):

| Step | Means | Closes when | Loop |
|---|---|---|---|
| **Information given** | Component 1: a leaflet or a website, with the date | At once | None. Records only what was offered |
| **Signposted** | The person was told of a named service and chooses whether to contact it | At once, with a stated limit: "outcome will not be known" | An optional later question to the person, if they agreed |
| **Referred** | A service that accepts referrals received one, with the person's consent | By an answer, as today | Full. A contact details and consent note goes with it |

Further needs:
- **An organisation that has no system.** The recipient may be a charity with an email address. A referral record needs a recipient that is not a clinician, with secure sending, which NHSmail or a portal may or may not allow. See the same point in [care homes](../care-homes/index.md).
- **Outcome by consent.** A counselling service owes confidentiality to the client. A feedback note, if any, is attended, ended, or not taken up, sent only if the person agrees. No clinical content by default.
- **Ask the person.** The surest closed loop is the person: "Did that help? Would you like anything else?" At a moment that they chose.
- **A directory is content.** Services change. A directory needs an owner, a review date, an area and a language. A wrong phone number in a moment of grief is a harm. It is a pack, not code. The helpline number in one listing may be out of date, so I give none here.
- **Waiting.** A wait with no contact is a known failure, and the Welsh measures above fit a pathway step with a timer. Timers need the person's agreement to contact.
- **Risk.** Volunteers are not psychiatric services. A person with severe needs may need another route; a referral should say so, with consent. Cruse asks professionals to discuss this with the local branch [CRUSE].
- **Equity.** The Welsh standards name faith, culture, disability, LGBTQ+ people, Black, Asian and Minority Ethnic communities, and language, including BSL [WALES]. Bereavement services may not reach everyone, and a directory should say what each one offers.

## How a counsellor's notes are held

- **England.** BACP publishes good practice resources on confidentiality and record keeping (GPiA 065, January 2024) [BACP]. I saw only the title and contents.
- **United States.** HIPAA psychotherapy notes are notes that document or analyse a session, including a "group, joint, or family counseling session", **and that are separated from the rest of the medical record**. Medication, session times and similar are excluded. HHS commentary says notes routinely shared do not qualify. The rule is 45 CFR 164.501 [PSYNOTES]. It is a US comparison and does not govern UK counsellors.
- **Who holds them.** A charity, a hospice, an NHS talking therapies service, a private counsellor. Each is its own data controller in UK terms (my reasoning; a data protection officer must confirm). An NHS EPR often holds nothing, and that may be right.

**What this project would need (my reasoning).**
- Default: **a counsellor's notes are not in the shared record.** The shared record holds that a person is under a named service, with dates and a contact, if they agreed. Some people will not want even that.
- If a service uses this EPR, its notes are a separate class, under a label, reachable by that service only. `PSY` exists, but today it unmasks for clinician, nurse, allied health and patient, in any team, so it is **not** a counsellor's confidence. `PSYTHPN` is listed in [protected information](../access-control/protected-information.md) and is not in the code.
- Break-glass must not reach these notes, which matches the rule that it never unmasks a label. Yet a safety fact must be findable. This is the tension in protected-information hazard 3.
- Labels exist in `policy.rs`, but no clinical record carries one, and masking is not built (SE-4). **So no counselling note should be stored until SE-4 exists.**
- The audit row holds ids and versions, as elsewhere, never content. A service name in a patient-visible access log can disclose counselling ([protected information](../access-control/protected-information.md)).
- Immediate online release of a counsellor's formulation to a portal may hurt, and a partner may read it. The route needs a decision.
- Retention of counselling notes is a controller's decision; nothing is set here.

## Confidentiality between family members

Two people who share a loss may share a counsellor, or a GP, or a joint session. I found **no** source on how a joint record should be kept. The BACP resource may cover it, and I could not read it.

Sources do confirm the underlying risk: the trauma of suicide can cause tension and conflict within families [SAS2016]. HIPAA's definition expressly contemplates joint and family sessions [PSYNOTES].

**What this project would need (my reasoning, to be tested with counsellors).**
- **One record per person, always.** No family record. A family or couple is not a patient.
- **A joint session is recorded so that nothing said by one is released to the other.** The options, none perfect: a note per person with only that person's words; or a note held by the service alone and not in anyone's record. Each person's access copy must be reviewed.
- **Access is per person.** Seeing one family member gives no right to see another. A care relationship is per patient, which helps. There should be no "household" or "family view" in the product.
- **A counsellor's conflict is visible only to the counsellor's service.** If two relatives see one counsellor, the system does not tell other staff.
- **Family link data is itself sensitive.** It shows who was related to whom, and maybe who knew. Linked by consent only, and it may be better to write it as free text in the person's own words.
- **Proxy.** A parent-proxy must not read a joint-session note about the other parent or a sibling.
- **Disclosure of family conflict.** Abuse may emerge, or a dispute over a will. A note is not safe from a court. A counsellor should explain this, and it is outside a system's scope.

## Third-party information in a survivor's record

A survivor's record will be full of the person who died, and of others.

- **Living third parties.** The ICO says that if health information holds someone else's personal information, such as a family member's, the third-party rules must be considered before disclosure. Withholding on serious-harm grounds is limited: a health professional applies it, or gives an opinion within six months [ICOHEALTH].
- **The dead.** Data protection covers the living only [ICOHEALTH]. So the data protection third-party rule would not reach a dead person named in a survivor's notes (my reasoning; a lawyer must confirm). But the GMC treats confidentiality as continuing after death [GMCSUM], and what a survivor says may touch on other living relatives. Which duties apply, and who holds them, is unresolved here.
- **Information that came from the dead person's own record.** A clinician who knows both patients could copy a cause, a diagnosis or a history from one record to the other. That is a disclosure from a record which is still confidential. Copy-and-paste is a [manual step](../manual-steps/index.md) with a known risk.
- **Requests for a survivor's record** may be made after their own death, by a relative who is mentioned in it (my reasoning).

**What this project would need.**
- Words in a survivor's record about another person are written as what the survivor said: "reports that...", with `reported_by` ([manual steps](../manual-steps/index.md)). Never a clinical fact about the person who died.
- No copying between records, with a test in the manner of the route census.
- Every export, access copy and portal view of counselling or bereavement content is reviewed by a person. [Data exchange](../data-exchange/index.md) DE-H5 already names third-party content in an export.
- A survivor can't amend what a counsellor recorded about a third party through the patient-amendment route without a decision about whose record it is. The workflow treats it as their own record.

## Outcome measures

- **Wales** names CORE-10 and says it does not suit young children [WALES].
- **CORE-OM** measures wellbeing, problems, function and risk. Petals' reports show falls in severity in 42 patients and a later 107, but had no control group, so therapy effects cannot be separated from natural recovery [PETALS].
- **PG-13-R** was tested across Yale, Utrecht and Oxford (Cronbach's alpha 0.83 to 0.93) and appeared distinct from PTSD, depression and anxiety; a later older sample correlated strongly with a PTSD measure [PG13].
- **A palliative care study** proposes coping and wellbeing as two core outcomes [COPING]. I saw only the title and journal.
- I did not check licences for any instrument, or validity across languages and cultures. Eisma's cultural caution applies [EISMA].

**What this project would need.**
- An outcome is an ordinary observation with an instrument, version, date and who asked; a missing score is `null` with a reason, never zero.
- Under the [support-only tripwire](../../electronic-patient-record-service-with-rust/spec/index.md) (PA-3), a migration naming a column like `score` fails unless it is on an allow-list with a reason. That is the right friction: a score must inform and never rank who gets help.
- **A grief score does not close a pathway**, trigger a diagnosis or end support. A person may stop improving and be doing as well as they can (my reasoning).
- An aggregate is small and identifiable in a small service. See PA-7 ([service spec](../../electronic-patient-record-service-with-rust/spec/index.md)): counts under five are hidden.

## Anniversaries and trigger dates

**The evidence is mixed.**
- Swedish register data: women's suicide odds were 67 percent higher from a parent's death anniversary through two days after (OR 1.67), and men's were lower before it (OR 0.57) [SWEDEN].
- A Swedish cohort aged 12 to 24: some outcomes among young women rose around anniversaries; the numbers were small [SWEDENYOUNG].
- A Danish study of suicide-bereaved relatives and partners found **no** evidence of raised risk around death anniversaries or birthdays [DENMARK].
- The NBCP says to "discuss and acknowledge" stages or dates that may be particularly difficult in a later pregnancy [NBCPMC].

**Dates can also harm.** A reminder falling on the date of a baby's death, or a recall naming the dead child, or a mailing sent "one year on". A date of death does not mean that a person wants to be remembered on that date. I found no study of whether a health service's contact around an anniversary helps or hurts.

**What this project would need (my reasoning).**
- **No automatic contact on a date.** The evidence does not support a rule, and the harm of an unwanted message is plain.
- If a person asks for a contact at a date, that is their plan, on their pathway, and they can cancel it.
- A clinician may see that a date is near. Passive display, to staff only, by consent. Never to a survivor's proxy, never in a letter, never on a banner.
- A due date for a lost pregnancy, a baby's first birthday and similar dates are known to the system. **They must not drive any reminder.** Stopping them is a route test.
- A birth date on a reminder for a surviving twin needs care.

## Consent for proactive contact after a death

This section gathers what the sections above suggest.

- **Sources.** Consent for referral in real-time surveillance [RTS]. The NBCP's "with the woman's consent", and respect for a refusal [NBCPSB]. Research on telling relatives about trial participation: letters could reach only 55 percent of families in one UK study, and patient partners disagreed on timing, with views from four weeks to six months [POSED]. So the right moment differs, and contact details are often missing or wrong.
- **US hospice.** A bereavement plan exists, with contact expected for up to a year [HOSPICE]. It is a regulatory expectation that a person can decline; I did not read that detail.
- **Who is asked.** Anyone close: not only the next of kin. NICE's affected people include friends and colleagues [QS189]. The person who is asked first may not be the right person.
- **Not found.** Any source on a patient's own wish, made in life, about who should or should not be contacted after their death.

**What this project would need.** A consent for contact, on the survivor's record, naming what, by whom, when and how; with a withdrawal; and with a check against the dead patient's wishes and hidden-from list. A single contact attempt per offer unless the person agrees to more. The channel chosen by the person, and no message that mentions the death on a shared phone (my reasoning).

## Compared with this project

| Need | Today | Gap | Verified against the code |
|---|---|---|---|
| Any survivor pathway | Pathways not built | A pack template; the survivor's own pathway | [5.2.1](../05-domain-model.md); [service spec](../../electronic-patient-record-service-with-rust/spec/index.md) |
| A death to be bereaved by | **None**: no death is recorded | See [bereavement](../bereavement/index.md) | Search of service, matcher and UI `src`, migrations and tests: no hits for `death`, `deceased`, `bereave`, `grief` |
| A related person with no access | None; kinds are `care-team`, `active-encounter`, `self`, `proxy` | A non-access relationship, by consent | `src/relationships.rs` |
| Bereavement note on a survivor's record | No clinical note type; the record holds identity, allergies, observations | A dated life event, not a disorder | [service spec](../../electronic-patient-record-service-with-rust/spec/index.md) section 1 |
| Referral to a non-EPR service | Store, routes (`/api/patients/{id}/referrals`, `.../events`), worklist, timeouts and queue exist; the recipient is expected to answer | Signposted and information-given steps; a recipient organisation | `src/routes.rs`, `src/referrals_store.rs`, `src/referral.rs`, `src/referral_timeouts.rs` |
| A `no-response` that means nothing | Timeouts list any unanswered referral for a configured priority | The signposted step, which never times out | `src/referral_timeouts.rs` |
| Label for counselling notes | `R`, `V`, `PSY`, `HIV`, `SUD`, `ETH` only; no `PSYTHPN` | A service-limited class | `src/policy.rs` |
| `PSY` as confidence | Unmasks for `Clinician, Nurse, AlliedHealth, Patient`; no team test | A service test | `src/policy.rs`, `unmasked_for` |
| Records carrying a label | None; masking not applied | SE-4 before any counselling note | [protected information](../access-control/protected-information.md) |
| Consent for contact | Consent provisions cover access, not contact | A contact consent | [service spec](../../electronic-patient-record-service-with-rust/spec/index.md) SE-2 |
| Outbound messages | Outbox built; nothing enqueues; only a `log` sink | A date-driven rule must not exist; a route test | `src/outbox.rs` |
| Scores | Only the matcher's score is allow-listed; a `score` column trips PA-3 | Reason and allow-list if an outcome is stored | `tests/support_only.rs`, [service spec](../../electronic-patient-record-service-with-rust/spec/index.md) |
| Observation model for an outcome | Eight vital signs only, each with one UCUM unit | A questionnaire or outcome model | `src/observations.rs`, service spec RC-6 |
| Counsellor role | Eleven roles; `allied-health` is the nearest | A counsellor or volunteer role, a service limit | `src/policy.rs` |
| Proxy view of a survivor's note | A proxy reads identity, allergies, photo, access log by scope; nothing else | Keep a bereavement note out of every scope | `src/proxy.rs`, service spec PA-5 |
| Family or household view | None | **Keep it that way** | Search of `src/routes.rs` for `family` and `household`: no hits |
| Third-party review of an export | No patient-copy export | A human review step | [data exchange](../data-exchange/index.md) |
| Purpose of use | Seven purposes | Likely fits `treatment`; a voluntary service's use is unclear | `src/policy.rs`, `Purpose` |

**What surprised me.** The project's existing strictness helps here by accident. A proxy reads only named scopes, so anything new is hidden from a proxy by default. A `score` column trips a test. A column that means `withhold` or `suppress` does too, so a contact-suppression field will trip it and need a reason. That is a useful moment for someone to look.

## Candidate hazards

For the Clinical Safety Officer to review. None is accepted or closed, and none is in the [hazard log](../../governance/clinical-safety/hazard-log.csv). The ids are working labels.

- SG-1 **A survivor at risk is missed**, because a bereavement note or a volunteer service was taken for safety cover. A person in crisis needs a clinical route.
- SG-2 **A contact reaches someone who did not agree**, or at a bad moment, or by a phone someone else reads, after a suicide or other sudden death.
- SG-3 **The system infers a person is bereaved by suicide** from a household or next-of-kin link, and acts on it, wrongly.
- SG-4 **A death by suicide is recorded as fact** on a survivor's record before a coroner's conclusion, or a family member learns the cause from a letter, a banner or a shared view.
- SG-5 **A child learns how someone died** from a proxy view, a record or an adult's portal.
- SG-6 **Grief is medicalised by a timer**: a prompt at six or twelve months pushes a diagnosis, with stigma and insurance consequences.
- SG-7 **A diagnosis is copied across criteria**, or a code version is lost, so ICD-11 and DSM-5-TR are confused.
- SG-8 **A signposted person is assumed to be in care.** A GP believes support is in place, and it is not.
- SG-9 **Worklist noise**: endless unanswered referrals to services that do not reply lead staff to dismiss a list holding real referrals.
- SG-10 **A wrong service is listed**: a closed helpline, a wrong area, a number for another nation.
- SG-11 **A counsellor's notes are read by staff or a relative**, because `PSY` is not a service limit, or labels are not applied.
- SG-12 **Something said in a joint session is released to the other person**, by an access copy, a portal or a proxy.
- SG-13 **Counselling confidence is lost to a family dispute**, a court or an insurer, by a route the person was not warned about.
- SG-14 **The dead person's information crosses into a survivor's record**, by a copy from their chart.
- SG-15 **A third party in a survivor's record is disclosed** in a subject access copy, or a named relative is harmed.
- SG-16 **A date-driven message is sent** on a death date, a due date, a child's birthday or a "one year on" mailing.
- SG-17 **A flag stays for ever**: a person is treated as "the bereaved one" for years, or a survivor cannot end it.
- SG-18 **A grief score closes support**, or ranks who gets help. PA-3 applies.
- SG-19 **A tool is used across a culture or language** in which it is not valid, and a person is misjudged.
- SG-20 **A partner, or someone not asked, is left out**: a partner's own loss is not recorded and their care is missed.
- SG-21 **Consent is recorded but the dead patient's hidden-from wishes are not checked**, so an abuser receives an offer of support or a disclosure.

## Ideas

Proposals only.

1. **Three step types in the pathway editor**: information given, signposted, referred, each saying plainly what it will not tell you.
2. **A "life event" entry**, by consent, for bereavement and similar events. It is not a problem-list diagnosis.
3. **A bereavement note as a dated entry with an end**, and a question offered at a time the person chooses: keep or end.
4. **A directory pack** of services, with owner, area, languages, last-checked date and whether it accepts referrals or self-referral only. A stale entry shows as stale. No phone number appears in core code.
5. **A consent for contact**: who, how, when, how often, withdrawn when. Separate from access consent.
6. **A route test for dates**: no outbound message derived from a date of death, a due date or a dead child's birth date.
7. **A service-limited label** for counselling. Prerequisite: SE-4. Prefer holding no notes in the shared record.
8. **A "with whom" field for a joint session**, stored only within the service, and a reviewer step before any disclosure.
9. **A "reported by the person" form of words** for facts about other people, and a warning on a paste from another patient's record.
10. **Do not build** a family view, a household view, a list of people bereaved by suicide, an automatic referral, an anniversary mailing, or a grief risk score.
11. **Offer a parent-held or person-held summary**, so that a person does not have to retell. Printed or on a phone; a printed copy has its own risk.
12. **A questionnaire model** for outcome measures, with version, licence note and language.
13. **Co-design**: a panel with bereaved people, parents, suicide-bereaved people, a counsellor and a voluntary service before any screen. See OQ-PI6 in [open questions](../16-open-questions.md).
14. **Measure carefully**: offers made, offers taken up, referrals that reached a person, and time to first contact. Never rank individuals. PA-7 suppression applies.

## Open questions

1. How does a service support someone with no record in this system, such as a hospice family member? A new `Patient`, a lighter contact, or nothing? A registration for grief seems heavy, and the US rule needs something.
2. Should a bereavement note be seen by reception staff? NBCP's "all staff" points one way, minimum necessary the other.
3. Does an NHS EPR hold any of a charity's counselling record? Who is the controller, and can a volunteer service use this system at all?
4. What does a clinician need to know about a joint session, and how do counsellors really keep those notes?
5. Is one contact attempt the right default, or is "persistent pro-active contact" [POSTV] better? This is a real disagreement between practice and my caution.
6. Who may be offered help after a suicide when the family disagree?
7. How can a dead patient say in life who should not be contacted?
8. Does a survivor's record ever hold a link to the person who died, for clinical reasons such as a shared genetic risk? Who decides? This meets the [bereavement](../bereavement/index.md) rule of no link.
9. Which outcome measures, if any, should a pack carry, under what licence, and for which languages?
10. Is "survivor" the right word in a product? In suicide bereavement it can also mean someone who survived an attempt.
11. Do sources exist in countries beyond those sampled here? I read only English-language material, mainly from high-income countries. Grief is shaped by culture, and this page is not yet equitable in evidence.
12. Who reviews this, and how are people with lived experience paid and protected while doing so?

## Not verified

- **Could not open.** NICE NG105, QS189 and NG142 (403): the wording is from search summaries of NICE pages and secondary reports, and may have been updated; whether NG105 was revised is not confirmed. The eCFR (redirect): the hospice wording is from a CMS quotation in a search summary. I did not see the DSM-5-TR or the WHO ICD-11 text directly.
- **Partial reads.** The Eisma paper, the Welsh framework, the Cruse FAQ, the ICO page and the SASP real-time surveillance page were read as summaries of fetched pages. The PHE 2016 guide and the NBCP PDFs were read as extracted text, by searching for relevant lines, not from start to finish. The guide is from 2016 and describes services that may have changed.
- **Not read.** BACP GPiA 065 beyond its title, so I do not know its advice on joint records. The UK Commission on Bereavement's full report: I saw one finding. The Long Term Plan's outcome: coverage and funding today are unknown. Currier 2008 in full; the subgroup claims are secondary. The Harrop review and the coping and wellbeing paper: titles only.
- **Figures to treat with care.** Bereaved children's figures are charity estimates and differ. Pitman's risk figures depend on study and setting, and I do not quote a percentage. The 70 percent is one service's own figure. The Swedish odds ratios are from one study, and the Danish study points the other way.
- **Searched and not found.** A source on keeping joint records; national consent standards for proactive contact; any evidence on health service contact around anniversaries; GP flags for bereaved children; a patient's wishes on post-death contact; how refusal is recorded.
- **Not researched.** Homicide bereavement, major incidents, bereavement in prison or in the armed forces, bereavement leave and work, financial or legal support, faith-based pastoral care and chaplaincy, grief-focused therapy content, medicines. Northern Ireland and Scotland beyond passing mentions. Low- and middle-income countries.
- **Status may have changed.** SASP's and StandBy's arrangements. Whether Hospice UK's staff line still runs. The Suicide Prevention Strategy's progress. Service names and routes, and Cruse's in particular.
- **The code** was read and searched, not run, on 2026-11-04.
- Nobody bereaved, no counsellor, clinician, or lawyer has seen this page. It deals with people at a very hard time, and it should not be taken as settled until they have. The hazards are my reading, not a hazard analysis.
- Link check 2026-11-04: these cited pages returned 404. They may have moved, but what rests on them is unconfirmed: https://hse.ie/eng/about/who/acute-hospitals-division/woman-infants/bereavement-care/bereavement-symbol-pathway-in-pregnancy-loss-and-pregnancy-after-loss.pdf; https://www.wypartnership.co.uk/news-and-blog/news/dedicated-suicide-bereavement-support-rolled-out-across-england; https://www.sands.org.uk/professionals/projects-improve-bereavement-care/national-bereavement-care-pathway; https://supportaftersuicide.org.uk/funders/nhs-england/

## Sources

All read 2026-11-04. (fetched) means I fetched the page or PDF and read a summary or extracted text of it; (search summary) means a search tool's summary of results; (snippet) means a short extract or title in search results.

- [UKCB] UK Commission on Bereavement, Bereavement is Everyone's Business: https://bereavementcommission.org.uk/?p=24 ; Marie Curie: https://www.mariecurie.org.uk/research-and-policy/policy/uk-bereavement-commission ; progress report 2024: https://www.mariecurie.org.uk/document/ukcb-progress-report-2024 (search summary)
- [TIERS] National Bereavement Alliance, A Guide to Commissioning Bereavement Services in England: https://nationalbereavementalliance.org.uk/wp-content/uploads/2017/07/A-Guide-to-Commissioning-Bereavement-Services-in-England-WEB.pdf ; Care after caring: https://nationalbereavementalliance.org.uk/wp-content/uploads/2020/06/Care-after-Caring.pdf (search summary)
- [WALES] Welsh Government, National framework for the delivery of bereavement care: https://www.gov.wales/national-framework-delivery-bereavement-care-html (fetched)
- [CURRIER] Currier, Neimeyer and Berman, The effectiveness of psychotherapeutic interventions for bereaved persons: https://digitalcommons.memphis.edu/facpubs/8714 (search summary)
- [CRITIQUE] Grief Therapy: Evidence of Efficacy and Emerging Directions: https://https-sage-cnpereading-com-443.webvpn1.xju.edu.cn/doi/10.1111/j.1467-8721.2009.01666.x ; Bonanno and Lilienfeld: https://scottlilienfeld.com/wp-content/uploads/2021/01/2008_BonannoLilienfeld.pdf (search summary; the first is a proxy address, and I matched the claim to the list of results)
- [GROUPS] 2020 review of bereavement groups: https://informahealthcare.com/doi/abs/10.1080/07481187.2020.1772410 (search summary)
- [HARROP] Harrop et al. 2020: https://journals.sagepub.com/doi/10.1177/0269216320920533 (snippet)
- [HOSPICE] 42 CFR 418.64: https://www.ecfr.gov/current/title-42/chapter-IV/subchapter-B/part-418/subpart-C/subject-group-ECFR35b48a647589673/section-418.64 (search summary; fetch redirected); NHPCO tip sheet: https://www.nhpco.org/wp-content/uploads/BereavementCoPTip_Sheet.pdf (search summary)
- [HSESTD] HSE, National Standards for Bereavement Care Following Pregnancy Loss and Perinatal Death: https://starlegacyfoundation.org/wp-content/uploads/National-Standards-for-Bereavement-Care-Following-Pregnancy-Loss-and-Perinatal-Death.pdf (search summary)
- [HSESYM] HSE, bereavement symbol pathway: https://hse.ie/eng/about/who/acute-hospitals-division/woman-infants/bereavement-care/bereavement-symbol-pathway-in-pregnancy-loss-and-pregnancy-after-loss.pdf ; https://pregnancyandinfantloss.ie/wp-content/uploads/2025/04/BEREAVEMENT-SYMBOL-PATHWAY-NOV-2024.pdf (search summary)
- [CUMH] University College Cork, pregnancy after loss: https://www.ucc.ie/en/pregnancyloss/aboutus/whatwedo/pregnancyafterloss/ (search summary)
- [NBCPW] Wales NBCP: https://performanceandimprovement.nhs.wales/nbcp/ (snippet)
- [NBCPNI] Northern Ireland: https://www.publichealth.hscni.net/news/improving-bereavement-care-people-northern-ireland-affected-pregnancy-or-baby-loss (snippet)
- [ICD] ICD-11 6B42 as quoted: https://www.findacode.com/icd-11/code-1183832314.html ; https://www.cismef.org/page/detail/en/ICD11_CA_1183832314 (search summary)
- [DURATION] Prolonged Grief Disorder: Course, Diagnosis, Assessment, and Treatment: https://psychiatryonline.org/doi/full/10.1176/appi.focus.20200052 ; DSM-5-TR criteria page: https://simpleandpractical.com/prolonged-grief-disorder-diagnostic-criteria/ (search summary)
- [PGDPREV] Prolonged grief disorder in ICD-11 and DSM-5-TR: differences in prevalence and diagnostic criteria: https://www.ncbi.nlm.nih.gov/pmc/articles/PMC10881750/ (search summary)
- [EISMA] Eisma, Prolonged grief disorder in ICD-11 and DSM-5-TR: challenges and controversies, 2023: https://pmc.ncbi.nlm.nih.gov/articles/PMC10291380/ (fetched)
- [REDICAN] Redican et al., Are time and culture useful and necessary diagnostic requirements for ICD-11 prolonged grief disorder?: https://www.napier.ac.uk/research-and-innovation/research-search/outputs/are-time-and-culture-useful-and-necessary-diagnostic-requirements-for-icd-11-prolonged (search summary)
- [NG105] NICE NG105: https://www.nice.org.uk/guidance/NG105 (search summary)
- [QS189] NICE QS189, statement 5: https://www.nice.org.uk/guidance/qs189/chapter/Quality-statement-5-Supporting-people-bereaved-or-affected-by-a-suspected-suicide (search summary; fetch returned 403)
- [STRATEGY] DHSC, Suicide prevention in England: 5-year cross-sector strategy: https://www.gov.uk/government/publications/suicide-prevention-strategy-for-england-2023-to-2028 (search summary)
- [ROLLOUT] West Yorkshire Health and Care Partnership: https://www.wypartnership.co.uk/news-and-blog/news/dedicated-suicide-bereavement-support-rolled-out-across-england ; SASP, NHS England: https://supportaftersuicide.org.uk/funders/nhs-england/ (search summary)
- [SASP] Support After Suicide Partnership hub: https://hub.supportaftersuicide.org.uk/ (search summary)
- [HIAH] Help is at Hand: https://supportaftersuicide.org.uk/resource/help-is-at-hand/ (search summary)
- [HIOW] NHS Hampshire and Isle of Wight, Suicide Aware Practice Framework: https://www.hantsiow.icb.nhs.uk/suicide-aware/suicide-aware-practice-framework (search summary)
- [NHSEMP] NHS Employers, Suicide prevention and postvention: https://www.nhsemployers.org/articles/suicide-prevention-and-postvention (search summary)
- [PITMAN] Pitman et al., bereavement by suicide as a risk factor: https://discovery-pp.ucl.ac.uk/id/eprint/1476423/1/Bereavement%20by%20suicide%20as%20a%20risk%20factor%20for%20suicide%20attempt%3A%20a%20cross-sectional%20national%20UK-wide%20study%20of%203432%20young%20bereaved%20adults.pdf ; review: https://www.ncbi.nlm.nih.gov/pmc/articles/PMC10138333/ (search summary)
- [SAS2016] Public Health England and NSPA, Support after a suicide: a guide to providing local services, October 2016: https://assets.publishing.service.gov.uk/media/5a7f3db6ed915d74e6229407/support_after_a_suicide.pdf (fetched, PDF text)
- [ONSSUI] ONS FOI: https://www.ons.gov.uk/aboutus/transparencyandgovernance/freedomofinformationfoi/thenumberofsuicidesintheukin2020andthenumberofsuicidesawaitingacoronersinquest (search summary)
- [USGUIDE] National Action Alliance, U.S. National Guidelines: https://theactionalliance.org/resource/responding-grief-trauma-and-distress-after-suicide-us-national-guidelines ; SPRC: https://sprc.org/online-library/responding-grief-trauma-and-distress-after-suicide-us-national-guidelines/ (search summary)
- [STANDBY] JSS: https://jss.org.au/programs/standby-support-after-suicide/ ; Life in Mind: https://lifeinmind.org.au/news/standby-support-after-suicide ; ACT submission: https://www.treasury.act.gov.au/__data/assets/pdf_file/0011/3020888/67.-Youturn-Limited-StandBy-Support-After-Suicide.pdf (search summary)
- [RTS] SASP hub, Real Time Surveillance: https://hub.supportaftersuicide.org.uk/content/real-time-surveillance/ (fetched)
- [THRIVE] Thrive LDN, Suicide Prevention: https://thriveldn.co.uk/about/our-activities/suicide-prevention/ (search summary)
- [SELMIND] South East London Suicide Bereavement Service: https://selmind.org.uk/our-services-south-east-london-suicide-bereavement-support/ (search summary)
- [POSTV] Understanding impact and factors that improve postvention service delivery: https://www.tandfonline.com/doi/full/10.1080/13576275.2024.2417299 (search summary)
- [NBCPALL] Sands, National Bereavement Care Pathway: https://www.sands.org.uk/professionals/projects-improve-bereavement-care/national-bereavement-care-pathway (search summary)
- [COMMONS] House of Commons Library, Baby Loss: https://commonslibrary.parliament.uk/research-briefings/cdp-2025-0188/ (search summary)
- [NBCPSB] NBCP, Stillbirth, England, July 2022: https://www.nbcpathway.org.uk/wp-content/uploads/2024/03/NBCP-Stillbirth-July-2022.pdf (fetched, PDF text)
- [NBCPMC] NBCP, Miscarriage, ectopic and molar pregnancy: https://www.nbcpathway.org.uk/wp-content/uploads/2024/03/NBCP-Miscarriage-short-guidance-July-2022.pdf (fetched, PDF text)
- [PETALS] Petals evaluation: https://hrcak.srce.hr/264600 ; https://hrcak.srce.hr/265721 (search summary; I did not confirm which link gives which report)
- [CBN] NCB: https://www.ncb.org.uk/about-us/media-centre/news-opinion/childrens-grief-awareness-week-40000-children-bereaved-parent ; Childhood Bereavement Network evidence: https://committees.parliament.uk/writtenevidence/76937/html/ (search summary)
- [WW] Winston's Wish, facts and figures: https://winstonswish.org/about-us/facts-and-figures (search summary)
- [NSPACYP] NSPA, Supporting Children and Young People Bereaved by Suicide: https://nspa.org.uk/wp-content/uploads/2021/04/Supporting-Children-and-Young-People-Bereaved-by-Suicide.pdf (title only)
- [CRUSE] Cruse, Referring people to Cruse services, FAQs: https://www.cruse.org.uk/?p=1131 (fetched)
- [CRUSEWALES] Dewis Wales listing: https://www.dewis.wales/ResourceDirectory/ViewResource.aspx?id=21942 (search summary)
- [CRUSENI] Family Support NI listing: https://www.familysupportni.gov.uk/Service/3007 (search summary)
- [BACP] BACP, GPiA 065, January 2024: https://www.bacp.co.uk/media/20401/bacp-confidentiality-and-record-keeping-crp-gpia065-jan24.pdf (snippet)
- [PSYNOTES] 45 CFR 164.501 as summarised: https://hollandhart.com/hipaa-psychotherapy-notes-and-other-mental-health-records ; https://www.bricker.com/insights/resources/key/hipaa-privacy-regulations-definitions-psychotherapy-notes-164-501 (search summary)
- [ICOHEALTH] ICO, right of access, health data: https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/individual-rights/right-of-access/health-data (fetched)
- [GMCSUM] MDU, Confidentiality and disclosing information after death: https://themdu.com/guidance-and-advice/guides/disclosure-after-death ; MDDUS: https://mddus.com/resources/resource-library/risk-alerts/2020/september/patient-confidentiality-after-death (search summary)
- [PG13] Validation of the new DSM-5-TR criteria and the PG-13-R: https://psychotraumanet.org/en/node/6402 ; Validation in older adults: https://www.ncbi.nlm.nih.gov/pmc/articles/PMC12761280/ (search summary)
- [COPING] Coping and wellbeing in bereavement: https://bmcpalliatcare.biomedcentral.com/articles/10.1186/s12904-020-0532-4 (title only)
- [SWEDEN] Suicide around the anniversary of a parent's death in Sweden: https://www.suicideinfo.ca/wp-content/uploads/2023/04/Suicide-Around-the-Anniversary-of-a-Parents-Death-in-Sweden.pdf ; Stockholm University: https://su.se/department-of-public-health-sciences/news/women-s-suicide-risk-increases-around-the-anniversary-of-a-parent-s-death-1.654660 (search summary)
- [SWEDENYOUNG] Swedish cohort aged 12 to 24: https://www.x-mol.com/paper/1552801308256399360 (search summary)
- [DENMARK] Epidemiology and Psychiatric Sciences, January 2023: https://doaj.org/article/61a7f30a5d3a4edda8c9f716c8c0d34c (search summary)
- [POSED] Informing relatives of non-surviving patients about research participation: https://pmc.ncbi.nlm.nih.gov/articles/PMC12605857/ (search summary)
- This repository: `electronic-patient-record-service-with-rust/src/` (`policy.rs`, `proxy.rs`, `relationships.rs`, `outbox.rs`, `referral.rs`, `referrals_store.rs`, `referral_timeouts.rs`, `observations.rs`), `tests/support_only.rs`.
