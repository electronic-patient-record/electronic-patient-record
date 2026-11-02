//! Migration: the append-only `referral_versions` table and its `referral_current` view (task CA-3, first
//! slice). A referral is a series of versions, one per step of the state table in `src/referral.rs`
//! (requested, accepted, declined, ...); none is edited or removed, and triggers refuse `UPDATE`,
//! `DELETE` and `TRUNCATE`, so a referral cannot lose a step. `event` is what happened in the version and
//! `body` the request itself (recipient, reason, question, priority). `patient_id`, `reason`, `question`
//! and the notes in `event` are patient data.

use sea_orm_migration::prelude::*;

/// The referrals migration.
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
            "CREATE TABLE referral_versions (
                referral_id UUID NOT NULL,
                version INTEGER NOT NULL CHECK (version >= 1),
                patient_id UUID NOT NULL,
                recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                author TEXT NOT NULL,
                state TEXT NOT NULL CHECK (state IN (
                    'requested', 'accepted', 'declined', 'appointment-booked', 'no-show',
                    'interim-report', 'completed', 'cancel-requested', 'cancelled',
                    'information-requested', 'deferred', 'redirected')),
                event JSONB NOT NULL,
                body JSONB NOT NULL,
                PRIMARY KEY (referral_id, version)
            )",
            "CREATE INDEX referral_versions_patient_idx ON referral_versions (patient_id, recorded_at)",
            "CREATE VIEW referral_current AS
             SELECT v.referral_id, v.version, v.patient_id, v.recorded_at, v.author, v.state, v.event, v.body
             FROM referral_versions v
             WHERE NOT EXISTS (
                 SELECT 1 FROM referral_versions n
                 WHERE n.referral_id = v.referral_id AND n.version > v.version
             )",
            "CREATE FUNCTION referral_versions_immutable() RETURNS trigger AS $$
             BEGIN
               RAISE EXCEPTION 'referral tables are append-only';
             END;
             $$ LANGUAGE plpgsql",
            "CREATE TRIGGER referral_versions_no_change BEFORE UPDATE OR DELETE ON referral_versions
             FOR EACH ROW EXECUTE FUNCTION referral_versions_immutable()",
            "CREATE TRIGGER referral_versions_no_truncate BEFORE TRUNCATE ON referral_versions
             FOR EACH STATEMENT EXECUTE FUNCTION referral_versions_immutable()",
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
        crate::refuse_clinical_down("referrals")?;
        let db = m.get_connection();
        for sql in [
            "DROP VIEW IF EXISTS referral_current",
            "DROP TABLE IF EXISTS referral_versions",
            "DROP FUNCTION IF EXISTS referral_versions_immutable()",
        ] {
            db.execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
