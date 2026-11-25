# Military health: a service care setting (CF-3)

A draft from the customer's request of 2026-11-24 ("add capabilities for … military health"), a few public sources (below) and this repository's own files. A proposal: nothing is built, no military clinician, commander, lawyer, serving person or veteran has reviewed it, and it is NOT FOR CLINICAL USE. "My reasoning" marks the author's. The customer's answers on [custodial health](../custodial-health/index.md) were about custody and are **not** taken as answers here.

## Related links

[Custodial health](../custodial-health/index.md) (CF-2, the other profile where a person's keeper or employer is close to the record), [Care settings](../care-settings.md) (a setting is a profile, not a fork; section 4), [Protected information](../access-control/protected-information.md), [Patient identity](../patient-identity/index.md), [Reads as at a time](../as-at/index.md), [Open questions](../16-open-questions.md) (OQ-MS1 to OQ-MS8), [Scope and the equity rule](../02-scope.md).

## What was asked, and what is not known

A customer wants the system to support military health. The request did not say which country, which service, whether the customer is a defence medical service, a civilian provider that treats serving people, or a veterans' service. Those are three different products (OQ-MS1). This page is written so that each could use part of it.

## What the sources say

Examples from two countries, to show what varies. None of it is a rule of the core, and none was checked by a lawyer.

| Source | What it says | Read how |
|---|---|---|
| United States, 45 CFR 164.512(k)(1), [Cornell LII copy](https://www.law.cornell.edu/cfr/text/45/164.512) | Information about armed forces personnel may be used or disclosed for activities that the appropriate military command authorities deem necessary to the mission, **if** the authority has published by notice in the Federal Register who those authorities are and the purposes. On separation or discharge, a defence component may disclose to Veterans Affairs for deciding eligibility for benefits. Foreign military personnel: to their own foreign military authority, for the same purposes | Paragraph (k)(1) only, through an agent's excerpts, 2026-11-25. The page links an amendment published 19 November 2026 (91 FR 74963), **not read** |
| United States, Department of Defense Instruction 6490.08, command notification for mental health | A presumption that a provider does not notify a commander when a person seeks mental health or substance-misuse care, with listed exceptions (for example a serious risk of self-harm), and only what the commander needs in order to act | **Search summaries only.** The military sites could not be reached from here. The text, its list and its date must be read before anything is designed on it |
| United States, [GAO-26-108483](https://www.gao.gov/assets/gao-26-108483.pdf) on the Individual Longitudinal Exposure Record | It links deployment history with **possible** environmental exposures at dates and places, from 16 databases. Information on classified missions is not included, and people deployed before 2001 may have no record. Serving people may add self-reported updates, flagged as such. A correction is made by the owner of the data system that feeds it, and the programme is not staffed to validate corrections | Read 2026-11-25, in part |
| England, [NHS service leavers' guide](https://www.nhs.uk/nhs-services/armed-forces-community/service-leavers-guide/) | A leaver takes a form (FMed133A) to a general practice, which may then ask for the complete record from Defence Medical Services, with the person's permission; a paper summary is not usually given unless asked for. A person can ask for veteran status to be recorded by a code | Through an agent's excerpts, 2026-11-25. Page last reviewed 5 June 2024 |

What these share, by my reasoning: the **receiving** side starts the transfer, so a person who does not know to ask arrives with no record; and an exposure record is a statement about a place and a date, which is not proof about a person.

## What military service changes (my reasoning, to be checked with the customer)

1. **The employer is close to the record.** A commander decides where a person is sent, what they do and whether they carry on. Health information can end a career, so a person may not seek care. The default is the same as custody's: health information stays with the health team. What a commander may be told, if anything, is a **table in the country pack**, keyed by the duty, the purpose and the urgency. With no pack the core refuses and says why.
2. **A commander has no role that reads clinical data.** Rank is not a permission, and neither is being a clinician's superior officer. A clinician in uniform is in two positions at once (dual loyalty); a disclosure made under an order is recorded as that, with who gave it.
3. **Fitness is a statement, not a reason.** What leaves the health team is a functional statement from a reviewed list, with a date and an expiry and without the diagnosis ("not to deploy until", "no night driving"). It can itself disclose (a code that only one condition earns), so each item is reviewed before it goes on the list.
4. **Seeking care is not an event a commander sees.** An appointment, a place or a clinic name can disclose as much as a note. A mental-health appointment in a duty roster is the plain case.
5. **The record is made where there may be no network.** A deployed clinic may be offline for weeks, and the same person may be treated by an ally. Entries arrive late and out of order, so each carries when it happened, when it was recorded and when it arrived; a read says how recent its sources are ([as at a time](../as-at/index.md), [mobile viewability](../mobile-viewability.md)). A late entry never silently changes what was shown: it is a new version.
6. **Identity may be hidden on purpose.** A person may be treated under a protected or temporary identity, or unconscious with a tag. It must resolve to one person later without showing the real name to a reader who may not see it. The temporary identities the record has today (a tag that survives a read and a write) are a start, not this.
7. **Place can be secret.** A location, a unit or a date can be classified while the injury is not. The clinical fact is kept and the place is withheld **with a marker that it is withheld**, never left blank, so a later clinician knows there is something to ask about. **The core is not a classified system** and must not hold classified text (OQ-MS5).
8. **An exposure is evidence about a place.** Four things stay apart: a documented measurement above a threshold, a location-based possibility, a self-report, and nothing known. **No record is not "not exposed"** (the repository's rule: absent evidence is null with a reason). An exposure is never turned into a diagnosis, nor a missing one into a reason to refuse a claim.
9. **Leaving is a hand-over, not an end.** The person is the same patient before, during and after service. A release review of open items (medicines, referrals, results not seen, open exposures) is done before the date, and says what it could not see. If the next provider is on another system it is an export the pack names, and the hand-over is **sent, not left for the person to carry**, where the law allows that.
10. **Veteran status is the person's to tell.** It is asked, with the answers yes, no, prefers not to say and not asked kept apart, and shown only to those who need it. It is not inferred from an old address, a service number or a deployment. A reservist is both: serving and civilian, with two providers, at the same time.
11. **Dependants are patients in their own right.** A partner or child treated by a military provider has a record a commander has no claim on, and is not linked to the serving person's record for any reader.
12. **Consent under orders is weak.** A vaccination or a test may be required. The record says **required**, by whom and under what, and never records a consent the person did not give, nor a refusal they did not make.
13. **An injury may be evidence**: of an attack, a crime within the service or an accident. Release to an investigator follows the same table as custody's: a named basis, the smallest part, an audit row, and a named person who decided.

## One profile with custody, and where they differ

The acceptance asks for one design where the sources allow. My reasoning; the custody column is that page's, part of it the customer's.

| Point | Shared | Custody | Service |
|---|---|---|---|
| Profile | A setting type, an operator type and a country pack; one patient, one record | Prison, police, immigration, secure youth, hospital | Garrison, deployed, ship, reserve, training, veteran |
| Who is kept out | The organisation that holds or employs the person has no clinical role | Officers, police | Commanders, and colleagues who are also clinicians |
| What may leave | A reviewed functional list, dated, expiring, no reason | Cell, handcuffs, escort | Duty, deployment, equipment |
| Release rules | A table in the pack; with none, refuse; audited; an urgent release is reviewed | Courts, police | Command authority, investigators, an ally |
| Hidden by label | The **fact** of the setting is itself protected | The fact of custody | Not decided: for a veteran the fact is often useful, and for some it is dangerous (OQ-MS6) |
| Hand-over | A review of open items, complete or saying it is not | Release, transfer at short notice | Discharge on a known date, or a return to reserve |
| Not shared | | Escort, in-possession medicines, missed care | Classified places, exposures, protected identity, allies |

**The shared part is worth building once**: the pack's release table and refusal, the functional list with expiry, and the hand-over review. All three wait on the masking that counts (EH-8), because no clinical record can be hidden by label yet.

## What a service profile could hold (not built)

| Piece | Holds | Does not hold |
|---|---|---|
| Service episode | That the person serves or served, the service, the dates, whether it is their own word | Rank, unit or trade unless the pack needs it; a classified place |
| Fitness statement | A functional item, dates, who issued it | The diagnosis, the note |
| Exposure entry | Agent or event, dates, place or "withheld", which of the four kinds of evidence, the source | A diagnosis; a conclusion of "not exposed" |
| Command disclosure | Who asked, under which rule, what was given, who decided | A disclosure with no basis |
| Hand-over | Open items, what could not be seen, to whom it was sent, when it was acknowledged | "Sent" shown as "received" |

## Not decided: needs the customer

| Id | Question |
|---|---|
| OQ-MS1 | Which country, and which of the three: a defence medical service, a civilian provider that treats serving people, or a veterans' service? |
| OQ-MS2 | Is there one record through service and after it (as the customer chose for custody), or a defence record and a civilian record with a hand-over? Who is the controller of each? |
| OQ-MS3 | What may a commander be told, by which rule, and who decides? Is a functional statement with no diagnosis enough, as in custody? |
| OQ-MS4 | Must it work deployed and offline, and for how long? |
| OQ-MS5 | Will any information be classified? My recommendation is that the system **never** holds it, and holds only a marker that something is withheld. |
| OQ-MS6 | Who sees that a person is a veteran, and is it the person's choice? |
| OQ-MS7 | Does an exposure record already exist to read from, or must this hold one? If both, whose word wins? (My reasoning: neither; both are shown, with their sources.) |
| OQ-MS8 | Are reservists, dependants, cadets, contractors and allied personnel in scope? |

## Tasks and acceptance

`CF-3` is this page. A later build needs, before any code: answers to OQ-MS1 to OQ-MS3; the primary text of the rules for the customer's country; hazard-log rows for a disclosure to command, for care not sought because of it, for a missing exposure read as none, for a late deployed entry, and for a hand-over not received; a test that a command role reads no clinical data; and review by a military clinician, a serving person and a veteran.

## Not done, not verified

Only the sources in the table were used: one read in part, two through an agent's excerpts, and one through search summaries only. The amendment of 19 November 2026 was not read. A regulator's page about veterans in general practice refused the request and was not used. No source was sought for any country but the United States and England, for the laws of armed conflict and the protection of medical personnel and records, for NATO or allied exchange standards, or for FHIR profiles of service history. No one was asked. Nothing is built.
