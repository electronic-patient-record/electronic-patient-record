//! OpenAPI census (OS-7): the committed `openapi.json` is exactly what the code produces, every
//! served route is in it with its policy requirement, public routes say so, and no path carries a
//! version segment (the API is versioned by the `Accepts-version` header). And no answer but the one
//! that makes a secret holds a secret-named field (TL-11). Every array schema states its maximum
//! (TL-13).
//!
//! To accept a deliberate change: `EPR_BLESS=1 cargo test --test suite openapi_census::`, then review the diff.

use electronic_patient_record::{auth, openapi, routes};
use serde_json::Value;
use std::{collections::BTreeSet, fs, path::Path};

const FILE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/openapi.json");

#[test]
fn the_committed_document_is_what_the_code_produces() {
    let produced = openapi::rendered();
    if std::env::var("EPR_BLESS").as_deref() == Ok("1") {
        fs::write(FILE, &produced).expect("write openapi.json");
        return;
    }
    let committed = fs::read_to_string(Path::new(FILE)).expect(
        "openapi.json is missing: run `EPR_BLESS=1 cargo test --test suite openapi_census::` and commit it",
    );
    assert!(
        committed == produced,
        "openapi.json is stale: run `EPR_BLESS=1 cargo test --test suite openapi_census::`, review and commit"
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

fn arrays_without_a_maximum(value: &Value, path: &str, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            let is_array = map.get("type").is_some_and(|kind| {
                kind.as_str() == Some("array")
                    || kind
                        .as_array()
                        .is_some_and(|kinds| kinds.iter().any(|kind| kind == "array"))
            });
            if is_array && map.get("maxItems").and_then(Value::as_u64).is_none() {
                out.push(path.to_string());
            }
            for (key, child) in map {
                arrays_without_a_maximum(child, &format!("{path}/{key}"), out);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                arrays_without_a_maximum(child, &format!("{path}/{index}"), out);
            }
        }
        _ => {}
    }
}

#[test]
fn every_array_schema_states_a_maximum() {
    let mut missing = Vec::new();
    arrays_without_a_maximum(&openapi::document(), "", &mut missing);
    assert!(
        missing.is_empty(),
        "array schemas without maxItems: {missing:?}"
    );
}

#[test]
fn a_planted_array_without_a_maximum_is_found() {
    let mut doc = openapi::document();
    doc["components"]["schemas"]["Patient"]["properties"]["names"]
        .as_object_mut()
        .unwrap()
        .remove("maxItems");
    let mut missing = Vec::new();
    arrays_without_a_maximum(&doc, "", &mut missing);
    assert!(
        missing
            .iter()
            .any(|path| path.ends_with("/Patient/properties/names")),
        "{missing:?}"
    );
}

#[test]
fn a_patch_must_state_the_only_permitted_update_semantics() {
    fn without_semantics(doc: &Value) -> Vec<&str> {
        doc["paths"]
            .as_object()
            .unwrap()
            .iter()
            .filter_map(|(path, item)| {
                let patch = item.get("patch")?;
                (patch["x-epr-patch-semantics"]
                    != "absent-keeps-null-clears-no-implicit-array-append")
                    .then_some(path.as_str())
            })
            .collect()
    }

    assert!(
        without_semantics(&openapi::document()).is_empty(),
        "a PATCH must say absent keeps, null clears, and arrays never append implicitly"
    );
    let mut doc = openapi::document();
    doc["paths"]["/api/planted"] = serde_json::json!({"patch": {}});
    assert_eq!(without_semantics(&doc), ["/api/planted"]);
    doc["paths"]["/api/planted"]["patch"]["x-epr-patch-semantics"] =
        serde_json::json!("absent-keeps-null-clears-no-implicit-array-append");
    assert!(without_semantics(&doc).is_empty());
}

/// The words that name a secret, matched against each whole word of a field's name (TL-11).
const SECRET_WORDS: &[&str] = &[
    "secret",
    "password",
    "passphrase",
    "token",
    "hash",
    "salt",
    "credential",
    "credentials",
    "key",
];

