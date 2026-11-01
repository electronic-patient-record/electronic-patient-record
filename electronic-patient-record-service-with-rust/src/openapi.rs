//! The OpenAPI document (task OS-7), served at `GET /api/openapi.json` and kept in
//! `openapi.json` at the crate root.
//!
//! It is **derived**, never written by hand: the paths and methods are the routes the service
//! really serves ([`crate::controllers::all_routes`]), and each operation carries the resource and
//! action of its row in the policy table ([`crate::routes::TABLE`]). A route that is served but
//! missing from the document is therefore not possible, and `tests/openapi_census.rs` fails when
//! the committed copy differs from what this module produces.
//!
//! **What it describes of bodies:** the error bodies, which `api_error.rs` builds in one place
//! (`components.schemas.Error`, `ErrorList` and `OperationOutcome`, referenced by the 401 and 403
//! responses; a unit test checks real `ApiError` bodies against the required keys). **What it does
//! not describe:** any request body or success response except the patient create and update pair below, so a client cannot be generated from it.
//! The patient identity record (`Patient`, `Stored`, `CreatePatient`, `UpdatePatient`) is described for
//! `POST /api/patients` and `PUT /api/patients/{id}`; unit tests compare each schema's property names
//! with the fields the real types serialise, in both directions. It is an inventory of the surface with its authentication and authorization requirements, not
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

