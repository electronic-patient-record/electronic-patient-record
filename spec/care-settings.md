# Care settings and specialty views

**A direction from the author's instruction of 2026-11-04 (plan.md §27). Nothing here is built.** Two ideas: (1) acute, community and mental-health care are different **settings** that one record must serve; (2) a **specialty view** (renal is the first example) shows a specialty's work without a separate record.

## 1. The gap: systems built for one setting

Many EPR systems come from one setting (acute hospital, community, or mental health), and when an acute provider phases in community or mental-health services onto an acute-shaped system, it works badly. This is the author's observation; the sources below are examples from the trade press and suppliers, not a study.
What the examples show is that community and mental-health care differ in ways an acute design tends to assume away:

| Aspect | Acute (typical) | Community and mental health (typical) |
|---|---|---|
| Unit of work | A bed, a ward, an admission and a discharge | A caseload, a visit round, a care coordinator |
| Where care happens | In the building, at a workstation | At the patient's home or in a clinic; staff on mobile devices, sometimes offline |
| How long | Days, ending at discharge | Months or years; discharge from one team is not the end of care |
| Legal frame | General consent and capacity | Mental-health law (detention status, authority, dates, restrictions), capacity and consent to treatment, risk and safeguarding information, statutory returns |
| Plan of care | Orders, results, ward tasks | Care plans, risk assessments, outcome measures, multidisciplinary teams |

Sources: examples of trusts choosing a mental-health or community system partly for mobile access to records, for Mental Health Act compliance and for statutory reporting (Digital Health, supplier case studies: see [references](17-references.md)). No source found compares acute with community and mental-health requirements directly.

## 2. Design rules

1. **A setting is configuration, not a fork.** One record and one data model; a **setting profile** (acute, community, mental health, or a mixture, for a provider that runs all three) chooses the work-unit view (bed board or caseload), the default screens, the forms and the legal-status records. A provider that adds community care to an acute deployment adds a profile, not a second system.
2. **The core assumes no bed, no admission and no building.** An `Encounter` can be at a patient's home or by video; its class (FHIR `Encounter.class`, from the v3 ActCode set, where values such as home health and field exist: to confirm) says so; a discharge from a ward closes a ward stay and **never closes the care pathway** ([5.2.1](05-domain-model.md)). Long-running community and mental-health care is exactly the case the pathway-first model is for.
3. **The legal status of a patient is a record, not a flag.** For mental health: the status, the authority, the dates, the restrictions on the patient and on who may see what, who is told. The statutes differ by country, so this is a **pack** under the [equity rule](02-scope.md); the core ships none. What the record needs to hold is task `XP-7`.
4. **Sensitive notes may be restricted** within a team and across settings (label masking, FR-4.3). Mental-health information moving into an acute view is a disclosure decision, never a default.
5. **Mobile and offline are first-class** for community and mental-health staff ([mobile viewability](mobile-viewability.md)).
6. **Phase-in is by setting, with the same patient.** A person known to acute and community care has one patient record and one set of pathways; the profile changes what a clinician sees, not what exists.

## 3. Specialty views

A **specialty view** is how one specialty (renal, cardiology, diabetes) sees and works the same record. It is configuration, delivered as a **pack** (the pack mechanism of OS-10 and IO-18), not code in the core and not a separate record.
A specialty pack can hold: the value sets of codes the specialty uses; a layout of widgets (trend charts, tables) for its summary page; forms (FHIR `Questionnaire`) and any registry data-set mapping; alert thresholds, each with a review status; and pathway templates ([pathway design](pathway-design.md)).
Rules: a specialty view **adds, it never hides** the banner, allergies and alerts; permissions are those of the role, not the view; a clinician may switch views; the default view comes from the clinic or role; a view is clinician-reviewed before use and says who reviewed it (never claimed by the project).

### Worked example: renal (draft, to be written with a renal clinician)

From the sources read (UK Renal Registry material, a primary-care study), not from a renal clinician:

- **Kidney function trend.** eGFR and creatinine over time, with the **dates shown**, because kidney-function tests are not done on everybody and are not evenly spaced; a trend line that assumes regular intervals would mislead.
- **Dialysis session records**, where one session may carry several values of a measure (for example the blood flow rate), so a session is a record with a list, not one number.
- **A registry feed.** The UK Renal Registry receives electronic feeds from kidney centres; registration is triggered at the start of treatment or on an eGFR under 30 or an acute-kidney-injury flag from the laboratory. A renal pack would map to such a feed; which national registry applies is a country matter.
- **Not sourced and left for a renal clinician:** vascular access, transplant, modality changes, the set of alerts, and what the summary page should lead with.

## 4. Customer requests, not designed (2026-11-24)

