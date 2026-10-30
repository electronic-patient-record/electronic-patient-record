//! The OpenAPI document (task OS-7), served at `GET /api/openapi.json` and kept in
//! `openapi.json` at the crate root.
//!
//! It is **derived**, never written by hand: the paths and methods are the routes the service
//! really serves ([`crate::controllers::all_routes`]), and each operation carries the resource and
//! action of its row in the policy table ([`crate::routes::TABLE`]). A route that is served but
//! missing from the document is therefore not possible, and `tests/openapi_census.rs` fails when
//! the committed copy differs from what this module produces.
//!
//! **What it does not describe:** request and response bodies. No schema is stated, so a client
//! cannot be generated from it. It is an inventory of the surface with its authentication and
//! authorization requirements, not a contract. FHIR resource shapes are in `/fhir/R4/metadata`.

use crate::{auth, controllers, policy::Resource, routes};
use axum::{Json, response::Response};
use loco_rs::prelude::*;
use serde_json::{Map, Value, json};

/// The served `(method, path template)` pairs, normalised to `{name}` segments.
#[must_use]
pub fn served() -> Vec<(String, String)> {
    controllers::all_routes()
        .collect()
        .into_iter()
        .flat_map(|route| {
            let uri = routes::normalise(&route.uri);
            route
                .actions
                .into_iter()
                .map(move |m| (m.to_string(), uri.clone()))
        })
        .collect()
}

fn operation_id(method: &str, path: &str) -> String {
    let words: Vec<String> = path
        .split('/')
        .filter(|s| !s.is_empty())
        .map(|s| {
            let s = s.trim_start_matches('{').trim_end_matches('}');
            let mut c = s.chars();
            c.next().map_or_else(String::new, |f| {
                f.to_uppercase().collect::<String>() + c.as_str()
            })
        })
        .collect();
    format!("{}{}", method.to_ascii_lowercase(), words.concat())
}

fn tag(path: &str) -> String {
    let mut parts = path.split('/').filter(|s| !s.is_empty());
    match (parts.next(), parts.next()) {
        (Some("api"), Some(second)) => second.to_string(),
        (Some("fhir"), Some(release)) => format!("fhir-{}", release.to_ascii_lowercase()),
        (Some(first), _) => first.trim_start_matches('_').to_string(),
        _ => "root".to_string(),
    }
}

fn path_parameters(path: &str) -> Vec<Value> {
    path.split('/')
        .filter_map(|s| s.strip_prefix('{').and_then(|s| s.strip_suffix('}')))
        .map(|name| {
            json!({"name": name, "in": "path", "required": true, "schema": {"type": "string"}})
        })
        .collect()
}

fn resource_name(resource: Resource) -> String {
    format!("{resource:?}")
}

/// The document for the given routes. Pure, so a test can plant a route and see it appear.
#[must_use]
pub fn document_for(served: &[(String, String)]) -> Value {
    let mut paths: Map<String, Value> = Map::new();
    for (method, path) in served {
        let public = auth::is_public_path(path);
        let mut op = Map::new();
        op.insert("operationId".into(), json!(operation_id(method, path)));
        op.insert("summary".into(), json!(format!("{method} {path}")));
        op.insert("tags".into(), json!([tag(path)]));
        let params = path_parameters(path);
        if !params.is_empty() {
            op.insert("parameters".into(), Value::Array(params));
        }
        let mut responses = Map::new();
        responses.insert(
            "default".into(),
            json!({"description": "Bodies are not described in this document; see spec/09-api-surface.md."}),
        );
        if public {
            op.insert("security".into(), json!([]));
            op.insert("x-epr-public".into(), json!(true));
        } else {
            op.insert("security".into(), json!([{"bearerAuth": []}]));
            responses.insert("401".into(), json!({"description": "No valid token."}));
            responses.insert(
                "403".into(),
                json!({"description": "The policy engine refused the request; the refusal is audited."}),
            );
            let m = axum::http::Method::from_bytes(method.as_bytes());
            if let Some((_, req)) = m.ok().and_then(|m| routes::lookup(&m, path)) {
                op.insert("x-epr-resource".into(), json!(resource_name(req.resource)));
                op.insert("x-epr-action".into(), json!(format!("{:?}", req.action)));
            }
        }
        op.insert("responses".into(), Value::Object(responses));
        paths
            .entry(path.clone())
            .or_insert_with(|| Value::Object(Map::new()))
            .as_object_mut()
            .expect("an object was just inserted")
            .insert(method.to_ascii_lowercase(), Value::Object(op));
    }
    json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Electronic Patient Record service",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "NOT FOR CLINICAL USE. An inventory of the served routes with their authentication and policy requirements, derived from the code. Request and response bodies are not described. The API is versioned by the Accepts-version header; no path carries a version segment.",
            "x-epr-derived-from": ["controllers::all_routes", "routes::TABLE"]
        },
        "paths": paths,
        "components": {
            "securitySchemes": {"bearerAuth": {"type": "http", "scheme": "bearer"}}
        }
    })
}

/// The document for the routes this build serves.
#[must_use]
pub fn document() -> Value {
    document_for(&served())
}

/// The document as committed: pretty JSON with a final newline.
#[must_use]
pub fn rendered() -> String {
    let mut text = serde_json::to_string_pretty(&document()).expect("a Value always serialises");
    text.push('\n');
    text
}

async fn serve() -> Response {
    Json(document()).into_response()
}

/// The route.
#[must_use]
pub fn routes() -> Routes {
    Routes::new().add("/api/openapi.json", get(serve))
}
