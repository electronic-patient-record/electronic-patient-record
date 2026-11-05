# Death assertions

The rules for recording that a person has died (BV-1). A first draft for clinical and legal review: no clinician, medical examiner or bereaved person has reviewed it. The reasons and sources are on [bereavement](index.md).

**Built:** the rules and the read model in `electronic-patient-record-service-with-rust/src/death.rs`, and the append-only store in `death_store.rs` (table `death_assertion_versions`, view `death_assertion_current`, migration 25). The routes are in `controllers/deaths.rs`. The banner on the patient and summary pages says so, and `DeathSection.svelte` reports and takes steps. An HL7 message with PID-29 or PID-30 stores a report, as below. FHIR is below. **The hold:** the outbox runner holds any event about a patient with a standing assertion, as below. Nothing enqueues and no sink exists, so nothing is held in use today.

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

## Reports from a feed

An HL7 ADT message with PID-29, or PID-30 of `Y`, stores a report from that feed and is answered `AA`. It never changes the identity record. HL7's `""` null is not a report, and a date beside an indicator of `N` is, under rule 1.

| Case | What happens |
|---|---|
| A patient matches and this is new | A report is stored, with the rest of the message, in one transaction |
| The date cannot be read, or is in the future | A report with no date, saying why, because a report still holds contact |
| This feed already said this, and it stands | Nothing new: a source sends PID-29 on every later message |
| A person withdrew this same report as an error | `AE`, and it waits in the feed queue. It does not come back by itself, and it is not dropped either |
| The same feed gives a different date | A new report |
| No patient matches | `AE`, and it waits in the queue: a death does not register a patient |
| The message cannot be applied for another reason, or its event is not handled, such as `A03` | Refused as before, and the refusal says the death was not recorded |

A feed cannot retract a report: a later message with an indicator of `N` changes nothing. A report waits for a person to verify it; nothing tells anyone that one has arrived, except the banner when the record is opened.

## The hold

`outbox.rs` asks for the one answer before anything else about an event. While it holds, the event is not delivered, writes no attempt and stays pending; a withdrawal that leaves nothing standing releases it, and it goes at the next run. A hold is not a refusal, because a refusal is final. A failed look is a failed attempt, never a delivery. Held events do not use up a run. `tests/outbound_census.rs` fails if a new way out appears without a hold.

Every event about the patient is held, whatever it is. That needs review: a message that must reach someone after a death, such as telling the patient's GP, is held too, and no route yet says which may go. The contact a person makes by hand is held only by the banner's words. Nothing shows a person what is held, or how long it has waited.

## A proxy

A proxy has no relationship to a patient while any assertion stands: an unconfirmed report is enough, because a disclosure cannot be undone. `relationships::lookup` works this out on each request, so the relationship itself is not ended and a withdrawal restores it with nothing to do by hand. This covers the record, the photo, the access log and "my record", which leaves the patient out.

The cost is that a false report locks out a parent or carer until a clinician withdraws it, and they see only that the record is gone, because the list says nothing to a proxy about a death. Those words need people who have been through it. A patient's own login does not stop, because a living person wrongly reported as dead would lose their own record. The [bereavement](index.md) page asks for it to stop as well; that is not settled. A person acting after death, with a basis, a scope and an end date, is not built.

## FHIR

| The one answer | R4 and R5 `Patient` on read and search |
|---|---|
| No assertion stands | Nothing, which says nothing either way |
| Reported, not confirmed | The `meta.tag` `death-reported` in `urn:epr:record-kind`, and no `deceased[x]` |
| Confirmed, with one date | `deceasedDateTime`, as precise as it was given |
| Confirmed, with no date or dates that disagree | `deceasedBoolean: true` |

An unconfirmed report is not `deceased[x]` because any other system takes that as a fact, and a wrong report would spread to it. A patient and a proxy are not told, on any route. A failed look answers 500 rather than leaving the element out, which would read as nothing reported.

A `Patient` write cannot report, change or retract a death, because a report has an author and a reason. A write with `deceased[x]` gets a 422 that names the report route. The exception is an update that sends back exactly what a read gave, which changes nothing about the death, so that a client that reads, edits and writes keeps working. Leaving the element out of an update changes nothing either.

## The one answer

All of a patient's assertions, each at its current version, give one answer:

| Assertions that stand | Answer | Holds contact |
|---|---|---|
| None | `not-known-to-have-died` | No |
| One or more | `asserted`, with the furthest status, whether it is confirmed, the date from the furthest assertion that has one, and `dates_differ` if those that stand disagree | Yes |

`not-known-to-have-died` never means the person is known to be living, so no screen shows the word "alive". A withdrawal releases the hold only when no other assertion stands. A report on a record merged away counts for the surviving record.

## Not settled

- **A date of death in the matcher (OQ-BV3, decided, not in the service).** The matcher's `Patient` has `death_date` as weak match evidence, in the unreleased crate. To use it the service needs that release, must fill the field only from a standing assertion, and must never accept it from a client. The assertion stays apart from identity.
- **How soon a held contact is reviewed, and who owns that.** OQ-BV1 says a person confirms; it does not say by when.
- **Other fields on [the proposal](index.md#what-this-project-would-need).** `found_at`, `place`, `verified_at` and `coroner_referred` are not in this slice. They can be added to the stored body without a migration, once someone has said which are required.
