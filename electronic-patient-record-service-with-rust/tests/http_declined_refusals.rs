//! DB-gated, through the real router (DR-10): a refusal of disclosure can be declined only where the tenant's pack allows
//! it (never with no pack, never under `binds`), only for an active refusal, and names the pack's version; it is never
//! changed; the patient reads their own; the audit trail holds no word of why.
//!
//! ```text
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test main http_declined_refusals:: -- --ignored --test-threads=1
//! ```

use authentication_verifier::test_idp::{AUDIENCE, REALM, TestIdp};
use axa_test::prelude::request_with;
use axum::http::{HeaderName, HeaderValue, StatusCode};
use electronic_patient_record::app::App;
use electronic_patient_record::pool::Pool;
use serde_json::{Value, json};
use sqlx::Row;

fn letters() -> String {
    let mut n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mut s = String::new();
    while n > 0 {
        s.push((b'a' + (n % 26) as u8) as char);
        n /= 26;
    }
    s
}

/// A pack file with `strength` set, or none; its path, leaked for the settings.
fn pack(strength: &str) -> &'static str {
    let path =
        std::env::temp_dir().join(format!("epr-dr10-{strength}-{}.toml", std::process::id()));
    std::fs::write(
        &path,
        format!("[pack]\nid = \"t\"\nplace = \"Test\"\nlaw_as_of = \"2026-11-27\"\n[[rule]]\nkey = \"strength\"\nvalue = {strength:?}\nsource = \"test\"\n"),
    )
    .unwrap();
    Box::leak(path.to_string_lossy().into_owned().into_boxed_str())
}

fn declined() -> Value {
    json!({ "by": "Dr F, Caldicott guardian", "ground": "risk-to-another", "because": "QUOKKA at risk",
        "told_at": "2026-11-27T09:00:00Z", "told_how": "in person, with a letter", "complain_to": "the privacy officer" })
}

fn db_url() -> &'static str {
    Box::leak(
        std::env::var("DB_URL")
            .expect("DB_URL must be set for --ignored tests")
            .into_boxed_str(),
    )
}

