//! Interoperability: mapping between the service's identity record (the matcher's `Patient`) and
//! the formats other systems speak. Nothing here is reachable over HTTP yet: these are the
//! mappings the FHIR endpoint (ID-3) and the HL7 v2 inbound adapter (OR-3) will call.
//!
//! - [`fhir_patient`]: FHIR R4 `Patient` (crate `fhir`).
//! - [`hl7_adt`]: HL7 v2 ADT patient feeds and their acknowledgements (crates `er7`, `hl7`).
//! - [`terminology`]: SNOMED CT identifier checks (crate `snomed`).
//!
//! A mapping that cannot carry something says so (`dropped`, `warnings`) instead of discarding it
//! silently: this is a patient record.

pub mod fhir_patient;
pub mod hl7_adt;
pub mod terminology;
