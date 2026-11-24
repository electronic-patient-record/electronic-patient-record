//! DB-gated, through the real router with login off: with the database gone (the pool closed), every route that needs
//! it answers 503 "unavailable", never a plain 500 and never a panic (the same rule `store::unavailable` states). The
//! routes come from the served OpenAPI document, so a route added later is swept without a line here. A write route
//! with an empty body answers 422 before it reaches the database, so the sweep sends a body each route may refuse;
//! what it checks is that nothing answers 500.
//!
//! ```text
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test suite database_down_sweep:: -- --ignored --test-threads=1
//! ```

use axa_test::prelude::request;
use electronic_patient_record::app::App;
use electronic_patient_record::pool::Pool;
use serde_json::Value;

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn no_route_answers_500_when_the_database_is_gone() {
    let doc: Value = serde_json::from_str(include_str!("../openapi.json")).unwrap();
    request::<App, _, _>(|server, ctx| async move {
        ctx.pool().close().await;
        let id = "0c4f1e2a-0000-4000-8000-0000000000aa";
        let (mut unavailable, mut bad) = (0, Vec::new());
        for (path, item) in doc["paths"].as_object().unwrap() {
            let mut url = path.clone();
            while let (Some(a), Some(b)) = (url.find('{'), url.find('}')) {
                url.replace_range(a..=b, id);
            }
            for method in ["get", "post", "put", "patch", "delete"] {
                if item.get(method).is_none() {
                    continue;
                }
                let req = match method {
                    "get" => server.get(&url),
                    "post" => server.post(&url),
                    "put" => server.put(&url),
                    "patch" => server.patch(&url),
                    _ => server.delete(&url),
                };
                let r = req.json(&serde_json::json!({})).await;
                let s = r.status_code().as_u16();
                if s == 503 {
                    unavailable += 1;
                } else if s >= 500 {
                    bad.push(format!("{method} {path}: {s}"));
                }
            }
        }
        assert!(bad.is_empty(), "answered 5xx but not 503: {bad:#?}");
        assert!(unavailable >= 10, "only {unavailable} routes answered 503");
    })
    .await;
}