A customer asked for four things this document does not yet answer: condition-specific social groups (a monthly cancer support volunteer group); prison health; military health; and complex genetic conditions that affect several body systems, such as brain-lung-thyroid syndrome. They are tasks `CF-1` to `CF-4` in [tasks](../tasks.md). Prison and military health fit design rule 1 (a setting is a profile, with the legal frame as a pack); a multi-system genetic condition fits the specialty views of section 3 but needs a shared care plan across them; a support group raises a disclosure question, because belonging to it can reveal a diagnosis. Drafts exist for all four ([support groups](support-groups/index.md), [custodial health](custodial-health/index.md), [military health](military-health/index.md), [multi-system conditions](multi-system-conditions/index.md)). None is built or reviewed, except two building blocks for the last (a problem may name its cause, and [relative history](relative-history/index.md)), built 2026-11-24 in the service only.

## 5. Where this project assumes a hospital (XP-5, 2026-11-25)

Read: the code for the words ward, bed, admission, admit, discharge, inpatient, bedside, hospital and building; then the enums, states and settings that assume a place, a team or a connection without using those words; and the numbered spec documents 01 to 10 and the pages below. **No code was changed.**

**The words themselves are almost absent from the code**: comments, test data and one value in a closed list of death sources. No table, route or screen has a bed, a ward, an admission or a discharge. No clinical rule depends on an encounter or an episode (5.2.1 rule 4 holds, because neither is stored). What assumes a hospital is quieter:

| Where | What it assumes | Who it does not fit | What would help |
|---|---|---|---|
| **Referral state `no-show`**, shown as "Missed the appointment", recorded **with no note or reason** (`referrals_store.rs` `needs_note`) | That the patient could have come and did not | A person in custody who was not escorted, a child or a care-home resident who was not brought, a serving person who was deployed, a person with no transport. The record then says something about them that is not so | `XP-15`: a reason, chosen, with "not brought" apart from "did not come" |
| Care relationship `active-encounter` (`relationships.rs`) | That there is an encounter. There is none: the word is typed by an administrator, nothing starts or ends it | Community teams, whose relationship is a caseload over months | `CA-1`, `CA-2`; a caseload as a relationship of its own (`XP-6`) |
| Roles (`policy.rs`): clinician, nurse, pharmacist, allied health | Four kinds of professional and their hospital work | A social worker, care coordinator, peer worker, community support worker, approved mental-health professional, prison health worker: each must be given a role with more or less than they need | [Roles and assignments](access-control/roles-and-assignments.md); not decided |
| Break-the-glass: 4 hours, fixed (`break_glass.rs`) | An emergency that ends within a shift, with someone else to review it | A lone clinician on a ship or a remote clinic, where the reviewer may be the same person (who is refused) or days away | A note in `military-health` point 5; not decided |
| Death sources (`death.rs`): hospital, general practice, care home, coroner, registrar | Where people are known to die | A prison, a hospice, at home, on deployment, in the street: each becomes `other-system`, so they cannot be counted | A value set in a pack (`BV` tasks) |
| One site time zone and one list of holidays for a deployment (`site.rs`) | One place, one calendar | Several sites, a ship, a deployed clinic. An age or a due day may be a day out for its reader | `XP-6`: a zone for each site |
| The address is a postcode (matcher `Address`) | A settled home with one | No fixed abode, custody, barracks, a refuge. Postcode is evidence in matching, so a shared one (a prison, a hostel) makes strangers look more alike | Matcher and its spec: a place known to be shared should not count. **Measured 2026-11-25:** a shared postcode alone turns some pairs of two people from "ask a person" to "link" ([candidate recall](patient-identity/candidate-recall.md)) |
| The UI needs the network and signs out after a short idle time | A workstation in a building | A visit round with poor signal, gloves on, a shared tablet | `XP-4`, [mobile viewability](mobile-viewability.md) |
| HL7 ADT feed as the way patients arrive (`adt_feed.rs`) | A hospital patient administration system | A service that has none | The API and FHIR; no change needed |
| Task teams: the only team a built-in rule names is `clinician` (`task_rules.toml`) | One team that is always there | A caseload with a named coordinator | `CA-2`, `XP-6` |

In the **specifications**: [scope](02-scope.md) puts outpatient care first and then the inpatient core, and has a bed board and no caseload; community and mental health are on this page only. [Order entry](order-entry/index.md) takes inpatient laboratory orders first. [Medication administration](medication-administration/index.md) and [nursing flowsheets](nursing-flowsheets/index.md) are written for a ward round. [Clinical workflows](clinical-workflows/index.md) is the most acute page. [5.2.1](05-domain-model.md) is free of it, and says a pathway outlives a discharge.

## Not done, not verified

No clinician reviewed any of this. The acute versus community comparison is the author's, supported only by examples. The ActCode values are to be checked. No renal system was examined. Section 5 is one reading by words and by the main lists, not a proof: the pages it does not name, the mobile apps and the translated words were not read for it. Section 4 is a list of requests; the drafts it links are proposals. Tasks: `XP-5` to `XP-7`, `XP-10`, `XP-11`, `CF-1` to `CF-4`.
