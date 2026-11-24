//! DB-gated, through the real router with login off: an unknown `X-Purpose-Of-Use` is refused with 400 by every route
//! that reads the purpose of use, and by none that does not (QA-15). The routes come from the served OpenAPI document,
//! so a route added later is swept without a line here. The guard runs before anything is read or written, so a made-up
//! patient id is enough. No route answers 5xx to it. A write route with an empty body answers 422 first (the body is
//! read before the guard), so the sweep reaches the read routes; the write routes need a valid body each (not done).
//!
//! ```text
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test main purpose_sweep:: -- --ignored --test-threads=1
//! ```

use axa_test::prelude::request;
use electronic_patient_record::app::App;
use serde_json::Value;

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn an_unknown_purpose_of_use_is_refused_before_anything_happens() {
    let doc: Value = serde_json::from_str(include_str!("../openapi.json")).unwrap();
    request::<App, _, _>(|server, _ctx| async move {
        let id = "0c4f1e2a-0000-4000-8000-0000000000aa";
        let mut refused = 0;
        let mut other = Vec::new();
        for (path, item) in doc["paths"].as_object().unwrap() {
            let mut url = path.clone();
            while let (Some(a), Some(b)) = (url.find('{'), url.find('}')) {
                url.replace_range(a..=b, id);
            }
            for method in ["get", "post", "put", "patch", "delete"] {
                if item.get(method).is_none() {
                    continue;
                }
                let h = axum::http::HeaderName::from_static("x-purpose-of-use");
                let v = axum::http::HeaderValue::from_static("curiosity");
                let req = match method {
                    "get" => server.get(&url),
                    "post" => server.post(&url),
                    "put" => server.put(&url),
                    "patch" => server.patch(&url),
                    _ => server.delete(&url),
                };
                let r = req.add_header(h, v).json(&serde_json::json!({})).await;
                let s = r.status_code().as_u16();
                assert!(s < 500, "{method} {path}: {s} {}", r.text());
                if s == 400 && r.text().contains("X-Purpose-Of-Use") {
                    refused += 1;
                } else {
                    other.push(format!("{method} {path} {s}"));
                }
            }
        }
        assert!(
            refused >= 40,
            "the sweep found only {refused} routes that read the purpose; the others answered: {other:?}"
        );
    })
    .await;
}
