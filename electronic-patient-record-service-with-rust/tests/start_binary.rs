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
