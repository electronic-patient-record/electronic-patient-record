# Manual steps: how information changes when a person is in the middle

Public-source research read 2026-11-04 plus this repository's own files. A proposal: nothing here is built, and no clinician or data protection officer has reviewed it.

Question: how does information change if there is a manual step somewhere? A manual step is any point where a person, not a machine, carries information from one place to another: paper to screen, screen to screen, voice to paper, scan to record. Related: [data exchange](../data-exchange/index.md) (the machine paths in and out), [data quality](../data-quality/index.md) (how we measure and govern the result), [interoperability](../interoperability/index.md), [audit](../audit/index.md), [domain model](../05-domain-model.md) (section 5.3, versioning and provenance).

## Why a manual step matters

Every manual step is a copy, and a copy can lose, change, delay or misattach information. Four things change at the step:

1. **Content.** A digit is mistyped, a unit dropped, a word misread. A published outpatient study that re-keyed point-of-care glucose results found a mistranscription in 3.2% of 6,930 events, and noted that published rates span an order of magnitude [1].
2. **Time.** The value is true when read and late when keyed. The record shows the time of keying unless the writer is given a place to say when it was observed.
3. **Attribution.** The record names the person who typed, not the person who observed or decided. Without a separate "reported by" the typist looks like the source.
4. **Attachment.** The right value goes to the wrong patient. Copy-and-paste into the wrong chart is one of the risks named in a systematic review of copy-and-paste in electronic records [2].

## Kinds of manual step

**Paper to digital transcription.** Rates depend on how an error is counted (per field, per record, per entry), and a factor of two or more can come from the counting alone [1][3]. A meta-analysis (a preprint, so not peer reviewed when seen) of clinical-research data processing reported lower error rates for double data entry than single entry: pooled 0.14% against 0.29% [3]. That is research data capture, not clinical care, and the figures should not be read across.

**Double keying and verification.** Two people key the same item and a system compares. It is effective for research forms, but it doubles the cost, and a cited objection is that range checks and logic tests catch many errors more cheaply [3]. In care, the cheaper analogue is a read-back or a "re-enter to confirm" on the few fields where an error is dangerous (weight, dose, allergy, identifier).

**Reading one screen and typing into another.** The commonest manual step in practice: a clinician reads a legacy or external system and re-keys into this one. No source I found measures this separately from transcription; treat it as the same risk plus the loss of any structure the first screen held.

**Phone, fax and verbal handover.** Structured handover tools such as SBAR (situation, background, assessment, recommendation) are endorsed by the Joint Commission, AHRQ, IHI and WHO, and a narrative review reports fewer adverse events with them [4]. The dates and wording of the Joint Commission requirement conflict between the secondary sources I saw [4]. A verbal handover leaves no record unless someone writes it down, which is another manual step.

**Downtime paper.** When the system is unavailable, care continues on paper and is back-keyed later. The back-keying is a transcription (above) with the extra hazard that the time of the event and the time of keying differ by hours. I did not find a public NHS source on downtime procedure in this pass (the search covered only the SBAR half of the query), so no claim is made about national guidance.

**Scanned documents.** A scan preserves the image and loses the structure. Unless the scan is indexed to a patient, date and type by a person, it is findable only by luck; the index is itself a manual step. Optical scanning of forms had a pooled error rate of 0.74% in the research-data meta-analysis above [3], again a different setting.

**Copy-and-paste and copy-forward.** The systematic review (51 publications, 2010 to January 2015) found that 66% to 90% of clinicians routinely copy and paste, listed bloated notes, internal inconsistency, propagated errors and wrong-patient filing as risks, and concluded that evidence of direct patient harm is sparse and limited [2]. Its four recommendations: make pasted content identifiable, keep its source visible, train staff, and monitor the practice [2].

**Manual merge, override and correction.** A person decides two records are one patient, or overrides a warning, or edits a value to match what they believe is true. The National Back Office for the Personal Demographics Service handles merges of duplicate NHS numbers and "confusions" (two patients linked to one number); NHS sources describe both as patient-safety risks and show a case where seven duplicates arose from one registration error and persisted five months [5].

## What public practice suggests

- Capture the **source and the transcriber separately**. FHIR `Provenance` holds agents and activity for a resource version, and may be referenced for any version, including past ones [6]. A "reported by" agent role distinct from the "author" role is the natural place.
- **Record the original alongside the copy** where the original is paper: keep the scan, link it, and say it is the source.
- **Mark pasted content** and keep its origin [2].
- **Verify the few dangerous fields**, not all fields [3].
- **Make the manual step visible in the record**, so a reader can discount a transcribed value correctly.

## Compared with this project

| Need | This project today | Gap |
| --- | --- | --- |
| Say who observed versus who typed | Every version has author, time, reason and a Provenance naming the agent (spec 5.3, planned; the version tables are built for patient, consent, allergy and amendments) | No "reported by", "transcribed from" or "observed at" field defined |
| Observed time versus entry time | UTC rule exists; `recorded_at` is the entry time | No `occurred_at` rule for back-keyed downtime entries |
| Keep the paper source | Photos are stored for patients; no document store for clinical scans | No scan, index or link-to-source design |
| Correct a transcription error without erasing it | Append-only versions and an amendment-request workflow (migration 15) exist | No "entered in error" status tied to a transcription; not tested against this case |
| Detect a manual merge gone wrong | Merge and unmerge routes exist and are audited | No review-after-merge queue; no measure of merge reversals |
| Flag pasted content | None | UI has no paste detection or marking |
| Verify dangerous fields at entry | Absent evidence is `null` with a reason (rule exists) | No read-back or double-entry design for dose, weight or allergy |
| Downtime procedure | None; no backup procedure exists (retention schedule) | No downtime or back-keying design, no downtime read-only view |
| Structured verbal handover | None | Out of scope for a record, but a handover note template is not designed |

