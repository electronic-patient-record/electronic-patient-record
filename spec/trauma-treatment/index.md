# Trauma treatment: psychological trauma and physical major trauma

Public-source research read 2026-11-04 plus this repository's own files. A proposal, not clinical or legal advice: nothing here is built, and no clinician or person with lived experience has reviewed it. **NOT FOR CLINICAL USE.**

## Related links

[Protected information](../access-control/protected-information.md), [Roles and assignments](../access-control/roles-and-assignments.md), [Access control and consent](../access-control/index.md), [Suicide watch](../suicide-watch/index.md), [Patient identity](../patient-identity/index.md), [Matcher spec](../../electronic-patient-record-matcher-with-rust/spec/index.md), [Domain model 5.2.1](../05-domain-model.md), [Pathway design](../pathway-design.md), [Care settings](../care-settings.md), [Emergency triage](../triage/index.md), [Manual steps](../manual-steps/index.md), [Data exchange](../data-exchange/index.md), [Maternity](../maternity/index.md), the [hazard log](../../governance/clinical-safety/hazard-log.csv).

Method: web searches and fetches, primary sources where they could be opened, and a reading of the code. England is the worked example; the US, Australia, New Zealand and Canada appear as examples, never as the rule ([equity rule](../02-scope.md)). Law belongs in a country pack. This page says what evidence bodies say and what a record must hold. It does not say how to treat anyone.

## Why both senses are on one page

The request names "trauma treatment", and in a hospital the word means two things. They meet in one person. Someone injured in an assault arrives unknown, is resuscitated, and months later may be living with post-traumatic stress and a police investigation. Part one and part two share three design problems: identity under pressure, the same story asked for again, and who may see what.

## Part one: psychological trauma

### What evidence bodies say

Stated as evidence, not as instruction.

- **NICE NG116 (England, December 2018).** Recognises PTSD in people who may have experienced single, repeated or multiple events. Offer individual trauma-focused CBT to adults with PTSD or clinically important symptoms who present more than 1 month after an event (1.6.16). EMDR: consider between 1 and 3 months after a non-combat trauma if the person prefers it (1.6.18); offer later in the terms of 1.6.19 [NG116]. "Do not offer psychologically-focused debriefing for the prevention or treatment of PTSD" (1.6.5). Do not offer drug treatments to prevent or treat PTSD in children and young people (1.6.14), or, for adults, to prevent it (1.6.24). Active monitoring may be considered for subthreshold symptoms (1.6.4). NICE's page gives a last-reviewed date of 23 July 2026 [NG116P].
- **WHO.** The 2013 guideline covers acute stress, post-traumatic stress and grief in non-specialised settings; the mhGAP update of 2023 lists options including trauma-focused CBT and stress management [WHO]. WHO's module says stress management may be the most feasible option where trauma-focused treatments are not available [WHO]. That is why a record must not assume one therapy model (my reasoning).
- **ICD-11.** Complex PTSD (6B41) is a separate diagnosis from PTSD (6B40): all PTSD requirements, plus disturbances in self-organisation, meaning affect regulation, negative self-concept and relationships [ICD]. DSM-5 has no separate diagnosis [ICD]. NICE notes it as arising most commonly after prolonged or repetitive events from which escape is difficult, and says to build in extra time for trust and to consider safety and stability (1.7.3) [NG116]. I did not read the WHO browser.
- **Risk.** Where assessment identifies significant risk, NICE asks for a risk management and safety plan (1.6.3) [NG116]. That links to [suicide watch](../suicide-watch/index.md), which also explains why such a plan should not be a score.

