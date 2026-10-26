//! DB-gated, through the real router with login **on** (RC-6): who may record, read and retract an
//! observation, that a refused record is 422 with a code for each problem, that a proxy is refused until it
//! has a scope for observations, that a read needs a care relationship like any clinical content, and that
//! the audit trail never holds a measured value. One test, because the token guard is configured once per
//! process.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test http_observations -- --ignored --test-threads=1
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

fn unix() -> i64 {
    i64::try_from(nanos() / 1_000_000_000).unwrap()
}

/// A birth date no earlier run shares, so the duplicate check does not shortlist their patients.
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

fn observation(value: f64, unit: &str, status: &str) -> Value {
    json!({
        "code": "8867-4", "value": value, "unit": unit,
        "range": { "low": 60.0, "high": 100.0 },
        "effective_at": "2026-10-26T09:30:00Z", "status": status
    })
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn who_may_record_read_and_retract_an_observation() {
    let idp = TestIdp::start().await;
    let roles = [
        "clinical-admin",
        "clinician",
        "nurse",
        "receptionist",
        "proxy",
    ];
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
    const CLINICIAN: &str = "0c4f1e2a-0000-4000-8000-0000000000e1";
    const NURSE: &str = "0c4f1e2a-0000-4000-8000-0000000000e2";
    const PROXY: &str = "0c4f1e2a-0000-4000-8000-0000000000e3";
    let admin = token("clinical-admin", "0c4f1e2a-0000-4000-8000-0000000000e0");
    let clinician = token("clinician", CLINICIAN);
    let nurse = token("nurse", NURSE);
    let receptionist = token("receptionist", "0c4f1e2a-0000-4000-8000-0000000000e4");
    let proxy = token("proxy", PROXY);

    request::<App, _, _>(|server, ctx| async move {
        let (ah, av) = bearer(&admin);
        let (ch, cv) = bearer(&clinician);
        let (nh, nv) = bearer(&nurse);
        let (rh, rv) = bearer(&receptionist);
        let (ph, pv) = bearer(&proxy);
        let make = |label: &str, seed: u128| {
            let family = format!("obs{label}{}", nanos());
            json!({ "patient": serde_json::to_value(Patient {
                names: vec![HumanName { family: Some(family.clone()), given: vec![family] }],
                birth_date: Some(dob(seed)),
                ..Default::default()
            }).unwrap() })
        };
        let made = server
            .post("/api/patients")
            .add_header(ah.clone(), av.clone())
            .json(&make("a", 0))
            .await;
        assert_eq!(made.status_code(), StatusCode::CREATED, "{}", made.text());
        let pid = made.json::<Value>()["id"].as_str().unwrap().to_string();
        let stranger = server
            .post("/api/patients")
            .add_header(ah.clone(), av.clone())
            .json(&make("b", 777))
            .await;
        assert_eq!(stranger.status_code(), StatusCode::CREATED);
        let sid = stranger.json::<Value>()["id"].as_str().unwrap().to_string();

        // The clinician and the nurse are on this patient's care team; the proxy may see identity and
        // allergies, but no scope names observations.
        let rel = format!("/api/patients/{pid}/relationships");
        for (sub, kind, extra) in [
            (CLINICIAN, "care-team", Value::Null),
            (NURSE, "care-team", Value::Null),
            (
                PROXY,
                "proxy",
                json!({ "basis": "parental-responsibility", "scopes": ["identity", "allergies"],
                        "expires_at": unix() + 3600 }),
            ),
        ] {
            let mut relationship = json!({ "subject": sub, "kind": kind, "status": "active" });
            if !extra.is_null() {
                relationship["proxy"] = extra;
            }
            let r = server
                .post(&rel)
                .add_header(ah.clone(), av.clone())
                .json(&json!({ "relationship": relationship, "reason": "test" }))
                .await;
            assert_eq!(r.status_code(), StatusCode::CREATED, "{}", r.text());
        }
        let url = format!("/api/patients/{pid}/observations");

        // A clinician records one. The flag is computed from the sender's range, not stored.
        let created = server
            .post(&url)
            .add_header(ch.clone(), cv.clone())
            .json(&json!({ "observation": observation(71.37, "/min", "final"), "reason": "on arrival" }))
            .await;
        assert_eq!(created.status_code(), StatusCode::CREATED, "{}", created.text());
        let created: Value = created.json();
        assert_eq!(created["flag"], "normal");
        assert_eq!(created["version"], 1);
        let id = created["id"].as_str().unwrap().to_string();

        // A refused record is 422 with a code for each problem, in order, and is audited as a refusal.
        let refused = server
            .post(&url)
            .add_header(ch.clone(), cv.clone())
            .json(&json!({ "observation": {
                "code": "8310-5", "value": 98.6, "unit": "[degF]",
                "effective_at": "soon", "status": "final" } }))
            .await;
        assert_eq!(refused.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        let body: Value = refused.json();
        assert_eq!(body["error_codes"], json!(["unit_mismatch", "effective_at_invalid"]));
        assert_eq!(body["errors"].as_array().unwrap().len(), 2);
        let zero = server
            .post(&url)
            .add_header(ch.clone(), cv.clone())
            .json(&json!({ "observation": observation(0.0, "/min", "final") }))
            .await;
        assert_eq!(zero.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(zero.json::<Value>()["error_codes"], json!(["value_implausible"]));

        // A receptionist has no business with vital signs: refused by the route gate, read or write.
        assert_eq!(
            server.get(&url).add_header(rh.clone(), rv.clone()).await.status_code(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            server
                .post(&url)
                .add_header(rh.clone(), rv.clone())
                .json(&json!({ "observation": observation(70.0, "/min", "final") }))
                .await
                .status_code(),
            StatusCode::FORBIDDEN
        );

        // A proxy whose terms do not name observations is refused, though it may read the identity.
        assert_eq!(
            server.get(&url).add_header(ph.clone(), pv.clone())
                .add_header(
                    HeaderName::from_static("x-purpose-of-use"),
                    HeaderValue::from_static("patient-access"),
                )
                .await
                .status_code(),
            StatusCode::FORBIDDEN
        );

        // Clinical content needs a care relationship: the clinician has none with the stranger.
        let denied = server
            .get(&format!("/api/patients/{sid}/observations"))
            .add_header(ch.clone(), cv.clone())
            .await;
        assert_eq!(denied.status_code(), StatusCode::FORBIDDEN);
        assert_eq!(denied.json::<Value>()["break_glass_offer"], true);
        assert_eq!(
            server
                .post(&format!("/api/patients/{sid}/observations"))
                .add_header(ch.clone(), cv.clone())
                .json(&json!({ "observation": observation(70.0, "/min", "final") }))
                .await
                .status_code(),
            StatusCode::FORBIDDEN
        );

        // A nurse may record and correct, but not retract: marking a mistake is its own permission.
        let nurse_made = server
            .post(&url)
            .add_header(nh.clone(), nv.clone())
            .json(&json!({ "observation": observation(88.0, "/min", "final") }))
            .await;
        assert_eq!(nurse_made.status_code(), StatusCode::CREATED, "{}", nurse_made.text());
        let nurse_id = nurse_made.json::<Value>()["id"].as_str().unwrap().to_string();
        let corrected = server
            .put(&format!("{url}/{nurse_id}"))
            .add_header(nh.clone(), nv.clone())
            .json(&json!({ "observation": observation(86.0, "/min", "final"),
                           "expected_version": 1, "reason": "misread" }))
            .await;
        assert_eq!(corrected.status_code(), StatusCode::OK, "{}", corrected.text());
        let retract_nurse = server
            .put(&format!("{url}/{nurse_id}"))
            .add_header(nh.clone(), nv.clone())
            .json(&json!({ "observation": observation(86.0, "/min", "entered-in-error"),
                           "expected_version": 2 }))
            .await;
        assert_eq!(retract_nurse.status_code(), StatusCode::FORBIDDEN);
        let retract = server
            .put(&format!("{url}/{nurse_id}"))
            .add_header(ch.clone(), cv.clone())
            .json(&json!({ "observation": observation(86.0, "/min", "entered-in-error"),
                           "expected_version": 2, "reason": "wrong patient" }))
            .await;
        assert_eq!(retract.status_code(), StatusCode::OK, "{}", retract.text());
        // A stale version is a 409 naming the current one.
        let stale = server
            .put(&format!("{url}/{id}"))
            .add_header(ch.clone(), cv.clone())
            .json(&json!({ "observation": observation(72.0, "/min", "final"), "expected_version": 5 }))
            .await;
        assert_eq!(stale.status_code(), StatusCode::CONFLICT);

        // The list holds every status, each with its flag; the retracted one says so.
        let listed = server.get(&url).add_header(ch.clone(), cv.clone()).await;
        assert_eq!(listed.status_code(), StatusCode::OK, "{}", listed.text());
        let listed: Value = listed.json();
        let all = listed["observations"].as_array().unwrap();
        assert_eq!(all.len(), 2);
        assert!(all.iter().any(|o| o["status"] == "entered-in-error"));

        // The history keeps every version, oldest first, with its reason; the retraction does not hide
        // the values it replaced. The same access decision as the list applies.
        let versions_url = format!("{url}/{nurse_id}/versions");
        let history = server.get(&versions_url).add_header(ch.clone(), cv.clone()).await;
        assert_eq!(history.status_code(), StatusCode::OK, "{}", history.text());
        let history: Value = history.json();
        let v = history["versions"].as_array().unwrap();
        assert_eq!(v.len(), 3);
        assert_eq!(
            v.iter().map(|x| x["version"].as_i64().unwrap()).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert_eq!((v[0]["value"].as_f64(), v[1]["value"].as_f64()), (Some(88.0), Some(86.0)));
        assert_eq!(v[1]["reason"], "misread");
        assert_eq!(v[2]["status"], "entered-in-error");
        assert_eq!(
            server.get(&versions_url).add_header(rh.clone(), rv.clone()).await.status_code(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            server.get(&versions_url).add_header(ph.clone(), pv.clone()).await.status_code(),
            StatusCode::FORBIDDEN
        );
        // Under a patient the clinician has no relationship with, the history is refused, not "not found".
        assert_eq!(
            server
                .get(&format!("/api/patients/{sid}/observations/{nurse_id}/versions"))
                .add_header(ch.clone(), cv.clone())
                .await
                .status_code(),
            StatusCode::FORBIDDEN
        );

        // The audit trail names the records and never a measured value, a unit or a range.
        let rows = ctx
            .db
            .query_all_raw(Statement::from_string(
                sea_orm::DbBackend::Postgres,
                format!(
                    "SELECT detail::text AS d FROM audit_events WHERE resource_ref = 'Observation' AND patient_ref = 'Patient/{pid}'"
                ),
            ))
            .await
            .unwrap();
        assert!(rows.len() >= 5, "creates, a read and updates are audited: {}", rows.len());
        for row in rows {
            let d: String = row.try_get("", "d").unwrap();
            for secret in ["71.37", "/min", "\"low\"", "98.6", "degF"] {
                assert!(!d.contains(secret), "{secret} leaked into the audit row: {d}");
            }
        }
    })
    .await;
}
