# Request for comments

A request to clinicians, clinical safety officers, privacy officers, integrators and security reviewers.

## What is proposed

Open-source electronic patient record software that is **safe, standards-native and auditable**: an
append-only longitudinal record, FHIR R4 as both the data model and the API, explainable patient identity
matching, and a clinician UI designed against the documented causes of clinician burnout. The research
and decisions are in [`plan.md`](plan.md); the backlog is [`tasks.md`](tasks.md).

**It is a scaffold today. It holds no clinical record and is not for clinical use.**

## Questions

1. **Scope.** Is the first release (ambulatory core, ordering and results, patient portal) the right slice?
   What would make you refuse to trust it with a real patient?
2. **Market.** UK first (UK Core, PDS, DCB0129 / DTAC) or US first (US Core, ONC)? Both are planned.
3. **Safety.** Who should be the named Clinical Safety Officer, and what clinical advisory group is needed?
4. **Identity.** Are the matcher's thresholds and its refusals (never auto-link on a missing given name; never
   auto-link twins; never link two different verified NHS numbers) the right ones?
5. **Licence.** Keep the five-way choice, or Apache-2.0 alone like HAPI FHIR, Medplum and EHRbase?
6. **Offline.** Is offline-first data entry needed, or are read-only downtime snapshots enough?

## How to reply

Open an issue, or email joel@joelparkerhenderson.com. Decisions that change the plan are recorded in
[`plan.md`](plan.md) and, once adopted, in [`spec/`](spec/index.md).
