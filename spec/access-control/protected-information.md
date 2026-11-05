# Protected information

Public-source research read 2026-11-04 plus this repository's own files. A proposal and a reading, not legal or clinical advice: nothing here is built, and no lawyer, clinician, data protection officer or person with lived experience of these categories has reviewed it.

Related: [Roles and assignments](roles-and-assignments.md) (the companion page: who holds access and how it ends), [Access control and consent](index.md), [Audit](../audit/index.md), [Patient identity](../patient-identity/index.md), [Maternity](../maternity/index.md), [Care homes](../care-homes/index.md), [Data exchange](../data-exchange/index.md), the [DPIA](../../governance/dpia.md). **NOT FOR CLINICAL USE.** Examples from the UK, the US, Canada and Australia are examples, never "the rule"; law belongs in a country pack, not in the core.

## Why this page exists

Some information harms a person if the wrong person learns it, and the harm can be violence, loss of work, loss of housing or of a child, or a person staying away from care. The wrong person is often not a stranger: it may be a partner reading the patient portal, a parent, a colleague on the ward or a clerk printing a letter. So a control on the screen is not enough. Most of the routes out of a record are not screens.

The service can already mask a labelled record. It cannot yet do most of what this page lists, and no clinical record carries a label yet (SE-4).

## A defect found in this pass, now fixed: `ETH` means substance use, not ethnicity

The service used the code `ETH` for an ethnicity label. In the HL7 code system, `ETH` is **"substance abuse information sensitivity"** [HL7R4] [HL7THO]. A record arriving from another system labelled `ETH` would have been read as ethnicity and shown unmasked to nurses and allied health, where the same content labelled `SUD` is shown only to the clinician and the patient.

It was latent: nothing stored a label on a clinical record. Fixed 2026-11-04 (PI-1, H-045): the maintainer decided the project has no ethnicity label (OQ-PI1), so `Label::SubstanceAbuse` carries `ETH` with the same roles as `SUD`, and a test pins each code to its HL7 meaning. HL7's `RACE` code is available if a data protection officer later asks for it.

## Categories

"Protected by" lists what I found, not a complete legal position. "Label" is the nearest HL7 code [HL7R4]; a blank means none was found.

| Category | Why it is protected | Protected by (examples) | HL7 label |
| --- | --- | --- | --- |
| Sexual health, STIs | Stigma; people stay away if not assured of confidence | England: the 2000 Directions were **revoked on 12 October 2023**, so no STI-specific law applies; UK GDPR, the DPA 2018, the common-law duty and NHS England guidance do [DIRREV] [NHSHIV] | `STD`, `SEX` |
| HIV | As above, plus discrimination | As above: guidance says electronic records should restrict access, and dedicated clinics seek explicit consent before sharing outside the service [NHSHIV] | `HIV` |
| Gender history | Disclosure can bring violence or discrimination; in some cases it is a criminal offence | UK: Gender Recognition Act 2004 s.22; Equality Act 2010 s.7 protected characteristic [GRA22] [EQA7] | `GENDER` |
| Violence and assault | The attacker may seek the patient; the patient may be at risk in the ward | Local policy; UK GMC guidance on gunshot and knife wounds [GMCGUN]; hospital "confidential patient" policies [ALIAS] | `VIO` (unrelated person), `SDV` (mainly a related person) |
| Domestic abuse, sexual assault | The abuser may be the partner, the proxy or the person holding the phone | Safeguarding duties; UK guidance on coercion and online access [NHSCOERCE] | `SDV` |
| Female genital mutilation | Safeguarding of a girl; stigma for an adult | England and Wales: mandatory police notification for known cases in under-18s (FGM Act 2003 s.5B, in force 31 October 2015); it does not cover a disclosure by an adult about childhood [FGM] | |
| Children and young people | A competent young person is owed confidence, including from a parent | England and Wales: Gillick and Fraser, decision by decision; GMC 0-18 guidance [GMC018] [NSPCC] | `ADOL` |
| Mental health | Stigma, employment; psychotherapy notes are a particular class | Local law; US psychotherapy-note rules (not read) | `PSY`, `MH`, `BH`, `PSYTHPN` |
| Substance use | Discrimination, prosecution, loss of work or a child | US 42 CFR Part 2: final rule 16 February 2024, compliance by 16 February 2026 [PART2] | `SUD`, `ETH`, `ETHUD`, `OPIOIDUD` |
| Reproductive health, abortion | Prosecution or harm where it is restricted; family conflict | US: the 2024 HIPAA reproductive-health rule was **vacated** nationally by *Purl v. HHS* (N.D. Tex., 18 June 2025); one source reports the appeal was dismissed in September 2025 [PURL] | `SEX`, `PREGNANT` (version-dependent) |
| Genetic information | It speaks about relatives as well as the patient (my reasoning) | Not researched beyond the code | `GDIS` |
| Immigration status | Fear of enforcement keeps people from care | UK: the 2017 Home Office and NHS Digital memorandum was suspended in May 2018 and withdrawn that November; I found nothing on what replaced it [MOU] | |
| Location and address | Someone is looking for the patient | England: the PDS "sensitive" flag restricts location information [PDSS] | `LIVARG`, `LOCIS` |
| Staff, public figures | Curiosity; snooping | See [roles](roles-and-assignments.md), operation 11 | `EMP`, `CEL` |
| Adoption, witness protection | Identity must not link across the break | **Not researched.** The first is a known English process with the same shape as gender change [PCSEA]; the second I did not look at | |

