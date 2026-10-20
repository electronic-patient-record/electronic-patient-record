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
