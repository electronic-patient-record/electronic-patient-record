//! Migration (CQ-9): patient search by an identifier or a birth date seeks an index.
//! `patient_current` becomes an anti-join (as migration 11 did for the other `*_current` views), so a
//! filter on the record body reaches `patient_versions`, which gains a GIN index on the identifiers
//! (for `@>`) and an expression index on the birth date. Family-name search and the duplicate check's
//! family arm need a folded-name index, which is search parameter indexing (RC-2), not built here.

use sea_orm_migration::prelude::*;

/// The patient search indexes.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Replace the view and add the indexes.
    ///
    /// # Errors
    ///
    /// When any statement fails.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let db = m.get_connection();
        for sql in [
            "CREATE OR REPLACE VIEW patient_current AS
             SELECT v.patient_id, v.version, v.recorded_at, v.author, v.reason, v.body
             FROM patient_versions v
             WHERE NOT EXISTS (
                 SELECT 1 FROM patient_versions n WHERE n.patient_id = v.patient_id AND n.version > v.version
             )",
            "CREATE INDEX IF NOT EXISTS patient_versions_identifiers
             ON patient_versions USING gin ((body->'identifiers') jsonb_path_ops)",
            "CREATE INDEX IF NOT EXISTS patient_versions_birth_date
             ON patient_versions ((body->>'birth_date'))",
        ] {
            db.execute_unprepared(sql).await?;
        }
        Ok(())
    }

    /// Put back the `DISTINCT ON` view and drop the two indexes. No data is touched.
    ///
    /// # Errors
    ///
    /// When any statement fails.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let db = m.get_connection();
        for sql in [
            "CREATE OR REPLACE VIEW patient_current AS
             SELECT DISTINCT ON (patient_id) patient_id, version, recorded_at, author, reason, body
             FROM patient_versions ORDER BY patient_id, version DESC",
            "DROP INDEX IF EXISTS patient_versions_identifiers",
            "DROP INDEX IF EXISTS patient_versions_birth_date",
        ] {
            db.execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
