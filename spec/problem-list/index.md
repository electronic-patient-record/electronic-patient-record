# Problem list (EH-1)

**Built 2026-11-24, service only.** No screen, no FHIR, no clinician review. NOT FOR CLINICAL USE. The idea's source is [openEHR and `openehr-rust`](../lessons/openehr-rust/index.md); the rules below are the author's and no clinician or Clinical Safety Officer has read them. Hazard H-066. Part of RC-5.

## Rules

1. **A problem is a condition, diagnosis or complaint on the patient's running list.** It belongs to the patient, not to a visit: it has no encounter, and a field it does not know is refused.
2. **The title is in words** (up to 200 characters). A code may sit beside it; a SNOMED CT code has its shape and check digit checked and is not looked up.
3. **Certainty is required and never assumed**: `unconfirmed`, `provisional`, `differential`, `confirmed`, `refuted`. It is apart from whether the problem is current, so a suspected diagnosis is never shown as a confirmed one for want of a field. A refuted problem is kept and cannot be active or in remission.
4. **Status**: `active`, `remission`, `inactive`, `resolved`, `entered-in-error`. Entered in error is final, needs its own permission, and cannot be the first version.
5. **Dates are as precise as they are known** (`YYYY`, `YYYY-MM`, `YYYY-MM-DD`). One in the future is refused. An end before the start is refused only when that is certain: 2020 against 2020-05 is accepted. An active problem has no end.
6. **A problem may name its cause** (`due_to`), another standing problem of the same patient. Never itself, directly or through others. A problem that others name cannot be marked entered in error until they no longer do. One writer at a time holds the list while these are checked. This is for one condition with effects in several organs (CF-4), and it says nothing about genetics.
7. **Every change is a new version with a reason.** A whole record is sent each time, with the version it was made from.
8. **An empty list is `not-recorded`, never none.** There is no "no known problems" statement (OQ-EH2). The summary keeps confirmed and unconfirmed active problems apart.
9. **There is no note field.** A note has its own author, time and signature (CN-1).
10. **A patient and a proxy read none yet** (OQ-EH1), as for a suspicion: a list can hold a diagnosis the patient has not been told.
11. **The audit row** holds ids, versions, statuses and counts, never a title, a code or a reason.

## Routes

`GET` and `POST /api/patients/{id}/problems`, `PUT …/{problem_id}` `{record, expected_version, reason}`, `GET …/{problem_id}/versions`, and `GET …/as-at?time=` ([as at a time](../as-at/index.md)). Policy resource `Condition`: a clinician reads, records, changes and retracts; a nurse or allied professional reads, records and changes; a pharmacist reads.

## How it differs from its neighbours

- A **suspicion** (RS-1) is what a referrer suspects on one referral, with a pathway and an outcome. It is not put on this list, and an outcome of "confirmed" does not add a problem: a clinician does (OQ-EH3).
- An **observation** is a measurement; an **allergy** has its own list and banner.

## Not done

- The screen, the patient summary section and the ten locales (EH-15); FHIR `Condition` (EH-15); a lookup of codes; ordering by importance; duplicates of one condition are not found; amendment requests (RC-4) cannot name a problem.
- **After an unmerge a cause may be on the other record**: the id is then not on this list. Nothing reports it.
- A title is free text and can hold anything, including another person's name.