/// The patient identity schemas, matching the matcher's `Patient`, `patients_store::Stored` and the
/// request types in `controllers/patients.rs`.
fn patient_schemas() -> Value {
    let pair = json!({"type": "object", "required": ["system", "value"],
        "properties": {"system": {"type": "string"}, "value": {"type": "string"}}});
    json!({
        "Patient": {
            "type": "object",
            "description": "The thin identity record (the matcher's Patient). Absent evidence is null or empty, never zero.",
            "properties": {
                "id": {"type": ["string", "null"]},
                "names": {"type": "array", "items": {"type": "object", "properties": {
                    "family": {"type": ["string", "null"]},
                    "given": {"type": "array", "items": {"type": "string"}}}}},
                "birth_date": {"type": ["string", "null"], "description": "ISO 8601; may be partial (YYYY, YYYY-MM, YYYY-MM-DD)."},
                "sex": {"enum": ["male", "female", "other", "unknown", null]},
                "identifiers": {"type": "array", "items": pair.clone()},
                "addresses": {"type": "array", "items": {"type": "object", "properties": {
                    "postal_code": {"type": ["string", "null"]}}}},
                "telecom": {"type": "array", "items": pair}
            }
        },
        "Stored": {
            "type": "object", "required": ["id", "version", "author", "patient"],
            "properties": {
                "id": {"type": "string", "description": "Server-assigned UUID."},
                "version": {"type": "integer", "description": "1 for the first version."},
                "author": {"type": "string"},
                "patient": {"$ref": "#/components/schemas/Patient"},
                "drill": {"type": "boolean"}
            }
        },
        "CreatePatient": {
            "type": "object", "required": ["patient"],
            "properties": {
                "patient": {"$ref": "#/components/schemas/Patient"},
                "reason": {"type": ["string", "null"]},
                "override_reason": {"type": ["string", "null"], "description": "Register despite likely duplicates (409 otherwise); a reason is required."},
                "drill": {"type": "boolean"}
            }
        },
        "PatientView": {
            "type": "object",
            "required": ["id", "version", "author", "patient", "replaces", "site_time_zone"],
            "description": "A patient as read: the stored version plus the photo version, merge status and the site time zone.",
            "properties": {
                "id": {"type": "string"},
                "version": {"type": "integer"},
                "author": {"type": "string"},
                "patient": {"$ref": "#/components/schemas/Patient"},
                "drill": {"type": "boolean"},
                "photo_version": {"type": ["integer", "null"], "description": "Null when there is no photo or it was withdrawn."},
                "replaced_by": {"type": ["string", "null"], "description": "The surviving record, when this one was merged away."},
                "replaces": {"type": "array", "items": {"type": "string"}},
                "site_time_zone": {"type": "string"}
            }
        },
        "PatientSearchResults": {
            "type": "object", "required": ["results", "limit"],
            "properties": {
                "results": {"type": "array", "items": {"$ref": "#/components/schemas/Stored"}},
                "limit": {"type": "integer", "description": "The most results returned."}
            }
        },
        "AllergyRecord": {
            "type": "object", "required": ["substance", "status"],
            "description": "One allergy as recorded. An absent optional field means not stated; criticality is never defaulted to low.",
            "properties": {
                "substance": {"type": "string", "maxLength": 200, "description": "In words; required."},
                "coding": {"type": "object", "required": ["system", "code"], "properties": {
                    "system": {"type": "string", "description": "A URI; a SNOMED CT code is checked."},
                    "code": {"type": "string", "maxLength": 64}}},
                "category": {"enum": ["food", "medication", "environment", "biologic"]},
                "criticality": {"enum": ["low", "high", "unable-to-assess"]},
                "reaction": {"type": "string", "maxLength": 500},
                "onset": {"type": "string", "description": "YYYY, YYYY-MM or YYYY-MM-DD."},
                "status": {"enum": ["active", "inactive", "entered-in-error"]}
            }
        },
        "StoredAllergy": {
            "type": "object",
            "required": ["id", "patient_id", "version", "author", "recorded_at", "allergy"],
            "properties": {
                "id": {"type": "string"}, "patient_id": {"type": "string"},
                "version": {"type": "integer"}, "author": {"type": "string"},
                "recorded_at": {"type": "string", "description": "UTC."},
                "allergy": {"$ref": "#/components/schemas/AllergyRecord"}
            }
        },
        "CreateAllergy": {
            "type": "object", "required": ["allergy"],
            "properties": {
                "allergy": {"$ref": "#/components/schemas/AllergyRecord"},
                "reason": {"type": ["string", "null"]}
            }
        },
        "UpdateAllergy": {
            "type": "object", "required": ["allergy", "expected_version"],
            "properties": {
                "allergy": {"$ref": "#/components/schemas/AllergyRecord"},
                "expected_version": {"type": "integer"},
                "reason": {"type": ["string", "null"]}
            }
        },
        "AllergyList": {
            "type": "object", "required": ["allergies", "statement", "summary", "amendment_notices"],
            "description": "Every allergy in every status, the no-known-allergies statement, the banner summary and amendment notices by allergy id. The statement, summary and notices are described only as objects.",
            "properties": {
                "allergies": {"type": "array", "items": {"$ref": "#/components/schemas/StoredAllergy"}},
                "statement": {"type": ["object", "null"]},
                "summary": {"type": "object", "required": ["state"], "properties": {"state": {"type": "string"}}},
                "amendment_notices": {"type": "object"}
            }
        },
        "DuplicateConflict": {
            "type": "object", "required": ["error", "candidates", "candidates_cut"],
            "description": "409 on create: likely duplicates. A candidate the caller may not see is {\"restricted\": true}.",
            "properties": {
                "error": {"type": "string"},
                "candidates": {"type": "array", "items": {"type": "object"}},
                "candidates_cut": {"type": "boolean"}
            }
        },
        "VersionConflict": {
            "type": "object", "required": ["error", "current_version"],
            "description": "409 on update: expected_version is not the current one.",
            "properties": {"error": {"type": "string"}, "current_version": {"type": "integer"}}
        },
        "UpdatePatient": {
            "type": "object", "required": ["patient", "expected_version"],
            "properties": {
                "patient": {"$ref": "#/components/schemas/Patient"},
                "expected_version": {"type": "integer", "description": "Any other current version is a 409."},
                "reason": {"type": ["string", "null"]}
            }
        }
    })
}

/// `(method, path, request schema, success status, success schema, 409 schema)` for the routes
/// whose bodies are described; an empty 409 schema means the route has none. Each also answers 422
/// with `ErrorList`.
const BODIES: &[(&str, &str, &str, &str, &str, &str)] = &[
    (
        "POST",
        "/api/patients",
        "CreatePatient",
        "201",
        "Stored",
        "DuplicateConflict",
    ),
    (
        "PUT",
        "/api/patients/{id}",
        "UpdatePatient",
        "200",
        "Stored",
        "VersionConflict",
    ),
    (
        "POST",
        "/api/patients/{id}/allergies",
        "CreateAllergy",
        "201",
        "StoredAllergy",
        "",
    ),
    (
        "PUT",
        "/api/patients/{id}/allergies/{allergy_id}",
        "UpdateAllergy",
        "200",
        "StoredAllergy",
        "VersionConflict",
    ),
];

