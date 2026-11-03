//! Migration: the referrer's worklist (task PC-2/PC-3 groundwork, hazard H-044). A referral that moved into a
//! state the referrer should look at (declined, redirected, no-show, deferred, information requested, an interim
//! note, an outcome) stays on the referrer's worklist until the referrer acknowledges *that version*. An
//! acknowledgement is a row in `referral_acknowledgements`, never an edit: a later version of the same referral
//! needs a new one. The table is append-only: triggers refuse `UPDATE`, `DELETE` and `TRUNCATE`.

use sea_orm_migration::prelude::*;

/// The acknowledgements migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the table and triggers.
    ///
    /// # Errors
    ///
    /// When any statement fails.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let db = m.get_connection();
        for sql in [
            "CREATE TABLE referral_acknowledgements (
                referral_id UUID NOT NULL,
                version INTEGER NOT NULL,
                acknowledged_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                acknowledged_by TEXT NOT NULL,
                PRIMARY KEY (referral_id, version),
                FOREIGN KEY (referral_id, version) REFERENCES referral_versions (referral_id, version)
            )",
            "CREATE FUNCTION referral_acknowledgements_immutable() RETURNS trigger AS $$
             BEGIN
               RAISE EXCEPTION 'referral_acknowledgements is append-only';
             END;
             $$ LANGUAGE plpgsql",
            "CREATE TRIGGER referral_acknowledgements_no_change
             BEFORE UPDATE OR DELETE ON referral_acknowledgements
             FOR EACH ROW EXECUTE FUNCTION referral_acknowledgements_immutable()",
            "CREATE TRIGGER referral_acknowledgements_no_truncate
             BEFORE TRUNCATE ON referral_acknowledgements
             FOR EACH STATEMENT EXECUTE FUNCTION referral_acknowledgements_immutable()",
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
        crate::refuse_clinical_down("referral acknowledgements")?;
        let db = m.get_connection();
        for sql in [
            "DROP TABLE IF EXISTS referral_acknowledgements",
            "DROP FUNCTION IF EXISTS referral_acknowledgements_immutable()",
        ] {
            db.execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
