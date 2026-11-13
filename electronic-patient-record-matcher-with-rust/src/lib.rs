//! Explainable patient identity matching for a master patient index.
//!
//! Pure library: no IO, no clocks, no randomness. `compare` is symmetric and
//! total, and every result carries a per-field breakdown, so a reviewer can see
//! why two records were or were not linked. Missing evidence is `None`, never
//! zero, and it blocks an automatic link rather than counting as agreement.

#![deny(missing_docs)]

pub mod engine;
pub mod ihi;
pub mod national_number;
pub mod normalize;
pub mod patient;

pub use engine::{
    ConfigError, ConflictCode, Decision, FieldScore, GuardField, MatchConfig, MatchResult,
    MatchingEngine, Weights,
};
pub use ihi::{IHI_SYSTEM, is_valid_ihi};
pub use national_number::{
    NATIONAL_NUMBER_SYSTEM, is_placeholder_national_number, is_valid_national_number,
};
pub use patient::{Address, AdministrativeSex, HumanName, Identifier, Patient, Telecom};
