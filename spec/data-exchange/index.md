# Data exchange: getting data in and out

Public-source research read 2026-11-04 plus this repository's own files. A proposal: nothing here is built, and no clinician or data protection officer has reviewed it.

Question: how do we get data in and out, such as import and export? This file covers the machine paths (feeds, files, APIs, migrations) and the patient's right to a copy. People in the middle are covered in [manual steps](../manual-steps/index.md); how we judge and govern what arrives is in [data quality](../data-quality/index.md). It extends [interoperability](../interoperability/index.md) (standards and national packs), [audit](../audit/index.md) and [persistence](../10-persistence.md).

## The paths

**Live feeds (HL7 v2).** HL7 v2 messages are carried over MLLP, a TCP framing [1]. Typical feeds are ADT (admit, discharge, transfer), ORU (results) and SIU (scheduling). The sender expects an acknowledgement (accept, error, reject), and a message refused with no one watching is a silent loss.

**FHIR REST.** Search and read of single resources. Fine for small data; the Bulk Data IG notes that large exports can need hundreds of thousands of requests [2].

**FHIR Bulk Data `$export`.** The Bulk Data Access IG (2.0.0) defines a system-level `[base]/$export`, a patient-level `[base]/Patient/$export`, and a group-level form. They use the FHIR Asynchronous Request Pattern: the client asks, polls a status URL, then downloads files, by default NDJSON (`application/fhir+ndjson`). `_type` filters resource types, and a server declares conformance in `CapabilityStatement.instantiates` [2]. A patient-level export can include the latest or all related `Provenance` resources (a ballot-stage 4.0.0 draft of the IG, so not final) [3]. The IG is cited by US certification criterion 45 CFR 170.315(g)(10) [2]; the Australian Digital Health Agency lists 2.0.0 with "Trial" status [2]. Whether `Patient/$everything` returns Provenance is unclear in the base specification, per an open HL7 feedback ticket [3].

**Documents.** C-CDA documents, and IHE document sharing (XDS.b). IHE's "Sharing of IPS" profile exchanges the HL7 International Patient Summary over document sharing, built on FHIR and not CDA, and maps `Composition` elements to document-entry metadata [4]. I did not retrieve the IPS implementation guide itself, so its version and Provenance rules are unchecked.

**CSV and flat files.** Still the commonest way in for a legacy system and for reporting. No standard; every file needs a data dictionary and a checksum, and the mapping is where errors enter.

**Patient-facing export.** Three legal drivers, each different:
- HIPAA right of access (45 CFR 164.524): action within 30 calendar days with one 30-day extension on written notice; form and format requested, including electronic, if readily producible; cost-based fees; psychotherapy notes and litigation material excluded [5]. Seen through secondary sources only; the regulation text was not opened.
- UK GDPR Article 15 access: one calendar month in NHS trust guidance. Article 20 portability applies only to automated processing on consent (or contract) bases, and NHS guidance says public-task processing is outside it [6]. That is an NHS summary, not the legal text.
- US information blocking: ONC says that charging fees to export electronic health information to switch systems or to give patients their data does not qualify for the fees exception [7]. The EU European Health Data Space adds a portability right that does not depend on legal basis [8]; that is EU, not UK, law.

## Migration and cut-over

The most rigorous public source I found is a peer-reviewed migration of 10 data categories from over 300,000 patient records: automated checks for missing or extra records plus manual sampling, with the authors calling validation costly but critical for patient safety [9]. Vendor guides add four layers: row-level reconciliation (every source has a target with matching identifiers), field-level checks on critical fields, workflow validation by clinical users, and a rehearsed rollback [10]. They single out allergies and medications as safety-critical, advise a freeze window and parallel legacy access until validated [10]. They are marketing, and a "failure rate" repeated across them has no primary citation I could find; I do not use it.

