//! Lockout by client address (OS-2, H-049): an address answered too many `401`s is refused every request
//! until the window ends, with a valid token too; another address is not; a success never counts; the
//! probes are never refused; a FHIR refusal is an `OperationOutcome`; with the setting unset nothing is
//! counted; and the first refusal of an address in a window is one audit row naming it. The router
//! tests need no database (a disconnected pool fails the audit write, which is counted, not fatal); the
//! audit test is database-gated.
//!
//! ```text
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test http_lockout -- --ignored --test-threads=1
//! ```

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderMap, Request, StatusCode, header};
use axum::middleware::{self, Next};
use axum::routing::get;
use electronic_patient_record::lockout;
use electronic_patient_record::metrics;
use sqlx::{PgPool, Row};
use std::net::IpAddr;
use std::num::NonZeroU32;
use tower::ServiceExt;

/// Answers 401 to `X-Test-Token: bad` and 200 otherwise, as authentication would.
async fn guarded(headers: HeaderMap) -> StatusCode {
    if headers.get("x-test-token").is_some_and(|v| v == "bad") {
        StatusCode::UNAUTHORIZED
    } else {
        StatusCode::OK
    }
}

/// A router whose client is the address in `X-Test-Address`, as `axa` would have resolved it.
fn app(db: PgPool, after: Option<u32>) -> Router {
    let routes = Router::new()
        .route("/api/things", get(guarded))
        .route("/fhir/R4/Patient", get(guarded))
        .route("/_health/live", get(guarded));
    let limiter = after.map(|n| lockout::limiter(NonZeroU32::new(n).unwrap()));
    lockout::layer(routes, limiter, db).layer(middleware::from_fn(
        |mut req: Request<Body>, next: Next| async move {
            if let Some(a) = req
                .headers()
                .get("x-test-address")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<IpAddr>().ok())
            {
                req.extensions_mut().insert(a);
            }
            next.run(req).await
        },
    ))
}

fn nowhere() -> PgPool {
    sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(100))
        .connect_lazy("postgres://nobody@127.0.0.1:1/none")
        .unwrap()
}

fn request(path: &str, address: &str, token: &str) -> Request<Body> {
    Request::builder()
        .uri(path)
        .header("x-test-address", address)
        .header("x-test-token", token)
        .body(Body::empty())
        .unwrap()
}

async fn status(app: &Router, path: &str, address: &str, token: &str) -> StatusCode {
    app.clone()
        .oneshot(request(path, address, token))
        .await
        .unwrap()
        .status()
}

#[tokio::test]
async fn an_address_past_its_401s_is_refused_even_with_a_good_token_and_no_other_is() {
    let app = app(nowhere(), Some(3));
    let before = metrics::LOCKOUTS.get();
    for _ in 0..3 {
        assert_eq!(
            status(&app, "/api/things", "203.0.113.7", "bad").await,
            StatusCode::UNAUTHORIZED
        );
    }
    let refused = app
        .clone()
        .oneshot(request("/api/things", "203.0.113.7", "good"))
        .await
        .unwrap();
    assert_eq!(refused.status(), StatusCode::TOO_MANY_REQUESTS);
    let wait: u64 = refused.headers()[header::RETRY_AFTER]
        .to_str()
        .unwrap()
        .parse()
        .unwrap();
    assert!(
        wait > 0 && wait <= u64::from(lockout::WINDOW_SECS),
        "{wait}"
    );
    let body = axum::body::to_bytes(refused.into_body(), 1 << 16)
        .await
        .unwrap();
    let problem: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(problem["retry_after_seconds"], wait);
    assert!(metrics::LOCKOUTS.get() > before, "counted");
    assert_eq!(
        status(&app, "/api/things", "203.0.113.8", "good").await,
        StatusCode::OK,
        "another address is not locked out"
    );
    assert_eq!(
        status(&app, "/_health/live", "203.0.113.7", "good").await,
        StatusCode::OK,
        "the probes are never refused"
    );
}

#[tokio::test]
async fn a_success_never_counts_and_unset_counts_nothing() {
    let app_on = app(nowhere(), Some(1));
    for _ in 0..50 {
        assert_eq!(
            status(&app_on, "/api/things", "198.51.100.1", "good").await,
            StatusCode::OK
        );
    }
    let off = app(nowhere(), None);
    for _ in 0..50 {
        assert_eq!(
            status(&off, "/api/things", "198.51.100.2", "bad").await,
            StatusCode::UNAUTHORIZED,
            "with the setting unset there is no lockout"
        );
    }
}

#[tokio::test]
async fn an_ipv6_address_is_counted_by_its_64_and_a_fhir_refusal_is_an_operation_outcome() {
    let app = app(nowhere(), Some(1));
    assert_eq!(
        status(&app, "/fhir/R4/Patient", "2001:db8:1:2::1", "bad").await,
        StatusCode::UNAUTHORIZED
    );
    let refused = app
        .clone()
        .oneshot(request("/fhir/R4/Patient", "2001:db8:1:2::ffff", "good"))
        .await
        .unwrap();
    assert_eq!(refused.status(), StatusCode::TOO_MANY_REQUESTS, "same /64");
    assert_eq!(refused.headers()["content-type"], "application/fhir+json");
    let body = axum::body::to_bytes(refused.into_body(), 1 << 16)
        .await
        .unwrap();
    let outcome: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(outcome["resourceType"], "OperationOutcome");
    assert_eq!(
        status(&app, "/fhir/R4/Patient", "2001:db8:1:3::1", "good").await,
        StatusCode::OK,
        "another /64"
    );
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn the_first_refusal_of_an_address_in_a_window_is_one_audit_row() {
    let url = std::env::var("DB_URL").expect("DB_URL must be set for --ignored tests");
    let db = PgPool::connect(&url).await.expect("connect");
    electronic_patient_record::MIGRATOR
        .run(&db)
        .await
        .expect("migrate");
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let address = format!("10.{}.{}.{}", (n >> 16) % 256, (n >> 8) % 256, n % 256);
    let app = app(db.clone(), Some(1));
    assert_eq!(
        status(&app, "/api/things", &address, "bad").await,
        StatusCode::UNAUTHORIZED
    );
    for _ in 0..3 {
        assert_eq!(
            status(&app, "/api/things", &address, "good").await,
            StatusCode::TOO_MANY_REQUESTS
        );
    }
    let rows: i64 = sqlx::query(
        "SELECT count(*) AS n FROM audit_events
             WHERE network_address = $1 AND resource_ref = 'lockout' AND outcome = 4
               AND agent = 'anonymous' AND detail->>'denied' = 'lockout'",
    )
    .bind(&address)
    .fetch_one(&db)
    .await
    .unwrap()
    .try_get("n")
    .unwrap();
    assert_eq!(rows, 1, "three refusals, one audit row");
}
