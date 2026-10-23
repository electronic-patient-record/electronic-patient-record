//! DB-gated, through the real router (`controllers::all_routes()`, the route gate and the audit
//! chain, booted by `loco_rs::testing` with login off): the shapes of the duplicate refusal and
//! of the FHIR refusals (CQ-16). Not covered here: consent masking of candidates and the
//! break-glass flag, which need tokens with roles and are listed in `spec/index.md` as by-hand.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test http_patients -- --ignored --test-threads=1
//! ```

use axum::http::{HeaderName, HeaderValue, StatusCode};
use electronic_patient_record::app::App;
use electronic_patient_record_matcher::{HumanName, Patient};
use loco_rs::testing::prelude::request;

fn tag(label: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("Zz{label}{nanos}")
}

fn patient(family: &str, dob: &str) -> serde_json::Value {
    serde_json::to_value(Patient {
        names: vec![HumanName {
            family: Some(family.into()),
            given: vec!["Test".into()],
        }],
        birth_date: Some(dob.into()),
        ..Default::default()
    })
    .unwrap()
}

/// Login off, set before the app boots (the guard reads it once).
fn login_off() {
    // SAFETY: set before any thread of this test reads it, and the suite runs single-threaded.
    unsafe { std::env::set_var("EPR_REQUIRE_AUTH", "false") };
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_likely_duplicate_is_409_with_candidates_and_an_override_needs_a_reason() {
    login_off();
    request::<App, _, _>(|server, _ctx| async move {
        let family = tag("dup");
        let body = serde_json::json!({ "patient": patient(&family, "1980-03-14") });
        let first = server.post("/api/patients").json(&body).await;
        assert_eq!(first.status_code(), StatusCode::CREATED, "{}", first.text());

        let again = server.post("/api/patients").json(&body).await;
        assert_eq!(again.status_code(), StatusCode::CONFLICT);
        let json: serde_json::Value = again.json();
        let candidates = json["candidates"]
            .as_array()
            .expect("candidates in the 409");
        assert!(!candidates.is_empty());
        assert!(candidates[0].get("score").is_some());

        let blank = serde_json::json!({
            "patient": patient(&family, "1980-03-14"), "override_reason": "  "
        });
        let refused = server.post("/api/patients").json(&blank).await;
        assert_eq!(refused.status_code(), StatusCode::CONFLICT);

        let with_reason = serde_json::json!({
            "patient": patient(&family, "1980-03-14"), "override_reason": "twins"
        });
        let ok = server.post("/api/patients").json(&with_reason).await;
        assert_eq!(ok.status_code(), StatusCode::CREATED, "{}", ok.text());
    })
    .await;
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn fhir_refusals_are_operation_outcomes_and_never_carry_candidates() {
    login_off();
    request::<App, _, _>(|server, _ctx| async move {
        let family = tag("fhir");
        let created = server
            .post("/api/patients")
            .json(&serde_json::json!({ "patient": patient(&family, "1975-06-01") }))
            .await;
        assert_eq!(created.status_code(), StatusCode::CREATED);
        let id = created.json::<serde_json::Value>()["id"]
            .as_str()
            .expect("id")
            .to_string();

        // Update without If-Match: 428 OperationOutcome.
        let read = server.get(&format!("/fhir/R4/Patient/{id}")).await;
        assert_eq!(read.status_code(), StatusCode::OK);
        assert_eq!(read.header("etag"), "W/\"1\"");
        let resource: serde_json::Value = read.json();
        let put = server
            .put(&format!("/fhir/R4/Patient/{id}"))
            .json(&resource)
            .await;
        assert_eq!(put.status_code(), StatusCode::PRECONDITION_REQUIRED);
        assert_eq!(
            put.json::<serde_json::Value>()["resourceType"],
            "OperationOutcome"
        );

        // A stale If-Match: 412.
        let stale = server
            .put(&format!("/fhir/R4/Patient/{id}"))
            .add_header(
                HeaderName::from_static("if-match"),
                HeaderValue::from_static("W/\"7\""),
            )
            .json(&resource)
            .await;
        assert_eq!(stale.status_code(), StatusCode::PRECONDITION_FAILED);

        // A duplicate through FHIR: 409, an OperationOutcome, no candidates.
        let mut dup = resource.clone();
        dup.as_object_mut().unwrap().remove("id");
        let conflict = server.post("/fhir/R4/Patient").json(&dup).await;
        assert_eq!(conflict.status_code(), StatusCode::CONFLICT);
        let outcome: serde_json::Value = conflict.json();
        assert_eq!(outcome["resourceType"], "OperationOutcome");
        assert!(!outcome.to_string().contains(&family));

        // An element the record cannot hold: 422, not stored with it dropped.
        let mut marital = dup;
        marital["maritalStatus"] = serde_json::json!({ "text": "married" });
        marital["name"][0]["family"] = serde_json::json!(tag("fhir2"));
        let unprocessable = server.post("/fhir/R4/Patient").json(&marital).await;
        assert_eq!(
            unprocessable.status_code(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
    })
    .await;
}
