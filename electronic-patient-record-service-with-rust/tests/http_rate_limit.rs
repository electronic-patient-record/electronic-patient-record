//! Request limits (OS-2): the limit trips, `Retry-After` is set, the body says so, a request with no
//! subject is not counted, the compare call has its own limit, and the first refusal in a window is an
//! audit row. The router tests need no database (a disconnected connection fails the audit write, which
//! is counted, not fatal); the audit test is database-gated.
//!
//! ```text
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test http_rate_limit -- --ignored --test-threads=1
//! ```

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use axum::middleware::{self, Next};
use axum::routing::{get, post};
use electronic_patient_record::auth::Agent;
use electronic_patient_record::metrics;
use electronic_patient_record::rate_limit::{Gate, Limiter, Limits, rate_limit_mw};
use electronic_patient_record_migration::MigratorTrait;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement};
use std::sync::Arc;
use tower::ServiceExt;

/// A router whose caller is the subject named in `X-Test-Subject` (none when absent), as the
/// authentication middleware would have set it.
fn app(db: DatabaseConnection, general: u32, compare: u32) -> Router {
    let gate = Gate {
        limiter: Arc::new(Limiter::new(Limits { general, compare })),
        db,
    };
    Router::new()
        .route("/api/things", get(|| async { "ok" }))
        .route("/api/patients/compare", post(|| async { "ok" }))
        .layer(middleware::from_fn_with_state(gate, rate_limit_mw))
        .layer(middleware::from_fn(
            |mut req: Request<Body>, next: Next| async move {
                if let Some(s) = req
                    .headers()
                    .get("x-test-subject")
                    .and_then(|v| v.to_str().ok())
                {
                    let agent = Agent(s.to_string());
                    req.extensions_mut().insert(agent);
                }
                next.run(req).await
            },
        ))
}

fn request(method: Method, path: &str, subject: Option<&str>) -> Request<Body> {
    let mut b = Request::builder().method(method).uri(path);
    if let Some(s) = subject {
        b = b.header("x-test-subject", s);
    }
    b.body(Body::empty()).unwrap()
}

#[tokio::test]
async fn a_subject_past_its_limit_gets_429_with_retry_after() {
    let app = app(DatabaseConnection::default(), 2, 2);
    let before = metrics::RATE_LIMITED.get();
    for _ in 0..2 {
        let res = app
            .clone()
            .oneshot(request(Method::GET, "/api/things", Some("dr-limit")))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
    let res = app
        .clone()
        .oneshot(request(Method::GET, "/api/things", Some("dr-limit")))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::TOO_MANY_REQUESTS);
    let wait: u64 = res.headers()[header::RETRY_AFTER]
        .to_str()
        .unwrap()
        .parse()
        .unwrap();
    assert!((1..=60).contains(&wait), "{wait}");
    let body = axum::body::to_bytes(res.into_body(), 1 << 16)
        .await
        .unwrap();
    let text = String::from_utf8(body.to_vec()).unwrap();
    assert!(text.contains("too many requests"), "{text}");
    assert!(metrics::RATE_LIMITED.get() > before);
    // Another subject is not affected.
    let other = app
        .oneshot(request(Method::GET, "/api/things", Some("dr-other")))
        .await
        .unwrap();
    assert_eq!(other.status(), StatusCode::OK);
}

#[tokio::test]
async fn a_request_with_no_subject_is_not_counted() {
    let app = app(DatabaseConnection::default(), 1, 1);
    for _ in 0..5 {
        let res = app
            .clone()
            .oneshot(request(Method::GET, "/api/things", None))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
}

#[tokio::test]
async fn compare_has_its_own_limit_and_counts_apart_from_the_rest() {
    let app = app(DatabaseConnection::default(), 100, 1);
    let first = app
        .clone()
        .oneshot(request(
            Method::POST,
            "/api/patients/compare",
            Some("dr-cmp"),
        ))
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::OK);
    let second = app
        .clone()
        .oneshot(request(
            Method::POST,
            "/api/patients/compare",
            Some("dr-cmp"),
        ))
        .await
        .unwrap();
    assert_eq!(second.status(), StatusCode::TOO_MANY_REQUESTS);
    // The general limit is still open for the same subject.
    let other = app
        .oneshot(request(Method::GET, "/api/things", Some("dr-cmp")))
        .await
        .unwrap();
    assert_eq!(other.status(), StatusCode::OK);
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn the_first_refusal_in_a_window_is_one_audit_row() {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for --ignored tests");
    let db = Database::connect(url).await.expect("connect");
    electronic_patient_record_migration::Migrator::up(&db, None)
        .await
        .expect("migrate");
    let subject = format!(
        "dr-audit-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let app = app(db.clone(), 1, 1);
    for _ in 0..4 {
        app.clone()
            .oneshot(request(Method::GET, "/api/things", Some(&subject)))
            .await
            .unwrap();
    }
    let rows: i64 = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT count(*) AS n FROM audit_events
             WHERE agent = $1 AND resource_ref = 'rate-limit' AND outcome = 4",
            vec![subject.into()],
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "n")
        .unwrap();
    assert_eq!(rows, 1, "three refusals, one audit row");
}
