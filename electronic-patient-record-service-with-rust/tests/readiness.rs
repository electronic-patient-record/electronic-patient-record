//! `/_health/ready` answers for the database: it is `axa`'s route, and `axa` asks the pool on the context.
//!
//! Covered: 503 naming `database`, with no error text, when the pool cannot connect; 200 when it can.
//! Not covered: how long a failing answer is reused, which `axa` tests.
//! `/_dependencies` is here too, because it reports the same outage without refusing.
//!
//! ```text
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test readiness -- --ignored --test-threads=1
//! ```

use axa::config::{Config, Defaults};
use axa::{AppContext, Environment, routes::AppRoutes};
use axa_test::prelude::request;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use electronic_patient_record::app::App;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use tower::ServiceExt;

/// Nothing listens on port 1.
#[tokio::test]
async fn a_database_that_does_not_answer_is_a_503_that_names_it_and_nothing_else() {
    let pool = PgPoolOptions::new()
        .acquire_timeout(Duration::from_millis(50))
        .connect_lazy("postgres://secret-user:secret-pass@127.0.0.1:1/nothing")
        .unwrap();
    let defaults = Defaults {
        database: "",
        ..Defaults::default()
    };
    let config = Config::from_lookup(&Environment::Test, &defaults, |_| None).unwrap();
    let ctx = AppContext::new(Environment::Test, pool, config);
    let router = AppRoutes::empty()
        .add_route(axa::health::routes())
        .into_router(ctx);
    let response = router
        .oneshot(
            Request::builder()
                .uri("/_health/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    // Since `axa` 0.6 the refusal is an RFC 9457 problem that still carries `ok` and `failed`, and names
    // nothing but the check: no address, no driver message.
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        body,
        serde_json::json!({
            "type": "about:blank",
            "title": "Service Unavailable",
            "status": 503,
            "code": "not_ready",
            "ok": false,
            "failed": ["database"],
        })
    );
}

/// The other half of the same outage: `/_dependencies` says the same thing with a 200, because it
/// only reports. It is the route's own test; the rounds are tested in `src/dependencies.rs`.
#[tokio::test]
async fn a_database_that_does_not_answer_is_reported_with_a_200() {
    let pool = PgPoolOptions::new()
        .acquire_timeout(Duration::from_millis(50))
        .connect_lazy("postgres://secret-user:secret-pass@127.0.0.1:1/nothing")
        .unwrap();
    let defaults = Defaults {
        database: "",
        ..Defaults::default()
    };
    let config = Config::from_lookup(&Environment::Test, &defaults, |_| None).unwrap();
    let ctx = AppContext::new(Environment::Test, pool, config);
    let router = AppRoutes::empty()
        .add_route(electronic_patient_record::dependencies::routes())
        .into_router(ctx);
    let response = router
        .oneshot(
            Request::builder()
                .uri("/_dependencies")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let text = String::from_utf8(body.to_vec()).unwrap();
    assert_eq!(
        text,
        r#"{"ok":false,"checked":["database"],"degraded":["database"]}"#
    );
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_database_that_answers_is_ready() {
    request::<App, _, _>(|server, _ctx| async move {
        let r = server.get("/_health/ready").await;
        assert_eq!(r.status_code(), StatusCode::OK, "{}", r.text());
        // Through the whole middleware stack, with no token.
        let r = server.get("/_dependencies").await;
        assert_eq!(r.status_code(), StatusCode::OK, "{}", r.text());
        assert!(
            r.text().contains(r#""checked":["database"]"#),
            "{}",
            r.text()
        );
    })
    .await;
}
