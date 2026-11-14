# Clinical documentation: notes, letters and summaries

Public-source research read 2026-11-14 plus this repository's own files. A proposal and a reading, not clinical advice: nothing here is built, and no clinician has reviewed it.

Method: web searches and fetches (some full pages, many search snippets; "Not verified" says which) and the project's specs: [domain model](../05-domain-model.md) (5.2.1 pathway first, 5.3 versioning), [functional requirements](../06-functional-requirements.md) (FR-5.1 to FR-5.5), [implementation status](../14-implementation-status.md), the [service spec](../../electronic-patient-record-service-with-rust/spec/index.md) (results with a version-named sign-off, amendment requests, entered-in-error, referrals, assessments under versioned forms), [measurement](../clinical-safety/measurement.md), [usability](../usability/index.md), and [`tasks.md`](../../tasks.md) entries DO-1 to DO-6, EM-8, RC-4 and IO-5. Style follows the [virtual ward](../virtual-ward/index.md) page. The companion for orders is [order entry](../order-entry/index.md). Inpatient first: admission, ward round, nursing and discharge come before clinic letters.

## What it is

Clinical documentation is the narrative and structured record that clinicians write about a patient: what they found, what they think, what they decided and what happens next. In this project it covers:

- **Admission clerking.** The first full medical assessment on admission: presenting complaint, history, examination, impression, plan, and escalation and resuscitation decisions.
- **Ward round and progress notes.** A short daily or more frequent note: who was present, what changed, the plan, and the escalation plan.
- **Nursing notes.** Shift notes and assessments, many of them structured (the built assessments are one kind).
- **Discharge summary.** The document that goes with the patient to the next carer (general practice, care home, community team) at discharge.
- **Clinic letters.** The outpatient letter, written to the patient and copied to the general practitioner, or the reverse.

Cross-cutting features: templates, signing and co-signing, amendments and addenda, entered-in-error, copy-forward, and content drafted by an AI tool.

## Evidence and how others do it

- **Most note text is not typed.** In 23,630 inpatient progress notes by 460 clinicians at one US centre, 18% of text was typed by hand, 46% copied and 36% imported [WANG2017] (abstract and press summaries).
- **Notes are long and repetitive.** About 3 million outpatient notes at one academic centre grew from a median of 401 words in 2009 to 642 in 2018, and the share of text identical to the patient's previous note rose from 47.9% to 58.8% [RULE2021]. Across all notes at a second centre in 2015 to 2020, 50.1% of text was duplicated from earlier text about the same patient [STEINKAMP2022] (abstract only).
- **Length is partly a regulatory artefact.** An editorial reported, from the vendor's data, that US notes were on average four times as long as those in several other countries [DOWNING2018] (secondary coverage only; the editorial itself not read).
- **Copy-paste guidance.** A US accrediting body's safety alert asks organisations to state what may be copied, monitor accuracy, feed back to clinicians, and "ensure that the provenance of copy-and-paste material is readily available", noting this may need software changes [TJC-QS10] (secondary summaries of the alert; the PDF was not opened).
- **Nurses' time.** In one US health system's 2019 timestamp data, nurses spent on average 157 minutes per 12-hour shift (22%) documenting in the record; their own estimate was 43% [NURSE2022] (search summary).
- **Co-signature in teaching.** US federal billing rules since 2018 let a teaching physician verify a medical student's documentation rather than redo it, and a July 2018 instruction treats a signature and date on the entry as that verification [CMS2018] (secondary summaries; the transmittal was not opened). This is a billing rule in one country, cited to show that "co-sign" has legal weight somewhere; it is not a rule for this project.
- **AI scribes, trial evidence.** A three-arm pragmatic trial of 238 outpatient physicians found one ambient scribe cut time-in-note by 9.5% versus usual care and the other made no significant difference; physicians reported that notes "occasionally" held clinically significant inaccuracies, mostly omissions, and one mild safety event [LUKAC2025] (abstract and press release).
- **AI scribes, guidance.** England's national health body's guidance (version 3, last updated 29 July 2026) asks organisations to "ensure users review and approve any product outputs prior to further actions", to have the supplier's clinical safety case under DCB0129 and their own under DCB0160, and to settle how long audio, transcripts and outputs are kept; it does not require the record to say a scribe was used [AVT2026] (page fetched).
- **Discharge summary content.** The PRSB eDischarge summary standard (Transfer of Care, Acute Inpatient Discharge, v2.1) has 22 sections, six mandatory, including diagnoses, medications and medical devices, allergies, plan and requested actions, person completing record, and distribution list; it says to send it as a FHIR message, and its ownership moved to England's national health body on 1 January 2026 [PRSB-DS] (page fetched).
- **Clinic letters.** The PRSB has a separate outpatient letter standard (final report 2017) [PRSB-OPL]. The Academy of Medical Royal Colleges recommends writing most outpatient letters directly to the patient in plain English, copying the GP, and telephoning rather than writing when a result may upset [AOMRC2026] (search summaries of the 2026 update).
- **Ward rounds.** A royal college report written with nursing, pharmacy and physiotherapy bodies has a chapter on ward round documentation and recommends standard proformas and one multidisciplinary record where possible [RCP-MWR] (search summary; the report was not read).
- **Elsewhere.** These are examples from high-income settings. The [equity rule](../02-scope.md) applies: a note must work as plain text on a slow link, and a template is a pack, not a product feature.

