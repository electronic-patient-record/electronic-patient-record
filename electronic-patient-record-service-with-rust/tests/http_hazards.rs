//! DB-gated, through the real router with login **on** (AS-17): only the Clinical Safety Officer retires a
//! hazard, and only with evidence; the role is the token's, never the body's; any writer may reopen; a stale
//! version is 409; a role with no business here is refused; and the audit trail holds the evidence hashes
//! but never the free text. One test, because the token guard is configured once per process.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test http_hazards -- --ignored --test-threads=1
//! ```

use authentication_verifier::test_idp::{AUDIENCE, REALM, TestIdp};
use axum::http::{HeaderName, HeaderValue, StatusCode};
use electronic_patient_record::app::App;
use loco_rs::testing::prelude::request;
use sea_orm::{ConnectionTrait, Statement};
use serde_json::{Value, json};

fn bearer(token: &str) -> (HeaderName, HeaderValue) {
    (
        HeaderName::from_static("authorization"),
        HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
    )
}

const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn retire(hash: &str, version: i64) -> Value {
    json!({
        "expected_version": version,
        "event": "retire",
        "evidence": [{
            "kind": "test-run",
            "reference": "tests/example.rs secret-reference",
            "taken_at": "2026-10-27T09:00:00Z",
            "sha256": hash
        }],
        "note": "secret-note: checked"
    })
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn only_the_clinical_safety_officer_retires_a_hazard_and_only_with_evidence() {
    let idp = TestIdp::start().await;
    let roles = [
        "clinical-safety-officer",
        "clinician",
        "auditor",
        "receptionist",
        "integrator",
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
    let cso = token(
        "clinical-safety-officer",
        "0c4f1e2a-0000-4000-8000-0000000000c1",
    );
    let clinician = token("clinician", "0c4f1e2a-0000-4000-8000-0000000000c2");
    let auditor = token("auditor", "0c4f1e2a-0000-4000-8000-0000000000c3");
    let receptionist = token("receptionist", "0c4f1e2a-0000-4000-8000-0000000000c4");
    let integrator = token("integrator", "0c4f1e2a-0000-4000-8000-0000000000c5");

    request::<App, _, _>(|server, ctx| async move {
        let (oh, ov) = bearer(&cso);
        let (ch, cv) = bearer(&clinician);
        let (ah, av) = bearer(&auditor);
        let (rh, rv) = bearer(&receptionist);
        let (ih, iv) = bearer(&integrator);
        let purpose = HeaderName::from_static("x-purpose-of-use");
        let audit = HeaderValue::from_static("audit");
        let url = "/api/hazards";

        // A clinician raises a hazard; a receptionist and a service identity have no business here.
        let raised = server
            .post(url)
            .add_header(ch.clone(), cv.clone())
            .json(&json!({ "title": "secret-title", "severity": 4, "likelihood": 2 }))
            .await;
        assert_eq!(
            raised.status_code(),
            StatusCode::CREATED,
            "{}",
            raised.text()
        );
        let id = raised.json::<Value>()["id"].as_str().unwrap().to_string();
        let one = format!("{url}/{id}");
        for (h, v) in [(&rh, &rv), (&ih, &iv)] {
            assert_eq!(
                server
                    .get(url)
                    .add_header(h.clone(), v.clone())
                    .await
                    .status_code(),
                StatusCode::FORBIDDEN
            );
            assert_eq!(
                server
                    .post(url)
                    .add_header(h.clone(), v.clone())
                    .json(&json!({ "title": "t", "severity": 1, "likelihood": 1 }))
                    .await
                    .status_code(),
                StatusCode::FORBIDDEN
            );
        }
        // The auditor reads (with the audit purpose) and cannot write.
        let seen = server
            .get(url)
            .add_header(ah.clone(), av.clone())
            .add_header(purpose.clone(), audit.clone())
            .await;
        assert_eq!(seen.status_code(), StatusCode::OK, "{}", seen.text());
        assert_eq!(
            server
                .post(url)
                .add_header(ah.clone(), av.clone())
                .add_header(purpose.clone(), audit.clone())
                .json(&json!({ "title": "t", "severity": 1, "likelihood": 1 }))
                .await
                .status_code(),
            StatusCode::FORBIDDEN
        );

        // A clinician cannot retire, however complete the evidence is.
        let denied = server
            .put(&one)
            .add_header(ch.clone(), cv.clone())
            .json(&retire(HASH, 1))
            .await;
        assert_eq!(
            denied.status_code(),
            StatusCode::FORBIDDEN,
            "{}",
            denied.text()
        );

        // The Clinical Safety Officer cannot retire without evidence, or with a bad hash: 422.
        let none = server
            .put(&one)
            .add_header(oh.clone(), ov.clone())
            .json(&json!({
                "expected_version": 1, "event": "retire", "evidence": [], "note": "n"
            }))
            .await;
        assert_eq!(
            none.status_code(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            none.text()
        );
        let bad = server
            .put(&one)
            .add_header(oh.clone(), ov.clone())
            .json(&retire("abc", 1))
            .await;
        assert_eq!(
            bad.status_code(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            bad.text()
        );

        // A stale version is a conflict.
        let stale = server
            .put(&one)
            .add_header(oh.clone(), ov.clone())
            .json(&retire(HASH, 7))
            .await;
        assert_eq!(
            stale.status_code(),
            StatusCode::CONFLICT,
            "{}",
            stale.text()
        );

        // With evidence, the Clinical Safety Officer retires it.
        let done = server
            .put(&one)
            .add_header(oh.clone(), ov.clone())
            .json(&retire(HASH, 1))
            .await;
        assert_eq!(done.status_code(), StatusCode::OK, "{}", done.text());
        let done: Value = done.json();
        assert_eq!(done["hazard"]["state"], "retired");
        assert_eq!(
            done["hazard"]["retired_by"],
            "0c4f1e2a-0000-4000-8000-0000000000c1"
        );
        assert_eq!(done["version"], 2);

        // The role is the token's: a field in the body does not make a clinician the officer.
        let mut claim = retire(HASH, 2);
        claim["is_cso"] = json!(true);
        let back_door = server
            .put(&one)
            .add_header(ch.clone(), cv.clone())
            .json(&claim)
            .await;
        assert_eq!(back_door.status_code(), StatusCode::FORBIDDEN);

        // Any writer reopens it, with a reason.
        let blank = server
            .put(&one)
            .add_header(ch.clone(), cv.clone())
            .json(&json!({ "expected_version": 2, "event": "reopen", "reason": " " }))
            .await;
        assert_eq!(blank.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        let reopened = server
            .put(&one)
            .add_header(ch.clone(), cv.clone())
            .json(&json!({ "expected_version": 2, "event": "reopen", "reason": "seen again" }))
            .await;
        assert_eq!(
            reopened.status_code(),
            StatusCode::OK,
            "{}",
            reopened.text()
        );
        assert_eq!(reopened.json::<Value>()["hazard"]["state"], "open");

        // The audit trail holds the evidence hash, never the free text.
        let rows = ctx
            .db
            .query_all_raw(Statement::from_string(
                sea_orm::DbBackend::Postgres,
                "SELECT detail::text AS d FROM audit_events WHERE resource_ref = 'Hazard'"
                    .to_string(),
            ))
            .await
            .unwrap();
        let all: Vec<String> = rows.iter().map(|r| r.try_get("", "d").unwrap()).collect();
        assert!(all.len() >= 6, "{all:?}");
        assert!(all.iter().any(|d| d.contains(HASH)), "{all:?}");
        for d in &all {
            for secret in [
                "secret-title",
                "secret-reference",
                "secret-note",
                "seen again",
            ] {
                assert!(
                    !d.contains(secret),
                    "{secret} leaked into the audit row: {d}"
                );
            }
        }
    })
    .await;
}