/// The one answer that may hold a secret: the token just issued, shown once (TL-1).
const SHOWN_ONCE: (&str, &str, &str) = ("POST /api/tokens", "IssuedToken", "token");

fn is_secret_name(field: &str) -> bool {
    field
        .split(|c: char| c == '_' || c == '-' || !c.is_ascii_alphanumeric())
        .flat_map(|w| {
            // camelCase: split before each capital.
            let mut words = Vec::new();
            let mut cur = String::new();
            for c in w.chars() {
                if c.is_ascii_uppercase() && !cur.is_empty() {
                    words.push(std::mem::take(&mut cur));
                }
                cur.push(c.to_ascii_lowercase());
            }
            words.push(cur);
            words
        })
        .any(|w| SECRET_WORDS.contains(&w.as_str()))
}

/// Every (operation, schema, field) where a response schema, followed through `$ref`s, names a secret.
fn secret_fields(doc: &Value) -> BTreeSet<(String, String, String)> {
    fn walk(
        doc: &Value,
        schema: &Value,
        name: &str,
        op: &str,
        seen: &mut BTreeSet<String>,
        out: &mut BTreeSet<(String, String, String)>,
    ) {
        match schema {
            Value::Object(map) => {
                if let Some(r) = map.get("$ref").and_then(Value::as_str) {
                    let target = r.trim_start_matches("#/components/schemas/");
                    if seen.insert(target.to_string()) {
                        walk(
                            doc,
                            &doc["components"]["schemas"][target],
                            target,
                            op,
                            seen,
                            out,
                        );
                    }
                }
                if let Some(props) = map.get("properties").and_then(Value::as_object) {
                    for (field, sub) in props {
                        if is_secret_name(field) {
                            out.insert((op.to_string(), name.to_string(), field.clone()));
                        }
                        walk(doc, sub, name, op, seen, out);
                    }
                }
                for (k, v) in map {
                    if k != "properties" && k != "$ref" {
                        walk(doc, v, name, op, seen, out);
                    }
                }
            }
            Value::Array(items) => items.iter().for_each(|v| walk(doc, v, name, op, seen, out)),
            _ => {}
        }
    }
    let mut out = BTreeSet::new();
    for (path, item) in doc["paths"].as_object().unwrap() {
        for (method, op) in item.as_object().unwrap() {
            let label = format!("{} {path}", method.to_ascii_uppercase());
            for response in op["responses"]
                .as_object()
                .into_iter()
                .flat_map(|r| r.values())
            {
                for media in response["content"]
                    .as_object()
                    .into_iter()
                    .flat_map(|c| c.values())
                {
                    walk(
                        doc,
                        &media["schema"],
                        "(inline)",
                        &label,
                        &mut BTreeSet::new(),
                        &mut out,
                    );
                }
            }
        }
    }
    out
}

#[test]
fn no_answer_but_the_one_that_makes_a_secret_holds_a_secret_named_field() {
    let found = secret_fields(&openapi::document());
    let allowed = (
        SHOWN_ONCE.0.to_string(),
        SHOWN_ONCE.1.to_string(),
        SHOWN_ONCE.2.to_string(),
    );
    assert!(
        found.contains(&allowed),
        "the census no longer sees the issued token: {found:?}"
    );
    let others: Vec<_> = found.iter().filter(|f| **f != allowed).collect();
    assert!(
        others.is_empty(),
        "secret-named fields in answers: {others:?}"
    );
}

#[test]
fn a_planted_secret_in_an_answer_is_found() {
    let mut doc = openapi::document();
    doc["components"]["schemas"]["ApiToken"]["properties"]["signingSecret"] =
        serde_json::json!({"type": "string"});
    let found = secret_fields(&doc);
    assert!(
        found.contains(&(
            "GET /api/tokens".into(),
            "ApiToken".into(),
            "signingSecret".into()
        )),
        "{found:?}"
    );
    for (name, secret) in [
        ("signing_secret", true),
        ("api_key", true),
        ("passwordHash", true),
        ("keyed", false),
        ("tokens", false),
        ("monkey", false),
        ("token", true),
    ] {
        assert_eq!(is_secret_name(name), secret, "{name}");
    }
}
