# Interoperability

FHIR R4 (4.0.1) with UK Core and US Core 6.1.0 (tracking 8.x), IPS; SMART App Launch 2.2; CDS Hooks 2.0; Bulk Data 2.0; HL7 v2.5.1 inbound (ADT, ORU, SIU); C-CDA only when a US network requires it (HTI-5 would drop it, not final). Tasks IO-1 to IO-8, OR-3.

Planned; nothing here is built yet unless stated. **Built (2026-10-21):** `/fhir/R4/Patient`, a read-only `/fhir/R4/AllergyIntolerance?patient=` and `/fhir/R4/metadata` (controller `controllers/fhir.rs`; see the service spec) over `interop::fhir_patient` (FHIR R4 `Patient` <-> the identity record, via the `fhir` crate), `interop::fhir_allergy` (an allergy out as `AllergyIntolerance`; empty is never "none"), `mllp` and `adt_feed` (an MLLP listener for ADT feeds, off by default; see [`operations/hl7`](../../operations/hl7/README.md)) over `interop::hl7_adt` (HL7 v2 ADT A01/A04/A08/A28/A31 to an identity record, with `AA`/`AE`/`AR` acknowledgements, via `er7` and `hl7`) and `interop::terminology` (SNOMED CT identifier check, via `snomed`). See also [the `openehr` crate compared with this project](openehr-comparison.md). Source: [`../../plan.md`](../../plan.md).

## FHIR R5

R4 (4.0.1) stays the baseline: it is what UK Core and US Core track. R5 (5.0.0) is added as a **second, read-only facade at
`/fhir/R5`**, built from the same stored record, so that a partner that speaks R5 (the patient administration system does,
OQ-PA3) does not need its own edge. Planned; **nothing of it is built.** The `/api` rule against version segments does not
apply: that rule is about the application API, which versions by the `Accepts-version` header, and FHIR already carries its
release in the path (`/fhir/R4`).

**Rules**

1. **R4 is the source; R5 is derived.** The service writes and stores R4-shaped resources only. An R5 response is made by
   converting the R4 resource, never by a second set of converters that could drift from the first.
2. **A conversion that loses something does not pass silently.** The `fhir` crate's `convert::between::<R4, R5>` returns a
   loss report. For a clinical resource (`AllergyIntolerance`, `Observation`) a report that names a dropped or changed field
   makes the response fail (an `OperationOutcome`, 5xx) rather than serve a quieter record. Absent evidence is never "none".
3. **The same access rules.** Every `/fhir/R5` route has a row in the policy table, the same audit row and the same amendment
   notice as its R4 twin; the route census and the audit-failure census cover it.
4. **Say which release.** `/fhir/R5/metadata` states `fhirVersion` 5.0.0. A request that names the release another way (the
   `fhirVersion` parameter on `Accept`) is answered or refused with the release it asked for, never the other one.
5. **R5 is validated.** The HL7 validator runs on what `/fhir/R5` writes, with `-version 5.0`, as it does for R4.
6. **No R5 writes at first.** Reads only (Patient, AllergyIntolerance, Observation, `metadata`). A write from an R5 sender is
   a separate decision, because a lossy R5 to R4 conversion on the way in would change what the record says.

**What differs, and where it bites (from my knowledge of the release, not read from the specification: every item is
[uncertain] until checked, task IO-17)**

| Resource | Likely differences R4 to R5 | Risk |
|---|---|---|
| `Patient` | Few; mostly additions | Low |
| `AllergyIntolerance` | `type` is a coded concept, not a code; `reaction.manifestation` becomes a coded reference; `recorder` and `asserter` move into `participant` | Medium: the amendment notice and the "not recorded" states ride on these fields |
| `Observation` | New `triggeredBy` and `bodyStructure`; value and range handling is close | Low to medium: a reference range or unit must not be lost |
| `AuditEvent`, `Consent`, `Provenance` | Reshaped more heavily | Not served in R5 at first |
| Subscriptions | Topic-based subscription is native in R5 (R4 has a backport) | A reason to have R5 for IO-6 |

**Profiles.** UK Core and US Core are R4 guides; I found no R5 release of either. So `/fhir/R5` is **base R5 only**, with no
claim of UK Core or US Core conformance, and says so in its capability statement.

**Clinical risk.** Serving a second shape of the same record can change what a clinician or a partner's system sees. Treat
the change that builds it as `clinical-impact` and add a hazard row first (AS-1). Tasks IO-16 and IO-17.

