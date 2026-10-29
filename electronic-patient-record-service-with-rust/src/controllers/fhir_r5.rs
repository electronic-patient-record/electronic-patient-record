//! `/fhir/R5`: a read-only FHIR R5 view of the record, derived from the R4 answer (IO-16).
//!
//! R4 is the only shape the service writes or stores. Each route here calls its R4 twin, so it has
//! the same gate, care relationship, purpose of use, consent and audit row, and then converts the
//! body with the `fhir` crate, which reports everything it changed or dropped. A conversion that
//! loses anything on a clinical resource fails the response: a quieter record is not served.

use super::fhir::{self as r4, fhir, outcome};
use crate::auth::Agent;
use crate::policy::Subject;
use axum::{
    Extension,
    extract::{OriginalUri, Path, Query, State},
    http::{HeaderMap, HeaderName, StatusCode, header},
    response::Response,
};
use fhir::{convert, r4::R4, r5::R5};
use loco_rs::prelude::*;
use std::collections::HashMap;

const R4_BASE: &str = "/fhir/R4/";
const R5_BASE: &str = "/fhir/R5/";

/// A conversion that cannot be served: what was lost, never the content.
#[derive(Debug, PartialEq, Eq)]
pub struct Lossy(pub String);

/// An R4 body as R5 JSON, answering `request` (path and query). A resource other than `OperationOutcome` must convert with no loss and no
/// warning (`strict`), including every resource inside a `Bundle`; otherwise the paths that lost
/// something are returned. The paths in the report are element paths, not values.
///
/// # Errors
///
/// [`Lossy`] with the element paths the conversion changed or dropped.
pub fn to_r5(body: &serde_json::Value, request: &str) -> Result<serde_json::Value, Lossy> {
    let mut body = body.clone();
    retype(&mut body);
    let converted = convert::between::<R4, R5>(&body);
    let tolerant = body["resourceType"] == "OperationOutcome";
    if !tolerant && !converted.report.is_lossless() {
        let paths: Vec<String> = converted
            .report
            .iter()
            .map(|l| format!("{} ({})", l.path, l.kind))
            .collect();
        return Err(Lossy(paths.join(", ")));
    }
    let mut value = converted.value;
    rebase(&mut value);
    // R5 requires a self link on a search set (bdl-18); R4 did not, so the R4 body has none.
    if value["resourceType"] == "Bundle"
        && value["type"] == "searchset"
        && value.get("link").is_none()
    {
        value["link"] = serde_json::json!([{ "relation": "self", "url": request }]);
    }
    Ok(value)
}

/// The one retyping this service's R4 output needs. The converter is structural and does not know a
/// retyped element: `AllergyIntolerance.reaction.manifestation` is a `CodeableConcept` in R4 and a
/// `CodeableReference` in R5, whose `concept` holds the R4 value unchanged. Left alone, the text
/// would be reported as dropped and every allergy with a reaction would fail. Any other retyped
/// element is still reported and still fails the response.
fn retype(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Array(items) => items.iter_mut().for_each(retype),
        serde_json::Value::Object(map) => {
            if map.get("resourceType").and_then(|t| t.as_str()) == Some("AllergyIntolerance")
                && let Some(reactions) = map.get_mut("reaction").and_then(|r| r.as_array_mut())
            {
                for reaction in reactions {
                    if let Some(items) = reaction
                        .get_mut("manifestation")
                        .and_then(|m| m.as_array_mut())
                    {
                        for m in items {
                            if m.get("concept").is_none() && m.get("reference").is_none() {
                                *m = serde_json::json!({ "concept": m.take() });
                            }
                        }
                    }
                }
            }
            map.values_mut().for_each(retype);
        }
        _ => {}
    }
}

/// Point the links the R4 handlers wrote at this release.
fn rebase(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::String(s) => {
            if let Some(rest) = s.strip_prefix(R4_BASE) {
                *s = format!("{R5_BASE}{rest}");
            }
        }
        serde_json::Value::Array(items) => items.iter_mut().for_each(rebase),
        serde_json::Value::Object(map) => map.values_mut().for_each(rebase),
        _ => {}
    }
}

