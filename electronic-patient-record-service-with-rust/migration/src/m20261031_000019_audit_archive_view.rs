//! Migration: `audit_events_all`, the live audit table and every archived partition as one view (OS-13).
//!
//! A patient's access log and the audit search read this view, so a row that moved to `audit_archive`
//! is still listed. `audit_refresh_all_view()` rebuilds it from the tables in `audit_archive`;
//! `audit_archive_before` (replaced here) calls it in the same transaction as the move. The chain check
//! and the next append read `audit_events` and the anchors, never this view.

use sea_orm_migration::prelude::*;

/// The archive-view migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the view function, the view, and the archive function that refreshes it.
    ///
    /// # Errors
    ///
    /// When any statement fails.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let db = m.get_connection();
        for sql in [
            "CREATE FUNCTION audit_refresh_all_view() RETURNS void AS $$
             DECLARE
               q text := 'SELECT id, recorded_at, action, outcome, agent, patient_ref, resource_ref, purpose_of_use, break_glass, network_address, detail, prev_digest, digest FROM audit_events';
               r record;
             BEGIN
               FOR r IN SELECT c.relname::text AS name
                        FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
                        WHERE n.nspname = 'audit_archive' AND c.relkind = 'r'
                        ORDER BY c.relname
               LOOP
                 q := q || format(' UNION ALL SELECT id, recorded_at, action, outcome, agent, patient_ref, resource_ref, purpose_of_use, break_glass, network_address, detail, prev_digest, digest FROM audit_archive.%I', r.name);
               END LOOP;
               EXECUTE 'CREATE OR REPLACE VIEW audit_events_all AS ' || q;
             END;
             $$ LANGUAGE plpgsql",
            "CREATE OR REPLACE FUNCTION audit_archive_before(cutoff timestamptz) RETURNS text[] AS $$
             DECLARE
               r record;
               moved text[] := '{}';
               n bigint; fid bigint; lid bigint; fprev text; ldig text;
             BEGIN
               FOR r IN
                 SELECT c.relname::text AS name,
                        substring(pg_get_expr(c.relpartbound, c.oid) from 'TO \\(''([^'']+)''\\)')::timestamptz AS upper_bound
                 FROM pg_inherits i JOIN pg_class c ON c.oid = i.inhrelid
                 WHERE i.inhparent = 'audit_events'::regclass
                 ORDER BY 2
               LOOP
                 EXIT WHEN r.upper_bound > cutoff;
                 EXECUTE format('SELECT count(*), min(id), max(id) FROM %I', r.name) INTO n, fid, lid;
                 IF n > 0 THEN
                   EXECUTE format('SELECT prev_digest FROM %I WHERE id = %s', r.name, fid) INTO fprev;
                   EXECUTE format('SELECT digest FROM %I WHERE id = %s', r.name, lid) INTO ldig;
                   INSERT INTO audit_archive_anchors
                     (partition, first_id, last_id, row_count, first_prev_digest, last_digest)
                   VALUES (r.name, fid, lid, n, fprev, ldig);
                 END IF;
                 EXECUTE format('ALTER TABLE audit_events DETACH PARTITION %I', r.name);
                 EXECUTE format('ALTER TABLE %I SET SCHEMA audit_archive', r.name);
                 -- Detaching drops the row trigger the partition inherited; the archive stays append-only.
                 EXECUTE format(
                   'CREATE TRIGGER audit_events_no_change BEFORE UPDATE OR DELETE ON audit_archive.%I
                    FOR EACH ROW EXECUTE FUNCTION audit_events_immutable()', r.name);
                 moved := moved || r.name;
               END LOOP;
               PERFORM audit_refresh_all_view();
               RETURN moved;
             END;
             $$ LANGUAGE plpgsql",
            "SELECT audit_refresh_all_view()",
        ] {
            db.execute_unprepared(sql).await?;
        }
        Ok(())
    }

    /// Drop the view and the view function. The archive function keeps calling the dropped one, so this
    /// is a development convenience; production never rolls back.
    ///
    /// # Errors
    ///
    /// When any statement fails.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        crate::refuse_clinical_down("audit_archive_view")?;
        let db = m.get_connection();
        db.execute_unprepared("DROP VIEW IF EXISTS audit_events_all")
            .await?;
        Ok(())
    }
}
