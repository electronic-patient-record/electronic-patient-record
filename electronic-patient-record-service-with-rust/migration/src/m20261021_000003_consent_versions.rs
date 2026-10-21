//! Migration: the append-only `consent_versions` table and the `consent_current` view (task SE-2).
//!
//! A consent is never edited or removed: withdrawing it, correcting it or marking it entered in
//! error is a new version with its author, time and reason. Triggers refuse `UPDATE`, `DELETE` and
//! `TRUNCATE`. `patient_id` is carried on every version so a patient's consents can be found in
//! one indexed read; it is not a foreign key because `patient_versions` has no unique patient row.

use sea_orm_migration::prelude::*;

/// The `consent_versions`-table migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the table, the index, the view and the immutability triggers.
    ///
    /// # Errors
    ///
    /// When any statement fails.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let db = m.get_connection();
        db.execute_unprepared(
            "CREATE TABLE consent_versions (
                consent_id UUID NOT NULL,
                version INTEGER NOT NULL CHECK (version >= 1),
                patient_id UUID NOT NULL,
                recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                author TEXT NOT NULL,
                reason TEXT,
                status TEXT NOT NULL CHECK (status IN ('active', 'inactive', 'entered-in-error')),
                body JSONB NOT NULL,
                PRIMARY KEY (consent_id, version)
            )",
        )
        .await?;
        db.execute_unprepared(
            "CREATE INDEX consent_versions_patient ON consent_versions (patient_id)",
        )
        .await?;
        db.execute_unprepared(
            "CREATE VIEW consent_current AS
             SELECT DISTINCT ON (consent_id) consent_id, version, patient_id, recorded_at, author,
                    reason, status, body
             FROM consent_versions ORDER BY consent_id, version DESC",
        )
        .await?;
        db.execute_unprepared(
            "CREATE FUNCTION consent_versions_immutable() RETURNS trigger AS $$
             BEGIN
               RAISE EXCEPTION 'consent_versions is append-only';
             END;
             $$ LANGUAGE plpgsql",
        )
        .await?;
        db.execute_unprepared(
            "CREATE TRIGGER consent_versions_no_change BEFORE UPDATE OR DELETE ON consent_versions
             FOR EACH ROW EXECUTE FUNCTION consent_versions_immutable()",
        )
        .await?;
        db.execute_unprepared(
            "CREATE TRIGGER consent_versions_no_truncate BEFORE TRUNCATE ON consent_versions
             FOR EACH STATEMENT EXECUTE FUNCTION consent_versions_immutable()",
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
        let db = m.get_connection();
        db.execute_unprepared("DROP VIEW IF EXISTS consent_current")
            .await?;
        db.execute_unprepared("DROP TABLE IF EXISTS consent_versions")
            .await?;
        db.execute_unprepared("DROP FUNCTION IF EXISTS consent_versions_immutable()")
            .await?;
        Ok(())
    }
}
