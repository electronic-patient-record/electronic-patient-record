//! DB-gated, through the real router with login **on** (DM-1): who may record and read compatibility typing; that a
//! patient reads none; that reports which disagree are said to, across a merge too; that nothing names another person;
//! and that the audit trail never holds a group, an allele, a sample or a laboratory. And, with no database, that nothing
//! in the service matches a donor.
//!
//! ```text
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test main http_typing:: -- --ignored --test-threads=1
//! ```

use authentication_verifier::test_idp::{AUDIENCE, REALM, TestIdp};
use axa_test::prelude::request_with;
use axum::http::{HeaderName, HeaderValue, StatusCode};
use electronic_patient_record::app::App;
use electronic_patient_record::pool::Pool;
use electronic_patient_record::{openapi, routes};
use electronic_patient_record_matcher::{HumanName, Patient};
use serde_json::{Value, json};
use sqlx::Row;

fn bearer(token: &str) -> (HeaderName, HeaderValue) {
    (
        HeaderName::from_static("authorization"),
        HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
    )
}

fn abo(group: &str, sample: &str, at: &str) -> Value {
    json!({ "finding": { "kind": "abo", "group": group }, "laboratory": "SECRETlab", "sample": sample,
            "collected_at": at, "status": "reported" })
}

/// A route, or a schema of this feature, that said who may give to whom would be a match. A prompt for someone to look.
#[test]
fn nothing_in_the_service_matches_a_donor() {
    let words = [
        "donor",
        "recipient-of",
        "compatible",
        "crossmatch",
        "allocat",
    ];
    for (_, path, resource, _) in routes::TABLE {
        let name = format!("{path} {}", resource.name()).to_lowercase();
        assert!(words.iter().all(|w| !name.contains(w)), "{name}");
    }
    let document = openapi::document();
    let schemas = document["components"]["schemas"].as_object().unwrap();
    let mut seen = 0;
    for (name, schema) in schemas.iter().filter(|(n, _)| n.contains("Typing")) {
        seen += 1;
        let mut text = schema.clone();
        // The description may say what is not done.
        text.as_object_mut().unwrap().remove("description");
        let text = text.to_string().to_lowercase();
        for w in [
            "donor",
            "recipient",
            "\"match",
            "compatible\"",
            "crossmatch",
        ] {
            assert!(!text.contains(w), "{name}: {w}");
        }
    }
    assert_eq!(seen, 8);
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn who_may_record_and_read_typing_and_that_disagreement_is_said() {
    let idp = TestIdp::start().await;
    let roles = [
        "clinical-admin",
        "clinician",
        "nurse",
        "pharmacist",
        "allied-health",
        "receptionist",
        "patient",
    ];
    let map: Value = roles
        .iter()
        .map(|r| ((*r).to_string(), json!({ "role": [r] })))
        .collect();
    let db_url = std::env::var("DB_URL").expect("DB_URL must be set for --ignored tests");
    let role_map = map.to_string();
    let settings: Vec<(&str, &str)> = vec![
        ("DB_URL", &db_url),
        ("EPR_REQUIRE_AUTH", "true"),
        ("EPR_KEYCLOAK_URL", &idp.base_url),
        ("EPR_KEYCLOAK_REALM", REALM),
        ("EPR_KEYCLOAK_AUDIENCES", AUDIENCE),
        ("EPR_KEYCLOAK_ROLE_MAP", &role_map),
    ];
    let sub = |role: &str| {
        let n = roles.iter().position(|r| *r == role).unwrap();
        format!("0c4f1e2a-0000-4000-8000-0000000001d{n}")
    };
    let token = |role: &str| {
        idp.token(|c| {
            c["realm_access"] = json!({ "roles": [role] });
            c["sub"] = json!(sub(role));
        })
    };

    request_with::<App, _, _>(&settings, |server, ctx| async move {
        let server = &server;
        let call = |method: &'static str, role: &str, path: String, body: Value| {
            let (h, v) = bearer(&token(role));
            let purpose = if role == "patient" { "patient-access" } else { "treatment" };
            async move {
                let r = match method {
                    "GET" => server.get(&path),
                    "POST" => server.post(&path).json(&body),
                    _ => server.put(&path).json(&body),
                };
                let r = r
                    .add_header(h, v)
                    .add_header(
                        HeaderName::from_static("x-purpose-of-use"),
                        HeaderValue::from_str(purpose).unwrap(),
                    )
                    .await;
                let text = r.text();
                (r.status_code(), serde_json::from_str::<Value>(&text).unwrap_or(json!(text)))
            }
        };
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let family = format!("typ{stamp}");
        let person = serde_json::to_value(Patient {
            names: vec![HumanName { family: Some(family.clone()), given: vec![family] }],
            birth_date: Some("1966-07-08".into()),
            ..Default::default()
        })
        .unwrap();
        let mut ids = Vec::new();
        for _ in 0..2 {
            let (status, made) = call("POST", "clinical-admin", "/api/patients".into(),
                json!({ "patient": person, "override_reason": "typing test" })).await;
            assert_eq!(status, StatusCode::CREATED, "{made}");
            let id = made["id"].as_str().unwrap().to_string();
            for (role, kind) in [
                ("clinician", "care-team"),
                ("nurse", "care-team"),
                ("pharmacist", "care-team"),
                ("allied-health", "care-team"),
                ("patient", "self"),
            ] {
                let (status, text) = call("POST", "clinical-admin", format!("/api/patients/{id}/relationships"),
                    json!({ "relationship": { "subject": sub(role), "kind": kind, "status": "active" }, "reason": "t" })).await;
                assert_eq!(status, StatusCode::CREATED, "{text}");
            }
            ids.push(id);
        }
        let url = |id: &str| format!("/api/patients/{id}/compatibility-typing");
        let (kept, gone) = (ids[0].clone(), ids[1].clone());
        let t1 = "2026-11-20T09:00:00Z";
        let t2 = "2026-11-21T09:00:00Z";
        let body = |r: Value| json!({ "record": r, "reason": "SECRETreason" });

        // Only a clinician records; a nurse reads; nobody else has any of it.
        let (status, made) = call("POST", "clinician", url(&kept), body(abo("a", "SECRET1", t1))).await;
        assert_eq!(status, StatusCode::CREATED, "{made}");
        for role in ["nurse", "pharmacist", "allied-health", "receptionist", "patient"] {
            let (status, _) = call("POST", role, url(&kept), body(abo("a", "S", t1))).await;
            assert_eq!(status, StatusCode::FORBIDDEN, "{role}");
        }
        for (role, want) in [
            ("clinician", StatusCode::OK),
            ("nurse", StatusCode::OK),
            ("pharmacist", StatusCode::FORBIDDEN),
            ("allied-health", StatusCode::FORBIDDEN),
            ("receptionist", StatusCode::FORBIDDEN),
            ("patient", StatusCode::FORBIDDEN),
        ] {
            for path in [url(&kept), format!("{}/as-at?time=2026-11-24T00:00:00Z", url(&kept))] {
                let (status, text) = call("GET", role, path, Value::Null).await;
                assert_eq!(status, want, "{role}: {text}");
            }
        }
        let agreement = async |id: &str| {
            let (status, list) = call("GET", "nurse", url(id), Value::Null).await;
            assert_eq!(status, StatusCode::OK);
            list["summary"]["abo"].clone()
        };
        assert_eq!(agreement(&kept).await, "one-collection");
        // A second tube at the same time is the same collection; a later one that agrees is not.
        call("POST", "clinician", url(&kept), body(abo("a", "SECRET2", t1))).await;
        assert_eq!(agreement(&kept).await, "one-collection");
        call("POST", "clinician", url(&kept), body(abo("a", "SECRET3", t2))).await;
        assert_eq!(agreement(&kept).await, "agree");

        // Another record of "the same person" has another group: after a merge the list says so.
        call("POST", "clinician", url(&gone), body(abo("o", "SECRET4", t2))).await;
        let (status, text) = call("POST", "clinical-admin", format!("/api/patients/{kept}/merge"),
            json!({ "source": gone, "reason": "registered twice" })).await;
        assert!(status.is_success(), "{text}");
        assert_eq!(agreement(&kept).await, "disagree");

        // A report of another person, or a conclusion, is refused; so is a future sample.
        for bad in [
            json!({ "finding": { "kind": "crossmatch" }, "laboratory": "L", "sample": "S", "collected_at": t1, "status": "reported" }),
            {
                let mut r = abo("a", "S", t1);
                r["donor"] = json!("someone");
                r
            },
            abo("a", "S", "2999-01-01T00:00:00Z"),
        ] {
            let (status, _) = call("POST", "clinician", url(&kept), body(bad)).await;
            assert!(status.is_client_error(), "{status}");
        }
        // Only a clinician retracts; the retracted report no longer counts.
        let id = made["id"].as_str().unwrap();
        let mut mistake = abo("a", "SECRET1", t1);
        mistake["status"] = json!("entered-in-error");
        let change = json!({ "record": mistake, "expected_version": 1, "reason": "SECRETreason" });
        let (status, _) = call("PUT", "nurse", format!("{}/{id}", url(&kept)), change.clone()).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        let (status, text) = call("PUT", "clinician", format!("{}/{id}", url(&kept)), change).await;
        assert_eq!(status, StatusCode::OK, "{text}");

        let rows = sqlx::query(
            "SELECT detail::text AS d FROM audit_events WHERE resource_ref LIKE 'CompatibilityTyping%'
                 AND (patient_ref = ANY($1) OR patient_ref IS NULL)",
        )
        .bind(vec![format!("Patient/{kept}"), format!("Patient/{gone}")])
        .fetch_all(&ctx.pool())
        .await
        .unwrap();
        assert!(rows.len() > 10, "{}", rows.len());
        for r in &rows {
            let d: String = r.try_get::<Option<String>, _>("d").unwrap().unwrap_or_else(String::new);
            for word in ["SECRET", "\"abo\"", "group"] {
                assert!(!d.contains(word), "{word} in the audit trail: {d}");
            }
        }
    })
    .await;
}
