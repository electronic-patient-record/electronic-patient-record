//! DB-gated (task OS-13): `audit_events` is partitioned by month; the chain holds across a partition
//! boundary and across the archive; archiving moves rows and keeps an anchor, and deletes nothing.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DATABASE_URL=postgres://loco:loco@127.0.0.1:5543/electronic_patient_record_test \
//!   cargo test --test audit_partitions -- --ignored --test-threads=1
//! ```

use electronic_patient_record::audit::{self, Action, Chain, Event, Verdict};
use electronic_patient_record::audit_retention;
use electronic_patient_record_migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, Statement};

/// A scratch database, migrated to `upto` migrations (all when `None`), dropped when the test ends.
struct Scratch {
    db: DatabaseConnection,
    admin: DatabaseConnection,
    name: String,
}

impl Scratch {
    async fn new(upto: Option<u32>) -> Self {
        let url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for --ignored tests");
        let (base, _) = url.rsplit_once('/').expect("a database URL");
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let name = format!("epr_scratch_{}_{}", std::process::id(), nanos);
        let admin = Database::connect(url.clone()).await.expect("connect admin");
        admin
            .execute_unprepared(&format!("CREATE DATABASE {name}"))
            .await
            .expect("create scratch db");
        let db = Database::connect(format!("{base}/{name}"))
            .await
            .expect("connect scratch");
        Migrator::up(&db, upto).await.expect("migrate");
        Self { db, admin, name }
    }

