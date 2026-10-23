//! Migration: the append-only `patient_versions` table and the `patient_current` view (plan ID-3).
//!
//! A patient record is never updated in place: every change is a new version row with its
//! author, time and reason. Triggers refuse `UPDATE`, `DELETE` and `TRUNCATE`. There is no
//! delete path; a wrongly created record is withdrawn by a later version, not removed.

use sea_orm_migration::prelude::*;

/// The `patient_versions`-table migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the table, the view and the immutability triggers.
    ///
    /// # Errors
    ///
    /// When any statement fails.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let db = m.get_connection();
        db.execute_unprepared(
            "CREATE TABLE patient_versions (
                patient_id UUID NOT NULL,
                version INTEGER NOT NULL CHECK (version >= 1),
                recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                author TEXT NOT NULL,
                reason TEXT,
                body JSONB NOT NULL,
                PRIMARY KEY (patient_id, version)
            )",
        )
        .await?;
        db.execute_unprepared(
            "CREATE VIEW patient_current AS
             SELECT DISTINCT ON (patient_id) patient_id, version, recorded_at, author, reason, body
             FROM patient_versions ORDER BY patient_id, version DESC",
        )
        .await?;
        db.execute_unprepared(
            "CREATE FUNCTION patient_versions_immutable() RETURNS trigger AS $$
             BEGIN
               RAISE EXCEPTION 'patient_versions is append-only';
             END;
             $$ LANGUAGE plpgsql",
        )
        .await?;
        db.execute_unprepared(
            "CREATE TRIGGER patient_versions_no_change BEFORE UPDATE OR DELETE ON patient_versions
             FOR EACH ROW EXECUTE FUNCTION patient_versions_immutable()",
        )
        .await?;
        db.execute_unprepared(
            "CREATE TRIGGER patient_versions_no_truncate BEFORE TRUNCATE ON patient_versions
             FOR EACH STATEMENT EXECUTE FUNCTION patient_versions_immutable()",
        )
        .await?;
        Ok(())
    }

    /// Drop the view and table. Only a development convenience; production never rolls back.
    ///
    /// # Errors
    ///
    /// When any statement fails.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        crate::refuse_clinical_down("patient_versions")?;
        let db = m.get_connection();
        db.execute_unprepared("DROP VIEW IF EXISTS patient_current")
            .await?;
        db.execute_unprepared("DROP TABLE IF EXISTS patient_versions")
            .await?;
        db.execute_unprepared("DROP FUNCTION IF EXISTS patient_versions_immutable()")
            .await?;
        Ok(())
    }
}