## Standards

- **FHIR R4 `Composition`** is the structured document (sections, author, attester). Status is `preliminary`, `final`, `amended` or `entered-in-error`, and receivers "SHALL NOT ignore" `entered-in-error`. `attester.mode` is `personal`, `professional`, `legal` or `official`. `relatesTo.code` is `replaces`, `transforms`, `signs` or `appends`. A Composition is a document only as the first entry of a `Bundle` of type `document` [FHIR-COMP].
- **FHIR R4 `DocumentReference`** indexes any document with a MIME type (a scanned letter, a PDF, a FHIR document); `docStatus` uses the same four codes, and `relatesTo` the same four relations [FHIR-DOCREF].
- **Provenance** names the agent of each version, human, device or AI, as [5.3](../05-domain-model.md) already requires.
- **PRSB** discharge summary and outpatient letter standards for headings [PRSB-DS] [PRSB-OPL]; the International Patient Summary for `$summary` (IO-5) reuses the same Composition machinery.
- **Clinical safety.** DCB0129 for the manufacturer; any AI drafting tool brings its own DCB0129 case [AVT2026].

Proposed mapping (to confirm against UK Core and US Core profiles): a note is a `Composition` with a `type` code (LOINC document ontology or SNOMED CT record artefact), one `section` per heading, an `author`, and `attester` entries for signer and co-signer. An addendum is a new `Composition` with `relatesTo.code = appends`. A correction is an `amended` version. A note in error is `entered-in-error`. An external or scanned document is a `DocumentReference`.

## Workflows

Each step names what the clinician needs on screen. Steps 1 to 4 are shared by every note.

1. **Open the patient** (banner: name, birth date, national health number, allergies summary, location). The note pane names the patient again in its header. Data: identity, allergy banner (built).
2. **Choose a note type and template.** The list shows each template's name, version and review status (draft templates are marked). Data: template pack.
3. **Write.** The editor shows, beside each block, where it came from: typed, copied (from which note, which version, by whom, when), imported (from which result, observation or allergy version), template text, or AI-drafted (which tool, which version). Data pulled in is a reference to a version, not a copy that can drift.
4. **Sign.** The sign screen shows the patient again, the note's copied and imported share, any block still marked AI-drafted and unreviewed, and who must co-sign. Signing names the version signed, as a result sign-off already does.

Admission clerking (inpatient, first):

