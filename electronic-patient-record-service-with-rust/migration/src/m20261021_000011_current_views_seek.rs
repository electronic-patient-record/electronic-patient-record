//! Migration (CQ-9): the consent, allergy, relationship and link `*_current` views as an anti-join
//! ("no later version of this record exists") instead of `DISTINCT ON` the record's id. A page asks
//! for one patient's records; under `DISTINCT ON (consent_id)` Postgres cannot push a `patient_id`
//! filter below the `DISTINCT ON`, so it built the current version of every record first. As an
//! anti-join the filter reaches the versions table and its patient index, and the "no later
//! version" probe uses the primary key. The columns, and the rows, are the same
//! (`tests/current_views.rs` compares them with the old definition).

use sea_orm_migration::prelude::*;

/// The views that seek.
#[derive(DeriveMigrationName)]
pub struct Migration;

/// (view, versions table, record key, columns)
const VIEWS: [(&str, &str, &str, &str); 4] = [
    (
        "consent_current",
        "consent_versions",
        "consent_id",
        "consent_id, version, patient_id, recorded_at, author, reason, status, body",
    ),
    (
        "allergy_current",
        "allergy_versions",
        "allergy_id",
        "allergy_id, version, patient_id, recorded_at, author, reason, status, body",
    ),
    (
        "relationship_current",
        "relationship_versions",
        "relationship_id",
        "relationship_id, version, patient_id, subject, kind, status, recorded_at, author, reason, body",
    ),
    (
        "patient_link_current",
        "patient_link_versions",
        "link_id",
        "link_id, version, recorded_at, author, reason, source_patient_id, target_patient_id, status, match_score, match_decision",
    ),
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Replace each view with the anti-join form.
    ///
    /// # Errors
    ///
    /// When any statement fails.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let db = m.get_connection();
        for (view, table, key, columns) in VIEWS {
            let qualified: Vec<String> = columns.split(", ").map(|c| format!("v.{c}")).collect();
            db.execute_unprepared(&format!(
                "CREATE OR REPLACE VIEW {view} AS
                 SELECT {} FROM {table} v
                 WHERE NOT EXISTS (
                     SELECT 1 FROM {table} n WHERE n.{key} = v.{key} AND n.version > v.version
                 )",
                qualified.join(", ")
            ))
            .await?;
        }
        Ok(())
    }

    /// Put back the `DISTINCT ON` form. Views only: no data is touched.
    ///
    /// # Errors
    ///
    /// When any statement fails.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let db = m.get_connection();
        for (view, table, key, columns) in VIEWS {
            db.execute_unprepared(&format!(
                "CREATE OR REPLACE VIEW {view} AS
                 SELECT DISTINCT ON ({key}) {columns} FROM {table} ORDER BY {key}, version DESC"
            ))
            .await?;
        }
        Ok(())
    }
}
