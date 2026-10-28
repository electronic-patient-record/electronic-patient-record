# Data Protection Impact Assessment (DRAFT, needs a data protection officer)

**NOT FOR CLINICAL USE.** This is a scaffold (milestone M0). Facts below come from the code and the specs; judgements are marked **[controller to decide]**. Nothing here is legal advice, and no data protection officer, Clinical Safety Officer or privacy lead has reviewed it.

## 1. What is processed, and why

A clinical record: patient identity (with a photo), consents, care and proxy relationships, allergies, vital signs, patients' requests to correct a record, and an audit trail of every access. Staff use it to identify a patient and read the record; a patient can read their own record and ask for a reading to be corrected. Purpose of use is declared on each request. The legal basis and the controller are **[controller to decide]**.

Every database column is classified in [`personal-data-inventory.csv`](personal-data-inventory.csv), and `tests/personal_data_inventory.rs` fails if one is not. Retention classes are in [`retention-schedule.md`](retention-schedule.md); **no period is set**.

## 2. Special-category data

Health data (allergies, vital signs, the raw HL7 messages kept when the feed refuses one), a photo of the patient (biometric when used to identify), and what a patient says about their own record. Children and people lacking capacity are within scope through proxy relationships. **[controller to decide]**: the Article 9 condition, and whether a child's record needs a further assessment.

## 3. Controls that exist

- **Who may see what:** every route has a policy entry (`tests/route_census.rs`); a care relationship, a declared purpose and the patient's consent are required for clinical content; a proxy has only the scopes it was given and none for observations or amendments.
- **Emergency access:** break-the-glass opens a grant with a reason and an expiry, and each grant is reviewed (`break_glass_reviews`).
- **Audit:** every read and write is audited in the same transaction as the change; `audit_events` is append-only (database triggers) and hash-chained. Audit rows hold no values, unit, range or request text for observations and amendments.
- **No deletion:** the clinical record is append-only; a mistake is a new version marked entered in error.
- **Input hardening:** a seeded chaos test sends hostile input to every route (no 5xx, no leaked internals).
- **Patient access:** the patient sees the access log for their own record.
- **Transport and storage:** TLS and disk encryption are the deployer's responsibility.

## 4. Risks and gaps

| Risk | State |
| --- | --- |
| Deletion and erasure requests conflict with an append-only record | Open: no policy. See the retention schedule |
| The HL7 MLLP listener has no authentication or encryption (off unless configured) | Documented; do not expose |
| Raw refused HL7 messages are kept with no period | Open |
| No multi-factor sign-in requirement is shipped in the development realm | Open |
| A proxy cannot ask for a correction; a cross-patient alert of overdue requests does not exist | Not built |
| Time limits for correction requests (60 and 30 days) and refusal grounds are unreviewed guesses | Unreviewed |
| Photo is not checked to be of the patient | Not built |
| Translations are machine-written and unreviewed | Unreviewed |
| Hosting location, sub-processors and international transfers | **[controller to decide]** |
| Penetration test, accessibility audit, usability testing | Not done |

## 5. Consultation and sign-off

Not done: no patient or clinician consultation, no data protection officer opinion, no sign-off. Residual risk is **not assessed**; this document is a list of facts and gaps to start from.