5. Template sections: presenting complaint, history, medicines on admission (a reference to the reconciliation record when RC-10 exists, otherwise "not reconciled"), allergies (from the record, never retyped), examination, observations at admission (from the record), impression, plan, escalation and resuscitation decision (a reference, or "not recorded" with a reason).
6. The plan's actions become orders or tasks on the patient's pathway (see [order entry](../order-entry/index.md)); each one says it is on a pathway or "not on a pathway" (5.2.1 rule 2).
7. A trainee's clerking goes to the named supervising clinician's co-sign list; until co-signed it shows "awaiting co-signature by X".

Ward round and progress note:

8. Header pre-filled: date, time, who led, who was present (picked, not typed).
9. Since-last-review panel: new results (with sign-off state), observation trend, new allergies, orders placed or completed. These are shown, not pasted into the note.
10. Body: assessment and plan; escalation plan with a review time.
11. Copy-forward from yesterday's note is allowed per section, each copied block marked and kept linked to its source; a block copied unchanged for a configured number of days is shown to the author before signing.

Nursing note:

12. Shift, author, and the structured assessments due or done (built: assessments under versioned forms), with free-text notes beside them. A nursing note is signed by its author; no co-sign by default.

Discharge summary (EM-8):

13. Sections follow the PRSB headings in use. Diagnoses, procedures, results, allergies and medicines are references to record versions. Medicines need the reconciled list (RC-10); until then the section says "medicines not reconciled" and the summary cannot be sent.
14. Plan and requested actions: each action for the receiver is listed separately so that it can be tracked (a step on the pathway, owned by the receiver after acceptance).
15. Sign, then send. An unsigned summary cannot be sent. The distribution list shows who received it and when, and whether each receiver acknowledged.

Clinic letter:

16. Addressee chosen: the patient (default where the deployment follows [AOMRC2026]) or the GP; copies listed.
17. A readability figure is shown to the author (advisory only).
18. Sign, then send; an amended letter after sending is resent with "this replaces the letter of date" and the receivers listed.

Amendment and error (all notes):

19. A signed note cannot be edited. The author or another clinician adds an addendum (a new signed note that appends), or marks the note `amended` with a reason (a new version), or marks it `entered-in-error` with a reason (final; the note stays visible as struck through with the reason, to readers allowed to see it). A patient's request to correct a note follows the built amendment request flow (RC-4), which today covers observations and allergies only.

## Rules

Each is a testable sentence.

1. A note is stored as append-only versions; no route updates or deletes a stored version, and a database trigger refuses `UPDATE`, `DELETE` and `TRUNCATE`.
2. A change to a signed note other than an addendum, an amendment with a reason, or entered-in-error is refused with 409.
3. Signing names the version signed; signing a version that is not current is 409 with the current version.
4. A note type configured as needing co-signature for the author's role shows "awaiting co-signature" until a clinician with the configured role signs the same version.
5. A co-signer cannot be the author.
6. Every block of a note carries a source kind: `typed`, `copied`, `imported`, `template` or `ai-drafted`.
7. A `copied` block keeps the source note id and version after save; an `imported` block keeps the source resource type, id and version.
8. The copied share of a note is computed from block lengths; a note with blocks of unknown source (for example a migrated legacy note) reports the share as `null` with the reason `source-unknown`, never 0.
9. A note with any `ai-drafted` block that has no review action by the signer cannot be signed.
10. The Provenance of an AI-drafted block names the tool and its version and survives signing and amendment.
11. A discharge summary cannot be sent unless it is signed and its medicines section references a reconciliation record; "not reconciled" blocks sending.
12. A sent document that is later amended or entered in error produces a notice to every receiver on its distribution list.
13. Entered-in-error is final: no later version follows it, and it needs the `MarkEnteredInError` permission.
14. Drafts are visible only to their author and to roles the policy names; a draft never appears in another clinician's view as if it were signed.
15. Every read and write of a note is audited; the audit row holds ids, versions, note type and counts, never note text.
16. A template is data with an id, version, approver and day, like the built assessment forms; a note records which template version it used.
17. A note on a record merged into another is listed and amended through the survivor.

