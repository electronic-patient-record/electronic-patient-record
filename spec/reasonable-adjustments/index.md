# Reasonable adjustments and communication needs

What a person needs in order to use care: an interpreter, large print, step-free access, a quiet place to wait (PA-6). A first draft: no clinician, disabled person, interpreter or lawyer has reviewed it. Read from the sister project's [patient administration system](../../plan.md) (its `adjustments`, `interpreter` and `needs_review` modules, read 2026-11-05 at commit `15e6389`), and reworked to fit this record.

**Built:** the rules in `electronic-patient-record-service-with-rust/src/adjustments.rs`, the append-only store in `adjustments_store.rs` (table `adjustment_versions`, view `adjustment_current`, migration 26) and the routes in `controllers/adjustments.rs`. The patient and summary banners show what stands and whether a check is overdue, and `AdjustmentsSection.svelte` on the patient page records, checks and ends a need. A failed look says so, and an empty list says "none recorded", which does not mean there are none.

**Not built:** the patient's own view, a proxy scope, a FHIR `Flag`, a list across patients of reviews that are overdue, booking an interpreter, and anything beside an appointment, because the record holds no appointments.

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
| Clinician, nurse, pharmacist, allied health, receptionist, clinical administrator | Yes | Yes |
| Patient, proxy, auditor, integration, Clinical Safety Officer | No | No |

No care relationship is needed, because reception must know before the person arrives. A refused consent stops a read, as it stops any read by staff; this needs review, because not knowing a person needs an interpreter is its own harm. This table is a first draft.

## Routes

| Route | Does |
|---|---|
| `GET /api/patients/{id}/adjustments` | Every adjustment, newest first, ended ones included, each with `overdue` |
| `POST /api/patients/{id}/adjustments` | Record one: `{code, note?, review_by?, reason}` |
| `PUT /api/patients/{id}/adjustments/{adjustment_id}` | `{expected_version, action, review_by?, reason}`, where `action` is `review` or `end` |

A stale `expected_version` is a 409 and a refused change a 422. Every read and write is audited; the audit row holds ids, versions and statuses, never a code, a note or a reason, because a code can say something about a person's health.

## Not settled

- **Whether a patient records their own needs.** The standard's first step is to ask them. Today only staff write here, on what the person said.
- **Who a need is shared with.** Sending one to another organisation needs the person's agreement, and no sender exists.
- **Interpreters.** The sister project raises a request when an appointment is booked. This record has no appointments, so it holds the need and leaves the request to whatever books.
- **A note is free text**, so it could hold clinical detail. Staff who may read it include reception.