All of these fall under UK GDPR special category or are close to it: Article 6 and an Article 9 condition are both needed, some conditions need a Schedule 1 basis, and an appropriate policy document may be required [ICO9]. Article 9 is not a list of what a clinician may read; it says only that processing needs a lawful footing.

## Gender history: the case most easily got wrong

The request names this category and the law is unusually specific, so here it is in more detail.

- **The offence.** Under section 22 of the Gender Recognition Act 2004 it is an offence to disclose "protected information" acquired in an official capacity, which includes employment and professional services. Protected information is that a person has applied for recognition, or about their gender before the acquired gender took effect. The penalty on summary conviction is a fine up to level 5 [GRA22]. There are exceptions: the person cannot be identified, the person agrees, a court orders it, preventing or investigating crime, and others. A further order under s.22(4) to (5) covers some medical disclosures; I did not read it, and a lawyer must. **Do not assume that a clinician needing to know is an exception until someone has read that order.**
- **The English process.** PCSE describes it for GP practices: adults only (gender-marker changes for under-18s have been stopped); usually a new NHS number and a new record, leaving out information about the previous identity; the practice must not write the new number on the old record; the change is not reversible, and going back gives a third number; it can lose old records, screening invitations and waiting-list places. Alternatively an adult can change the marker and keep the number. NHS England says this process is under review [PCSE]. Several sources say a Gender Recognition Certificate is not required to ask [GENDERREC].
- **Why this is hard for this project.** Three of the rules here meet:
  1. *The matcher.* It is designed to find that two records are the same person. Here the English process deliberately makes two records with no link, and matching them may disclose exactly what the law protects. Equally, the old record may hold an allergy.
  2. *The append-only record.* Nothing is deleted, and the old name is in version history, in the audit trail and in signed documents.
  3. *Clinical safety.* Screening, reference ranges and medicines may depend on physiology, not on the recorded marker. A clinician may need that fact; few need the history. The record needs to carry the relevant organ or physiology fact without the previous name.
- **Deadnaming.** Showing a former name is a harm in itself, and may be the offence above. The same rule applies to name search, autocomplete, letters, audit views and "also known as".

The proposal is below, as ideas 5 and 6. Nobody with the relevant experience has reviewed it, and it should not be built before someone has.

## Leakage routes

A label only works where something checks it. Each route is a place that must be tested; "masked" on the record page proves nothing about the others.

