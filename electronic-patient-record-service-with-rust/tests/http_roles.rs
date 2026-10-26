//! DB-gated, through the real router with login **on** (CQ-16): Keycloak-shaped tokens with roles,
//! minted by a local OIDC stand-in (`authentication_verifier::test_idp`), so the route gate and
//! the record decision both run. Covers the role refusal on `/api` and `/fhir`, consent masking
//! of duplicate candidates, and the break-the-glass offer, grant and flag.
//!
//! One test: the guard is configured once per process, from the environment, before the app boots.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test http_roles -- --ignored --test-threads=1
//! ```

use authentication_verifier::test_idp::{AUDIENCE, REALM, TestIdp};
use axum::http::{HeaderName, HeaderValue, StatusCode};
use electronic_patient_record::app::App;
use electronic_patient_record_matcher::{HumanName, Patient};
use loco_rs::testing::prelude::request;
use serde_json::{Value, json};

/// A family name no earlier run shares: letters only, from the clock, so the matcher's name
/// similarity stays low against patients left in the database by other runs.
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

/// A birth date no earlier run shares, so the duplicate check does not shortlist their patients.
fn dob(seed: u128) -> String {
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
        / 1000
        + seed;
    format!(
        "{}-{:02}-{:02}",
        1900 + n % 100,
        1 + n / 100 % 12,
        1 + n / 1200 % 28
    )
}

fn patient(family: &str, dob: &str) -> Value {
    serde_json::to_value(Patient {
        names: vec![HumanName {
            family: Some(family.into()),
            given: vec![family.into()],
        }],
        birth_date: Some(dob.into()),
        ..Default::default()
    })
    .unwrap()
}

