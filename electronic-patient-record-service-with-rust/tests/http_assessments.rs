//! DB-gated, through the real router with login **on** (WL-3): two versions of one form, answers under
//! each, both shown with their own version and that version's questions, the newest version named beside
//! them and each form a draft until approved; a missing answer, or no answer without a reason, refused;
//! who may record and read; and no answer or reason in the audit trail. One test, because the token
//! guard is configured once per process.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test http_assessments -- --ignored --test-threads=1
//! ```

use authentication_verifier::test_idp::{AUDIENCE, REALM, TestIdp};
use axa_test::prelude::request;
use axum::http::{HeaderName, HeaderValue, StatusCode};
use electronic_patient_record::app::App;
use electronic_patient_record::pool::Pool;
use electronic_patient_record_matcher::{HumanName, Patient};
use serde_json::{Value, json};
use sqlx::{AssertSqlSafe, Row};

fn nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

fn dob(seed: u128) -> String {
    let n = nanos() / 1000 + seed;
    format!(
        "{}-{:02}-{:02}",
        1900 + n % 100,
        1 + n / 100 % 12,
        1 + n / 1200 % 28
    )
}

fn bearer(token: &str) -> (HeaderName, HeaderValue) {
    (
        HeaderName::from_static("authorization"),
        HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
    )
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn answers_keep_the_form_version_they_were_given_under() {
    let idp = TestIdp::start().await;
    let roles = ["clinical-admin", "nurse", "receptionist"];
    let map: Value = roles
        .iter()
        .map(|r| ((*r).to_string(), json!({ "role": [r] })))
        .collect();
    // SAFETY: set before the app boots and before any other thread reads them; one test per process.
    unsafe {
        std::env::set_var("EPR_REQUIRE_AUTH", "true");
        std::env::set_var("EPR_KEYCLOAK_URL", &idp.base_url);
        std::env::set_var("EPR_KEYCLOAK_REALM", REALM);
        std::env::set_var("EPR_KEYCLOAK_AUDIENCES", AUDIENCE);
        std::env::set_var("EPR_KEYCLOAK_ROLE_MAP", map.to_string());
    }
    let token = |role: &str, sub: &str| {
        idp.token(|c| {
            c["realm_access"] = json!({ "roles": [role] });
            c["sub"] = json!(sub);
        })
    };
    const NURSE: &str = "0c4f1e2a-0000-4000-8000-0000000000c7";
    let admin = token("clinical-admin", "0c4f1e2a-0000-4000-8000-0000000000c6");
    let nurse = token("nurse", NURSE);
    let receptionist = token("receptionist", "0c4f1e2a-0000-4000-8000-0000000000c8");

    request::<App, _, _>(|server, ctx| async move {
        let (ah, av) = bearer(&admin);
        let (nh, nv) = bearer(&nurse);
        let (rh, rv) = bearer(&receptionist);
        let family = format!("assess{}", nanos());
        let made = server
            .post("/api/patients")
            .add_header(ah.clone(), av.clone())
            .json(&json!({ "patient": serde_json::to_value(Patient {
                names: vec![HumanName { family: Some(family.clone()), given: vec![family] }],
                birth_date: Some(dob(0)),
                ..Default::default()
            }).unwrap() }))
            .await;
        assert_eq!(made.status_code(), StatusCode::CREATED, "{}", made.text());
        let pid = made.json::<Value>()["id"].as_str().unwrap().to_string();
        let rel = server
            .post(&format!("/api/patients/{pid}/relationships"))
            .add_header(ah.clone(), av.clone())
            .json(&json!({ "relationship": { "subject": NURSE, "kind": "care-team", "status": "active" },
                           "reason": "test" }))
            .await;
        assert_eq!(rel.status_code(), StatusCode::CREATED, "{}", rel.text());
        let url = format!("/api/patients/{pid}/assessments");
        let post = |body: Value| {
            let server = &server;
            let (h, v) = (nh.clone(), nv.clone());
            let url = url.clone();
            async move { server.post(&url).add_header(h, v).json(&json!({ "assessment": body })).await }
        };

        // Refused: a question left out; no answer and no reason; an answer the question does not take.
        for body in [
            json!({ "form": "skin-check", "form_version": 1, "answers": [{ "question": "intact", "value": true }] }),
            json!({ "form": "skin-check", "form_version": 1, "answers": [
                { "question": "intact", "value": true }, { "question": "redness", "value": null }] }),
            json!({ "form": "skin-check", "form_version": 2, "answers": [
                { "question": "intact", "value": true }, { "question": "redness", "value": false },
                { "question": "where", "value": "knees" }] }),
        ] {
            let r = post(body.clone()).await;
            assert_eq!(r.status_code(), StatusCode::UNPROCESSABLE_ENTITY, "{body}: {}", r.text());
        }

        // Under version 1, with one question unanswered for a reason; then under version 2.
        let one = post(json!({ "form": "skin-check", "form_version": 1, "answers": [
            { "question": "intact", "value": true },
            { "question": "redness", "value": null, "absent_reason": "declined to be examined there" }] }))
        .await;
        assert_eq!(one.status_code(), StatusCode::CREATED, "{}", one.text());
        let two = post(json!({ "form": "skin-check", "form_version": 2, "answers": [
            { "question": "intact", "value": false }, { "question": "redness", "value": true },
            { "question": "where", "value": "sacrum" }] }))
        .await;
        assert_eq!(two.status_code(), StatusCode::CREATED, "{}", two.text());

        // Reception may not read or record them.
        assert_eq!(
            server.get(&url).add_header(rh.clone(), rv.clone()).await.status_code(),
            StatusCode::FORBIDDEN
        );

        // Both are shown with their own version and that version's questions.
        let list = server.get(&url).add_header(nh.clone(), nv.clone()).await;
        assert_eq!(list.status_code(), StatusCode::OK, "{}", list.text());
        let list: Value = list.json();
        assert_eq!(list["forms"].as_array().unwrap().len(), 2, "the catalogue: {list}");
        let items = list["assessments"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        let by_version = |v: i64| {
            items
                .iter()
                .find(|a| a["form"]["version"] == v)
                .unwrap_or_else(|| panic!("version {v}: {list}"))
        };
        let (v1, v2) = (by_version(1), by_version(2));
        assert_eq!(v1["answers"].as_array().unwrap().len(), 2);
        assert_eq!(v2["answers"].as_array().unwrap().len(), 3);
        assert_eq!(v1["answers"][1]["value"], Value::Null);
        assert_eq!(v1["answers"][1]["absent_reason"], "declined to be examined there");
        assert_eq!(v2["answers"][2]["text"], "Where is the skin damaged or red?");
        for a in [v1, v2] {
            assert_eq!(a["form"]["latest_version"], 2);
            assert_eq!(a["form"]["approved"], false, "a draft says so");
        }

        // The trail names the form version, never an answer or a reason.
        let rows = sqlx::query(AssertSqlSafe(format!(
            "SELECT detail::text AS d FROM audit_events WHERE resource_ref = 'QuestionnaireResponse' AND patient_ref = 'Patient/{pid}'"
        )))
        .fetch_all(&ctx.pool())
        .await
        .unwrap();
        assert!(rows.len() >= 3, "two records and a read: {}", rows.len());
        for row in rows {
            let d: String = row.try_get("d").unwrap();
            for secret in ["declined", "sacrum", "knees"] {
                assert!(!d.contains(secret), "{secret} leaked into the audit row: {d}");
            }
        }
    })
    .await;
}