| Route | How it leaks | Evidence |
| --- | --- | --- |
| Patient portal | A partner, parent or abuser reads the account, or forces the login | Refuge and the BMA urged survivors to ask for record sharing to be switched off [REFUGE]; the RCGP warns of forced credential sharing [NHSCOERCE] |
| Proxy access | The proxy is the person the patient needs protection from | [NHSCOERCE]; an adult's proxy access needs consent |
| The redaction itself | Hiding an entry, or abruptly ending proxy access, can reveal by inference that something exists | Training material [NHSCOERCE] |
| Printing | One GP-systems guide warns that hiding from online view does **not** redact a printout | [NHSCOERCE] |
| Letters and reminders | Clinic name or specialty on an envelope, a text message on a shared phone, the home address | My reasoning; the sensitive-location flag exists because of this [PDSS] |
| Results release | An automatic result reaches a portal before the clinician speaks to the patient | HL7 `TBOO` exists for this case [HL7R4] |
| Billing and insurance | The itemised bill names the service | My reasoning; US-shaped |
| Referrals, discharge summaries | The problem list travels into a letter to someone with no need | My reasoning |
| Shared care records | A local restriction is not carried across the boundary | DS4P says a label works only where the recipient honours it [DS4P] |
| Name search, the banner | Search reveals an alias, a former name or that a patient exists; the banner shows a problem or a flag | Hospital "no information" policies [ALIAS] |
| The word "restricted" | A mask that says "restricted information exists" tells a proxy that there is something to find | Same inference risk [NHSCOERCE]; this project's mask does exactly that |
| The audit trail | A clinic name in a patient-visible access log discloses the service; an auditor sees what they should not | My reasoning; the log here shows roles and organisations |
| Export and subject access | The bulk export or the patient copy carries restricted or third-party content | [Data exchange](../data-exchange/index.md), DE-H5 |
| Analytics and the outbox | A derived table, an event or a log line loses the label | My reasoning |
| Telephone and reception | Confirming that a patient is here | "We have no information on a patient by that name" is one hospital's script [ALIAS] |

## How others handle it

- **Sensitivity labels.** HL7 supplies a confidentiality code (one, always) and optional sensitivity codes. FHIR DS4P 1.0.0 (trial use) maps these into FHIR and allows labels below the whole resource. It is policy-agnostic: it does not say who assigns a label, and labels bind only those who agree to honour them [DS4P]. Codes change by release: `SEX` and `STD` are in the R4 value set and the 3.1.0 set, and `SICKLE` has an apparent definition error [HL7R4] [HL7THO]. **Pin the release.**
- **Segregated services.** English sexual-health and HIV services often keep records apart and seek explicit consent to share outside [NHSHIV].
- **Alias and confidential patients.** Practice divides. One US policy marks a patient confidential and **bars** an alias; others use a fictitious name in the master index and swap it back after discharge. An AHIMA sample warns that aliases carry safety and data-integrity issues, and a California breach report shows staff reading an alias record anyway [ALIAS]. An alias is not a control on its own.
- **Lockbox.** Ontario's "lock box" is not a term in PHIPA, but a patient may withhold consent and give express instructions, and a custodian must tell another custodian that information is locked. At one hospital, staff confirm express consent each time they open such a record [ONT].
- **Patient-held codes.** Australia's My Health Record offers a record access code and a limited document access code, with emergency access to a restricted record [MHR].
- **The opposite pressure.** In the US, information-blocking rules make withholding a risk; 45 CFR 171.202(e) lets an actor honour an individual's request not to share, unless disclosure is required by law, and 171.201 is a preventing-harm exception whose conditions I did not read [INFOBLOCK]. So "hide by default" is not free: a clinician who cannot see an allergy, an HIV medicine or a safeguarding alert is also a hazard.

## Compared with this project

| Need | This project today | Gap |
| --- | --- | --- |
| Labels | Six: `R`, `V`, `PSY`, `HIV`, `SUD`, `ETH` | None for `STD`, `SEX`, `SDV`, `VIO`, `GENDER`, `ADOL`, `GDIS`, `EMP`, `CEL`, location |
| Unmasking | A role list per label, then consent; break-glass never unmasks a label | Unreviewed. No relationship or service test: a clinician in any team unmasks `HIV` |
| Records with labels | None: nothing stores a label on clinical content | SE-4 |
| Who applies a label | Nobody; no route | No rule for sensitive by default, no steward, no lift process |
| The mask | Shows "restricted information exists" | Reveals existence to a proxy or a partner reading along |
| Proxy | A role with scopes | No hidden-from list; no rule for a young person; no coercion route |
| Alias, no-information | None | The directory, search and banner all show the patient |
| Name and gender history | None | Old-name suppression; matcher behaviour across a deliberate break |
| Outputs | No letters, no portal results, no notifications yet | Build each one with a label test from the first line |
| Audit | Every allow audited; the log shows roles and organisations | Whether a service name in the log discloses; who may view audit rows on a protected record |
| Twins, adoption, old and new identity | The matcher never auto-links twins | Nothing for a deliberate unlink |

