# Suicide watch: caring for someone at risk of suicide

Public-source research read 2026-11-04 plus this repository's own files. A proposal, not clinical or legal advice: nothing here is built, and no clinician or person with lived experience has reviewed it. **NOT FOR CLINICAL USE.**

## Related links

[Protected information](../access-control/protected-information.md), [Roles and assignments](../access-control/roles-and-assignments.md), [Access control and consent](../access-control/index.md), [Audit](../audit/index.md), [Care settings](../care-settings.md), [Domain model 5.2.1](../05-domain-model.md), [Pathway design](../pathway-design.md), [Closed-loop referral](../interoperability/closed-loop-referral.md), [Manual steps](../manual-steps/index.md), [Emergency triage](../triage/index.md), [Virtual ward](../virtual-ward/index.md), [Care homes](../care-homes/index.md), [Trauma treatment](../trauma-treatment/index.md), the [hazard log](../../governance/clinical-safety/hazard-log.csv).

Method: web searches and fetches, primary sources where they could be opened, and a reading of the code. England is the worked example because the project is UK-first; other countries appear as examples, never as the rule ([equity rule](../02-scope.md)). Law belongs in a country pack. Statements about practice say what a source says. They are not instructions. This page gives no detail of method, means or place, and uses "environmental safety" and "means restriction" only in general. It is about software. Anyone struggling should contact their local crisis service; in the United States the number is 988 [988].

## Why this page exists

1. **The record is part of the safety net.** When someone is observed closely, the record says who holds that person now, what was seen and what is still owed. Coroners have reported deaths where the chart looked complete and was not.
2. **The most obvious design is harmful.** A "suicide risk: high" field is what an EHR would naturally add. English guidance says not to do this [NG225] [SSS]. A flag or score can also stigmatise, leak and stay for ever.
3. **Append-only helps, but less than it seems.** It shows a late entry as late. It cannot show that a first entry was false.

## Words

"Suicide watch" is the name of this directory because the request used it. It is not the word I found in UK clinical sources, and it describes the thing people are moving away from.

| Place | Terms found | Source |
|---|---|---|
| England | Enhanced therapeutic observation and care (ETOC); locally enhanced care, 1:1s, cohorting, specialing | [ETOC-POL] |
| England, acute hospitals | Continuous, enhanced, specialling, constant, close, one-to-one, two-to-one, used interchangeably | [HSSIB24] |
| Scotland | "From observation to intervention": aims to end enhanced observation in its old form | [HIS] |
| New South Wales | Therapeutic engagement and observation; "consumer" for patient | [NSW] |
| United States (hospitals) | Suicide precautions, constant observation, sitter | [USPOL] (local policies, examples only) |
| England and Wales (prisons) | ACCT: Assessment, Care in Custody and Teamwork | [PPO] |

My reading, with no source that says so outright: "suicide watch" is a US and custodial word. The sources do show the shift in meaning. England's national health body defines therapeutic enhanced observation as moving "away from passively observing and watching patients to actively supporting and engaging with them" [ETOC-TP]. One patient quoted there says: "You cannot make me safe; you can only care for me in the way that I need, and then I will feel safe." The screens should not say "watch".

Enhanced observation is also not only about suicide. England's national health body's data guide lists falls, self-harm, dementia, delirium, absconding, violence and property damage among the reasons [ETOC-DATA]. So an observation order need not tell everyone why it exists (my reasoning, and the most useful single consequence of this page).

**What this project would need.** A setting-neutral concept, "observation order", with local display terms from a pack. No word "watch" and no reason shown by default on a ward list.

## Levels and policy