/// An R4 response as an R5 one: the same status and headers (the weak `ETag`), the converted body.
/// A body that is not FHIR JSON, or that loses something in conversion, becomes a 500 outcome that
/// says nothing about the record.
async fn derive(response: Response, request: &str) -> Response {
    let (parts, body) = response.into_parts();
    let Ok(bytes) = axum::body::to_bytes(body, 4 << 20).await else {
        return failed("the answer could not be read");
    };
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return failed("the answer was not FHIR JSON");
    };
    match to_r5(&value, request) {
        Ok(converted) => {
            let mut out = fhir(parts.status, &converted, None);
            for name in [
                header::ETAG,
                header::LOCATION,
                HeaderName::from_static("vary"),
            ] {
                if let Some(v) = parts.headers.get(&name) {
                    out.headers_mut().insert(name, v.clone());
                }
            }
            out
        }
        Err(Lossy(paths)) => {
            tracing::error!(%paths, "R4 to R5 conversion lost something; not served");
            failed("this record cannot be shown as R5 without losing something")
        }
    }
}

/// The path and query as the client sent them.
fn request_of(uri: &axum::http::Uri) -> String {
    uri.path_and_query()
        .map_or_else(|| uri.path().to_string(), ToString::to_string)
}

fn failed(why: &str) -> Response {
    outcome(StatusCode::INTERNAL_SERVER_ERROR, "exception", why)
}

/// A request that names another release in `Accept` is refused, never answered in this one.
fn wrong_release(headers: &HeaderMap) -> Option<Response> {
    let accept = headers.get(header::ACCEPT)?.to_str().ok()?;
    let named = accept
        .split(',')
        .flat_map(|part| part.split(';').skip(1))
        .filter_map(|p| p.trim().strip_prefix("fhirVersion="))
        .map(|v| v.trim_matches('"').to_string())
        .find(|v| !(v == "5.0" || v.starts_with("5.0.")));
    named.map(|v| {
        outcome(
            StatusCode::NOT_ACCEPTABLE,
            "not-supported",
            &format!("this is FHIR R5 (5.0.0); fhirVersion {v} was asked for"),
        )
    })
}

async fn read_patient(
    OriginalUri(uri): OriginalUri,
    state: State<AppContext>,
    agent: Extension<Agent>,
    subject: Extension<Subject>,
    headers: HeaderMap,
    id: Path<String>,
) -> Response {
    if let Some(refused) = wrong_release(&headers) {
        return refused;
    }
    derive(
        r4::read_patient(state, agent, subject, headers, id).await,
        &request_of(&uri),
    )
    .await
}

async fn search_patients(
    OriginalUri(uri): OriginalUri,
    state: State<AppContext>,
    agent: Extension<Agent>,
    subject: Extension<Subject>,
    headers: HeaderMap,
    params: Query<HashMap<String, String>>,
) -> Response {
    if let Some(refused) = wrong_release(&headers) {
        return refused;
    }
    derive(
        r4::search_patients(state, agent, subject, headers, params).await,
        &request_of(&uri),
    )
    .await
}

async fn search_allergies(
    OriginalUri(uri): OriginalUri,
    state: State<AppContext>,
    agent: Extension<Agent>,
    subject: Extension<Subject>,
    headers: HeaderMap,
    params: Query<HashMap<String, String>>,
) -> Response {
    if let Some(refused) = wrong_release(&headers) {
        return refused;
    }
    derive(
        r4::search_allergies(state, agent, subject, headers, params).await,
        &request_of(&uri),
    )
    .await
}

async fn search_observations(
    OriginalUri(uri): OriginalUri,
    state: State<AppContext>,
    agent: Extension<Agent>,
    subject: Extension<Subject>,
    headers: HeaderMap,
    params: Query<HashMap<String, String>>,
) -> Response {
    if let Some(refused) = wrong_release(&headers) {
        return refused;
    }
    derive(
        r4::search_observations(state, agent, subject, headers, params).await,
        &request_of(&uri),
    )
    .await
}

