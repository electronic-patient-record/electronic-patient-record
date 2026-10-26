//! Migration: the append-only `observation_versions` table and its `observation_current` view (task
//! RC-6, first store slice). A correction or an entered-in-error mark is a new version, never an edit;
//! triggers refuse `UPDATE`, `DELETE` and `TRUNCATE`. `effective_at` is the instant the value was
//! measured (UTC); `site_zone` is the site's IANA zone when it was recorded, so a local time of day
//! stays meaningful after a clock change.

use sea_orm_migration::prelude::*;

/// The observations migration.
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
            "CREATE TABLE observation_versions (
                observation_id UUID NOT NULL,
                version INTEGER NOT NULL CHECK (version >= 1),
                patient_id UUID NOT NULL,
                recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                author TEXT NOT NULL,
                reason TEXT,
                status TEXT NOT NULL CHECK (status IN ('final', 'entered-in-error')),
                effective_at TIMESTAMPTZ NOT NULL,
                site_zone TEXT NOT NULL,
                body JSONB NOT NULL,
                PRIMARY KEY (observation_id, version)
            )",
            "CREATE INDEX observation_versions_patient_idx ON observation_versions (patient_id, effective_at)",
            // The anti-join form (CQ-9): a filter on the patient reaches the versions table and its index.
            "CREATE VIEW observation_current AS
             SELECT v.observation_id, v.version, v.patient_id, v.recorded_at, v.author, v.reason, v.status,
                    v.effective_at, v.site_zone, v.body
             FROM observation_versions v
             WHERE NOT EXISTS (
                 SELECT 1 FROM observation_versions n
                 WHERE n.observation_id = v.observation_id AND n.version > v.version
             )",
            "CREATE FUNCTION observation_versions_immutable() RETURNS trigger AS $$
             BEGIN
               RAISE EXCEPTION 'observation tables are append-only';
             END;
             $$ LANGUAGE plpgsql",
            "CREATE TRIGGER observation_versions_no_change BEFORE UPDATE OR DELETE ON observation_versions
             FOR EACH ROW EXECUTE FUNCTION observation_versions_immutable()",
            "CREATE TRIGGER observation_versions_no_truncate BEFORE TRUNCATE ON observation_versions
             FOR EACH STATEMENT EXECUTE FUNCTION observation_versions_immutable()",
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
        crate::refuse_clinical_down("observations")?;
        let db = m.get_connection();
        for sql in [
            "DROP VIEW IF EXISTS observation_current",
            "DROP TABLE IF EXISTS observation_versions",
            "DROP FUNCTION IF EXISTS observation_versions_immutable()",
        ] {
            db.execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
