//! Migration: an index so the patient-visible access log (`audit::patient_log`) reads one patient's
//! rows without scanning the audit table. Audit rows record `Patient/<id>` in the case the caller
//! used, so the index is on the lower-cased value.

use sea_orm_migration::prelude::*;

/// The patient-log-index migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the index.
    ///
    /// # Errors
    ///
    /// When the statement fails.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "CREATE INDEX IF NOT EXISTS audit_events_patient_log_idx
                 ON audit_events (lower(patient_ref), id DESC)",
            )
            .await?;
        Ok(())
    }

    /// Drop the index.
    ///
    /// # Errors
    ///
    /// When the statement fails.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared("DROP INDEX IF EXISTS audit_events_patient_log_idx")
            .await?;
        Ok(())
    }
}