/// The R5 capability statement: base R5, reads only, no UK Core or US Core claim.
fn capability() -> serde_json::Value {
    let read_search = |kind: &str, interactions: &[&str]| {
        serde_json::json!({
            "type": kind,
            "interaction": interactions.iter().map(|c| serde_json::json!({ "code": c })).collect::<Vec<_>>(),
        })
    };
    serde_json::json!({
        "resourceType": "CapabilityStatement",
        "status": "active",
        "date": "2026-10-29",
        "kind": "instance",
        "software": { "name": "electronic-patient-record", "version": env!("CARGO_PKG_VERSION") },
        "fhirVersion": "5.0.0",
        "format": ["json"],
        "description": "NOT FOR CLINICAL USE. A read-only R5 view derived from the R4 record: Patient (read, search-type), AllergyIntolerance and Observation (search-type, by patient). Base R5 only: no UK Core or US Core conformance is claimed. A resource that cannot be converted without losing something is not served.",
        "rest": [{ "mode": "server", "resource": [
            read_search("Patient", &["read", "search-type"]),
            read_search("AllergyIntolerance", &["search-type"]),
            read_search("Observation", &["search-type"]),
        ] }],
    })
}

async fn metadata(headers: HeaderMap) -> Response {
    if let Some(refused) = wrong_release(&headers) {
        return refused;
    }
    fhir(StatusCode::OK, &capability(), None)
}