## Candidate hazards

For the Clinical Safety Officer; none is accepted or closed by this page. H-045 is in the [hazard log](../../governance/clinical-safety/hazard-log.csv) as `open`. The rest go in with the first code that creates the exposure.

1. **A label misread between systems** (H-045, controlled, unreviewed). `ETH` was read as ethnicity, not substance use. The same risk applies to any code added later.
2. **A safeguarding or domestic-abuse entry reaches the abuser** through a portal, a proxy view, a letter or a text.
3. **A masked fact is needed in an emergency.** An HIV medicine, an interaction or a safeguarding alert is hidden, and break-glass does not unmask a label. This is the reverse of H-007 and at least as serious.
4. **A former name or gender is shown or searchable**, in the UI, a letter, an audit view or an export.
5. **The old and new records are linked or matched**, disclosing a protected fact; or they are never linked, and an allergy is lost.
6. **Screening or a reference range follows the marker**, not the physiology.
7. **An alias is taken for a real identity**, so a result, a transfusion or a medicine goes to the wrong person, or the alias is not reversed.
8. **A young person's confidential entry reaches a parent** with proxy access.
9. **The mask itself discloses** that something is hidden.
10. **The label is lost in a derived output**: export, FHIR, outbox, analytics, log.
11. **A label sits unreviewed.** It was applied wrongly, or a person who wants it lifted cannot get it lifted.

## Ideas

1. **`ETH` is fixed** (PI-1): it has its HL7 meaning, and a test pins each code. Checking the pin against HL7 itself remains to do.
2. **Extend the labels** through a value set pinned to one HL7 release, with `STD`, `SEX`, `SDV`, `VIO`, `GENDER`, `ADOL`, `GDIS`, `EMP`, `CEL` as proposals; local policy says who sees each.
3. **Sensitive by default, by rule.** A coded list (a pack) marks a condition, a medicine, a clinic or a service as carrying a label, with a person able to add one by hand. Absent evidence is never "not sensitive".
4. **A label steward and a lift process.** Who may add, who may lift, a reason, a named reviewer who is not the subject; append-only.
5. **A protected-identity mechanism.** A person-level flag that suppresses former names in search, autocomplete, banner, letters and exports; history is kept but readable only by a named role, with a reason, and audited. Written with trans patients and a lawyer.
6. **A matcher rule for a deliberate break.** A recorded "do not link" pair that the matcher honours and does not reveal; with a way, approved by the patient, to carry safety facts (allergies, implants, physiology) across without the former identity. Depends on [the matcher spec](../../electronic-patient-record-matcher-with-rust/spec/index.md).
7. **Alias and no-information patients.** The true identity stays on the record with a display name for the stay; the directory and telephone answer say nothing; the banner always tells a clinician that the name is an alias; ending it is a step on the pathway.
8. **A hidden-information class**, the same one [maternity](../maternity/index.md) asks for: its existence is not shown to a proxy or a shared view, and every output honours it.
9. **Proxy rules with dates and hidden-from lists**: a parent's proxy narrows as a child grows, by a setting in the pack; a young person can be asked; a coercion flag ends proxy access in a way that doesn't announce why.
10. **A route census.** In the manner of the existing census tests: every output route has a test that a labelled record does not appear in it. Add a new route, and the census fails until it names its label behaviour. **Built as a first slice (PI-7):** `tests/label_census.rs` lists every output route and what it does with a label. Today no route applies one: 34 give out patient data with no label applied, and two more tests fail on the day that changes. The test that a labelled record does not appear waits for labelled records.
11. **A neutral envelope.** Letters, texts and portal notices use a neutral sender and no specialty for labelled pathways.
12. **An emergency rule.** Decide with clinicians what break-glass shows of a labelled record: at least the safety fact (a drug, an interaction), not the narrative. Today it shows neither.
13. **Protected-record audit.** Audit views of a protected record require a named role and are themselves reviewed.
14. **Measure it.** Label coverage against the sensitive-by-default list, and time to lift ([measuring clinical safety](../clinical-safety/measurement.md)), with planted failures.

