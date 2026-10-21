//! Interoperability: mapping between the service's records and the formats other systems speak.
//! The FHIR `Patient` and `AuditEvent` mappings are served by `controllers/fhir.rs`; the HL7 v2
//! and SNOMED ones are library only (no listener yet: OR-3).
//!
//! - [`fhir_audit`]: an audit row as a FHIR R4 `AuditEvent` (BALP-shaped).
//! - [`fhir_patient`]: FHIR R4 `Patient` (crate `fhir`).
//! - [`hl7_adt`]: HL7 v2 ADT patient feeds and their acknowledgements (crates `er7`, `hl7`).
//! - [`terminology`]: SNOMED CT identifier checks (crate `snomed`).
//!
//! A mapping that cannot carry something says so (`dropped`, `warnings`) instead of discarding it
//! silently: this is a patient record.

pub mod fhir_audit;
pub mod fhir_patient;
pub mod hl7_adt;
pub mod terminology;
