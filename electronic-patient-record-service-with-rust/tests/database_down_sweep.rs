//! DB-gated, through the real router with login off: with the database gone (the pool closed), every route that needs
//! it answers 503 "unavailable", never a plain 500 and never a panic (the same rule `store::unavailable` states). The
//! routes come from the served OpenAPI document, so a route added later is swept without a line here. A write route
//! with an empty body answers 422 before it reaches the database, so the sweep sends a body each route may refuse;
//! what it checks is that nothing answers 500.
//!
//! ```text
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test main database_down_sweep:: -- --ignored --test-threads=1
//! ```

use axa_test::prelude::request;
use electronic_patient_record::app::App;
use electronic_patient_record::pool::Pool;
use serde_json::{Value, json};

/// A value that satisfies `schema`: required fields only, the first enum value, a format-shaped string. Not meant to
/// be clinically sensible, only to get past the body's structure to what the route does next.
fn sample(doc: &Value, schema: &Value, depth: usize) -> Value {
    if let Some(r) = schema.get("$ref").and_then(Value::as_str) {
        let name = r.rsplit('/').next().unwrap_or("");
        return sample(doc, &doc["components"]["schemas"][name], depth);
    }
    if depth > 6 {
        return Value::Null;
    }
    for key in ["oneOf", "anyOf", "allOf"] {
        if let Some(first) = schema.get(key).and_then(|v| v.get(0)) {
            return sample(doc, first, depth + 1);
        }
    }
    if let Some(first) = schema.get("enum").and_then(|v| v.get(0)) {
        return first.clone();
    }
    let kind = match schema.get("type") {
        Some(Value::String(t)) => t.as_str(),
        Some(Value::Array(a)) => a
            .iter()
            .filter_map(Value::as_str)
            .find(|t| *t != "null")
            .unwrap_or("null"),
        _ => "object",
    };
    match kind {
        "string" => match schema.get("format").and_then(Value::as_str) {
            Some("date") => json!("1990-01-01"),
            Some("date-time") => json!("2026-11-24T09:00:00Z"),
            Some("uuid") => json!("0c4f1e2a-0000-4000-8000-0000000000aa"),
            _ => json!("x"),
        },
        "integer" => json!(schema.get("minimum").and_then(Value::as_i64).unwrap_or(1)),
        "number" => json!(1.0),
        "boolean" => json!(true),
        "array" => {
            let min = schema.get("minItems").and_then(Value::as_u64).unwrap_or(0);
            match schema.get("items") {
                Some(items) if min > 0 => json!([sample(doc, items, depth + 1)]),
                _ => json!([]),
            }
        }
        "null" => Value::Null,
        _ => {
            let mut object = serde_json::Map::new();
            let required: Vec<&str> = schema
                .get("required")
                .and_then(Value::as_array)
                .map_or_else(Vec::new, |a| a.iter().filter_map(Value::as_str).collect());
            for name in required {
                let property = &schema["properties"][name];
                object.insert(name.to_string(), sample(doc, property, depth + 1));
            }
            Value::Object(object)
        }
    }
}

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
                let body = item[method]["requestBody"]["content"]["application/json"]["schema"]
                    .as_object()
                    .map_or_else(|| json!({}), |_| {
                        sample(&doc, &item[method]["requestBody"]["content"]["application/json"]["schema"], 0)
                    });
                let r = req.json(&body).await;
                let s = r.status_code().as_u16();
                if s == 503 {
                    unavailable += 1;
                } else if s >= 500 {
                    bad.push(format!("{method} {path}: {s}"));
                }
            }
        }
        assert!(bad.is_empty(), "answered 5xx but not 503: {bad:#?}");
        assert!(unavailable >= 60, "only {unavailable} routes answered 503");
    })
    .await;
}
