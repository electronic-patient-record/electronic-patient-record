//! DB-gated: the real binary's `start` command, the path a deployment runs (`App::run`, the settings checks and the
//! background loops they start). It boots against a scratch database with the optional parts switched on (the MLLP
//! listener, the outbox relay, task reminders, the request limit and the lockout), answers `/_health/ready`, and stops
//! cleanly on SIGTERM. Only the child process gets these settings; the test's own environment is not written.
//!
//! ```text
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test main start_binary:: -- --ignored --test-threads=1
//! ```

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use sqlx::{AssertSqlSafe, PgPool};

const BINARY: &str = env!("CARGO_BIN_EXE_electronic-patient-record");

/// A port that was free a moment ago.
fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// The status line of `GET path`, or `None` if nothing answers yet.
fn status(port: u16, path: &str) -> Option<String> {
    let mut s = TcpStream::connect(("127.0.0.1", port)).ok()?;
    s.set_read_timeout(Some(Duration::from_secs(2))).ok()?;
    write!(
        s,
        "GET {path} HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n"
    )
    .ok()?;
    let mut out = String::new();
    s.read_to_string(&mut out).ok()?;
    out.lines().next().map(String::from)
}

/// Start on `database` without migrating, and say whether it became ready. If not, what it said as it ended.
async fn starts_on(database: &str, allow_newer: Option<&str>) -> Result<(), String> {
    let port = free_port();
    let mut command = Command::new(BINARY);
    command
        .args([
            "start",
            "--binding",
            "127.0.0.1",
            "--port",
            &port.to_string(),
        ])
        .env_clear()
        .env("APP_ENV", "test")
        .env("DB_URL", database)
        .env("DB_AUTO_MIGRATE", "false")
        .env("EPR_REQUIRE_AUTH", "false")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(value) = allow_newer {
        command.env("EPR_START_ON_NEWER_DATABASE", value);
    }
    if let Some(profile) = std::env::var_os("LLVM_PROFILE_FILE") {
        command.env("LLVM_PROFILE_FILE", profile);
    }
    let mut child = command.spawn().expect("the binary starts");
    let deadline = Instant::now() + Duration::from_secs(60);
    let mut ready = false;
    while Instant::now() < deadline && child.try_wait().unwrap().is_none() {
        if status(port, "/_health/ready").is_some_and(|s| s.contains("200")) {
            ready = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    let _ = child.kill();
    let out = child.wait_with_output().unwrap();
    if ready {
        return Ok(());
    }
    Err(format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    ))
}

/// H-074: production does not migrate at start, so a build could be started on a database it was not made for.
#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn start_refuses_a_database_it_was_not_made_for() {
    let url = std::env::var("DB_URL").expect("DB_URL must be set for --ignored tests");
    let (base, _) = url.rsplit_once('/').expect("a database URL");
    let name = format!("epr_schema_{}", std::process::id());
    let admin = PgPool::connect(&url).await.expect("connect admin");
    let drop = format!("DROP DATABASE IF EXISTS {name} WITH (FORCE)");
    for sql in [drop.clone(), format!("CREATE DATABASE {name}")] {
        sqlx::raw_sql(AssertSqlSafe(sql))
            .execute(&admin)
            .await
            .unwrap();
    }
    let scratch = format!("{base}/{name}");
    let empty = starts_on(&scratch, Some("true")).await;
    let db = PgPool::connect(&scratch).await.unwrap();
    electronic_patient_record::MIGRATOR.run(&db).await.unwrap();
    let same = starts_on(&scratch, None).await;
    sqlx::raw_sql(
        "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time)
         VALUES (99990101000000, 'from a newer build', true, '', 0)",
    )
    .execute(&db)
    .await
    .unwrap();
    let newer = starts_on(&scratch, None).await;
    let allowed = starts_on(&scratch, Some("true")).await;
    let wrong = starts_on(&scratch, Some("yes")).await;
    db.close().await;
    sqlx::raw_sql(AssertSqlSafe(drop))
        .execute(&admin)
        .await
        .unwrap();

    assert!(
        empty
            .as_ref()
            .is_err_and(|e| e.contains("older than this build")),
        "{empty:?}"
    );
    assert_eq!(same, Ok(()));
    assert!(
        newer
            .as_ref()
            .is_err_and(|e| e.contains("newer than this build")),
        "{newer:?}"
    );
    assert_eq!(allowed, Ok(()));
    assert!(
        wrong
            .as_ref()
            .is_err_and(|e| e.contains("must be true or false")),
        "{wrong:?}"
    );
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn start_serves_with_every_optional_part_on_and_stops_on_sigterm() {
    let url = std::env::var("DB_URL").expect("DB_URL must be set for --ignored tests");
    let (base, _) = url.rsplit_once('/').expect("a database URL");
    let name = format!("epr_start_{}", std::process::id());
    let admin = PgPool::connect(&url).await.expect("connect admin");
    sqlx::raw_sql(AssertSqlSafe(format!(
        "DROP DATABASE IF EXISTS {name} WITH (FORCE)"
    )))
    .execute(&admin)
    .await
    .unwrap();
    sqlx::raw_sql(AssertSqlSafe(format!("CREATE DATABASE {name}")))
        .execute(&admin)
        .await
        .expect("create scratch db");

    let (port, mllp) = (free_port(), free_port());
    let mut command = Command::new(BINARY);
    command
        .args([
            "start",
            "--binding",
            "127.0.0.1",
            "--port",
            &port.to_string(),
        ])
        .env_clear()
        .env("APP_ENV", "test")
        .env("DB_URL", format!("{base}/{name}"))
        .env("EPR_REQUIRE_AUTH", "false")
        .env("EPR_MLLP_BIND", format!("127.0.0.1:{mllp}"))
        .env("EPR_OUTBOX_INTERVAL_SECS", "3600")
        .env("EPR_TASK_REMINDER_MINUTES", "60")
        .env("EPR_RATE_LIMIT_PER_MINUTE", "1000")
        .env("LOCKOUT_CLIENT_ADDRESS_AFTER_COUNT", "50")
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // The one variable kept from the test's own environment, so a coverage run records what the child does.
    if let Some(profile) = std::env::var_os("LLVM_PROFILE_FILE") {
        command.env("LLVM_PROFILE_FILE", profile);
    }
    let mut child = command.spawn().expect("the binary starts");

    let deadline = Instant::now() + Duration::from_secs(90);
    let mut ready = None;
    while Instant::now() < deadline {
        if let Some(s) = status(port, "/_health/ready") {
            ready = Some(s);
            break;
        }
        if child.try_wait().unwrap().is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    let exited_early = child.try_wait().unwrap();
    let ready_line = ready.clone().unwrap_or_else(String::new);
    let listening = TcpStream::connect(("127.0.0.1", mllp)).is_ok();

    // Stop it as a deployment does: SIGTERM, then it must exit by itself.
    let _ = Command::new("kill")
        .args(["-TERM", &child.id().to_string()])
        .status();
    let stop_by = Instant::now() + Duration::from_secs(30);
    let mut code = None;
    while Instant::now() < stop_by {
        if let Some(c) = child.try_wait().unwrap() {
            code = Some(c);
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    if code.is_none() {
        let _ = child.kill();
        let _ = child.wait();
    }
    let _ = sqlx::raw_sql(AssertSqlSafe(format!(
        "DROP DATABASE IF EXISTS {name} WITH (FORCE)"
    )))
    .execute(&admin)
    .await;

    assert!(exited_early.is_none(), "it exited early: {exited_early:?}");
    assert!(ready_line.contains("200"), "not ready: {ready_line:?}");
    assert!(listening, "the MLLP listener is not accepting");
    assert!(code.is_some(), "it did not stop on SIGTERM within 30 s");
}