fn bearer(token: &str) -> (HeaderName, HeaderValue) {
    (
        HeaderName::from_static("authorization"),
        HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
    )
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn roles_consent_and_break_glass_through_the_router() {
    let idp = TestIdp::start().await;
    let roles = ["clinical-admin", "clinician", "auditor"];
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
    let admin = token("clinical-admin", "0c4f1e2a-0000-4000-8000-0000000000a1");
    let clinician = token("clinician", "0c4f1e2a-0000-4000-8000-0000000000c1");
    let auditor = token("auditor", "0c4f1e2a-0000-4000-8000-0000000000d1");

    request::<App, _, _>(|server, _ctx| async move {
        let (h, v) = bearer(&admin);
        let family = tag("roles");
        let born = dob(0);
        let body = json!({ "patient": patient(&family, &born) });
        let first = server
            .post("/api/patients")
            .add_header(h.clone(), v.clone())
            .json(&body)
            .await;
        assert_eq!(first.status_code(), StatusCode::CREATED, "{}", first.text());
        let id = first.json::<Value>()["id"].as_str().unwrap().to_string();

        // The route gate: an auditor may not register anyone, on /api or on /fhir.
        let (ah, av) = bearer(&auditor);
        let refused = server
            .post("/api/patients")
            .add_header(ah.clone(), av.clone())
            .json(&body)
            .await;
        assert_eq!(refused.status_code(), StatusCode::FORBIDDEN);
        assert_eq!(refused.json::<Value>()["error"], "forbidden");
        let fhir = server
            .post("/fhir/R4/Patient")
            .add_header(ah.clone(), av.clone())
            .json(&json!({ "resourceType": "Patient" }))
            .await;
        assert_eq!(fhir.status_code(), StatusCode::FORBIDDEN);
        assert_eq!(fhir.json::<Value>()["resourceType"], "OperationOutcome");

        // No token at all: 401.
        let anon = server.post("/api/patients").json(&body).await;
        assert_eq!(anon.status_code(), StatusCode::UNAUTHORIZED);

        // Consent masking: the patient refuses disclosure for operations, so a later
        // registration of the same person is still 409 but the candidate is only "restricted".
        let consent = json!({ "consent": {
            "kind": "general", "status": "active",
            "provisions": [{ "effect": "deny", "purposes": ["operations"] }]
        }});
        let c = server
            .post(&format!("/api/patients/{id}/consents"))
            .add_header(h.clone(), v.clone())
            .json(&consent)
            .await;
        assert_eq!(c.status_code(), StatusCode::CREATED, "{}", c.text());
        let again = server
            .post("/api/patients")
            .add_header(h.clone(), v.clone())
            .json(&body)
            .await;
        assert_eq!(
            again.status_code(),
            StatusCode::CONFLICT,
            "{}",
            again.text()
        );
        let json: Value = again.json();
        assert_eq!(json["candidates"], json!([{ "restricted": true }]));
        assert!(
            !json.to_string().contains(&family),
            "the 409 must not name the person"
        );

        // Break the glass: a clinician with no relationship is offered it, takes it, and reads.
        let other = server
            .post("/api/patients")
            .add_header(h.clone(), v.clone())
            .json(&json!({ "patient": patient(&tag("glass"), &dob(555)) }))
            .await;
        assert_eq!(other.status_code(), StatusCode::CREATED, "{}", other.text());
        let pid = other.json::<Value>()["id"].as_str().unwrap().to_string();
        let (ch, cv) = bearer(&clinician);
        let allergies = format!("/api/patients/{pid}/allergies");
        let denied = server
            .get(&allergies)
            .add_header(ch.clone(), cv.clone())
            .await;
        assert_eq!(denied.status_code(), StatusCode::FORBIDDEN);
        assert_eq!(denied.json::<Value>()["break_glass_offer"], true);

        let grant = format!("/api/patients/{pid}/break-glass");
        let blank = server
            .post(&grant)
            .add_header(ch.clone(), cv.clone())
            .json(&json!({ "reason": " " }))
            .await;
        assert_eq!(
            blank.status_code(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            blank.text()
        );
        let opened = server
            .post(&grant)
            .add_header(ch.clone(), cv.clone())
            .json(&json!({ "reason": "collapsed in the street" }))
            .await;
        assert_eq!(
            opened.status_code(),
            StatusCode::CREATED,
            "{}",
            opened.text()
        );
        let expires = opened.json::<Value>()["expires_at"].clone();
        let repeat = server
            .post(&grant)
            .add_header(ch.clone(), cv.clone())
            .json(&json!({ "reason": "collapsed in the street" }))
            .await;
        assert_eq!(
            repeat.status_code(),
            StatusCode::OK,
            "asking again does not extend it"
        );
        assert_eq!(repeat.json::<Value>()["expires_at"], expires);

        let read = server
            .get(&allergies)
            .add_header(ch.clone(), cv.clone())
            .await;
        assert_eq!(read.status_code(), StatusCode::OK, "{}", read.text());

        // The read is flagged in the trail, and the grant waits for review.
        let flagged = server
            .get(&format!("/api/audit?break_glass=true&patient={pid}"))
            .add_header(ah.clone(), av.clone())
            .await;
        assert_eq!(flagged.status_code(), StatusCode::OK, "{}", flagged.text());
        let rows = flagged.json::<Value>();
        let text = rows.to_string();
        let events = rows["events"].as_array().expect("events");
        assert_eq!(events.len(), 3, "grant, repeat and read: {text}");
        assert!(events.iter().all(|e| e["break_glass"] == true));
        assert!(
            !text.contains("collapsed in the street"),
            "no free text in the trail"
        );
        assert!(
            text.contains(&pid),
            "a flagged row names the patient: {text}"
        );
        let reviews = server
            .get("/api/break-glass/reviews")
            .add_header(ah, av)
            .add_header(
                HeaderName::from_static("x-purpose-of-use"),
                HeaderValue::from_static("audit"),
            )
            .await;
        assert_eq!(reviews.status_code(), StatusCode::OK, "{}", reviews.text());
        assert!(reviews.text().contains(&pid));
        assert!(reviews.json::<Value>()["site_time_zone"].is_string());
        let (ah2, av2) = bearer(&auditor);
        let count = server
            .get("/api/break-glass/reviews/count")
            .add_header(ah2, av2)
            .add_header(
                HeaderName::from_static("x-purpose-of-use"),
                HeaderValue::from_static("audit"),
            )
            .await;
        assert!(count.json::<Value>()["unreviewed"].as_i64() >= Some(1));
    })
    .await;
}
