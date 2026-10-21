//! `/fhir/R4/Patient`: the identity record as a FHIR R4 `Patient` (task ID-3).
//!
//! A thin translation over the JSON handlers in `patients.rs`: each request is turned into the
//! JSON request, the *same handler* runs (so the role gate, relationship, consent, purpose of use
//! and audit row are exactly those of `/api/patients`), and its answer is turned back into FHIR.
//! Nothing here decides access.
//!
//! Supported: `read`, `search-type` (`family`, `birthdate`, `identifier`), `create`, `update`
//! (`If-Match` required) and `metadata`. Not supported: history, delete (there is none), conditional
//! create, `_include`, paging, and any search prefix other than exact dates. A Patient carrying an
//! element the identity record cannot hold is **refused** (422), not stored with the element
//! silently dropped. A possible duplicate is refused (409) without the candidates: override it
//! through `/api/patients`, where the reason is recorded.

use super::patients::{self, CreateRequest, UpdateRequest};
use crate::auth::Agent;
use crate::interop::fhir_patient::{from_fhir_json, to_fhir};
use crate::patients_store::{Search, Stored};
use crate::policy::Subject;
use axum::{
    Extension,
    extract::{Json, Path, Query},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use loco_rs::prelude::*;
use std::collections::HashMap;

const FHIR_JSON: &str = "application/fhir+json; charset=utf-8";
const BASE: &str = "/fhir/R4";

/// A FHIR resource as a response, with a weak `ETag` for the version when it is one patient.
fn fhir(status: StatusCode, body: &serde_json::Value, version: Option<i32>) -> Response {
    let mut response = (status, body.to_string()).into_response();
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(FHIR_JSON));
    if let Some(v) = version
        && let Ok(value) = HeaderValue::from_str(&format!("W/\"{v}\""))
    {
        headers.insert(header::ETAG, value);
    }
    response
}

/// An `OperationOutcome` with one issue.
fn outcome(status: StatusCode, code: &str, diagnostics: &str) -> Response {
    let severity = if status.is_server_error() {
        "fatal"
    } else {
        "error"
    };
    fhir(
        status,
        &serde_json::json!({
            "resourceType": "OperationOutcome",
            "issue": [{ "severity": severity, "code": code, "diagnostics": diagnostics }],
        }),
        None,
    )
}

/// The JSON handlers' answer, read back: status, headers and body.
async fn read_back(response: Response) -> (StatusCode, serde_json::Value) {
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 4 << 20)
        .await
        .unwrap_or_default();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
    )
}

/// A refused or failed JSON answer as an `OperationOutcome`. Candidates and other bodies are not
/// copied: only the status and a fixed or error-string diagnostic, so nothing the JSON API
/// withheld can leak through here.
fn error_outcome(status: StatusCode, body: &serde_json::Value, update: bool) -> Response {
    let text = body.get("error").and_then(|e| e.as_str());
    match status {
        StatusCode::CONFLICT if body.get("candidates").is_some() => outcome(
            status,
            "duplicate",
            "possible duplicate; register through /api/patients with an override reason",
        ),
        StatusCode::CONFLICT if update => {
            let current = body
                .get("current_version")
                .and_then(serde_json::Value::as_i64);
            outcome(
                StatusCode::PRECONDITION_FAILED,
                "conflict",
                &match current {
                    Some(v) => format!("the Patient has changed; its current version is {v}"),
                    None => "the Patient has changed".to_string(),
                },
            )
        }
        StatusCode::BAD_REQUEST => outcome(status, "invalid", text.unwrap_or("bad request")),
        StatusCode::FORBIDDEN => outcome(status, "forbidden", "forbidden"),
        StatusCode::NOT_FOUND => outcome(status, "not-found", "not found"),
        StatusCode::UNPROCESSABLE_ENTITY => {
            let problems: Vec<&str> = body
                .get("errors")
                .and_then(|e| e.as_array())
                .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();
            outcome(status, "invalid", &problems.join("; "))
        }
        s if s.is_server_error() => outcome(s, "exception", "internal error"),
        s => outcome(s, "processing", text.unwrap_or("not accepted")),
    }
}

