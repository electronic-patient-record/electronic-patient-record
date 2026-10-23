//! DB-gated, through the real router with login on: tokens from a local OIDC provider
//! (`authentication_verifier::test_idp`) carry roles, mapped by the shipped
//! `operations/keycloak/role-map.json`. Checks the break-glass flow end to end (CQ-16): a clinician
//! with no care relationship is refused with the offer, a grant opens the record, the grant is
//! not extended by asking again, and the audit rows are flagged and reach the auditor's search.
//! Not covered: consent masking of duplicate candidates.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test http_authenticated -- --ignored --test-threads=1
//! ```

use authentication_verifier::test_idp::{AUDIENCE, REALM, TestIdp};
use axum::http::{HeaderName, HeaderValue, StatusCode};
use electronic_patient_record::app::App;
use electronic_patient_record_matcher::{HumanName, Patient};
use loco_rs::testing::prelude::request;
use serde_json::json;

fn bearer(idp: &TestIdp, sub: &str, role: &str) -> (HeaderName, HeaderValue) {
    let token = idp.token(|c| {
        c["sub"] = json!(sub);
        c["realm_access"] = json!({ "roles": [role] });
    });
    (
        HeaderName::from_static("authorization"),
        HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
    )
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn break_glass_opens_a_record_once_and_is_flagged_in_the_audit_trail() {
    let idp = TestIdp::start().await;
    // SAFETY: set before the app boots and before any thread of this test reads them; the suite runs
    // single-threaded and this binary holds one test (the guard reads its settings once).
    unsafe {
        std::env::set_var("EPR_REQUIRE_AUTH", "true");
        std::env::set_var("EPR_KEYCLOAK_URL", &idp.base_url);
        std::env::set_var("EPR_KEYCLOAK_REALM", REALM);
        std::env::set_var("EPR_KEYCLOAK_AUDIENCES", AUDIENCE);
        std::env::set_var(
            "EPR_KEYCLOAK_ROLE_MAP_FILE",
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../operations/keycloak/role-map.json"
            ),
        );
    }
    let admin = bearer(&idp, "admin-1", "clinical-admin");
    let doctor = bearer(&idp, "doctor-1", "clinician");
    let auditor = bearer(&idp, "auditor-1", "auditor");

    request::<App, _, _>(|server, _ctx| async move {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let family = format!("ZzBg{nanos}");
        let patient = serde_json::to_value(Patient {
            names: vec![HumanName {
                family: Some(family),
                given: vec!["Glass".into()],
            }],
            birth_date: Some("1969-07-20".into()),
            ..Default::default()
        })
        .unwrap();
        let created = server
            .post("/api/patients")
            .add_header(admin.0.clone(), admin.1.clone())
            .json(&json!({ "patient": patient, "override_reason": "test fixture, reruns share a birth date" }))
            .await;
        assert_eq!(
            created.status_code(),
            StatusCode::CREATED,
            "{}",
            created.text()
        );
        let id = created.json::<serde_json::Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();
        let allergies = format!("/api/patients/{id}/allergies");

        // No care relationship: refused, with the offer of break-the-glass.
        let refused = server
            .get(&allergies)
            .add_header(doctor.0.clone(), doctor.1.clone())
            .await;
        assert_eq!(
            refused.status_code(),
            StatusCode::FORBIDDEN,
            "{}",
            refused.text()
        );
        assert_eq!(
            refused.json::<serde_json::Value>()["break_glass_offer"],
            true
        );

        // A reason is required.
        let blank = server
            .post(&format!("/api/patients/{id}/break-glass"))
            .add_header(doctor.0.clone(), doctor.1.clone())
            .json(&json!({ "reason": "  " }))
            .await;
        assert!(blank.status_code().is_client_error(), "{}", blank.text());

        // Declared: a grant, then the record opens.
        let opened = server
            .post(&format!("/api/patients/{id}/break-glass"))
            .add_header(doctor.0.clone(), doctor.1.clone())
            .json(&json!({ "reason": "collapsed in the corridor" }))
            .await;
        assert_eq!(
            opened.status_code(),
            StatusCode::CREATED,
            "{}",
            opened.text()
        );
        let grant = opened.json::<serde_json::Value>();
        let again = server
            .post(&format!("/api/patients/{id}/break-glass"))
            .add_header(doctor.0.clone(), doctor.1.clone())
            .json(&json!({ "reason": "still there" }))
            .await;
        assert_eq!(again.status_code(), StatusCode::OK);
        assert_eq!(
            again.json::<serde_json::Value>()["expires_at"],
            grant["expires_at"],
            "asking again does not extend the grant"
        );
        let read = server
            .get(&allergies)
            .add_header(doctor.0.clone(), doctor.1.clone())
            .await;
        assert_eq!(read.status_code(), StatusCode::OK, "{}", read.text());

        // The auditor sees flagged rows for this patient.
        let audit = server
            .get(&format!("/api/audit?patient={id}&break_glass=true"))
            .add_header(auditor.0.clone(), auditor.1.clone())
            .await;
        assert_eq!(audit.status_code(), StatusCode::OK, "{}", audit.text());
        let rows = audit.json::<serde_json::Value>()["events"]
            .as_array()
            .cloned()
            .expect("events");
        assert_eq!(rows.len(), 3, "the grant, the repeat and the read: {rows:?}");
        assert!(rows.iter().all(|r| r["break_glass"] == true));
        assert!(
            !audit.text().contains("collapsed in the corridor"),
            "no free text in the trail"
        );

        // The grant waits in the review queue.
        let count = server
            .get("/api/break-glass/reviews/count")
            .add_header(auditor.0.clone(), auditor.1.clone())
            .add_header(
                HeaderName::from_static("x-purpose-of-use"),
                HeaderValue::from_static("audit"),
            )
            .await;
        assert_eq!(count.status_code(), StatusCode::OK, "{}", count.text());
        assert!(count.json::<serde_json::Value>()["unreviewed"].as_i64() >= Some(1));
    })
    .await;
}
