# Condition-specific support groups (CF-1)

A draft from the customer's request of 2026-11-24 ("a monthly cancer support community volunteer group") and this repository's own files. A proposal: nothing is built, no clinician, no volunteer group and no patient has reviewed it, no outside source was read for it, and it is NOT FOR CLINICAL USE. Statements marked "my reasoning" are the author's.

## Related links

[Survivor grief](../survivor-grief/index.md) (the same question for bereavement support: help from volunteers outside the health service), [Care settings](../care-settings.md) (section 4), [Protected information](../access-control/protected-information.md), [Roles and assignments](../access-control/roles-and-assignments.md), [Patient portal](../patient-portal/index.md), [Open questions](../16-open-questions.md) (OQ-CF1 to OQ-CF5).

## What was asked, and what is not known

A customer wants the system to support groups tied to one condition, for example a monthly cancer support group run by volunteers. The request does not say whether the group is run by the health service or a charity, whether staff or patients refer people, whether the system should hold membership, or what volunteers may see. Those are questions to the customer (OQ-CF1), not assumptions to build on.

## Design rules (my reasoning)

1. **Belonging to a condition group discloses the condition.** Membership of a cancer support group says the person has, or had, cancer, or cares for someone who does. It is protected information in the same way as a diagnosis, and a list of members is a list of diagnoses.
2. **The default is a referral, not a membership.** The record holds that a clinician offered a group and what the patient said (accepted, declined, later), as a signposting entry with the group's name and contact details. The group's own attendance and notes stay with the group. This is the stance of [survivor grief](../survivor-grief/index.md): much support is outside the health service, with no EPR and no reply. Holding membership is a further step that needs a controller decision (OQ-CF2).
3. **A volunteer is not staff.** A volunteer role, if one exists, grants no clinical read: no patient list, no record, no search. What a volunteer sees is only what the group itself holds. An actor with such a role may run a group's roster only if OQ-CF2 says the system holds one.
4. **The patient chooses; nothing is automatic.** An offer, a reminder or a contact from a group is sent only if the patient agreed, by the channel they chose, and with no condition named in a message that could be read by someone else (a phone's lock screen, a family member). No automatic referral from a diagnosis code, and no "patients like you" list.
5. **A carer or family member is a person, not a patient.** A group may include relatives. The system holds nothing about a relative beyond what the patient says, in the "reported by the person" form of words; whether a relative may be recorded as a member at all is OQ-CF3.
6. **A group is a directory entry, not a clinical service.** It carries a name, a condition it is for, a schedule (for example monthly), a place or link, who runs it, who vouched for it and when, and a contact. A directory entry is reviewed by a named person on a date, and an unreviewed entry is shown as such (never "approved by this project").
7. **Support is not safety.** A group is not clinical cover. A person in crisis needs a clinical route, and the offer of a group never replaces it (survivor grief, hazard SG-1).

## What it could look like (not built)

| Piece | Holds | Does not hold |
|---|---|---|
| Group directory entry | Name, condition label, schedule, venue or link, organiser, review date and reviewer | Member names |
| Offer record on a patient | Group, date offered, by whom, the patient's answer, and the channel the patient allowed | Attendance, what was said |
| Patient portal view | The patient's own offers and a way to answer or withdraw | Other patients' data |
| Reports | Counts of offers, takes-up and withdrawals, suppressed under a small-numbers rule (PA-7) | A person-level list of members |

## Not decided: needs the customer

| Id | Question |
|---|---|
| OQ-CF1 | Who runs the group (the service, a charity, the patients), and who refers into it? |
| OQ-CF2 | Does the system hold a group's membership or attendance, or only the offer? If it holds them, who is the controller, and is a volunteer allowed to use the system at all? |
| OQ-CF3 | May a carer or relative be recorded as a member, and under whose consent? |
| OQ-CF4 | Which channels may a group use to contact a person, and what may a message say? |
| OQ-CF5 | Which condition labels are used, and does the label ever reach the patient's own device or a printed letter? |

## Tasks and acceptance

`CF-1` is this page. A later build needs, before any code: answers to OQ-CF1 to OQ-CF5; a hazard-log row for disclosure of a diagnosis by a group list or a message (the Clinical Safety Officer sets its status); a test that a volunteer role reads no clinical data; a test that a message names no condition by default; and a review of the directory by a person who runs such a group.

## Not done, not verified

No source outside this repository was read. No law was checked (the controller and consent questions differ by country, so they belong in a country pack, [equity rule](../02-scope.md)). No customer, clinician, volunteer or patient was asked. Nothing is built.
