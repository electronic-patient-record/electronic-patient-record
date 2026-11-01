//! DB-gated, through the real router with login **on** (OS-7): real responses from the role-gated
//! routes validate against the schemas the OpenAPI document serves. One test, because the token
//! guard is configured once per process. The validator is shared with `openapi_responses.rs`.
//!
//! Covered: the hazard register (list, raise, a control, a refused retirement, a retirement by the
//! Clinical Safety Officer, a stale step, a refused rating), the 401 and 403 bodies, and the
//! break-glass routes (open, repeat, list, count, review, a repeat review).
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test openapi_responses_roles -- --ignored --test-threads=1
//! ```

use authentication_verifier::test_idp::{AUDIENCE, REALM, TestIdp};
use axum::http::{HeaderName, HeaderValue, StatusCode};
use electronic_patient_record::app::App;
use loco_rs::testing::prelude::request;
use serde_json::{Value, json};

mod openapi_support;
use openapi_support::{check, response_schema};

fn bearer(token: &str) -> (HeaderName, HeaderValue) {
    (
        HeaderName::from_static("authorization"),
        HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
    )
}

fn rand_letter() -> u8 {
    (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .subsec_nanos()
        % 26) as u8
}

const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn real_responses_of_the_role_gated_routes_validate_against_the_served_schemas() {
    let idp = TestIdp::start().await;
    let roles = [
        "clinical-safety-officer",
        "clinical-admin",
        "clinician",
        "auditor",
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
        "0c4f1e2a-0000-4000-8000-0000000000d1",
    );
    let clinician = token("clinician", "0c4f1e2a-0000-4000-8000-0000000000d2");
    let auditor = token("auditor", "0c4f1e2a-0000-4000-8000-0000000000d3");
    let admin = token("clinical-admin", "0c4f1e2a-0000-4000-8000-0000000000d4");

    request::<App, _, _>(|server, _ctx| async move {
        let (oh, ov) = bearer(&cso);
        let (ch, cv) = bearer(&clinician);
        let (ah, av) = bearer(&auditor);
        let (mh, mv) = bearer(&admin);
        let purpose = HeaderName::from_static("x-purpose-of-use");
        let audit = HeaderValue::from_static("audit");

        // The document is served to any authenticated caller; fetch it with the clinician's token.
        let doc: Value = server
            .get("/api/openapi.json")
            .add_header(ch.clone(), cv.clone())
            .await
            .json();
        let mut problems: Vec<String> = Vec::new();
        let mut checked = 0;
        let mut expect =
            |label: &str, path: &str, method: &str, status: StatusCode, body: &Value| {
                let schema = response_schema(&doc, path, method, status.as_str());
                let mut found = Vec::new();
                check(&doc, schema, body, "$", &mut found);
                checked += 1;
                problems.extend(
                    found
                        .into_iter()
                        .map(|p| format!("{label} ({method} {path}): {p}")),
                );
            };

        // Hazards: raise, list, a control, a refused retirement, a retirement, a stale step.
        let raised = server
            .post("/api/hazards")
            .add_header(ch.clone(), cv.clone())
            .json(&json!({ "title": "schema check", "severity": 4, "likelihood": 2 }))
            .await;
        assert_eq!(
            raised.status_code(),
            StatusCode::CREATED,
            "{}",
            raised.text()
        );
        let raised: Value = raised.json();
        expect(
            "hazard raise",
            "/api/hazards",
            "post",
            StatusCode::CREATED,
            &raised,
        );
        let id = raised["id"].as_str().unwrap().to_string();
        let one = format!("/api/hazards/{id}");

        let bad = server
            .post("/api/hazards")
            .add_header(ch.clone(), cv.clone())
            .json(&json!({ "title": "schema check", "severity": 9, "likelihood": 2 }))
            .await;
        assert_eq!(
            bad.status_code(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            bad.text()
        );
        expect(
            "hazard bad rating",
            "/api/hazards",
            "post",
            StatusCode::UNPROCESSABLE_ENTITY,
            &bad.json(),
        );

        let seen = server
            .get("/api/hazards")
            .add_header(ah.clone(), av.clone())
            .add_header(purpose.clone(), audit.clone())
            .await;
        assert_eq!(seen.status_code(), StatusCode::OK, "{}", seen.text());
        expect(
            "hazard list",
            "/api/hazards",
            "get",
            StatusCode::OK,
            &seen.json(),
        );

        let controlled = server
            .put(&one)
            .add_header(ch.clone(), cv.clone())
            .json(&json!({ "expected_version": 1, "event": "control", "text": "a control" }))
            .await;
        assert_eq!(
            controlled.status_code(),
            StatusCode::OK,
            "{}",
            controlled.text()
        );
        expect(
            "hazard control",
            "/api/hazards/{hazard_id}",
            "put",
            StatusCode::OK,
            &controlled.json(),
        );

        let evidence = json!([{ "kind": "test-run", "reference": "tests/example.rs",
            "taken_at": "2026-10-27T09:00:00Z", "sha256": HASH }]);
        let retire = |version: i64| {
            json!({ "expected_version": version, "event": "retire",
                "evidence": evidence, "note": "checked" })
        };
        let refused = server
            .put(&one)
            .add_header(ch.clone(), cv.clone())
            .json(&retire(2))
            .await;
        assert_eq!(
            refused.status_code(),
            StatusCode::FORBIDDEN,
            "{}",
            refused.text()
        );
        expect(
            "hazard retire refused",
            "/api/hazards/{hazard_id}",
            "put",
            StatusCode::FORBIDDEN,
            &refused.json(),
        );

        let stale = server
            .put(&one)
            .add_header(oh.clone(), ov.clone())
            .json(&retire(1))
            .await;
        assert_eq!(
            stale.status_code(),
            StatusCode::CONFLICT,
            "{}",
            stale.text()
        );
        expect(
            "hazard stale",
            "/api/hazards/{hazard_id}",
            "put",
            StatusCode::CONFLICT,
            &stale.json(),
        );

        let retired = server
            .put(&one)
            .add_header(oh.clone(), ov.clone())
            .json(&retire(2))
            .await;
        assert_eq!(retired.status_code(), StatusCode::OK, "{}", retired.text());
        expect(
            "hazard retire",
            "/api/hazards/{hazard_id}",
            "put",
            StatusCode::OK,
            &retired.json(),
        );
        assert_eq!(retired.json::<Value>()["hazard"]["state"], "retired");

        let no_token = server.get("/api/hazards").await;
        assert_eq!(no_token.status_code(), StatusCode::UNAUTHORIZED);
        expect(
            "no token",
            "/api/hazards",
            "get",
            StatusCode::UNAUTHORIZED,
            &no_token.json(),
        );

        // Break glass: a clinician with no relationship opens one; an auditor lists, counts and
        // reviews. The admin registers the patient.
        let name: String = std::iter::repeat_with(|| (b'a' + rand_letter()) as char)
            .take(12)
            .collect();
        let patient = server
            .post("/api/patients")
            .add_header(mh.clone(), mv.clone())
            .json(
                &json!({ "patient": { "names": [{ "family": name, "given": [name] }],
                "birth_date": "1944-05-06" } }),
            )
            .await;
        assert_eq!(
            patient.status_code(),
            StatusCode::CREATED,
            "{}",
            patient.text()
        );
        let pid = patient.json::<Value>()["id"].as_str().unwrap().to_string();
        let glass = format!("/api/patients/{pid}/break-glass");
        let blank = server
            .post(&glass)
            .add_header(ch.clone(), cv.clone())
            .json(&json!({ "reason": " " }))
            .await;
        assert_eq!(
            blank.status_code(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            blank.text()
        );
        expect(
            "glass blank",
            "/api/patients/{id}/break-glass",
            "post",
            StatusCode::UNPROCESSABLE_ENTITY,
            &blank.json(),
        );
        let opened = server
            .post(&glass)
            .add_header(ch.clone(), cv.clone())
            .json(&json!({ "reason": "collapsed in the street" }))
            .await;
        assert_eq!(
            opened.status_code(),
            StatusCode::CREATED,
            "{}",
            opened.text()
        );
        expect(
            "glass open",
            "/api/patients/{id}/break-glass",
            "post",
            StatusCode::CREATED,
            &opened.json(),
        );
        let grant_id = opened.json::<Value>()["id"].as_str().unwrap().to_string();
        let again = server
            .post(&glass)
            .add_header(ch.clone(), cv.clone())
            .json(&json!({ "reason": "collapsed in the street" }))
            .await;
        assert_eq!(again.status_code(), StatusCode::OK, "{}", again.text());
        expect(
            "glass again",
            "/api/patients/{id}/break-glass",
            "post",
            StatusCode::OK,
            &again.json(),
        );

        let listed = server
            .get("/api/break-glass/reviews")
            .add_header(ah.clone(), av.clone())
            .add_header(purpose.clone(), audit.clone())
            .await;
        assert_eq!(listed.status_code(), StatusCode::OK, "{}", listed.text());
        expect(
            "glass list",
            "/api/break-glass/reviews",
            "get",
            StatusCode::OK,
            &listed.json(),
        );
        assert!(
            listed.text().contains(&grant_id),
            "the new grant is unreviewed"
        );
        let counted = server
            .get("/api/break-glass/reviews/count")
            .add_header(ah.clone(), av.clone())
            .add_header(purpose.clone(), audit.clone())
            .await;
        assert_eq!(counted.status_code(), StatusCode::OK, "{}", counted.text());
        expect(
            "glass count",
            "/api/break-glass/reviews/count",
            "get",
            StatusCode::OK,
            &counted.json(),
        );

        let review_url = format!("/api/break-glass/reviews/{grant_id}");
        let blank_note = server
            .post(&review_url)
            .add_header(ah.clone(), av.clone())
            .add_header(purpose.clone(), audit.clone())
            .json(&json!({ "outcome": "appropriate", "note": " " }))
            .await;
        assert_eq!(
            blank_note.status_code(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            blank_note.text()
        );
        expect(
            "glass blank note",
            "/api/break-glass/reviews/{id}",
            "post",
            StatusCode::UNPROCESSABLE_ENTITY,
            &blank_note.json(),
        );
        let reviewed = server
            .post(&review_url)
            .add_header(ah.clone(), av.clone())
            .add_header(purpose.clone(), audit.clone())
            .json(&json!({ "outcome": "appropriate", "note": "a real emergency" }))
            .await;
        assert_eq!(
            reviewed.status_code(),
            StatusCode::OK,
            "{}",
            reviewed.text()
        );
        expect(
            "glass review",
            "/api/break-glass/reviews/{id}",
            "post",
            StatusCode::OK,
            &reviewed.json(),
        );
        assert_eq!(reviewed.json::<Value>()["review"]["outcome"], "appropriate");
        let twice = server
            .post(&review_url)
            .add_header(ah.clone(), av.clone())
            .add_header(purpose.clone(), audit.clone())
            .json(&json!({ "outcome": "appropriate", "note": "again" }))
            .await;
        assert_eq!(
            twice.status_code(),
            StatusCode::CONFLICT,
            "{}",
            twice.text()
        );
        expect(
            "glass twice",
            "/api/break-glass/reviews/{id}",
            "post",
            StatusCode::CONFLICT,
            &twice.json(),
        );

        assert!(
            problems.is_empty(),
            "schema mismatches:\n{}",
            problems.join("\n")
        );
        assert!(checked >= 16, "only {checked} responses were checked");
    })
    .await;
}
