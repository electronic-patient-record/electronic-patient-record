# Family history (EH-3)

**Built 2026-11-24, service only.** No screen, no FHIR, no clinician or clinical geneticist review. NOT FOR CLINICAL USE. The idea's source is [openEHR and `openehr-rust`](../lessons/openehr-rust/index.md): an entry about a family member that does not say so reads as a finding about the patient. Hazard H-067. For CF-4.

## Rules

1. **An entry is a finding about a relative, in the patient's record.** It is its own kind of record and never appears on the problem list.
2. **The relative is required, and is by blood**, since an inherited risk is what an entry is for: adoptive and step relatives are not offered. `unknown-relative` is for a blood relative who is not known.
3. **A relative is never named.** There is no place for a name, a birth date, an identifier or a note, and a field the record does not know is refused. A number tells two sisters apart. The relative has not agreed to be in this record (hazard H-065; the customer's answer to OQ-CF3).
4. **Present, absent and unknown are three findings.** An empty list is `not-recorded` and says none of them.
5. **An age at onset** is in whole years and only for a condition the relative had.
6. **The source is required**: the patient, someone acting for them, a genetics service's report about this patient, or another professional's letter about this patient. **A relative's own health record is not a source**: using it discloses something about the relative, and this does not decide that (OQ-EH4).
7. **Every change is a new version with a reason**; entered in error is final and is a clinician's.
8. **A patient and a proxy read none yet** (OQ-EH1). A pharmacist reads none.
9. **Nothing is worked out from it**: no score, no pedigree, no recall.

## It is not a family view

[Survivor grief](../survivor-grief/index.md) refuses a family or household view (SG-2): seeing one person gives no right to see another. An entry does not break that: it sits under one patient, is decided by that patient's access, holds no link to another record, and nothing lists entries across patients. `tests/grief_census.rs` refuses any route with the word *family*, and now allows these four paths, spelled in full, with the reason; `a_family_history_entry_reaches_no_other_person` holds what makes that safe and fails when a field is added. **The exception is the author's decision and the maintainer has not confirmed it** (OQ-EH5).

## Routes

`GET` and `POST /api/patients/{id}/family-history`, `PUT …/{entry_id}`, `GET …/{entry_id}/versions`, `GET …/as-at?time=`. Policy resource `FamilyMemberHistory`.

## Not done

The screen and FHIR `FamilyMemberHistory` (EH-15); genetic test results (RC-12); a relative's consent; consanguinity, pregnancy losses and pedigrees. The condition is free text of up to 120 characters and **can hold a name**: nothing can refuse that.