    async fn n(&self, sql: &str) -> i64 {
        self.db
            .query_one_raw(Statement::from_string(
                sea_orm::DbBackend::Postgres,
                sql.to_string(),
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get_by_index(0)
            .unwrap()
    }

    async fn text(&self, sql: &str) -> Option<String> {
        self.db
            .query_one_raw(Statement::from_string(
                sea_orm::DbBackend::Postgres,
                sql.to_string(),
            ))
            .await
            .unwrap()
            .and_then(|r| r.try_get_by_index(0).ok())
    }

    async fn exec(&self, sql: &str) {
        self.db.execute_unprepared(sql).await.unwrap();
    }

    async fn refused(&self, sql: &str) -> bool {
        self.db
            .execute_unprepared(sql)
            .await
            .is_err_and(|e| e.to_string().contains("append-only"))
    }

    async fn finish(self) {
        let Scratch { db, admin, name } = self;
        drop(db);
        let _ = admin
            .execute_unprepared(&format!("DROP DATABASE IF EXISTS {name} WITH (FORCE)"))
            .await;
    }
}

fn event(n: usize) -> Event {
    let mut e = Event::new(Action::Read, format!("user-{n}"));
    e.patient_ref = Some(format!("Patient/{n}"));
    e
}

/// Append a row to the chain with `recorded_at` set by the SQL expression `at` (a timestamp), without
/// `audit::append`: that needs the newest schema, and a test may stand before or beyond a boundary.
async fn append_at(s: &Scratch, chain: &Chain, n: usize, at: &str) -> i64 {
    let prev = s
        .text("SELECT digest FROM audit_events ORDER BY id DESC LIMIT 1")
        .await;
    let ts = s
        .text(&format!(
            "SELECT to_char(({at}) AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"')"
        ))
        .await
        .unwrap();
    let e = event(n);
    let digest = chain.digest(prev.as_deref(), &ts, &e);
    s.n(&format!(
        "INSERT INTO audit_events (recorded_at, action, outcome, agent, patient_ref, break_glass,
                                  prev_digest, digest)
         VALUES ('{ts}'::timestamptz, 'R', 0, '{}', '{}', false, {}, '{digest}') RETURNING id",
        e.agent,
        e.patient_ref.as_deref().unwrap(),
        prev.map_or("NULL".to_string(), |p| format!("'{p}'")),
    ))
    .await
}

/// A row in the month after this one.
async fn append_next_month(s: &Scratch, chain: &Chain, n: usize) -> i64 {
    append_at(
        s,
        chain,
        n,
        &format!(
            "date_trunc('month', now() AT TIME ZONE 'UTC') + interval '1 month' + make_interval(days => 1, mins => {n})"
        ),
    )
    .await
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn the_table_is_partitioned_has_months_ahead_and_stays_append_only() {
    let s = Scratch::new(None).await;
    assert_eq!(
        s.text("SELECT relkind::text FROM pg_class WHERE oid = 'audit_events'::regclass")
            .await
            .as_deref(),
        Some("p")
    );
    assert!(
        s.n("SELECT count(*) FROM pg_inherits WHERE inhparent = 'audit_events'::regclass")
            .await
            >= 24
    );
    assert_eq!(
        audit_retention::ensure_partitions(&s.db, 24).await.unwrap(),
        0,
        "ensuring again makes nothing"
    );
    let chain = Chain::unkeyed();
    audit::append(&s.db, &chain, &event(1)).await.unwrap();
    for sql in [
        "UPDATE audit_events SET agent = 'x'",
        "DELETE FROM audit_events",
        "TRUNCATE audit_events",
        "UPDATE audit_events_p0 SET agent = 'x'",
        "DELETE FROM audit_events_p0",
        "TRUNCATE audit_events_p0",
    ] {
        assert!(s.refused(sql).await, "{sql}");
    }
    // A row in a partition the maintenance function made (an empty table fires no row trigger).
    append_next_month(&s, &chain, 10).await;
    let part = s
        .text(
            "SELECT c.relname::text FROM pg_inherits i JOIN pg_class c ON c.oid = i.inhrelid
             WHERE i.inhparent = 'audit_events'::regclass AND c.relname <> 'audit_events_p0'
               AND EXISTS (SELECT 1 FROM audit_events WHERE tableoid = c.oid) LIMIT 1",
        )
        .await
        .unwrap();
    for sql in [
        format!("DELETE FROM {part}"),
        format!("TRUNCATE {part}"),
        format!("UPDATE {part} SET agent = 'x'"),
    ] {
        assert!(s.refused(&sql).await, "{sql}");
    }
    assert_eq!(s.n("SELECT count(*) FROM audit_events").await, 2);
    s.finish().await;
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn rows_written_before_the_migration_keep_their_ids_and_the_chain() {
    let s = Scratch::new(Some(17)).await;
    let chain = Chain::unkeyed();
    for n in 0..3 {
        append_at(&s, &chain, n, "now() AT TIME ZONE 'UTC'").await;
    }
    let ids_before = s
        .text("SELECT string_agg(id::text, ',' ORDER BY id) FROM audit_events")
        .await;
    Migrator::up(&s.db, None).await.unwrap();
    assert_eq!(
        s.text("SELECT string_agg(id::text, ',' ORDER BY id) FROM audit_events_p0")
            .await,
        ids_before
    );
    assert_eq!(
        audit::verify(&s.db, &chain).await.unwrap(),
        Verdict::Intact { rows: 3 }
    );
    for n in 3..5 {
        audit::append(&s.db, &chain, &event(n)).await.unwrap();
    }
    assert_eq!(
        audit::verify(&s.db, &chain).await.unwrap(),
        Verdict::Intact { rows: 5 }
    );
    assert!(s.refused("DELETE FROM audit_events").await);
    s.finish().await;
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn the_chain_holds_across_a_partition_boundary_and_a_break_is_found_on_the_far_side() {
    let s = Scratch::new(None).await;
    let chain = Chain::unkeyed();
    for n in 0..2 {
        audit::append(&s.db, &chain, &event(n)).await.unwrap();
    }
    append_next_month(&s, &chain, 10).await;
    append_next_month(&s, &chain, 11).await;
    assert!(
        s.n("SELECT count(*) FROM audit_events_p0").await == 2
            && s.n("SELECT count(*) FROM audit_events").await == 4,
        "two rows in the first partition, two in the next month's"
    );
    assert_eq!(
        audit::verify(&s.db, &chain).await.unwrap(),
        Verdict::Intact { rows: 4 }
    );
    s.db.execute_unprepared("ALTER TABLE audit_events DISABLE TRIGGER USER")
        .await
        .unwrap();
    s.db.execute_unprepared("UPDATE audit_events SET agent = 'forged' WHERE agent = 'user-11'")
        .await
        .unwrap();
    s.db.execute_unprepared("ALTER TABLE audit_events ENABLE TRIGGER USER")
        .await
        .unwrap();
    assert!(matches!(
        audit::verify(&s.db, &chain).await.unwrap(),
        Verdict::Broken { .. }
    ));
    s.finish().await;
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn archiving_moves_partitions_keeps_an_anchor_and_the_chain_goes_on() {
    let s = Scratch::new(None).await;
    let chain = Chain::unkeyed();
    for n in 0..2 {
        audit::append(&s.db, &chain, &event(n)).await.unwrap();
    }
    let last_old = s
        .text("SELECT digest FROM audit_events ORDER BY id DESC LIMIT 1")
        .await
        .unwrap();
    append_next_month(&s, &chain, 10).await;
    append_next_month(&s, &chain, 11).await;
    // Everything that ends by the first day of next month's second day: the first partition only.
    let moved = s
        .text(
            "SELECT array_to_string(audit_archive_before(
                 (date_trunc('month', now() AT TIME ZONE 'UTC') + interval '1 month') AT TIME ZONE 'UTC'
                 + interval '1 day'), ',')",
        )
        .await
        .unwrap();
    assert_eq!(moved, "audit_events_p0");
    assert_eq!(
        s.n("SELECT count(*) FROM audit_archive.audit_events_p0")
            .await,
        2,
        "moved, not deleted"
    );
    assert_eq!(s.n("SELECT count(*) FROM audit_events").await, 2);
    let anchor = s
        .text(&format!(
            "SELECT (first_prev_digest IS NULL AND row_count = 2 AND last_digest = '{last_old}')::text
             FROM audit_archive_anchors WHERE partition = 'audit_events_p0'"
        ))
        .await;
    assert_eq!(anchor.as_deref(), Some("true"));
    // The check starts at the anchor and finds the two live rows intact.
    assert_eq!(
        audit::verify(&s.db, &chain).await.unwrap(),
        Verdict::Intact { rows: 2 }
    );
    // An archived row is still in a patient's access log: the view reads the archive too.
    let log = audit::patient_log(
        &s.db,
        "0",
        "someone",
        &audit::PatientLogFilter::default(),
        10,
    )
    .await
    .unwrap();
    assert_eq!(log.len(), 1, "Patient/0 was written before the archive");
    let found = audit::search(
        &s.db,
        &audit::AuditQuery {
            agent: Some("user-1".into()),
            ..audit::AuditQuery::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(found.len(), 1, "the audit search finds an archived row");
    assert_eq!(s.n("SELECT count(*) FROM audit_events_all").await, 4);
    assert_eq!(s.n("SELECT count(*) FROM audit_events").await, 2);
    // The archive and the anchors stay append-only.
    for sql in [
        "UPDATE audit_archive.audit_events_p0 SET agent = 'x'",
        "DELETE FROM audit_archive.audit_events_p0",
        "TRUNCATE audit_archive.audit_events_p0",
        "UPDATE audit_archive_anchors SET row_count = 0",
        "DELETE FROM audit_archive_anchors",
        "TRUNCATE audit_archive_anchors",
    ] {
        assert!(s.refused(sql).await, "{sql}");
    }
    // The current month has no partition now; the maintenance step makes it, and writing carries on.
    assert!(audit_retention::ensure_partitions(&s.db, 24).await.unwrap() >= 1);
    audit::append(&s.db, &chain, &event(20)).await.unwrap();
    assert_eq!(
        audit::verify(&s.db, &chain).await.unwrap(),
        Verdict::Intact { rows: 3 }
    );
    s.finish().await;
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn an_empty_live_table_continues_the_chain_from_the_newest_anchor() {
    let s = Scratch::new(None).await;
    let chain = Chain::unkeyed();
    for n in 0..2 {
        audit::append(&s.db, &chain, &event(n)).await.unwrap();
    }
    let last = s
        .text("SELECT digest FROM audit_events ORDER BY id DESC LIMIT 1")
        .await
        .unwrap();
    s.text("SELECT array_to_string(audit_archive_before('2200-01-01'::timestamptz), ',')")
        .await;
    assert_eq!(s.n("SELECT count(*) FROM audit_events").await, 0);
    assert_eq!(
        audit::verify(&s.db, &chain).await.unwrap(),
        Verdict::Intact { rows: 0 }
    );
    audit_retention::ensure_partitions(&s.db, 24).await.unwrap();
    audit::append(&s.db, &chain, &event(30)).await.unwrap();
    assert_eq!(
        s.text("SELECT prev_digest FROM audit_events")
            .await
            .as_deref(),
        Some(last.as_str()),
        "the new row links to the archived chain, not to nothing"
    );
    assert_eq!(
        audit::verify(&s.db, &chain).await.unwrap(),
        Verdict::Intact { rows: 1 }
    );
    s.finish().await;
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn the_archive_check_finds_an_archived_table_that_no_longer_matches_its_anchor() {
    let s = Scratch::new(None).await;
    let chain = Chain::unkeyed();
    for n in 0..3 {
        audit::append(&s.db, &chain, &event(n)).await.unwrap();
    }
    append_next_month(&s, &chain, 10).await;
    // No archive yet: nothing to check.
    assert_eq!(
        audit::verify_archive(&s.db, &chain, 2).await.unwrap(),
        Verdict::Intact { rows: 0 }
    );
    s.text(
        "SELECT array_to_string(audit_archive_before(
             (date_trunc('month', now() AT TIME ZONE 'UTC') + interval '1 month') AT TIME ZONE 'UTC'
             + interval '1 day'), ',')",
    )
    .await
    .unwrap();
    // A small batch crosses batches; the archive is intact.
    assert_eq!(
        audit::verify_archive(&s.db, &chain, 2).await.unwrap(),
        Verdict::Intact { rows: 3 }
    );
    // The triggers refuse an edit, so a superuser who disables them is the case: do that, edit, restore.
    s.exec("ALTER TABLE audit_archive.audit_events_p0 DISABLE TRIGGER USER")
        .await;
    s.exec("UPDATE audit_archive.audit_events_p0 SET agent = 'someone-else' WHERE id = 2")
        .await;
    s.exec("ALTER TABLE audit_archive.audit_events_p0 ENABLE TRIGGER USER")
        .await;
    match audit::verify_archive(&s.db, &chain, 2).await.unwrap() {
        Verdict::Broken { id, reason } => {
            assert_eq!(id, 2);
            assert!(reason.contains("audit_events_p0"), "{reason}");
        }
        other => panic!("expected a break, got {other:?}"),
    }
    s.finish().await;
}
