# Interoperability

FHIR R4 (4.0.1) with UK Core and US Core 6.1.0 (tracking 8.x), IPS; SMART App Launch 2.2; CDS Hooks 2.0; Bulk Data 2.0; HL7 v2.5.1 inbound (ADT, ORU, SIU); C-CDA only when a US network requires it (HTI-5 would drop it, not final). Tasks IO-1 to IO-8, OR-3.

Planned; nothing here is built yet unless stated. **Built (2026-10-21):** `/fhir/R4/Patient`, a read-only `/fhir/R4/AllergyIntolerance?patient=` and `/fhir/R4/metadata` (controller `controllers/fhir.rs`; see the service spec) over `interop::fhir_patient` (FHIR R4 `Patient` <-> the identity record, via the `fhir` crate), `interop::fhir_allergy` (an allergy out as `AllergyIntolerance`; empty is never "none"), `mllp` and `adt_feed` (an MLLP listener for ADT feeds, off by default; see [`operations/hl7`](../../operations/hl7/README.md)) over `interop::hl7_adt` (HL7 v2 ADT A01/A04/A08/A28/A31 to an identity record, with `AA`/`AE`/`AR` acknowledgements, via `er7` and `hl7`) and `interop::terminology` (SNOMED CT identifier check, via `snomed`). See also [the `openehr` crate compared with this project](openehr-comparison.md). Source: [`../../plan.md`](../../plan.md).

## National standards

The target is worldwide, so FHIR support means **each country's own standard**, not UK Core and US Core alone, under the
[equity rule](../02-scope.md). The same pack idea covers other protocols: HL7 v2 national variants, CDA and C-CDA, openEHR
and national messaging, and the federations and exchanges a region runs (for example the European Health Data Space,
national and state exchanges, and the US TEFCA). Those too are packs, not special cases.

1. **Base is international.** The stored record is base FHIR (R4 today, R5 as a derived read, see below). A country's rules
   are layered on, never baked into the base model.
2. **A national profile pack per country.** A pack names: the country's core implementation guide and version (for
   example UK Core, US Core, AU Core, the Canadian Core, the German MII or ISiK profiles, the French and Spanish national
   guides, the IPS-based European guides, and others as they exist), its required terminologies and code systems, its
   patient and provider identifier systems (NHS number, SSN-style ids, IHI, and so on), its extensions, and its
   capability-statement claims.
3. **Selected by deployment, advertised honestly.** A deployment turns on the packs it needs. `metadata` lists only the
   profiles of enabled packs that pass validation, and claims no other conformance.
4. **A pack is supported only when its validator passes.** The HL7 validator runs with the pack's package loaded, as it does
   for UK Core and US Core today, with a control that must fail. A country with no published FHIR guide gets base FHIR
   and is listed as such.
5. **Terminology is not assumed.** SNOMED CT is licensed per member country, and some countries use other systems (ICD-10
   modifications, local drug codes, LOINC use varies). A pack declares its code systems; the service never invents
   a mapping between them.
6. **No pack changes the clinical rules.** Absent evidence stays `null` with a reason, and the record stays append-only,
   whatever the country.
7. **Review by someone who knows the country.** A pack written from a published guide is machine-assembled until a person
   who works with that country's standard reviews it. Never claim it is reviewed before that.

Status: **planned, nothing built.** The list of countries above is from my knowledge of what exists and has not been
checked against each publisher (task IO-18). Today the service has UK Core and US Core as references only; no pack mechanism
exists.

## FHIR R5

R4 (4.0.1) stays the baseline: it is what UK Core and US Core track. R5 (5.0.0) is added as a **second, read-only facade at
`/fhir/R5`**, built from the same stored record, so that a partner that speaks R5 (the patient administration system does,
OQ-PA3) does not need its own edge. Planned here, and **the read side is built (see below).** The `/api` rule against version segments does not
apply: that rule is about the application API, which versions by the `Accepts-version` header, and FHIR already carries its
release in the path (`/fhir/R4`).

