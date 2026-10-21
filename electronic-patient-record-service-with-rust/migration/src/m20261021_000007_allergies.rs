//! Migration: the append-only `allergy_versions` and `allergy_assertion_versions` tables and their
//! `*_current` views (the first clinical content, task RC-5 in part). A change, an end or an
//! entered-in-error mark is a new version, never an edit; triggers refuse `UPDATE`, `DELETE` and
//! `TRUNCATE`. "No known allergies" is its own recorded statement by a named author, not the
//! absence of rows.

use sea_orm_migration::prelude::*;

/// The allergies migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the tables, indexes, views and triggers.
    ///
    /// # Errors
    ///
    /// When any statement fails.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let db = m.get_connection();
        for sql in [
            "CREATE TABLE allergy_versions (
                allergy_id UUID NOT NULL,
                version INTEGER NOT NULL CHECK (version >= 1),
                patient_id UUID NOT NULL,
                recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                author TEXT NOT NULL,
                reason TEXT,
                status TEXT NOT NULL CHECK (status IN ('active', 'inactive', 'entered-in-error')),
                body JSONB NOT NULL,
                PRIMARY KEY (allergy_id, version)
            )",
            "CREATE INDEX allergy_versions_patient_idx ON allergy_versions (patient_id, recorded_at)",
            "CREATE VIEW allergy_current AS
             SELECT DISTINCT ON (allergy_id) allergy_id, version, patient_id, recorded_at, author,
                    reason, status, body
             FROM allergy_versions ORDER BY allergy_id, version DESC",
            "CREATE FUNCTION allergy_versions_immutable() RETURNS trigger AS $$
             BEGIN
               RAISE EXCEPTION 'allergy tables are append-only';
             END;
             $$ LANGUAGE plpgsql",
            "CREATE TRIGGER allergy_versions_no_change BEFORE UPDATE OR DELETE ON allergy_versions
             FOR EACH ROW EXECUTE FUNCTION allergy_versions_immutable()",
            "CREATE TRIGGER allergy_versions_no_truncate BEFORE TRUNCATE ON allergy_versions
             FOR EACH STATEMENT EXECUTE FUNCTION allergy_versions_immutable()",
            "CREATE TABLE allergy_assertion_versions (
                patient_id UUID NOT NULL,
                version INTEGER NOT NULL CHECK (version >= 1),
                recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                author TEXT NOT NULL,
                reason TEXT,
                kind TEXT NOT NULL CHECK (kind IN ('no-known-allergies', 'withdrawn')),
                PRIMARY KEY (patient_id, version)
            )",
            "CREATE VIEW allergy_assertion_current AS
             SELECT DISTINCT ON (patient_id) patient_id, version, recorded_at, author, reason, kind
             FROM allergy_assertion_versions ORDER BY patient_id, version DESC",
            "CREATE TRIGGER allergy_assertions_no_change BEFORE UPDATE OR DELETE ON allergy_assertion_versions
             FOR EACH ROW EXECUTE FUNCTION allergy_versions_immutable()",
            "CREATE TRIGGER allergy_assertions_no_truncate BEFORE TRUNCATE ON allergy_assertion_versions
             FOR EACH STATEMENT EXECUTE FUNCTION allergy_versions_immutable()",
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
        let db = m.get_connection();
        for sql in [
            "DROP VIEW IF EXISTS allergy_assertion_current",
            "DROP TABLE IF EXISTS allergy_assertion_versions",
            "DROP VIEW IF EXISTS allergy_current",
            "DROP TABLE IF EXISTS allergy_versions",
            "DROP FUNCTION IF EXISTS allergy_versions_immutable()",
        ] {
            db.execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
