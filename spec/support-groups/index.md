# Condition-specific support groups (CF-1)

A draft from the customer's request of 2026-11-24 ("a monthly cancer support community volunteer group") and this repository's own files. A proposal: nothing is built, no clinician, no volunteer group and no patient has reviewed it, one outside page was read (the worked example), no other source was, and it is NOT FOR CLINICAL USE. Statements marked "my reasoning" are the author's.

## Related links

[Survivor grief](../survivor-grief/index.md) (the same question for bereavement support: help from volunteers outside the health service), [Care settings](../care-settings.md) (section 4), [Protected information](../access-control/protected-information.md), [Roles and assignments](../access-control/roles-and-assignments.md), [Patient portal](../patient-portal/index.md), [Open questions](../16-open-questions.md) (OQ-CF4 and OQ-CF5 (OQ-CF1 to OQ-CF3 answered)).

## What was asked, and what is not known

A customer wants the system to support groups tied to one condition, for example a monthly cancer support group run by volunteers. The request does not say whether the group is run by the health service or a charity, whether staff or patients refer people, whether the system should hold membership, or what volunteers may see. Those are questions to the customer (OQ-CF1), not assumptions to build on.

## The customer's answer to OQ-CF1 (2026-11-24)

Groups are run by patients, by charities and by hospital staff; some live on social media networks; some are local, some regional, some global. What follows is my reasoning, not the customer's:

- **One kind of entry cannot assume one kind of operator.** A directory entry carries who runs it (patients, charity, hospital staff, other) and where it meets (in person, online meeting, social media network), and its reach (local, regional, global). A group's reach decides whether a place or an area is shown.
- **Only a hospital-run group could have been in the system's control.** The customer then decided (OQ-CF2) that no group's roster is held, so the offer record stands for every kind, a staff-run group included; a staff-run group keeps its own roster outside this system.
- **A social media group is public or semi-public and not under anyone's control here.** The system may show its name and link, never post, tag or invite on a patient's behalf, and the offer says the group is outside the health service and its members can see each other. Joining may reveal the condition to the person's own contacts (rule 1).
- **A global or online group has no local safeguarding or venue.** The directory says so by type; it is the person's choice, not a referral by the service.
- **The endorsement statement matters more.** The entry says whether the service endorses, does not endorse, or has not reviewed the group, as the Welsh page does, but with a date.

## Design rules (my reasoning)

