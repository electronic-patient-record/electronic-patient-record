//! `/_readiness` answers for the database (it is `axa`'s route, with our named check).
//!
//! Covered: 503 naming `database`, with no error text, when the pool cannot connect; 200 when it can.
//! Not covered: how long a failing answer is reused, which `axa` tests.
//!
//! ```text
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test readiness -- --ignored --test-threads=1
//! ```

use axa::config::{Config, Defaults};
use axa::testing::prelude::request;
use axa::{AppContext, Environment, routes::AppRoutes};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use electronic_patient_record::app::{App, readiness_checks};
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
    let mut ctx = AppContext::new(Environment::Test, (), config);
    ctx.health_checks = readiness_checks(pool);
    let router = AppRoutes::empty()
        .add_route(axa::health::routes())
        .into_router(ctx);
    let response = router
        .oneshot(
            Request::builder()
                .uri("/_readiness")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let text = String::from_utf8(body.to_vec()).unwrap();
    assert_eq!(text, r#"{"ok":false,"failed":["database"]}"#);
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_database_that_answers_is_ready() {
    request::<App, _, _>(|server, _ctx| async move {
        let r = server.get("/_readiness").await;
        assert_eq!(r.status_code(), StatusCode::OK, "{}", r.text());
    })
    .await;
}
