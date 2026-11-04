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

## Not done, not verified

No clinician reviewed any of this. The acute versus community comparison is the author's, supported only by examples. The ActCode values are to be checked. No renal system was examined. Tasks: `XP-5` to `XP-7`, `XP-10`, `XP-11`.