1. **Belonging to a condition group discloses the condition.** Membership of a cancer support group says the person has, or had, cancer, or cares for someone who does. It is protected information in the same way as a diagnosis, and a list of members is a list of diagnoses.
2. **The default is a referral, not a membership.** The record holds that a clinician offered a group and what the patient said (accepted, declined, later), as a signposting entry with the group's name and contact details. The group's own attendance and notes stay with the group. This is the stance of [survivor grief](../survivor-grief/index.md): much support is outside the health service, with no EPR and no reply. The customer decided (OQ-CF2, 2026-11-24) that the system holds only the offer.
3. **A volunteer is not staff.** A volunteer role, if one exists, grants no clinical read: no patient list, no record, no search. What a volunteer sees is only what the group itself holds. The system holds no roster, so no role is needed to run one (OQ-CF2).
4. **The patient chooses; nothing is automatic.** An offer, a reminder or a contact from a group is sent only if the patient agreed, by the channel they chose, and with no condition named in a message that could be read by someone else (a phone's lock screen, a family member). No automatic referral from a diagnosis code, and no "patients like you" list.
5. **A carer or family member is not recorded as a member.** The customer decided (OQ-CF3, 2026-11-24): no, because the groups are third-party, and the system holds no membership at all (OQ-CF2). A group may welcome relatives; the system does not record that. If the patient says a relative will go, that stays in the patient's own words in a note, in the "reported by the person" form, and creates no person record.
6. **A group is a directory entry, not a clinical service.** It carries a name, a type (see the worked example below), a condition it is for, a schedule (for example monthly), a place or link, who runs it, who vouched for it and when, and a contact. A directory entry is reviewed by a named person on a date, and an unreviewed entry is shown as such (never "approved by this project").
7. **Support is not safety.** A group is not clinical cover. A person in crisis needs a clinical route, and the offer of a group never replaces it (survivor grief, hazard SG-1).

## What it could look like (not built)

| Piece | Holds | Does not hold |
|---|---|---|
| Group directory entry | Name, condition label, schedule, venue or link, organiser, review date and reviewer | Member names |
| Offer record on a patient | Group, date offered, by whom, the patient's answer, and the channel the patient allowed | Attendance, what was said |
| Patient portal view | The patient's own offers and a way to answer or withdraw | Other patients' data |
| Reports | Counts of offers, takes-up and withdrawals, suppressed under a small-numbers rule (PA-7) | A person-level list of members |

## A worked example from a real service: Wales' adult inherited metabolic disease service

Read on 2026-11-24, in full, by an assistant fetching the page: [Support Groups and Organisations, All Wales Adult Inherited Metabolic Disease Service, Cardiff and Vale University Health Board](https://cavuhb.nhs.wales/our-services/all-wales-adult-inherited-metabolic-disease-service/information-for-patients/support-groups-and-organisations/) [CAVSG]. The Welsh-language version and the roughly 60 external sites it links to were not read. It is one page from one service, so it shows one practice, not the norm.

- **It is a directory of signposts, not a membership.** About 60 entries; the service holds no list of who joins any of them. This supports design rule 2.
- **The service disclaims responsibility.** It says it does not endorse any listed group, that the listings are "only provided as examples of where to look for information", and that the content of the external sites "rests entirely with the organisation running the website". It gives no statement on referral, confidentiality or data sharing, no named contact for the page, and no date of publication or review. Design rule 6 asks for more than this: a review date and a reviewer on each entry.
- **The entries are of several kinds**, which the directory should not flatten: condition-specific charities and peer groups (for example one per metabolic condition); umbrella and rare-disease bodies; clinical, research and registry bodies; NHS and government bodies (wellbeing, benefits, student finance); and general carer and benefits charities. A "support group" in the customer's sense is only the first kind, so an entry needs a type.
- **Contact details are rare.** Only one listing gives a phone number and an email; the rest are links. Opening hours and the Welsh, English and British Sign Language options apply to the health board, not to the groups.
- **One entry is a service the board is trialling**, described as "a limited deployment" for people moving from children's to adult services. A group or service can therefore be in a pilot, and the directory should say so.
- **The condition is an umbrella**: the page covers many conditions at once, and condition-specific groups sit beside umbrella bodies. This is relevant to [CF-4](../../tasks.md) (a multi-system genetic condition).

What this changes in the draft: a directory entry gets a **type** (peer group, charity, umbrella, registry or research, public body), an **endorsement statement** (endorsed, not endorsed, or not reviewed) and a **last-checked date**; and the system never claims it checked an outside group's content. It does not change the rule that membership is not held by default (decided by OQ-CF2).

Source [CAVSG]: the page above, read 2026-11-24.

## Not decided: needs the customer

| Id | Question |
|---|---|
| OQ-CF1 | **Answered 2026-11-24 by the customer:** various. Run by patients, charities or hospital staff; some run on social media networks; some local, some regional, some global. **Still open:** who refers into a group. |
| OQ-CF2 | **Answered 2026-11-24 by the customer:** the system holds only the offer. No membership, no attendance, no roster, for any kind of group. |
| OQ-CF3 | **Answered 2026-11-24 by the customer:** no. A carer or relative is not recorded as a member, because the groups are third-party. |
| OQ-CF4 | Which channels may a group use to contact a person, and what may a message say? |
| OQ-CF5 | Which condition labels are used, and does the label ever reach the patient's own device or a printed letter? |

## Tasks and acceptance

`CF-1` is this page. A later build needs, before any code: answers to OQ-CF4 and OQ-CF5 (OQ-CF1 to OQ-CF3 answered); a hazard-log row for disclosure of a diagnosis by a group list or a message (the Clinical Safety Officer sets its status); a test that a volunteer role reads no clinical data; a test that a message names no condition by default; and a review of the directory by a person who runs such a group.

## Not done, not verified

One outside page was read, only its text: none of the groups it lists were opened. No law was checked (the controller and consent questions differ by country, so they belong in a country pack, [equity rule](../02-scope.md)). No customer, clinician, volunteer or patient was asked. Nothing is built.
