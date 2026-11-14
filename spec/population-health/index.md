# Population health: care management, risk, outreach and quality measures

Public-source research read 2026-11-14 plus this repository's own files. A proposal and a reading, not clinical advice: nothing here is built, and no clinician has reviewed it. No registry, risk score, outreach campaign or quality measure exists in the code today.

Method: web searches and a few fetched pages (most claims rest on search snippets, not full documents; each source in the list says which, and "Not verified" collects the gaps) and the project's specs: [pathway design](../pathway-design.md), [domain model 5.2.1](../05-domain-model.md) (the pathway comes first), [equity audit](../equity-audit.md), [privacy-preserving analysis](../privacy-analytics.md), [implementation status](../14-implementation-status.md), [closed-loop referral](../interoperability/closed-loop-referral.md), [reasonable adjustments](../reasonable-adjustments/index.md), [suicide watch](../suicide-watch/index.md) (the census of risk fields), [survivor grief](../survivor-grief/index.md) (what not to build), and the page shape of [virtual ward](../virtual-ward/index.md). Tasks read: CA-2, CA-6, OR-5, IO-4, IO-6, IO-7, IO-14, XP-14, EM-3, WL-3, WL-6, PA-7, PA-13, SW-1, SM-2. Sources are keyed [S1] and so on and listed at the end.

## What it is

Population health is the work of looking after a defined group of people, not only the patient in front of you: who is in the group, who in it needs more help, who is overdue for something, who has been contacted, and how well the group is doing on agreed measures. It has four parts here.

1. **Care management.** A care manager (often a nurse or social worker) holds a caseload of patients, each with a care plan, goals, tasks and contact history, and follows them across transitions (hospital to home, home to care home).
2. **Risk stratification.** A registry is a list of patients who meet a definition (for example "adults with diabetes"). A risk score orders a registry by predicted need (for example readmission within 30 days, or frailty).
3. **Outreach.** A gap-in-care list names patients overdue for a service (a screening test, a review). Staff or the system contact them by letter, phone, text or portal and record what happened, honouring opt-outs.
4. **Quality measure reporting.** A measure counts, for a defined population, how many received the right care. In the US these feed value-based contracts (MIPS, the Medicare Shared Savings Program, the successor to ACO REACH, health plan HEDIS). In England a primary care contract pays practices on indicators of the same shape (QOF).

In this record's terms all four are **views over the pathway**: a registry is a set of patients sharing a pathway or a condition, a gap is a pathway step that is due and not done, an outreach attempt is a step of its own, and a measure counts steps done against steps owed. Nothing here creates a second, parallel record of care.

## Part 1. Care management

### How others do it

- **Epic Healthy Planet** is described as Epic's population health module with registries, risk scoring, care plans, goals, tasks and outreach built into the EHR rather than beside it [S20] [S21]. Most descriptions are by customers and third parties; Epic's own documentation is not public.
- **Arcadia** describes a patient registry builder, configurable risk stratification and "actionable work lists to care managers", over combined clinical, claims and pharmacy data [S22]. Vendor marketing.
- **Innovaccer** describes care-gap outreach and post-discharge check-ins by SMS, voice (including automated "voice AI agents") and email, and self-reports a 28% cut in documentation time [S23] [S24]. Vendor marketing; the figures are not independent.
- **Open source.** DHIS2 Tracker has "working lists" of enrolled people filtered by event status (including `OVERDUE`), scheduled date and a follow-up flag, and an immunisation registry design that ships a predefined "missed appointments" list [S25]. OpenMRS has a cohort module with members, membership dates proposed, and household cohorts for outreach programmes [S26]. Neither is a care-management product; both show the list-of-people-with-a-due-date shape.
- **What a payer asks for (US).** Medicare's chronic care management service requires a comprehensive care plan based on a physical, mental, cognitive, psychosocial, functional and environmental assessment, shared with the patient and carer, for patients with two or more chronic conditions expected to last at least 12 months [S27] (older editions and secondary readings; the June 2025 booklet itself returned 403). The AMA reads the CMS booklet as not requiring consent to be re-taken on a schedule [S28].

### Evidence for transitions of care

