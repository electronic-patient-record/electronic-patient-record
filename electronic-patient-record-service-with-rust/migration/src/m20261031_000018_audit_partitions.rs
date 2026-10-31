//! Migration: `audit_events` becomes a table partitioned by month on `recorded_at` (task OS-13).
//!
//! The existing table is not copied or dropped: it is renamed `audit_events_p0`, its primary key gains
//! `recorded_at` (a partition key must be in every unique key), and it is attached as the first
//! partition under a new parent that takes the name `audit_events`. Rows keep their ids (the sequence is
//! shared) and the hash chain is untouched. The append-only triggers are dropped from the old table and
//! created on the parent, which gives every partition the row trigger; each partition gets its own
//! `TRUNCATE` trigger (a statement trigger is not inherited). It all runs in this migration's
//! transaction, so no session sees the table unprotected.
//!
//! `audit_ensure_partitions(n)` makes the partitions for this month and the next `n`; the service calls it
//! when it starts and every few hours. `audit_archive_before(cutoff)` moves the oldest partitions that end
//! at or before `cutoff` into the `audit_archive` schema: it records an anchor first (the first row's
//! `prev_digest`, the last row's `digest`, the row count) and then detaches and moves. Nothing is deleted;
//! the anchors are append-only, the moved table gets its row trigger back (detaching drops the inherited
//! one), and the chain check starts from the newest anchor.

use sea_orm_migration::prelude::*;

/// The partitioning migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Partition the table, add the anchors, the archive schema and the two functions.
    ///
    /// # Errors
    ///
    /// When any statement fails; the migration's transaction then leaves the table as it was.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let db = m.get_connection();
        for sql in [
            "CREATE SCHEMA IF NOT EXISTS audit_archive",
            "CREATE TABLE audit_archive_anchors (
                partition TEXT PRIMARY KEY,
                first_id BIGINT NOT NULL,
                last_id BIGINT NOT NULL,
                row_count BIGINT NOT NULL,
                first_prev_digest TEXT,
                last_digest TEXT NOT NULL,
                archived_at TIMESTAMPTZ NOT NULL DEFAULT now()
            )",
            "CREATE FUNCTION audit_anchors_immutable() RETURNS trigger AS $$
             BEGIN
               RAISE EXCEPTION 'audit_archive_anchors is append-only';
             END;
             $$ LANGUAGE plpgsql",
            "CREATE TRIGGER audit_anchors_no_change BEFORE UPDATE OR DELETE ON audit_archive_anchors
             FOR EACH ROW EXECUTE FUNCTION audit_anchors_immutable()",
            "CREATE TRIGGER audit_anchors_no_truncate BEFORE TRUNCATE ON audit_archive_anchors
             FOR EACH STATEMENT EXECUTE FUNCTION audit_anchors_immutable()",
            // The old table: out of the way, key widened, triggers and index names released.
            "ALTER TABLE audit_events RENAME TO audit_events_p0",
            "DROP TRIGGER audit_events_no_change ON audit_events_p0",
            "DROP TRIGGER audit_events_no_truncate ON audit_events_p0",
            "ALTER TABLE audit_events_p0 DROP CONSTRAINT audit_events_pkey",
            "ALTER TABLE audit_events_p0 ADD PRIMARY KEY (id, recorded_at)",
            "CREATE TABLE audit_events (LIKE audit_events_p0 INCLUDING DEFAULTS INCLUDING CONSTRAINTS)
             PARTITION BY RANGE (recorded_at)",
            "ALTER TABLE audit_events ADD PRIMARY KEY (id, recorded_at)",
            // Every other index moves its name to the parent; the old table's equal index is adopted
            // on attach, under a new name.
            "DO $$
             DECLARE r record;
             BEGIN
               FOR r IN SELECT indexname, indexdef FROM pg_indexes
                        WHERE schemaname = current_schema() AND tablename = 'audit_events_p0'
                          AND indexname <> 'audit_events_p0_pkey'
               LOOP
                 EXECUTE format('ALTER INDEX %I RENAME TO %I', r.indexname, r.indexname || '_p0');
                 EXECUTE replace(r.indexdef, 'audit_events_p0 ', 'audit_events ');
               END LOOP;
             END $$",
            "CREATE TRIGGER audit_events_no_change BEFORE UPDATE OR DELETE ON audit_events
             FOR EACH ROW EXECUTE FUNCTION audit_events_immutable()",
            "CREATE TRIGGER audit_events_no_truncate BEFORE TRUNCATE ON audit_events
             FOR EACH STATEMENT EXECUTE FUNCTION audit_events_immutable()",
            "DO $$
             DECLARE bound timestamptz :=
               (date_trunc('month', now() AT TIME ZONE 'UTC') + interval '1 month') AT TIME ZONE 'UTC';
             BEGIN
               EXECUTE format(
                 'ALTER TABLE audit_events ATTACH PARTITION audit_events_p0 FOR VALUES FROM (MINVALUE) TO (%L)',
                 bound);
             END $$",
            "CREATE TRIGGER audit_events_no_truncate BEFORE TRUNCATE ON audit_events_p0
             FOR EACH STATEMENT EXECUTE FUNCTION audit_events_immutable()",
            "CREATE FUNCTION audit_ensure_partitions(months_ahead integer) RETURNS integer AS $$
             DECLARE
               month_start timestamp;
               m timestamptz;
               part text;
               made integer := 0;
             BEGIN
               FOR i IN 0..months_ahead LOOP
                 month_start := date_trunc('month', now() AT TIME ZONE 'UTC') + make_interval(months => i);
                 m := month_start AT TIME ZONE 'UTC';
                 part := 'audit_events_p' || to_char(month_start, 'YYYYMM');
                 IF to_regclass(part) IS NULL AND to_regclass('audit_archive.' || part) IS NULL THEN
                   BEGIN
                     EXECUTE format(
                       'CREATE TABLE %I PARTITION OF audit_events FOR VALUES FROM (%L) TO (%L)',
                       part, m, (month_start + interval '1 month') AT TIME ZONE 'UTC');
                     EXECUTE format(
                       'CREATE TRIGGER audit_events_no_truncate BEFORE TRUNCATE ON %I
                        FOR EACH STATEMENT EXECUTE FUNCTION audit_events_immutable()', part);
                     made := made + 1;
                   EXCEPTION WHEN invalid_object_definition THEN
                     NULL; -- this month lies inside the first partition
                   END;
                 END IF;
               END LOOP;
               RETURN made;
             END;
             $$ LANGUAGE plpgsql",
            "CREATE FUNCTION audit_archive_before(cutoff timestamptz) RETURNS text[] AS $$
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
               RETURN moved;
             END;
             $$ LANGUAGE plpgsql",
            "SELECT audit_ensure_partitions(24)",
        ] {
            db.execute_unprepared(sql).await?;
        }
        Ok(())
    }

    /// Partitioning is not undone: that would rewrite the audit table, so this always refuses.
    ///
    /// # Errors
    ///
    /// Always.
    async fn down(&self, _m: &SchemaManager) -> Result<(), DbErr> {
        crate::refuse_clinical_down("audit_partitions")?;
        Err(DbErr::Migration(
            "audit_partitions: not reversible (it would move the audit table)".into(),
        ))
    }
}
