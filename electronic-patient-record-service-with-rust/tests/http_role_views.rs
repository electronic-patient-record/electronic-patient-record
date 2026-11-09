//! DB-gated, through the real router with login **on** (WL-7): a vaccinator, whose permissions come only
//! from its view in `src/role_views.toml`, reads the patient's identity and allergies on REST and FHIR,
//! and is refused their vital signs and results on both, and any write. One test, because the token guard
//! is configured once per process.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test http_role_views -- --ignored --test-threads=1
//! ```

use authentication_verifier::test_idp::{AUDIENCE, REALM, TestIdp};
use axa::testing::prelude::request;
use axum::http::{HeaderName, HeaderValue, StatusCode};
use electronic_patient_record::app::App;
use electronic_patient_record_matcher::{HumanName, Patient};
use serde_json::{Value, json};

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
async fn a_vaccinator_reads_its_subset_and_nothing_else() {
    let idp = TestIdp::start().await;
    let roles = ["clinical-admin", "clinician", "vaccinator"];
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
    const CLINICIAN: &str = "0c4f1e2a-0000-4000-8000-0000000000a7";
    const VACCINATOR: &str = "0c4f1e2a-0000-4000-8000-0000000000a8";
    let admin = token("clinical-admin", "0c4f1e2a-0000-4000-8000-0000000000a6");
    let clinician = token("clinician", CLINICIAN);
    let vaccinator = token("vaccinator", VACCINATOR);

    request::<App, _, _>(|server, _ctx| async move {
        let (ah, av) = bearer(&admin);
        let (ch, cv) = bearer(&clinician);
        let (vh, vv) = bearer(&vaccinator);
        let family = format!("vacc{}", nanos());
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
        for sub in [CLINICIAN, VACCINATOR] {
            let r = server
                .post(&format!("/api/patients/{pid}/relationships"))
                .add_header(ah.clone(), av.clone())
                .json(&json!({ "relationship": { "subject": sub, "kind": "care-team", "status": "active" },
                               "reason": "test" }))
                .await;
            assert_eq!(r.status_code(), StatusCode::CREATED, "{}", r.text());
        }
        let allergy = server
            .post(&format!("/api/patients/{pid}/allergies"))
            .add_header(ch.clone(), cv.clone())
            .json(&json!({ "allergy": { "substance": "penicillin", "status": "active" } }))
            .await;
        assert_eq!(allergy.status_code(), StatusCode::CREATED, "{}", allergy.text());

        let get = |path: String| {
            let server = &server;
            let (h, v) = (vh.clone(), vv.clone());
            async move { server.get(&path).add_header(h, v).await }
        };
        // Its subset: identity and allergies, on REST and FHIR.
        for path in [
            format!("/api/patients/{pid}"),
            format!("/api/patients/{pid}/allergies"),
            format!("/fhir/R4/Patient/{pid}"),
            format!("/fhir/R4/AllergyIntolerance?patient={pid}"),
        ] {
            let r = get(path.clone()).await;
            assert_eq!(r.status_code(), StatusCode::OK, "{path}: {}", r.text());
        }
        let read = get(format!("/api/patients/{pid}/allergies")).await;
        assert!(read.text().contains("penicillin"), "{}", read.text());
        // Nothing else: vital signs and results, on REST and FHIR.
        for path in [
            format!("/api/patients/{pid}/observations"),
            format!("/fhir/R4/Observation?patient={pid}"),
            format!("/api/patients/{pid}/results"),
        ] {
            let r = get(path.clone()).await;
            assert_eq!(r.status_code(), StatusCode::FORBIDDEN, "{path}: {}", r.text());
        }
        // A view grants reads only.
        let write = server
            .post(&format!("/api/patients/{pid}/allergies"))
            .add_header(vh.clone(), vv.clone())
            .json(&json!({ "allergy": { "substance": "latex", "status": "active" } }))
            .await;
        assert_eq!(write.status_code(), StatusCode::FORBIDDEN, "{}", write.text());
    })
    .await;
}
