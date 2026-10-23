//! Migration: the append-only `audit_events` table (plan AU-1).
//!
//! One row per access (read or write). Rows are hash-chained: `prev_digest` is
//! the previous row's `digest`, so a missing or altered row breaks the chain.
//! Triggers refuse `UPDATE`, `DELETE` and `TRUNCATE`.

use sea_orm_migration::prelude::*;

/// The `audit_events`-table migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the table and its immutability triggers.
    ///
    /// # Errors
    ///
    /// When any statement fails.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let db = m.get_connection();
        db.execute_unprepared(
            "CREATE TABLE audit_events (
                id BIGSERIAL PRIMARY KEY,
                recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                -- FHIR AuditEvent.action: C, R, U, D, E
                action CHAR(1) NOT NULL CHECK (action IN ('C','R','U','D','E')),
                -- 0 success, 4 minor failure, 8 serious failure, 12 major failure
                outcome SMALLINT NOT NULL DEFAULT 0 CHECK (outcome IN (0,4,8,12)),
                agent TEXT NOT NULL,
                patient_ref TEXT,
                resource_ref TEXT,
                purpose_of_use TEXT,
                break_glass BOOLEAN NOT NULL DEFAULT FALSE,
                network_address TEXT,
                detail JSONB,
                prev_digest TEXT,
                digest TEXT NOT NULL
            )",
        )
        .await?;
        db.execute_unprepared(
            "CREATE INDEX audit_events_patient_idx ON audit_events (patient_ref, recorded_at)",
        )
        .await?;
        db.execute_unprepared(
            "CREATE FUNCTION audit_events_immutable() RETURNS trigger AS $$
             BEGIN
               RAISE EXCEPTION 'audit_events is append-only';
             END;
             $$ LANGUAGE plpgsql",
        )
        .await?;
        db.execute_unprepared(
            "CREATE TRIGGER audit_events_no_change BEFORE UPDATE OR DELETE ON audit_events
             FOR EACH ROW EXECUTE FUNCTION audit_events_immutable()",
        )
        .await?;
        db.execute_unprepared(
            "CREATE TRIGGER audit_events_no_truncate BEFORE TRUNCATE ON audit_events
             FOR EACH STATEMENT EXECUTE FUNCTION audit_events_immutable()",
        )
        .await?;
        Ok(())
    }

    /// Drop the table. Only a development convenience; production never rolls back.
    ///
    /// # Errors
    ///
    /// When any statement fails.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        crate::refuse_clinical_down("audit_events")?;
        let db = m.get_connection();
        db.execute_unprepared("DROP TABLE IF EXISTS audit_events")
            .await?;
        db.execute_unprepared("DROP FUNCTION IF EXISTS audit_events_immutable()")
            .await?;
        Ok(())
    }
}
