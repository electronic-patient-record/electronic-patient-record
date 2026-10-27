//! DB-gated, through the real router with login **on** (RC-4): who may ask for a record to be amended and
//! who may decide, that the requester is the token and never the body, that a patient cannot decide and a
//! clinician cannot speak for the patient, that a stale version is 409, and that the audit trail never holds
//! the request text or a reason. One test, because the token guard is configured once per process.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test http_amendments -- --ignored --test-threads=1
//! ```

use authentication_verifier::test_idp::{AUDIENCE, REALM, TestIdp};
use axum::http::{HeaderName, HeaderValue, StatusCode};
use electronic_patient_record::app::App;
use electronic_patient_record_matcher::{HumanName, Patient};
use loco_rs::testing::prelude::request;
use sea_orm::{ConnectionTrait, Statement};
use serde_json::{Value, json};

fn nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

/// A birth date no earlier run shares, so the duplicate check does not shortlist their patients.
fn dob() -> String {
    let n = nanos() / 1000;
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
async fn who_may_ask_decide_and_withdraw() {
    let idp = TestIdp::start().await;
    let roles = ["clinical-admin", "clinician", "patient", "receptionist"];
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
    const CLINICIAN: &str = "0c4f1e2a-0000-4000-8000-0000000000f1";
    const PATIENT: &str = "0c4f1e2a-0000-4000-8000-0000000000f2";
    let admin = token("clinical-admin", "0c4f1e2a-0000-4000-8000-0000000000f0");
    let clinician = token("clinician", CLINICIAN);
    let patient = token("patient", PATIENT);
    let receptionist = token("receptionist", "0c4f1e2a-0000-4000-8000-0000000000f3");

    request::<App, _, _>(|server, ctx| async move {
        let (ah, av) = bearer(&admin);
        let (ch, cv) = bearer(&clinician);
        let (ph, pv) = bearer(&patient);
        let (rh, rv) = bearer(&receptionist);
        let purpose = HeaderName::from_static("x-purpose-of-use");
        let access = HeaderValue::from_static("patient-access");

        let family = format!("amend{}", nanos());
        let made = server
            .post("/api/patients")
            .add_header(ah.clone(), av.clone())
            .json(&json!({ "patient": serde_json::to_value(Patient {
                names: vec![HumanName { family: Some(family.clone()), given: vec![family] }],
                birth_date: Some(dob()),
                ..Default::default()
            }).unwrap() }))
            .await;
        assert_eq!(made.status_code(), StatusCode::CREATED, "{}", made.text());
        let pid = made.json::<Value>()["id"].as_str().unwrap().to_string();
        for (sub, kind) in [(CLINICIAN, "care-team"), (PATIENT, "self")] {
            let r = server
                .post(&format!("/api/patients/{pid}/relationships"))
                .add_header(ah.clone(), av.clone())
                .json(&json!({
                    "relationship": { "subject": sub, "kind": kind, "status": "active" },
                    "reason": "test"
                }))
                .await;
            assert_eq!(r.status_code(), StatusCode::CREATED, "{}", r.text());
        }
        let recorded = server
            .post(&format!("/api/patients/{pid}/observations"))
            .add_header(ch.clone(), cv.clone())
            .json(&json!({ "observation": {
                "code": "8867-4", "value": 71.37, "unit": "/min",
                "effective_at": "2026-10-26T09:30:00Z", "status": "final" } }))
            .await;
        assert_eq!(recorded.status_code(), StatusCode::CREATED, "{}", recorded.text());
        let observation_id = recorded.json::<Value>()["id"].as_str().unwrap().to_string();

        let url = format!("/api/patients/{pid}/amendments");
        let ask = json!({
            "kind": "Observation", "target_id": observation_id, "version": 1,
            "text": "my pulse was never that fast"
        });

        // A clinician does not ask on the patient's behalf; a receptionist has no business here at all.
        assert_eq!(
            server.post(&url).add_header(ch.clone(), cv.clone()).json(&ask).await.status_code(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            server.get(&url).add_header(rh.clone(), rv.clone()).await.status_code(),
            StatusCode::FORBIDDEN
        );

        // The patient asks. The requester is the token, whatever the body says.
        let created = server
            .post(&url)
            .add_header(ph.clone(), pv.clone())
            .add_header(purpose.clone(), access.clone())
            .json(&json!({
                "kind": "Observation", "target_id": observation_id, "version": 1,
                "text": "my pulse was never that fast", "requester": "someone-else"
            }))
            .await;
        assert_eq!(created.status_code(), StatusCode::CREATED, "{}", created.text());
        let created: Value = created.json();
        let id = created["id"].as_str().unwrap().to_string();
        assert_eq!(created["version"], 1);
        assert_eq!(created["notice"]["notice"], "pending", "{created}");
        assert_eq!(created["request"]["requester"], PATIENT, "the token is the requester");
        assert!(created["due"].is_string());

        // A record that does not exist, or a version it lacks, is refused.
        let missing = server
            .post(&url)
            .add_header(ph.clone(), pv.clone())
            .add_header(purpose.clone(), access.clone())
            .json(&json!({ "kind": "Observation", "target_id": observation_id, "version": 9, "text": "x" }))
            .await;
        assert!(missing.status_code().is_client_error(), "{}", missing.text());

        // A step belongs to one role.
        let step = format!("{url}/{id}");
        assert_eq!(
            server
                .put(&step)
                .add_header(ph.clone(), pv.clone())
                .add_header(purpose.clone(), access.clone())
                .json(&json!({ "expected_version": 1, "step": "accept", "reason": "I say so" }))
                .await
                .status_code(),
            StatusCode::FORBIDDEN,
            "a patient does not decide"
        );
        assert_eq!(
            server
                .put(&step)
                .add_header(ch.clone(), cv.clone())
                .json(&json!({ "expected_version": 1, "step": "withdraw" }))
                .await
                .status_code(),
            StatusCode::FORBIDDEN,
            "a clinician does not speak for the patient"
        );

        // A decision needs a reason, a stale version is a conflict, and the right step goes through.
        let blank = server
            .put(&step)
            .add_header(ch.clone(), cv.clone())
            .json(&json!({ "expected_version": 1, "step": "accept", "reason": "  " }))
            .await;
        assert_eq!(blank.status_code(), StatusCode::UNPROCESSABLE_ENTITY, "{}", blank.text());
        let stale = server
            .put(&step)
            .add_header(ch.clone(), cv.clone())
            .json(&json!({ "expected_version": 5, "step": "accept", "reason": "the chart shows 62" }))
            .await;
        assert_eq!(stale.status_code(), StatusCode::CONFLICT, "{}", stale.text());
        let accepted = server
            .put(&step)
            .add_header(ch.clone(), cv.clone())
            .json(&json!({ "expected_version": 1, "step": "accept", "reason": "the chart shows 62" }))
            .await;
        assert_eq!(accepted.status_code(), StatusCode::OK, "{}", accepted.text());
        let accepted: Value = accepted.json();
        assert_eq!(accepted["version"], 2);

        // The patient lists it; a clinician reads it too.
        let listed = server
            .get(&url)
            .add_header(ph.clone(), pv.clone())
            .add_header(purpose.clone(), access.clone())
            .await;
        assert_eq!(listed.status_code(), StatusCode::OK, "{}", listed.text());
        assert_eq!(listed.json::<Value>()["amendments"].as_array().unwrap().len(), 1);
        assert_eq!(
            server.get(&url).add_header(ch.clone(), cv.clone()).await.status_code(),
            StatusCode::OK
        );

        // A reader of the observation is told it is under amendment, by the same words as the request.
        let obs = server
            .get(&format!("/api/patients/{pid}/observations"))
            .add_header(ch.clone(), cv.clone())
            .await;
        assert_eq!(obs.status_code(), StatusCode::OK, "{}", obs.text());
        let obs: Value = obs.json();
        let notices = &obs["amendment_notices"][&observation_id];
        assert_eq!(notices.as_array().map(Vec::len), Some(1), "{obs}");
        assert_eq!(notices[0], accepted["notice"], "{obs}");

        // An allergy can be asked about too, and its list carries the notice the same way.
        let allergy = server
            .post(&format!("/api/patients/{pid}/allergies"))
            .add_header(ch.clone(), cv.clone())
            .json(&json!({ "allergy": { "substance": "penicillin", "status": "active" } }))
            .await;
        assert_eq!(allergy.status_code(), StatusCode::CREATED, "{}", allergy.text());
        let allergy_id = allergy.json::<Value>()["id"].as_str().unwrap().to_string();
        let asked = server
            .post(&url)
            .add_header(ph.clone(), pv.clone())
            .add_header(purpose.clone(), access.clone())
            .json(&json!({
                "kind": "AllergyIntolerance", "target_id": allergy_id, "version": 1,
                "text": "I am not allergic to penicillin"
            }))
            .await;
        assert_eq!(asked.status_code(), StatusCode::CREATED, "{}", asked.text());
        let asked: Value = asked.json();
        let allergies = server
            .get(&format!("/api/patients/{pid}/allergies"))
            .add_header(ch.clone(), cv.clone())
            .await;
        assert_eq!(allergies.status_code(), StatusCode::OK, "{}", allergies.text());
        let allergies: Value = allergies.json();
        assert_eq!(
            allergies["amendment_notices"][&allergy_id],
            json!([asked["notice"]]),
            "{allergies}"
        );
        assert!(allergies["amendment_notices"].get(&observation_id).is_none());

        // The audit trail names the request and its state, never the words in it.
        let rows = ctx
            .db
            .query_all_raw(Statement::from_string(
                sea_orm::DbBackend::Postgres,
                format!(
                    "SELECT detail::text AS d FROM audit_events WHERE resource_ref = 'Amendment' AND patient_ref = 'Patient/{pid}'"
                ),
            ))
            .await
            .unwrap();
        assert!(rows.len() >= 4, "create, two reads and a step are audited: {}", rows.len());
        for row in rows {
            let d: String = row.try_get("", "d").unwrap();
            for secret in ["pulse", "never that fast", "chart shows", "someone-else", "penicillin"] {
                assert!(!d.contains(secret), "{secret} leaked into the audit row: {d}");
            }
        }
    })
    .await;
}
