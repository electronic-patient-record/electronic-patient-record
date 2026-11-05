# Death assertions

The rules for recording that a person has died (BV-1). A first draft for clinical and legal review: no clinician, medical examiner or bereaved person has reviewed it. The reasons and sources are on [bereavement](index.md).

**Built:** the rules and the read model in `electronic-patient-record-service-with-rust/src/death.rs`, and the append-only store in `death_store.rs` (table `death_assertion_versions`, view `death_assertion_current`, migration 25). The routes are in `controllers/deaths.rs`. The banner on the patient and summary pages says so, and `DeathSection.svelte` reports and takes steps. **Not built:** a feed that writes a report, or anything that a hold acts on. Until a feed can record a death, a report that arrives is refused rather than dropped: an HL7 message with PID-29 or PID-30 is answered `AE`, changes nothing and waits in the feed queue for a person, and a FHIR write with `deceased[x]` gets a 422. The service has no outbound contact yet, so nothing is held today.

## The model

A death is an assertion with evidence, not a true-or-false field on the identity record. One patient can have several assertions, from different sources, and each may be wrong.

| Status | Meaning | Holds contact | Confirmed |
|---|---|---|---|
| `reported` | Someone or something said so | Yes | No |
| `verified` | A person established that the patient has died | Yes | Yes |
| `certified` | The cause has been certified; the cause is a separate, later fact | Yes | Yes |
| `registered` | The death has been registered | Yes | Yes |
| `withdrawn-in-error` | A person found the assertion wrong | No | Final |

## Rules

1. **Any report holds contact at once**, whoever or whatever made it, including a feed ([OQ-BV1](../16-open-questions.md)).
2. **Only a person moves an assertion on.** A feed can report. It cannot confirm or withdraw, even when the source is a registrar.
3. **Steps go forward only**, and may be skipped. A person told by a registrar can go from `reported` to `registered`.
4. **Withdrawal is possible from any status and is final.** The assertion stays in the history and stops counting. A new report is a new assertion.
5. **Every report and step needs an author and a reason.** The reason is not optional on a withdrawal: a corrected record with no reason is the failure the US audit found ([bereavement](index.md#a-death-recorded-in-error)).
6. **A date may be `YYYY`, `YYYY-MM`, `YYYY-MM-DD` or an RFC 3339 instant, or absent with a reason.** It cannot be later than now; a partial date is compared at its start.

## Who may do what

Decided by the maintainer as OQ-BV2, not yet reviewed by a Clinical Safety Officer.

| Role | The one answer | The assertions, with reasons | Report | Step forward | Withdraw |
|---|---|---|---|---|---|
| Clinician | Yes | Yes | Yes | Yes | Yes |
| Nurse | Yes | Yes | Yes | Yes | No |
| Pharmacist, allied health, receptionist, clinical administrator | Yes | No | Yes | No | No |
| Integration | No | No | Yes, as a feed | No | No |
| Patient, proxy, auditor, Clinical Safety Officer | No | No | No | No | No |

The decision names verifying and withdrawing. `certified` and `registered` are read as the same kind of step as verifying. No care relationship is needed, because reception must see the answer before ringing a family and the staff who confirm a death are often not on the care team. The reasons are kept from the wider group because they are free text and may hold clinical detail. A refused consent stops a read of the answer, as it stops any read by staff; this needs review, because a banner that cannot answer is less safe than one that can.

## Routes

| Route | Does |
|---|---|
| `GET /api/patients/{id}/death` | The one answer, and `holds_contact` |
| `GET /api/patients/{id}/death/assertions` | Every assertion, newest first, withdrawn ones included |
| `POST /api/patients/{id}/death/assertions` | Report: `{source, occurred, reason}`. The reporter is the token |
| `PUT /api/patients/{id}/death/assertions/{assertion_id}` | One step: `{expected_version, to, reason}` |

A stale `expected_version` is a 409 and a refused step a 422. Every read and write is audited; the audit row holds ids, versions and statuses, never a reason or a date.

## The one answer

All of a patient's assertions, each at its current version, give one answer:

| Assertions that stand | Answer | Holds contact |
|---|---|---|
| None | `not-known-to-have-died` | No |
| One or more | `asserted`, with the furthest status, whether it is confirmed, the date from the furthest assertion that has one, and `dates_differ` if those that stand disagree | Yes |

`not-known-to-have-died` never means the person is known to be living, so no screen shows the word "alive". A withdrawal releases the hold only when no other assertion stands. A report on a record merged away counts for the surviving record.

## Not settled

- **A date of death in the matcher (OQ-BV3, decided, not built).** The matcher's `Patient` will carry a date of death as weak match evidence. The assertion stays apart from identity.
- **How soon a held contact is reviewed, and who owns that.** OQ-BV1 says a person confirms; it does not say by when.
- **Other fields on [the proposal](index.md#what-this-project-would-need).** `found_at`, `place`, `verified_at` and `coroner_referred` are not in this slice. They can be added to the stored body without a migration, once someone has said which are required.
