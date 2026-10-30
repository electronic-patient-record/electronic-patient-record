//! OpenAPI census (OS-7): the committed `openapi.json` is exactly what the code produces, every
//! served route is in it with its policy requirement, public routes say so, and no path carries a
//! version segment (the API is versioned by the `Accepts-version` header).
//!
//! To accept a deliberate change: `EPR_BLESS=1 cargo test --test openapi_census`, then review the diff.

use electronic_patient_record::{auth, openapi, routes};
use std::{fs, path::Path};

const FILE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/openapi.json");

#[test]
fn the_committed_document_is_what_the_code_produces() {
    let produced = openapi::rendered();
    if std::env::var("EPR_BLESS").as_deref() == Ok("1") {
        fs::write(FILE, &produced).expect("write openapi.json");
        return;
    }
    let committed = fs::read_to_string(Path::new(FILE)).expect(
        "openapi.json is missing: run `EPR_BLESS=1 cargo test --test openapi_census` and commit it",
    );
    assert!(
        committed == produced,
        "openapi.json is stale: run `EPR_BLESS=1 cargo test --test openapi_census`, review and commit"
    );
}

#[test]
fn every_served_route_is_in_the_document_with_its_requirement() {
    let doc = openapi::document();
    let served = openapi::served();
    assert!(!served.is_empty(), "the census saw no routes");
    for (method, path) in &served {
        let op = &doc["paths"][path][method.to_ascii_lowercase()];
        assert!(op.is_object(), "{method} {path} is not in the document");
        if auth::is_public_path(path) {
            assert_eq!(op["x-epr-public"], true, "{method} {path}");
            assert_eq!(op["security"], serde_json::json!([]), "{method} {path}");
        } else {
            let (_, req) = routes::lookup(&method.parse().unwrap(), path)
                .unwrap_or_else(|| panic!("{method} {path} has no policy row"));
            assert_eq!(
                op["x-epr-resource"],
                format!("{:?}", req.resource),
                "{method} {path}"
            );
            assert_eq!(
                op["x-epr-action"],
                format!("{:?}", req.action),
                "{method} {path}"
            );
            assert_eq!(
                op["security"],
                serde_json::json!([{"bearerAuth": []}]),
                "{method} {path}"
            );
        }
    }
}

#[test]
fn no_path_carries_a_version_segment() {
    let doc = openapi::document();
    for path in doc["paths"].as_object().unwrap().keys() {
        for seg in path.split('/') {
            let versioned = seg.len() > 1
                && seg.starts_with('v')
                && seg[1..].chars().all(|c| c.is_ascii_digit());
            assert!(!versioned, "{path} has a version segment");
        }
    }
}

#[test]
fn a_route_added_to_the_served_set_appears_in_the_document() {
    let mut served = openapi::served();
    served.push(("GET".into(), "/api/planted/{thing}".into()));
    let doc = openapi::document_for(&served);
    let op = &doc["paths"]["/api/planted/{thing}"]["get"];
    assert!(op.is_object(), "a planted route must appear");
    assert_eq!(op["parameters"][0]["name"], "thing");
    // ... and it is absent from the document of the real served set.
    assert!(openapi::document()["paths"]["/api/planted/{thing}"].is_null());
}

#[test]
fn operation_ids_are_unique() {
    let doc = openapi::document();
    let mut ids = Vec::new();
    for item in doc["paths"].as_object().unwrap().values() {
        for op in item.as_object().unwrap().values() {
            ids.push(op["operationId"].as_str().unwrap().to_string());
        }
    }
    let n = ids.len();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), n, "operationId collision");
}
