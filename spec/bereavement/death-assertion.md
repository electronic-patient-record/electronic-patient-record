# Death assertions

The rules for recording that a person has died (BV-1). A first draft for clinical and legal review: no clinician, medical examiner or bereaved person has reviewed it. The reasons and sources are on [bereavement](index.md).

**Built:** the rules and the read model in `electronic-patient-record-service-with-rust/src/death.rs`, and the append-only store in `death_store.rs` (table `death_assertion_versions`, view `death_assertion_current`, migration 25). The routes are in `controllers/deaths.rs`. The banner on the patient and summary pages says so, and `DeathSection.svelte` reports and takes steps. An HL7 message with PID-29 or PID-30 stores a report, as below. FHIR is below. **The hold:** the outbox runner holds any event about a patient with a standing assertion, as below. Nothing enqueues and no sink exists, so nothing is held in use today. **The list:** `/api/death-reports` and the `/death-reports` page, as below.

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
| Clinician, on-duty clinician | Yes | Yes | Yes | Yes | Yes |
| Nurse | Yes | Yes | Yes | Yes | No |
| Pharmacist, allied health, receptionist, clinical administrator | Yes | No | Yes | No | No |
| Integration | No | No | Yes, as a feed | No | No |
| Patient, proxy, auditor, Clinical Safety Officer | No | No | No | No | No |

The decision names verifying and withdrawing. `certified` and `registered` are read as the same kind of step as verifying. No care relationship is needed, because reception must see the answer before ringing a family and the staff who confirm a death are often not on the care team. The reasons are kept from the wider group because they are free text and may hold clinical detail. A refused consent stops a read of the answer, as it stops any read by staff; this needs review, because a banner that cannot answer is less safe than one that can.

## Who is told, and who is accountable

Decided by the maintainer on 2026-11-06 as OQ-BV4, not yet reviewed by a Clinical Safety Officer: clinicians, nurses and administrators are alerted to a new report, and **the accountable person is the role of on-duty clinician**.

- **The role is `on-duty-clinician`.** A person holds it for a shift, alongside `clinician`. On its own it gives the death work and nothing else in the record, so that the person accountable can always verify or withdraw.
- **The alert is a list and a count.** Every report still at `reported`, oldest first, up to 50, and a list that is cut says so. The count shows in the navigation on the home page, the patient list and a patient's record. It stays until a person verifies or withdraws the report, so it cannot be dismissed.
- **It is a pointer.** Ids and the time of the report, with no reason, source, reporter or date, because an administrator reads it and may not read the reasons. A report on a record merged away points to the surviving record.
- **A refused consent does not hide a report here,** since the list names no clinical fact and these are the people who must act. Opening the record is still subject to consent, which needs review.

**Not built, and it matters:** the service has no rota. It cannot say who holds the role now, or whether anyone does, so a shift with no holder has no accountable person and nothing says so. Nothing records that the on-duty clinician took a report on, and no person's name is on the list. There is no push: someone has to open a page to see the count. No time limit for a report to wait has been set. Out of hours, the list depends on someone holding the role.

## Routes

| Route | Does |
|---|---|
| `GET /api/patients/{id}/death` | The one answer, and `holds_contact` |
| `GET /api/patients/{id}/death/assertions` | Every assertion, newest first, withdrawn ones included |
| `POST /api/patients/{id}/death/assertions` | Report: `{source, occurred, reason}`. The reporter is the token |
| `PUT /api/patients/{id}/death/assertions/{assertion_id}` | One step: `{expected_version, to, reason}` |
| `GET /api/death-reports` | Every report that waits for a person: `{items: [{assertion_id, patient_id, version, reported_at}], limit, more, accountable_role, site_time_zone}` |
| `GET /api/death-reports/count` | `{open}` |

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

A feed cannot retract a report: a later message with an indicator of `N` changes nothing. A report waits for a person to verify it, on the [list](#who-is-told-and-who-is-accountable) and in the banner.

## The hold

`outbox.rs` asks for the one answer before anything else about an event. While it holds, the event is not delivered, writes no attempt and stays pending; a withdrawal that leaves nothing standing releases it, and it goes at the next run. A hold is not a refusal, because a refusal is final. A failed look is a failed attempt, never a delivery. Held events do not use up a run. `tests/outbound_census.rs` fails if a new way out appears without a hold.

Every event about the patient is held, whatever it is. That needs review: a message that must reach someone after a death, such as telling the patient's GP, is held too, and no route yet says which may go. The contact a person makes by hand is held only by the banner's words. Nothing shows a person what is held, or how long it has waited.

## A proxy and the patient's own login

Decided by the maintainer on 2026-11-05: a proxy and a patient's own login both stop when a person has **confirmed** the death, not at a report. A report is unchecked and can be wrong, and it must not cut a parent, a carer or a living patient off from the record. `relationships::lookup` works this out on each request, so the relationship itself is not ended and a withdrawal restores it with nothing to do by hand. This covers the record, the photo, the access log and "my record", which leaves the patient out.

The cost runs both ways. While a report waits for a person, a proxy can still read the record, and a relative could learn something from it before anyone has spoken to them, or read what the patient kept private. Neither can be undone. Staff see a report on the [list](#who-is-told-and-who-is-accountable), which shortens that wait but does not end it. A person wrongly confirmed as dead, and their proxy, lose the record until a clinician withdraws it, and see only that it has gone, because the list says nothing about a death. Those words need people who have been through it. A person acting after death, with a basis, a scope and an end date, is not built.

## The matcher

Decided as OQ-BV3: the matcher's `Patient` has `death_date` as weak evidence (matcher 0.7.0, weight 0.05, counted only when both records have one). The assertion stays apart from identity.

- **It comes only from a standing assertion.** `death_store::for_matching` fills it from the one answer, and nothing else does. It is never stored on the identity.
- **A date in dispute is left out**, so that it cannot move a score. Nor is a date used once a report is withdrawn. An unconfirmed report's date does count: this is a judgement for review.
- **A client cannot send it.** A patient write with `death_date` is a 422 with the code `death_date_not_accepted`, and `POST /api/patients/compare` refuses it too.
- **Only a merge uses it**, because only a merge compares two stored records. A new registration has no date, so the field does not count there.

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

## Scenarios

`electronic-patient-record-service-with-rust/tests/death_scenarios.rs` runs seven invented deaths through the rules (BV-8): expected, sudden, coroner, wrong patient, withdrawn, baby and twin. They are test data in that file, because the project has no seeded synthetic patients yet. No clinician or bereaved person has read them.

They show one thing to review. A coroner's `2026-09` beside a hospital's `2026-09-14` sets `dates_differ`, though the two agree as far as each goes. It is the cautious reading, since a person looks, but it may be noise.

## Not settled

- **How soon a held contact is reviewed, and who owns that.** OQ-BV1 says a person confirms; it does not say by when.
- **Other fields on [the proposal](index.md#what-this-project-would-need).** `found_at`, `place`, `verified_at` and `coroner_referred` are not in this slice. They can be added to the stored body without a migration, once someone has said which are required.
