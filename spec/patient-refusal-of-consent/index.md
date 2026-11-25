# A patient's refusal of consent

Researched 2026-11-25 from public sources and from this repository. **A proposal: nothing here is built, and no lawyer, clinician, Clinical Safety Officer, data protection officer, patient or person who has made an advance decision has reviewed it. It is not legal advice.** It separates two refusals that are easily taken for one, checks what the law was said to be, and says what this software should hold, show and never decide.

Related: [life-or-limb emergencies](../access-control/index.md#life-or-limb-emergencies) (FR-4.6), [masking](../access-control/masking.md), [suicide watch](../suicide-watch/index.md), [care homes](../care-homes/index.md), [custodial health](../custodial-health/index.md), [order entry](../order-entry/index.md), tasks EC-5 and EC-6.

## The short answer

1. **Two refusals.** A refusal of **treatment** ("do not give me blood") and a refusal of **disclosure** ("do not show my record"). The brief is about the first. The consent records built here are about the second.
2. **An emergency does not override a capable adult's refusal of treatment**, in England and Wales, and by the sources read in Ontario and before the European Court of Human Rights. It changes what may be done when the person **cannot decide** and no refusal that holds is known.
3. **For disclosure the published rule has the same shape.** The UK doctors' regulator: where a patient objects to information being shared for their own care, do not disclose it unless it is justified in the public interest or is of overall benefit to a patient who **lacks capacity** [GMC].
4. **So the question is not "is it an emergency?" but "can this person be asked, now?"** The software cannot answer that. A clinician can, and has to say so.
5. **This software keeps no refusal of treatment at all today.** That is the larger gap.
6. **The two meet badly.** A refusal of disclosure could hide an advance decision in the emergency that it was written for.

## The two refusals

| | Treatment | Disclosure |
|---|---|---|
| What is refused | An intervention: blood, intubation, a medicine, an operation, admission | Showing some or all of the record, to some people, for some purposes |
| Harm if ignored | An assault on the person; a life lived as they chose not to | A secret told; violence, discrimination, lost trust |
| Harm if wrongly followed | Death or disability of someone who would have accepted | A clinician without a fact that they needed |
| Who decides that it applies | The treating clinician, and a court if there is doubt | The service by rule, and a person in an emergency |
| Held here today | **Nothing** | `consent.rs`: permits and refusals by role, purpose, kind of record and label |
| In an emergency here today | Nothing | Break-glass does **not** lift it (OQ-PI8) |

## What the brief said, checked

| Claim in the brief | Found | Verdict |
|---|---|---|
| A life-or-limb emergency does not override a refusal by an adult with capacity | A capable adult's refusal "must be respected", "even if refusing treatment would result in their death" [NHS]. An adult with capacity may refuse though it leads to death: Re B (2002) [REB] (secondary). Ontario: a refusal on a card bound the doctor when the patient was unconscious, Malette v Shulman (1990) [MAL] (secondary). Strasbourg: blood given in emergency surgery despite her refusal was a violation, Pindo Mulla v Spain (2024) [PM] (secondary) | Right for England and Wales on the sources read |
| "including in the UK" | The Mental Capacity Act is England and Wales. Scotland has its own Act; **in Northern Ireland the 2016 Act is in force only in part**, and what governs advance decisions there was not established [NI] | Too broad |
| Without consent only when the person lacks capacity and care is urgent | Emergency treatment goes ahead for someone who "needs emergency treatment to save their life, but they're incapacitated", explained afterwards [NHS]. Outside that: treatment for mental disorder under the Mental Health Act s.63 [MHA], and children | Right for adults and physical care |
| Capacity is to understand, retain, weigh and communicate | Section 3(1)(a) to (d): understand, retain, "use or weigh", communicate. **It is presumed (s.1(2)), about this matter at this time (s.2(1)), and not lost "merely because he makes an unwise decision" (s.1(4))** [MCA] | Right, with three parts left out that matter most |
| Unconsciousness, shock, intoxication or impairment | Each may remove it. None shows by itself that it has gone: the test is of the four abilities | Right, easily misread as a list |
| Treat in best interests if no valid refusal is known | Best interests (s.1(5)), weighing past and present wishes and any written statement made with capacity (s.4(6)(a)). The protection for care (s.5) does not affect advance decisions (s.5(4)) [MCA] | Right, and an advance decision is stronger than a factor to weigh |
| "a relevant advance decision" | Made at 18 or over with capacity (s.24(1)); withdrawn at any time with capacity, **and not necessarily in writing** (s.24(3) and (4)). For life-sustaining treatment: a statement that it applies "even if life is at risk", in writing, signed and witnessed (s.25(5) and (6)). A person who treats is liable only if satisfied that a valid and applicable one exists (s.26(2)). **While a court is asked, life-sustaining treatment, and what prevents serious deterioration, may be given (s.26(5))** [MCA] | Right, and the conditions are what software has to hold |
| Detention does not cover physical emergencies | Section 63: no consent needed for treatment "for the mental disorder from which he is suffering" [MHA]; "treatment for unrelated physical conditions still requires consent" [NHS]. Changes by the Mental Health Act 2025 are listed as not yet applied, and their dates were not established | Right |
| Under 18 it depends on competence, parental responsibility and law | In an emergency "treatment can proceed without consent" where waiting is a risk; at 16 and 17 a refusal is overruled only in exceptional circumstances [NHSC]. E & F (2021): two competent young people refused blood, and it was lawful to give it; a young person's decision is not determinative [EF] (secondary) | Right |

**Not in the brief, and it matters:** doubt. A doubtful refusal is the common case: a scan of a form with no date, a relative's word, a note that a person "would not want it". The law read here lets a clinician who is **not satisfied** treat. Software that shows a doubtful refusal as a certain one, or a certain one as a note, changes what happens.

## Disclosure in an emergency, elsewhere

| Where | Rule read |
|---|---|
| UK, doctors [GMC] (search extracts, not the page) | Objection holds unless public interest, or overall benefit where capacity is lacking. If a capable patient still objects to what is essential for safe care, explain that the care cannot be given or referred |
| England, national guidance [NHSE] (search extract) | Do not share what a patient specifically objects to for their individual care unless it is in the overriding public interest |
| England, summary record [CPE] (secondary) | If the patient cannot give permission and the clinician judges it necessary, "emergency access". What happens for someone who opted out was **not established**; accounts differ |
| US, HIPAA 45 CFR 164.522 [H522] | A provider need not agree to a restriction. **One it agreed to may be broken where the patient needs emergency treatment and the restricted information is needed for it**, and the receiver must be asked not to use or disclose it further |
| US, substance use, 42 CFR 2.51 [P2] | To medical personnel as far as needed for a "bona fide medical emergency in which the patient's prior written consent cannot be obtained"; then documented at once: who, when, and the nature of the emergency |
| EU, GDPR 9(2)(c) [GDPR] | Vital interests, "where the data subject is physically or legally incapable of giving consent" |
| Australia, My Health Record s.64 [OAIC] | Past a person's access controls where the access is reasonably believed needed for a serious threat and consent is unreasonable or impracticable to obtain. Restricted documents are shown, deleted ones not; recorded; the person can be told; a limit of days |

**They agree on one thing: the emergency route is for someone who cannot be asked.** They differ on whether an earlier refusal then counts.

## What this means for OQ-PI8

OQ-PI8 asks whether a life-or-limb emergency lifts a patient's refusal of **disclosure**. An earlier recommendation here was "yes, for treatment only, unless the refusal says that it holds in an emergency". **That was too wide: it did not ask whether the patient can be asked.** In its place:

| The patient | The refusal | Proposed |
|---|---|---|
| Can be asked | Any | **It holds.** Ask. If they agree now, record a new consent, for now. If they do not, say what cannot be done without it |
| Cannot be asked, and a clinician says so | Says that it holds in an emergency | **It holds**, until a lawyer says otherwise in this place |
| Cannot be asked, and a clinician says so | Does not speak of emergencies | **May be lifted for treatment**, as break-glass: a reason, flagged, reviewed, the patient told afterwards |
| Not known | Any | It holds |
| A child | Any | Not decided here: competence, parents and law differ by place |

Until it is decided, nothing changes: **break-glass lifts no refusal**, which is the safer side for the refusal and the less safe for the patient who would have agreed. Both are hazards. FR-4.6 ("emergencies that are life-or-limb threatening override **normal** access restrictions") is read here as not covering a refusal by someone who can be asked; whether the maintainer meant that is OQ-RF1.

## Rules proposed

1. **Never decide capacity.** The software does not score, suggest or infer it, from a diagnosis, a word in a note, a triage category, a language, an age over 18 or a refusal itself. It records who judged it, when, about what, and why.
2. **Never say "valid" or "binding".** It shows what is recorded, where the paper is, and what is missing from it. A person decides.
3. **Keep the two apart** in the data, the words and the screen. "Consent refused" alone is never shown.
4. **A refusal of treatment is found in seconds or it is not held.** It is on the banner's way, above allergies in an emergency view, and says the treatment and not only "has an advance decision".
5. **It is not hidden by a refusal of disclosure, nor by a label, from someone about to treat.** Only the words needed: the treatment, the condition, the date, where the paper is. The reason may be private. This is an exception to masking that the patient is told of when it is recorded.
6. **Absent is not "no refusal".** "None recorded here" is what is known. Papers are in wallets, homes and other systems.
7. **Never a block.** A refusal does not stop an order. It interrupts once, names the refusal, and asks for what the clinician found: withdrawn, not satisfied that it applies, does not cover this, agreed now. The answer is kept.
8. **Withdrawal is easy and at once**, by word of mouth, by anyone who heard it, and shown as that. Adding one is careful; ending one is not.
9. **It goes stale honestly.** The date it was made and last confirmed are beside it. The software does not expire it.
10. **Doubt is a state, with a reason**, and not a yes or a no.
11. **A refusal now is recorded as care:** what was offered, what was refused, the risks explained, by whom, and that the person may change their mind. The US emergency rule asks for a description of what was refused and reasonable steps for a written refusal [EMTALA].
12. **Nobody is pressed.** No screen words a refusal as a problem to solve. It does not reduce other care, and it is not shown to those who do not need it.
13. **Coercion both ways.** A refusal, or its withdrawal, may be somebody else's. Who was in the room is recordable, and the entry can be kept from a proxy.

## What to hold, proposed

| Field | Why |
|---|---|
| Kind: spoken now, advance decision, advance statement, order by a clinician (for example, no resuscitation), from an attorney or a court | They differ in force, and are often confused |
| The treatment refused, in words and coded if it can be; the circumstances | Section 25 turns on what it covers |
| "Even if life is at risk", stated or not | Needed for life-sustaining treatment in England and Wales |
| Written, signed, witnessed: yes, no, **not known** | Each is a condition; not known is the usual answer |
| Date made; age then, worked out; date last confirmed | Adult at the time |
| Where the paper is; a copy; who saw the original | |
| Recorded by, and how they know | A relative's word is a source, not the decision |
| State: stands, doubted (why, by whom), withdrawn (how), entered in error | Append-only, as everywhere |
| What is accepted | Often as important: "no blood, but albumin" |

Which of these a country requires is a **pack**, with its source and reviewer, as elsewhere in this project. Nothing above is hard-coded.

**Exchange.** FHIR R5 `Consent` has a place for it ("or record of refusal to consent"), but "only the privacy use case is fully modeled" [FHIR]. The US advance directive guide is 1.0.0, trial use, and covers documents written by the person; portable medical orders are left for later [PACIO]. Reading one in is an import with a source, never a fact of this record.

## Candidate hazards

For the Clinical Safety Officer. None is added to the log: nothing is built.

1. A refusal is not seen, and the person is treated against it.
2. A refusal is shown that was withdrawn, is another person's, or does not cover this treatment, and a person who would have accepted dies.
3. A refusal of one treatment is read as a refusal of all care.
4. A refusal of disclosure is read as a refusal of treatment, or the other way.
5. A refusal of disclosure, or a label, hides an advance decision.
6. A merge joins two people, and one has the other's refusal (H-001, H-020). **A merge must ask a person about each one.**
7. An order that a clinician made is shown as the patient's own decision.
8. "None recorded" is read as "none".
9. A clinician takes the interruption for legal advice.
10. A refusal after self-harm is followed, or ignored, without the assessment it needs ([suicide watch](../suicide-watch/index.md)).

## Order of work, proposed

| Step | What | Acceptance |
|---|---|---|
| RF-1 | Decide OQ-RF1 and OQ-PI8 | A recorded decision, with a lawyer's view |
| RF-2 | Words: "refusal of treatment" and "refusal of disclosure" in the glossary and wherever consent is named | A census of screens and API words |
| RF-3 (M, clinical) | A pure library for a recorded refusal of treatment: the fields, states and what is missing; no "valid" | Domain tests; no function answers "may I treat?" |
| RF-4 (M, clinical) | The store and API, with a merge that asks | Tests for each hazard that has one |
| RF-5 (M, clinical) | The emergency view and banner | Tested with clinicians against time |
| RF-6 (S, clinical) | Break-glass asks whether the patient can be asked (with PI-10) | The grant and its review hold it |
| RF-7 (S, clinical) | Refusal now, and leaving against advice (EC-6) | One record shape for both |

Order entry does not exist, so rule 7 waits for it.

## Open questions

Summarised as OQ-RF1 to OQ-RF6 in [open questions](../16-open-questions.md).

1. Does FR-4.6 cover a refusal of disclosure by someone who can be asked? Recommendation: no. (OQ-RF1)
2. May an advance decision be shown to someone about to treat, though the patient refused disclosure of the rest? Recommendation: yes, and tell the patient when it is recorded. (OQ-RF2)
3. Does the first deployment hold advance decisions, or show where another system holds them? Two copies can disagree. (OQ-RF3)
4. Who may record a withdrawal that they were told of, and does it take effect at once? Recommendation: any treating clinician, at once, shown as spoken. (OQ-RF4)
5. Under 18: what is shown of a competent young person's refusal, and to whom? (OQ-RF5)
6. Which people are asked before a build: people who have made an advance decision, people who refuse blood, people with fluctuating capacity, families, emergency clinicians? (OQ-RF6)

## Not done, not verified

- **The regulator's pages, the case-law site, the Ontario judgment, the England summary-record page and the Council of Europe treaty refused automated reading and were not read.** What is said of them is from search extracts or other sites, and is marked secondary.
- Re T, Re MB, Hunter and New England v A, the Mental Health Act 2025's dates, section 25(2) to (4) of the 2005 Act, the code of practice, the law on attorneys and courts, and any case after 2024 were not read.
- The Mental Capacity Act sections, Mental Health Act s.63, the Scottish section 47, HIPAA 164.522, 42 CFR 2.51, 42 CFR 489.24 and the GDPR article were read at their source, or at a published copy of it. The EMTALA page notes that the paragraph's record-keeping waits for an approval that was not checked.
- One figure found and not used as fact: a US study is reported to find capacity documented in 22 per cent of charts of refused emergency care (search extract).
- Scotland, Northern Ireland, Wales's own policy, and every other country need their own lawyer.

## Sources

Read 2026-11-25.

- [MCA] Mental Capacity Act 2005, sections 1 to 5 and 24 to 26: https://www.legislation.gov.uk/ukpga/2005/9/contents
- [MHA] Mental Health Act 1983, section 63: https://www.legislation.gov.uk/ukpga/1983/20/section/63
- [NHS] Consent to treatment: https://www.nhs.uk/conditions/consent-to-treatment/
- [NHSC] Consent to treatment, children and young people, reviewed 8 December 2022: https://www.nhs.uk/conditions/consent-to-treatment/children/
- [SC] Adults with Incapacity (Scotland) Act 2000, section 47: https://www.legislation.gov.uk/asp/2000/4/section/47
- [NI] Mental Capacity Act (Northern Ireland) 2016, section 307: https://www.legislation.gov.uk/nia/2016/18/section/307
- [GMC] General Medical Council, Confidentiality, direct care (search extract): https://www.gmc-uk.org/professional-standards/the-professional-standards/confidentiality/using-and-disclosing-patient-information-for-direct-care
- [NHSE] Consent to using and sharing patient information (search extract): https://www.england.nhs.uk/long-read/consent-to-using-and-sharing-patient-information/
- [CPE] Summary Care Record, for pharmacies (secondary): https://cpe.org.uk/digital-and-technology/electronic-health-records/summary-care-record-scr/
- [REB] Re B (2002), a chambers' summary (secondary): https://www.4pb.com/case-detail/re-b-consent-to-treatment-capacity/
- [EF] E & F [2021] EWCA Civ 1888, summary (secondary) https://www.39essex.com/our-thinking/cases/ak-case-1-e-f-minors-blood-transfusion-2021-ewca-civ-1888/ ; the judgment (not read): https://caselaw.nationalarchives.gov.uk/ewca/civ/2021/1888
- [MAL] Malette v Shulman (1990) (secondary): https://www.globalhealthrights.org/malette-v-shulman-et-al/
- [PM] Pindo Mulla v Spain (2024), legal summary (search extract): https://biodiritto.org/ocmultibinary/download/4702/54854/2/41025e227c9001d8da340c06f53680ae/file/Legal+summary+Pindo+Mulla+v.+Spain+%5BGC%5D.pdf
- [H522] 45 CFR 164.522: https://www.law.cornell.edu/cfr/text/45/164.522
- [P2] 42 CFR 2.51: https://www.law.cornell.edu/cfr/text/42/2.51
- [EMTALA] 42 CFR 489.24(d)(3): https://www.law.cornell.edu/cfr/text/42/489.24
- [GDPR] Article 9(2)(c): https://gdpr-info.eu/art-9-gdpr/
- [OAIC] My Health Record emergency access (search extract): https://www.oaic.gov.au/privacy/privacy-guidance-for-organisations-and-government-agencies/health-service-providers/my-health-record/my-health-record-emergency-access-function
- [FHIR] Consent, R5: https://hl7.org/fhir/R5/consent.html
- [PACIO] Advance directive interoperability 1.0.0: https://hl7.org/fhir/us/pacio-adi/