/// Routes under `/fhir/R5`.
#[must_use]
pub fn routes() -> Routes {
    Routes::new()
        .prefix("fhir/R5")
        .add("/metadata", get(metadata))
        .add("/Patient", get(search_patients))
        .add("/Patient/{id}", get(read_patient))
        .add("/AllergyIntolerance", get(search_allergies))
        .add("/Observation", get(search_observations))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_patient_converts_and_keeps_what_it_said() {
        let r4 = json!({
            "resourceType": "Patient", "id": "p1", "active": true,
            "name": [{ "family": "Smith", "given": ["Ann"] }],
            "birthDate": "1980-03-14", "gender": "female",
        });
        let r5 = to_r5(&r4, "/fhir/R5/x").unwrap();
        assert_eq!(r5["name"][0]["family"], "Smith");
        assert_eq!(r5["birthDate"], "1980-03-14");
    }

    #[test]
    fn a_field_r5_cannot_hold_fails_the_conversion_instead_of_vanishing() {
        let r4 = json!({
            "resourceType": "Patient", "id": "p1",
            "notAnElementOfAnyRelease": "x",
        });
        let Err(Lossy(paths)) = to_r5(&r4, "/fhir/R5/x") else {
            panic!("a dropped element must fail the conversion");
        };
        assert!(paths.contains("notAnElementOfAnyRelease"), "{paths}");
    }

    #[test]
    fn a_loss_inside_a_bundle_entry_fails_the_whole_bundle() {
        let r4 = json!({
            "resourceType": "Bundle", "type": "searchset",
            "entry": [{ "resource": { "resourceType": "Patient", "id": "p1", "bogus": 1 } }],
        });
        assert!(to_r5(&r4, "/fhir/R5/x").is_err());
    }

    #[test]
    fn a_reaction_keeps_its_manifestation_as_a_concept_and_other_retypings_still_fail() {
        let allergy = |reaction: serde_json::Value| {
            json!({ "resourceType": "AllergyIntolerance", "id": "a1",
                "patient": { "reference": "Patient/p1" }, "reaction": [reaction] })
        };
        let r5 = to_r5(
            &allergy(json!({ "manifestation": [{ "text": "rash" }] })),
            "/fhir/R5/x",
        )
        .unwrap();
        assert_eq!(
            r5["reaction"][0]["manifestation"][0]["concept"]["text"],
            "rash"
        );
        // `type` is a code in R4 and a concept in R5: not remapped, so it fails rather than changes.
        let mut typed = allergy(json!({ "manifestation": [{ "text": "rash" }] }));
        typed["type"] = json!("allergy");
        assert!(to_r5(&typed, "/fhir/R5/x").is_err());
    }

    #[test]
    fn links_point_at_r5_and_a_refusal_is_still_an_outcome() {
        let r4 = json!({
            "resourceType": "Bundle", "type": "searchset",
            "entry": [{ "fullUrl": "/fhir/R4/Patient/p1", "resource": { "resourceType": "Patient", "id": "p1" } }],
        });
        let r5 = to_r5(&r4, "/fhir/R5/x").unwrap();
        assert_eq!(r5["entry"][0]["fullUrl"], "/fhir/R5/Patient/p1");
        let oo = json!({ "resourceType": "OperationOutcome", "issue": [{ "severity": "error", "code": "forbidden", "diagnostics": "forbidden" }] });
        assert_eq!(
            to_r5(&oo, "/fhir/R5/x").unwrap()["issue"][0]["code"],
            "forbidden"
        );
    }

    /// A record R5 cannot hold is not served in a quieter form: the route answers 500 with an outcome
    /// that says nothing of the record, and the lost field's value never reaches the body.
    #[tokio::test]
    async fn a_planted_lossy_field_makes_the_route_fail_instead_of_answer() {
        let r4 = fhir(
            StatusCode::OK,
            &json!({ "resourceType": "Patient", "id": "p1", "plantedLossyField": "SECRET-VALUE" }),
            Some(2),
        );
        let out = derive(r4, "/fhir/R5/Patient/p1").await;
        assert_eq!(out.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let bytes = axum::body::to_bytes(out.into_body(), 1 << 20)
            .await
            .unwrap();
        let text = String::from_utf8(bytes.to_vec()).unwrap();
        assert!(text.contains("OperationOutcome"), "{text}");
        assert!(
            !text.contains("SECRET-VALUE") && !text.contains("plantedLossyField"),
            "{text}"
        );
    }

    #[tokio::test]
    async fn a_good_answer_keeps_its_status_and_etag() {
        let r4 = fhir(
            StatusCode::OK,
            &json!({ "resourceType": "Patient", "id": "p1" }),
            Some(2),
        );
        let out = derive(r4, "/fhir/R5/Patient/p1").await;
        assert_eq!(out.status(), StatusCode::OK);
        assert_eq!(out.headers()[header::ETAG], "W/\"2\"");
    }

    #[test]
    fn a_search_set_gets_the_self_link_r5_requires_and_nothing_else_does() {
        let set = json!({ "resourceType": "Bundle", "type": "searchset", "entry": [] });
        let r5 = to_r5(&set, "/fhir/R5/Patient?family=Smith").unwrap();
        assert_eq!(r5["link"][0]["relation"], "self");
        assert_eq!(r5["link"][0]["url"], "/fhir/R5/Patient?family=Smith");
        let one = json!({ "resourceType": "Patient", "id": "p1" });
        assert!(
            to_r5(&one, "/fhir/R5/Patient/p1")
                .unwrap()
                .get("link")
                .is_none()
        );
    }

    #[test]
    fn another_release_in_accept_is_refused_and_this_one_is_not() {
        let with = |accept: &str| {
            let mut h = HeaderMap::new();
            h.insert(header::ACCEPT, accept.parse().unwrap());
            h
        };
        assert!(wrong_release(&with("application/fhir+json; fhirVersion=4.0")).is_some());
        assert!(wrong_release(&with("application/fhir+json; fhirVersion=5.0")).is_none());
        assert!(wrong_release(&with("application/fhir+json")).is_none());
        assert!(wrong_release(&HeaderMap::new()).is_none());
    }

    #[test]
    fn the_capability_statement_says_r5_reads_and_no_national_profile() {
        let c = capability();
        assert_eq!(c["fhirVersion"], "5.0.0");
        let text = c.to_string();
        assert!(text.contains("no UK Core or US Core conformance is claimed"));
        assert!(!text.contains("\"create\"") && !text.contains("\"update\""));
    }
}
