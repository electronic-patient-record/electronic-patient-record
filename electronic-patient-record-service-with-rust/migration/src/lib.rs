//! `sea-orm-migration` schema migrations for `electronic-patient-record`.
//!
//! The migrations create the append-only `audit_events` and `patient_versions` tables. The
//! clinical record is never deleted, and neither is its audit trail: database
//! triggers refuse `UPDATE` and `DELETE`.

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![allow(elided_lifetimes_in_paths)]
pub use sea_orm_migration::prelude::*;
mod m20261020_000001_audit_events;
mod m20261021_000002_patient_versions;

/// Runs the ordered list of migrations.
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20261020_000001_audit_events::Migration),
            Box::new(m20261021_000002_patient_versions::Migration),
        ]
    }
}
