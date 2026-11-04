# Data quality assurance and governance

Public-source research read 2026-11-04 plus this repository's own files. A proposal: nothing here is built, and no clinician or data protection officer has reviewed it.

Question: what data quality assurance and governance? Quality is not a feature of one screen. It comes from how data is captured ([manual steps](../manual-steps/index.md)), how it arrives ([data exchange](../data-exchange/index.md)), and who answers for it afterwards. See also [audit](../audit/index.md), [patient identity](../patient-identity/index.md), [domain model](../05-domain-model.md) and the [retention schedule](../../governance/retention-schedule.md).

## Frameworks in public use

**Kahn et al. (2016).** A harmonised terminology for secondary use of EHR data: three categories, conformance (does it match its defined format and rules), completeness (how often attributes are present, regardless of value) and plausibility (are values believable), assessed in two contexts, verification and validation [1]. A German consortium tool aligned its checks to it [1]. I read the abstract and summaries, not the table of definitions.

**ISO/IEC 25012 (2008).** A data quality model of 15 characteristics, from two points of view: inherent (about the data itself, such as accuracy, completeness, consistency, credibility, currentness) and system-dependent (about the computing environment, such as backup and migration) [2].

**ISO 8000-61.** A process reference model for managing data quality: planning, control, assurance and improvement, related to data governance [3]. I did not check whether a newer edition than 2016 exists, though a 2023 adopted national version is listed [3]. One practitioner source calls 25012 (what to measure) complementary to 8000 (how to manage); that is opinion, not either standard [2].

**DAMA-DMBOK.** The body of knowledge describes data quality dimensions and data governance roles. Secondary sources cite 15 dimensions in four groups and three roles: data owner, data steward and data custodian, with the owner settling conflicts and the steward monitoring quality, fixing issues and carrying out improvement plans [4]. These are vendor and wiki summaries; I did not see DMBOK itself.

**NHS England Data Quality Maturity Index (DQMI).** A monthly composite score per provider, from coverage, completeness, validity and use of excessive default values at data-item level across eight national datasets. Meaningless defaults such as "Not Known" are removed from the valid measure; new items are scored experimentally before entering the standard list; it is referenced in the NHS Standard Contract and supports a Data Quality Improvement Plan; the statutory basis cited is section 266 of the Health and Social Care Act 2012 [5]. Seen as a search summary of the methodology pages, not the full methodology.

**Duplicates and confusions.** In England the National Back Office for the Personal Demographics Service merges duplicate NHS numbers and resolves "confused" records (two patients on one number); duplicates often start when a registration is created without checking for an existing number, and NHS sources describe the risks as split histories, wrong prescriptions and missed referrals [6].

**FHIR Provenance.** Records the activity, agents and entities behind a version of a resource and points at its target, so it can describe any version, including past ones. A `_revinclude=Provenance:target` search returns resources with their provenance [7].

## Entry versus batch

- **At entry** (conformance): format, code-set membership, required fields, range and unit checks, cross-field logic. Cheap, immediate, and it gives the writer a chance to fix. A hard stop on a missing field produces made-up values, so the project's rule applies: absent is `null` with a reason, never zero or a default. The DQMI treats excessive defaults as a defect in itself [5].
- **In batch** (completeness, plausibility, uniqueness): rates over time, outliers, duplicates, orphaned references, feed lag, and drift between a source system and this one. Reported to the people who can fix the cause.
- **Both feed the same issue queue.**

## Governance

- **Named roles.** A data owner (an accountable clinical or corporate lead per domain), stewards (day-to-day correction and rule maintenance) and custodians (the technical team) [4]. In the NHS these map to local roles such as information asset owner and Caldicott Guardian; I did not check that mapping in a public source.
- **A data dictionary** per field: definition, code system, allowed values, owner, source, whether absence is allowed, and the DQ rule that applies. For this project the schema map (`spec/schema-map.md`) and the personal data inventory are the start.
- **A governance board** that triages issues, approves rule changes and reviews the dashboard, with the Clinical Safety Officer present for anything that could change what a clinician sees.
- **Issue triage**: severity by clinical consequence, not by count; an owner and a due date; a link to the hazard log where a hazard is plausible.

## Correction in an append-only record

Data quality work normally edits data. Here it cannot. A correction is a new version that references the one it corrects, with a Provenance giving the reason, and the old version stays readable. A wrong-patient entry is corrected by marking it entered-in-error and filing the true content to the right patient, both audited; neither deletes. Retention periods are not set and no deletion path may be added ([retention schedule](../../governance/retention-schedule.md)). The amendment-request workflow already built is the entry point for patient-raised corrections.

## Compared with this project

