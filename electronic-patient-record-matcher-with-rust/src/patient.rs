//! The thin, matchable patient identity record. This is also the API DTO.

use serde::{Deserialize, Serialize};

/// FHIR `Patient.gender` (administrative sex).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AdministrativeSex {
    Male,
    Female,
    Other,
    Unknown,
}

/// A name. `given` keeps order; `family` is a single string.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HumanName {
    #[serde(default)]
    pub family: Option<String>,
    #[serde(default)]
    pub given: Vec<String>,
}

/// An identifier such as an NHS number or a medical record number.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identifier {
    pub system: String,
    pub value: String,
}

/// An address. Only the postcode takes part in matching.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Address {
    #[serde(default)]
    pub postal_code: Option<String>,
}

/// A phone number or email address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Telecom {
    pub system: String,
    pub value: String,
}

/// The identity record. `birth_date` is ISO 8601 and may be partial
/// (`YYYY`, `YYYY-MM`, `YYYY-MM-DD`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Patient {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub names: Vec<HumanName>,
    #[serde(default)]
    pub birth_date: Option<String>,
    #[serde(default)]
    pub sex: Option<AdministrativeSex>,
    #[serde(default)]
    pub identifiers: Vec<Identifier>,
    #[serde(default)]
    pub addresses: Vec<Address>,
    #[serde(default)]
    pub telecom: Vec<Telecom>,
}
