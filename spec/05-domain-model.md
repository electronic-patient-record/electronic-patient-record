## 5. Domain Model

### 5.1 `Patient` (matcher crate): the thin matchable identity record

`id`, `names[] {family, given[]}`, `birth_date` (ISO 8601, possibly partial), `sex` (`male|female|other|unknown`),
`identifiers[] {system, value}`, `addresses[] {postal_code}`, `telecom[] {system, value}`. It is the API identity DTO
(no adapter) and is persisted as JSONB. Built; see the [matcher spec](../electronic-patient-record-matcher-with-rust/spec/index.md).

### 5.2 Clinical resources (planned)

FHIR R4, one resource type per concept: Condition, AllergyIntolerance, MedicationRequest/Statement, Immunization,
Observation, DiagnosticReport, ServiceRequest, Encounter, EpisodeOfCare, Composition/DocumentReference,
Questionnaire(Response), CarePlan, Goal, CareTeam, Task, Appointment/Slot/Schedule, Consent, Provenance, AuditEvent.
Profiled to UK Core and US Core; coded with SNOMED CT, LOINC (UCUM units), RxNorm/dm+d, CVX.

### 5.2.1 The care pathway comes first, not the separate episode

**Decided as the direction 2026-11-04 (the author's instruction; OQ-CP1 to OQ-CP4 hold the open details). Nothing is built.**

A patient's care is organised around **care pathways**: the planned, goal-directed route a patient takes through care for a condition or a need, across
teams, settings, organisations and time. The record, its screens and its safety checks treat the pathway as the unit that matters. An **encounter** (one contact)
and an **episode of care** (a period in which an organisation takes responsibility, kept for tracking, reporting or billing) are things that **happen along or under**
a pathway. They are not what a clinician navigates by, and they are not where responsibility is held or lost.

Why this ordering. Care is lost *between* separate episodes: a referral not answered, a result nobody owns after a transfer, a follow-up that belonged to a closed episode
(hazard H-044 is one such case, built so far for referrals only). A record organised by episode shows what each episode did and says nothing about what is still owed to the
patient. FHIR draws the same line: `EpisodeOfCare` is "a tracking resource, rather than a planning resource", it "usually exists before the CarePlan", and "you don't need a
CarePlan to use an EpisodeOfCare"; the `CarePlan` "describes the intention of how one or more practitioners intend to deliver care" and may be a "multi-disciplinary cross-organizational" plan.
So the pathway is a plan, not an episode, and cannot be modelled by one.

Design rules that follow:

1. **The first view of a patient is their open pathways**, each with its owner (a team, not only a person), its current step, its next step with a date or a stated reason there is none, and anything overdue.
   Encounters appear as dated events on a pathway's timeline. A patient with no pathway sees their encounters as before; a pathway is never required to record care.
2. **Every step is on a pathway or says it is not.** A referral, order, appointment, task or result expected back is a *step* of a pathway; its closed loop (the
   [referral state table](interoperability/closed-loop-referral.md) is the first) is shown on the pathway as an open or closed step. A step with no pathway is allowed, and then it is shown as "not on a pathway", never hidden.
3. **A pathway outlives its episodes.** Discharge, transfer, a new referral or a change of organisation end an episode or an encounter, not the pathway. A pathway ends only by a recorded outcome
   (completed, stopped with a reason, or handed over to a named owner). A pathway with no owner is a safety condition to be listed, not a state to be tolerated.
4. **Episodes are derived or administrative.** An `EpisodeOfCare` may be created for reporting, billing or a provider's own tracking, and may span or sit inside several pathways.
   No clinical rule depends on an episode existing.
5. **Mapping (FHIR R4, to be confirmed against the profiles in use):** the patient's pathway is a `CarePlan` (its `partOf` nests a step-plan inside a parent plan; `activity.reference` points at
   `ServiceRequest`, `Task` and `Appointment`); a reusable pathway template is a `PlanDefinition` applied to the patient (`instantiatesCanonical`); `Encounter` is linked to the pathway through the request that caused it,
   and optionally to an `EpisodeOfCare`. How exactly an `Encounter` points back to its `CarePlan` is not settled here (OQ-CP2).
6. **Templates are content, and content is a pack.** The service does not ship clinical pathways. A pathway template comes from a deployment's pack under the [equity rule](02-scope.md), with its own review status; a pathway
   built from a template says which template and which version.
7. **Not claimed.** There is no pathway engine, no guideline authoring and no automatic instantiation. Nothing here has had clinical review. A hazard row for pathways (for example a pathway that looks
   "on track" while a step was missed, or a template applied to the wrong patient) is added with the first code, in the same commit (AGENTS.md), not now.

### 5.3 Versioning and provenance

Every change writes a new `resource_version` row (never an update in place) with `meta.versionId`, author, time, reason
and a Provenance naming the agent (human, device or AI). The current version is a materialised view. `_history` and
"as seen at time T" fall out. Signed notes accept only addenda.

### 5.4 Partition

Thin matchable `Patient` identity vs the high-volume clinical resources; only identity takes part in matching.

### 5.5 Persistence outline

See [10 Persistence](10-persistence.md).

### 5.6 Sensitivity

FHIR security labels (`R`, `V`, `PSY`, `HIV`, `SUD`, `ETH`) mask content; a masked response says restricted
information exists and contains none of it.

### 5.7 UI types

The UI mirrors `Patient` and `MatchResult` in `src/routes/patients/+page.svelte`; change both together.

### 5.8 Shared invariants

The eight invariants in [`index.md`](index.md#shared-invariants-summary-58-is-canonical): append-only; every access
audited; no disclosure without lawful basis; coded where it matters; absent is not zero; UTC; references not copies;
hazard log before shipping.