| Need | This project today | Gap |
| --- | --- | --- |
| Entry validation | Types and invariants in the Rust service; SNOMED identifier check; HL7 ADT parsing with accept/error/reject | No catalogue of rules; no per-field dictionary |
| Absent is not zero | Stated rule; `null` with reason | Not measured; no census of defaults |
| Batch DQ checks | None | No jobs, no rule engine, no results store |
| Duplicate and overlay monitoring | Matcher scores; duplicate check on create; merge and unmerge routes, audited | No scheduled duplicate sweep; no reversal-rate metric; no overlay detection |
| DQ dashboard | Metrics counters (for example audit append failures) | No clinical-data quality view, no DQMI-style score |
| Provenance | Planned on every version; audit shape is IHE BALP | Provenance not yet general; no import or transcription roles |
| Data dictionary | `spec/schema-map.md` and `personal-data-inventory.csv` | No semantic dictionary (definition, owner, rule) |
| Stewardship roles | Stakeholders chapter; Clinical Safety Officer role | No data owner or steward roles defined |
| Governance board | Change control (spec 18); hazard log | No data quality board or terms of reference |
| Correction | Append-only versions; amendment requests with open/resolve (migration 15) | Entered-in-error semantics not specified or tested |
| Issue triage | Hazard log and feed refusal queue | No general DQ issue register |
| Retention | Draft schedule, periods not set | Not decided; erasure against append-only unresolved |

## Candidate hazards

For the Clinical Safety Officer, none accepted; none is yet in the hazard log.

- DQ-H1. A duplicate record hides part of a patient's history (allergy on one, medication on another).
- DQ-H2. An overlay or confusion puts one patient's data in another's record, and nobody notices because no sweep runs.
- DQ-H3. A rule rejects valid unusual data (a mononym, a partial date of birth), so a patient cannot be registered or a value is forced to a default.
- DQ-H4. A default value (for example "unknown" coded as a real value) is read as evidence.
- DQ-H5. A correction creates a second "current" value, or the old value is still shown.
- DQ-H6. A good aggregate score (high completeness) hides a dangerous gap in one clinical field.
- DQ-H7. A dashboard is read as clinical truth when it measures only form.

## Ideas

1. Adopt the Kahn three categories as the vocabulary for rules (conformance, completeness, plausibility), so every rule has a category and a context [1].
2. A rule catalogue in the repository, one row per rule: id, field, category, entry or batch, severity, owner, test.
3. A data dictionary generated from the schema and extended by hand with owner and definition; a CI check that fails when a column has no entry.
4. A DQMI-style per-deployment score of completeness, validity and default use for chosen fields, published as a number with the method, and never a single composite alone. Fields chosen by clinicians.
5. A weekly duplicate sweep using the matcher at batch scale, listing pairs above a threshold for a steward, with a count of merges later reversed.
6. A Provenance profile with agent roles: author, informant, transcriber, importer, and `source` pointing to an import run.
7. A `data_quality_issue` register, append-only, with severity, owner, status, hazard link and resolution.
8. A governance board terms of reference, with a stated place for patients and for a data protection officer.
9. Entered-in-error as a status with a required reference to the corrected version.
10. A census test (in the manner of the existing route census) that fails if a new clinical field has no dictionary row or no rule.
11. A rule that a quality metric is never shown without its denominator.

## Open questions

- OQ-DQ1. Who is the data owner for each clinical domain in a deployment, and does the project ship a default?
- OQ-DQ2. Which fields are safety-critical for scoring, and who decides?
- OQ-DQ3. Should batch checks run inside the service or in a separate reporting store?
- OQ-DQ4. Does a quality score belong in the product at all, or in the deployer's reporting?
- OQ-DQ5. How should entered-in-error interact with a patient's right to rectification and with retention?
- OQ-DQ6. Which national measures (DQMI is English) become packs, and how are non-UK equivalents found?

## Not verified

- DQMI, ISO 8000-61 and ISO/IEC 25012 were seen through catalogue pages and search summaries; the standards and the full methodology were not read, and ISO 25012 and 8000 are paid documents.
- DAMA roles and dimension counts come from wiki and vendor summaries and may not match DMBOK.
- The Kahn definitions were read from abstract level.
- The NHS role mapping (information asset owner and so on) is from my knowledge, not a source.
- No clinician has reviewed these ideas; severity and scoring choices are for them.

## Sources

1. Kahn et al., A Harmonized Data Quality Assessment Terminology and Framework for the Secondary Use of EHR Data, eGEMS 2016: https://pubmed.ncbi.nlm.nih.gov/27713905/ and https://pmc.ncbi.nlm.nih.gov/articles/PMC5051581
2. ISO/IEC 25012 summaries: https://quality.arc42.org/standards/iso-iec-25012 and https://iso25000.com/index.php/en/iso-25000-standards/iso-25012
3. ISO 8000-61 catalogue entries: https://www.dinmedia.de/en/standard/iso-8000-61/266302693 and https://iteh.es/catalog/standards/iso/0b5737db-6738-48ff-8c0e-0c083adf7194/iso-8000-61-2016
4. DAMA-based role and dimension summaries (secondary): https://datamanagement.wiki/_export/raw/role/data_steward and https://dqs-website.pages.dev/resources/best-practices/data-governance-framework
5. NHS England DQMI methodology: https://digital.nhs.uk/data-and-information/data-tools-and-services/data-services/data-quality/data-quality-maturity-index-methodology
6. PCSE and National Back Office on duplicates and confusions: https://pcse.england.nhs.uk/services/patient-registrations/duplicates-and-confusions/about-duplicates and https://pcse.england.nhs.uk/services/patient-registrations/duplicates-and-confusions/about-confusions
7. US Core Provenance guidance: https://build.fhir.org/ig/HL7/US-Core/basic-provenance.html