## Hazards to add when built

Described here, not added to the [hazard log](../../governance/clinical-safety/hazard-log.csv); each goes in with the first code (AS-1).

- **CN-H1 Copied text is out of date.** A copied examination finding or plan is read as today's. Controls: block source kind, age shown, stale-copy prompt.
- **CN-H2 Wrong-patient note.** Text is written or pasted into another patient's note. Controls: patient named in the note header and at sign; paste from another patient's note is marked and needs confirmation.
- **CN-H3 Unreviewed AI content signed.** An omission or invented finding by a scribe is signed. Controls: rule 9, review per block, provenance kept.
- **CN-H4 Unsigned or uncosigned note acted on.** A trainee's plan is followed before supervision, or an unsigned draft is read as final. Controls: status visible on every view; co-sign list with age.
- **CN-H5 Discharge summary late, missing or with the wrong medicines.** Controls: rule 11, distribution acknowledgement, unsent summaries on a worklist.
- **CN-H6 Correction not seen downstream.** An amended or entered-in-error letter is still believed by its receiver. Controls: rule 12.
- **CN-H7 Note bloat hides the key finding.** Controls: since-last-review panel shown, not pasted; copied share visible.

## First slice

The smallest useful piece: inpatient progress notes with sign, addendum, entered-in-error and block provenance. No templates beyond one built-in plain layout, no co-sign, no AI, no sending.

- **Store.** Table `note_versions` (patient id, note id, version, type code, status `draft|final|amended|entered-in-error`, author, signed-by, signed-version, reason, blocks as JSONB each with `kind` and source reference, created at) and view `note_current`; append-only by trigger, like `result_versions`.
- **Routes.** `GET /api/patients/{id}/notes`, `POST /api/patients/{id}/notes` (a draft), `PUT /api/patients/{id}/notes/{note_id}` (`expected_version`; drafts only), `POST …/notes/{note_id}/sign` (`{version}`), `POST …/notes/{note_id}/addenda`, `POST …/notes/{note_id}/entered-in-error` (`{reason}`). Unversioned `/api` path; the `Accepts-version` header as elsewhere. Policy resource `Composition` (clinical).
- **Screen.** A "Notes" section on the patient page: list (type, author, status, time), a plain editor with one block per paragraph showing its source kind, a sign button that shows the patient and the copied share.
- **Tests.** Rules 1 to 3, 6 to 8, 13 and 15 as unit and Postgres tests; a census test that every note route is audited; a UI test that a signed note has no edit control.

## What this repo has and lacks

| Need | Today | Gap |
|---|---|---|
| Append-only versions with a version-named sign-off | Built for results (`result_versions`, `result_signoffs`) | Not for notes |
| Amendment and entered-in-error | Built for observations, allergies, results; patient amendment requests for two kinds | No note kind; no addendum |
| Structured forms with versions | Built: assessments under versioned forms (WL-3) | No narrative note around them; no Questionnaire round-trip (DO-4) |
| Provenance naming an AI agent | Designed (5.3) | Not built |
| Copy-forward provenance | Measure named in [measurement](../clinical-safety/measurement.md) | No note text to measure |
| Templates as packs | Pattern exists (assessment forms, suspicion pathways) | No note templates |
| Discharge summary and letters | EM-8 written | No Composition, no medicines, no sending |
| Pathway link for plan actions | Designed (5.2.1) | No pathway store |

## Not verified

- The UCSF, Penn, nurse-time, editorial and trial figures are from abstracts, press releases or search summaries, not full papers [WANG2017] [STEINKAMP2022] [DOWNING2018] [NURSE2022] [LUKAC2025].
- The accrediting body's copy-paste alert and the US co-signature rules were read in secondary summaries only [TJC-QS10] [CMS2018].
- The royal college ward-round report, the outpatient letter standard and the 2026 letter guidance were not opened; their content here is from search summaries [RCP-MWR] [PRSB-OPL] [AOMRC2026].
- The profiles (UK Core, US Core) for Composition and the document type codes were not checked. [uncertain] whether UK Core profiles Composition at all.
- Whether the PRSB discharge standard has a version later than 2.1 after the change of owner is [uncertain].
- No clinician, nurse, information governance lead or lawyer has reviewed any of this.

