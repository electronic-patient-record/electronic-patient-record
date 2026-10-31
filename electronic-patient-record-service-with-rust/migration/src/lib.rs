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
mod m20261021_000010_break_glass;
mod m20261021_000011_current_views_seek;
mod m20261021_000012_patient_search_indexes;
mod m20261024_000013_patient_drill;
mod m20261026_000014_observations;
mod m20261027_000015_amendments;
mod m20261028_000016_hazard_events;
mod m20261030_000017_outbox;
mod m20261031_000018_audit_partitions;

/// Whether a `down` that drops clinical tables may run: only in development or test, and only when
/// `EPR_ALLOW_DESTRUCTIVE_DOWN=true` says so. A missing environment name counts as production.
fn down_allowed(env: Option<&str>, flag: Option<&str>) -> bool {
    matches!(env, Some("development" | "test")) && flag == Some("true")
}

/// Refuse to drop clinical tables (patient, consent, relationship, allergy, photo, link, feed,
/// break-glass, audit) unless [`down_allowed`]. The clinical record is never deleted; a `down` is a
/// development convenience, and in any other environment it fails before it changes anything.
pub(crate) fn refuse_clinical_down(migration: &str) -> Result<(), DbErr> {
    let env = std::env::var("LOCO_ENV").ok();
    let flag = std::env::var("EPR_ALLOW_DESTRUCTIVE_DOWN").ok();
    if down_allowed(env.as_deref(), flag.as_deref()) {
        Ok(())
    } else {
        Err(DbErr::Migration(format!(
            "{migration}: down would drop clinical tables, which is allowed only with LOCO_ENV=development or test and EPR_ALLOW_DESTRUCTIVE_DOWN=true"
        )))
    }
}

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
            Box::new(m20261021_000010_break_glass::Migration),
            Box::new(m20261021_000011_current_views_seek::Migration),
            Box::new(m20261021_000012_patient_search_indexes::Migration),
            Box::new(m20261024_000013_patient_drill::Migration),
            Box::new(m20261026_000014_observations::Migration),
            Box::new(m20261027_000015_amendments::Migration),
            Box::new(m20261028_000016_hazard_events::Migration),
            Box::new(m20261030_000017_outbox::Migration),
            Box::new(m20261031_000018_audit_partitions::Migration),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::down_allowed;

    #[test]
    fn down_needs_a_development_or_test_environment_and_the_flag() {
        assert!(down_allowed(Some("development"), Some("true")));
        assert!(down_allowed(Some("test"), Some("true")));
        assert!(!down_allowed(Some("production"), Some("true")));
        assert!(!down_allowed(None, Some("true")));
        assert!(!down_allowed(Some("development"), None));
        assert!(!down_allowed(Some("development"), Some("1")));
    }
}
