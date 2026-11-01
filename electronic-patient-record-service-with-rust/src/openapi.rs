//! The OpenAPI document (task OS-7), served at `GET /api/openapi.json` and kept in
//! `openapi.json` at the crate root.
//!
//! It is **derived**, never written by hand: the paths and methods are the routes the service
//! really serves ([`crate::controllers::all_routes`]), and each operation carries the resource and
//! action of its row in the policy table ([`crate::routes::TABLE`]). A route that is served but
//! missing from the document is therefore not possible, and `tests/openapi_census.rs` fails when
//! the committed copy differs from what this module produces.
//!
//! **What it describes of bodies:** only the error bodies, which `api_error.rs` builds in one place
//! (`components.schemas.Error`, `ErrorList` and `OperationOutcome`, referenced by the 401 and 403
//! responses; a unit test checks real `ApiError` bodies against the required keys). **What it does
//! not describe:** any request body or success response, so a client cannot be generated from it.
//! It is an inventory of the surface with its authentication and authorization requirements, not
//! a contract. FHIR resource shapes are in `/fhir/R4/metadata`.

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

/// The media type and schema of an error body on `path`: an `OperationOutcome` under `/fhir`.
fn error_ref(path: &str) -> (&'static str, Value) {
    if path.starts_with("/fhir/") {
        (
            "application/fhir+json",
            json!({"$ref": "#/components/schemas/OperationOutcome"}),
        )
    } else {
        (
            "application/json",
            json!({"$ref": "#/components/schemas/Error"}),
        )
    }
}

/// The error schemas, matching `api_error.rs`.
fn error_schemas() -> Value {
    json!({
        "Error": {
            "type": "object", "required": ["error"],
            "properties": {"error": {"type": "string"}},
            "additionalProperties": true,
            "description": "api_error.rs: fixed wording; extra fields such as current_version or break_glass_offer may follow."
        },
        "ErrorList": {
            "type": "object", "required": ["errors"],
            "properties": {"errors": {"type": "array", "items": {"type": "string"}}},
            "additionalProperties": true,
            "description": "422: the record was refused, one line per reason."
        },
        "OperationOutcome": {
            "type": "object", "required": ["resourceType", "issue"],
            "properties": {
                "resourceType": {"const": "OperationOutcome"},
                "issue": {"type": "array", "items": {"type": "object"}}
            },
            "additionalProperties": true
        }
    })
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
            let error = error_ref(path);
            responses.insert(
                "401".into(),
                json!({"description": "No valid token.", "content": {error.0: {"schema": error.1.clone()}}}),
            );
            responses.insert(
                "403".into(),
                json!({"description": "The policy engine refused the request; the refusal is audited.",
                       "content": {error.0: {"schema": error.1}}}),
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
            "description": "NOT FOR CLINICAL USE. An inventory of the served routes with their authentication and policy requirements, derived from the code. Only error bodies are described; request bodies and success responses are not. The API is versioned by the Accepts-version header; no path carries a version segment.",
            "x-epr-derived-from": ["controllers::all_routes", "routes::TABLE"]
        },
        "paths": paths,
        "components": {
            "securitySchemes": {"bearerAuth": {"type": "http", "scheme": "bearer"}},
            "schemas": error_schemas()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api_error::ApiError;

    fn required(name: &str) -> Vec<String> {
        error_schemas()[name]["required"]
            .as_array()
            .expect("required list")
            .iter()
            .map(|v| v.as_str().expect("key").to_string())
            .collect()
    }

    #[test]
    fn real_error_bodies_carry_the_keys_the_schemas_require() {
        let plain = ApiError::forbidden().with("break_glass_offer", true).body();
        for key in required("Error") {
            assert!(plain.get(&key).is_some(), "Error body lacks {key}");
        }
        let list = ApiError::invalid(vec!["x".into()]).body();
        for key in required("ErrorList") {
            assert!(list.get(&key).is_some(), "ErrorList body lacks {key}");
        }
    }

    #[test]
    fn every_error_response_points_at_a_schema_that_exists() {
        let doc = document();
        let schemas = doc["components"]["schemas"].as_object().expect("schemas");
        let mut seen = 0;
        for (_, methods) in doc["paths"].as_object().expect("paths") {
            for (_, op) in methods.as_object().expect("methods") {
                for code in ["401", "403"] {
                    let Some(r) = op["responses"].get(code) else {
                        continue;
                    };
                    let reference = r["content"]
                        .as_object()
                        .and_then(|c| c.values().next())
                        .and_then(|m| m["schema"]["$ref"].as_str())
                        .expect("a $ref");
                    let name = reference.rsplit('/').next().unwrap();
                    assert!(schemas.contains_key(name), "{reference} is not defined");
                    seen += 1;
                }
            }
        }
        assert!(seen > 0);
    }
}