**What this project would need.** Diagnosis coded by a terminology that distinguishes ICD-11 from DSM (a code system pack; the record's fixture has no such code). A therapy course as a pathway: modality as data, sessions, outcome measures, breaks in treatment. Modality names are never a recommendation.

### Trauma-informed care

- **England.** The working definition (November 2022) has six principles: safety, trust, choice, collaboration, empowerment and cultural consideration. It "seeks to avoid re-traumatisation", and it is not treatment, which stays with specialist services [OHID].
- **United States.** SAMHSA's 2014 concept (SMA14-4884) names six principles: safety; trustworthiness and transparency; peer support; collaboration and mutuality; empowerment, voice and choice; and cultural, historical and gender issues. It says to realise, recognise, respond and resist re-traumatisation [SAMHSA].
- **Inpatient standards.** Culture of care says "Inpatient care must be trauma informed, autism informed and culturally competent", calls restraint, seclusion and coercion harmful, and supports people seeing and helping write their notes [CULT].
- **Environment.** NICE warns against assessing or treating people in noisy or restricted settings, or restraining them, because these may worsen symptoms (1.4.4) [NG116].

**What this project would need.** The principles map to software. Safety: not exposing a survivor. Trust: telling people what is held and who saw it (the access log exists). Choice: a say over sharing, gender of clinician and contact method. Collaboration: the person's own voice in the record. These mappings are mine.

### Re-traumatisation by systems: telling the story once

- HSSIB found that information lost at transitions eroded trust, and that for young people repeatedly retelling history "had the potential to re-traumatise" [HSSIB25].
- NHS England tells organisations to give access to past records so that people need not repeat themselves [SSS].
- NICE asks that assessments be comprehensive, that transitions be supported, and that who monitors be agreed in writing where care is shared (1.2.3, 1.2.4) [NG116]. It also suggests a key professional for continuity, in its rationale [NG116].
- **Evidence gap.** I found no empirical study that tests one shared trauma history against repeated assessment. The retelling sources are guidance. So "tell it once" is a sensible aim, not a proven intervention.

**What a record can do**, my reasoning:

1. **Two layers.** A small, widely visible, person-approved layer: "I have spoken about this; ask me before raising it", what helps, what to avoid (for example restraint or a lone male clinician). A second layer holds the narrative, for named people only. Most staff need the first.
2. **Consent before reading, not only before writing.** Reading is also an exposure.
3. **A pointer, not a copy.** "Told to X on this date" lets a new clinician ask X. It does not put a copied narrative in each letter.
4. **The person edits the summary.** Later versions are new versions. The old one stays, restricted.
5. **Permission to not repeat.** Clinicians still need to ask in some situations, such as new risk. The record must not turn "we have it" into not listening.

### A survivor's control over who sees it

Link: [protected information](../access-control/protected-information.md), with categories, leakage routes, the mask and the hazards. Repeated here only where trauma changes the answer.

- A survivor may want a named clinician to know and no one else. Labels today are role lists, so a clinician in any team unmasks. This is the "no relationship test" gap on that page.
- The person may want a fact shared later, then not. Consent versions support changing that. Nothing can recall what someone has already read.
- A portal or proxy can reach an abuser. Refuge and the BMA have urged survivors to ask for record sharing to be switched off (cited on that page).
- HL7 has `SDV` (sexual and domestic violence), `VIO` and `PSYTHPN` (psychotherapy note). This project has none of them.

### Sexual assault referral centres and forensic records

- **England.** NHS England's national service specification requires secure storage of medical records and adherence to Forensic Science Regulator standards, with documented, traceable records, exhibit handling and control of contamination; competence and confidentiality for children need careful negotiation; SARCs supply record-level data for commissioning [SARC].
- **Choice.** A local NHS SARC says a person chooses whether to report to the police and may have forensic samples stored for up to two years [SARCLOCAL]. FFLM guidance covers sample storage under the Human Tissue Act and asks for confidentiality protocols [FFLM].
- **Disclosure.** A search result from that local SARC site says material may be disclosable if there is an investigation [SARCLOCAL], though the page I fetched did not contain that wording. So treat it as unconfirmed.
- **Not found.** I found no source on keeping SARC records apart from the GP or hospital record, or what goes to the GP. The current FSR code was not read.

**What this project would need.** A forensic record is not a clinical note, and a chain of custody is a different duty. My reasoning: it does not belong in an EPR's general store. The EPR holds, at most, a pointer agreed with the person. It holds clinical facts a treating clinician needs, such as prophylaxis, through the person's choice. A SARC deployment would be a separate tenant or system.

### Refugees and torture survivors

- NICE: for refugees and asylum seekers at high risk of PTSD, think about routine use of a validated brief screen as part of a health screen (1.1.9); culturally and linguistically appropriate assessment (1.5.2); interpreters or a choice of therapist (1.5.3) [NG116]. Services should be designed around groups including migrants and asylum seekers (1.3.1, as summarised) [NG116P].
- **Istanbul Protocol.** The 2022 edition, from OHCHR, adds a chapter on health professionals' roles in documenting torture in different contexts [IP]. Physicians for Human Rights says medical notes, charts and body charts are part of medico-legal documentation [IP]. An ordinary clinician's note may therefore become evidence. I read summaries, not the protocol.
- **Immigration.** See [protected information](../access-control/protected-information.md). Fear of enforcement keeps people from care, and the position after 2018 is unknown.
- **Not found.** Freedom from Torture's practice and how NHS records are used in asylum claims.

**What this project would need.** A record of who interpreted and in what language, so that a mistranslation can be seen as such. An allowed body-map attachment, restricted. Absent identity documents do not block care. An inconsistent history is not a data-quality error to "fix": the matcher's and the data-quality rules must not treat it as one (my reasoning).

### Veterans

- **England.** Op COURAGE (launched 2021) is an NHS veterans' mental health service. Veterans can self-refer. One NHS page says one full day's service is enough, regardless of when it was [OPC].
- **United States.** Veterans can press 1 after dialling 988 [988]. The VA/DoD 2023 guideline was not found. The Veterans Health Administration flag is on the [suicide watch](../suicide-watch/index.md) page.
- NICE draws a line between combat-related and other trauma in its EMDR wording (1.6.18) [NG116]. A single veteran sample showed high rates of PTSD, depression and anxiety; this cannot be generalised [OPC].

**What this project would need.** Veteran status as a person-given fact, dated, with a source, for referral and eligibility. It is not an inferred label. Another country's service is a pack.

### Children and young people

- NICE: individual trauma-focused CBT for children and young people with PTSD (1.6.11, in its stated terms) and no drug treatments for prevention or treatment (1.6.14) [NG116]. WHO rates evidence for individual trauma-focused CBT and EMDR as moderate for children [WHO].
- A competent young person is owed confidence, including from a parent (see [protected information](../access-control/protected-information.md)).
- **England, new.** The Crime and Policing Act 2026 (Royal Assent 29 April 2026) creates a duty on people in regulated activity with children, including healthcare professionals, to report child sexual abuse, with no commencement date in the source I read; guidance is promised [NSPCC]. A related offence covers deterring a report. This changes what a clinician may have to do with a disclosure, so it changes what a record must log (my reasoning). I did not read the Act.

**What this project would need.** A record of a report made, by whom, when, to whom. Commencement and scope in a pack. A proxy rule that narrows with age. A young person's therapy must not appear in a parent's view.

### Children's and third-party information in the notes

- Trauma notes are full of other people: a child, a perpetrator, a partner, a sibling. ICO guidance says third-party rules apply before health information is disclosed, and withholding without consent is usually reasonable where a duty of confidence applies [ICO3].
- In England a controller is exempt from disclosing health data on access where that would likely cause serious harm to a person's physical or mental health. A non-health professional needs a professional's opinion, dated within six months [ICO3]. This is meant for exceptional cases (practitioner summary) [ICO3].
- A parent's record may name a child, and a child's a parent. A child reading their own notes at 18, or a parent reading a child's, is a known risk (my reasoning; I did not read DPA 2018 Schedule 2 paragraph 16).

**What this project would need.** A third-party mark on content. An access-request flow that lists third-party and serious-harm review as steps by a health professional. Export follows it ([data exchange](../data-exchange/index.md), DE-H5).

### Session notes: what is shared

- **United States.** Under HIPAA, "psychotherapy notes" are defined narrowly and must be kept separate from the rest of the record. Summaries of diagnosis, treatment plan, symptoms and progress are excluded. Disclosure generally requires a specific authorisation [HIPAA]. So the protection follows the separation.
- **England.** I found no equivalent class in law. Culture of care supports people seeing their notes [CULT]. These two ideas differ: one protects a note from others, the other opens it to the person.
- **Practice.** Notes for a treating team, a formal letter, and a therapist's own working notes differ. BACP's journal reports practitioners focusing notes on continuity, concerns and observations, not a full transcript [BACP]. This is what practitioners say, not a rule, and this page gives no instruction.

**What this project would need.** A note class with a purpose, an author and a visible audience. The default audience of a therapy note is narrow. `PSYTHPN` is a candidate label. What crosses to the wider team is a separately written summary, by the therapist, as a new item. The two are linked but not copied.

### Court and police access to therapy notes

The law here has changed recently. This is a lay reading.

- **England and Wales: the statute.** Section 28 of the Victims and Prisoners Act 2024 inserts Chapter 3A into Part 2 of the Police, Crime, Sentencing and Courts Act 2022. legislation.gov.uk shows it in force from 12 January 2026 [VPA]. A victim information request must be necessary and proportionate for a reasonable line of enquiry, in writing, with notice to the person, subject to exceptions [VPA] [S44]. A counselling request needs more: the information must be likely to have substantial probative value [S44]. Regulations define counselling services (SI 2025/1168) [S44].
- **The code (12 January 2026).** It starts from "the presumption that requests for counselling information are not necessary and proportionate" (para 83), with senior approval, such as a Chief Inspector (para 93). Counselling covers paid or voluntary "psychological, therapeutic or emotional support", from registered clinicians to unregistered advisers (paras 79, 80). It says there is "no standardised method" for such notes (para 81). Medical records are third-party material; a request for "the lifetime records of a victim" is "highly unlikely" to pass (para 77). The holder "is the legal owner of the information" and is not obliged to release under those Acts, but should consider its own lawful basis (paras 126, 130). It may be asked to preserve material (para 129), attach conditions (para 132), or allow viewing on site (para 134). A court order or summons is a last resort (para 127) [VIC]. The code is not itself legally binding (para 29), though a court may take it into account [VIC].
- **Disagreement.** BACP says the code is not as robust as the law requires and wants more protection; campaigners want a judge to approve requests [BACP] [EVAW]. The law was not changed to that by anything I found. I found nothing on counselling records in the Crime and Policing Act 2026 [CPA], but I did not read the Act.
- **Prosecutors.** CPS guidance now reflects the higher test [CPS]. One CPS page says pre-trial therapy guidance was removed after the Home Office code, and another page still discusses it. This is unresolved [CPS].
- **Effect on writing.** BACP's journal reports that therapists should not change note-taking because of possible proceedings, and that notes are written "for clinical purposes and not criminal proceedings" [BACP]. Yet the possibility of disclosure affects what is written, by that article's own existence (my reading).
- **New South Wales.** A sexual assault communications privilege covers "protected confidence" counselling communications, limits disclosure early, and has funded representation for those whose records are sought [NSWP].
- **Canada.** Criminal Code section 278 provides a two-step process for third-party records; they are described as presumptively not produced [CAN]. Advocates still criticise it [CAN].
- **United States.** Psychotherapy notes as above. State and federal privilege were not read.

**What this project would need.** A disclosure request record: who asked, the legal basis given, the scope, the senior approval, the decision, what was released, when, and whether the person was told. Release of the named item only, never a whole record, with a template that resists "the lifetime record". A preservation mark, which an append-only record gets almost for free. Supervised viewing as an option. Every release is a route in the label census. The record should tell a writer plainly who may read a note, before it is written. A note cannot be withdrawn afterwards, as the record has no delete path. That is the sharpest place where an append-only record and a survivor's interests pull apart. It needs a lawyer and survivors, not a design from here.

## Part two: physical major trauma

### Networks and registries

- **England.** Trauma networks went live from April 2012 after a National Audit Office report of 2010 (secondary source) [NET]. A 2023 NHS England specification describes 22 networks, each with major trauma centres, trauma units and local emergency hospitals, covering the whole pathway from injury to rehabilitation [SPEC]. Its metrics include 48-hour repatriations and data submission rates [SPEC].
- **Registry.** The Trauma Audit and Research Network, run from the University of Manchester since about 1990, passed to NHS England in 2023 and relaunched in April 2024 as the National Major Trauma Registry (NMTR). It covers England, Wales, Northern Ireland and Ireland, is rolling out in phases, and collects patients with a stay over three nights or critical care, including transfers [NMTR]. One source ties a pause in data entry to a 2023 cyberattack [NMTR2]. NICE replaced TARN with NMTR in its guideline in May 2025 [NG40U]. A data provision notice requires submission [NMTR2]. The 2023 specification still says TARN [SPEC].
- **United States.** The American College of Surgeons' National Trauma Data Standard is the data dictionary for the National Trauma Data Bank and TQIP. It has over 80 core and over 30 TQIP elements; the 2026 edition applies to 2026 admissions, and a 2027 edition is listed [NTDS].
- **Australia and New Zealand.** A bi-national minimum dataset of 67 fields since 2010 underlies the Australian Trauma Registry, and New Zealand's national minimum data set aligns with it where possible [ATR].

**What this project would need.** Registry extract as a pack per country, with a versioned dictionary and a null reason. The EPR stores facts; the registry derives. Registry submission is not clinical care, so its access is separate. A registry's rule changes year by year, as the dictionary shows.

### The unknown patient and temporary identity

- **The English alert.** NHS/PSA/RE/2018/008 (5 December 2018) says emergency departments often care for people unable or unwilling to give identity. It warns against similar names such as "unknown male", sequential numbers and identical dates of birth. It recommends a random first and second name from an edited phonetic alphabet, such as Foxtrot Whisky; a random seven-digit number with a trust code prefix; sex; and an estimated date of birth formed as 1 January of an estimated year. It says 01Jan1900 is rejected by pathology systems and misleads normal ranges. It asks that all IT systems accept these formats, and for "a robust system for merging medical records once a patient's identity is confirmed" [ALERT]. It also says numbers unique locally may not suit a transfer [ALERT].
- **Elsewhere.** Not researched.

**Compared with this project.** This is the sharpest finding on this side. The [matcher spec](../../electronic-patient-record-matcher-with-rust/spec/index.md) treats a name part that folds to `unknown`, `unidentified`, `anonymous`, `unk` or `unkn` as a placeholder, so absent evidence blocks an auto-link. That is the very convention the alert says to avoid. A name made by the alert's safer method, such as Foxtrot Whisky, is not a placeholder. The matcher will read it as a real name, and read the estimated 1 January date as a real birth date. Two consequences (my reading of the spec; not run):

1. The absent-evidence protection does not fire for the convention NHS England recommends.
2. A year-only date, which `valid_birth_date` allows, would score by shared precision and might hold back an auto-link (not tested). But the alert's format is a full date.

Random names make a collision unlikely (my reasoning). Even so, a temporary identity that the system cannot tell from a real one is a hazard of its own.

**What this project would need.** An explicit `temporary identity` state on a person, not a guess from the name. It requires a pack for number format. It is displayed always. Today `family_name_required` is met by the second phonetic word, so nothing marks the record as temporary. Merge exists: `links.rs` makes a link, never a rewrite, and "the target's allergies include the source's". It needs testing for a temporary record. Each result, blood record and image must travel by the link.

### Mass-casualty and major-incident identity

- The alert says several unidentified patients may arrive together, and that debriefs after mass casualty events showed variation in local practice [ALERT].
- **Manchester Arena.** A study using the national trauma registry found that scene records could not be retrieved once patients reached emergency departments, as the folding triage card system did not carry information into hospital care. It recommends individual patient identifiers and an agreed core dataset, with linked records from scene to hospital. Hospital information was gathered afterwards [ARENA]. The Saunders inquiry report was not read for this [ARENA].
- **NHS England.** The concept of operations (uploaded 2018; may be superseded) makes health organisations responsible for reporting casualty numbers, and states police report fatalities [CONOPS]. The burns annex asks for an appropriate tracking system and longer-term tracking [CONOPS]. The NHS Major Incident Triage Tool replaced earlier tools, with rollout due by 30 June 2024 [CONOPS].
- **Not found.** How hospitals reconcile records with police casualty bureaux; family enquiry; and any guidance on bulk registration.

**What this project would need.** Fast bulk temporary registration with one action, offline if needed, and a link from an ambulance or scene number. Rate limits must not refuse clinicians during a surge (H-039). Downtime is the normal state ([manual steps](../manual-steps/index.md)). Protect the list of who is here from enquiries. A drill mode that exercises this with synthetic data.

### Pre-alert, handover and trauma team activation

- **Pre-alert.** RCEM learning material says NICE recommends an ATMISTER format for pre-alerts: age and sex, time, mechanism, injuries, signs, treatment, ETA and requirements. The ATMIST part then largely forms the handover [ATM]. I did not see that wording in NICE itself, as NICE pages returned 403. Scottish network guidance gives a short bedside handover with an identifiable team leader, and says the pre-alert form and ambulance record go into the patient's notes [ATM].
- **Team.** NG40 includes a tiered team response in major trauma centres and immediate multispecialty activation in trauma units; for patients needing critical interventions at a major trauma centre, leaving the sending department within 30 minutes of the decision to transfer [NG40]. Search results only.
- **Scribes.** Not researched.

**What this project would need.** A pre-alert that exists before the patient does: an expected arrival, tied to a temporary identity on arrival, by a person. The form is fixed by a pack. Times matter here. As with [observations](../suicide-watch/index.md), a resuscitation note is often written after the event, so both clocks must show. Activation level and team roster are facts. A team leader is a role for this encounter.

### Trauma scoring: a data item

- The Injury Severity Score is derived from Abbreviated Injury Scale grades across six body regions; major trauma is commonly defined as above 15. It is mainly used in research and benchmarking; it loses discrimination at higher scores and older ages, and the threshold shifts between AIS versions. An unknown injury severity gives an unassigned code, not a number [ISS]. Who codes it and when was not confirmed; a registry dictionary would settle that [ISS].

**What this project would need.** ISS only as a data item, held with its AIS version, for the registry. Not a bedside prompt. Never zero when absent. Never a decision on who gets a pathway.

### Massive transfusion and the wrong-patient risk

- The alert says a unique identity helps prevent results going to the wrong patient and "fatal ABO incompatible blood transfusion". It cites SHOT's 2017 report of 115 errors involving incorrect patient identification [ALERT]. SHOT's 2016 report has a case of an unknown patient receiving blood labelled for another unknown patient [SHOT]. Wrong blood in tube was the largest share of near misses in SHOT 2019 [SHOT]. A 2019 case was also found where the name did not match [SHOT].
- British guidance says transfusion sample tubes must never be pre-labelled, and that the lab should get patient details as soon as possible for traceability after emergency group O units [BSH]. Canadian and Australasian guidance covers emergency release [BSH]. These are for clinicians; none is restated here.
- **Not found.** Anything on identifiers changing mid-episode. It is the gap that matters most, and it is my inference that it matters.

**What this project would need.** Transfusion is not built, and it may be outside the record's scope; a laboratory system and a bedside system hold most of it. If the EPR holds any of it, the identity on a band, a sample and a unit must be checked by the receiving system. A merge during a transfusion must not re-label anything already in use. After a merge, a blood group must not be copied across without a check. This needs a transfusion practitioner and a laboratory.

### Rehabilitation prescription

- NICE NG211 (January 2022) says people meeting major trauma criteria should have a rehabilitation assessment and prescription during admission, and suggests a named coordinator or key worker within 72 hours, the time limit for starting a prescription [NG211]. It covers adults and children.
- **Not found.** The current data rule and any copy-to-patient requirement.

**What this project would need.** A rehabilitation prescription as a pathway document with an owner, a copy for the person, and a next step after discharge. Discharge never closes it ([5.2.1](../05-domain-model.md)).

### Transfer between hospitals

- Metrics include repatriation within 48 hours [SPEC]; NG40 sets 30 minutes for time-critical transfers [NG40]. A temporary number may not survive a move [ALERT].

**What this project would need.** Identity carried with the temporary number, a transfer summary, images, the pathway owner, a loop-back that closes on receipt ([closed-loop referral](../interoperability/closed-loop-referral.md)), and a named clinician at each end.

### From physical to psychological

- PTSD after injury is common but estimates vary: 10 to 20 percent in a review, 26 percent screened positive in a US centre at about 86 days, 45 percent in one regional registry in the first year, and 4.8 percent at 12 to 15 years in one study [PTIS]. A UK study reports almost 80 percent of those screening positive had no formal diagnosis [PTIS]. No standard screening approach exists, says one review; the American College of Surgeons has a statement [PTIS].
- **Critical care.** NICE CG83 and QS158 ask for review at 2 to 3 months for those at risk [CG83]. Post-intensive care syndrome is put at 25 to 50 percent, and evidence on follow-up clinics and diaries is mixed [CG83]. Diary studies conflict, so a diary feature should not be built on a claim of benefit.
- **Self-harm.** A person treated for injury after self-harm needs both. See [suicide watch](../suicide-watch/index.md), and the acute-hospital findings there.
- **Staff.** Staff also witness trauma; HSSIB reports this [HSSIB24].

**What this project would need.** A pathway step for psychological review after major injury or critical care, with an owner, offered and declined as facts. Screen results as data. The trauma network and mental health team must see the step, without exposing content across services by default ([care settings](../care-settings.md), rule 4).

### Violence, police and protected information

Link: [protected information](../access-control/protected-information.md), including the alias and no-information model, GMC guidance on gunshot and knife wounds, and `VIO` and `SDV`. Added here:

- **An assault patient may be at risk on the ward.** A temporary identity and an alias look alike and differ. One hides who someone is from staff. The other hides it from visitors. They must not share a mechanism (my reasoning).
- **Information sharing to tackle violence (Cardiff Model).** Anonymised emergency department data on date, time, precise location, weapon and relationship to the attacker, usually recorded at reception, shared monthly with authorities under the Serious Violence Duty; RCEM says it will be collected through the Emergency Care Data Set [ISTV]. Much of this violence goes unreported to police, say sources [ISTV]. The evidence claims come mainly from the model's advocates [ISTV]. Free text about a location could identify a person (my reasoning).
- **Receptionists** would record this; `Receptionist` sees no clinical content, so the policy table would need a decision.
- **Evidence and police at the bedside.** Not researched.

## Compared with this project

"Verified" names what I read in the code on 2026-11-04.

| Need | Today | Gap | Verified against |
|---|---|---|---|
| Temporary identity state | None. A family name is required; `birth_date` may be year or year-month | No state; a phonetic name reads as real | `patients_store.rs` (`family_name_required`, `valid_birth_date`) |
| Placeholder names | Matcher folds `unknown` and similar to absent | Matches the convention the alert warns against; not the alert's format | [matcher spec](../../electronic-patient-record-matcher-with-rust/spec/index.md) |
| Merge | Link, never a rewrite; reversible; allergies follow; reason required | Other data classes not built; merge during a transfusion unconsidered | `links.rs` |
| Mass casualty registration | Duplicate check with a reasoned override; no bulk path | Bulk, offline | `controllers/patients.rs`; [manual steps](../manual-steps/index.md) |
| Pre-alert before arrival | None | All of it | grep of `src` |
| Times in resuscitation | Two times on vitals: `effective_at`, `recorded_at` | Only for vitals | `observations_store.rs` |
| Pathway, rehabilitation, follow-up | Direction only; referral loop built | Pathway; timers beyond referrals | spec [5.2.1](../05-domain-model.md), `referral_timeouts.rs` |
| Medicines, transfusion | Not built | Whole area | `tasks.md` RC-5; grep of `src` |
| Labels | Six: `R`, `V`, `PSY`, `HIV`, `SUD`, `ETH` | No `SDV`, `VIO`, `PSYTHPN`; `ETH` defect (H-045) | `policy.rs` `Label` |
| Label on records | None (SE-4) | Nothing is masked in practice | [protected information](../access-control/protected-information.md) |
| Fine-grained survivor control | Consent permits by role, purpose, label and time; refusals respected | Named-person access; relationship test on labels | `consent.rs`, `policy.rs` |
| Break-glass | Does not unmask a label | A safety fact in a labelled note stays hidden | `break_glass.rs` |
| Therapy note class, audience | None | All of it | grep of `src` |
| Disclosure request log | Audit logs access; nothing logs a police request | All of it | `audit.rs` (by name), grep of `src` |
| Third-party mark, serious-harm review | None | All of it | grep of `src` |
| Export | [Data exchange](../data-exchange/index.md) direction | Selective release | spec |
| Registry extract | None | A pack per country | grep of `src` |
| Roles | Eleven | No trauma team leader (an encounter fact, not a role), therapist or independent adviser | `policy.rs` `Role` |

I searched `src` and `migration` for trauma, unknown patient, temporary identity, major incident and mass casualty, and found nothing. The word `unknown` appears as a sex value and as an absent reason.

## Candidate hazards

For the Clinical Safety Officer. None is accepted, and none is added to the [hazard log](../../governance/clinical-safety/hazard-log.csv) by this page. Ids are working labels.

- TT-1 A temporary identity is read as a real person, or two unknown patients are mixed, and a result, a medicine or a blood component goes to the wrong one.
- TT-2 A wrong merge, or a merge at the wrong time, moves or loses a blood group, allergy or result.
- TT-3 A sample or a unit is labelled for the wrong person during a major haemorrhage, or an identity change breaks a check.
- TT-4 A temporary number is lost on transfer, so a person's early care is lost.
- TT-5 In a mass casualty event, records from scene to hospital do not link, the system refuses or slows clinicians, or a list of those present leaks.
- TT-6 A pre-alert is attached to the wrong patient or is never linked.
- TT-7 ISS is treated as a clinical indicator, or an absent score is read as zero.
- TT-8 A retrospective resuscitation note reads as contemporaneous.
- TT-9 A rehabilitation prescription or psychological review has no owner after discharge.
- TT-10 Post-traumatic stress goes unrecognised because no step exists, and no one asks.
- TT-11 A survivor's narrative reaches an abuser by a portal, proxy, letter or banner.
- TT-12 A care preference is invisible: a person is restrained, or left with the wrong staff, because the safe summary is masked or absent.
- TT-13 Forensic material mixes with clinical data, breaks a chain of custody, or is disclosed without choice.
- TT-14 A police request leads to over-disclosure: the whole record, an unlogged release, or no notice to the person where notice is due.
- TT-15 Third-party content (a child, a perpetrator) is released through access, export or portal.
- TT-16 Fear of disclosure means a clinician writes less, so safety information is missing (my reasoning, with the ethics source as a pointer [STIG]).
- TT-17 "Told once" becomes "never asked again": a stale or mistranslated history is relied on, or new risk is missed.
- TT-18 An alias and a temporary identity are confused, or a phone answer confirms that a person is here.
- TT-19 A free-text violence location identifies a patient.
- TT-20 A history that differs between tellings is read as an error or as a lack of credibility in a tribunal or court.
- TT-21 A young person's therapy reaches a parent's proxy view; or a statutory report is not recorded.

## Ideas

Proposals only.

1. **A temporary identity state**, set by a person, shown always, in the alert's format through a pack, and honoured by the matcher as absent evidence. Change the matcher spec first, as the rule here is that it owns identity.
2. **A temporary identity test** that registers Foxtrot Whisky with an estimated date, and proves no auto-link and a clear banner.
3. **One-action bulk arrival** with a drill mode using synthetic data.
4. **Expected arrival** for a pre-alert: a small record that becomes a real one by a person's link.
5. **Two-layer trauma history**: a safe summary the person approves, and a restricted narrative.
6. **A therapy note class** with a narrow audience, and a written summary as the only crossing.
7. **A disclosure request record** and a release of named items only.
8. **A third-party mark** with an access-request step.
9. **Psychological review as a default pathway step** in a major trauma template, offered and declinable; clinician-reviewed.
10. **Rehabilitation prescription as a pathway document.**
11. **A registry extract pack**: NMTR, NTDS and the bi-national dataset as three tests of whether the core holds the right facts.
12. **Interpreter record** beside any history.
13. **Keep forensic data out**, with a pointer by agreement.
14. **Add labels `SDV`, `VIO`, `PSYTHPN`** under the [pinned value set](../access-control/protected-information.md).
15. **Add unknown patient and pre-alert rows to [manual steps](../manual-steps/index.md)** later.

## Open questions

1. Should the record hold a survivor's narrative at all, or only a pointer to where it was told?
2. What may a survivor have restricted from an emergency clinician? See OQ-PI2.
3. Does the English position allow a therapy-note class like the US one? Who decides its default audience?
4. Which staff may see a "safe summary"? Is it a clinical fact, or a person's preference?
5. How should a note, once written, respond to a survivor's wish to withdraw it, in a record with no delete path? Restriction is the only tool; is it enough?
6. Who owns and signs a therapy record held in a shared record, when the holder must weigh a police request (para 126 of the code)?
7. How will the 2026 child sexual abuse reporting duty commence, and what must be logged?
8. Is the phonetic format still the right one, and does anyone outside England want it? A pack should not assume it.
9. How does a temporary identity cross to another hospital and the ambulance record?
10. Is transfusion in scope? If not, what must the record refuse to imply?
11. What should a major incident drill prove, and who runs it?
12. Who represents survivors, veterans, refugees and young people in this design?

## Not verified

- **NICE site pages returned 403.** NG116 was read from a third-party copy of a 2019 print: numbers and words may differ today. NICE's page reports a last review on 23 July 2026, which was not read. NG39, NG40, NG211, CG83 and QS158 were seen only through search results.
- **Law.** Everything is a lay reading. I read the code and section 28 through fetched summaries, and the page's in-force date is as shown there. Sections 44A to 44E were seen as search results, not read. The remaining twelve thousand characters of the code were not read. The Crime and Policing Act 2026 was not read. NSW, Canada and HIPAA come from secondary sources.
- **CPS guidance** conflicts between its own pages, as above; the current position is unknown.
- **SARCs.** The service specification is dated 2018 with data to 2022 and may be superseded. FFLM guidance and the FSR code were not read. No source was found on separation from GP records.
- **WHO, ICD-11 and SAMHSA.** I did not read the primary pages. The Four Rs wording comes from a SAMHSA extract, and part of it was not displayed in the summary; check it.
- **Retelling.** The only source on telling in a safe context is low quality. No trial was found.
- **PTSD prevalence** figures come from different populations and instruments, as search summaries. They are not comparable.
- **The 22 networks** come from a 2023 specification. Its text still uses TARN.
- **ATMISTER** as a NICE recommendation is from a secondary source. The Scottish material is local.
- **SHOT** items are search snippets from reports of different years. BSH guidance was seen as summaries.
- **Manchester Arena.** The registry study was seen as a search summary. PubMed Central blocked direct reads.
- **The matcher finding** comes from reading the spec. It was not run.
- **Not researched:** scribes in resuscitation, evidence and police at the bedside, casualty bureaux, US and Australian unknown-patient practice, the VA/DoD guideline, Freedom from Torture, DPA 2018 Schedule 2 paragraph 16, and US privilege law.
- **Code:** read and grepped, not run. No test was added.
- Link check 2026-11-04: these cited pages returned 404. They may have moved, but what rests on them is unconfirmed: https://www.england.nhs.uk/wp-content/uploads/2018/03/concept-operations-management-mass-casualties.pdf

## Sources

Read 2026-11-04. Marks: **(fetched)** the page was opened and summarised by the fetch tool; **(PDF text)** text extracted and read directly; **(search summary)** from a search tool's summary; **(snippet)** a search result only.

- [NG116] NICE NG116, Post-traumatic stress disorder, December 2018, third-party copy of a 2019 print: https://traumapsychology.co.uk/wp-content/uploads/2023/03/NICE-2018.pdf (PDF text)
- [NG116P] NICE NG116 overview: https://www.nice.org.uk/guidance/ng116 ; recommendations, 403: https://www.nice.org.uk/guidance/ng116/chapter/recommendations (search summary)
- [WHO] WHO, Guidelines for the management of conditions specifically related to stress: https://apps.who.int/iris/bitstream/handle/10665/85119/9789241505406_eng.pdf ; conditions related to stress: https://www.who.int/teams/mental-health-and-substance-use/treatment-care/mental-health-gap-action-programme/evidence-centre/conditions-related-to-stress (search summary)
- [ICD] Brewin on ICD-11 complex PTSD: https://resolve.cambridge.org/core/product/E53B8CD7CF9B725FE651720EE58E93A4/core-reader ; a BJPsych Advances article: https://core-prod.cambridgecore.org/core/services/aop-cambridge-core/content/view/2977140CBDAAF402610715BB609F688C/S2056467819000483a.pdf/complex_posttraumatic_stress_disorder_a_new_diagnosis_in_icd11.pdf (search summary)
- [OHID] OHID, Working definition of trauma-informed practice, 2 November 2022: https://www.gov.uk/government/publications/working-definition-of-trauma-informed-practice/working-definition-of-trauma-informed-practice (fetched)
- [SAMHSA] SAMHSA, SMA14-4884: https://library.samhsa.gov/product/samhsas-concept-trauma-and-guidance-trauma-informed-approach/sma14-4884 ; CDC principles: https://stacks.cdc.gov/view/cdc/138924/cdc_138924_DS1.pdf (search summary)
- [CULT] NHS England, Culture of care standards: https://www.england.nhs.uk/long-read/culture-of-care-standards-for-mental-health-inpatient-services/ (fetched)
- [HSSIB24] HSSIB, Patients at risk of self-harm: continuous observation: https://www.hssib.org.uk/patient-safety-investigations/patients-at-risk-of-self-harm-continuous-observation/investigation-report/ (fetched)
- [HSSIB25] HSSIB, Mental health inpatient settings: https://www.hssib.org.uk/patient-safety-investigations/mental-health-inpatient-settings/fifth-investigation-report/ (fetched)
- [SSS] NHS England, Staying safe from suicide, copy: https://www.infocop.es/wp-content/uploads/2025/06/NHS-England-Staying-safe-from-suicide.pdf (PDF text)
- [SARC] NHS England, National service specification for SARCs: https://www.england.nhs.uk/wp-content/uploads/2018/04/PRN00577-national-service-specification-sexual-assault-referral-centres-updated.pdf (PDF text, relevant passages)
- [SARCLOCAL] Devon and Cornwall SARC: https://sarchelp.co.uk/professionals/guidance-patient-disclosure/ (fetched); https://sarchelp.co.uk/sarcs/what-is-a-sarc/ (snippet)
- [FFLM] FFLM, SARC storage of forensic samples, July 2024: https://fflm.ac.uk/wp-content/uploads/2024/07/SARC-Storage-of-Forensic-Samples-the-Human-Tissue-Act-Jul-24.pdf (snippet)
- [IP] OHCHR, Istanbul Protocol launch: https://www.ohchr.org/en/press-briefing-notes/2022/06/istanbul-protocol-launch-revised-version-international-guidelines ; PHR summary: https://phr.org/wp-content/uploads/2022/12/The-Istanbul-Protocol-2022-Edition-Summary-of-Changes-PPT.pdf (search summary)
- [OPC] NHS England, Op COURAGE: https://www.england.nhs.uk/2021/03/nhs-launches-op-courage-veterans-mental-health-service/ ; providers: https://www.tewv.nhs.uk/op-courage-the-veterans-mental-health-and-wellbeing-service/ and https://www.northlondonmentalhealth.nhs.uk/news/london-veterans-encouraged-to-access-nhs-mental-health-support-through-op-courage-8099 ; a study: https://researchinnovation.kingston.ac.uk/en/publications/trauma-exposure-and-clinical-presentation-of-uk-veterans-seeking-/ (search summary)
- [988] SAMHSA, 988 FAQ: https://www.samhsa.gov/mental-health/988/faqs (search summary)
- [NSPCC] NSPCC, Mandatory reporting, May 2026: https://learning.nspcc.org.uk/news/2026/may/mandatory-reporting-child-sexual-abuse-england (fetched)
- [CPA] Crime and Policing Act 2026: https://bills.parliament.uk/bills/3938 (snippet)
- [ICO3] ICO, Health information: https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/individual-rights/right-of-access/health-information/ ; a practice briefing: https://ig.n3i.co.uk/wp-content/uploads/2025/01/N3i-Info-Gov-22-Serious-Harm-Test.pdf (search summary)
- [HIPAA] HHS FAQ: https://www.hhs.gov/hipaa/for-professionals/faq/2088/does-hipaa-provide-extra-protections-mental-health-information-compared-other-health.html ; a law firm: https://www.hollandhart.com/hipaa-psychotherapy-notes-and-other-mental-health-records (search summary; the regulation itself was not read)
- [BACP] BACP Therapy Today, December 2025: https://www.bacp.co.uk/bacp-journals/therapy-today/2025/articles-december/the-big-issue/ (fetched); July 2025: https://www.bacp.co.uk/news/news-from-bacp/2025/7-july-complete-confidentiality-must-be-guaranteed-for-survivors-accounts-of-sexual-offences/ (snippet)
- [VPA] Victims and Prisoners Act 2024, section 28: https://www.legislation.gov.uk/ukpga/2024/21/section/28 (fetched)
- [S44] Counselling services regulations: https://www.legislation.gov.uk/uksi/2025/1168/made ; Hansard, 1 December 2025: https://hansard.parliament.uk/Commons/2025-12-01/debates/25120117000020/VictimInformationRequestsCodeOfPractice (search summary)
- [VIC] Home Office, Victim information requests, code of practice: https://www.gov.uk/government/publications/victim-information-requests-code-of-practice--2/victim-information-requests-code-of-practice-accessible (fetched, about nine-tenths)
- [CPS] CPS, Rape and sexual offences guidance: https://www.cps.gov.uk/prosecution-guidance/rape-and-sexual-offences-prosecution-guidance-full-page-version ; child sexual abuse: https://www.cps.gov.uk/prosecution-guidance/child-sexual-abuse-guidelines-prosecuting-cases-child-sexual-abuse (search summary)
- [EVAW] End Violence Against Women: https://www.endviolenceagainstwomen.org.uk/campaign-win-government-changes-law-to-help-keep-rape-survivors-counselling-notes-confidential/ (search summary; campaign)
- [NSWP] NSW Judicial Commission bench book: https://judcom.nsw.gov.au/publications/benchbks/criminal/sexual_assault_communications_privilege.html ; Legal Aid NSW: https://publications.legalaid.nsw.gov.au/PublicationsResourcesService/PublicationImprints/Files/518.pdf (search summary)
- [CAN] Office of the Federal Ombudsperson for Victims of Crime: https://www.canada.ca/en/office-federal-ombudsperson-victims-crime/sissa-essas/therapeuticrecords.html ; a Nova Scotia decision: https://decisions.courts.ns.ca/nsc/nssc/en/492356/1/document.do (search summary)
- [NET] Time to definitive care in major trauma networks: https://academic.oup.com/bjsopen/article/4/5/963/6136103 ; NICE NG39 full guideline: https://www.nice.org.uk/guidance/ng39/evidence/full-guideline-pdf-2308122833 (search summary)
- [SPEC] NHS England, Major trauma clinical network specification, 2023: https://www.england.nhs.uk/wp-content/uploads/2024/03/PRN231106-major-trauma-network-specification-2023.pdf (PDF text, relevant passages)
- [NMTR] NHS England, National Major Trauma Registry: https://www.england.nhs.uk/outcomes-and-registries-programme/nmtr/ (fetched)
- [NMTR2] Data provision notice: https://digital.nhs.uk/about-nhs-digital/corporate-information-and-documents/directions-and-data-provision-notices/data-provision-notices-dpns/national-major-trauma-registry-nmtr ; a report on the pause: https://d7g406zpx7bgk.cloudfront.net/x/cbce911363/major-trauma-audit-report-focused-on-older-adults-2017-2021-final.pdf (search summary)
- [NG40U] NICE NG40 update information: https://www.nice.org.uk/guidance/NG40/chapter/update-information (search summary)
- [NG40] NICE NG40, Major trauma: service delivery: https://www.nice.org.uk/guidance/ng40/chapter/recommendations (search summary; fetch 403)
- [NTDS] American College of Surgeons, National Trauma Data Standard: https://www.facs.org/quality-programs/trauma/quality/national-trauma-data-bank/national-trauma-data-standard/ ; 2026 dictionary: https://www.facs.org/for-medical-professionals/news-publications/news-and-articles/acs-brief/november-25-2025-issue/download-free-2026-trauma-data-dictionary/ (search summary)
- [ATR] Bi-national minimum dataset: https://openresearch-repository.anu.edu.au/entities/publication/4d8411f9-6f88-4b65-b19c-d1cb2c4f98f2/full ; New Zealand: https://www.majortrauma.nz/nz-mtr/national-minimum-data-set-for-trauma (search summary)
- [ALERT] NHS Improvement, Patient Safety Alert NHS/PSA/RE/2018/008: https://www.england.nhs.uk/wp-content/uploads/2019/12/Patient_Safety_Alert_-_unknown_or_unidentified_patients_FINAL.pdf (PDF text)
- [ARENA] Manchester Arena registry study: https://pmc.ncbi.nlm.nih.gov/articles/PMC8461451/ (search summary)
- [CONOPS] NHS England, Concept of operations: https://www.england.nhs.uk/wp-content/uploads/2018/03/concept-operations-management-mass-casualties.pdf ; burns annex: https://www.england.nhs.uk/long-read/concept-of-operations-for-the-management-of-mass-casualties-burns-annex/ ; triage tools: https://www.england.nhs.uk/long-read/major-incident-triage-tools/ (search summary)
- [ATM] RCEM Learning, ATMIST(ER): https://www.rcemlearning.org/modules/pre-hospital-communication/lessons/conveying-useful-information/topic/atmister/ ; NHS Scotland network guidance: https://www.rightdecisions.scot.nhs.uk/south-east-scotland-major-trauma-guidelines/hospital-pre-alert-handover/ (search summary)
- [ISS] Injury Severity Score: https://en.wikipedia.org/wiki/Injury_Severity_Score ; review: https://rcm.mums.ac.ir/article_4339.html (search summary; secondary)
- [SHOT] SHOT 2016: https://www.shotuk.org/wp-content/uploads/2025/04/10.-Incorrect-Blood-Components-Transfused-IBCT.pdf ; 2019 near miss: https://www.shotuk.org/wp-content/uploads/2024/10/12a.-Near-Miss-%E2%80%93-Wrong-Blood-in-Tube-WBIT-1.pdf ; 2019 IBCT: https://www.shotuk.org/wp-content/uploads/2024/10/9.-Incorrect-Blood-Component-Transfused-IBCT-1.pdf (search summary)
- [BSH] British Society for Haematology, administration of blood components: https://onlinelibrary.wiley.com/doi/full/10.1111/tme.12481 ; Canadian guide: https://professionaleducation.blood.ca/en/transfusion/clinical-guide/pre-transfusion-testing ; ANZSBT: https://anzsbt.org.au/wp-content/uploads/2025/06/Guidelines-for-transfusion-and-immunohaematology-laboratory-practice_June-2025-PUBLISHED-1.pdf (search summary)
- [NG211] NICE NG211: https://www.nice.org.uk/guidance/ng211/chapter/Context ; rationale: https://www.nice.org.uk/guidance/ng211/chapter/rationale-and-impact (search summary)
- [PTIS] Prevalence: https://pmc.ncbi.nlm.nih.gov/articles/PMC8081481/ ; long term: https://pmc.ncbi.nlm.nih.gov/articles/PMC11414644/ ; UK under-diagnosis: https://www.sciencedirect.com/science/article/pii/S0020138326000641 ; ACS statement: https://www.facs.org/about-acs/statements/ptsd-in-adults/ ; review: https://www.sciencedirect.com/science/article/abs/pii/S0022480420306867 ; screening: https://www.ncbi.nlm.nih.gov/pmc/articles/PMC12093061/ ; registry: https://doi.org/10.1071/HC17091 (search summary)
- [CG83] NICE CG83: https://www.nice.org.uk/guidance/cg83 ; QS158: https://www.nice.org.uk/guidance/qs158/resources/rehabilitation-after-critical-illness-in-adults-pdf-75545546693317 ; review: https://link.springer.com/article/10.1186/s40560-023-00716-w ; diary trial: https://www.sciencedirect.com/science/article/pii/S0163834320301018 (search summary)
- [ISTV] RCEM, Cardiff Model explained: https://rcem.ac.uk/wp-content/uploads/2025/06/The-Cardiff-Model-explained-FINAL.pdf ; Bevan Brittan: https://www.bevanbrittan.com/insights/articles/2025/information-sharing-to-tackle-violence-the-recording-of-data-in-emergency-departments-to-help-reduce-serious-violence/ ; RCN: https://rcnlearn.rcn.org.uk/Search/Emergency-and-Urgent-Care-Topic-Page/Emergency-and-Urgent-Care-Topic-Page/Information-sharing-to-tackle-violence (search summary)
- [STIG] AMA Journal of Ethics: https://journalofethics.ama-assn.org/article/when-and-how-should-clinicians-share-details-health-record-patients-mental-illness/2017-03 (search summary)
- This repository: `electronic-patient-record-service-with-rust/src/policy.rs`, `patients_store.rs`, `links.rs`, `observations_store.rs`, `break_glass.rs`, `consent.rs`, `referral_timeouts.rs`; [matcher spec](../../electronic-patient-record-matcher-with-rust/spec/index.md); [protected information](../access-control/protected-information.md).
