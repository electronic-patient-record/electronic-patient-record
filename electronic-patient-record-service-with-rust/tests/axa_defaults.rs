//! What `axa` 0.11 adds to every response with no code here asking for it (AX-11), pinned so a later
//! upgrade that drops or changes one is noticed: `cross-origin-opener-policy` and
//! `cross-origin-resource-policy` of `same-origin`, and a `permissions-policy` that turns off the
//! camera, microphone, location, payment and USB. No database: the pool points nowhere.
//!
//! Not covered: the 503 a database outage now answers on a store route (it needs the stores), the
//! cookie `no-store` rule (the service sets no cookie), and compression.

use axa::config::{Config, Defaults};
use axa::{AppContext, Environment, routes::AppRoutes};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use tower::ServiceExt;

#[tokio::test]
async fn every_response_carries_the_isolation_and_permissions_headers() {
    let pool = PgPoolOptions::new()
        .acquire_timeout(Duration::from_millis(50))
        .connect_lazy("postgres://nobody@127.0.0.1:1/nothing")
        .unwrap();
    let defaults = Defaults {
        database: "",
        ..Defaults::default()
    };
    let config = Config::from_lookup(&Environment::Test, &defaults, |_| None).unwrap();
    let ctx = AppContext::new(Environment::Test, pool, config);
    // As `App::router` does: the routes, then `axa`'s standard stack around them.
    let router = AppRoutes::empty()
        .add_route(axa::health::routes())
        .into_router(ctx.clone());
    let router = axa::middleware::apply(router, &ctx.config.http).unwrap();
    let response = router
        .oneshot(
            Request::builder()
                .uri("/_health/live")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let h = response.headers();
    let get = |name: &str| {
        h.get(name)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_else(|| panic!("{name} is missing: {h:?}"))
            .to_string()
    };
    assert_eq!(get("cross-origin-opener-policy"), "same-origin");
    assert_eq!(get("cross-origin-resource-policy"), "same-origin");
    let permissions = get("permissions-policy");
    for feature in [
        "camera=()",
        "microphone=()",
        "geolocation=()",
        "payment=()",
        "usb=()",
    ] {
        assert!(permissions.contains(feature), "{feature} in {permissions}");
    }
}
