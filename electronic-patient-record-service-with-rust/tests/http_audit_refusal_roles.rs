//! DB-gated, through the real router with login **on** (CQ-39): the writes that need a role and, for the grant,
//! no care relationship, refuse when their audit row cannot be written: opening a break-the-glass grant, reviewing
//! one, and resolving a refused feed message. The mechanism is the one in `http_audit_refusal.rs`
//! (`audit_switch`): 500 with no row stored, then the same request succeeds. One test, because the token guard is
//! configured once per process.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test http_audit_refusal_roles -- --ignored --test-threads=1
//! ```

use authentication_verifier::test_idp::{AUDIENCE, REALM, TestIdp};
use axum::http::{HeaderName, HeaderValue};
use electronic_patient_record::app::App;
use electronic_patient_record_matcher::{HumanName, Patient};
use loco_rs::testing::prelude::request;
use sea_orm::ConnectionTrait;
use serde_json::{Value, json};

mod audit_switch;
use audit_switch::{DRIVEN, LOGIN_ON, install, refused_then_accepted, uninstall};

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
fn dob() -> String {
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
        / 1000;
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
async fn break_glass_and_the_feed_queue_refuse_when_their_audit_row_cannot_be_written() {
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

    request::<App, _, _>(|server, ctx| async move {
        let db = &ctx.db;
        let (ah, av) = bearer(&admin);
        let (ch, cv) = bearer(&clinician);
        let (uh, uv) = bearer(&auditor);
        let audit_purpose = (
            HeaderName::from_static("x-purpose-of-use"),
            HeaderValue::from_static("audit"),
        );

        // A patient the clinician has no relationship with, made by the administrator.
        let family = tag("auditglass");
        let made = server
            .post("/api/patients")
            .add_header(ah.clone(), av.clone())
            .json(&json!({ "patient": serde_json::to_value(Patient {
                names: vec![HumanName { family: Some(family.clone()), given: vec![family.clone()] }],
                birth_date: Some(dob()),
                ..Default::default()
            }).unwrap() }))
            .await;
        assert_eq!(made.status_code(), 201, "{}", made.text());
        let pid = made.json::<Value>()["id"].as_str().unwrap().to_string();

        // A refused feed message waiting for the administrator.
        db.execute_unprepared(
            "INSERT INTO feed_refusals (agent, event, control_id, kind, message)
             VALUES ('test', 'ADT^A01', 'audit-refusal-test', 'unsupported', 'MSH|test')",
        )
        .await
        .unwrap();
        let waiting = server
            .get("/api/feed/refusals")
            .add_header(ah.clone(), av.clone())
            .await;
        assert!(waiting.status_code().is_success(), "{}", waiting.text());
        let refusal = waiting.json::<Value>();
        let refusal_id = refusal["refusals"]
            .as_array()
            .and_then(|a| a.first())
            .and_then(|r| r["id"].as_str().or(r["refusal_id"].as_str()))
            .unwrap_or_else(|| panic!("a refusal id in {refusal}"))
            .to_string();

        install(db).await;

        // Open a grant. The first run is refused, the second opens it.
        let grant_id = std::sync::Mutex::new(String::new());
        refused_then_accepted(db, "POST /api/patients/{id}/break-glass", || async {
            let r = server
                .post(&format!("/api/patients/{pid}/break-glass"))
                .add_header(ch.clone(), cv.clone())
                .json(&json!({ "reason": "collapsed in the street" }))
                .await;
            let text = r.text();
            if r.status_code().is_success() {
                let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
                if let Some(id) = v["id"].as_str().or(v["grant_id"].as_str()) {
                    *grant_id.lock().unwrap() = id.to_string();
                }
            }
            (r.status_code(), text)
        })
        .await;
        let grant = grant_id.lock().unwrap().clone();
        assert!(!grant.is_empty(), "the grant's id was not in the response");

        // Review it, as someone who did not use it.
        refused_then_accepted(db, "POST /api/break-glass/reviews/{id}", || async {
            let r = server
                .post(&format!("/api/break-glass/reviews/{grant}"))
                .add_header(uh.clone(), uv.clone())
                .add_header(audit_purpose.0.clone(), audit_purpose.1.clone())
                .json(&json!({ "outcome": "appropriate", "note": "an emergency" }))
                .await;
            (r.status_code(), r.text())
        })
        .await;

        // Resolve the feed refusal.
        refused_then_accepted(db, "POST /api/feed/refusals/{id}/resolve", || async {
            let r = server
                .post(&format!("/api/feed/refusals/{refusal_id}/resolve"))
                .add_header(ah.clone(), av.clone())
                .json(&json!({ "outcome": "dismissed", "reason": "test message" }))
                .await;
            (r.status_code(), r.text())
        })
        .await;

        uninstall(db).await;

        let mut driven = DRIVEN.lock().unwrap().clone();
        driven.sort();
        let mut listed: Vec<String> = LOGIN_ON.iter().map(ToString::to_string).collect();
        listed.sort();
        assert_eq!(driven, listed, "LOGIN_ON must list exactly what this test drives");
    })
    .await;
}
