//! Migration: the append-only `hazard_versions` table and its `hazard_current` view (task AS-17). A
//! hazard is a series of versions (raised, a control added, retired, reopened); none is edited or
//! removed, and triggers refuse `UPDATE`, `DELETE` and `TRUNCATE`, so retiring a hazard cannot erase
//! its history. `event` is what happened in this version and `body` the whole hazard after it. The
//! table holds no patient data.

use sea_orm_migration::prelude::*;

/// The hazard register migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the table, view and triggers.
    ///
    /// # Errors
    ///
    /// When any statement fails.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let db = m.get_connection();
        for sql in [
            "CREATE TABLE hazard_versions (
                hazard_id UUID NOT NULL,
                version INTEGER NOT NULL CHECK (version >= 1),
                recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                author TEXT NOT NULL,
                state TEXT NOT NULL CHECK (state IN ('open', 'retired')),
                event JSONB NOT NULL,
                body JSONB NOT NULL,
                PRIMARY KEY (hazard_id, version)
            )",
            "CREATE INDEX hazard_versions_recorded_idx ON hazard_versions (recorded_at)",
            "CREATE VIEW hazard_current AS
             SELECT v.hazard_id, v.version, v.recorded_at, v.author, v.state, v.event, v.body
             FROM hazard_versions v
             WHERE NOT EXISTS (
                 SELECT 1 FROM hazard_versions n
                 WHERE n.hazard_id = v.hazard_id AND n.version > v.version
             )",
            "CREATE FUNCTION hazard_versions_immutable() RETURNS trigger AS $$
             BEGIN
               RAISE EXCEPTION 'hazard tables are append-only';
             END;
             $$ LANGUAGE plpgsql",
            "CREATE TRIGGER hazard_versions_no_change BEFORE UPDATE OR DELETE ON hazard_versions
             FOR EACH ROW EXECUTE FUNCTION hazard_versions_immutable()",
            "CREATE TRIGGER hazard_versions_no_truncate BEFORE TRUNCATE ON hazard_versions
             FOR EACH STATEMENT EXECUTE FUNCTION hazard_versions_immutable()",
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
        crate::refuse_clinical_down("hazard_events")?;
        let db = m.get_connection();
        for sql in [
            "DROP VIEW IF EXISTS hazard_current",
            "DROP TABLE IF EXISTS hazard_versions",
            "DROP FUNCTION IF EXISTS hazard_versions_immutable()",
        ] {
            db.execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
