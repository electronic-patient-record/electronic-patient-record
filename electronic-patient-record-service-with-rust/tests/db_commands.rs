//! DB-gated: the real binary's `db` commands against a scratch database (`db status`, `migrate`, `reset`, and the two
//! the service refuses even when confirmed: `down` and `truncate`). `destroy_refused` shows the refusals that happen
//! before the service runs; these reach the service's own code.
//!
//! ```text
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test main db_commands -- --ignored --test-threads=1
//! ```

use std::process::{Command, Output};

use sqlx::{AssertSqlSafe, PgPool};

const BINARY: &str = env!("CARGO_BIN_EXE_electronic-patient-record");

/// A new empty database, dropped when the test ends (best effort).
struct Scratch {
    admin: PgPool,
    name: String,
    url: String,
}

impl Scratch {
    async fn new() -> Self {
        let url = std::env::var("DB_URL").expect("DB_URL must be set for --ignored tests");
        let (base, _) = url.rsplit_once('/').expect("a database URL");
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let name = format!("epr_dbcmd_{}_{}", std::process::id(), nanos);
        let admin = PgPool::connect(&url).await.expect("connect admin");
        sqlx::raw_sql(AssertSqlSafe(format!("CREATE DATABASE {name}")))
            .execute(&admin)
            .await
            .expect("create scratch db");
        let url = format!("{base}/{name}");
        Self { admin, name, url }
    }

    fn run(&self, args: &[&str], confirmed: bool) -> Output {
        let mut command = Command::new(BINARY);
        command
            .args(args)
            .env_clear()
            .env("APP_ENV", "test")
            .env("DB_URL", &self.url);
        if confirmed {
            command.env("AXA_DESTROY_DATA", "true");
        }
        // The one variable kept from the test's own environment, so a coverage run records what this program does.
        if let Some(profile) = std::env::var_os("LLVM_PROFILE_FILE") {
            command.env("LLVM_PROFILE_FILE", profile);
        }
        command.output().expect("the binary runs")
    }

    async fn finish(self) {
        let _ = sqlx::raw_sql(AssertSqlSafe(format!(
            "DROP DATABASE IF EXISTS {} WITH (FORCE)",
            self.name
        )))
        .execute(&self.admin)
        .await;
    }
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn status_lists_every_migration_pending_then_applied_after_migrate() {
    let scratch = Scratch::new().await;

    let before = scratch.run(&["db", "status"], false);
    assert!(before.status.success(), "{}", stderr(&before));
    let pending = stdout(&before);
    assert!(pending.lines().count() > 10);
    assert!(pending.lines().all(|l| l.starts_with("pending\t")));

    let migrate = scratch.run(&["db", "migrate"], false);
    assert!(migrate.status.success(), "{}", stderr(&migrate));
    let after = stdout(&scratch.run(&["db", "status"], false));
    assert_eq!(after.lines().count(), pending.lines().count());
    assert!(after.lines().all(|l| l.starts_with("applied\t")));

    // A second migrate changes nothing.
    assert!(scratch.run(&["db", "migrate"], false).status.success());
    scratch.finish().await;
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn down_and_truncate_are_refused_even_when_confirmed() {
    let scratch = Scratch::new().await;
    let down = scratch.run(&["db", "down"], true);
    assert!(!down.status.success());
    assert!(stderr(&down).contains("migrations have no down files"));
    let truncate = scratch.run(&["db", "truncate"], true);
    assert!(!truncate.status.success());
    assert!(stderr(&truncate).contains("this service has no truncate"));
    scratch.finish().await;
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_confirmed_reset_drops_what_was_there_and_migrates_again() {
    let scratch = Scratch::new().await;
    assert!(scratch.run(&["db", "migrate"], false).status.success());
    let db = PgPool::connect(&scratch.url)
        .await
        .expect("connect scratch");
    sqlx::raw_sql("CREATE TABLE left_behind (id int)")
        .execute(&db)
        .await
        .unwrap();
    drop(db);

    let reset = scratch.run(&["db", "reset"], true);
    assert!(reset.status.success(), "{}", stderr(&reset));

    let db = PgPool::connect(&scratch.url)
        .await
        .expect("connect scratch");
    let left: bool = sqlx::query_scalar("SELECT to_regclass('public.left_behind') IS NOT NULL")
        .fetch_one(&db)
        .await
        .unwrap();
    let migrated: bool =
        sqlx::query_scalar("SELECT to_regclass('public.audit_events') IS NOT NULL")
            .fetch_one(&db)
            .await
            .unwrap();
    assert!(!left, "reset drops every table");
    assert!(migrated, "reset migrates again");
    drop(db);
    scratch.finish().await;
}