The Coleman care transitions trial (2006) found lower rehospitalisation at 30 days (8.3% against 11.9%) and 90 days (16.7% against 22.5%) with a nurse "transition coach", a personal health record and a home visit [S29] (abstract via AHRQ PSNet and the trial's own site). One trial in one Colorado system: a reason to model the transition as a pathway step with an owner, not a promise of the same effect.

### Standards

- **CarePlan** (the plan for one patient, possibly cross-organisational), **CareTeam** (who is on it), **Goal**, **Task** (a unit of work with an owner and a status), **ServiceRequest**. The project already reads CarePlan as the patient's instance of a pathway template ([pathway design](../pathway-design.md)). Base specification pages [S14].
- **EpisodeOfCare** is the tracking resource (for example "enrolled in care management from 1 March") and is not where responsibility lives ([5.2.1](../05-domain-model.md)).

## Part 2. Registries and risk stratification

### How others do it

Every vendor above sells registries and risk scores together. Arcadia names "HCC suspecting" (finding diagnoses that would raise a payment risk score) among its risk tools [S22]; Innovaccer lists CMS-HCC, HHS-HCC, CDPS and SVI scores [S24] (third-party listing). These are payment risk-adjustment and cost models. **A cost model used to decide who gets extra care is exactly the failure Obermeyer and colleagues found**: a commercial algorithm used for about 70 million people predicted cost, less is spent on Black patients with the same need, so at the same score Black patients were considerably sicker; correcting the label would have raised the share of Black patients picked for extra help from 17.7% to 46.5% [S30]. The lesson the authors draw is that the choice of a convenient proxy label is itself a source of bias.

### Registries in FHIR

A registry is a **Group**. Group "represents a defined collection of entities that may be discussed or acted upon collectively"; `actual = false` is a definition (the criteria), `actual = true` is a list of real members; `characteristic` holds the shared traits with an `exclude` flag; `member.period` and `member.inactive` show when someone joined and left [S13]. Group is also the unit of a Bulk Data Group export [S12] and of a Da Vinci attribution list [S10]. A registry definition can also be a CQL `Library` expression (Part 4), so that the registry and the measure that counts it use one definition.

### Scores that exist, their evidence and their licences

| Score | Predicts | Evidence read | Licence | Status here |
|---|---|---|---|---|
| LACE (van Walraven 2010) | Death or unplanned readmission within 30 days | Derivation C statistic 0.684; poor in an older UK population; a Swiss study of patients 80 and over found LACE, HOSPITAL and two others all near chance (AUC 0.51 to 0.59) [S31] (secondary summaries) | Published in CMAJ; licence for encoding not found [uncertain] | None |
| HOSPITAL (Donzé 2013, international validation 2016) | Potentially avoidable 30-day readmission | Better than LACE in one US hospital; good in one Swiss prospective study; poor in the over-80 study above [S31] (secondary) | Not found [uncertain] | None |
| Readmission models in general | | A systematic review of 26 models found most "perform poorly"; models with functional and social variables did better [S32] (abstract and press release) | n/a | None |
| eFI (Clegg 2016) | Frailty from 36 coded deficits in primary care | Predictive validity for death, admission and nursing home admission; equal weights and fixed cut-points were later named as limits [S33] (abstract) | Not found for the 2016 index [uncertain] | None |
| eFI2 (Best 2025) | Frailty, reworked | External C-index 0.723 with "some evidence of miscalibration"; paper CC BY-NC [S33] (abstract) | The paper is CC BY-NC; the model's licence is not stated in what was read [uncertain] | None |
| Clinical Frailty Scale (Rockwood) | Frailty, judged by a clinician | n/a here | Dalhousie grants non-commercial clinical use usually without a fee through a permission portal; commercial use may need a licence; translations need their own permission [S34] | None; EM-3 and OQ-M2 apply |
| QAdmissions (ClinRisk) | Emergency admission, ages 18 to 100 | Developed and externally validated [S35] (one paper, abstract) | One tender says the open-source code is AGPL-3.0, with commercial kits from ClinRisk [S35] (single secondary source) [uncertain] | None |
| CMS-HCC and similar | Next year's cost, for payment adjustment | Not researched this pass | CMS publishes the model [uncertain] | **Never as a care-priority score** (rule R-7) |

### Validation, bias and explainability

- **Reporting.** TRIPOD+AI (BMJ 2024) is a 27-item checklist for reporting prediction models, regression or machine learning, with items on fairness [S36]. A score adopted here should come with a TRIPOD+AI-shaped card.
- **Transparency rule (US).** ONC's HTI-1 rule created certification criterion § 170.315(b)(11), "decision support interventions", with 31 source attributes for a predictive intervention (who made it, training data, performance, fairness, maintenance) that certified software must let users see in plain language, and must say when an attribute is unavailable [S37] (fact sheet; dates **[uncertain]**). This project is not certified, but the 31 attributes are a good list for a model card.
- **Race in algorithms.** Vyas and colleagues catalogue clinical tools that "correct" by race and steer resources toward white patients [S38]. A score here never takes race as an input to adjust care priority; race and ethnicity are used to **audit** a score, not to drive it.
- **Absent is unknown.** Every score has inputs a record may lack (a frailty deficit never coded, a social factor never asked). The repository's rule already says a missing item is "not assessed", never scored as zero (EM-3). A score with missing inputs is shown as incomplete, with the list of what is missing, not as low.
- **The core forbids score fields.** SW-1 built `tests/risk_field_census.rs`, which fails if a score-shaped field appears in the core; a pack that adds one must declare it. So a risk score belongs in a **declared pack**, never in the core tables.

## Part 3. Outreach

### How others do it

The vendors above all run outreach campaigns over a list (letter, call centre, text, portal, and automated voice) and count responses [S22] [S23] [S24]. DHIS2 shows a simpler shape: an overdue list that a health worker works through [S25].

### Evidence that outreach works, and its limits

A meta-analysis of 73 US trials (about 366,766 people) found mailed stool-test outreach roughly doubled colorectal screening completion against usual care (risk ratio 2.26) and patient navigation did similarly (2.01); the authors note effects may differ in disadvantaged groups [S39] (summaries, not the full text). One cluster trial in community health centres found a much smaller real-world gain (13.9% against 10.4%) [S39]. Outreach is worth building, and its effect must be measured, not assumed.

### Rules others must follow (pointers, not legal advice)

- **US calls and texts.** The FCC's 2015 health care exemption to the Telephone Consumer Protection Act is described as covering free-to-the-patient automated calls and texts by or for a provider, to the number the patient gave, with an immediate opt-out ("STOP"), at most one a day and three a week, and not for billing [S40] (law-firm summaries; the order itself was not read) **[uncertain]**.
- **England, data opt-out.** The operational policy of England's national data opt-out says risk stratification for case finding by a provider involved in the patient's care is individual care, and the opt-out does not apply; it does apply where the disclosure relies on section 251 support [S41] (version 4.0, 2020, via a mirror; the live page may differ). The service already holds a `national-data-opt-out` consent kind that denies the `research` purpose.
- **Death.** The outbox already holds any event for a patient with a standing report of death (BV-3). A recall letter to a dead patient is a known harm to a family; the hold must cover every outreach channel.

### Standards

**CommunicationRequest** (an outreach that should happen), **Communication** (one that did, with its channel and outcome), **Task** (the work item for staff), and **Consent** for channel preferences and opt-outs [S14]. The DEQM gaps-in-care report (Part 4) is the standard input list.

## Part 4. Quality measure reporting and attribution

### United States

- **eCQMs and the move to FHIR.** CMS's eCQMs are written in CQL today over the older QDM data model and reported in QRDA. CMS is moving to FHIR-based digital quality measures (dQMs) that keep CQL and drop QDM and QRDA; draft dQM packages for 49 clinician and 21 hospital measures were out for comment in early 2026, and one summary describes optional reporting first and mandatory later, with no date set [S1] [S2]. The 2026 eCQM specifications are on the eCQI Resource Center [S3]. Dates **[uncertain]**; IO-14 already says so.
- **QI-Core** (STU 7.0.2, FHIR R4, aligned to US Core 7) is the profile set measures are written against. It defines two kinds of absence: data that is not there, and "documented absence of data with a valid reason" through negation profiles (an activity intentionally not done, with a reason) [S4]. The second fits this repository's rule that absent evidence is null with a reason.
- **Quality Measure IG** (CQF Measures STU 5) profiles `Measure` and `Library` (CQL) for proportion, ratio, continuous-variable and cohort measures [S5]. The population codes are initial population, denominator, denominator exclusion (removed before the numerator is checked), denominator exception (removed only when the numerator is not met; proportion measures only), numerator, numerator exclusion, and the continuous-variable populations [S6].
- **DEQM** (Da Vinci Data Exchange for Quality Measures, STU 5.0.0) defines **MeasureReport** as individual (one per attributed patient, with the data used), subject list, and summary (counts and score only); data exchange scenarios to submit and collect a measure's data; and a **gaps-in-care report** of open, closed and prospective gaps for a patient or group [S7].
- **Engines and test tools, open source.** The CQL-to-ELM translator and the Java CQL engine are Apache-2.0 in the HL7 `clinical_quality_language` project; cqf-ruler puts the engine in a HAPI FHIR server; Blaze and an experimental Google engine in Go also run CQL [S8]. Cypress is ONC's open-source tool for testing eCQM calculation (user guide 7.1, June 2025); Bonnie tests measure logic [S9].
- **MIPS** asks for at least 75% data completeness in 2026; eCQMs are all-payer, so they cover every patient seen [S15] (CMS pages and implementation guide, via search).
- **Medicare Shared Savings Program.** ACOs report the APP Plus measure set (six measures in 2025, eight in 2026, rising later), by eCQM, MIPS CQM or Medicare CQM; when MIPS CQMs stop being allowed is contested between sources [S16] **[uncertain]**.
- **ACO REACH ends at the end of 2026**; CMS's **LEAD** model (Long-term Enhanced ACO Design) runs ten years from 1 January 2027, with claims-based and voluntary alignment [S17] [S18].
- **HEDIS** belongs to NCQA. Internal, non-commercial use needs a user licence to the specifications; a vendor that computes measures for others needs a commercial licence; HEDIS value sets may not be changed or reused in other measures without permission; results may not be called HEDIS rates until audited [S19]. NCQA plans to retire the hybrid (chart review) method by measurement year 2029 and move to digital HEDIS by about 2030 [S42]. **This project cannot ship HEDIS logic or value sets** under its own licence.

### Attribution

Attribution decides which provider is accountable for which patient. Medicare assigns by claims (who provided most primary care services) and by voluntary alignment, where the patient names a provider, which supersedes claims [S43]; LEAD uses both [S18]. **Da Vinci ATR** (STU 2.1.0, FHIR R4) exchanges a payer's member attribution list with providers as a Group, with `$member-add`, `$member-remove`, a confirm operation and Bulk Data export [S10] [S11]. An attributed list is a payer's statement; it is not a care relationship and must not grant access by itself.

### England: an indicator framework of the same shape

England's primary care contract pays on the Quality and Outcomes Framework. Its clinical codes and extraction logic are "business rules" written entirely in SNOMED CT, with code clusters published as reference sets, and addressed to GP system suppliers [S44]. Since April 2019 the older "exception reporting" was replaced by **personalised care adjustments** with stated reasons (the patient chose not to have the intervention, it is clinically unsuitable, the service is unavailable, and others), and achievement recorded in the period overrides an adjustment [S45] (guidance and secondary readings). That is the same idea as QI-Core's documented absence with a reason. The licence of the business rules was not established: the pages read carry the issuing body's copyright notice and no Open Government Licence statement was found **[uncertain]**. A national (Welsh) or other framework was not researched.

## Workflows

1. **Define a registry.** A named owner writes a definition (conditions, codes, age, a pathway) as a draft, a second person reviews, a third approves; it is versioned like a pathway template (requirement 2 of [pathway design](../pathway-design.md)). The definition is a `Group` with `actual = false` and a CQL expression.
2. **Run the registry.** The service evaluates the definition over data that exists, under a stated purpose, and writes a dated member list (`Group` with `actual = true`), each member with the rule that put them in and the data it read. A patient whose record lacks a field the rule needs is listed as **"cannot tell"** with the missing field, not left out silently.
3. **Score the registry (optional, a declared pack).** A scoring run attaches, per member, the score, the model and version, the inputs used, the inputs missing, and the date. A member with missing inputs is "incomplete", never "low".
4. **Review and assign.** A care manager sees the list ordered by a reason they can read, picks patients for care management, and records why (and why not). The score never enrols anyone.
5. **Enrol and plan.** Enrolment opens an `EpisodeOfCare` and a pathway with a `CarePlan`, `CareTeam`, goals and tasks; the care manager is the step's owner (5.2.1 rule 3). Patient consent to the programme, where the payer or law asks for it, is recorded as said.
6. **Work the caseload.** The care manager's worklist (CA-6) shows each patient's next step, overdue tasks, and transitions (an admission, a discharge, an emergency visit) as they arrive (IO-6 subscriptions, HL7 ADT already inbound).
7. **Find gaps.** A measure or protocol is run as a gaps-in-care report (DEQM): open, closed and prospective gaps per patient, each with the rule and the data behind it.
8. **Build an outreach list.** From the gaps, a staff member builds a list. Before anything is sent, the list is filtered for: death (the BV-3 hold), opt-outs and channel refusals (consent), a reasonable adjustment that names a channel or format (PA-6), an untold suspicion (RS-4), and a recent contact (frequency cap). Each patient removed is counted with the reason.
9. **Contact.** Each attempt is a `CommunicationRequest` through the one outbound gate (outbox, PA-13), on a channel the patient accepts. A phone call made by a person is recorded as a `Communication` with its outcome.
10. **Record the outcome.** Reached and booked; reached and declined (with the patient's words); not reached (wrong number, no answer, returned letter); opted out (applied at once, to all future lists). "No reply" is not "declined".
11. **Close the gap** only when the care is done or a documented reason is recorded (a personalised-care-adjustment-shaped reason with who and when). Outreach alone never closes a gap.
12. **Report.** A measure run produces `MeasureReport`s: individual (for the care team's own use), summary (counts and score) for a contract, with the measure version, the period, the attribution list used, and the count of patients that could not be evaluated. Every external summary passes the small-cell rule (PA-7).
13. **Check fairness.** Each registry, score and measure report is run again split by the groups the deployment records (age band, sex, ethnicity where recorded, deprivation, language, disability), showing missing-data rates beside each rate (Part 2).

## Rules (proposed)

Each is written to be testable.

1. **R-1 A registry shows its definition.** Every member list names the definition, its version, the run time and the rule each member met; a list without these is refused.
2. **R-2 Cannot tell is a result.** A patient whose record lacks a field a rule reads is reported as "cannot tell" with the field named, never counted as in, out, met or not met.
3. **R-3 A score shows its working.** A risk score is shown only with its model name, version, run date, the input values used, and the inputs that were missing (shown as unknown, never as zero or normal).
4. **R-4 Missing inputs are not low risk.** A score with any required input missing is shown as "incomplete" and is not ranked below complete scores; a test feeds a record with every input missing and checks it is not at the bottom of a list.
5. **R-5 A score proposes, a person decides.** No score, registry or gap enrols, discharges, orders, sends or closes anything by itself; each such act names the person who did it (pathway design requirement 7).
6. **R-6 Scores live in packs.** A risk score is a declared pack (SW-1 census `DECLARED`), with a model card covering at least the HTI-1 predictive source attributes and the TRIPOD+AI items, a named clinical approver and a date, or it is shown as a draft.
7. **R-7 Cost is not need.** A model trained to predict cost or utilisation is never used to order patients for care; a pack declares its outcome label, and the census rejects a care-priority pack whose label is cost.
8. **R-8 No race correction.** A care-priority score takes no race or ethnicity input; race and ethnicity are used only in the fairness report.
9. **R-9 Every score and measure has a fairness report** split by each recorded group, with the missing-data rate per group, before it is published and at each new version.
10. **R-10 Outreach respects the patient.** An outreach list excludes anyone with a standing report of death, an opt-out or channel refusal, or a consent denying the purpose; a test sends to each excluded case and finds every send refused and audited at the outbox.
11. **R-11 One gate.** Every outreach message passes the outbox gate (PA-13); a test finds no other send path.
12. **R-12 Opt-out is immediate and permanent until withdrawn.** An opt-out recorded at 10:00 removes the patient from a list built at 10:01 and from any queued message not yet sent.
13. **R-13 No reply is not a decline.** The outcome codes keep "not reached" and "declined" distinct, and only "declined" with the patient's words may support a documented reason.
14. **R-14 Outreach never closes a gap.** A gap closes on the care being done or a documented reason with who and when; a test with a sent letter and no care finds the gap open.
15. **R-15 Attribution is not access.** An attribution list never creates a care relationship; a test reads a patient attributed but with no relationship and is refused.
16. **R-16 A measure report states what it could not see.** Every `MeasureReport` names the measure and version, period, attribution list, and the count of patients not evaluable with reasons.
17. **R-17 Small cells are hidden.** Every summary leaving the service passes PA-7 (k at least 5, secondary suppression).
18. **R-18 Measures run off the clinical database.** Measure and registry runs read an export or views (IO-4, IO-7), never the live clinical tables (IO-14).
19. **R-19 Licensed content is not shipped.** No HEDIS logic or value set, and no score whose licence is unknown, is committed to the repository; a licence field is required on every measure and score pack.
20. **R-20 Purpose is named.** A registry or outreach run reads under `treatment` or `operations` for the deployment's own patients; contract reporting reads under `payment`; neither is ever `research`. No role holds `payment` today, so contract reporting cannot run until a role is granted it on purpose (OQ-PH2).

## Hazards to add when built

Described only, for the Clinical Safety Officer; none is accepted or closed, and none is in the hazard log yet. Each goes to AS-1 with the first code that could cause it.

- **PH-H1 A biased score lowers care for a group.** A model whose label is a proxy (cost, past use) ranks a group as less needy than it is, so fewer of them are offered care management [S30].
- **PH-H2 Missing data read as low risk.** A patient with little recorded (new, rarely seen, records elsewhere) scores low and is never offered help.
- **PH-H3 Score trusted beyond its evidence.** A score with poor or untested discrimination in this population (readmission models [S31] [S32]) is treated as accurate.
- **PH-H4 Silent registry miss.** A coding change, a value-set update or a new version of the definition drops patients from a registry without anyone seeing it.
- **PH-H5 Outreach to the wrong person or the dead.** A letter or text goes to a deceased patient's home, to a stale number, or to a shared phone, revealing a condition or causing distress.
- **PH-H6 Opt-out ignored.** A patient who refused a channel or opted out is contacted again.
- **PH-H7 Gap closed by contact, not care.** A gap marked closed because a letter was sent; the patient never has the test.
- **PH-H8 Documented reason misused.** A "patient declined" or "clinically unsuitable" reason added to improve a measure, so the patient is left out of care and of the count (the problem QOF's 2019 change addressed [S45]).
- **PH-H9 Attribution taken for responsibility.** A team believes another provider is looking after an attributed patient, or that being attributed gives the right to read the record.
- **PH-H10 Care manager caseload lost at a transition.** An admission or discharge is not seen by the owning care manager, or the patient leaves the programme with no owner.
- **PH-H11 Measure logic error.** A wrong denominator exclusion or exception misreports care to a payer and hides a real gap.
- **PH-H12 Small numbers identify a patient** in a published report.

## First slice

The smallest piece that is useful and safe, using only what exists:

1. **A registry definition and run over existing data**, with no score: for example "patients with an active allergy to a penicillin-class drug and an open referral", or a definition over observations and assessments. Definition versioned, run dated, each member with the rule met, "cannot tell" listed (R-1, R-2). Read under `operations`. No screen beyond a list.
2. **A gap list from one protocol** written by the project as an example (clearly marked as not a clinical standard), e.g. "an assessment of form X due and not done" using WL-3's versioned forms, as a DEQM-shaped gaps report.
3. **An outreach list builder that only filters**: it applies the death hold, opt-outs and consent, and counts removals with reasons (R-10, R-12). It sends nothing until PA-13 has a real sink.
4. **A summary MeasureReport** of the gap list through PA-7's suppression, on synthetic data.

No risk score, no HEDIS, no CMS submission, and no attribution in the first slice.

## What this repo has and lacks

| Need | Here today | Gap |
|---|---|---|
| Who the patient is | Identity and matching built | No address or phone model for outreach (see virtual ward) |
| Data a registry could read | Allergies, observations, results, referrals with a closed loop, assessments with versioned forms | No problem list, medicines, immunisations or encounters history; no coded conditions to define most registries |
| A list of patients a clinician keeps | Watch lists with a reason (WL-6), own list only | No team list, no definition-driven list |
| Worklist | Referral worklist built | No general team inbox (CA-6) |
| Care plan, care team, tasks | Designed (5.2.1, pathway design) | Not built (CA-2, XP-12) |
| Risk score | None; core forbids score fields (SW-1) | Pack mechanism (XP-10), model cards, fairness report |
| Consent and opt-out | Consent with purpose of use; national data opt-out kind denies `research` | No channel preferences; `payment` and `research` permitted to no role |
| Outbound messages | Outbox with policy, consent and death hold; only a `log` sink | No letter, text or portal sender; PA-13 opt-outs and quiet hours not built |
| Measure engine | None | CQL engine, QI-Core mapping, Measure and MeasureReport (IO-14) |
| Export for analysis | None | Bulk Data (IO-4), SQL-on-FHIR views (IO-7) |
| Small-cell rule | `privacy.rs` (PA-7), no caller | A caller |
| Attribution | None | ATR Group import, and the rule that it is not access |

## Not verified

- Most facts are from search snippets or secondary summaries. Pages fetched and read in part: the DEQM, ATR, QI-Core and Quality Measure IG home pages, the Bulk Data export page (version line only), FHIR R4 Group and the measure-population code system.
- Not read in full: any CMS rule, the June 2025 chronic care management booklet (403), the FCC 2015 order, the LEAD request for applications, the MSSP assignment specification, any HEDIS licence text, the QOF business rules or their licence, the national data opt-out policy's live page, and every paper cited (abstracts or summaries only). CommunicationRequest, Communication, CarePlan, CareTeam and Task are cited from the base specification by name and were not re-read.
- Licences for LACE, HOSPITAL, eFI and QAdmissions are not established; the QAdmissions AGPL claim rests on one tender notice.
- The Obermeyer and Vyas DOIs are given from memory and match the titles found; the article pages were not opened.
- Epic, Arcadia and Innovaccer were described from their marketing and third-party listings, not product documentation or a demonstration.
- No Welsh, Scottish or non-UK, non-US framework was researched (Australia, Canada, low-income settings), so the shape here leans on US and English examples; under the [equity rule](../02-scope.md) these are examples, not the default.
- No clinician, care manager, lawyer or data protection officer has reviewed anything.

## Sources

- [S1] eCQI Resource Center, CMS flyer on the transition from eCQMs to FHIR dQMs: https://ecqi.healthit.gov/sites/default/files/CMS-Transiton-from-the-eCQM-approach-to-a-FHIR-ASTP-Flyer.pdf (snippet)
- [S2] CMS MMS Hub, dQM public comment webinar, January 2026: https://mmshub.cms.gov/2026/2026-01/register-cms-webinar-january-21-2026-digital-quality-measure-dqm-public-comment-period (snippet); trade summary of the roadmap: https://www.hcinnovationgroup.com/policy-value-based-care/value-based-care-quality-measurement/article/55355919/cms-roadmap-for-switching-to-fhir-based-digital-quality-measures (snippet)
- [S3] eCQI Resource Center, 2026 eCQM specifications: https://ecqi.healthit.gov/updated-ecqm-specifications-and-implementation-resources-2026-reporting/performance-period (snippet)
- [S4] HL7 QI-Core Implementation Guide STU 7.0.2: https://hl7.org/fhir/us/qicore/ (fetched, home page)
- [S5] HL7 Quality Measure IG (CQF Measures) STU 5: https://hl7.org/fhir/us/cqfmeasures/ (fetched, home page)
- [S6] FHIR R4 MeasurePopulationType code system: https://hl7.org/fhir/R4/codesystem-measure-population.html (fetched)
- [S7] HL7 Da Vinci DEQM STU 5.0.0: https://hl7.org/fhir/us/davinci-deqm/ (fetched, home page)
- [S8] HL7 CQL reference implementations: https://cql.hl7.org/10-c-referenceimplementations.html; cql-engine merged into clinical_quality_language: https://github.com/cqframework/cql-engine; Google CQL in Go: https://opensource.googleblog.com/2024/07/google-cql-from-clinical-measurements-to-action.html (snippets)
- [S9] Cypress: https://ecqi.healthit.gov/tool/cypress and https://www.healthit.gov/cypress/about.html; Bonnie: https://ecqi.healthit.gov/tool/bonnie (snippets)
- [S10] HL7 Da Vinci Member Attribution List (ATR) STU 2.1.0: https://hl7.org/fhir/us/davinci-atr/ (fetched, home page)
- [S11] HL7 Bulk Data Access STU 3: https://hl7.org/fhir/uv/bulkdata/ (version only)
- [S12] Bulk Data export operation: https://hl7.org/fhir/uv/bulkdata/export.html (version line only; the Group-level endpoint was not read on this pass) [uncertain]
- [S13] FHIR R4 Group: https://hl7.org/fhir/R4/group.html (fetched)
- [S14] FHIR R4 base resources: https://hl7.org/fhir/R4/careplan.html, https://hl7.org/fhir/R4/careteam.html, https://hl7.org/fhir/R4/task.html, https://hl7.org/fhir/R4/communication.html, https://hl7.org/fhir/R4/communicationrequest.html, https://hl7.org/fhir/R4/measure.html, https://hl7.org/fhir/R4/measurereport.html, https://hl7.org/fhir/R4/library.html (cited by name, not re-read)
- [S15] QPP, APP quality requirements 2026: https://qpp.cms.gov/reporting-requirements/ways-to-report/app/quality?py=2026; CMS 2026 QRDA III EC IG: https://www.cms.gov/files/document/2026-cms-qrda-iii-ec-ig-v1-1.pdf (snippets)
- [S16] 42 CFR 425.512: https://www.law.cornell.edu/cfr/text/42/425.512; MSSP 2026 summary: https://mdinteractive.com/mips-blog/cms-finalizes-2026-updates-medicare-shared-savings-program; CY 2027 proposal summary: https://www.appliedpolicy.com/wp-content/uploads/CY-2027-PFS_MSSP-Detailed-Summary-7.20.26.pdf (snippets) [uncertain]
- [S17] Healthcare Dive on LEAD replacing ACO REACH: https://www.healthcaredive.com/news/aco-lead-medicare-accountable-cms-reach-replacement/808356/ (snippet)
- [S18] CMS LEAD model overview: https://www.cms.gov/priorities/innovation/files/lead-model-overview.pdf; LEAD FAQ: https://www.cms.gov/priorities/innovation/lead-model-frequently-asked-questions (snippets)
- [S19] NCQA, using HEDIS measures: https://ncqa.org/hedis/using-hedis-measures; Digital Content Services handbook: https://wpcdn.ncqa.org/www-prod/wp-content/uploads/NCQA1289-0326-Digital-Content-Services-Customer-Handbook.pdf (snippets)
- [S20] OCHIN, Healthy Planet one-pager: https://ochin.org/wp-content/uploads/2023/06/HealthyPlanetOne-Pager.pdf (snippet)
- [S21] University Hospitals, population health with Healthy Planet: https://www.uhhospitals.org/for-clinicians/internal-communications/articles/2022/07/population-health-management-with-epic-healthy-planet (snippet)
- [S22] Arcadia, population health: https://www.arcadia.io/population-health (snippet, vendor)
- [S23] Innovaccer, voice agents: https://innovaccer.com/voice-agents; care management agents release: https://innovaccer.com/news/innovaccer-launches-copilots-agents-care-management-reducing-documentation-time-boosting-patient-engagement (snippets, vendor)
- [S24] Avia marketplace comparison listing Innovaccer features: https://marketplace.aviahealth.com/compare/54974/25515 (snippet, third party)
- [S25] DHIS2 Tracker API working lists: https://docs.dhis2.org/en/develop/using-the-api/dhis-core-version-241/tracker.html; immunisation eRegistry design: https://docs.dhis2.org/en/implement/health/immunization/eir-immunization-eregistry/design.html (snippets)
- [S26] OpenMRS Cohort Module: https://openmrs.atlassian.net/wiki/spaces/projects/pages/26935775/Cohort+Module (snippet)
- [S27] CMS chronic care management booklet MLN909188 (June 2025): https://www.hhs.gov/guidance/sites/default/files/hhs-guidance-documents/CMS/mln909188_chroniccaremanagement_jun2025.pdf (returned 403; content from older editions in search snippets) [uncertain]
- [S28] AMA, consent for chronic care management: https://www.ama-assn.org/practice-management/medicare-medicaid/consent-chronic-care-management-required-regularly (snippet)
- [S29] Coleman et al. 2006, care transitions intervention trial: https://psnet.ahrq.gov/issue/care-transitions-intervention-results-randomized-controlled-trial and https://caretransitions.org/files/wp-content/uploads/2015/06/39_the-care-transitions-intervention-results-of-a-randomized-controlled-trial.pdf (snippets)
- [S30] Obermeyer, Powers, Vogeli and Mullainathan 2019, Science 366:447: https://doi.org/10.1126/science.aax2342 (DOI from memory; findings from summaries, e.g. https://chmi.berkeley.edu/news/dissecting-racial-bias-in-an-algorithm-used-to-manage-the-health-of-populations)
- [S31] LACE and HOSPITAL: AAFP review https://www.aafp.org/afp/2016/0815/p307; PeerJ comparison https://peerj.com/articles/3137.pdf; CMAJ LACE https://cmaj.ca/content/182/6/551; Swiss over-80 comparison https://smw.ch/article/doi/smw.2018.14624 (snippets)
- [S32] Kansagara et al. 2011, JAMA 306(15):1688, readmission risk models review: https://www.eurekalert.org/pub_releases/2011-10/jaaj-mhr101311.php (press release and abstract snippets)
- [S33] eFI, Clegg et al. 2016: https://pubmed.ncbi.nlm.nih.gov/26944937/; eFI2, Best et al. 2025, Age and Ageing 54(4) afaf077: https://academic.oup.com/ageing/article/54/4/afaf077/8101467 (abstracts)
- [S34] Dalhousie University, permission for use (Clinical Frailty Scale): https://www.dal.ca/sites/gmr/our-tools/permission-for-use.html (snippet)
- [S35] QAdmissions paper: https://pmc.ncbi.nlm.nih.gov/articles/PMC3753502; licence via Australian tender notice: https://www.tenders.gov.au/ATM/Show/f0cf86f2-a29d-ce0b-fd18-8b0403ae2d38 (snippets) [uncertain]
- [S36] Collins et al. 2024, TRIPOD+AI, BMJ 385:e078378: https://www.ncbi.nlm.nih.gov/pmc/articles/PMC11025451/ (snippet)
- [S37] ONC HTI-1 decision support interventions fact sheet: https://www.healthit.gov/sites/default/files/page/2023-12/HTI-1_DSI_fact%20sheet_508.pdf (snippet) [uncertain on dates]
- [S38] Vyas, Eisenstein and Jones 2020, NEJM 383:874, "Hidden in Plain Sight": https://doi.org/10.1056/NEJMms2004740 (DOI from memory; summary at https://psnet.ahrq.gov/issue/hidden-plain-sight-reconsidering-use-race-correction-clinical-algorithms)
- [S39] Dougherty et al. 2018, JAMA Intern Med, interventions to increase colorectal screening: https://doi.org/10.1001/jamainternmed.2018.4637 (summaries, e.g. https://www.healio.com/news/hematology-oncology/20181115/multicomponent-interventions-increase-colorectal-cancer-screening-rates); Coronado et al. 2018 STOP CRC: https://pubmed.ncbi.nlm.nih.gov/30083752/ (snippets)
- [S40] TCPA health care exemption summaries: https://www.bassberry.com/news/tcpa-exemptions-for-healthcare-companies/ and https://www.klgates.com/Health-Care-Entities-Get-Clarity-from-FCC-on-Telephone-Communications-08-07-2015 (snippets, not the FCC order) [uncertain]
- [S41] National data opt-out operational policy, specific purposes: https://digital.nhs.uk/services/national-data-opt-out/operational-policy-guidance-document/policy-considerations-for-specific-organisations-or-purposes (snippet of version 4.0 via a mirror)
- [S42] NCQA, timeline for retiring HEDIS hybrid measures: https://www.ncqa.org/blog/ncqas-proposed-timeline-for-retiring-and-replacing-hedis-hybrid-measures/ (snippet)
- [S43] CMS, MSSP shared savings and losses and assignment methodology: https://www.cms.gov/files/document/medicare-shared-savings-program-shared-savings-and-losses-and-assignment-methodology-specifications.pdf-1 (snippet)
- [S44] QOF business rules and SNOMED reference sets: https://digital.nhs.uk/data-and-information/data-collections-and-data-sets/data-collections/quality-and-outcomes-framework-qof; QOF guidance 2026/27: https://www.england.nhs.uk/wp-content/uploads/2026/03/prn02356-quality-outcomes-framework-guidance-2026-27-july-update.pdf (snippets)
- [S45] QOF guidance 2019 (personalised care adjustments): https://www.england.nhs.uk/wp-content/uploads/2019/05/gms-contract-qof-guidance-april-2019.pdf; QOF guidance 2025/26: https://www.england.nhs.uk/wp-content/uploads/2025/03/quality-outcomes-framework-guidance-for-2025-26.pdf (snippets)