/// A stored patient as FHIR JSON.
fn patient_json(stored: &Stored) -> Result<serde_json::Value, String> {
    let resource = to_fhir(stored)?;
    serde_json::to_value(resource).map_err(|e| e.to_string())
}

fn internal(why: &str) -> Response {
    tracing::error!(error = %why, "could not express a stored patient as FHIR");
    outcome(
        StatusCode::INTERNAL_SERVER_ERROR,
        "exception",
        "internal error",
    )
}

/// Turn the JSON API's single-patient answer into FHIR.
async fn one(response: Response, success: StatusCode, update: bool) -> Response {
    let (status, body) = read_back(response).await;
    if status != StatusCode::OK && status != StatusCode::CREATED {
        return error_outcome(status, &body, update);
    }
    let Ok(stored) = serde_json::from_value::<Stored>(body) else {
        return internal("unreadable stored patient");
    };
    match patient_json(&stored) {
        Ok(json) => {
            let mut response = fhir(success, &json, Some(stored.version));
            if success == StatusCode::CREATED
                && let Ok(location) =
                    HeaderValue::from_str(&format!("{BASE}/Patient/{}", stored.id))
            {
                response.headers_mut().insert(header::LOCATION, location);
            }
            response
        }
        Err(e) => internal(&e),
    }
}

/// FHIR search parameters as the store's criteria. Unknown parameters are ignored (the server
/// does not claim to support them). A date prefix other than `eq` is refused rather than
/// treated as exact.
fn search_from(params: &HashMap<String, String>) -> Result<Search, String> {
    let birth_date = match params.get("birthdate") {
        None => None,
        Some(v) => {
            let v = v.strip_prefix("eq").unwrap_or(v);
            if v.starts_with(|c: char| c.is_ascii_alphabetic()) {
                return Err("only an exact birthdate is supported".to_string());
            }
            Some(v.to_string())
        }
    };
    Ok(Search {
        family: params.get("family").cloned(),
        birth_date,
        identifier: params.get("identifier").cloned(),
    })
}