/// `(method, path, success schema, query parameters)` for the described reads.
const READS: &[(&str, &str, &str, &[&str])] = &[
    ("GET", "/api/patients/{id}", "PatientView", &[]),
    ("GET", "/api/patients/{id}/allergies", "AllergyList", &[]),
    (
        "GET",
        "/api/patients",
        "PatientSearchResults",
        &["family", "birth_date", "identifier", "drill"],
    ),
];

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

fn all_schemas() -> Value {
    let mut all = error_schemas();
    all.as_object_mut()
        .expect("an object")
        .extend(patient_schemas().as_object().expect("an object").clone());
    all
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
        if let Some((_, _, request, status, success, conflict)) =
            BODIES.iter().find(|(m, p, ..)| m == method && p == path)
        {
            op.insert(
                "requestBody".into(),
                json!({"required": true, "content": {"application/json": {
                    "schema": {"$ref": format!("#/components/schemas/{request}")}}}}),
            );
            if !conflict.is_empty() {
                responses.insert(
                    "409".into(),
                    json!({"description": "A likely duplicate or a stale expected_version.",
                           "content": {"application/json": {"schema": {"$ref": format!("#/components/schemas/{conflict}")}}}}),
                );
            }
            responses.insert(
                "422".into(),
                json!({"description": "The record was refused, one line per reason.",
                       "content": {"application/json": {"schema": {"$ref": "#/components/schemas/ErrorList"}}}}),
            );
            responses.insert(
                (*status).into(),
                json!({"description": "The stored patient.", "content": {"application/json": {
                    "schema": {"$ref": format!("#/components/schemas/{success}")}}}}),
            );
        }
        if let Some((_, _, success, query)) =
            READS.iter().find(|(m, p, ..)| m == method && p == path)
        {
            responses.insert(
                "200".into(),
                json!({"description": "OK.", "content": {"application/json": {
                    "schema": {"$ref": format!("#/components/schemas/{success}")}}}}),
            );
            let mut parameters = op
                .get("parameters")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            for name in *query {
                let schema = if *name == "drill" {
                    json!({"type": "boolean"})
                } else {
                    json!({"type": "string"})
                };
                parameters.push(
                    json!({"name": name, "in": "query", "required": false, "schema": schema}),
                );
            }
            if !parameters.is_empty() {
                op.insert("parameters".into(), Value::Array(parameters));
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
            "schemas": all_schemas()
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

    fn properties(name: &str) -> std::collections::BTreeSet<String> {
        patient_schemas()[name]["properties"]
            .as_object()
            .expect("properties")
            .keys()
            .cloned()
            .collect()
    }

    fn keys(value: &Value) -> std::collections::BTreeSet<String> {
        value
            .as_object()
            .expect("an object")
            .keys()
            .cloned()
            .collect()
    }

    fn full_patient() -> electronic_patient_record_matcher::Patient {
        use electronic_patient_record_matcher::{
            Address, AdministrativeSex, HumanName, Identifier, Patient, Telecom,
        };
        Patient {
            id: Some("i".into()),
            names: vec![HumanName {
                family: Some("f".into()),
                given: vec!["g".into()],
            }],
            birth_date: Some("2000".into()),
            sex: Some(AdministrativeSex::Other),
            identifiers: vec![Identifier {
                system: "s".into(),
                value: "v".into(),
            }],
            addresses: vec![Address {
                postal_code: Some("p".into()),
            }],
            telecom: vec![Telecom {
                system: "s".into(),
                value: "v".into(),
            }],
        }
    }

    #[test]
    fn the_patient_schemas_name_exactly_the_fields_the_types_serialise() {
        let patient = serde_json::to_value(full_patient()).unwrap();
        assert_eq!(keys(&patient), properties("Patient"));
        let schema = patient_schemas();
        let inner = |field: &str, of: &str| -> std::collections::BTreeSet<String> {
            schema["Patient"]["properties"][field]["items"][of]
                .as_object()
                .expect("item properties")
                .keys()
                .cloned()
                .collect()
        };
        assert_eq!(keys(&patient["names"][0]), inner("names", "properties"));
        assert_eq!(
            keys(&patient["addresses"][0]),
            inner("addresses", "properties")
        );
        assert_eq!(
            keys(&patient["identifiers"][0]),
            inner("identifiers", "properties")
        );
        assert_eq!(keys(&patient["telecom"][0]), inner("telecom", "properties"));

        let stored = serde_json::to_value(crate::patients_store::Stored {
            id: "x".into(),
            version: 1,
            author: "a".into(),
            patient: full_patient(),
            drill: false,
        })
        .unwrap();
        assert_eq!(keys(&stored), properties("Stored"));
    }

    #[test]
    fn the_request_schemas_name_exactly_the_fields_the_handlers_read() {
        use crate::controllers::patients::{CreateRequest, UpdateRequest};
        let create: CreateRequest = serde_json::from_value(serde_json::json!({
            "patient": {}, "reason": "r", "override_reason": "o", "drill": true
        }))
        .unwrap();
        assert!(create.drill);
        let update: UpdateRequest = serde_json::from_value(serde_json::json!({
            "patient": {}, "expected_version": 1, "reason": "r"
        }))
        .unwrap();
        assert_eq!(update.expected_version, 1);
        // The required keys are the ones the types cannot default.
        assert!(serde_json::from_value::<CreateRequest>(serde_json::json!({})).is_err());
        assert!(
            serde_json::from_value::<UpdateRequest>(serde_json::json!({"patient": {}})).is_err()
        );
        let schema = patient_schemas();
        assert_eq!(
            schema["CreatePatient"]["required"],
            serde_json::json!(["patient"])
        );
        assert_eq!(
            schema["UpdatePatient"]["required"],
            serde_json::json!(["patient", "expected_version"])
        );
        assert_eq!(
            properties("CreatePatient"),
            ["patient", "reason", "override_reason", "drill"]
                .map(String::from)
                .into()
        );
        assert_eq!(
            properties("UpdatePatient"),
            ["patient", "expected_version", "reason"]
                .map(String::from)
                .into()
        );
    }

    #[test]
    fn the_conflict_schemas_match_what_the_service_sends() {
        let version =
            ApiError::from(&crate::patients_store::StoreError::Conflict { current: 3 }).body();
        let schema = patient_schemas();
        for key in schema["VersionConflict"]["required"].as_array().unwrap() {
            assert!(version.get(key.as_str().unwrap()).is_some(), "lacks {key}");
        }
        let duplicate = ApiError::conflict("possible duplicate")
            .with("candidates", serde_json::json!([]))
            .with("candidates_cut", false)
            .body();
        for key in schema["DuplicateConflict"]["required"].as_array().unwrap() {
            assert!(
                duplicate.get(key.as_str().unwrap()).is_some(),
                "lacks {key}"
            );
        }
    }

    #[test]
    fn the_read_schemas_name_exactly_the_fields_the_service_sends() {
        use crate::controllers::patients::PatientView;
        use crate::patients_store::{SEARCH_LIMIT, Search, Stored};
        let stored = Stored {
            id: "x".into(),
            version: 1,
            author: "a".into(),
            patient: full_patient(),
            drill: false,
        };
        let view = serde_json::to_value(PatientView {
            stored: stored.clone(),
            photo_version: Some(1),
            replaced_by: Some("y".into()),
            replaces: vec!["z".into()],
            site_time_zone: "UTC".into(),
        })
        .unwrap();
        assert_eq!(keys(&view), properties("PatientView"));
        let results = serde_json::json!({"results": [stored], "limit": SEARCH_LIMIT});
        assert_eq!(keys(&results), properties("PatientSearchResults"));
        // Every documented query parameter is one `Search` reads, and each is accepted.
        let (_, _, _, query) = READS.iter().find(|r| r.1 == "/api/patients").unwrap();
        let all: serde_json::Map<String, Value> = query
            .iter()
            .map(|k| {
                (
                    (*k).to_string(),
                    if *k == "drill" {
                        true.into()
                    } else {
                        "v".into()
                    },
                )
            })
            .collect();
        let parsed: Search = serde_json::from_value(Value::Object(all)).unwrap();
        assert!(parsed.drill && parsed.family.is_some() && parsed.birth_date.is_some());
        assert!(parsed.identifier.is_some());
    }

    #[test]
    fn the_allergy_schemas_match_the_types() {
        use crate::allergies::{
            AllergyRecord, Category, Coding, Criticality, Status, StoredAllergy,
        };
        let schema = patient_schemas();
        let record = AllergyRecord {
            substance: "s".into(),
            coding: Some(Coding {
                system: "u".into(),
                code: "c".into(),
            }),
            category: Some(Category::Food),
            criticality: Some(Criticality::UnableToAssess),
            reaction: Some("r".into()),
            onset: Some("2000".into()),
            status: Status::EnteredInError,
        };
        let value = serde_json::to_value(&record).unwrap();
        assert_eq!(keys(&value), properties("AllergyRecord"));
        assert_eq!(
            keys(&value["coding"]),
            schema["AllergyRecord"]["properties"]["coding"]["properties"]
                .as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect()
        );
        // The enum words are the ones serde writes, variant by variant.
        let words = |name: &str| -> Vec<Value> {
            schema["AllergyRecord"]["properties"][name]["enum"]
                .as_array()
                .unwrap()
                .clone()
        };
        let said = |v: &[Value]| v.to_vec();
        assert_eq!(
            words("category"),
            said(
                &[
                    Category::Food,
                    Category::Medication,
                    Category::Environment,
                    Category::Biologic
                ]
                .map(|c| serde_json::to_value(c).unwrap())
            )
        );
        assert_eq!(
            words("criticality"),
            said(
                &[
                    Criticality::Low,
                    Criticality::High,
                    Criticality::UnableToAssess
                ]
                .map(|c| serde_json::to_value(c).unwrap())
            )
        );
        assert_eq!(
            words("status"),
            said(
                &[Status::Active, Status::Inactive, Status::EnteredInError]
                    .map(|c| serde_json::to_value(c).unwrap())
            )
        );
        // A bare record omits what was not stated, and still has the required keys.
        let bare = serde_json::to_value(AllergyRecord {
            substance: "s".into(),
            coding: None,
            category: None,
            criticality: None,
            reaction: None,
            onset: None,
            status: Status::Active,
        })
        .unwrap();
        for key in schema["AllergyRecord"]["required"].as_array().unwrap() {
            assert!(bare.get(key.as_str().unwrap()).is_some());
        }
        let stored = serde_json::to_value(StoredAllergy {
            id: "i".into(),
            patient_id: "p".into(),
            version: 1,
            author: "a".into(),
            recorded_at: "t".into(),
            allergy: record,
        })
        .unwrap();
        assert_eq!(keys(&stored), properties("StoredAllergy"));
        let create: crate::controllers::allergies::CreateRequest =
            serde_json::from_value(serde_json::json!({"allergy": bare, "reason": "r"})).unwrap();
        assert!(create.reason.is_some());
        assert!(
            serde_json::from_value::<crate::controllers::allergies::UpdateRequest>(
                serde_json::json!({"allergy": bare})
            )
            .is_err()
        );
    }

    #[test]
    fn the_allergy_list_schema_names_the_fields_the_handler_sends() {
        use crate::allergies::Summary;
        use crate::controllers::allergies::AllergyList;
        let list = serde_json::to_value(AllergyList {
            allergies: vec![],
            statement: None,
            summary: Summary::Active { items: vec![] },
            amendment_notices: std::collections::BTreeMap::new(),
        })
        .unwrap();
        assert_eq!(keys(&list), properties("AllergyList"));
        assert_eq!(list["summary"]["state"], "active");
        assert!(list["statement"].is_null());
    }
}
