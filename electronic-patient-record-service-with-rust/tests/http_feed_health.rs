//! DB-gated, through the real router with login off: `GET /api/feed/health` judges the configured feed now (IH-8).
//! With no feed configured the list is empty; once the listener's health loop has started, the answer names the
//! sender, its state, the window and the thresholds. The configured feed is kept in the app's runtime (QA-16).
//!
//! ```text
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test main http_feed_health:: -- --ignored --test-threads=1
//! ```

use axa_test::prelude::request_with;
use axum::http::StatusCode;
use electronic_patient_record::app::App;
use electronic_patient_record::feed_health::{self, Thresholds};
use electronic_patient_record::pool::Pool;
use electronic_patient_record::runtime::Runtime;
use serde_json::Value;

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn the_health_route_lists_no_feed_until_one_is_configured_and_then_judges_it() {
    let db_url = std::env::var("DB_URL").expect("DB_URL must be set for --ignored tests");
    let settings = [("DB_URL", db_url.as_str()), ("EPR_REQUIRE_AUTH", "false")];
    request_with::<App, _, _>(&settings, |server, ctx| async move {
        let before = server.get("/api/feed/health").await;
        assert_eq!(before.status_code(), StatusCode::OK);
        let body: Value = before.json();
        assert_eq!(body["feeds"], serde_json::json!([]), "{body}");

        let agent = "mllp:health-test".to_string();
        let runtime: std::sync::Arc<Runtime> = ctx
            .shared_store
            .get_ref::<std::sync::Arc<Runtime>>()
            .map_or_else(Runtime::current, |r| (*r).clone());
        let health_loop = tokio::spawn(feed_health::run_loop(
            runtime.clone(),
            ctx.pool(),
            agent.clone(),
            Thresholds::default(),
        ));
        // The loop records its configuration before its first tick.
        for _ in 0..100 {
            if feed_health::configured(&runtime).is_some() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }

        let after = server.get("/api/feed/health").await;
        assert_eq!(after.status_code(), StatusCode::OK);
        let body: Value = after.json();
        let feed = &body["feeds"][0];
        assert_eq!(feed["sender"], agent.as_str(), "{body}");
        assert!(feed["state"].is_string(), "{body}");
        assert!(feed["window"].is_string(), "{body}");
        assert!(feed["thresholds"].is_object(), "{body}");
        assert!(feed["last_message_at"].is_null(), "no message was sent");
        health_loop.abort();
    })
    .await;
}