## Open questions

Summarised as OQ-PI1 to OQ-PI6 in [open questions](../16-open-questions.md).

1. Decided: no ethnicity label (OQ-PI1).
2. What does break-glass show of a labelled record? Today, nothing; is that safe? (OQ-PI2)
3. Does s.22 of the Gender Recognition Act, with its orders, allow a clinician to see gender history for care, and what does that mean outside the UK? (OQ-PI3)
4. Is a deliberate break between old and new records ever bridged, and who decides? (OQ-PI4)
5. Should a mask say anything at all? "Restricted information exists" helps a clinician and harms a person being watched: does it differ by viewer? (OQ-PI5)
6. Which of these people are consulted before a build, and how: people living with HIV, trans patients, survivors, young people? (OQ-PI6)
7. Do labels follow the patient across tenants or countries, where the rules differ?
8. Who is the steward in a small clinic?

## Not verified

- **All law here is a lay reading.** I read section 22 of the Gender Recognition Act, section 7 of the Equality Act, the Directions' revocation and the PCSE page as summaries of fetched pages, and the rest as search results. I did not read the order made under s.22(4) to (5), which is the point that matters most for clinical care.
- The HL7 code lists come from two releases fetched as summaries: the R4 value set and THO 3.1.0. The current THO page could not be read (it returned a redirect notice), and code definitions differ between releases. The display names are as fetched.
- FGM: the sources are legislation notes and professional-body sheets; current Home Office guidance was not read.
- GMC guidance on gunshot and knife wounds and on 0-18 years: seen as search results; the wording of what a doctor should do was not read, and I do not state it. The dates in the snippets conflict, so none is given here.
- The 42 CFR Part 2 facts come from law-firm commentary, and one source conflicts on the date. The tolled accounting-of-disclosures date needs the primary source. The *Purl* appeal outcome rests on one source.
- The immigration results all date from 2018. **Whether any agreement is in force now is unknown.**
- PCSE says the English gender process is under review; it may have changed. Scotland, Wales and Northern Ireland differ and were not read.
- Adoption and witness protection were asked about and not researched. Nor was psychotherapy-note law, or US state law.
- Sources in the table marked "my reasoning" are mine, with no source behind them.
- No vendor documents were read. Nobody with lived experience of any category has seen this page, and a page about them should not be taken as settled without them.

## Sources