Principles that follow, and that fit an append-only record:
1. **Import is a write, and is audited like one**, each with the source system, file, run and row.
2. **Import never overwrites.** Imported data becomes new versions with a Provenance naming the import run and the source; it does not edit what is there.
3. **Identity first.** Each imported patient goes through the matcher; a non-certain match goes to a review queue, never to an automatic link (a missing field blocks a link; it never counts as agreement).
4. **Reconcile before go-live.** Counts per resource type, per patient, and field-level agreement on a sample, with the report kept.
5. **Rollback in an append-only store** means marking a run's versions as withdrawn (a status and a Provenance), never deleting them.
6. **Unmapped is kept, not dropped.** Anything with no target stays in a raw-source store with the reason.

## Export safety

- Encrypt exports at rest and in transit; sign or checksum them; give the recipient a manifest.
- Apply consent and security labels to exports as to reads: a masked item says restricted data exists and carries none of it (spec 5.6).
- An export is a disclosure and a read of many patients: audit it as one, with purpose of use and the requesting agent.
- A bulk token should be short-lived and scoped; the status URL and files need the same access check as the request.

## Compared with this project

| Need | This project today | Gap |
| --- | --- | --- |
| HL7 v2 inbound | ADT A01/A04/A08/A28/A31 over MLLP (off by default), AA/AE/AR acknowledgements, review queue for refusals, raw message kept (`feed_refusals`) | ORU and SIU planned only; no outbound v2; MLLP over TLS not claimed |
| FHIR read | R4 Patient and AllergyIntolerance read facade, `metadata`; R5 as derived read | No other resources; no `_history` export; no `$everything` |
| Bulk `$export` | None | Nothing: no async pattern, no NDJSON, no status endpoint |
| Documents (C-CDA, XDS, IPS) | Named in the interoperability plan only | No document store, no IPS builder |
| CSV or flat-file import | None | No mapping tool, no dictionary, no dry-run |
| Legacy migration | None | No import run entity, no reconciliation report, no rollback design |
| Identity on import | Matcher with explainable scores; merge and unmerge audited | No batch mode, no review queue for imports (the feed review queue is for HL7 refusals) |
| Audit of import and export | Audit rule: the audit row is part of the write; reads audited | No row shape for a bulk run |
| Outbox | Append-only `outbox_events` and `outbox_attempts`; delivery loop off unless set | Nothing is enqueued yet |
| Patient right of access | Patients can see who accessed their record (planned) | No patient copy export, no timer for the 30-day or one-month deadline |
| Encrypted export | Per-tenant keys planned | No export encryption design |
| National standards as packs | Pack mechanism designed, not built | No pack exists |
| Retention of imported raw data | Retention schedule is draft; feed raw messages kept for ever | Raw import files have no class or period |

## Candidate hazards

For the Clinical Safety Officer, none accepted; none is yet in the hazard log.

- DE-H1. A migration drops, truncates or mis-maps an allergy or medication, so a clinician sees "none" where there was a record. Empty must never become "none".
- DE-H2. An imported patient is auto-linked to the wrong existing patient, mixing two records.
- DE-H3. An imported duplicate creates a second record for one patient, splitting history.
- DE-H4. A feed message is refused and nobody reviews the queue, so an admission or result is missing.
- DE-H5. An export includes data the patient or a proxy should not receive (third-party information, restricted labels), or omits data they are owed.
- DE-H6. Units, codes or timezones are changed in transit, so a value is read at the wrong scale or time.
- DE-H7. A rollback or a re-run duplicates or hides clinical versions.
- DE-H8. An export file is lost or sent to the wrong recipient.

## Ideas

