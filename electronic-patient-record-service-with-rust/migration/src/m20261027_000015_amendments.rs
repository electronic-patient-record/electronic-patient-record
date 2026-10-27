//! Migration: the append-only `amendment_versions` table and its `amendment_current` view (task RC-4,
//! store slice). A request to amend a record is a series of versions (opened, decided, applied, a
//! statement of disagreement added); none is edited or removed, and triggers refuse `UPDATE`,
//! `DELETE` and `TRUNCATE`. `event` is what happened in this version and `body` the whole request
//! after it.

use sea_orm_migration::prelude::*;

/// The amendments migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the table, indexes, view and triggers.
    ///
    /// # Errors
    ///
    /// When any statement fails.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let db = m.get_connection();
        for sql in [
            "CREATE TABLE amendment_versions (
                amendment_id UUID NOT NULL,
                version INTEGER NOT NULL CHECK (version >= 1),
                patient_id UUID NOT NULL,
                recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                author TEXT NOT NULL,
                state TEXT NOT NULL CHECK (state IN ('requested', 'accepted', 'applied', 'denied', 'withdrawn')),
                target_kind TEXT NOT NULL,
                target_id UUID NOT NULL,
                event JSONB NOT NULL,
                body JSONB NOT NULL,
                PRIMARY KEY (amendment_id, version)
            )",
            "CREATE INDEX amendment_versions_patient_idx ON amendment_versions (patient_id, recorded_at)",
            "CREATE INDEX amendment_versions_target_idx ON amendment_versions (target_kind, target_id)",
            "CREATE VIEW amendment_current AS
             SELECT v.amendment_id, v.version, v.patient_id, v.recorded_at, v.author, v.state,
                    v.target_kind, v.target_id, v.event, v.body
             FROM amendment_versions v
             WHERE NOT EXISTS (
                 SELECT 1 FROM amendment_versions n
                 WHERE n.amendment_id = v.amendment_id AND n.version > v.version
             )",
            "CREATE FUNCTION amendment_versions_immutable() RETURNS trigger AS $$
             BEGIN
               RAISE EXCEPTION 'amendment tables are append-only';
             END;
             $$ LANGUAGE plpgsql",
            "CREATE TRIGGER amendment_versions_no_change BEFORE UPDATE OR DELETE ON amendment_versions
             FOR EACH ROW EXECUTE FUNCTION amendment_versions_immutable()",
            "CREATE TRIGGER amendment_versions_no_truncate BEFORE TRUNCATE ON amendment_versions
             FOR EACH STATEMENT EXECUTE FUNCTION amendment_versions_immutable()",
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
        crate::refuse_clinical_down("amendments")?;
        let db = m.get_connection();
        for sql in [
            "DROP VIEW IF EXISTS amendment_current",
            "DROP TABLE IF EXISTS amendment_versions",
            "DROP FUNCTION IF EXISTS amendment_versions_immutable()",
        ] {
            db.execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
