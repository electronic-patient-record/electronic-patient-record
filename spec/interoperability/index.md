# Interoperability

FHIR R4 (4.0.1) with UK Core and US Core 6.1.0 (tracking 8.x), IPS; SMART App Launch 2.2; CDS Hooks 2.0; Bulk Data 2.0; HL7 v2.5.1 inbound (ADT, ORU, SIU); C-CDA only when a US network requires it (HTI-5 would drop it, not final). Tasks IO-1 to IO-8, OR-3.

Planned; nothing here is built yet unless stated. **Built (2026-10-21, library only, no endpoint):** `interop::fhir_patient` (FHIR R4 `Patient` <-> the identity record, via the `fhir` crate), `interop::hl7_adt` (HL7 v2 ADT A01/A04/A08/A28/A31 to an identity record, with `AA`/`AE`/`AR` acknowledgements, via `er7` and `hl7`) and `interop::terminology` (SNOMED CT identifier check, via `snomed`). See also [the `openehr` crate compared with this project](openehr-comparison.md). Source: [`../../plan.md`](../../plan.md).