## Sources

- [WANG2017] Wang, Khanna, Najafi. Characterizing the source of text in electronic health record progress notes. JAMA Internal Medicine 2017: https://jamanetwork.com/journals/jamainternalmedicine/article-abstract/2629493 ; summary: https://www.fiercehealthcare.com/ehr/clinicians-copy-and-paste-nearly-half-ehr-progress-notes
- [RULE2021] Rule, Bedrick, Chiang, Hribar. Length and redundancy of outpatient progress notes across a decade. JAMA Network Open 2021: https://pmc.ncbi.nlm.nih.gov/articles/PMC8290305/
- [STEINKAMP2022] Steinkamp, Kantrowitz, Airan-Javia. Prevalence and sources of duplicate information in the electronic medical record. JAMA Network Open 2022: https://pubmed.ncbi.nlm.nih.gov/36156143/
- [DOWNING2018] Downing, Bates, Longhurst. Physician burnout in the EHR era. Annals of Internal Medicine 2018: https://psnet.ahrq.gov/issue/physician-burnout-electronic-health-record-era-are-we-ignoring-real-cause ; coverage: https://med.stanford.edu/news/insights/2018/05/u-s-doctors-saddled-with-four-times-the-amount-of-note-taking-as-foreign-counterparts.html
- [TJC-QS10] The Joint Commission. Quick Safety issue 10, preventing copy-and-paste errors in EHRs, 2015: https://www.jointcommission.org/-/media/tjc/documents/newsletters/quick_safety_issue_10pdf.pdf
- [NURSE2022] Nurse documentation time, timestamp and survey study: https://pmc.ncbi.nlm.nih.gov/articles/PMC9300261
- [CMS2018] US Centers for Medicare and Medicaid Services, transmittal R4068CP on student documentation: https://cms.gov/regulations-and-guidance/guidance/transmittals/2018downloads/r4068cp.pdf ; summary: https://ama-assn.org/practice-management/medicare/medical-student-documentation-myth
- [LUKAC2025] Lukac et al. Ambient AI scribes in clinical practice: a randomized trial. NEJM AI 2025: https://pubmed.ncbi.nlm.nih.gov/41497288/ ; release: https://www.uclahealth.org/news/release/ucla-study-finds-ai-scribes-may-reduce-documentation-time
- [AVT2026] England's national health body. Guidance on the use of AI-enabled ambient scribing products in health and care settings, version 3: https://www.england.nhs.uk/long-read/guidance-on-the-use-of-ai-enabled-ambient-scribing-products-in-health-and-care-settings
- [PRSB-DS] PRSB. eDischarge summary standard (Transfer of Care, Acute Inpatient Discharge), v2.1: https://theprsb.org/standards/edischargesummary/
- [PRSB-OPL] PRSB. Outpatient letter standards, final report draft 2.1: https://dv8vgeb1jof7a.cloudfront.net/prsb-assets/18-Outpatient%20Letter%20Standard/Outpatient-Letter-Standards-Final-Report-Draft-2.1-FINAL.pdf
- [AOMRC2026] Academy of Medical Royal Colleges. Please write to me, update 2026: https://www.aomrc.org.uk/wp-content/uploads/2026/06/Please_write_to_me_update_130426.pdf
- [RCP-MWR] Royal College of Physicians and others. Modern ward rounds: https://www.rcp.ac.uk/resources/modern-ward-rounds/
- [FHIR-COMP] HL7 FHIR R4 Composition: https://hl7.org/fhir/R4/composition.html
- [FHIR-DOCREF] HL7 FHIR R4 DocumentReference: https://hl7.org/fhir/R4/documentreference.html
