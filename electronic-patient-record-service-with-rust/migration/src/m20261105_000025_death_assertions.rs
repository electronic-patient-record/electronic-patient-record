//! Migration: the append-only `death_assertion_versions` table and its `death_assertion_current` view
//! (task BV-1). A report that a patient has died is a series of versions; none is edited or removed,
//! and triggers refuse `UPDATE`, `DELETE` and `TRUNCATE`. A withdrawal is a new version, so a death
//! recorded in error stays in the history with its reason.

use sea_orm_migration::prelude::*;

/// The death assertions migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the table, index, view and triggers.
    ///
    /// # Errors
    ///
    /// When any statement fails.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let db = m.get_connection();
        for sql in [
            "CREATE TABLE death_assertion_versions (
                assertion_id UUID NOT NULL,
                version INTEGER NOT NULL CHECK (version >= 1),
                patient_id UUID NOT NULL,
                recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                author TEXT NOT NULL,
                status TEXT NOT NULL CHECK (status IN ('reported', 'verified', 'certified', 'registered', 'withdrawn-in-error')),
                body JSONB NOT NULL,
                PRIMARY KEY (assertion_id, version)
            )",
            "CREATE INDEX death_assertion_versions_patient_idx
             ON death_assertion_versions (patient_id, recorded_at)",
            "CREATE VIEW death_assertion_current AS
             SELECT v.assertion_id, v.version, v.patient_id, v.recorded_at, v.author, v.status, v.body
             FROM death_assertion_versions v
             WHERE NOT EXISTS (
                 SELECT 1 FROM death_assertion_versions n
                 WHERE n.assertion_id = v.assertion_id AND n.version > v.version
             )",
            "CREATE FUNCTION death_assertion_versions_immutable() RETURNS trigger AS $$
             BEGIN
               RAISE EXCEPTION 'death_assertion_versions is append-only';
             END;
             $$ LANGUAGE plpgsql",
            "CREATE TRIGGER death_assertion_versions_no_change
             BEFORE UPDATE OR DELETE ON death_assertion_versions
             FOR EACH ROW EXECUTE FUNCTION death_assertion_versions_immutable()",
            "CREATE TRIGGER death_assertion_versions_no_truncate
             BEFORE TRUNCATE ON death_assertion_versions
             FOR EACH STATEMENT EXECUTE FUNCTION death_assertion_versions_immutable()",
        ] {
            db.execute_unprepared(sql).await?;
        }
        Ok(())
    }

    /// Drop everything. Only a development convenience; production never rolls back.
    ///
    /// # Errors
    ///
    /// When any statement fails.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        crate::refuse_clinical_down("death assertions")?;
        let db = m.get_connection();
        for sql in [
            "DROP VIEW IF EXISTS death_assertion_current",
            "DROP TABLE IF EXISTS death_assertion_versions",
            "DROP FUNCTION IF EXISTS death_assertion_versions_immutable()",
        ] {
            db.execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
