//! DB-gated, through the real router with login off: the shapes of version conflicts (409
//! `{"error": "version conflict", "current_version": n}`) on patients, allergies, the allergy
//! statement and photos, and of the refusals that merge and unmerge answer instead (422 with the
//! reason). A merge race that ends in 409 is tested at the store (`tests/links_store.rs`), not
//! here: it cannot be made to happen in sequence (CQ-16).
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test http_conflicts -- --ignored --test-threads=1
//! ```

use axum::http::{HeaderName, HeaderValue, StatusCode};
use electronic_patient_record::app::App;
use electronic_patient_record_matcher::{HumanName, Patient};
use loco_rs::testing::prelude::request;
use serde_json::{Value, json};

const PNG: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 1, 2, 3];

/// Letters only, from the clock: no earlier run's patient looks like a duplicate.
fn tag(label: &str) -> String {
    let mut n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mut letters = String::new();
    while n > 0 {
        letters.push((b'a' + (n % 26) as u8) as char);
        n /= 26;
    }
    format!("{label}{letters}")
}

fn patient(family: &str, seed: u128) -> Value {
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
        / 1000
        + seed;
    serde_json::to_value(Patient {
        names: vec![HumanName {
            family: Some(family.into()),
            given: vec![family.into()],
        }],
        birth_date: Some(format!(
            "{}-{:02}-{:02}",
            1900 + n % 100,
            1 + n / 100 % 12,
            1 + n / 1200 % 28
        )),
        ..Default::default()
    })
    .unwrap()
}

fn login_off() {
    // SAFETY: set before any thread of this test reads it, and the suite runs single-threaded.
    unsafe { std::env::set_var("EPR_REQUIRE_AUTH", "false") };
}

