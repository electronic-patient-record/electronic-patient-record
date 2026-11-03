//! The thin, matchable patient identity record. This is also the API DTO.

use serde::{Deserialize, Serialize};

/// FHIR `Patient.gender` (administrative sex).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AdministrativeSex {
    /// Male.
    Male,
    /// Female.
    Female,
    /// Another administrative sex.
    Other,
    /// Not known, or no usable answer was given. Never read as a specific sex.
    Unknown,
}

/// A name. `given` keeps order; `family` is a single string.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HumanName {
    /// The family name, if known.
    #[serde(default)]
    pub family: Option<String>,
    /// The given names, in order.
    #[serde(default)]
    pub given: Vec<String>,
}

/// An identifier such as an NHS number or a medical record number.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identifier {
    /// The identifier's system, such as the NHS number system URI.
    pub system: String,
    /// The identifier's value within that system.
    pub value: String,
}

/// An address. Only the postcode takes part in matching.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Address {
    /// The postcode, if known.
    #[serde(default)]
    pub postal_code: Option<String>,
}

/// A phone number or email address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Telecom {
    /// The kind of contact, such as `phone` or `email`.
    pub system: String,
    /// The number or address.
    pub value: String,
}

/// The identity record. `birth_date` is ISO 8601 and may be partial
/// (`YYYY`, `YYYY-MM`, `YYYY-MM-DD`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Patient {
    /// The record's id once it is stored; absent before.
    #[serde(default)]
    pub id: Option<String>,
    /// The person's names. A record may carry several.
    #[serde(default)]
    pub names: Vec<HumanName>,
    /// The birth date, if known.
    #[serde(default)]
    pub birth_date: Option<String>,
    /// The administrative sex, if known. Absent is not the same as unknown.
    #[serde(default)]
    pub sex: Option<AdministrativeSex>,
    /// Identifiers such as an NHS number or a medical record number.
    #[serde(default)]
    pub identifiers: Vec<Identifier>,
    /// Addresses; only the postcode takes part in matching.
    #[serde(default)]
    pub addresses: Vec<Address>,
    /// Phone numbers and email addresses.
    #[serde(default)]
    pub telecom: Vec<Telecom>,
}
