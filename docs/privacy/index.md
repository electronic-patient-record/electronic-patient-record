# Privacy: what it means here

Public-source research read 2026-11-04 plus this repository's own files. A proposal and a reading: targets here are not commitments, and nothing was measured by the author of this note unless it says so.

## Definition

ISO/IEC 25010:2023 has no separate privacy characteristic. Privacy sits mainly under security's confidentiality (data accessible only to those authorised), with accountability and authenticity supporting it; the 2023 model also treats some user-facing privacy matters under interaction capability and safety (secondary summaries only: [Sonar](https://www.sonarsource.com/resources/library/iso-iec-25010-explained/), [arc42](https://quality.arc42.org/standards/iso-25010); I did not read the standard). ISO/IEC 27701 is the privacy extension to ISO 27001 ([ISO](https://www.iso.org/standard/71670.html), from memory).

Here privacy means more than secrecy: lawful basis, purpose limitation, data minimisation, accuracy, storage limitation, rights of the person (access, correction, erasure, objection, portability), and transparency. The legal frames are the UK GDPR and Data Protection Act 2018 ([ICO guide](https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/)), the EU GDPR, particularly Article 9 on health data ([GDPR text](https://gdpr-info.eu/art-9-gdpr/)), and in the US the HIPAA Privacy Rule ([HHS](https://www.hhs.gov/hipaa/for-professionals/privacy/index.html)) and 42 CFR Part 2 for substance use disorder records ([eCFR](https://www.ecfr.gov/current/title-42/chapter-I/subchapter-A/part-2)). Privacy by design and default is GDPR Article 25 ([GDPR text](https://gdpr-info.eu/art-25-gdpr/)).

## Why it matters for an EPR

- Every record is special-category data; a leak harms dignity, employment and safety, and cannot be undone.
- Snooping by authorised staff is a recurring finding in regulator actions; patient-visible access logs are a deterrent (this is my reading; no count was collected).
- The ICO fined Advanced about 3.07 million pounds after a 2022 ransomware attack that exposed personal data of NHS patients ([Computing](https://computing.co.uk/news/2025/security/ico-fines-advanced), snippet only).
- In England, the national data opt-out lets patients opt out of use of their data for research and planning ([NHS England](https://digital.nhs.uk/services/national-data-opt-out)); the NHS Records Management Code of Practice sets retention periods ([NHS England](https://transform.england.nhs.uk/information-governance/guidance/records-management-code/)).
- Erasure under GDPR Article 17 has an exemption for health care and public health, so a clinical record is normally retained, not erased ([GDPR text](https://gdpr-info.eu/art-17-gdpr/)); how that applies is for the controller.
- Children, people lacking capacity and proxies need special handling.

## What the project says today

- `governance/dpia.md` is a DRAFT, "needs a data protection officer"; no data protection officer, Clinical Safety Officer or privacy lead has reviewed it; residual risk is not assessed; the controller, lawful basis and Article 9 condition are "controller to decide".
- `governance/personal-data-inventory.csv` (columns: table, column, category, purpose, retention_class) classifies every database column, and `tests/personal_data_inventory.rs` fails if a column is missing. `governance/record-of-processing.md` and `governance/data-flow.md` are drafts.
- `governance/retention-schedule.md`: **no period is set** for any class; the record, audit and feed classes are kept for ever; `axa_idempotency` keys are deleted after 24 hours (unreviewed). No deletion job exists, and none should be written before the erasure-versus-append-only question is decided.
- Access: policy engine with care relationship, purpose of use and consent for clinical content (SE-1, SE-2 partly: identity only); national data opt-out and 42 CFR Part 2 via an `SUD` label are specified in `spec/access-control/index.md`, with sensitive masking (SE-4) not built; proxies get only granted scopes; break-glass is reviewed.
- Minimisation: audit rows hold no values, unit, range or request text for observations and amendments (dpia section 3). NFR-12 forbids scores that reduce care.
- Patient rights: the patient sees an access log (SE-6, partly: staff by role and organisation, not by name); a patient can ask for a correction (amendment requests), with the 60 and 30 day limits described as "unreviewed guesses". A proxy cannot ask for a correction. No subject access request, portability export or erasure route is described as built.
- `spec/12-compliance.md`: all patient data is special-category and PHI; "supports compliance; cannot be compliant"; no certification claimed.
- Open per the DPIA: raw refused HL7 messages kept with no period; hosting location, sub-processors, international transfers; photo biometric status; no patient consultation.
- Rule: a missing field is `null` with a reason, never zero (AGENTS.md). Data minimisation on the wire is not described beyond the audit rows.

## Measures

| Measure | How to measure | Suggested target or rule for setting one | Where in our data/CI |
| --- | --- | --- | --- |
| Column classification coverage | Columns missing from the inventory | Zero | `tests/personal_data_inventory.rs` (built) |
| Retention classes with a set period | Count of classes with a reviewed period | All, before real data | `retention-schedule.md` (0 set) |
| Data-subject request time | Days from request to answer, per type | Law gives one month in UK GDPR ([ICO](https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/individual-rights/)); set internal target below that | None; no route built |
| Access-log availability to patients | Share of patient reads showing the log | 100 per cent of patient accounts | SE-6 tests |
| Unjustified access found | Audit review findings per 1,000 accesses | No fixed number; trend, with review of every break-glass | `break_glass_reviews`, `/api/audit` |
| Consent-masking correctness | Test cases of denied reads leaking existence | Zero | `tests/` policy and consent tests |
| Personal data in logs | Sampled log lines containing identifiers | Zero in logs and metrics labels | Not audited (data-flow note) |
| DPIA currency | Age of last sign-off against last material change | Reviewed on each new data type | `dpia.md` (not signed) |
| Third-party recipients | Count of external calls from UI and service | Zero undeclared | CSP; no audit beyond it |

## Trade-offs with other attributes

- Privacy versus auditability: the audit trail is itself personal data, kept for ever by default; minimal rows help ([auditability](../auditability/index.md)).
- Privacy versus safety: masking a sensitive item may hide an allergy or drug interaction from a clinician; break-glass is the valve ([safety](../safety/index.md)). Privacy versus functional suitability and interoperability: consent filters change search and FHIR results ([interoperability](../interoperability/index.md)).
- Append-only record versus erasure and correction: handled by "entered in error" versions, not deletion.
- Privacy versus reliability: backups copy personal data and prolong its life ([reliability](../reliability/index.md)). Versus security: see [security](../security/index.md).

## Gaps and risks

- No retention periods, no erasure or redaction policy, no subject access or portability route, no DPO opinion.
- Feed refusals store whole HL7 messages with no period.
- Consent is identity only so far (SE-2); masking not built; so the access model is partly design.
- No statement on international transfers, so a worldwide-equitable project has no per-region privacy mapping (UK, EU, US, others).
- Machine translations of notices are unreviewed.

## Ideas

1. Set draft retention periods per class from the NHS Records Management Code, labelled unreviewed, so the controller starts from something (AS-2).
2. Decide the erasure policy options in writing: retain under exemption, restrict, redact in a derived view, crypto-shred a per-tenant key (needs SE-5 and per-tenant keys). Record as an open question for the privacy lead.
3. Build a subject access export (FHIR bundle of the patient's record and access log), audited.
4. Add a short retention for `feed_refusals` raw messages with a redaction step.
5. Add a log-content test: no identifier appears in `tracing` output for the main routes.
6. Map regions: a table of UK GDPR, EU GDPR, HIPAA, and a few others with rights and deadlines (plan.md section 12 primary-source checks, AS-11).
7. Let proxies request corrections (gap in DPIA).
8. Name the first Article 9 condition candidates for the controller without choosing for them.

## Open questions

- Who is controller and who is processor in a hosted deployment of an open-source product?
- Is the patient photo biometric data in this use?
- Do sensitive labels follow the patient across tenants?
- What does the equity rule mean where no data-protection law exists?

## Not verified

- ISO 25010:2023 treatment of privacy is from secondary summaries and my reading.
- GDPR, HIPAA and NHS pages are cited from memory and were not re-read this session; deadlines and exemptions must be checked by a privacy lawyer.
- The one-month response time and the Article 17 health exemption are my recollection.
- No legal advice is given or implied.

## Sources

- [Sonar](https://www.sonarsource.com/resources/library/iso-iec-25010-explained/), [arc42](https://quality.arc42.org/standards/iso-25010): 25010 summaries.
- [ISO/IEC 27701](https://www.iso.org/standard/71670.html): privacy information management.
- [ICO UK GDPR guidance](https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/) and [individual rights](https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/individual-rights/): UK law guidance.
- [GDPR Art 9](https://gdpr-info.eu/art-9-gdpr/), [Art 17](https://gdpr-info.eu/art-17-gdpr/), [Art 25](https://gdpr-info.eu/art-25-gdpr/): special category, erasure, by design.
- [HHS HIPAA Privacy Rule](https://www.hhs.gov/hipaa/for-professionals/privacy/index.html): US.
- [eCFR 42 CFR Part 2](https://www.ecfr.gov/current/title-42/chapter-I/subchapter-A/part-2): substance use records.
- [NHS national data opt-out](https://digital.nhs.uk/services/national-data-opt-out): England opt-out.
- [NHS Records Management Code](https://transform.england.nhs.uk/information-governance/guidance/records-management-code/): retention.
- [Computing, ICO fines Advanced](https://computing.co.uk/news/2025/security/ico-fines-advanced): fine after ransomware.