/// A patient with a refusal of disclosure and a permission, through `server` with login off: (patient, refusal, permission).
async fn setup(server: &axa_test::TestServer) -> (String, String, String) {
    let made = server
        .post("/api/patients")
        .json(&json!({ "patient": { "names": [{ "family": format!("dr{}", letters()), "given": ["G"] }], "birth_date": "1970-01-01" },
            "override_reason": "test setup" }))
        .await;
    assert_eq!(made.status_code(), StatusCode::CREATED, "{}", made.text());
    let pid = made.json::<Value>()["id"].as_str().unwrap().to_string();
    let consent = |effect: &str, purpose: &str| json!({ "consent": { "kind": "general", "status": "active", "provisions": [{ "effect": effect, "purposes": [purpose] }] } });
    let refusal = server
        .post(&format!("/api/patients/{pid}/consents"))
        .json(&consent("deny", "research"))
        .await;
    assert_eq!(
        refusal.status_code(),
        StatusCode::CREATED,
        "{}",
        refusal.text()
    );
    let permit = server
        .post(&format!("/api/patients/{pid}/consents"))
        .json(&consent("permit", "treatment"))
        .await;
    assert_eq!(
        permit.status_code(),
        StatusCode::CREATED,
        "{}",
        permit.text()
    );
    (
        pid,
        refusal.json::<Value>()["id"].as_str().unwrap().to_string(),
        permit.json::<Value>()["id"].as_str().unwrap().to_string(),
    )
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn with_no_pack_or_under_binds_a_refusal_cannot_be_declined() {
    for settings in [
        vec![("DB_URL", db_url()), ("EPR_REQUIRE_AUTH", "false")],
        vec![
            ("DB_URL", db_url()),
            ("EPR_REQUIRE_AUTH", "false"),
            ("EPR_DISCLOSURE_RULES_FILE", pack("binds")),
        ],
    ] {
        request_with::<App, _, _>(&settings, |server, _ctx| async move {
            let (pid, refusal, _) = setup(&server).await;
            let r = server
                .post(&format!("/api/patients/{pid}/declined-refusals"))
                .json(&json!({ "consent_id": refusal, "declined": declined() }))
                .await;
            assert_eq!(
                r.status_code(),
                StatusCode::UNPROCESSABLE_ENTITY,
                "{}",
                r.text()
            );
            assert!(r.text().contains("cannot be declined here"), "{}", r.text());
            let list = server
                .get(&format!("/api/patients/{pid}/declined-refusals"))
                .await
                .json::<Value>();
            assert_eq!(list, json!({ "declines": [] }));
        })
        .await;
    }
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn where_the_pack_allows_it_an_active_refusal_is_declined_named_by_the_pack_and_kept_as_made()
{
    let settings = vec![
        ("DB_URL", db_url()),
        ("EPR_REQUIRE_AUTH", "false"),
        ("EPR_DISCLOSURE_RULES_FILE", pack("considered")),
    ];
    request_with::<App, _, _>(&settings, |server, ctx| async move {
        let (pid, refusal, permit) = setup(&server).await;
        let list = format!("/api/patients/{pid}/declined-refusals");
        // A permission is not a refusal, and an unknown consent is not there.
        let r = server.post(&list).json(&json!({ "consent_id": permit, "declined": declined() })).await;
        assert_eq!(r.status_code(), StatusCode::UNPROCESSABLE_ENTITY, "{}", r.text());
        let r = server
            .post(&list)
            .json(&json!({ "consent_id": "00000000-0000-4000-8000-000000000000", "declined": declined() }))
            .await;
        assert_eq!(r.status_code(), StatusCode::NOT_FOUND, "{}", r.text());
        let mut unsaid = declined();
        unsaid["complain_to"] = json!(" ");
        let r = server.post(&list).json(&json!({ "consent_id": refusal, "declined": unsaid })).await;
        assert_eq!(r.status_code(), StatusCode::UNPROCESSABLE_ENTITY);

        let made = server.post(&list).json(&json!({ "consent_id": refusal, "declined": declined() })).await;
        assert_eq!(made.status_code(), StatusCode::CREATED, "{}", made.text());
        let made = made.json::<Value>();
        assert!(made["pack_version"].as_str().unwrap().starts_with("t@2026-11-27#"), "{made}");
        assert_eq!(made["consent_id"], refusal.as_str());
        let did = made["id"].as_str().unwrap().to_string();
        let shown = server.get(&list).await.json::<Value>();
        assert_eq!(shown["declines"].as_array().unwrap().len(), 1);
        assert_eq!(shown["declines"][0]["from_merged_record"], false);

        // The refusal still applies: declining it is recorded and decides nothing yet.
        let consents = server.get(&format!("/api/patients/{pid}/consents")).await.json::<Value>();
        assert!(consents["consents"].as_array().unwrap().iter().any(|c| c["id"] == refusal.as_str() && c["consent"]["status"] == "active"));

        let eie = format!("{list}/{did}/entered-in-error");
        assert_eq!(server.post(&eie).json(&json!({ "expected_version": 1, "reason": " " })).await.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        let done = server.post(&eie).json(&json!({ "expected_version": 1, "reason": "WOMBAT" })).await;
        assert_eq!(done.status_code(), StatusCode::OK, "{}", done.text());
        assert_eq!(done.json::<Value>()["state"], "entered-in-error");

        let rows: Vec<String> = sqlx::query("SELECT detail::text AS d FROM audit_events WHERE patient_ref = $1 AND detail IS NOT NULL")
            .bind(format!("Patient/{pid}"))
            .fetch_all(&ctx.pool())
            .await
            .unwrap()
            .iter()
            .map(|r| r.get("d"))
            .collect();
        let all = rows.join("\n");
        for secret in ["QUOKKA", "WOMBAT", "Caldicott", "privacy officer"] {
            assert!(!all.contains(secret), "{secret}: {all}");
        }
    })
    .await;
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn the_administrator_declines_and_the_patient_reads_their_own() {
    let idp = TestIdp::start().await;
    let map: Value = ["clinical-admin", "clinician", "nurse", "patient"]
        .iter()
        .map(|r| ((*r).to_string(), json!({ "role": [r] })))
        .collect();
    let db_url = db_url();
    let role_map = map.to_string();
    let file = pack("considered");
    let settings: Vec<(&str, &str)> = vec![
        ("DB_URL", db_url),
        ("EPR_REQUIRE_AUTH", "true"),
        ("EPR_KEYCLOAK_URL", &idp.base_url),
        ("EPR_KEYCLOAK_REALM", REALM),
        ("EPR_KEYCLOAK_AUDIENCES", AUDIENCE),
        ("EPR_KEYCLOAK_ROLE_MAP", &role_map),
        ("EPR_DISCLOSURE_RULES_FILE", file),
    ];
    let sub = |n: u8| format!("0c4f1e2a-0000-4000-8000-00000000f6{n:02x}");
    let token = |role: &str, n: u8| {
        idp.token(|c| {
            c["realm_access"] = json!({ "roles": [role] });
            c["sub"] = json!(sub(n));
        })
    };
    let (admin, doctor, nurse, own) = (
        token("clinical-admin", 0),
        token("clinician", 1),
        token("nurse", 2),
        token("patient", 3),
    );
    request_with::<App, _, _>(&settings, |server, _ctx| async move {
        let server = &server;
        let call = |token: &str, purpose: &'static str, path: String, body: Option<Value>| {
            let auth = HeaderValue::from_str(&format!("Bearer {token}")).unwrap();
            async move {
                let r = match &body {
                    Some(b) => server.post(&path).json(b),
                    None => server.get(&path),
                };
                let r = r
                    .add_header(HeaderName::from_static("authorization"), auth)
                    .add_header(HeaderName::from_static("x-purpose-of-use"), HeaderValue::from_static(purpose))
                    .await;
                (r.status_code(), r.text())
            }
        };
        let (status, text) = call(&admin, "operations", "/api/patients".into(), Some(json!({
            "patient": { "names": [{ "family": format!("dr{}", letters()), "given": ["G"] }], "birth_date": "1970-01-01" },
            "override_reason": "test setup" }))).await;
        assert_eq!(status, StatusCode::CREATED, "{text}");
        let pid = serde_json::from_str::<Value>(&text).unwrap()["id"].as_str().unwrap().to_string();
        for (n, kind) in [(1, "care-team"), (2, "care-team"), (3, "self")] {
            let (status, text) = call(&admin, "operations", format!("/api/patients/{pid}/relationships"),
                Some(json!({ "relationship": { "subject": sub(n), "kind": kind, "status": "active" }, "reason": "test" }))).await;
            assert_eq!(status, StatusCode::CREATED, "{text}");
        }
        let (status, text) = call(&admin, "operations", format!("/api/patients/{pid}/consents"), Some(json!({ "consent": {
            "kind": "general", "status": "active", "provisions": [{ "effect": "deny", "purposes": ["research"] }] } }))).await;
        assert_eq!(status, StatusCode::CREATED, "{text}");
        let refusal = serde_json::from_str::<Value>(&text).unwrap()["id"].as_str().unwrap().to_string();
        let list = format!("/api/patients/{pid}/declined-refusals");
        let body = json!({ "consent_id": refusal, "declined": declined() });
        // A clinician reads and cannot decline; the administrator declines.
        assert_eq!(call(&doctor, "treatment", list.clone(), Some(body.clone())).await.0, StatusCode::FORBIDDEN);
        let (status, text) = call(&admin, "operations", list.clone(), Some(body)).await;
        assert_eq!(status, StatusCode::CREATED, "{text}");
        assert_eq!(call(&doctor, "treatment", list.clone(), None).await.0, StatusCode::OK);
        assert_eq!(call(&nurse, "treatment", list.clone(), None).await.0, StatusCode::FORBIDDEN);
        // The patient is told: they read their own.
        let (status, text) = call(&own, "patient-access", list.clone(), None).await;
        assert_eq!(status, StatusCode::OK, "{text}");
        assert_eq!(serde_json::from_str::<Value>(&text).unwrap()["declines"].as_array().unwrap().len(), 1);
    })
    .await;
}