## Candidate hazards

For the Clinical Safety Officer to triage. None is accepted, and none is yet a row in `governance/clinical-safety/hazard-log.csv`; they would be added with the first code that creates the exposure (AGENTS.md rule 4).

- MS-H1. A value is mistyped when re-keyed from paper or another screen, and a clinician acts on it.
- MS-H2. A back-keyed downtime entry carries the keying time, so a clinician reads an old value as current.
- MS-H3. A transcribed entry looks like a first-hand observation, so it is over-trusted (or the transcriber is blamed).
- MS-H4. A scan is attached to the wrong patient or is unindexed, so a result is lost or misattributed.
- MS-H5. Pasted text carries an outdated or other-patient statement into a new note.
- MS-H6. A manual merge joins two different patients, or a manual override silences a correct warning.
- MS-H7. A "correction" is made by a new entry that is not linked to the one it corrects, so both appear current.

## Ideas

1. Define `reported_by`, `transcribed_from` (a link to a document or a free-text source) and `occurred_at` as optional fields on every clinical version, each `null` with a reason when absent, never defaulted.
2. A "transcribed" flag that the UI shows next to the value, with the source scan one click away.
3. A downtime back-keying screen that asks for the event time first and refuses to default it to now.
4. Read-back confirmation (type the dose twice, or tick a displayed value) for a short, named list of dangerous fields, chosen by the Clinical Safety Officer, not by us.
5. Paste detection in free-text fields: keep a "pasted" span marker and a source reference, shown on read [2].
6. A document-index step with a required patient, date and type, and a second check of the patient banner against the scan before filing.
7. A post-merge review list and a count of merges later reversed, fed to the data-quality dashboard.
8. A measured baseline: sample N back-keyed entries against paper and report the error rate per field, rather than adopting a literature figure.
9. A read-only downtime extract (see [data exchange](../data-exchange/index.md), idea 9) that a ward can print.

## Open questions

- OQ-MS1. Does a transcribed entry need a different signing rule from a first-hand one?
- OQ-MS2. Who may back-key downtime entries, and is a second person's check required?
- OQ-MS3. Is a scan a clinical document with its own versions, or an attachment to another resource?
- OQ-MS4. Should the service refuse a version whose `occurred_at` is later than `recorded_at`?
- OQ-MS5. Is paste marking a clinical-safety feature or an annoyance that clinicians will work around?

## Not verified

- All error rates above are from other settings (research data capture, point-of-care testing) and may not hold for clinical entry in any specific hospital.
- The meta-analysis [3] was seen as a preprint via a search snippet; I did not read it.
- The Wilton and Pennisi 10.2% immunisation figure was seen only second-hand in the search summary of [1] and is deliberately not used.
- No NHS downtime or business-continuity source was read.
- The Joint Commission requirement wording was not checked at its own site [4].
- No source here has been reviewed by a clinician; the hazards are my reading, not a hazard analysis.

## Sources

1. Mays and Mathias, JAMIA 2019, manual transcription error in outpatient point-of-care testing (search snippet and citation list only): https://academic.oup.com/jamia/article/26/3/269/5287977 and https://www.ovid.com/journals/jamia/fulltext/10.1093/jamia/ocy170
2. Tsou, Lehmann, Michel et al., Safe practices for copy and paste in the EHR, Applied Clinical Informatics 2017;8:12-34 (abstract and summaries): https://pubmed.ncbi.nlm.nih.gov/28074211/ and https://psnet.ahrq.gov/issue/safe-practices-copy-and-paste-ehr-systematic-review-recommendations-and-novel-model-health-it
3. Systematic review and meta-analysis of error rates by data processing method, 2023 preprint (snippet only): https://www.researchsquare.com/article/rs-10878982 ; Wahi et al., paper-form transcription strategies: https://pmc.ncbi.nlm.nih.gov/articles/PMC2409998
4. SBAR handover, narrative review: https://link.springer.com/article/10.1186/s40886-018-0073-1 ; CASRAI guide (notes the unverified Joint Commission wording): https://casrai.org/guides/sbar-handoff-communication
5. NHS England and PCSE guidance on duplicates and confusions: https://pcse.england.nhs.uk/services/patient-registrations/duplicates-and-confusions/about-duplicates and https://pcse.england.nhs.uk/services/patient-registrations/duplicates-and-confusions/about-confusions ; NBO case study: https://digital.nhs.uk/services/personal-demographics-service/national-back-office-for-the-personal-demographics-service/data-quality-incidents/duplicate-registration-case-study (seen via a mirror)
6. HL7 FHIR Provenance as used in US Core: https://build.fhir.org/ig/HL7/US-Core/basic-provenance.html