fn conflict(body: &Value, current: i64) {
    assert_eq!(
        body,
        &json!({ "error": "version conflict", "current_version": current })
    );
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn stale_versions_are_409_with_the_current_version_and_bad_merges_are_422() {
    login_off();
    request::<App, _, _>(|server, _ctx| async move {
        let mut ids = Vec::new();
        for seed in [0, 7_000_000] {
            let created = server
                .post("/api/patients")
                .json(&json!({ "patient": patient(&tag("conf"), seed) }))
                .await;
            assert_eq!(created.status_code(), StatusCode::CREATED, "{}", created.text());
            ids.push(created.json::<Value>()["id"].as_str().unwrap().to_string());
        }
        let id = &ids[0];

        // Patient: a stale PUT.
        let stored = server.get(&format!("/api/patients/{id}")).await.json::<Value>();
        let put = server
            .put(&format!("/api/patients/{id}"))
            .json(&json!({ "patient": stored["patient"], "expected_version": 9 }))
            .await;
        assert_eq!(put.status_code(), StatusCode::CONFLICT, "{}", put.text());
        conflict(&put.json::<Value>(), 1);

        // Allergy: create, then change from a stale version.
        let allergy = json!({ "substance": "penicillin", "status": "active" });
        let made = server
            .post(&format!("/api/patients/{id}/allergies"))
            .json(&json!({ "allergy": allergy }))
            .await;
        assert_eq!(made.status_code(), StatusCode::CREATED, "{}", made.text());
        let made = made.json::<Value>();
        let allergy_id = made["id"].as_str().or(made["allergy_id"].as_str()).expect("id");
        let stale = server
            .put(&format!("/api/patients/{id}/allergies/{allergy_id}"))
            .json(&json!({ "allergy": allergy, "expected_version": 5 }))
            .await;
        assert_eq!(stale.status_code(), StatusCode::CONFLICT, "{}", stale.text());
        conflict(&stale.json::<Value>(), 1);

        // The allergy statement: state it, then state it again from version 0.
        let statement = format!("/api/patients/{id}/allergy-statement");
        // "No known allergies" is refused while an allergy is active, so withdraw first by ending it.
        let ended = server
            .put(&format!("/api/patients/{id}/allergies/{allergy_id}"))
            .json(&json!({ "allergy": { "substance": "penicillin", "status": "inactive" }, "expected_version": 1 }))
            .await;
        assert_eq!(ended.status_code(), StatusCode::OK, "{}", ended.text());
        let first = server
            .put(&statement)
            .json(&json!({ "none_known": true, "expected_version": 0 }))
            .await;
        assert!(first.status_code().is_success(), "{}", first.text());
        let again = server
            .put(&statement)
            .json(&json!({ "none_known": true, "expected_version": 0 }))
            .await;
        assert_eq!(again.status_code(), StatusCode::CONFLICT, "{}", again.text());
        conflict(&again.json::<Value>(), 1);

        // Photo: put from 0, then put and withdraw from a stale version.
        let photo = format!("/api/patients/{id}/photo");
        let png = HeaderValue::from_static("image/png");
        let ct = HeaderName::from_static("content-type");
        let first = server
            .put(&format!("{photo}?expected_version=0"))
            .add_header(ct.clone(), png.clone())
            .bytes(PNG.to_vec().into())
            .await;
        assert!(first.status_code().is_success(), "{}", first.text());
        let stale_put = server
            .put(&format!("{photo}?expected_version=0"))
            .add_header(ct, png)
            .bytes(PNG.to_vec().into())
            .await;
        assert_eq!(stale_put.status_code(), StatusCode::CONFLICT, "{}", stale_put.text());
        conflict(&stale_put.json::<Value>(), 1);
        let stale_withdraw = server
            .post(&format!("{photo}/withdraw?expected_version=4"))
            .await;
        assert_eq!(stale_withdraw.status_code(), StatusCode::CONFLICT);
        conflict(&stale_withdraw.json::<Value>(), 1);

        // Consent and relationship: a stale change.
        let consent = json!({ "kind": "general", "status": "active",
            "provisions": [{ "effect": "permit", "purposes": ["treatment"] }] });
        let made = server
            .post(&format!("/api/patients/{id}/consents"))
            .json(&json!({ "consent": consent }))
            .await;
        assert_eq!(made.status_code(), StatusCode::CREATED, "{}", made.text());
        let consent_id = made.json::<Value>()["id"].as_str().expect("id").to_string();
        let stale = server
            .put(&format!("/api/patients/{id}/consents/{consent_id}"))
            .json(&json!({ "consent": consent, "expected_version": 4 }))
            .await;
        assert_eq!(stale.status_code(), StatusCode::CONFLICT, "{}", stale.text());
        conflict(&stale.json::<Value>(), 1);
        let rel = json!({ "subject": "nurse-1", "kind": "care-team", "status": "active" });
        let made = server
            .post(&format!("/api/patients/{id}/relationships"))
            .json(&json!({ "relationship": rel }))
            .await;
        assert_eq!(made.status_code(), StatusCode::CREATED, "{}", made.text());
        let rel_id = made.json::<Value>()["id"].as_str().expect("id").to_string();
        let stale = server
            .put(&format!("/api/patients/{id}/relationships/{rel_id}"))
            .json(&json!({ "relationship": rel, "expected_version": 4 }))
            .await;
        assert_eq!(stale.status_code(), StatusCode::CONFLICT, "{}", stale.text());
        conflict(&stale.json::<Value>(), 1);

        // FHIR: a miss and an unsupported search are OperationOutcomes, not the JSON error shapes.
        let missing = server
            .get("/fhir/R4/Patient/00000000-0000-4000-8000-000000000000")
            .await;
        assert_eq!(missing.status_code(), StatusCode::NOT_FOUND);
        assert_eq!(missing.json::<Value>()["resourceType"], "OperationOutcome");
        let prefix = server.get("/fhir/R4/Patient?birthdate=gt1990-01-01").await;
        assert_eq!(prefix.status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(prefix.json::<Value>()["resourceType"], "OperationOutcome");

        // Merge and unmerge: refusals are 422 with the reason, never a 409 or a 500.
        let into_self = server
            .post(&format!("/api/patients/{id}/merge"))
            .json(&json!({ "source": id, "reason": "same" }))
            .await;
        assert_eq!(into_self.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        assert!(into_self.json::<Value>()["errors"].is_array());
        let not_merged = server
            .post(&format!("/api/patients/{id}/unmerge"))
            .json(&json!({ "source": ids[1], "reason": "oops" }))
            .await;
        assert_eq!(not_merged.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        assert!(not_merged.text().contains("not currently merged"));
    })
    .await;
}
