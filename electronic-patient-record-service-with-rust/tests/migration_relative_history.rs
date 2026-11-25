//! DB-gated (EH-3): migration 43, which replaces an empty clinical table, refuses to run while the table holds anything,
//! and then changes nothing. No migration may remove a clinical row, and this is the only one that drops a clinical table.
//!
//! ```text
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test main migration_relative_history:: -- --ignored --test-threads=1
//! ```

use electronic_patient_record::MIGRATOR;
use sqlx::{AssertSqlSafe, PgPool};

const VERSION: i64 = 20_261_124_000_043;

/// A new database migrated to just before migration 43, dropped when the test ends (best effort).
struct Before {
    db: PgPool,
    admin: PgPool,
    name: String,
}

impl Before {
    async fn new() -> Self {
        let url = std::env::var("DB_URL").expect("DB_URL must be set for --ignored tests");
        let (base, _) = url.rsplit_once('/').expect("a database URL");
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let name = format!("epr_scratch_{}_{}", std::process::id(), nanos);
        let admin = PgPool::connect(&url).await.expect("connect admin");
        sqlx::raw_sql(AssertSqlSafe(format!("CREATE DATABASE {name}")))
            .execute(&admin)
            .await
            .expect("create scratch db");
        let db = PgPool::connect(&format!("{base}/{name}"))
            .await
            .expect("connect scratch");
        let mut ran = 0;
        for m in MIGRATOR.iter().filter(|m| m.version < VERSION) {
            sqlx::raw_sql(AssertSqlSafe(m.sql.as_str().to_string()))
                .execute(&db)
                .await
                .unwrap_or_else(|e| panic!("{}: {e}", m.description));
            ran += 1;
        }
        assert_eq!(ran, 42, "the migrations before it");
        Self { db, admin, name }
    }

    /// Run migration 43 in a transaction, as the migrator does.
    async fn migrate(&self) -> Result<(), sqlx::Error> {
        let m = MIGRATOR
            .iter()
            .find(|m| m.version == VERSION)
            .expect("migration 43");
        assert!(!m.no_tx, "it must be all or nothing");
        let mut tx = self.db.begin().await?;
        sqlx::raw_sql(AssertSqlSafe(m.sql.as_str().to_string()))
            .execute(&mut *tx)
            .await?;
        tx.commit().await
    }

    async fn exists(&self, table: &str) -> bool {
        sqlx::query_scalar("SELECT to_regclass($1) IS NOT NULL")
            .bind(table)
            .fetch_one(&self.db)
            .await
            .unwrap()
    }

    async fn finish(self) {
        self.db.close().await;
        let _ = sqlx::raw_sql(AssertSqlSafe(format!(
            "DROP DATABASE IF EXISTS {} WITH (FORCE)",
            self.name
        )))
        .execute(&self.admin)
        .await;
    }
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn it_refuses_while_there_is_an_entry_and_then_nothing_has_changed() {
    let before = Before::new().await;
    sqlx::raw_sql(
        "INSERT INTO family_history_versions (entry_id, version, patient_id, author, status, body)
         VALUES (gen_random_uuid(), 1, gen_random_uuid(), 'dr-1', 'recorded', '{\"condition\": \"kept\"}')",
    )
    .execute(&before.db)
    .await
    .unwrap();
    let refused = before.migrate().await.expect_err("it must not run");
    assert!(refused.to_string().contains("holds entries"), "{refused}");
    let kept: String = sqlx::query_scalar("SELECT body->>'condition' FROM family_history_current")
        .fetch_one(&before.db)
        .await
        .unwrap();
    assert_eq!(kept, "kept");
    assert!(!before.exists("relative_history_versions").await);
    // It is still append-only: a refusal left the triggers as they were.
    let changed = sqlx::raw_sql("DELETE FROM family_history_versions")
        .execute(&before.db)
        .await;
    assert!(changed.unwrap_err().to_string().contains("append-only"));
    before.finish().await;
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn it_replaces_an_empty_table_and_both_tables_stay_append_only() {
    let before = Before::new().await;
    before.migrate().await.expect("an empty table is replaced");
    for gone in ["family_history_versions", "family_history_current"] {
        assert!(!before.exists(gone).await, "{gone}");
    }
    for (table, id, status) in [
        ("relative_history_versions", "entry_id", "recorded"),
        ("problem_versions", "problem_id", "active"),
    ] {
        sqlx::raw_sql(AssertSqlSafe(format!(
            "INSERT INTO {table} ({id}, version, patient_id, author, status, body)
             VALUES (gen_random_uuid(), 1, gen_random_uuid(), 'dr-1', '{status}', '{{}}')"
        )))
        .execute(&before.db)
        .await
        .unwrap();
        for sql in [
            format!("UPDATE {table} SET author = 'x'"),
            format!("DELETE FROM {table}"),
            format!("TRUNCATE {table}"),
        ] {
            let e = sqlx::raw_sql(AssertSqlSafe(sql.clone()))
                .execute(&before.db)
                .await
                .expect_err(&sql);
            assert!(e.to_string().contains("append-only"), "{sql}: {e}");
        }
    }
    before.finish().await;
}
