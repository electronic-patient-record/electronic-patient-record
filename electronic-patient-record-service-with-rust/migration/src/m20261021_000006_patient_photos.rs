//! Migration: the append-only `patient_photos` table and `patient_photo_current` view (patient
//! banner photo). Same pattern as `patient_versions`: a new photo, or the withdrawal of one, is a
//! new version, never an edit; triggers refuse `UPDATE`, `DELETE` and `TRUNCATE`.

use sea_orm_migration::prelude::*;

/// The `patient_photos`-table migration.
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
            "CREATE TABLE patient_photos (
                patient_id UUID NOT NULL,
                version INTEGER NOT NULL CHECK (version >= 1),
                recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                author TEXT NOT NULL,
                reason TEXT,
                -- NULL bytes and content type: this version withdraws the photo
                content_type TEXT CHECK (content_type IN ('image/png', 'image/jpeg')),
                bytes BYTEA CHECK (bytes IS NULL OR octet_length(bytes) BETWEEN 1 AND 524288),
                sha256 TEXT,
                PRIMARY KEY (patient_id, version),
                CHECK ((bytes IS NULL) = (content_type IS NULL) AND (bytes IS NULL) = (sha256 IS NULL))
            )",
            "CREATE VIEW patient_photo_current AS
             SELECT DISTINCT ON (patient_id) patient_id, version, recorded_at, author, reason,
                    content_type, bytes, sha256
             FROM patient_photos ORDER BY patient_id, version DESC",
            "CREATE FUNCTION patient_photos_immutable() RETURNS trigger AS $$
             BEGIN
               RAISE EXCEPTION 'patient_photos is append-only';
             END;
             $$ LANGUAGE plpgsql",
            "CREATE TRIGGER patient_photos_no_change BEFORE UPDATE OR DELETE ON patient_photos
             FOR EACH ROW EXECUTE FUNCTION patient_photos_immutable()",
            "CREATE TRIGGER patient_photos_no_truncate BEFORE TRUNCATE ON patient_photos
             FOR EACH STATEMENT EXECUTE FUNCTION patient_photos_immutable()",
        ] {
            db.execute_unprepared(sql).await?;
        }
        Ok(())
    }

    /// Drop the view and table. Only a development convenience; production never rolls back.
    ///
    /// # Errors
    ///
    /// When any statement fails.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let db = m.get_connection();
        db.execute_unprepared("DROP VIEW IF EXISTS patient_photo_current")
            .await?;
        db.execute_unprepared("DROP TABLE IF EXISTS patient_photos")
            .await?;
        db.execute_unprepared("DROP FUNCTION IF EXISTS patient_photos_immutable()")
            .await?;
        Ok(())
    }
}