**Built 2026-10-29 (IO-16), read side only:** `GET /fhir/R5/metadata`, `Patient` (search and read by id),
`AllergyIntolerance` and `Observation` (search by patient). Each route calls its R4 twin and converts the body, so the gate,
care relationship, purpose, consent and audit row are the R4 ones. The conversion fails the response (500 outcome, no value
named) if it reports any loss or warning; one retyping is mapped by hand (`AllergyIntolerance.reaction.manifestation`, a
`CodeableConcept` in R4, a `CodeableReference` in R5, so the R4 value goes in `concept`) and any other is still a failure.
A search set gets the `self` link R5 requires (bdl-18). Not built: `AuditEvent`, `Consent`, `Provenance`, any write,
`fhirVersion` negotiation on R4, and the check of the differences table below (IO-17).

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

**What differs, and where it bites.** Read 2026-10-29 from the HL7 R5 "Differences" page
(<https://hl7.org/fhir/R5/diff.html>), which lists R4 to R5 changes per element (task IO-17). The per-resource "version maps"
pages compare R3, not R4, and were not used.

| Resource | R4 to R5 differences, as the page states them | Risk |
|---|---|---|
| `Patient` | Only `communication.language`: binding changes from preferred (Common Languages) to required (All Languages) | Low: a language code outside All Languages would now be invalid |
| `AllergyIntolerance` | `verificationStatus` gains `presumed`; `type` becomes a `CodeableConcept` with no required binding; `reaction.manifestation` becomes a `CodeableReference`; `recorder` and `asserter` are deleted and replaced by `participant` (with `function` and a mandatory `actor`) | Medium: the amendment notice and the "not recorded" states ride on these fields; the converter's handling of `participant` is tested only where the service writes a recorder |
| `Observation` | Added `instantiates[x]`, `triggeredBy`, `bodyStructure`, `referenceRange.normalValue`; `referenceRange.text` string to markdown; `value[x]` gains `Attachment` and `Reference(MolecularSequence)`; `derivedFrom` loses `Media` as a target | Low: value, unit and range are not renamed |
| `AuditEvent` | `type`/`subtype` renamed `category`/`code` as `CodeableConcept`; `outcome` a backbone; `purposeOfEvent` renamed `authorization`; `period`, `outcomeDesc`, `agent.name`, `agent.altId`, `entity.type`, `entity.name` and more deleted; `patient`, `encounter`, `severity`, `occurred[x]` added | High: fields deleted outright; not served in R5 |
| `Consent` | `status` loses `proposed` and `rejected`; `patient` renamed `subject`; `dateTime` renamed `date` (type narrowed to `date`); `provision.type` moved to `decision`; `scope`, `policyRule`, `source[x]` deleted | High: a lossy conversion could change who may see a record; not served in R5 |
| `Provenance` | `reason` renamed `authorization` as a `CodeableReference`; `recorded` becomes optional; `patient`, `encounter`, `basedOn` added; `entity.role` code `derivation` removed | Medium: not served in R5 |
| Subscriptions | Topic-based subscription is native in R5 (R4 has a backport) | Not read from the specification; still `[uncertain]` |

**`fhirVersion`.** The HTTP page defines it as a MIME parameter (`Accept: application/fhir+json; fhirVersion=5.0`) naming
major and minor version, valid on `Accept`, `Content-Type` or both, and an `Accept` that disagrees with `Content-Type` is an
error unless the call is a conversion operation. It states **no status code for an unsupported version** (406 and 415 are
described for unsupported formats only). The service's 406 for another `fhirVersion` is therefore our choice, not a rule.

**Profiles.** UK Core and US Core are R4 guides; I found no R5 release of either: UK Core's published releases (STU2 line) and US Core 9.0.0 ("based on FHIR 4.0.1") are R4. So `/fhir/R5` is **base R5 only**, with no
claim of UK Core or US Core conformance, and says so in its capability statement.

**Clinical risk.** Serving a second shape of the same record can change what a clinician or a partner's system sees. Treat
the change that builds it as `clinical-impact` and add a hazard row first (AS-1). Tasks IO-16 and IO-17.

