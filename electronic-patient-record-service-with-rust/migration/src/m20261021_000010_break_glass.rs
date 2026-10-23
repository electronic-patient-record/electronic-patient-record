//! Migration: break-the-glass (task SE-3). `break_glass_grants` records each time a treating
//! clinician, with no care relationship, declared an emergency and gave a reason; the grant ends at
//! `expires_at`. `break_glass_reviews` holds at most one review per grant (a primary key on the grant),
//! by someone else, so every grant ends up looked at. Both tables are append-only: triggers refuse
//! `UPDATE`, `DELETE` and `TRUNCATE`. "Unreviewed" is the absence of a review row.

use sea_orm_migration::prelude::*;

/// The break-the-glass migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the tables, indexes and triggers.
    ///
    /// # Errors
    ///
    /// When any statement fails.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let db = m.get_connection();
        for sql in [
            "CREATE TABLE break_glass_grants (
                grant_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                patient_id UUID NOT NULL,
                subject TEXT NOT NULL,
                reason TEXT NOT NULL CHECK (length(btrim(reason)) > 0 AND length(reason) <= 500),
                granted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                expires_at TIMESTAMPTZ NOT NULL CHECK (expires_at > granted_at)
            )",
            "CREATE INDEX break_glass_lookup_idx ON break_glass_grants (patient_id, subject, expires_at)",
            "CREATE TABLE break_glass_reviews (
                grant_id UUID PRIMARY KEY REFERENCES break_glass_grants (grant_id),
                reviewed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                reviewed_by TEXT NOT NULL,
                outcome TEXT NOT NULL CHECK (outcome IN ('appropriate', 'inappropriate')),
                note TEXT NOT NULL CHECK (length(btrim(note)) > 0 AND length(note) <= 500)
            )",
            "CREATE FUNCTION break_glass_immutable() RETURNS trigger AS $$
             BEGIN
               RAISE EXCEPTION '% is append-only', TG_TABLE_NAME;
             END;
             $$ LANGUAGE plpgsql",
            "CREATE TRIGGER break_glass_grants_no_change BEFORE UPDATE OR DELETE ON break_glass_grants
             FOR EACH ROW EXECUTE FUNCTION break_glass_immutable()",
            "CREATE TRIGGER break_glass_grants_no_truncate BEFORE TRUNCATE ON break_glass_grants
             FOR EACH STATEMENT EXECUTE FUNCTION break_glass_immutable()",
            "CREATE TRIGGER break_glass_reviews_no_change BEFORE UPDATE OR DELETE ON break_glass_reviews
             FOR EACH ROW EXECUTE FUNCTION break_glass_immutable()",
            "CREATE TRIGGER break_glass_reviews_no_truncate BEFORE TRUNCATE ON break_glass_reviews
             FOR EACH STATEMENT EXECUTE FUNCTION break_glass_immutable()",
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
        crate::refuse_clinical_down("break_glass")?;
        let db = m.get_connection();
        for sql in [
            "DROP TABLE IF EXISTS break_glass_reviews",
            "DROP TABLE IF EXISTS break_glass_grants",
            "DROP FUNCTION IF EXISTS break_glass_immutable()",
        ] {
            db.execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
