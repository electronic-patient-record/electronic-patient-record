//! Migration (EC-10): a patient version may be a drill or test record. `patient_versions.drill` is false for
//! every existing row and every new row unless the registration says otherwise, and each version of a patient
//! carries the flag of the one before. `patient_current` gains the column so search and the duplicate check
//! can keep drill and real records apart. The table stays append-only; no row is touched.

use sea_orm_migration::prelude::*;

/// The drill flag.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Add the column and replace the view.
    ///
    /// # Errors
    ///
    /// When any statement fails.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let db = m.get_connection();
        for sql in [
            "ALTER TABLE patient_versions ADD COLUMN drill BOOLEAN NOT NULL DEFAULT false",
            "CREATE OR REPLACE VIEW patient_current AS
             SELECT v.patient_id, v.version, v.recorded_at, v.author, v.reason, v.body, v.drill
             FROM patient_versions v
             WHERE NOT EXISTS (
                 SELECT 1 FROM patient_versions n WHERE n.patient_id = v.patient_id AND n.version > v.version
             )",
        ] {
            db.execute_unprepared(sql).await?;
        }
        Ok(())
    }

    /// Put back the view without the column and drop it. This forgets which records were drills, so it is
    /// refused outside development and test like the other clinical downs.
    ///
    /// # Errors
    ///
    /// When the environment forbids it, or a statement fails.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        crate::refuse_clinical_down("patient_versions.drill")?;
        let db = m.get_connection();
        for sql in [
            "DROP VIEW patient_current",
            "ALTER TABLE patient_versions DROP COLUMN drill",
            "CREATE VIEW patient_current AS
             SELECT v.patient_id, v.version, v.recorded_at, v.author, v.reason, v.body
             FROM patient_versions v
             WHERE NOT EXISTS (
                 SELECT 1 FROM patient_versions n WHERE n.patient_id = v.patient_id AND n.version > v.version
             )",
        ] {
            db.execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
