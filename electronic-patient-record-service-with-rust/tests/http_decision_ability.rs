//! DB-gated, through the real router (RF-8): a judgement of whether a person can make one decision is recorded as made,
//! listed newest first with its age and gaps, never changed, and marked entered in error by a clinician only, with a
//! reason; two people who disagree show as a dispute; the audit trail holds no word of the decision or the reasons.
//!
//! ```text
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test main http_decision_ability:: -- --ignored --test-threads=1
//! ```

use authentication_verifier::test_idp::{AUDIENCE, REALM, TestIdp};
use axa_test::prelude::request_with;
use axum::http::{HeaderName, HeaderValue, StatusCode};
use electronic_patient_record::app::App;
use electronic_patient_record::pool::Pool;
use electronic_patient_record_matcher::{HumanName, Patient};
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

fn person() -> Value {
    let family = format!("da{}", letters());
    serde_json::to_value(Patient {
        names: vec![HumanName {
            family: Some(family.clone()),
            given: vec![family],
        }],
        birth_date: Some("1950-03-04".into()),
        ..Default::default()
    })
    .unwrap()
}

fn judged(at: &str, by: &str, decides: &str) -> Value {
    json!({ "about": "whether to accept QUOKKA", "at": at, "conscious": "yes", "communicates": "speaks",
        "decides": decides, "help_given": ["time"], "because": "explained WOMBAT back", "by": by })
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_judgement_is_kept_as_made_and_two_who_disagree_show_as_a_dispute() {
    let url: &'static str = Box::leak(
        std::env::var("DB_URL")
            .expect("DB_URL must be set for --ignored tests")
            .into_boxed_str(),
    );
    let settings = vec![("DB_URL", url), ("EPR_REQUIRE_AUTH", "false")];
    request_with::<App, _, _>(&settings, |server, ctx| async move {
        let made = server
            .post("/api/patients")
            .json(&json!({ "patient": person(), "override_reason": "test setup" }))
            .await;
        assert_eq!(made.status_code(), StatusCode::CREATED, "{}", made.text());
        let pid = made.json::<Value>()["id"].as_str().unwrap().to_string();
        let list = format!("/api/patients/{pid}/decision-ability");

        let empty = server.get(&list).await.json::<Value>();
        assert_eq!(empty, json!({ "judgements": [], "disputed": [] }));

        let first = server
            .post(&list)
            .json(&json!({ "judgement": judged("2026-11-26T09:00:00Z", "Dr A", "can") }))
            .await;
        assert_eq!(first.status_code(), StatusCode::CREATED, "{}", first.text());
        let first_id = first.json::<Value>()["id"].as_str().unwrap().to_string();
        // An unaided judgement that they cannot decide is kept, with its gap said.
        let mut unaided = judged("2026-11-26T10:00:00Z", "Dr B", "cannot");
        unaided["help_given"] = json!([]);
        let second = server.post(&list).json(&json!({ "judgement": unaided })).await;
        assert_eq!(second.status_code(), StatusCode::CREATED, "{}", second.text());

        let shown = server.get(&list).await.json::<Value>();
        let items = shown["judgements"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["judgement"]["by"], "Dr B", "newest first");
        assert_eq!(items[0]["gaps"].as_array().unwrap().len(), 1);
        assert!(items[0]["age_minutes"].as_i64().is_some());
        assert_eq!(shown["disputed"], json!(["whether to accept QUOKKA"]));

        // A score, a time in the future and a blank reason are refused.
        let mut scored = judged("2026-11-26T09:00:00Z", "Dr A", "can");
        scored["score"] = json!(3);
        assert!(server.post(&list).json(&json!({ "judgement": scored })).await.status_code().is_client_error());
        let future = server
            .post(&list)
            .json(&json!({ "judgement": judged("2999-01-01T00:00:00Z", "Dr A", "can") }))
            .await;
        assert_eq!(future.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        // There is no route that changes a judgement.
        let changed = server
            .put(&format!("{list}/{first_id}"))
            .json(&json!({ "judgement": judged("2026-11-26T09:00:00Z", "Dr A", "cannot") }))
            .await;
        assert!(changed.status_code().is_client_error(), "{}", changed.status_code());

        // Entered in error needs a reason and the current version; then it stays, and counts for nothing.
        let eie = format!("{list}/{first_id}/entered-in-error");
        let blank = server.post(&eie).json(&json!({ "expected_version": 1, "reason": " " })).await;
        assert_eq!(blank.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        let stale = server.post(&eie).json(&json!({ "expected_version": 2, "reason": "wrong patient" })).await;
        assert_eq!(stale.status_code(), StatusCode::CONFLICT);
        let done = server.post(&eie).json(&json!({ "expected_version": 1, "reason": "wrong patient" })).await;
        assert_eq!(done.status_code(), StatusCode::OK, "{}", done.text());
        assert_eq!(done.json::<Value>()["state"], "entered-in-error");
        let again = server.post(&eie).json(&json!({ "expected_version": 2, "reason": "again" })).await;
        assert_eq!(again.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        let after = server.get(&list).await.json::<Value>();
        assert_eq!(after["judgements"].as_array().unwrap().len(), 2, "it stays");
        assert_eq!(after["disputed"], json!([]), "one entered in error counts for nothing");

        // The trail names no decision, reason or conclusion.
        let rows: Vec<String> = sqlx::query("SELECT detail::text AS d FROM audit_events WHERE patient_ref = $1 AND detail IS NOT NULL")
            .bind(format!("Patient/{pid}"))
            .fetch_all(&ctx.pool())
            .await
            .unwrap()
            .iter()
            .map(|r| r.get("d"))
            .collect();
        assert!(!rows.is_empty());
        let all = rows.join("\n");
        for secret in ["QUOKKA", "WOMBAT", "cannot", "wrong patient"] {
            assert!(!all.contains(secret), "{secret}: {all}");
        }
    })
    .await;
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn those_who_treat_record_and_read_and_only_a_clinician_marks_one_entered_in_error() {
    let idp = TestIdp::start().await;
    let map: Value = ["clinical-admin", "clinician", "nurse", "pharmacist"]
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
    let sub = |n: u8| format!("0c4f1e2a-0000-4000-8000-00000000f3{n:02x}");
    let token = |role: &str, n: u8| {
        idp.token(|c| {
            c["realm_access"] = json!({ "roles": [role] });
            c["sub"] = json!(sub(n));
        })
    };
    let admin = token("clinical-admin", 0);
    let (doctor, nurse, pharmacist) = (
        token("clinician", 1),
        token("nurse", 2),
        token("pharmacist", 3),
    );
    request_with::<App, _, _>(&settings, |server, _ctx| async move {
        let server = &server;
        let call = |method: &'static str, token: &str, path: String, body: Value| {
            let auth = HeaderValue::from_str(&format!("Bearer {token}")).unwrap();
            async move {
                let r = match method {
                    "GET" => server.get(&path),
                    _ => server.post(&path).json(&body),
                };
                let r = r
                    .add_header(HeaderName::from_static("authorization"), auth)
                    .add_header(HeaderName::from_static("x-purpose-of-use"), HeaderValue::from_static("treatment"))
                    .await;
                (r.status_code(), r.text())
            }
        };
        let (status, text) = call("POST", &admin, "/api/patients".into(),
            json!({ "patient": person(), "override_reason": "test setup" })).await;
        assert_eq!(status, StatusCode::CREATED, "{text}");
        let pid = serde_json::from_str::<Value>(&text).unwrap()["id"].as_str().unwrap().to_string();
        for n in 1..=2 {
            let (status, text) = call("POST", &admin, format!("/api/patients/{pid}/relationships"),
                json!({ "relationship": { "subject": sub(n), "kind": "care-team", "status": "active" }, "reason": "test" })).await;
            assert_eq!(status, StatusCode::CREATED, "{text}");
        }
        let list = format!("/api/patients/{pid}/decision-ability");
        let (status, text) = call("POST", &nurse, list.clone(),
            json!({ "judgement": judged("2026-11-26T09:00:00Z", "Nurse N", "can") })).await;
        assert_eq!(status, StatusCode::CREATED, "{text}");
        let jid = serde_json::from_str::<Value>(&text).unwrap()["id"].as_str().unwrap().to_string();
        assert_eq!(call("GET", &admin, list.clone(), Value::Null).await.0, StatusCode::FORBIDDEN);
        assert_eq!(call("GET", &pharmacist, list.clone(), Value::Null).await.0, StatusCode::FORBIDDEN);
        assert_eq!(call("GET", &nurse, list.clone(), Value::Null).await.0, StatusCode::OK);
        let eie = format!("{list}/{jid}/entered-in-error");
        let body = json!({ "expected_version": 1, "reason": "wrong patient" });
        assert_eq!(call("POST", &nurse, eie.clone(), body.clone()).await.0, StatusCode::FORBIDDEN);
        let (status, text) = call("POST", &doctor, eie, body).await;
        assert_eq!(status, StatusCode::OK, "{text}");
    })
    .await;
}