- **Levels (England).** England's national health body's policy guide adopts four NICE levels for psychiatric wards: low-level intermittent (every 30 to 60 minutes), high-level intermittent (every 15 to 30 minutes), continuous (one-to-one, within eyesight or arm's length), and multi-professional continuous [ETOC-POL]. I saw these through that guide; NICE NG10 itself was not read.
- **No national interval or standard is uniform.** A coroner noted in 2025 that there is no national standard, so each trust sets its own check interval, and that confusion remained on whether a 15-minute check may fall in a 15 to 30 minute window [PFD-TAB]. So a pack carries intervals, and a pack must also say how much tolerance a check has. The core cannot.
- **Review.** Reassessment at least every 24 hours, ideally each shift, with multidisciplinary oversight of any case over a week [ETOC-POL] [ETOC-TP]. Step-down is planned "right from the start" [ETOC-TP]. Reduction in privacy must be justified, time-limited, recorded in the care plan and reviewed [ETOC-POL].
- **Least restrictive.** Use needs individual justification [ETOC-TP]; "must never be applied in isolation or as a default intervention" [ETOC-POL]. Culture of care standards say one-to-one time can build a relationship, and that relationships come before surveillance technology [CULT].
- **Evidence is thin.** HSSIB found "a lack of evidence about how to optimise the safety and quality of continuous observations of adults" and recommended research and national guidance, including a competency framework [HSSIB24]. Whether that guidance now fully answers its recommendation I did not establish.
- **Other jurisdictions.** New South Wales issued a mandatory directive on 20 April 2026, replacing PD2017_025 [NSW]. Scotland's framework dates from January 2019 [HIS].
- **Adjacent.** ETOC data shows the demand side: hours filled and unfilled, by substantive, bank and agency staff, and incidents during ETOC. Leeds found incident reporting during ETOC "low" and not reflecting actual incidents [ETOC-DATA].

**What this project would need.** An order with level, start, who authorised it, why (restricted), what privacy limits apply, review due, step-down plan, end, and who ended it. Each change is a new version. The level names and intervals come from a pack. Unfilled hours must be visible: a shift with no observer is a fact, not a blank.

## Who holds the observation

- **Who does it.** Mixed groups, often healthcare support workers and temporary staff [HSSIB24]. In England unregistered staff deliver ETOC and cannot take on care planning; registered staff supervise them; the nurse in charge allocates staff and oversees documentation and debriefs [ETOC-POL].
- **Length and breaks.** HSSIB cites national guidance of no more than 2 hours for one observer, and found about 12-hour shifts in its reference case, with a single 1-hour break and staff fatigue [HSSIB24]. England's national health body's 2025 guide sets no maximum and asks for "regular breaks" [ETOC-POL]. NSW requires rotation to manage "constant observation fatigue" and breaks [NSW].
- **Handover.** Structured handover is asked for by [ETOC-POL] and [NSW]. HSSIB's prompts include a clear handover, shared roles written in care plans, and access to records for observing staff [HSSIB24].
- **Two teams.** In HSSIB's case, mental health and acute staff worked largely apart and each assumed the other had made a safety check [HSSIB24]. That is the record's problem exactly: a task with no single owner.
- **Name the holder.** A New South Wales coroner recommended that the nurse responsible for observation be clearly identified in the health record [NSWCOR] (search summary).
- **Staff are affected.** HSSIB says witnessing a serious event can be traumatic, and staff cannot always get support [HSSIB24].

**What this project would need.** A holder ledger: one row each time someone takes over, with the person, the start, who handed over and what was said. At any moment, exactly one person (or a named set, for two-to-one) holds the order. A break is a handover. Time with no holder is shown as a gap. The screen a person sees should be the observer's own short view of this patient, including what helps and any part of the plan they need ([ETOC-TP], "supportive enquiry").

## Falsified and back-filled records

This is documented, repeated and not rare enough to treat as an edge case.

- **Coroners, England.** "observation records were backfilled despite the observations not having been conducted"; spot checks looked "only" at quality and timing; and the senior nurses auditing had entries "of the same poor quality" (2024) [PFD-BACK]. Elsewhere: "records were falsified to show the observations had been undertaken", and "the current audit of the observation logs will not highlight the falsifying of checks" (2022) [PFD-AUD]. A 2026 report also records falsified entries in a chart that was fully completed [PFD-2026] (snippet).
- **Electronic does not solve it.** A coroner in April 2026 reported that staff stopped completing a comments field because the electronic device lost Wi-Fi, "making it impossible to record the observations until they were back in the ward office" [PFD-WIFI].
- **Quality, not only truth.** One chart's entries "simply recorded" the patient's location [PFD-BACK]. NSW asks for mental state and activity as well as location [NSW].
- **East London.** Commentary by a retired psychiatrist reports twelve staff referred to the Nursing and Midwifery Council and a 2024 human factors review that found improvement measures "did not fully address" missed and falsified observations [ALEX]. This rests on her account. A council record says the trust told it no deaths on its wards had been linked to erroneous recording, and that falsification goes to disciplinary process [HACK] (snippet).
- **Essex.** The Lampard Inquiry covers mental health inpatient deaths in Essex from 2000 to 2023. It has **not** reported. The final report is expected in mid-2028 [LAMPT]. Reported evidence includes a witness from the charity Inquest citing a "high prevalence of falsified observation records" [LAMPE]: this is evidence, not a finding.
- **Prisons.** Prisons and Probation Ombudsman reports repeatedly describe missed checks logged as done, and a 2025 report is said to identify systemic falsification around ACCT [PPO] (snippets and press; report not read). Same shape, another setting.
- **Why it may happen.** HSSIB describes a "culture of blame and fear" and that "'Doing' tasks, like 'ticking' checklists, overshadow meaningful, empathetic 'being' interactions" [HSSIB25]. Fatigue and staffing are in [HSSIB24]. I found no study that names causes of falsification itself; the human factors review was not read.
- **What policy asks.** NSW: entries at the time or as soon as possible, and "Clearly mark retrospective entries with the actual time of documentation and reason for delay"; daily random reviews of accuracy [NSW]. England's national health body's local policy guide does not address falsification [ETOC-POL].

### What an append-only record can and cannot do

My reasoning, checked against the code.

| It can | It cannot |
|---|---|
| Keep every first entry, and show any later change as a new version | Prove the observer looked |
| Hold the true write time beside the claimed time: `recorded_at` is set by the database and `effective_at` by the writer (`observation_versions`) | Stop a false entry made on time |
| Show a late entry as late, for ever | Stop a person writing a note that is plausible and untrue |
| Show a gap, if a slot exists for each due check | Show a gap if there is no expected schedule: today nothing says a check was due |
| Let a reviewer read what the chart said before an event | Replace a culture in which people can say a check was missed |

**The honest entry has to be easy.** "Not done, and why" must be as fast as "done", and must not lead to blame by default. Otherwise a design that exposes gaps creates a reason to fill them falsely. Audit that compares the log with an independent signal is what coroners asked about ([PFD-AUD] names CCTV), and that signal is another question for privacy (below).

**What this project would need.** Expected-check slots from the order. Each slot closes as seen, or not seen with a reason, by the holder. A late entry is labelled late on every screen, with its reason. The chart shows gaps and never interpolates. Offline devices keep both the device time and the time received, and show both (a queued entry is not a false one; the April 2026 report is the case for this). Review tools flag patterns for a person to look at, such as a burst of entries or one person holding two patients. They do not accuse. The review must itself be sampled from outside the ward (the auditors in [PFD-BACK] had the same problem).

## Electronic and contactless monitoring

- **Oxevision** is a camera and sensor system for bedrooms, also called LIO in reports. A BBC-syndicated report says half of England's mental health trusts have used it; a campaign puts it lower [OXE-USE]. Treat both figures as unsettled.
- **Evidence.** A systematic review from the NIHR mental health policy research unit found insufficient evidence that surveillance technologies in inpatient mental health settings achieve their intended outcomes [GRIF].
- **Principles (February 2025).** England's national health body: decisions must rest on consent or a best-interests process; never a "blanket approach"; technology should not be a solution to low staffing or reduce time with patients; surveillance "is a form of restriction that may breach a person's human rights" [DIGP]. Care plans should record the technology, the reason and how data is used [DIGP].
- **Consent dispute.** Rethink asks for ongoing, specific, withdrawable consent and in February 2025 called for a pause [RETH]. A summary by the Stop Oxevision campaign of Lampard hearings in July 2026 reports witnesses accepting that the system had been relied on instead of in-person checks, contrary to trust policy; an expert calling that unsafe; and opt-out models at some trusts [STOP]. This is a campaign's account of evidence, not a finding.
- **Inquiry.** Further hearings ran in October 2026, with submissions on interim recommendations set for 20 October [BIND]. I found no decision. A report that England's national health body referred ten trusts to the CQC over consent rests on a weak secondary source [OXE-REF].
- **HSSIB** did not assess the technology. It says implementation must support therapeutic engagement and be subject to ward-level audit, and that patients' views were mixed: some found it distressing, some felt safer [HSSIB25]. Night observation can be "one of the more severe forms of intrusion" [HSSIB25].

**What this project would need.** It should not integrate any camera feed. If a deployment does record contactless monitoring, it is a separate kind of data with its own consent record. A device event never closes an observation slot. I found nothing on electronic observation apps from vendors, on proximity tags, or on what these systems keep; that gap is in Open questions. Any review tool that uses CCTV or a device is itself surveillance, of staff and patients, and needs the same test [DIGP].

## Assessment, and the danger of a score

- **NICE NG225 (2022).** "Do not use risk assessment tools and scales to predict future suicide or repetition of self-harm" (1.6.1), nor to decide who is offered treatment or discharged (1.6.2). The same two rules apply to "global risk stratification into low, medium or high risk" (1.6.3, 1.6.4). Focus instead on the person's needs and safety (1.6.5). Mental health professionals undertake a risk formulation in every psychosocial assessment (1.6.6). The committee said stratification implies that risk is static [NG225].
- **England's national health body (2025).** *Staying safe from suicide* calls static stratification "unacceptable", says tools "don't work", and says practitioners should explore risk collaboratively. It tells organisations to "update record systems to eliminate risk stratification", to allow narrative input for assessment, formulation and safety plans, and to give access to past records so people need not repeat themselves [SSS].
- **Findings.** NCISH reports that at final contact, immediate risk was judged "not present or low" for 81% of patients who died, a share unchanged over the report period [NCISH26].
- **EPRs lag.** HSSIB's 2025 thematic review says some EPRs still required staff to categorise patient risk, against the direction of guidance, and that EPRs "did not facilitate good person-centred safety assessment" [HSSIBEPR].
- **United States: a different frame.** Joint Commission NPSG 15.01.01 requires screening with a validated tool for people treated primarily for behavioural health conditions, and its materials name screening from age 12 [JC]. Tools named include the C-SSRS and the ASQ [JC]. The ASQ is four questions, developed in paediatric emergency departments, with a toolkit for ages 8 and over and a brief safety assessment after a positive screen [ASQ]. AHRQ's PSNet says these screeners have been validated in emergency departments [PSNET]. Zero Suicide names seven elements: Lead, Train, Identify, Engage, Treat, Transition and Improve [ZS]. A hospital council summary says the standard expects overall risk and a mitigation plan to be documented [JC].

So the jurisdictions differ. A US deployment may need a screen and a recorded level, where an English one should not have one. Screening to open a conversation and predicting are different acts, but a number in a record is read the same way (my reasoning).

**What this project would need.** No risk level or score in the core. Narrative formulation, dated and authored, as the main content. A screen result, if a pack holds one, is the answers and the instrument version, shown with its date. It does not compute a category, drive discharge, or show in a banner. "Not assessed" stays `null` with a reason ([triage](../triage/index.md) already separates suggestion from confirmation and never reads `not-assessed` as `absent`). A pack that wants a level must say so and carries a review status ([care settings](../care-settings.md)). Under [pathway design](../pathway-design.md) rule 7, no template decides.

## Safety plans

- **Stanley and Brown.** Six steps, as quoted by England's national health body: warning signs; coping strategies; distraction through connection; support from others; professional support; environment safety [SSS]. A US cohort study at nine VA emergency departments (1,640 patients, not randomised) found less suicidal behaviour over 6 months with safety planning plus telephone follow-up: 3.03% against 5.29% [STANLEY].
- **NICE.** Consider a safety plan made in partnership. It should be developed with family and carers as appropriate, "held by the person", shared "as decided by the person", and accessible to the person and professionals "at times of crisis" (1.11.7, 1.11.8) [NG225]. The care plan is also given to the person (1.5.16).
- **England's national health body.** Written down, reviewed, shared as appropriate; includes what would be unhelpful from professionals; integrated into care [SSS]. Digital principles treat safety planning as personal and co-produced [DIGP].
- **Inpatient use.** A quality improvement programme in ten London wards that included personal safety plans reported falls of 33 to 75 percent in enhanced observation days [ETOC-TP]. It was a local programme with several changes at once.

**What this project would need.** A safety plan that belongs to the person. It has versions and an author line that includes the person. It is printable, and available without a login to the person, in their words and language. The sharing list is the person's decision, recipient by recipient, and is honoured by every route. An emergency department seeing the person at night must be able to find it. That conflicts with a label that hides it (see [protected information](../access-control/protected-information.md), hazard 3). The contacts in a plan are third-party data. The environmental-safety step is the part that needs the most care in shared views.

## After discharge

- **NCISH (Annual report 2026, UK and Jersey, 2013 to 2023).** 2,250 patients died within 3 months of inpatient discharge, 13% of patient suicides, about 205 a year; the rate rose in 2021 to 2023, to 16.0 per 10,000 discharges in 2023. Deaths peaked on days 3 and 4; in 2020 to 2023 the peaks moved to days 4 and 6, which "raises a concern that early post-discharge suicides have been postponed rather than prevented". 190 (10%) died before first follow-up. 311 (23%) had been discharged to housing, financial or employment problems and 255 (19%) to poor social support [NCISH26].
- **72 hours (England).** In the standard health service contract for England since 1 April 2020, with an 80 percent ambition; measured through the Mental Health Services Data Set [72H]. Reported as met for 73 to 76 percent in sources dated 2022 and 2023 [72H]. In NCISH real-time data, 16 of 21 who died had been seen within 3 days [NCISH26]: a very small sample, and a reminder that the timer is not the goal.
- **After self-harm.** Where there are safety concerns, initial aftercare within 48 hours of the psychosocial assessment (NG225 1.10.2) [NG225]. An ombudsman has noted that 72-hour follow-up covers inpatient discharges and not emergency department discharges [72H].
- **US.** Zero Suicide's Transition element and Veterans Health Administration contacts (below).

**What this project would need.** Discharge closes a ward stay and never the pathway ([5.2.1](../05-domain-model.md)). Follow-up is a step with a named owner, a timer from a pack and a loop-back to a person, as in [closed-loop referral](../interoperability/closed-loop-referral.md). It must survive transfer to another organisation. "Contact made" should record what happened, not close on a tick, because a target met is not the same as a person helped. Social problems at discharge should be visible to the follow-up team.

## Emergency departments and general hospitals

- NICE: joint governance so that physical and mental healthcare can be delivered together, including "access to electronic record systems for both mental health services and medical treatment at the point of care" (1.7.17) [NG225]. Staff should have specialist and legal advice on capacity and consent (1.2.1).
- HSSIB's report is about this case. It found no process for admitting people at risk of self-harm to acute wards, an environment that hindered observation, and a lack of shared access to electronic records between providers [HSSIB24].
- England's national health body's policy guide says it applies to all clinical settings, with role training for emergency departments [ETOC-POL].
- One local paediatric policy sets the observation level from a brief triage in the emergency department [CYP] (snippet).

**What this project would need.** A second organisation's observer may need access to one patient for one shift. The acute team needs mental-health content that a label may hide. [Care settings](../care-settings.md) rule 4 makes this a disclosure decision, never a default. An observation order should work the same on a medical ward. It should use the [triage](../triage/index.md) signals only as prompts.

## Children and young people

- HSSIB's report excludes children [HSSIB24]. A systematic review found no standardised, evidence-based protocols for self-harm in acute paediatric care [CYP] (snippet). I found no RCPCH guidance.
- England's national health body treats family or carer involvement as key for young people, barring safeguarding concerns [SSS]. The consensus statement covers adults, and says the position differs for children [CONS].
- Competence, confidentiality and proxy rules are in [protected information](../access-control/protected-information.md). NCISH reports were not read for under-18s.

**What this project would need.** Age-aware proxy rules, a safeguarding route, and an answer to whether a young person's safety plan reaches a parent. No answer is proposed here without young people and a safeguarding lead.

## Mental Health Act and mental capacity

- **England and Wales.** The Mental Health Act 2025 received Royal Assent on 18 December 2025. Most of it is not yet in force, sources disagree on how much is, and full implementation is estimated at about ten years. It replaces the nearest relative with a nominated person [MHA25]. So a pack must carry both regimes for years (my inference).
- **Capacity.** The Mental Capacity Act assumes capacity. An unwise decision does not by itself show a lack of it. Doubts may be greater at imminent risk [CONS]. RCEM's 2017 guideline covers how the Act, deprivation of liberty and the Mental Health Act meet in the emergency department [RCEM]. The MCA does not authorise deprivation of liberty as such, though a short temporary one may be allowed in some circumstances [MCA]. One local ETOC approvals process has the safeguarding team review patients with deprivation of liberty applications [ETOC-TP].
- **Refusal after an act of self-harm.** A 2019 study reports difficulty with advance decisions in this setting [ADRT]. I do not summarise the law here.
- **Other countries.** Scotland, Northern Ireland, the US states and Australia have their own laws. None was read.

**What this project would need.** Legal status as a dated record with an authority, not a flag (task XP-7). An observation order that restricts liberty or privacy names its legal basis. A capacity entry is decision-specific and dated, so it can never be copied across decisions.

## Families

- **England.** The consensus statement (2021, adults) says: where the common-law duty applies, a refusal is usually respected if the person has capacity and is a risk only to themselves; disclosure may be justified in the public interest, as a matter of judgement; data protection law does not stop sharing in an emergency; share what is necessary and proportionate; if capacity is absent, best interests apply, and disclose the minimum. It tells staff to ask routinely whether and how a person wants family and friends involved, ideally who should and should not be, and to record "their decision about sharing information on each occasion". Confidentiality does not stop staff listening [CONS]. NG225 agrees on listening (1.4.1) [NG225]. A SHARE guide supports it, with a Northern Ireland version [SHARE].
- **Telling a family.** England's national health body says practitioners may breach confidentiality to inform others if necessary, guided by legal and ethical frameworks [SSS]. The harder case is a family member who is the danger (see [protected information](../access-control/protected-information.md)).

**What this project would need.** A person-level statement of who to involve and who not, asked early and revisited. A sharing decision log: what, with whom, on what basis, by whom, each time. A contact the person says must not be told must be protected as strictly as the risk itself. The record must let staff record what a family told them, with its source, without disclosing back.

## After a death in care

- **Learning response (England).** PSIRF is the current framework. Trusts say it does not change the duty of candour. Trusts report a locally led investigation for deaths of detained patients where problems in care may be linked [PSIRF]. Those affected, including families, are involved, may bring an advocate, and agree timeframes [PSIRF]. A coroner's report of February 2026 is described as criticising one trust's process because an investigator did not seek treating staff's recollections; this is second-hand [ALEX].
- **Duty of candour.** Regulation 20: a notifiable incident needs a face-to-face account, an apology, a written follow-up and records, including attempts to reach someone [CAND].
- **Coroners.** Prevention of future deaths reports (above) are public and quote records.
- **Records.** Mental health records in England are reported as retained for 20 years, or 10 years after death [RET]; the older schedule said 8. Detained patients' records may be kept longer [RET]. I found no rule on inquiry or coroner holds for England, and no moratorium source for Lampard. A 2023 prison report says records were altered after a death [PPO] (press).
- **NSW and US.** Not researched.

**What this project would need.** A legal hold on a person's record and its audit rows. Today `audit_retention.rs` can move audit rows to an archive after a set number of months; they stay readable through `audit_events_all`, and nothing marks them as held (hazard H-042). A mark on anything written after a known death or serious event, so it reads as afterwards. A frozen export for the investigator and coroner, with what was shown to whom and when. Family access under a stated rule, and a bereaved person's request logged. A way to ask staff for their account that is outside the clinical record. The record should support learning; it should not become the only witness.

## The system's own risks

- **The flag that stigmatises.** Staying safe says to avoid stigmatising terms such as "attention-seeking" or "manipulative", avoid risk labels, and prefer the person's own words. It says "die by suicide", not "commit suicide", and "fatal" or "non-fatal suicide attempt", not "successful" or "failed" [SSS]. Studies find negative wording more common for people with serious mental illness [STIG]. One reports stigmatising terms copied forward in up to 25 percent of notes [STIG]. Nothing I found studied self-harm notes.
- **A flag that follows someone for ever.** The Veterans Health Administration has a High Risk for Suicide Patient Record Flag, governed by Directive 1166 of November 2023. It is reviewed about every 90 days, with inactivation decided at 80 to 100 days and a documented reason [VAFLAG]. Risk stayed elevated after inactivation, and contacts must follow it [VAFLAG]. So ending is a moment of need, not an all-clear. I did not read the directive or the provider-perspective study [VAFLAG].
- **A flag no one sees.** HSSIB found staff "unaware of the flag" and that EPR functions not meeting needs go unused [HSSIBEPR] [HSSIB25].
- **Alert fatigue.** Alerts do not exist in this project yet. FR-6.5 and [pathway design](../pathway-design.md) carry the principle.
- **Exposure.** A portal, a proxy, a letter, a text or a banner can disclose. A mask that says "restricted information exists" reveals existence (see [protected information](../access-control/protected-information.md)). In England a controller can withhold health data where disclosure would be likely to cause serious harm [SAR]; so a portal rule and a subject-access rule both matter. Culture of care says patients may see and help write notes, and a disagreement should be documented [CULT].
- **The person's words.** Words said in crisis, kept for ever, in an append-only record, can harm. The answer is restriction and an amendment route, not deletion: the record has no delete path. Today amendments cover `Observation` and `AllergyIntolerance` kinds. A quotation also needs the context of who wrote it down and when. A person's words about method should stay out of any shared view (my reasoning).
- **Copied narrative.** See the copying point above. One assessment copied across encounters looks like many.
- **Blame.** Pattern analysis that names staff can deepen the culture HSSIB describes. It must be for learning first [HSSIB25].

**What this project would need.** No free-standing "suicide risk" flag. Instead: an active observation order that ends; a current safety plan; a narrative formulation. If a pack adds a flag, it is time-boxed, has a named reviewer, an end reason and a follow-up on ending. It shows only to those who need it, and is never inferred by software. The person knows it exists, unless a clinician records why not.

## Design: reason-neutral lists (SW-6)

A first draft. Nothing is built: the project has no ward list, bed board or observation order yet (SW-3). No nurse, observer or person who has been observed has reviewed it.

A ward list is read by whoever passes it, and it may be on a screen. The reason someone is watched is the most sensitive thing on that screen, and the people who need the level usually do not need the reason.

| Shown | On a list or board | To the observer | To the care team, opening the record |
|---|---|---|---|
| That enhanced care is in place | Yes, as "Enhanced care" | Yes | Yes |
| The level, with the pack's words | Yes | Yes | Yes |
| Who holds it now, and when the next check is due | Yes | Yes | Yes |
| What to watch for and what to do | No | Yes, as written for this order | Yes |
| The reason | No | Only what the order's author chose to include (OQ-SW1) | Yes, under the usual access rules |

Rules:

1. **One word for every reason.** The list says "Enhanced care" whether the reason is self-harm, falls, confusion or a risk to others, so the words cannot give it away. A level that exists only for one reason gives the reason away too, so the pack must not have one.
2. **No code, colour, icon or sort order on a list may stand for a reason**, and the reason is not in a list's search or export.
3. **A list shows only what a person holding that list needs.** A receptionist's view does not show the level. Which roles see which column is pack data, and is for clinicians to set.
4. **The reason stays restricted on the order**, as a labelled field, under PI-3's rules when they exist. Break-glass does not change what a list shows.
5. **The level goes when the order ends.** An ended order leaves no mark on a list, so that a person is not known for it afterwards.

**A support worker role.** Support workers do most of this work and have no role here, so with RA-2 add a `support-worker` sub-role, and a visiting observer with a time-boxed relationship. Their authority: see the list and their own order, record checks, and no care planning, which matches unregistered staff under [ETOC-POL]. The role does not exist yet, and a role matrix change is a clinical change.

Open: whether an observer needs the reason (OQ-SW1); whether "enhanced care" itself is read as "suicide watch" on a ward where most orders are; and what a shared screen in a corridor should show at all.

## Design: preserve after an event (SW-7)

A first draft, for a Clinical Safety Officer and a lawyer. Nothing is built. **A correction:** this page used to say that archiving takes audit rows out of a patient's log. It does not: the log and the audit search read `audit_events_all`, which includes the archive. The research was wrong, and the gap is narrower than it said.

**What is at risk today, and what is not.**

| What could be lost | Today | Needed |
|---|---|---|
| A clinical entry | Append-only; no delete path | Nothing more |
| An audit row | Moved, never deleted; a trigger refuses change, and the archive keeps it | A rule for the future, below |
| The archive itself | A database owner can drop the schema. `GET /api/audit/verify` finds a damaged archive; nothing stops the owner | Outside the service: access to the database and tested backups |
| What a screen showed at the time | Not kept. The versions are, so it can be rebuilt only in part | A frozen export, later |
| How a late entry reads | It shows its written time ([SW-2](#falsified-and-back-filled-records)) | A mark, below |

**The archive and a hold.** Moving rows to the archive destroys nothing, so a hold need not stop it, and stopping it for one patient would hold a whole month of everyone's rows. The hold stands in front of anything that would destroy. No such thing exists. **The rule: any future retention job, export-and-purge or archive removal must refuse a row or record under a hold, and no such job may ship without a test that it does.**

**A hold.** An append-only record: patient, the learning response or inquiry it belongs to, who placed it, when, why, and when and by whom it was lifted. Placing it is simple and takes effect at once. Lifting it needs a named person and a reason, and a second person who did not place it. It changes no access: an investigator gets no new right to read by holding a record, and nothing about a hold reaches a proxy or a patient's view.

**A mark.** Anything written after a known event reads as written afterwards. It compares `recorded_at`, the database's clock, with the time of the event, and never `effective_at`. A hold may name the event, or a [death assertion](../bereavement/death-assertion.md) may be it. A mark says when, not that an entry is wrong: people must go on writing after a death, and a late entry made in good faith is not an accusation.

Open:

- Who may place and lift a hold.
- Whether staff should see that a record is held, since it may change what they write.
- Whether a report that is later withdrawn leaves a mark.
- What a family may see, and when.
- How long a hold lasts when nobody lifts it. Retention law and the schedule [RET] were not read for this.

## Compared with this project

"Verified" names what I read in the code on 2026-11-04.

| Need | Today | Gap | Verified against |
|---|---|---|---|
| Append-only store with true write time | Built for vitals: `observation_versions`, triggers refuse change, `recorded_at` set by the database, `effective_at` by the writer | Only vital signs; nothing for a therapeutic observation | `migrations/20261026000014_observations.sql`, `observations.rs` (`SUPPORTED`) |
| A late entry shown as late | `recorded_at` stored and sent as FHIR `issued`. The vital signs list, trend, history and summary show both clocks, and a first entry written more than 60 minutes after its measured time says "late entry" (SW-2; the 60 is a guess) | A reading more than 7 days old needs a reason, and one before the patient's birth date is refused. Late says when it was written and cannot show that a false entry made on time is false | `observations_store.rs` (`parse_effective`), `interop/fhir_observation.rs`, UI `vitals.ts` (`writtenText`) |
| Absent evidence | `AbsentReason`: not done, declined, failed, unknown | No "no observer", no reason text, no expected slot | `observations.rs` |
| Observation order, holder, handover | None | All of it | grep of `src` and `migration` found no such term |
| Timers and loop-back | Built for referrals: configured periods, no default, a worklist | Not generalised; pathway not built | `referral_timeouts.rs`, spec [5.2.1](../05-domain-model.md) |
| Roles | Eleven: clinician, nurse, pharmacist, allied-health, receptionist, clinical-admin, patient, proxy, auditor, integrator, clinical-safety-officer | **No healthcare support worker or observer, though they do most of this work.** No bank, agency or visiting-observer shape | `policy.rs` `Role` |
| Mental health label | `PSY`: unmasked for clinician, nurse, allied health, patient | Pharmacist masked; a proxy sees the mask; nothing carries a label yet (SE-4) | `policy.rs` `unmasked_for` |
| Emergency access | Break-glass lifts only a missing relationship; never a label | A safety plan under a label stays hidden in an emergency | `break_glass.rs`, `policy.rs` |
| Mask | Says "restricted information exists" | Reveals existence to a proxy | `policy.rs` `Obligation::Mask` |
| Proxy | Scoped, dated, revocable authority | No hidden-from list; no young-person rule | `proxy.rs` |
| Sharing decision log | Consent records and permits exist | No per-occasion disclosure record | `consent.rs` |
| Amendment | Patient route for `Observation` and `AllergyIntolerance` | No route for narrative or a plan, as those do not exist | `service spec` RC-4, `amendment.rs` |
| Alerts, risk score | None. Alert text appears only in code tests | Good: nothing to remove. FR-6.5 is unbuilt | grep of `src` |
| Legal status, capacity | None; XP-7 is a design note task | All of it | `tasks.md` XP-7 |
| Legal hold, post-event mark | The audit archive moves rows, if switched on, and they stay in the patient's log through `audit_events_all`. Nothing deletes an audit row | No hold, no mark | `audit_retention.rs`, migration 19 |
| Notifications, letters | No outbound messages to people; an outbox decides per recipient | Build each with a label test | `outbox.rs` |

I searched `src` and `migration` for suicide, self-harm, safety plan, observation level, Mental Health Act and trauma, and found nothing.

## Candidate hazards

For the Clinical Safety Officer. None is accepted, and none is added to the [hazard log](../../governance/clinical-safety/hazard-log.csv) by this page. Ids are working labels. Nothing here creates the exposure yet, because nothing is built.

- SW-1 A risk level or score is stored or displayed, gives false reassurance and is used to refuse care or discharge.
- SW-2 A false observation made on time looks true. Append-only does not detect it.
- SW-3 A back-filled entry displays as contemporaneous, because the screen shows the claimed time only.
- SW-4 No one holds the observation: a break, a handover or an unfilled shift leaves a gap that the screen does not show.
- SW-5 An order lapses unreviewed, or a restrictive order continues with no review.
- SW-6 A person at risk arrives in another department and the plan or order is hidden by a label, and break-glass does not unmask it (the reverse of H-007).
- SW-7 A flag, a plan or a family note is exposed through a portal, proxy, letter, text or banner to a partner, parent or employer.
- SW-8 A permanent flag stigmatises and changes care, or physical symptoms are read as mental health (the second is my reasoning).
- SW-9 Ending a flag or order is read as "safe" and support stops, in the period when evidence suggests risk stays raised.
- SW-10 Alerts are so frequent that they are dismissed.
- SW-11 A safety plan is out of date, cannot be found in a crisis, or is shared beyond what the person decided.
- SW-12 A follow-up target is met by a tick; the follow-up has no owner after transfer; or a person discharged from an emergency department has none at all.
- SW-13 Offline entries are lost, bunched or dated by a wrong clock.
- SW-14 A device event closes an observation, or monitoring replaces an in-person check.
- SW-15 An observer with no account uses another person's login, so entries name the wrong person.
- SW-16 In a cohort, an entry goes on the wrong patient.
- SW-17 A sharing decision is unrecorded; or a person's "do not tell" is missed; or a family's account is shown back to the person in a way that endangers them.
- SW-18 A restriction runs without a legal basis, or capacity from one decision is read as applying to another.
- SW-19 After a death, a late entry reads as if made before; audit rows are archived; or retention destroys what an inquiry needs.
- SW-20 Stigmatising words or a quotation are copied forward, or method detail reaches a shared view.
- SW-21 Pattern review creates blame, so people stop saying a check was missed.

## Ideas

Proposals only. None is to be built before clinicians, observers and people with lived experience have shaped it.

1. **Observation order and holder ledger** as a setting-neutral record, with slots from a pack.
2. **Three-state check:** seen, not seen with a reason, or no holder. Equal effort for each. The chart never interpolates.
3. **Both clocks on every screen** for observations: when it happened and when it was written; late entries say so. This uses what `observation_versions` already holds.
4. **A short observer view** on a phone, for one patient: the plan, what helps, the next check, take over and hand over.
5. **A census test for risk words.** Built (SW-1): `electronic-patient-record-service-with-rust/tests/risk_field_census.rs` fails if a `risk_level`-shaped field appears in the core; a pack that adds one must declare it. It sees field names only: it cannot stop a score written in free text or held under another name.
6. **A safety plan owned by the person**, versioned, printable, with a recipient list the person controls.
7. **An emergency rule for a safety fact** with [OQ-PI2](../16-open-questions.md): what break-glass shows of a plan.
8. **Follow-up as a pathway step** with an owner and a loop-back, reusing the referral pattern.
9. **A support worker and a visiting observer role** ([roles](../access-control/roles-and-assignments.md)), with a time-boxed relationship.
10. **A sharing log and an involve/do-not-involve statement.**
11. **Reason-neutral ward lists:** show "enhanced care" and a level, not why.
12. **Hold and post-event marks**, tied to a learning response.
13. **Pattern review for learning**, sampled by someone outside the ward, reported in aggregate first.
14. **Add the observation row to [manual steps](../manual-steps/index.md)** when the shape settles, since much of this happens on paper today.

## Open questions

1. Who should shape this first: observers, patients, families, or the Clinical Safety Officer? All of them, but in what order?
2. Should "suicide watch" stay as the directory name? It is what was asked for, and not the UK word.
3. Should a US deployment's screen and level be allowed in the same data model, or only in a pack?
4. What should an observer see of the reason for observation?
5. What does break-glass show of a safety plan, and is the person asked in advance?
6. How does a safety plan work when a person has no phone, or a partner reads it?
7. Is there any honest way to detect a false on-time entry that does not place cameras on patients or treat staff as suspects?
8. What do existing electronic observation systems do about offline work, late entries and proof of presence? Not researched.
9. What does the national guidance recommended by HSSIB now say, and is it what the 2025 ETOC guides are?
10. Will the Lampard Inquiry's interim decision or final report change this? Revisit then.
11. What does a family get to see after a death, and on what legal basis?
12. Should the person be told about any flag, always?
13. What is the right rule for a young person's plan and a parent?
14. Which Mental Health Act regime does a pack model during the long transition?

## Not verified

- **NICE site pages returned 403.** NG225 was read from a copy on the Lampard Inquiry site (text dated 2024 in its footer). Recommendation numbers are from that copy. The live page may differ. NG10 was not read.
- **England's national health body's `Staying safe`** was read from a copy held elsewhere dated 14 April 2025; England's national health body's page says it was updated on 10 September 2025.
- **Coroners' reports** were read as fetched summaries, or snippets for [PFD-2026]. They state concerns, not findings of the cause of a death. I left out names and detail on purpose, though the URLs contain names.
- **Lampard.** Status changes quickly. I saw no decision after 20 October 2026. The evidence described is reported by press, lawyers and a campaign, none of which is the transcript.
- **East London.** The commentary is one author's. The human factors review, the FOI material and the September 2025 report were not read. The trust's position comes from a council record snippet.
- **Oxevision.** Uptake figures conflict. The ten-trust referral rests on a weak aggregator. CQC's brief guide returned 403; the CQC 2024/25 report page did not mention observation directly, and the point on relationships comes from a search summary.
- **PPO:** the 2025 report was not read; a local press item is the only source on altered records.
- **NCISH:** only the 2026 web page was read, up to about nine-tenths of its length. Figures for 2021 to 2023 are partly estimated, as the report says.
- **US:** Joint Commission text returned 403 and may be revised. VHA directive not read. CMS rules were not read. Hospital policies are local and old.
- **Law:** every statement is a lay reading of a summary. The Mental Health Act 2025 commencement position was not checked against commencement orders.
- **Retention figures** are from secondary summaries and a local schedule; the current code was not opened.
- **Not found:** RCPCH guidance; any study of what causes falsification; any study of self-harm notes and language; vendor electronic observation documents.
- **No evaluation exists** of any idea here. The "what this project would need" lines are mine.
- **Code:** read and grepped, not run. No test was added.
- Link check 2026-11-04: these cited pages returned 404. They may have moved, but what rests on them is unconfirmed: https://www.yahoo.com/news/articles/concerns-over-monitoring-system-used-092624694.html; https://www.aol.com/news/outcome-lampard-inquiry-delayed-six-160300574.html; https://www.thurrock.gov.uk/sites/default/files/assets/documents/safeguarding-adults-mentalcapacity-202212-v01.pdf

## Sources

Read 2026-11-04. Marks: **(fetched)** the page was opened and summarised by the fetch tool; **(PDF text)** text extracted and read directly; **(search summary)** from a search tool's summary; **(snippet)** a search result only.

- [NG225] NICE NG225, Self-harm, published 7 September 2022, copy: https://lampardinquiry.org.uk/wp-content/uploads/2025/04/20240816-NG225-NICE-guideline-LIVE.pdf (PDF text); live page, 403: https://www.nice.org.uk/guidance/ng225/chapter/Recommendations (snippet)
- [SSS] England's national health body, Staying safe from suicide: https://www.england.nhs.uk/publication/staying-safe-from-suicide/ (fetched); copy: https://www.infocop.es/wp-content/uploads/2025/06/NHS-England-Staying-safe-from-suicide.pdf (PDF text)
- [ETOC-TP] England's national health body, ETOC, embedding therapeutic practice, PRN01862_iii, 2025: https://www.england.nhs.uk/wp-content/uploads/2025/11/PRN01862-iii-etoc-embedding-therapeutic-practice.pdf (PDF text)
- [ETOC-POL] England's national health body, ETOC, developing a local policy, 28 August 2025: https://www.england.nhs.uk/long-read/enhanced-therapeutic-observation-and-care-developing-a-local-policy/ (fetched)
- [ETOC-DATA] England's national health body, ETOC data guide: https://www.england.nhs.uk/long-read/etoc-guide-improving-data-collection-reporting/ (fetched)
- [CULT] England's national health body, Culture of care standards: https://www.england.nhs.uk/long-read/culture-of-care-standards-for-mental-health-inpatient-services/ (fetched)
- [DIGP] England's national health body, Principles for using digital technologies, February 2025: https://www.england.nhs.uk/long-read/principles-for-using-digital-technologies-in-mental-health-inpatient-treatment-and-care/ (fetched)
- [HSSIB24] HSSIB, Patients at risk of self-harm: continuous observation, 8 May 2024: https://www.hssib.org.uk/patient-safety-investigations/patients-at-risk-of-self-harm-continuous-observation/investigation-report/ (fetched)
- [HSSIB25] HSSIB, Mental health inpatient settings overarching report, 13 May 2025: https://www.hssib.org.uk/patient-safety-investigations/mental-health-inpatient-settings/fifth-investigation-report/ (fetched)
- [HSSIBEPR] HSSIB, EPR thematic review, 27 November 2025: https://www.hssib.org.uk/patient-safety-investigations/electronic-patient-record-epr-systems-thematic-review/investigation-report/ (fetched)
- [HIS] Healthcare Improvement Scotland, From Observation to Intervention: https://ihub.scot/project-toolkits/improving-observation-practice/from-observation-to-intervention (search summary)
- [NSW] NSW Health, PD2026_022: https://www1.health.nsw.gov.au/pds/ActivePDSDocuments/PD2026_022.pdf (PDF text)
- [NSWCOR] NSW coroner's findings: https://coroners.nsw.gov.au/content/dam/dcj/ctsd/coronerscourt/documents/findings/2018/MA%20Findings.pdf (search summary)
- [PFD-BACK] Prevention of future deaths report, July 2024: https://www.judiciary.uk/prevention-of-future-death-reports/anna-elliott-prevention-of-future-deaths-report/ (fetched)
- [PFD-AUD] Prevention of future deaths report, November 2022: https://www.judiciary.uk/prevention-of-future-death-reports/rowan-thompson-prevention-of-future-deaths-report/ (fetched)
- [PFD-WIFI] Prevention of future deaths report, April 2026: https://www.judiciary.uk/prevention-of-future-death-reports/kiefer-fraser-phillips-prevention-of-future-deaths-report/ (fetched)
- [PFD-TAB] Prevention of future deaths report, October 2025: https://www.judiciary.uk/prevention-of-future-death-reports/shannon-lee-prevention-of-future-deaths-report/ (fetched)
- [PFD-2026] Prevention of future deaths report, 2026: https://www.judiciary.uk/wp-content/uploads/2026/04/Mark-Smith-Prevention-of-future-deaths-report-2026-2025_Published.pdf (snippet)
- [ALEX] M. Alexander, commentary on East London mental health foundation trust: https://minhalexander.com/2026/03/29/falsification-of-observation-records-in-mental-health-a-closer-look-at-outstanding-east-london-nhs-foundation-trust/ (fetched; one author's account)
- [HACK] Hackney council committee record: https://hackney.moderngov.co.uk/mgAi.aspx?ID=44819 (snippet; fetch returned 403)
- [LAMPE] ITV, Lampard Inquiry evidence, May 2025: https://www.itv.com/news/anglia/2025-05-12/falsifying-safety-records-well-known-at-mental-health-trust-inquiry-told (search summary)
- [LAMPT] Farleys on the delay: https://www.farleys.com/delay-in-the-outcome-of-the-lampard-inquiry-contributed-to-by-the-late-disclosure-of-evidence/ ; AOL: https://www.aol.com/news/outcome-lampard-inquiry-delayed-six-160300574.html (search summary)
- [BIND] Bindmans, 2 October 2026: https://www.bindmans.com/news-insights/news/lampard-inquiry-to-consider-further-evidence-in-support-of-interim-recommendations-on-the-use-of-oxevision-in-mental-health-inpatient-settings/ (fetched; a solicitor for families)
- [STOP] Stop Oxevision, summary of July 2026 hearings: https://stopoxevision.com/2026/08/17/lampard-inquiry-july-2026-hearings-about-vision-based-monitoring-oxevision-lio-a-summary/ (fetched; campaign source)
- [RETH] Rethink Mental Illness, position on Oxevision: https://www.rethink.org/news-and-stories/media-centre/2023/11/our-position-on-oxevision-the-new-monitoring-system-in-mental-health-units/ (fetched)
- [GRIF] Griffiths et al., BMC Medicine 2024: https://link.springer.com/content/pdf/10.1186/s12916-024-03673-9.pdf (search summary)
- [OXE-USE] Press on uptake: https://www.yahoo.com/news/articles/concerns-over-monitoring-system-used-092624694.html ; campaign figure in [STOP] search result (snippets)
- [OXE-REF] Report of referrals: https://streamlinefeed.co.ke/news/ten-nhs-trusts-referred-health-regulator-oxevision-surveillance-cameras (snippet; weak)
- [NCISH26] NCISH Annual report 2026: https://sites.manchester.ac.uk/ncish/reports/annual-report-2026/ (fetched)
- [72H] Nuffield Trust: https://nuffieldtrust.org.uk/resource/follow-up-care-for-adults-with-mental-health-problems ; RCPsych: https://www.rcpsych.ac.uk/improving-care/nccmh/dyfodol/dyfodol-news/detail/2022/08/22/72-hr-discharge-follow-up-target-for-mental-health-patients-missed-38-000-times ; national health standards: https://standards.nhs.uk/published-standards/mental-health-community-teams-activity ; ombudsman press: https://sg.news.yahoo.com/unsafe-discharge-putting-mental-health-000100121.html (search summary)
- [STANLEY] Stanley et al. 2018, summary: https://www.healio.com/news/psychiatry/20180711/intervention-with-followup-reduces-suicidal-behavior-after-ed-discharge ; paper: https://pmc.ncbi.nlm.nih.gov/articles/PMC6142908 (search summary)
- [CONS] DHSC, consensus statement, 2021: https://www.gov.uk/government/publications/consensus-statement-for-information-sharing-and-suicide-prevention/information-sharing-and-suicide-prevention-consensus-statement (fetched)
- [SHARE] SHARE guide: https://www.gov.uk/government/publications/share-consent-confidentiality-and-information-sharing-in-mental-healthcare-and-suicide-prevention/share-consent-confidentiality-and-information-sharing-in-mental-healthcare-and-suicide-prevention ; Northern Ireland: https://www.health-ni.gov.uk/sites/default/files/publications/health/doh-share-guide-ni.pdf (search summary)
- [PSIRF] England's national health body, PSIRF: https://www.england.nhs.uk/patient-safety/patient-safety-insight/incident-response-framework/ ; engaging and involving: https://www.england.nhs.uk/patient-safety/patient-safety-insight/incident-response-framework/engaging-and-involving-patients-families-and-staff-following-a-patient-safety-incident/ ; standards: https://www.england.nhs.uk/long-read/patient-safety-incident-response-standards/ ; a mental health trust: https://www.bsmhft.nhs.uk/service-users-and-carers/service-user-information/patient-safety-incident-response-framework-psirf/ ; a safety guide: https://safework.place/guides/psirf/ (search summary)
- [CAND] CQC, Regulation 20: https://www.cqc.org.uk/guidance-regulation/providers/regulations-service-providers-and-managers/health-social-care-act/regulation-20/what-you-must-do ; in full: https://www.cqc.org.uk/guidance-regulation/providers/regulations-service-providers-and-managers/health-social-care-act/regulation-20/regulation-20-in-full (search summary)
- [RET] England's national health body retention schedule: https://digital.nhs.uk/data-and-information/information-governance/guidance/records-management-code-of-practice/appendix-ii/retention-schedule-care-records ; a summary: https://docflow.co.uk/nhs-records-management/ (search summary)
- [PPO] A Prisons and Probation Ombudsman report: https://cloud-platform-e218f50a4812967ba1215eaecede923f.s3.amazonaws.com/uploads/sites/14/2025/08/F6970-24-Death-of-Mr-Michael-Pugh-Swaleside-29-06-2024-SI-22-30-29-1.pdf ; press on the 2025 report: https://www.aol.com/news/prison-staff-fake-checks-suicidal-172355661.html ; local press: https://www.shropshirestar.com/news/local-hubs/north-shropshire/market-drayton/2023/12/11/report-finds-staff-altered-prisoners-record-after-death/ (snippets)
- [JC] Joint Commission FAQ: https://www.jointcommission.org/en/knowledge-library/support-center/standards-interpretation/standards-faqs/000001215 (snippet; fetch 403); R3 report: https://digitalassets.jointcommission.org/api/public/content/459bbe2be1ab4e5082b8a8d49d0c94e0?v=69255bb9 (search summary); hospital council summary: https://hospitalcouncil.org/wp-content/uploads/sites/2/2022/06/Suicide-Risk-Prevention-Best-Practice-Recommendations.pdf (search summary)
- [ASQ] NIMH ASQ toolkit: https://www.nimh.nih.gov/research/research-conducted-at-nimh/asq-toolkit-materials (search summary)
- [PSNET] AHRQ PSNet: https://psnet.ahrq.gov/perspective/suicide-prevention (search summary)
- [ZS] Zero Suicide: https://zerosuicide.edc.org/resources/resource-database/zest-toolkit-suicide-safer-care-centers ; Pew: https://www.pew.org/en/research-and-analysis/articles/2022/09/06/a-proven-roadmap-to-reduce-suicides-starts-with-transformative-health-care (search summary)
- [988] SAMHSA FAQ: https://www.samhsa.gov/mental-health/988/faqs ; launch: https://www.samhsa.gov/newsroom/press-announcements/20220715/us-transition-988-suicide-crisis-lifeline-begins-saturday (search summary)
- [VAFLAG] VHA Directive 1166: https://www.va.gov/VHAPublications/ViewPublication.asp?pub_ID=11547 (snippet); GAO: https://www.gao.gov/assets/gao-22-105165.pdf ; mortality study: https://pubmed.ncbi.nlm.nih.gov/34320826/ ; provider perspectives, could not be opened: https://pmc.ncbi.nlm.nih.gov/articles/PMC12614300/ (search summary)
- [USPOL] Local US policies, examples: https://www.upstate.edu/medstaff/pdf/forms/Suicide_Precautions.pdf ; https://cdn2.hubspot.net/hubfs/2101401/Clinical%20Operations/600-263%20Sitter%20Orientation_Revised.pdf (snippets)
- [CYP] Local policy: https://royalwolverhampton.nhs.uk/wp-content/uploads/2024/10/17517_CP_63_PUBLIC_Policy_Printable_Version.pdf ; review: https://irep.ntu.ac.uk/id/eprint/54535/1/2490818_Breedon.pdf (snippets; the mapping of the review to this file is a best reading of the search result)
- [MHA25] Mental Health Act 2025: https://www.legislation.gov.uk/ukpga/2025/33 ; Browne Jacobson: https://www.brownejacobson.com/insights/mental-health-bill-receives-royal-assent ; Deans Court: https://deanscourt.co.uk/articles/the-mental-health-act-2025 (search summary)
- [RCEM] RCEM, Mental Capacity Act in emergency medicine: https://rcem.ac.uk/wp-content/uploads/2024/12/RCEM_Mental_Capacity_Act_in_EM_Practice-Feb_2017_V2-Copy.pdf (search summary)
- [MCA] Training workbook: https://www.thurrock.gov.uk/sites/default/files/assets/documents/safeguarding-adults-mentalcapacity-202212-v01.pdf (search summary; weak for law)
- [ADRT] BJPsych Open 2019: https://www.cambridge.org/core/product/3365ABDAD49526E22073A5B8F801CD6F/core-reader (search summary)
- [SAR] ICO, health information: https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/individual-rights/right-of-access/health-information/ (search summary)
- [STIG] AMA Journal of Ethics: https://journalofethics.ama-assn.org/article/when-and-how-should-clinicians-share-details-health-record-patients-mental-illness/2017-03 ; notes study: https://pmc.ncbi.nlm.nih.gov/articles/PMC13005122/ ; copying forward: https://link.springer.com/article/10.1186/s12913-026-15358-5 (search summary)
- This repository: `electronic-patient-record-service-with-rust/src/policy.rs`, `observations.rs`, `observations_store.rs`, `break_glass.rs`, `proxy.rs`, `consent.rs`, `audit_retention.rs`, `referral_timeouts.rs`, `outbox.rs`, `migrations/20261026000014_observations.sql`; [service spec](../../electronic-patient-record-service-with-rust/spec/index.md).
