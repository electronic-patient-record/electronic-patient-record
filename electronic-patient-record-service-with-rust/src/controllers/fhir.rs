//! `/fhir/R4/Patient` (task ID-3), `/fhir/R4/AllergyIntolerance` (task RC-5, read-only),
//! `/fhir/R4/Observation` (task RC-6, read-only) and `/fhir/R4/AuditEvent` (task AU-2).
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

use super::audit::search_audited;
use super::common::authorize;
use super::patients::{self, CreateRequest, UpdateRequest};
use crate::allergies::{StoredAllergy, Summary};
use crate::api_error::ApiError;
use crate::auth::Agent;
use crate::interop::fhir_patient::{from_fhir_json, to_fhir};
use crate::interop::{fhir_allergy, fhir_audit, fhir_observation};
use crate::observations_store::StoredObservation;
use crate::patients_store::{Search, Stored};
use crate::policy::Subject;
use crate::policy::{self, Purpose, Resource};
use axum::{
    Extension,
    extract::{Path, Query, RawQuery},
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

/// A merged record says where it went, and a surviving one what it replaced (task ID-5): `active`
/// is false and `link` holds `replaced-by` for the old record, `replaces` for the new. The result is
/// checked against the typed R4 model again.
fn with_links(
    mut json: serde_json::Value,
    replaced_by: Option<&str>,
    replaces: &[String],
) -> Result<serde_json::Value, String> {
    let mut links = Vec::new();
    if let Some(to) = replaced_by {
        links.push(serde_json::json!({ "other": { "reference": format!("Patient/{to}") }, "type": "replaced-by" }));
        json["active"] = serde_json::json!(false);
    }
    for from in replaces {
        links.push(serde_json::json!({ "other": { "reference": format!("Patient/{from}") }, "type": "replaces" }));
    }
    if links.is_empty() {
        return Ok(json);
    }
    json["link"] = serde_json::Value::Array(links);
    serde_json::from_value::<fhir::r4::resources::Resource>(json.clone())
        .map_err(|e| e.to_string())?;
    Ok(json)
}

/// A refusal from the service as an `OperationOutcome`.
fn refusal(e: &ApiError, update: bool) -> Response {
    error_outcome(e.status(), &e.body(), update)
}

/// A stored patient, and where it was merged to or what was merged into it, as FHIR.
fn one(
    result: Result<(Stored, Option<String>, Vec<String>), ApiError>,
    success: StatusCode,
    update: bool,
) -> Response {
    let (stored, replaced_by, replaces) = match result {
        Ok(v) => v,
        Err(e) => return refusal(&e, update),
    };
    match patient_json(&stored).and_then(|j| with_links(j, replaced_by.as_deref(), &replaces)) {
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

/// Refuse a search parameter the resource does not read, so a filter is never silently lost: a
/// search for `gender=female` that returned every Smith would look filtered and not be. Only the
/// result-shaping `_count`, `_format` and `_pretty` are let through.
fn refuse_unknown(params: &HashMap<String, String>, known: &[&str]) -> Result<(), String> {
    let mut unknown: Vec<&str> = params
        .keys()
        .map(String::as_str)
        .filter(|k| !known.contains(k) && !matches!(*k, "_count" | "_format" | "_pretty"))
        .collect();
    unknown.sort_unstable();
    match unknown.first() {
        None => Ok(()),
        Some(k) => Err(format!("the search parameter {k} is not supported")),
    }
}

/// FHIR search parameters as the store's criteria. An unknown parameter is refused, not ignored
/// (`refuse_unknown`). A date prefix other than `eq` is refused rather than treated as exact.
fn search_from(params: &HashMap<String, String>) -> Result<Search, String> {
    refuse_unknown(params, &["family", "birthdate", "identifier"])?;
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
        // The FHIR facade shows real patients only (EC-10).
        drill: false,
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
    let r = patients::read_service(&ctx, &agent, &subject, &headers, &id)
        .await
        .map(|v| (v.stored, v.replaced_by, v.replaces));
    one(r, StatusCode::OK, false)
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
    let found = match patients::search_service(&ctx, &agent, &subject, &headers, &search).await {
        Ok(found) => found,
        Err(e) => return refusal(&e, false),
    };
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
        drill: false,
    };
    let r = patients::create_service(&ctx, &agent, &subject, &headers, &request)
        .await
        .map(|s| (s, None, Vec::new()));
    one(r, StatusCode::CREATED, false)
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
    let r = patients::update_service(&ctx, &agent, &subject, &headers, &id, &request)
        .await
        .map(|s| (s, None, Vec::new()));
    one(r, StatusCode::OK, true)
}

/// AuditEvent search parameters as audit criteria. `date` may repeat with the `ge`, `gt`, `le` and
/// `lt` prefixes; a bare date covers its whole day (`le2026-10-01` includes all of 1 October,
/// `gt2026-10-01` starts on 2 October), an instant is exact. A date with no prefix is refused
/// (FHIR reads it as `eq`, which this does not support). `patient` takes a bare id or
/// `Patient/<id>`. A search parameter this does not know (a misspelt `agnet`) is refused, not
/// ignored: an audit search that silently loses its filter answers a different question. That
/// includes the `_` filters (`_id`, `_lastUpdated`): only the result-shaping `_count`, `_format` and
/// `_pretty` are ignored.
fn audit_query_from(pairs: &[(String, String)]) -> Result<crate::audit::AuditQuery, String> {
    let mut q = crate::audit::AuditQuery::default();
    for (key, value) in pairs {
        match key.as_str() {
            "agent" => q.agent = Some(value.clone()),
            "patient" => {
                q.patient = Some(value.strip_prefix("Patient/").unwrap_or(value).to_string())
            }
            "action" => q.action = Some(value.clone()),
            "outcome" => {
                q.outcome = Some(
                    value
                        .parse()
                        .map_err(|_| "outcome must be 0, 4, 8 or 12".to_string())?,
                );
            }
            "_before" => {
                q.before = Some(
                    value
                        .parse()
                        .map_err(|_| "_before must be a row id".to_string())?,
                );
            }
            "date" => {
                if let Some(v) = value.strip_prefix("ge") {
                    q.from = Some(v.to_string());
                } else if let Some(v) = value.strip_prefix("lt") {
                    q.to = Some(v.to_string());
                } else if let Some(v) = value.strip_prefix("gt") {
                    q.after = Some(v.to_string());
                } else if let Some(v) = value.strip_prefix("le") {
                    q.through = Some(v.to_string());
                } else {
                    return Err("date supports the ge, gt, le and lt prefixes".to_string());
                }
            }
            "_count" | "_format" | "_pretty" => {}
            k => return Err(format!("the search parameter {k} is not supported")),
        }
    }
    Ok(q)
}

async fn search_audit_events(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    RawQuery(raw): RawQuery,
) -> Response {
    let raw = raw.unwrap_or_default();
    let Ok(pairs) = serde_urlencoded::from_str::<Vec<(String, String)>>(&raw) else {
        return outcome(
            StatusCode::BAD_REQUEST,
            "invalid",
            "the query string is not valid",
        );
    };
    let query = match audit_query_from(&pairs) {
        Ok(q) => q,
        Err(m) => return outcome(StatusCode::BAD_REQUEST, "not-supported", &m),
    };
    if let Err(r) = authorize(
        &ctx.db,
        &agent,
        &subject,
        Resource::AuditLog,
        policy::Action::Search,
        Purpose::Audit,
        None,
    )
    .await
    {
        return refusal(&r, false);
    }
    match search_audited(&ctx.db, &agent, &query).await {
        Ok((rows, next)) => {
            let entries: Result<Vec<_>, String> = rows
                .iter()
                .map(|r| {
                    let resource = fhir_audit::to_fhir(r)?;
                    Ok(serde_json::json!({
                        "fullUrl": format!("{BASE}/AuditEvent/{}", r.id),
                        "resource": serde_json::to_value(resource).map_err(|e| e.to_string())?,
                        "search": { "mode": "match" },
                    }))
                })
                .collect();
            let Ok(entries) = entries else {
                return internal("an audit row the FHIR model rejects");
            };
            let mut bundle = serde_json::json!({
                "resourceType": "Bundle", "type": "searchset", "entry": entries,
            });
            if let Some(id) = next {
                let kept: Vec<(String, String)> =
                    pairs.into_iter().filter(|(k, _)| k != "_before").collect();
                let mut qs = serde_urlencoded::to_string(&kept).unwrap_or_default();
                if !qs.is_empty() {
                    qs.push('&');
                }
                bundle["link"] = serde_json::json!([{
                    "relation": "next",
                    "url": format!("{BASE}/AuditEvent?{qs}_before={id}"),
                }]);
            }
            fhir(StatusCode::OK, &bundle, None)
        }
        Err(Ok(message)) => outcome(StatusCode::BAD_REQUEST, "invalid", &message),
        Err(Err(e)) => {
            tracing::error!(error = %e, "audit search failed");
            outcome(
                StatusCode::INTERNAL_SERVER_ERROR,
                "exception",
                "internal error",
            )
        }
    }
}

/// An `OperationOutcome` entry saying that an empty or partial list is not "none". Allergy data
/// that is missing must never read as an allergy that is absent.
fn not_a_statement_of_none(why: &str) -> serde_json::Value {
    serde_json::json!({
        "resource": {
            "resourceType": "OperationOutcome",
            "issue": [{
                "severity": "warning",
                "code": "incomplete",
                "diagnostics": format!("{why} This does not mean the patient has no allergies."),
            }],
        },
        "search": { "mode": "outcome" },
    })
}

/// A `searchset` of the patient's allergies, built from the JSON handler's answer.
fn allergy_bundle(patient: &str, body: &serde_json::Value) -> Result<serde_json::Value, String> {
    let all: Vec<StoredAllergy> =
        serde_json::from_value(body.get("allergies").cloned().unwrap_or_default())
            .map_err(|e| e.to_string())?;
    let summary: Summary = serde_json::from_value(body.get("summary").cloned().unwrap_or_default())
        .map_err(|e| e.to_string())?;
    let mut entries = Vec::new();
    for a in &all {
        if let Some(r) = fhir_allergy::to_fhir(a, patient)? {
            entries.push(serde_json::json!({
                "fullUrl": format!("{BASE}/AllergyIntolerance/{}", a.id),
                "resource": r,
                "search": { "mode": "match" },
            }));
        }
    }
    match &summary {
        Summary::NoKnown { asserted_at, .. } => {
            let r = fhir_allergy::no_known_allergies(patient, asserted_at)?;
            entries.push(serde_json::json!({
                "resource": r,
                "search": { "mode": "match" },
            }));
        }
        Summary::NotRecorded => entries.push(not_a_statement_of_none(
            "No allergy information has been recorded.",
        )),
        Summary::NoneActive { .. } => entries.push(not_a_statement_of_none(
            "No allergy is active, but nobody has stated that there are none known.",
        )),
        Summary::Active { .. } => {}
    }
    Ok(serde_json::json!({
        "resourceType": "Bundle",
        "type": "searchset",
        "entry": entries,
    }))
}

/// `GET /fhir/R4/AllergyIntolerance?patient=` : the same handler, so the same gate, care
/// relationship, purpose of use, consent and audit row as `/api/patients/{id}/allergies`.
async fn search_allergies(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    if let Err(m) = refuse_unknown(&params, &["patient"]) {
        return outcome(StatusCode::BAD_REQUEST, "not-supported", &m);
    }
    let Some(patient) = params
        .get("patient")
        .map(|p| p.strip_prefix("Patient/").unwrap_or(p).to_string())
        .filter(|p| !p.is_empty())
    else {
        return outcome(
            StatusCode::BAD_REQUEST,
            "required",
            "the patient search parameter is required",
        );
    };
    let list =
        match super::allergies::list_service(&ctx, &agent, &subject, &headers, &patient).await {
            Ok(list) => list,
            Err(e) => {
                // A refusal, a consent refusal or a failure: say that this is not an empty list.
                if e.status() == StatusCode::FORBIDDEN {
                    return outcome(
                        e.status(),
                        "forbidden",
                        "forbidden. This does not mean the patient has no allergies.",
                    );
                }
                return refusal(&e, false);
            }
        };
    let Ok(body) = serde_json::to_value(&list) else {
        return internal("an allergy list that cannot be written");
    };
    match allergy_bundle(&patient, &body) {
        Ok(b) => fhir(StatusCode::OK, &b, None),
        Err(e) => internal(&e),
    }
}

/// A `searchset` of the patient's vital signs, built from the JSON handler's answer. A list with
/// nothing in it carries a warning: no vital signs here is not a normal reading.
fn observation_bundle(
    patient: &str,
    all: &[StoredObservation],
) -> Result<serde_json::Value, String> {
    let mut entries = Vec::new();
    for o in all {
        if let Some(r) = fhir_observation::to_fhir(o, patient)? {
            entries.push(serde_json::json!({
                "fullUrl": format!("{BASE}/Observation/{}", o.id),
                "resource": r,
                "search": { "mode": "match" },
            }));
        }
    }
    if entries.is_empty() {
        entries.push(serde_json::json!({
            "resource": {
                "resourceType": "OperationOutcome",
                "issue": [{
                    "severity": "warning",
                    "code": "incomplete",
                    "diagnostics": "No vital sign has been recorded here. This does not mean the patient's readings are normal or that none were taken.",
                }],
            },
            "search": { "mode": "outcome" },
        }));
    }
    Ok(serde_json::json!({
        "resourceType": "Bundle",
        "type": "searchset",
        "entry": entries,
    }))
}

/// `GET /fhir/R4/Observation?patient=` : the same service function, so the same gate, care
/// relationship, purpose of use, consent and audit row as `/api/patients/{id}/observations`.
async fn search_observations(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    if let Err(m) = refuse_unknown(&params, &["patient"]) {
        return outcome(StatusCode::BAD_REQUEST, "not-supported", &m);
    }
    let Some(patient) = params
        .get("patient")
        .map(|p| p.strip_prefix("Patient/").unwrap_or(p).to_string())
        .filter(|p| !p.is_empty())
    else {
        return outcome(
            StatusCode::BAD_REQUEST,
            "required",
            "the patient search parameter is required",
        );
    };
    let list =
        match super::observations::list_service(&ctx, &agent, &subject, &headers, &patient).await {
            Ok(list) => list,
            Err(e) => {
                if e.status() == StatusCode::FORBIDDEN {
                    return outcome(
                        e.status(),
                        "forbidden",
                        "forbidden. This does not mean the patient has no vital signs recorded.",
                    );
                }
                return refusal(&e, false);
            }
        };
    match observation_bundle(&patient, &list.observations) {
        Ok(b) => fhir(StatusCode::OK, &b, None),
        Err(e) => internal(&e),
    }
}

/// The interactions the mounted routes give each resource type (`/Type`, `/Type/{id}`), in the
/// order FHIR lists them. The statement's `interaction` arrays are made from this, so a route
/// cannot be mounted without being stated, nor stated without being mounted (IO-1). A method and
/// path this does not know is listed as `unmapped-<METHOD>`, which is not a FHIR code, so the
/// statement stops parsing and a test fails.
fn mounted_interactions() -> std::collections::BTreeMap<String, Vec<String>> {
    use std::collections::{BTreeMap, BTreeSet};
    const ORDER: [&str; 4] = ["read", "search-type", "create", "update"];
    let mut found: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for handler in routes().handlers {
        let parts: Vec<&str> = handler.uri.trim_matches('/').split('/').collect();
        let (Some(kind), rest) = (parts.first().copied(), &parts[1.min(parts.len())..]) else {
            continue;
        };
        if kind == "metadata" {
            continue;
        }
        for method in &handler.actions {
            let code = match (method.as_str(), rest) {
                ("GET", []) => "search-type".to_string(),
                ("POST", []) => "create".to_string(),
                ("GET", ["{id}"]) => "read".to_string(),
                ("PUT", ["{id}"]) => "update".to_string(),
                (m, _) => format!("unmapped-{m}"),
            };
            found.entry(kind.to_string()).or_default().insert(code);
        }
    }
    found
        .into_iter()
        .map(|(kind, codes)| {
            let mut codes: Vec<String> = codes.into_iter().collect();
            codes.sort_by_key(|c| ORDER.iter().position(|o| o == c).unwrap_or(ORDER.len()));
            (kind, codes)
        })
        .collect()
}

/// What the statement says about a resource besides its interactions, which come from the routes:
/// versioning, documentation and search parameters. The types in `NOTED_TYPES` (a test's list of
/// what `resource_notes` has a case for) are those with something to say; a mounted type missing
/// from it is still stated, with only its interactions.
#[cfg(test)]
const NOTED_TYPES: [&str; 4] = ["Patient", "AllergyIntolerance", "Observation", "AuditEvent"];

fn resource_notes(kind: &str) -> serde_json::Value {
    match kind {
        "Patient" => serde_json::json!({
            "versioning": "versioned",
            "updateCreate": false,
            "conditionalCreate": false,
            "searchParam": [
                { "name": "family", "type": "string" },
                { "name": "birthdate", "type": "date", "documentation": "exact date only" },
                { "name": "identifier", "type": "token" },
            ],
        }),
        "AllergyIntolerance" => serde_json::json!({
            "documentation": "Search by patient only; read-only. Allergies entered in error are not returned. A result with no allergy in it is never a statement that there are none: it carries an OperationOutcome saying so, unless a clinician has stated no known allergies. verificationStatus and type are not recorded, so are not returned.",
            "searchParam": [
                { "name": "patient", "type": "reference", "documentation": "required" },
            ],
        }),
        "Observation" => serde_json::json!({
            "documentation": "Vital signs only; search by patient only; read-only. Observations entered in error are not returned. A result with no observation is never a normal reading: it carries an OperationOutcome saying so. The critical range is not returned, and a flag the service could not compute is no interpretation, never normal.",
            "searchParam": [
                { "name": "patient", "type": "reference", "documentation": "required" },
            ],
        }),
        "AuditEvent" => serde_json::json!({
            "documentation": "Auditor role only. Every search is itself audited.",
            "searchParam": [
                { "name": "agent", "type": "token", "documentation": "the token subject, exactly" },
                { "name": "patient", "type": "reference" },
                { "name": "action", "type": "token" },
                { "name": "outcome", "type": "token" },
                { "name": "date", "type": "date", "documentation": "ge, gt, le and lt prefixes; no bare or eq dates" },
            ],
        }),
        _ => serde_json::json!({}),
    }
}

/// What this server does, as a FHIR `CapabilityStatement`. The resource types and their
/// interactions are generated from the mounted routes; the notes on each are written by hand.
fn capability() -> serde_json::Value {
    let resources: Vec<serde_json::Value> = mounted_interactions()
        .into_iter()
        .map(|(kind, codes)| {
            let mut resource = resource_notes(&kind);
            resource["type"] = serde_json::json!(kind);
            resource["interaction"] = codes
                .iter()
                .map(|c| serde_json::json!({ "code": c }))
                .collect();
            resource
        })
        .collect();
    serde_json::json!({
        "resourceType": "CapabilityStatement",
        "status": "draft",
        "experimental": true,
        "date": "2026-10-21",
        "kind": "instance",
        "software": { "name": "electronic-patient-record", "version": env!("CARGO_PKG_VERSION") },
        "fhirVersion": "4.0.1",
        "format": ["json"],
        "description": "NOT FOR CLINICAL USE. Patient identity (read, search-type, create and update with If-Match), allergies (search by patient, read-only) and audit events. No history, delete, conditional create, paging or search prefixes.",
        "rest": [{ "mode": "server", "resource": resources }],
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
        .add("/AllergyIntolerance", get(search_allergies))
        .add("/Observation", get(search_observations))
        .add("/AuditEvent", get(search_audit_events))
}

#[cfg(test)]
mod tests {
    use super::*;
    use electronic_patient_record_matcher::{HumanName, Patient};

    fn stored(id: &str, family: &str) -> Stored {
        Stored {
            drill: false,
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

    /// A response's status and JSON body.
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

    async fn body_of(r: Response) -> serde_json::Value {
        read_back(r).await.1
    }

    fn allergy_body(summary: serde_json::Value, status: &str) -> serde_json::Value {
        serde_json::json!({
            "allergies": [{
                "id": "22222222-2222-4222-8222-222222222222",
                "patient_id": "11111111-1111-4111-8111-111111111111",
                "version": 1, "author": "a", "recorded_at": "2026-10-21T09:30:00.000000Z",
                "allergy": { "substance": "Penicillin", "status": status },
            }],
            "summary": summary,
        })
    }

    fn kinds(b: &serde_json::Value) -> Vec<(String, String)> {
        b["entry"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| {
                (
                    e["resource"]["resourceType"].as_str().unwrap().to_string(),
                    e["search"]["mode"].as_str().unwrap().to_string(),
                )
            })
            .collect()
    }

    fn observation(status: crate::observations_store::Status) -> StoredObservation {
        use crate::observations_store::ObservationRecord;
        StoredObservation {
            id: "o1".into(),
            patient_id: "P1".into(),
            version: 1,
            author: "a".into(),
            recorded_at: "2026-10-26T09:31:00Z".into(),
            site_zone: "Europe/London".into(),
            record: ObservationRecord {
                code: "8867-4".into(),
                value: Some(71.0),
                unit: Some("/min".into()),
                absent_reason: None,
                range: None,
                critical: None,
                effective_at: "2026-10-26T09:30:00Z".into(),
                status,
            },
            flag: None,
        }
    }

    #[test]
    fn a_standing_vital_sign_is_a_match_and_an_empty_list_is_a_warning_not_a_normal_reading() {
        use crate::observations_store::Status;
        let b = observation_bundle("P1", &[observation(Status::Final)]).unwrap();
        assert_eq!(
            kinds(&b),
            vec![("Observation".to_string(), "match".to_string())]
        );
        // Nothing, or only entries in error: a warning, never an empty (normal-looking) result.
        for list in [vec![], vec![observation(Status::EnteredInError)]] {
            let b = observation_bundle("P1", &list).unwrap();
            assert_eq!(
                kinds(&b),
                vec![("OperationOutcome".to_string(), "outcome".to_string())]
            );
            assert!(b.to_string().contains("does not mean"));
        }
    }

    #[test]
    fn an_active_allergy_is_a_match_and_needs_no_warning() {
        let body = allergy_body(
            serde_json::json!({ "state": "active", "items": [] }),
            "active",
        );
        let b = allergy_bundle("P1", &body).unwrap();
        assert_eq!(b["type"], "searchset");
        assert_eq!(
            kinds(&b),
            vec![("AllergyIntolerance".to_string(), "match".to_string())]
        );
    }

    #[test]
    fn an_empty_or_inactive_list_is_never_a_statement_of_none() {
        for summary in [
            serde_json::json!({ "state": "not-recorded" }),
            serde_json::json!({ "state": "none-active", "inactive": 1 }),
        ] {
            let mut body = allergy_body(summary, "inactive");
            if body["summary"]["state"] == "not-recorded" {
                body["allergies"] = serde_json::json!([]);
            }
            let b = allergy_bundle("P1", &body).unwrap();
            let last = b["entry"].as_array().unwrap().last().unwrap().clone();
            assert_eq!(last["search"]["mode"], "outcome", "{b}");
            let said = last["resource"]["issue"][0]["diagnostics"]
                .as_str()
                .unwrap();
            assert!(said.contains("does not mean"), "{said}");
        }
    }

    #[test]
    fn only_a_standing_statement_becomes_no_known_allergies() {
        let body = serde_json::json!({
            "allergies": [],
            "summary": { "state": "no-known", "asserted_at": "2026-10-21T09:30:00Z", "asserted_by": "x" },
        });
        let b = allergy_bundle("P1", &body).unwrap();
        let e = &b["entry"][0]["resource"];
        assert_eq!(e["code"]["coding"][0]["code"], "716186003");
        assert_eq!(kinds(&b).len(), 1);
    }

    #[test]
    fn an_allergy_entered_in_error_is_not_exported() {
        let body = allergy_body(
            serde_json::json!({ "state": "not-recorded" }),
            "entered-in-error",
        );
        let b = allergy_bundle("P1", &body).unwrap();
        assert!(
            kinds(&b).iter().all(|(t, _)| t != "AllergyIntolerance"),
            "{b}"
        );
    }

    #[test]
    fn the_json_handlers_own_answer_round_trips_into_the_bundle() {
        use crate::allergies::{
            AllergyRecord, Criticality, Status, StoredAllergy, Summary, SummaryItem,
        };
        let stored = StoredAllergy {
            id: "22222222-2222-4222-8222-222222222222".into(),
            patient_id: "11111111-1111-4111-8111-111111111111".into(),
            version: 1,
            author: "a".into(),
            recorded_at: "2026-10-21T09:30:00.000000Z".into(),
            allergy: AllergyRecord {
                substance: "Peanut".into(),
                coding: None,
                category: None,
                criticality: Some(Criticality::High),
                reaction: None,
                onset: None,
                status: Status::Active,
            },
        };
        let summary = Summary::Active {
            items: vec![SummaryItem {
                substance: "Peanut".into(),
                criticality: Some(Criticality::High),
            }],
        };
        // Exactly what `allergies::list` puts in its answer.
        let body =
            serde_json::json!({ "allergies": [stored], "statement": null, "summary": summary });
        let b = allergy_bundle("P1", &body).unwrap();
        assert_eq!(b["entry"][0]["resource"]["criticality"], "high");
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

    /// IO-1: the mounted routes and the CapabilityStatement say the same thing. A route added
    /// without a statement entry, or an entry without a route, fails here.
    #[test]
    fn the_capability_statement_lists_exactly_the_mounted_routes() {
        use std::collections::BTreeSet;
        let mounted: BTreeSet<String> = routes()
            .handlers
            .iter()
            .flat_map(|h| h.actions.iter().map(|m| format!("{m} {}", h.uri)))
            .collect();
        let mut stated: BTreeSet<String> = BTreeSet::from(["GET /metadata".to_string()]);
        let cap = capability();
        for r in cap["rest"][0]["resource"].as_array().unwrap() {
            let t = r["type"].as_str().unwrap();
            for i in r["interaction"].as_array().unwrap() {
                stated.insert(match i["code"].as_str().unwrap() {
                    "read" => format!("GET /{t}/{{id}}"),
                    "search-type" => format!("GET /{t}"),
                    "create" => format!("POST /{t}"),
                    "update" => format!("PUT /{t}/{{id}}"),
                    other => format!("UNMAPPED {other} {t}"),
                });
            }
        }
        assert_eq!(mounted, stated);
    }

    /// IO-1, third slice: the statement is generated from the routes, and its hand-written notes
    /// are not stale or missing.
    #[test]
    fn the_capability_notes_name_exactly_the_mounted_types_that_have_any() {
        let mounted = mounted_interactions();
        for kind in NOTED_TYPES {
            assert!(
                mounted.contains_key(kind),
                "notes for {kind}, which is not mounted"
            );
        }
        for (kind, codes) in &mounted {
            if codes.iter().any(|c| c == "search-type") {
                assert!(
                    NOTED_TYPES.contains(&kind.as_str()),
                    "{kind} is searchable but its search parameters are not described"
                );
            }
            assert!(
                codes.iter().all(|c| !c.starts_with("unmapped")),
                "{kind}: {codes:?}"
            );
        }
        assert!(routes().handlers.iter().any(|h| h.uri == "/metadata"));
        // FHIR's own order, whatever order the routes are mounted in.
        assert_eq!(
            mounted["Patient"],
            ["read", "search-type", "create", "update"]
        );
    }

    /// IO-1, second slice: the search parameters the statement lists are the ones the code reads.
    #[test]
    fn the_capability_statement_lists_the_search_parameters_the_code_reads() {
        use std::collections::BTreeSet;
        let cap = capability();
        let stated = |t: &str| -> BTreeSet<String> {
            cap["rest"][0]["resource"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["type"] == t)
                .unwrap()["searchParam"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| p["name"].as_str().unwrap().to_string())
                .collect()
        };
        let set =
            |names: &[&str]| -> BTreeSet<String> { names.iter().map(|n| (*n).into()).collect() };
        let one = |k: &str, v: &str| HashMap::from([(k.to_string(), v.to_string())]);

        // Patient: each stated parameter changes the search; an unstated one changes nothing.
        let patient = set(&["family", "birthdate", "identifier"]);
        assert_eq!(stated("Patient"), patient);
        for name in &patient {
            let s = search_from(&one(name, "1980")).unwrap();
            assert!(
                s.family.is_some() || s.birth_date.is_some() || s.identifier.is_some(),
                "{name} is stated but not read"
            );
        }
        for name in [
            "gender",
            "name",
            "_id",
            "address",
            "phone",
            "active",
            "_lastUpdated",
        ] {
            assert!(
                search_from(&one(name, "x")).is_err(),
                "{name} is accepted but not stated"
            );
        }
        assert!(search_from(&one("_count", "5")).is_ok());

        // AllergyIntolerance and Observation: patient only.
        assert_eq!(stated("AllergyIntolerance"), set(&["patient"]));
        assert_eq!(stated("Observation"), set(&["patient"]));

        // AuditEvent: each stated parameter is accepted; common FHIR ones that are not stated are refused.
        let audit = [
            ("agent", "x"),
            ("patient", "Patient/a"),
            ("action", "R"),
            ("outcome", "4"),
            ("date", "ge2026-10-01"),
        ];
        assert_eq!(
            stated("AuditEvent"),
            audit
                .iter()
                .map(|(k, _)| (*k).into())
                .collect::<BTreeSet<String>>()
        );
        for (k, v) in audit {
            assert!(
                audit_query_from(&[(k.to_string(), v.to_string())]).is_ok(),
                "{k} is stated but refused"
            );
        }
        for name in [
            "_id",
            "type",
            "subtype",
            "entity",
            "source",
            "site",
            "address",
            "policy",
            "_lastUpdated",
        ] {
            assert!(
                audit_query_from(&[(name.to_string(), "x".to_string())]).is_err(),
                "{name} is accepted but not stated"
            );
        }
    }

    #[test]
    fn an_unknown_search_parameter_is_refused_and_result_shaping_ones_are_not() {
        let q = |pairs: &[(&str, &str)]| -> HashMap<String, String> {
            pairs
                .iter()
                .map(|(k, v)| ((*k).into(), (*v).into()))
                .collect()
        };
        assert!(
            refuse_unknown(
                &q(&[("patient", "p"), ("_count", "5"), ("_format", "json")]),
                &["patient"]
            )
            .is_ok()
        );
        let e = refuse_unknown(&q(&[("patient", "p"), ("category", "food")]), &["patient"])
            .unwrap_err();
        assert!(e.contains("category"), "{e}");
    }

    #[test]
    fn audit_search_parameters_map_and_unsupported_date_prefixes_are_refused() {
        let pairs = |v: &[(&str, &str)]| -> Vec<(String, String)> {
            v.iter().map(|(k, v)| ((*k).into(), (*v).into())).collect()
        };
        let q = audit_query_from(&pairs(&[
            ("agent", "sub-1"),
            ("patient", "Patient/abc"),
            ("action", "R"),
            ("outcome", "4"),
            ("date", "ge2026-10-01"),
            ("date", "lt2026-11-01"),
            ("_before", "90"),
            ("_count", "5"),
        ]))
        .unwrap();
        assert_eq!(q.agent.as_deref(), Some("sub-1"));
        assert_eq!(q.patient.as_deref(), Some("abc"));
        assert_eq!((q.action.as_deref(), q.outcome), (Some("R"), Some(4)));
        assert_eq!(q.from.as_deref(), Some("2026-10-01"));
        assert_eq!(q.to.as_deref(), Some("2026-11-01"));
        assert_eq!(q.before, Some(90));
        let both = audit_query_from(&pairs(&[
            ("date", "gt2026-10-01"),
            ("date", "le2026-10-31"),
        ]))
        .unwrap();
        assert_eq!(both.after.as_deref(), Some("2026-10-01"));
        assert_eq!(both.through.as_deref(), Some("2026-10-31"));
        for bad in [
            ("date", "eq2026-10-01"),
            ("date", "2026-10-01"),
            ("outcome", "x"),
            ("_before", "x"),
            ("agnet", "sub-1"),
        ] {
            assert!(audit_query_from(&pairs(&[bad])).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn merge_status_becomes_fhir_links_and_an_inactive_old_record() {
        let base = patient_json(&stored("11111111-1111-4111-8111-111111111111", "Smith")).unwrap();
        // Live and unmerged: unchanged.
        assert_eq!(with_links(base.clone(), None, &[]).unwrap(), base);
        let old = with_links(
            base.clone(),
            Some("22222222-2222-4222-8222-222222222222"),
            &[],
        )
        .unwrap();
        assert_eq!(old["active"], false);
        assert_eq!(old["link"][0]["type"], "replaced-by");
        assert_eq!(
            old["link"][0]["other"]["reference"],
            "Patient/22222222-2222-4222-8222-222222222222"
        );
        let new = with_links(
            base,
            None,
            &["33333333-3333-4333-8333-333333333333".to_string()],
        )
        .unwrap();
        assert!(new.get("active").is_none());
        assert_eq!(new["link"][0]["type"], "replaces");
    }
}
