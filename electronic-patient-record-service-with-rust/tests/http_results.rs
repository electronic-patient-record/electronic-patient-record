//! DB-gated, through the real router with login **on** (OR-4 first slice, WL-1): a sign-off names the
//! result version it was made on. A result corrected after sign-off reads as changed since sign-off, with
//! both versions shown; signing the old version again is a 409; signing the new one closes it. Also: who
//! may file, read and sign; that a change needs a reason; that entered in error has nothing to sign; and
//! that the audit trail never holds a test name or a value. One test, because the token guard is
//! configured once per process.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test http_results -- --ignored --test-threads=1
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

fn potassium(value: &str, status: &str) -> Value {
    json!({
        "test": "Serum potassium", "value": value, "unit": "mmol/L",
        "reference_range": "3.5-5.3", "interpretation": "abnormal",
        "status": status, "issued": "2026-11-09T09:00:00Z", "performer": "Pathology"
    })
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_sign_off_names_the_version_and_a_change_reopens_it() {
    let idp = TestIdp::start().await;
    let roles = ["clinical-admin", "clinician", "nurse", "receptionist"];
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
    const NURSE: &str = "0c4f1e2a-0000-4000-8000-0000000000f2";
    let admin = token("clinical-admin", "0c4f1e2a-0000-4000-8000-0000000000f0");
    let clinician = token("clinician", CLINICIAN);
    let nurse = token("nurse", NURSE);
    let receptionist = token("receptionist", "0c4f1e2a-0000-4000-8000-0000000000f4");

    request::<App, _, _>(|server, ctx| async move {
        let (ah, av) = bearer(&admin);
        let (ch, cv) = bearer(&clinician);
        let (nh, nv) = bearer(&nurse);
        let (rh, rv) = bearer(&receptionist);
        let family = format!("res{}", nanos());
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
        for sub in [CLINICIAN, NURSE] {
            let r = server
                .post(&format!("/api/patients/{pid}/relationships"))
                .add_header(ah.clone(), av.clone())
                .json(&json!({ "relationship": { "subject": sub, "kind": "care-team", "status": "active" },
                               "reason": "test" }))
                .await;
            assert_eq!(r.status_code(), StatusCode::CREATED, "{}", r.text());
        }
        let url = format!("/api/patients/{pid}/results");

        // Only a clinician files; a nurse reads; reception sees nothing.
        let by_nurse = server
            .post(&url)
            .add_header(nh.clone(), nv.clone())
            .json(&json!({ "result": potassium("5.9", "final") }))
            .await;
        assert_eq!(by_nurse.status_code(), StatusCode::FORBIDDEN);
        let filed = server
            .post(&url)
            .add_header(ch.clone(), cv.clone())
            .json(&json!({ "result": potassium("5.9", "final") }))
            .await;
        assert_eq!(filed.status_code(), StatusCode::CREATED, "{}", filed.text());
        let rid = filed.json::<Value>()["id"].as_str().unwrap().to_string();
        assert_eq!(
            server.get(&url).add_header(rh.clone(), rv.clone()).await.status_code(),
            StatusCode::FORBIDDEN
        );
        let list = |token: (HeaderName, HeaderValue)| {
            let server = &server;
            let url = url.clone();
            async move {
                let r = server.get(&url).add_header(token.0, token.1).await;
                assert_eq!(r.status_code(), StatusCode::OK, "{}", r.text());
                r.json::<Value>()["results"][0].clone()
            }
        };
        let first = list((nh.clone(), nv.clone())).await;
        assert_eq!(first["sign_off"], "unsigned");
        assert_eq!(first["current"]["version"], 1);

        // A nurse cannot sign off; the clinician signs version 1.
        let sign = format!("{url}/{rid}/sign-off");
        assert_eq!(
            server
                .post(&sign)
                .add_header(nh.clone(), nv.clone())
                .json(&json!({ "version": 1 }))
                .await
                .status_code(),
            StatusCode::FORBIDDEN
        );
        let signed = server
            .post(&sign)
            .add_header(ch.clone(), cv.clone())
            .json(&json!({ "version": 1 }))
            .await;
        assert_eq!(signed.status_code(), StatusCode::CREATED, "{}", signed.text());
        assert_eq!(signed.json::<Value>()["version"], 1);
        let again = server
            .post(&sign)
            .add_header(ch.clone(), cv.clone())
            .json(&json!({ "version": 1 }))
            .await;
        assert_eq!(again.status_code(), StatusCode::OK, "signing twice is the same sign-off");
        assert_eq!(list((ch.clone(), cv.clone())).await["sign_off"], "signed");

        // The laboratory corrects the value. A change needs a reason.
        let change = format!("{url}/{rid}");
        let no_reason = server
            .put(&change)
            .add_header(ch.clone(), cv.clone())
            .json(&json!({ "result": potassium("4.1", "corrected"), "expected_version": 1 }))
            .await;
        assert_eq!(no_reason.status_code(), StatusCode::UNPROCESSABLE_ENTITY, "{}", no_reason.text());
        let corrected = server
            .put(&change)
            .add_header(ch.clone(), cv.clone())
            .json(&json!({ "result": potassium("4.1", "corrected"), "expected_version": 1,
                           "reason": "haemolysed sample; repeat" }))
            .await;
        assert_eq!(corrected.status_code(), StatusCode::OK, "{}", corrected.text());
        assert_eq!(corrected.json::<Value>()["version"], 2);

        // Now it reads as changed since sign-off, with the signed version shown beside the current.
        let now = list((ch.clone(), cv.clone())).await;
        assert_eq!(now["sign_off"], "changed-since-sign-off", "{now}");
        assert_eq!(now["current"]["version"], 2);
        assert_eq!(now["current"]["result"]["value"], "4.1");
        assert_eq!(now["signed_version"]["version"], 1);
        assert_eq!(now["signed_version"]["result"]["value"], "5.9");
        assert_eq!(now["last_sign_off"]["version"], 1);

        // Signing the old version again is a 409 naming the current one: the signer has not seen it.
        let stale = server
            .post(&sign)
            .add_header(ch.clone(), cv.clone())
            .json(&json!({ "version": 1 }))
            .await;
        assert_eq!(stale.status_code(), StatusCode::CONFLICT, "{}", stale.text());
        let fresh = server
            .post(&sign)
            .add_header(ch.clone(), cv.clone())
            .json(&json!({ "version": 2 }))
            .await;
        assert_eq!(fresh.status_code(), StatusCode::CREATED, "{}", fresh.text());
        let done = list((ch.clone(), cv.clone())).await;
        assert_eq!(done["sign_off"], "signed");
        assert!(done["signed_version"].is_null());

        // Back to preliminary is refused; entered in error has nothing to sign.
        let back = server
            .put(&change)
            .add_header(ch.clone(), cv.clone())
            .json(&json!({ "result": potassium("4.1", "preliminary"), "expected_version": 2,
                           "reason": "x" }))
            .await;
        assert_eq!(back.status_code(), StatusCode::UNPROCESSABLE_ENTITY, "{}", back.text());
        let error = server
            .put(&change)
            .add_header(ch.clone(), cv.clone())
            .json(&json!({ "result": potassium("4.1", "entered-in-error"), "expected_version": 2,
                           "reason": "wrong patient" }))
            .await;
        assert_eq!(error.status_code(), StatusCode::OK, "{}", error.text());
        assert_eq!(list((ch.clone(), cv.clone())).await["sign_off"], "not-applicable");
        assert_eq!(
            server
                .post(&sign)
                .add_header(ch.clone(), cv.clone())
                .json(&json!({ "version": 3 }))
                .await
                .status_code(),
            StatusCode::UNPROCESSABLE_ENTITY
        );

        // The audit trail names records and versions, never the test or a value.
        let rows = sqlx::query(AssertSqlSafe(format!(
            "SELECT detail::text AS d FROM audit_events WHERE resource_ref = 'DiagnosticReport' AND patient_ref = 'Patient/{pid}'"
        )))
        .fetch_all(&ctx.pool())
        .await
        .unwrap();
        assert!(rows.len() >= 8, "files, reads, changes and sign-offs are audited: {}", rows.len());
        let mut saw_sign_off = false;
        for row in rows {
            let d: String = row.try_get("d").unwrap();
            for secret in ["potassium", "5.9", "4.1", "mmol", "3.5-5.3", "haemolysed"] {
                assert!(!d.contains(secret), "{secret} leaked into the audit row: {d}");
            }
            saw_sign_off |= d.contains("\"signed_version\": 2") || d.contains("\"signed_version\":2");
        }
        assert!(saw_sign_off, "the sign-off of version 2 is audited with its version");
    })
    .await;
}
