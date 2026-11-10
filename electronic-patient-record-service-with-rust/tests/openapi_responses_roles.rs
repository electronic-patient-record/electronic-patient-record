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
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test openapi_responses_roles -- --ignored --test-threads=1
//! ```

use authentication_verifier::test_idp::{AUDIENCE, REALM, TestIdp};
use axa_test::prelude::request;
use axum::http::{HeaderName, HeaderValue, StatusCode};
use electronic_patient_record::app::App;
use electronic_patient_record::pool::Pool;
use serde_json::{Value, json};

mod openapi_support;
use openapi_support::{check, response_schema};

fn bearer(token: &str) -> (HeaderName, HeaderValue) {
    (
        HeaderName::from_static("authorization"),
        HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
    )
}

fn unique_name() -> String {
    let mut n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mut s = String::from("glass");
    while n > 0 {
        s.push((b'a' + (n % 26) as u8) as char);
        n /= 26;
    }
    s
}

const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn real_responses_of_the_role_gated_routes_validate_against_the_served_schemas() {
    let idp = TestIdp::start().await;
    let roles = [
        "clinical-safety-officer",
        "clinical-admin",
        "clinician",
        "auditor",
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
    let cso = token(
        "clinical-safety-officer",
        "0c4f1e2a-0000-4000-8000-0000000000d1",
    );
    let clinician = token("clinician", "0c4f1e2a-0000-4000-8000-0000000000d2");
    let auditor = token("auditor", "0c4f1e2a-0000-4000-8000-0000000000d3");
    let admin = token("clinical-admin", "0c4f1e2a-0000-4000-8000-0000000000d4");
    let proxy_sub = "0c4f1e2a-0000-4000-8000-0000000000d5";
    let proxy = token("proxy", proxy_sub);

    request::<App, _, _>(|server, ctx| async move {
        let (oh, ov) = bearer(&cso);
        let (ch, cv) = bearer(&clinician);
        let (ah, av) = bearer(&auditor);
        let (mh, mv) = bearer(&admin);
        let (xh, xv) = bearer(&proxy);
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
        let name = unique_name();
        let patient = server
            .post("/api/patients")
            .add_header(mh.clone(), mv.clone())
            .json(
                &json!({ "patient": { "names": [{ "family": name, "given": [name] }],
                "birth_date": "1944-05-06" },
                "override_reason": "schema check; earlier runs left similar records" }),
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

        // Audit: search, export (both formats), verify (intact), and the refusals.
        let get_audit = |path: String| {
            server
                .get(&path)
                .add_header(ah.clone(), av.clone())
                .add_header(purpose.clone(), audit.clone())
        };
        let found = get_audit(format!("/api/audit?patient={pid}&break_glass=true")).await;
        assert_eq!(found.status_code(), StatusCode::OK, "{}", found.text());
        expect(
            "audit search",
            "/api/audit",
            "get",
            StatusCode::OK,
            &found.json(),
        );
        assert!(
            !found.json::<Value>()["events"]
                .as_array()
                .unwrap()
                .is_empty(),
            "the break-glass reads of this patient are in the trail"
        );
        let all = get_audit("/api/audit?outcome=0".to_string()).await;
        assert_eq!(all.status_code(), StatusCode::OK, "{}", all.text());
        expect(
            "audit search outcome",
            "/api/audit",
            "get",
            StatusCode::OK,
            &all.json(),
        );
        let bad = get_audit("/api/audit?outcome=notanumber".to_string()).await;
        assert_eq!(bad.status_code(), StatusCode::BAD_REQUEST, "{}", bad.text());
        // A value of the wrong type is the JSON `Error`, like a bad date.
        assert!(bad.text().contains("outcome"), "{}", bad.text());
        expect(
            "audit search wrong type",
            "/api/audit",
            "get",
            StatusCode::BAD_REQUEST,
            &bad.json(),
        );
        let bad_date = get_audit("/api/audit?from=notadate".to_string()).await;
        assert_eq!(
            bad_date.status_code(),
            StatusCode::BAD_REQUEST,
            "{}",
            bad_date.text()
        );
        expect(
            "audit search bad date",
            "/api/audit",
            "get",
            StatusCode::BAD_REQUEST,
            &bad_date.json(),
        );

        let csv = get_audit(format!("/api/audit/export?patient={pid}")).await;
        assert_eq!(csv.status_code(), StatusCode::OK, "{}", csv.text());
        assert!(
            csv.header("content-type")
                .to_str()
                .unwrap()
                .starts_with("text/csv")
        );
        assert!(
            csv.text()
                .starts_with("id,at,action,outcome,agent,patient,")
        );
        let nd = get_audit(format!("/api/audit/export?patient={pid}&format=ndjson")).await;
        assert_eq!(nd.status_code(), StatusCode::OK, "{}", nd.text());
        assert_eq!(nd.header("content-type"), "application/x-ndjson");
        let first: Value = serde_json::from_str(nd.text().lines().next().unwrap()).unwrap();
        let mut found = Vec::new();
        check(
            &doc,
            &json!({"$ref": "#/components/schemas/AuditRow"}),
            &first,
            "$",
            &mut found,
        );
        assert!(found.is_empty(), "an exported row: {found:?}");
        let bad_format = get_audit("/api/audit/export?format=xml".to_string()).await;
        assert_eq!(
            bad_format.status_code(),
            StatusCode::BAD_REQUEST,
            "{}",
            bad_format.text()
        );
        expect(
            "audit export bad",
            "/api/audit/export",
            "get",
            StatusCode::BAD_REQUEST,
            &bad_format.json(),
        );

        // The trail is shared with every other database test, and `audit_chain.rs` tampers with
        // rows on purpose, so the chain may be intact (200) or already broken (409) when this
        // runs. Either answer is checked against its own schema; the checkpoint step needs 200.
        let verified = get_audit("/api/audit/verify".to_string()).await;
        let intact = verified.status_code() == StatusCode::OK;
        assert!(
            intact || verified.status_code() == StatusCode::CONFLICT,
            "{} {}",
            verified.status_code(),
            verified.text()
        );
        expect(
            "audit verify",
            "/api/audit/verify",
            "get",
            verified.status_code(),
            &verified.json(),
        );
        if intact {
            let head = verified.json::<Value>()["head"].clone();
            let confirmed = get_audit(format!(
                "/api/audit/verify?checkpoint_id={}&checkpoint_digest={}",
                head["id"],
                head["digest"].as_str().unwrap()
            ))
            .await;
            assert_eq!(
                confirmed.status_code(),
                StatusCode::OK,
                "{}",
                confirmed.text()
            );
            assert_eq!(confirmed.json::<Value>()["checkpoint_confirmed"], true);
            expect(
                "audit verify checkpoint",
                "/api/audit/verify",
                "get",
                StatusCode::OK,
                &confirmed.json(),
            );
        }
        let half = get_audit("/api/audit/verify?checkpoint_id=1".to_string()).await;
        assert_eq!(
            half.status_code(),
            StatusCode::BAD_REQUEST,
            "{}",
            half.text()
        );
        expect(
            "audit verify half",
            "/api/audit/verify",
            "get",
            StatusCode::BAD_REQUEST,
            &half.json(),
        );
        let typed =
            get_audit("/api/audit/verify?checkpoint_id=x&checkpoint_digest=y".to_string()).await;
        assert_eq!(
            typed.status_code(),
            StatusCode::BAD_REQUEST,
            "{}",
            typed.text()
        );
        expect(
            "audit verify wrong type",
            "/api/audit/verify",
            "get",
            StatusCode::BAD_REQUEST,
            &typed.json(),
        );
        let denied = server
            .get("/api/audit")
            .add_header(ch.clone(), cv.clone())
            .add_header(purpose.clone(), audit.clone())
            .await;
        assert_eq!(
            denied.status_code(),
            StatusCode::FORBIDDEN,
            "{}",
            denied.text()
        );
        expect(
            "audit clinician",
            "/api/audit",
            "get",
            StatusCode::FORBIDDEN,
            &denied.json(),
        );

        // Feed refusals: the feed stores one; the admin lists, counts, reads and closes it.
        let reasons = vec!["no PID segment".to_string()];
        let control = unique_name();
        let stored = electronic_patient_record::feed_queue::record(
            &ctx.pool(),
            electronic_patient_record::feed_queue::New {
                agent: "feed",
                peer: Some("127.0.0.1"),
                event: "A08",
                control_id: &control,
                kind: electronic_patient_record::feed_queue::Kind::Unsupported,
                reasons: &reasons,
                message: "MSH|^~\\&|",
            },
        )
        .await
        .expect("a refusal is stored");
        let queue = "/api/feed/refusals";
        let listed = server.get(queue).add_header(mh.clone(), mv.clone()).await;
        assert_eq!(listed.status_code(), StatusCode::OK, "{}", listed.text());
        expect("feed list", queue, "get", StatusCode::OK, &listed.json());
        assert!(listed.text().contains(&control), "the new refusal is open");
        assert!(
            !listed.text().contains("MSH|"),
            "a listing never carries the message"
        );
        let counted = server
            .get("/api/feed/refusals/count")
            .add_header(mh.clone(), mv.clone())
            .await;
        assert_eq!(counted.status_code(), StatusCode::OK, "{}", counted.text());
        expect(
            "feed count",
            "/api/feed/refusals/count",
            "get",
            StatusCode::OK,
            &counted.json(),
        );
        let one = format!("{queue}/{stored}");
        let read = server.get(&one).add_header(mh.clone(), mv.clone()).await;
        assert_eq!(read.status_code(), StatusCode::OK, "{}", read.text());
        expect(
            "feed read",
            "/api/feed/refusals/{id}",
            "get",
            StatusCode::OK,
            &read.json(),
        );
        assert!(
            read.json::<Value>()["message"].is_string(),
            "a read carries the message"
        );
        let blank = server
            .post(&format!("{one}/resolve"))
            .add_header(mh.clone(), mv.clone())
            .json(&json!({ "outcome": "dismissed", "reason": " " }))
            .await;
        assert_eq!(
            blank.status_code(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            blank.text()
        );
        expect(
            "feed blank reason",
            "/api/feed/refusals/{id}/resolve",
            "post",
            StatusCode::UNPROCESSABLE_ENTITY,
            &blank.json(),
        );
        let closed = server
            .post(&format!("{one}/resolve"))
            .add_header(mh.clone(), mv.clone())
            .json(&json!({ "outcome": "dismissed", "reason": "a test message" }))
            .await;
        assert_eq!(closed.status_code(), StatusCode::OK, "{}", closed.text());
        expect(
            "feed resolve",
            "/api/feed/refusals/{id}/resolve",
            "post",
            StatusCode::OK,
            &closed.json(),
        );
        assert_eq!(closed.json::<Value>()["resolution"]["outcome"], "dismissed");
        let twice = server
            .post(&format!("{one}/resolve"))
            .add_header(mh.clone(), mv.clone())
            .json(&json!({ "outcome": "handled", "reason": "again" }))
            .await;
        assert_eq!(
            twice.status_code(),
            StatusCode::CONFLICT,
            "{}",
            twice.text()
        );
        expect(
            "feed twice",
            "/api/feed/refusals/{id}/resolve",
            "post",
            StatusCode::CONFLICT,
            &twice.json(),
        );
        let gone = server.get(queue).add_header(mh.clone(), mv.clone()).await;
        assert!(
            !gone.text().contains(&control),
            "a closed refusal leaves the open list"
        );

        // The patient's own routes: a proxy whose terms list the identity and the access log.
        let expires = i64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        )
        .unwrap()
            + 3600;
        let tie = server
            .post(&format!("/api/patients/{pid}/relationships"))
            .add_header(mh.clone(), mv.clone())
            .json(&json!({
                "relationship": { "subject": proxy_sub, "kind": "proxy", "status": "active",
                    "proxy": { "basis": "parental-responsibility",
                               "scopes": ["identity", "access-log"], "expires_at": expires } },
                "reason": "schema check" }))
            .await;
        assert_eq!(tie.status_code(), StatusCode::CREATED, "{}", tie.text());
        let patient_access = HeaderValue::from_static("patient-access");
        let own = server
            .get("/api/me/patients")
            .add_header(xh.clone(), xv.clone())
            .add_header(purpose.clone(), patient_access.clone())
            .await;
        assert_eq!(own.status_code(), StatusCode::OK, "{}", own.text());
        let own: Value = own.json();
        assert!(
            own["records"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["patient"]["id"] == pid.as_str() && r["relationship"] == "proxy"),
            "the proxy sees the record: {own}"
        );
        expect(
            "own records",
            "/api/me/patients",
            "get",
            StatusCode::OK,
            &own,
        );
        let log_url = format!("/api/me/patients/{pid}/access-log");
        let log = server
            .get(&log_url)
            .add_header(xh.clone(), xv.clone())
            .add_header(purpose.clone(), patient_access.clone())
            .await;
        assert_eq!(log.status_code(), StatusCode::OK, "{}", log.text());
        let log: Value = log.json();
        assert!(
            !log["entries"].as_array().unwrap().is_empty(),
            "the registration and the proxy's own reads are in the log: {log}"
        );
        expect(
            "access log",
            "/api/me/patients/{id}/access-log",
            "get",
            StatusCode::OK,
            &log,
        );
        let refused_only = server
            .get(&format!("{log_url}?refused=false&by=you"))
            .add_header(xh.clone(), xv.clone())
            .add_header(purpose.clone(), patient_access.clone())
            .await;
        assert_eq!(refused_only.status_code(), StatusCode::OK);
        expect(
            "access log filtered",
            "/api/me/patients/{id}/access-log",
            "get",
            StatusCode::OK,
            &refused_only.json(),
        );
        let bad = server
            .get(&format!("{log_url}?by=nobody"))
            .add_header(xh.clone(), xv.clone())
            .add_header(purpose.clone(), patient_access.clone())
            .await;
        assert_eq!(bad.status_code(), StatusCode::BAD_REQUEST, "{}", bad.text());
        expect(
            "access log bad filter",
            "/api/me/patients/{id}/access-log",
            "get",
            StatusCode::BAD_REQUEST,
            &bad.json(),
        );
        let csv = server
            .get(&format!("{log_url}?format=csv"))
            .add_header(xh.clone(), xv.clone())
            .add_header(purpose.clone(), patient_access.clone())
            .await;
        assert_eq!(csv.status_code(), StatusCode::OK, "{}", csv.text());
        assert_eq!(
            csv.header("content-type").to_str().unwrap(),
            "text/csv; charset=utf-8"
        );
        assert!(csv.text().starts_with("id,at_utc,by,roles,"));
        let unrelated = server
            .get(&log_url)
            .add_header(ch.clone(), cv.clone())
            .add_header(purpose.clone(), patient_access.clone())
            .await;
        assert_eq!(unrelated.status_code(), StatusCode::FORBIDDEN);
        expect(
            "access log refused",
            "/api/me/patients/{id}/access-log",
            "get",
            StatusCode::FORBIDDEN,
            &unrelated.json(),
        );

        assert!(
            problems.is_empty(),
            "schema mismatches:\n{}",
            problems.join("\n")
        );
        assert!(checked >= 36, "only {checked} responses were checked");
    })
    .await;
}
