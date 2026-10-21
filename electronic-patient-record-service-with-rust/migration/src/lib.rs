//! `sea-orm-migration` schema migrations for `electronic-patient-record`.
//!
//! The migrations create the append-only `audit_events`, `patient_versions` and `consent_versions` tables. The
//! clinical record is never deleted, and neither is its audit trail: database
//! triggers refuse `UPDATE` and `DELETE`.

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![allow(elided_lifetimes_in_paths)]
pub use sea_orm_migration::prelude::*;
mod m20261020_000001_audit_events;
mod m20261021_000002_patient_versions;
mod m20261021_000003_consent_versions;
mod m20261021_000004_relationship_versions;
mod m20261021_000005_audit_patient_log_index;
mod m20261021_000006_patient_photos;
mod m20261021_000007_allergies;
mod m20261021_000008_patient_links;
mod m20261021_000009_feed_refusals;

/// Runs the ordered list of migrations.
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20261020_000001_audit_events::Migration),
            Box::new(m20261021_000002_patient_versions::Migration),
            Box::new(m20261021_000003_consent_versions::Migration),
            Box::new(m20261021_000004_relationship_versions::Migration),
            Box::new(m20261021_000005_audit_patient_log_index::Migration),
            Box::new(m20261021_000006_patient_photos::Migration),
            Box::new(m20261021_000007_allergies::Migration),
            Box::new(m20261021_000008_patient_links::Migration),
            Box::new(m20261021_000009_feed_refusals::Migration),
        ]
    }
}