1. An **import run** entity: id, source system, file checksum, mapping version, operator, start and end, counts in/out/refused, status. Every imported version carries its run id in Provenance.
2. A dry-run mode that matches and maps but writes nothing, and produces the reconciliation report.
3. A review queue for import identity decisions, reusing the shape of the feed refusal queue.
4. A reconciliation report as a kept artefact: counts by type, a field-level sample, and every refused or unmapped row.
5. Withdrawal of a run as an append-only status with a Provenance, and a UI that hides withdrawn versions but can show them.
6. `$export` first as system-level and patient-level, async, NDJSON, `_type`, with Provenance included; declare it in `metadata` only when a validator passes (the pack rule).
7. A patient copy export in a human-readable form plus FHIR JSON, with a due-date timer by jurisdiction (a pack setting).
8. Export manifests with SHA-256 checksums, and encryption with a key the recipient holds.
9. A read-only "downtime extract" per ward or clinic, produced on a schedule (links to [manual steps](../manual-steps/index.md)).
10. Raw-source retention class for import files, decided by the controller, defaulting to kept.
11. Country packs name their own import and export profiles (for example UK Core, US Core, AU Core), per the existing pack rules [see interoperability].

## Open questions

- OQ-DE1. Is an import run a clinical write (hazard-log scope) or an administrative one?
- OQ-DE2. Should bulk export require a second approver, as a bulk read of many patients?
- OQ-DE3. How are third-party and restricted items handled in a patient copy?
- OQ-DE4. What is the minimum legacy format a first deployment must accept (CSV, HL7 v2 batch, C-CDA)?
- OQ-DE5. Who owns the mapping tables, and do they need clinical review?
- OQ-DE6. Is rollback by withdrawal acceptable to a records manager when the record is meant to be append-only?

## Not verified

- The Bulk Data IG pages were seen as a search summary of the Argonaut-branch build, which is not an authorised publication; the 2.0.0 normative text, and the 4.0.0 ballot, were not read.
- HIPAA and UK GDPR points come from secondary summaries; the legal text was not opened. Time limits and fees may have changed.
- The IPS implementation guide and the MLLP specification were not retrieved.
- The migration sources are mostly vendor content; only [9] is peer-reviewed style.
- No national standard pack, other than UK Core and US Core as references, was checked.
- Link check 2026-11-04: these cited pages returned 404. They may have moved, but what rests on them is unconfirmed: https://build.fhir.org/ig/HL7/bulk-data/branches/argo24/OperationDefinition-export.html

## Sources

1. FHIR endpoint connection types (HL7v2 over MLLP described), via Sequoia Project: https://sequoiaproject.org/SequoiaProjectHealthcareDirectoryImplementationGuide/output/ValueSet-EndpointConnectionType.html
2. HL7 Bulk Data Access IG (export operation; trial status listings): https://build.fhir.org/ig/HL7/bulk-data/branches/argo24/OperationDefinition-export.html and https://developer.digitalhealth.gov.au/standards/hl7-fhir-implementation-guide-bulk-data-access-implementation-guide-v2-0-0
3. Bulk Data patient export and Provenance handling: https://build.fhir.org/ig/HL7/bulk-data/en/OperationDefinition-patient-export.html ; $everything ticket: https://jira.hl7.org/browse/FHIR-33173
4. IHE Sharing of IPS: https://profiles.ihe.net/ITI/sIPS/volume-1.html
5. HIPAA right of access (secondary): https://canons.sog.unc.edu/?p=3297 ; rule text: https://www.govinfo.gov/content/pkg/CFR-2010-title45-vol1/pdf/CFR-2010-title45-vol1-sec164-524.pdf (2010 edition, not current)
6. NHS England information governance, shared care records appendix on rights: https://digital.nhs.uk/data-and-information/information-governance/guidance/information-governance-framework-shared-care-records/appendix-4 (seen via a mirror)
7. ONC information blocking FAQ on fees for export: https://healthit.gov/faq/are-contractual-fees-export-electronic-health-information-ehi-using-technology-not-certified-45/
8. EIPA on EHDS data portability: https://www.eipa.eu/blog/data-portability-in-the-european-health-data-space/
9. Large EHR migration with validation (PMC): https://pmc.ncbi.nlm.nih.gov/articles/PMC11741007
10. EHR migration guides (vendor): https://www.mindbowser.com/ehr-data-migration/ and https://ehrsource.com/articles/ehr-data-migration-checklist/
