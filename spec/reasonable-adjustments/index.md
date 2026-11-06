# Reasonable adjustments and communication needs

What a person needs in order to use care: an interpreter, large print, step-free access, a quiet place to wait (PA-6). A first draft: no clinician, disabled person, interpreter or lawyer has reviewed it. Read from the sister project's [patient administration system](../../plan.md) (its `adjustments`, `interpreter` and `needs_review` modules, read 2026-11-05 at commit `15e6389`), and reworked to fit this record.

**Built:** the rules in `electronic-patient-record-service-with-rust/src/adjustments.rs`, the append-only store in `adjustments_store.rs` (table `adjustment_versions`, view `adjustment_current`, migration 26) and the routes in `controllers/adjustments.rs`. The patient and summary banners show what stands and whether a check is overdue, and `AdjustmentsSection.svelte` on the patient page records, checks and ends a need. A failed look says so, and an empty list says "none recorded", which does not mean there are none.

**Not built:** booking an interpreter, and anything beside an appointment, because the record holds no appointments.

## Why this is in the record

A person who cannot follow what is said cannot consent to it, and a clinician who does not know that may not notice. England's Accessible Information Standard (DAPB1605) sets the steps as identify, record, flag, share and meet, and its 2025 version adds review. The rule here does not depend on one country's standard.

## The model

An adjustment is a code from a fixed catalogue, with an optional short note and a date by which to check it with the person again.

| Field | Holds |
|---|---|
| `code` | One of 23 codes in `CATALOGUE`, under interpreting, communication, physical access, environment, understanding and appointments. It never changes: a different need is a new adjustment |
| `note` | At most 200 characters. A spoken interpreter needs the language here |
| `status` | `active` or `ended`. Ended is final |
| `review_by` | A real day, today or later, or none |
| `reason` | Why this version was written |

The codes are this project's own. **No national code list is used**, because the licence of the likely ones has not been checked, and a mapping is not guessed. `pictorial-pain-scale` is a preference only: it holds no score.

## Rules

1. **It informs staff and changes nothing by itself.** Nothing reads an adjustment to decide what a person is offered, and a need is never a reason to offer less.
2. **Every version has a reason.** Who is acting is the token, never the body.
3. **Nothing is deleted.** Ending one is a new version, and the time that version was written is the end date.
4. **The same need twice is refused while the first stands**, unless the note differs, so that a person can need two languages. Two writers at the same moment can both succeed; a person can end the duplicate.
5. **Overdue means active, with a date that has passed.** The day itself is in time. "Overdue" is worked out on each read from UTC, so near midnight it can differ from the site's day. An adjustment with no date is never overdue, which says only that no date was set.

## Who may do what

| Role | Read | Record, review, end |
|---|---|---|
| Clinician, nurse, pharmacist, allied health, receptionist, clinical administrator, on-duty clinician | Yes | Yes |
| Patient, for their own record | Yes | Yes |
| Proxy, with the `adjustments` scope | Yes | No |
| Auditor, integration, Clinical Safety Officer | No | No |

Decided by the maintainer on 2026-11-05: patients see and correct their own needs, on "my record" (PX-1). They may add, check and end one, and each change is written as a new version under their own name, so staff can see who changed it. Nothing waits for a member of staff to approve it, because the person is the authority on what they need: that reading is for review. A patient or proxy acts only through their own relationship, so neither can reach anyone else's needs. A proxy reads only, because a need recorded on someone's behalf is a different thing from their saying so.

No care relationship is needed for staff, because reception must know before the person arrives. A refused consent stops a read, as it stops any read by staff; this needs review, because not knowing a person needs an interpreter is its own harm. This table is a first draft.

## Overdue checks across patients

Decided by the maintainer on 2026-11-06, not yet reviewed by a Clinical Safety Officer: **the role of on-duty clinician owns the list of checks that are overdue** (PX-2), and nobody else reads it. The role reads, reviews and ends a need, so that it can do the work it owns. The rota belongs to workforce planning management, so the service cannot say who holds the role, and a shift with no holder leaves the list unowned and nothing says so ([who is accountable](../bereavement/death-assertion.md#who-is-told-and-who-is-accountable)).

- **What is on it.** Every active need with a date that has passed, by rule 5, oldest date first, up to 50, and a list that is cut says so.
- **It is a pointer.** Ids, the version and the date, with no code, note or reason, because a code can say something about a person's health. A need on a record merged away points to the surviving record.
- **A report of a death.** An item is marked when a report of the person's death stands, confirmed or not, so that nobody rings a family to check a need. It stays on the list, and a person ends it.
- **A refused consent does not hide an item here,** since it names no need. Opening the record is still subject to consent, which needs review.
- **Nothing is sent.** Someone with the role has to open the page: the navigation shows no count, because a count on every page would make one audited refusal per load for everyone else. Nothing records who took a check on, and a check overdue for a year looks like one overdue for a day apart from its date.

## Routes

| Route | Does |
|---|---|
| `GET /api/patients/{id}/adjustments` | Every adjustment, newest first, ended ones included, each with `overdue` |
| `POST /api/patients/{id}/adjustments` | Record one: `{code, note?, review_by?, reason}` |
| `GET /api/overdue-reviews` | `{items: [{adjustment_id, patient_id, version, review_by, contact_held}], limit, more, accountable_role}`, oldest date first |
| `GET /api/overdue-reviews/count` | `{open}` |
| `PUT /api/patients/{id}/adjustments/{adjustment_id}` | `{expected_version, action, review_by?, reason}`, where `action` is `review` or `end` |

A stale `expected_version` is a 409 and a refused change a 422. Every read and write is audited; the audit row holds ids, versions and statuses, never a code, a note or a reason, because a code can say something about a person's health.

## FHIR

`GET /fhir/R4/Flag?patient=` and `GET /fhir/R5/Flag?patient=` answer with the same decision and audit row as the list. They are read-only.

| Adjustment | `Flag` |
|---|---|
| Active | `status: active`, and no `period`, because a current version's time is its last change, not when the need began |
| Ended | `status: inactive`, with `period.end` |
| Code and category | `urn:epr:reasonable-adjustment` and `urn:epr:reasonable-adjustment-category`, this project's own |
| Note | `code.text` |
| Reason, review date, overdue | Not sent |

The shape follows the [HL7 `Flag` resource](https://hl7.org/fhir/R4/flag.html). It does not claim the NHS England Reasonable Adjustment Flag or any national profile, because no national code is used. An empty result carries a warning that this does not mean the person has none.

## Not settled

- **A need the patient ended that staff believe still applies.** The history shows both, and nothing resolves it.
- **Who a need is shared with.** Sending one to another organisation needs the person's agreement, and no sender exists.
- **Interpreters.** The sister project raises a request when an appointment is booked. This record has no appointments, so it holds the need and leaves the request to whatever books.
- **A note is free text**, so it could hold clinical detail. Staff who may read it include reception.