/// A `searchset` bundle of the patients.
fn bundle(found: &[Stored], limit: i64) -> Result<serde_json::Value, String> {
    let entries = found
        .iter()
        .map(|s| {
            Ok(serde_json::json!({
                "fullUrl": format!("{BASE}/Patient/{}", s.id),
                "resource": patient_json(s)?,
                "search": { "mode": "match" },
            }))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let mut b = serde_json::json!({
        "resourceType": "Bundle",
        "type": "searchset",
        "entry": entries,
    });
    // At the limit there may be more: do not claim a total.
    if i64::try_from(found.len()).is_ok_and(|n| n < limit) {
        b["total"] = serde_json::json!(found.len());
    }
    Ok(b)
}

/// The version in an `If-Match` header: `W/"3"` or `"3"`.
fn if_match_version(headers: &HeaderMap) -> Result<i32, Box<Response>> {
    let Some(raw) = headers.get(header::IF_MATCH) else {
        return Err(Box::new(outcome(
            StatusCode::PRECONDITION_REQUIRED,
            "processing",
            "send If-Match with the version you are changing, e.g. W/\"2\"",
        )));
    };
    raw.to_str()
        .ok()
        .map(|s| s.trim().trim_start_matches("W/").trim_matches('"'))
        .and_then(|s| s.parse::<i32>().ok())
        .ok_or_else(|| {
            Box::new(outcome(
                StatusCode::BAD_REQUEST,
                "invalid",
                "If-Match must be W/\"<version>\"",
            ))
        })
}

/// Read a Patient body: unreadable or carrying elements the record cannot hold is refused.
fn patient_from(
    body: &str,
    path_id: Option<&str>,
) -> Result<electronic_patient_record_matcher::Patient, Box<Response>> {
    if let (Some(path_id), Ok(raw)) = (path_id, serde_json::from_str::<serde_json::Value>(body))
        && let Some(id) = raw.get("id").and_then(|v| v.as_str())
        && !id.eq_ignore_ascii_case(path_id)
    {
        return Err(Box::new(outcome(
            StatusCode::BAD_REQUEST,
            "invalid",
            "the id in the body differs from the id in the URL",
        )));
    }
    let read = from_fhir_json(body).map_err(|u| {
        Box::new(outcome(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid",
            &u.0.join("; "),
        ))
    })?;
    // `text` is generated narrative: it can be rebuilt from the data, so it is not data lost.
    let dropped: Vec<&str> = read
        .dropped
        .iter()
        .map(String::as_str)
        .filter(|k| *k != "text")
        .collect();
    if !dropped.is_empty() {
        return Err(Box::new(outcome(
            StatusCode::UNPROCESSABLE_ENTITY,
            "not-supported",
            &format!(
                "this server cannot store these Patient elements: {}",
                dropped.join(", ")
            ),
        )));
    }
    Ok(read.patient)
}

async fn read_patient(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let r = patients::read(
        State(ctx),
        Extension(agent),
        Extension(subject),
        headers,
        Path(id),
    )
    .await;
    one(r, StatusCode::OK, false).await
}

async fn search_patients(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let search = match search_from(&params) {
        Ok(s) => s,
        Err(m) => return outcome(StatusCode::BAD_REQUEST, "not-supported", &m),
    };
    let r = patients::search(
        State(ctx),
        Extension(agent),
        Extension(subject),
        headers,
        Query(search),
    )
    .await;
    let (status, body) = read_back(r).await;
    if status != StatusCode::OK {
        return error_outcome(status, &body, false);
    }
    let found: Vec<Stored> = body
        .get("results")
        .cloned()
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();
    match bundle(&found, crate::patients_store::SEARCH_LIMIT) {
        Ok(b) => fhir(StatusCode::OK, &b, None),
        Err(e) => internal(&e),
    }
}

async fn create_patient(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    body: String,
) -> Response {
    let patient = match patient_from(&body, None) {
        Ok(p) => p,
        Err(r) => return *r,
    };
    let request = CreateRequest {
        patient,
        reason: None,
        override_reason: None,
    };
    let r = patients::create(
        State(ctx),
        Extension(agent),
        Extension(subject),
        headers,
        Json(request),
    )
    .await;
    one(r, StatusCode::CREATED, false).await
}

async fn update_patient(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: String,
) -> Response {
    let expected_version = match if_match_version(&headers) {
        Ok(v) => v,
        Err(r) => return *r,
    };
    let patient = match patient_from(&body, Some(&id)) {
        Ok(p) => p,
        Err(r) => return *r,
    };
    let request = UpdateRequest {
        patient,
        expected_version,
        reason: None,
    };
    let r = patients::update(
        State(ctx),
        Extension(agent),
        Extension(subject),
        headers,
        Path(id),
        Json(request),
    )
    .await;
    one(r, StatusCode::OK, true).await
}

/// What this server does, as a FHIR `CapabilityStatement`.
fn capability() -> serde_json::Value {
    serde_json::json!({
        "resourceType": "CapabilityStatement",
        "status": "draft",
        "experimental": true,
        "date": "2026-10-21",
        "kind": "instance",
        "software": { "name": "electronic-patient-record", "version": env!("CARGO_PKG_VERSION") },
        "fhirVersion": "4.0.1",
        "format": ["json"],
        "description": "NOT FOR CLINICAL USE. Patient identity only: read, search-type, create and update with If-Match. No history, delete, conditional create, paging or search prefixes.",
        "rest": [{
            "mode": "server",
            "resource": [{
                "type": "Patient",
                "versioning": "versioned",
                "updateCreate": false,
                "conditionalCreate": false,
                "interaction": [
                    { "code": "read" },
                    { "code": "search-type" },
                    { "code": "create" },
                    { "code": "update" },
                ],
                "searchParam": [
                    { "name": "family", "type": "string" },
                    { "name": "birthdate", "type": "date", "documentation": "exact date only" },
                    { "name": "identifier", "type": "token" },
                ],
            }],
        }],
    })
}

async fn metadata() -> Response {
    fhir(StatusCode::OK, &capability(), None)
}

/// Routes under `/fhir/R4`.
#[must_use]
pub fn routes() -> Routes {
    Routes::new()
        .prefix("fhir/R4")
        .add("/metadata", get(metadata))
        .add("/Patient", get(search_patients))
        .add("/Patient", post(create_patient))
        .add("/Patient/{id}", get(read_patient))
        .add("/Patient/{id}", put(update_patient))
}

#[cfg(test)]
mod tests {
    use super::*;
    use electronic_patient_record_matcher::{HumanName, Patient};

    fn stored(id: &str, family: &str) -> Stored {
        Stored {
            id: id.into(),
            version: 3,
            author: "a".into(),
            patient: Patient {
                id: Some(id.into()),
                names: vec![HumanName {
                    family: Some(family.into()),
                    given: vec!["Ann".into()],
                }],
                birth_date: Some("1980-03-14".into()),
                ..Default::default()
            },
        }
    }

    async fn body_of(r: Response) -> serde_json::Value {
        read_back(r).await.1
    }

    #[tokio::test]
    async fn a_patient_response_is_fhir_json_with_a_weak_etag_for_its_version() {
        let json = patient_json(&stored("11111111-1111-4111-8111-111111111111", "Smith")).unwrap();
        let r = fhir(StatusCode::OK, &json, Some(3));
        assert_eq!(r.headers()[header::CONTENT_TYPE], FHIR_JSON);
        assert_eq!(r.headers()[header::ETAG], "W/\"3\"");
        let b = body_of(r).await;
        assert_eq!(b["resourceType"], "Patient");
        assert_eq!(b["meta"]["versionId"], "3");
    }

    #[test]
    fn search_parameters_map_and_a_date_prefix_is_refused() {
        let p = |pairs: &[(&str, &str)]| -> HashMap<String, String> {
            pairs
                .iter()
                .map(|(k, v)| ((*k).into(), (*v).into()))
                .collect()
        };
        let s = search_from(&p(&[
            ("family", "Smi"),
            ("birthdate", "1980-03-14"),
            ("identifier", "urn:x|1"),
            ("_count", "5"),
        ]))
        .unwrap();
        assert_eq!(s.family.as_deref(), Some("Smi"));
        assert_eq!(s.birth_date.as_deref(), Some("1980-03-14"));
        assert_eq!(s.identifier.as_deref(), Some("urn:x|1"));
        assert_eq!(
            search_from(&p(&[("birthdate", "eq1980")]))
                .unwrap()
                .birth_date
                .as_deref(),
            Some("1980")
        );
        for prefix in ["ge", "le", "gt", "lt", "ne", "sa", "eb", "ap"] {
            assert!(
                search_from(&p(&[("birthdate", &format!("{prefix}1980-01-01"))])).is_err(),
                "{prefix}"
            );
        }
    }

    #[tokio::test]
    async fn a_bundle_counts_only_when_it_may_be_complete() {
        let found = vec![stored("11111111-1111-4111-8111-111111111111", "Smith")];
        let b = bundle(&found, 50).unwrap();
        assert_eq!(b["type"], "searchset");
        assert_eq!(b["total"], 1);
        assert_eq!(b["entry"][0]["resource"]["resourceType"], "Patient");
        assert_eq!(
            b["entry"][0]["fullUrl"],
            "/fhir/R4/Patient/11111111-1111-4111-8111-111111111111"
        );
        assert!(
            bundle(&found, 1).unwrap().get("total").is_none(),
            "at the limit there may be more"
        );
    }

    #[tokio::test]
    async fn errors_become_operation_outcomes_and_never_carry_the_candidates() {
        let dup = serde_json::json!({"error": "possible duplicate", "candidates": [{"existing": {"patient": {"names": [{"family": "Secret"}]}}}]});
        let r = error_outcome(StatusCode::CONFLICT, &dup, false);
        assert_eq!(r.status(), StatusCode::CONFLICT);
        let b = body_of(r).await;
        assert_eq!(b["resourceType"], "OperationOutcome");
        assert_eq!(b["issue"][0]["code"], "duplicate");
        assert!(!b.to_string().contains("Secret"));

        let stale = serde_json::json!({"error": "version conflict", "current_version": 4});
        let r = error_outcome(StatusCode::CONFLICT, &stale, true);
        assert_eq!(r.status(), StatusCode::PRECONDITION_FAILED);
        assert!(
            body_of(r).await["issue"][0]["diagnostics"]
                .as_str()
                .unwrap()
                .contains('4')
        );

        let bad =
            serde_json::json!({"errors": ["family name is required", "birth date is not a date"]});
        let b = body_of(error_outcome(StatusCode::UNPROCESSABLE_ENTITY, &bad, false)).await;
        assert!(
            b["issue"][0]["diagnostics"]
                .as_str()
                .unwrap()
                .contains("family name is required; birth date")
        );

        assert_eq!(
            error_outcome(
                StatusCode::FORBIDDEN,
                &serde_json::json!({"error": "forbidden"}),
                false
            )
            .status(),
            StatusCode::FORBIDDEN
        );
        let b = body_of(error_outcome(
            StatusCode::INTERNAL_SERVER_ERROR,
            &serde_json::json!({"error": "db password is hunter2"}),
            false,
        ))
        .await;
        assert_eq!(b["issue"][0]["diagnostics"], "internal error");
    }

    #[test]
    fn if_match_is_required_and_read_as_a_version() {
        let with = |v: &str| {
            let mut h = HeaderMap::new();
            h.insert(header::IF_MATCH, HeaderValue::from_str(v).unwrap());
            h
        };
        assert_eq!(if_match_version(&with("W/\"2\"")).ok(), Some(2));
        assert_eq!(if_match_version(&with("\"7\"")).ok(), Some(7));
        assert_eq!(
            if_match_version(&HeaderMap::new())
                .err()
                .map(|r| r.status()),
            Some(StatusCode::PRECONDITION_REQUIRED)
        );
        assert_eq!(
            if_match_version(&with("W/\"x\"")).err().map(|r| r.status()),
            Some(StatusCode::BAD_REQUEST)
        );
    }

    #[test]
    fn a_patient_body_with_an_unstorable_element_or_a_different_id_is_refused() {
        let ok = r#"{"resourceType":"Patient","text":{"status":"generated","div":"<div/>"},"name":[{"family":"Smith","given":["Ann"]}],"birthDate":"1980-03-14"}"#;
        assert!(patient_from(ok, None).is_ok(), "narrative text is not data");
        let extra = r#"{"resourceType":"Patient","maritalStatus":{"text":"M"},"name":[{"family":"Smith"}]}"#;
        let r = patient_from(extra, None).err().unwrap();
        assert_eq!(r.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let other = r#"{"resourceType":"Patient","id":"22222222-2222-4222-8222-222222222222","name":[{"family":"Smith"}]}"#;
        assert_eq!(
            patient_from(other, Some("11111111-1111-4111-8111-111111111111"))
                .err()
                .map(|r| r.status()),
            Some(StatusCode::BAD_REQUEST)
        );
        assert!(patient_from(other, Some("22222222-2222-4222-8222-222222222222")).is_ok());
        assert!(patient_from("not json", None).is_err());
    }

    #[test]
    fn the_capability_statement_parses_as_fhir_r4() {
        let resource: Result<fhir::r4::resources::Resource, _> =
            serde_json::from_value(capability());
        assert!(resource.is_ok(), "{:?}", resource.err());
    }
}
