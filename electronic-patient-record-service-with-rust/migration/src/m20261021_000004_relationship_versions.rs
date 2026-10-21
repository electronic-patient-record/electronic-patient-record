//! Migration: the append-only `relationship_versions` table and `relationship_current` view
//! (care-relationship lookup). Same pattern as `consent_versions`: ending a relationship is a new
//! version, never an edit; triggers refuse `UPDATE`, `DELETE` and `TRUNCATE`.

use sea_orm_migration::prelude::*;

/// The `relationship_versions`-table migration.
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
            "CREATE TABLE relationship_versions (
                relationship_id UUID NOT NULL,
                version INTEGER NOT NULL CHECK (version >= 1),
                patient_id UUID NOT NULL,
                subject TEXT NOT NULL CHECK (subject <> ''),
                kind TEXT NOT NULL CHECK (kind IN ('care-team', 'active-encounter', 'self', 'proxy')),
                status TEXT NOT NULL CHECK (status IN ('active', 'ended')),
                recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                author TEXT NOT NULL,
                reason TEXT,
                body JSONB NOT NULL,
                PRIMARY KEY (relationship_id, version)
            )",
            "CREATE INDEX relationship_versions_lookup ON relationship_versions (patient_id, subject)",
            "CREATE VIEW relationship_current AS
             SELECT DISTINCT ON (relationship_id) relationship_id, version, patient_id, subject, kind,
                    status, recorded_at, author, reason, body
             FROM relationship_versions ORDER BY relationship_id, version DESC",
            "CREATE FUNCTION relationship_versions_immutable() RETURNS trigger AS $$
             BEGIN
               RAISE EXCEPTION 'relationship_versions is append-only';
             END;
             $$ LANGUAGE plpgsql",
            "CREATE TRIGGER relationship_versions_no_change BEFORE UPDATE OR DELETE ON relationship_versions
             FOR EACH ROW EXECUTE FUNCTION relationship_versions_immutable()",
            "CREATE TRIGGER relationship_versions_no_truncate BEFORE TRUNCATE ON relationship_versions
             FOR EACH STATEMENT EXECUTE FUNCTION relationship_versions_immutable()",
        ] {
            db.execute_unprepared(sql).await?;
        }
        Ok(())
    }

    /// Drop the view and table. Only a development convenience.
    ///
    /// # Errors
    ///
    /// When any statement fails.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let db = m.get_connection();
        db.execute_unprepared("DROP VIEW IF EXISTS relationship_current")
            .await?;
        db.execute_unprepared("DROP TABLE IF EXISTS relationship_versions")
            .await?;
        db.execute_unprepared("DROP FUNCTION IF EXISTS relationship_versions_immutable()")
            .await?;
        Ok(())
    }
}