- [HL7R4] HL7 FHIR R4, InformationSensitivityPolicy value set: https://www.hl7.org/fhir/R4/v3/InformationSensitivityPolicy/vs.html (fetched, summarised)
- [HL7THO] HL7 Terminology 3.1.0, ActInformationSensitivityPolicy: https://terminology.hl7.org/3.1.0/ValueSet-v3-ActInformationSensitivityPolicy.html (fetched, summarised)
- [DS4P] HL7 FHIR Data Segmentation for Privacy 1.0.0: https://hl7.org/fhir/uv/security-label-ds4p/index.html (search summary)
- [GRA22] Gender Recognition Act 2004, s.22: https://www.legislation.gov.uk/ukpga/2004/7/section/22 (fetched, summarised)
- [EQA7] Equality Act 2010, s.7: https://www.legislation.gov.uk/ukpga/2010/15/section/7 (fetched, summarised)
- [PCSE] PCSE, gender reassignment: https://pcse.england.nhs.uk/help/patient-registrations/gender-reassignment (fetched, summarised)
- [PCSEA] PCSE, adoption and gender re-assignment processes: https://pcse.england.nhs.uk/help/registrations/adoption-and-gender-re-assignment-processes/ (title only)
- [GENDERREC] MDDUS, the care of transgender patients by GPs: https://mddus.com/-/media/files/training-and-cpd/advice-library-pdfs-q3-2024/the-care-of-transgender-patients-by-gps.pdf (snippet)
- [PDSS] NHS England PDS National Back Office (mirror): https://nhsd-proxy.openprescribing.net/services/personal-demographics-service/national-back-office-for-the-personal-demographics-service/management-of-nhs-numbers-and-pds-records (snippet)
- [DIRREV] STD Revocation Directions 2023: https://www.gov.uk/government/publications/nhs-trusts-and-primary-care-trusts-sexually-transmitted-diseases-revocation-directions-2023 (fetched, summarised)
- [NHSHIV] NHS England, HIV and STIs guidance: https://transform.england.nhs.uk/information-governance/guidance/hiv-and-sexually-transmitted-infections-stis (search summary)
- [GMCGUN] GMC, reporting gunshot and knife wounds: https://www.gmc-uk.org/professional-standards/the-professional-standards/confidentiality---reporting-gunshot-and-knife-wounds (snippet)
- [GMC018] GMC, 0-18 years: https://www.gmc-uk.org/professional-standards/the-professional-standards/0-18-years (snippet)
- [NSPCC] NSPCC, Gillick competence and Fraser guidelines: https://learning.nspcc.org.uk/child-protection-system/gillick-competence-fraser-guidelines (snippet)
- [FGM] Serious Crime Act 2015, explanatory note to s.74: https://legislation.gov.uk/ukpga/2015/9/notes/division/3/5/2/9 (snippet)
- [NHSCOERCE] NHS England, coercion: https://www.england.nhs.uk/long-read/coercion/ ; RCGP coercion guidance (April 2022): https://elearning.rcgp.org.uk/pluginfile.php/179161/mod_book/chapter/782/Coercion%20v5.0.pdf ; Medical Protection: https://www.medicalprotection.org/uk/advice-centre-articles/ask-the-expert-online-patient-records-nhs-guidelines-record-keeping (snippets)
- [REFUGE] Refuge, joint letter on survivors' records on the NHS App: https://refuge.org.uk/news/joint-letter-on-concerns-around-survivors-medical-records-on-nhs-app/ (snippet)
- [ICO9] ICO, the rules on special category data: https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/lawful-basis/special-category-data/what-are-the-rules-on-special-category-data/ (search summary)
- [PART2] 89 FR, 16 February 2024: https://www.govinfo.gov/content/pkg/FR-2024-02-16/html/2024-02544.htm (search summary, primary text not read)
- [PURL] Haynes Boone, rule remains vacated after appeals closed: https://www.haynesboone.com/news/publications/hipaa-reproductive-healthcare-privacy-rule-remains-vacated-after-appeals-closed (snippet)
- [INFOBLOCK] 45 CFR 171.202: https://www.law.cornell.edu/cfr/text/45/171.202 ; ONC FAQ: https://healthit.gov/faq/if-individual-requests-their-ehi-not-be-disclosed-it-information-blocking-if-actor-does-not/ (snippets)
- [ALIAS] University of Toledo policy: https://www.utoledo.edu/policies/utmc/administrative/pdfs/3364-100-01-13.pdf ; AHIMA, using aliases: https://journal.ahima.org/Portals/0/archives/AHIMA%20files/Using%20Aliases%20to%20Protect%20Privacy%20in%20Healthcare.pdf ; Relias, aliases: https://www.reliasmedia.com/articles/48472-hospital-uses-aliases-to-protect-patient-identity ; California breach report: https://www.cdph.ca.gov/Programs/CHCQ/LCP/CDPH%20Document%20Library/Breaches/LnCSantaMonicaUCLAMedCtr_20080404.pdf (snippets)
- [ONT] IPC Ontario, PHIPA Decision 144: https://decisions.ipc.on.ca/ipc-cipvp/phipa/en/496393/1/document.do ; Ottawa Hospital (snippets)
- [MHR] OAIC, My Health Record access controls: https://www.oaic.gov.au/privacy/guidance-and-advice/my-health-record/my-health-record-access-controls/ (snippet)
- [MOU] NHS Digital ends sharing, Digital Health, November 2018: https://www.digitalhealth.net/2018/11/nhs-digital-patient-data-sharing-home-office-end/ (snippet)
- This repository: `electronic-patient-record-service-with-rust/src/policy.rs`, `consent.rs`; [Access control](index.md); [Audit](../audit/index.md).
