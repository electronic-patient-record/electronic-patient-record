//! DB-gated, through the real router (RF-9): who speaks for a person is recorded one entry per person, changed as a new
//! version, marked entered in error by a clinician only, with a reason; the list keeps a power not seen apart and shows
//! where the powers that were seen disagree; the audit trail holds no names, paper or views.
//!
//! ```text
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test main http_speakers:: -- --ignored --test-threads=1
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

fn patient() -> Value {
    json!({ "patient": { "names": [{ "family": format!("sk{}", letters()), "given": ["G"] }], "birth_date": "2015-01-01" },
        "override_reason": "test setup" })
}

fn parent(name: &str, paper: &str, says: &str) -> Value {
    let mut p = json!({ "role": "parent", "name": name, "paper": paper,
        "views": [{ "about": "whether to give blood", "says": says }] });
    if paper != "not-seen" {
        p["seen_by"] = json!("ward clerk, QUOKKA certificate, today");
    }
    p
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn two_parents_who_disagree_are_two_entries_and_a_power_not_seen_is_not_a_power() {
    let url: &'static str = Box::leak(
        std::env::var("DB_URL")
            .expect("DB_URL must be set for --ignored tests")
            .into_boxed_str(),
    );
    let settings = vec![("DB_URL", url), ("EPR_REQUIRE_AUTH", "false")];
    request_with::<App, _, _>(&settings, |server, ctx| async move {
        let made = server.post("/api/patients").json(&patient()).await;
        assert_eq!(made.status_code(), StatusCode::CREATED, "{}", made.text());
        let pid = made.json::<Value>()["id"].as_str().unwrap().to_string();
        let list = format!("/api/patients/{pid}/speakers");

        let a = server.post(&list).json(&json!({ "speaker": parent("Parent A", "original", "agrees") })).await;
        assert_eq!(a.status_code(), StatusCode::CREATED, "{}", a.text());
        let a_id = a.json::<Value>()["id"].as_str().unwrap().to_string();
        let b = server.post(&list).json(&json!({ "speaker": parent("Parent B", "not-seen", "refuses") })).await;
        assert_eq!(b.status_code(), StatusCode::CREATED, "{}", b.text());
        let b_id = b.json::<Value>()["id"].as_str().unwrap().to_string();

        let shown = server.get(&list).await.json::<Value>();
        assert_eq!(shown["speakers"].as_array().unwrap().len(), 2, "two parents, two entries");
        assert_eq!(shown["summary"]["powers_seen"], json!([a_id]));
        assert_eq!(shown["summary"]["claimed_not_seen"], json!([b_id]), "a power not seen is not a power");
        assert_eq!(shown["summary"]["powers_disagree"], json!([]), "a claimed power makes no dispute");

        // Parent B's paper is seen later: a new version, and now the powers disagree.
        let one = format!("{list}/{b_id}");
        let seen = server.put(&one).json(&json!({ "speaker": parent("Parent B", "copy", "refuses"), "expected_version": 1 })).await;
        assert_eq!(seen.status_code(), StatusCode::OK, "{}", seen.text());
        assert_eq!(seen.json::<Value>()["version"], 2);
        let shown = server.get(&list).await.json::<Value>();
        assert_eq!(shown["summary"]["powers_disagree"], json!(["whether to give blood"]));
        let stale = server.put(&one).json(&json!({ "speaker": parent("Parent B", "copy", "agrees"), "expected_version": 1 })).await;
        assert_eq!(stale.status_code(), StatusCode::CONFLICT);

        // A relative's paper is refused; nobody beside a speaker is a contradiction.
        let r = server.post(&list).json(&json!({ "speaker": { "role": "relative", "name": "an uncle", "paper": "original", "seen_by": "x" } })).await;
        assert_eq!(r.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        let nobody = server.post(&list).json(&json!({ "speaker": { "role": "nobody" } })).await;
        assert_eq!(nobody.status_code(), StatusCode::CREATED, "{}", nobody.text());
        assert_eq!(server.get(&list).await.json::<Value>()["summary"]["contradicts"], true);

        // Entered in error needs a reason; then nothing changes it, and it counts for nothing.
        let mistaken = json!({ "speaker": parent("Parent B", "copy", "refuses"), "expected_version": 2, "state": "entered-in-error" });
        assert_eq!(server.put(&one).json(&mistaken).await.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        let mut with_reason = mistaken.clone();
        with_reason["reason"] = json!("WOMBAT");
        let done = server.put(&one).json(&with_reason).await;
        assert_eq!(done.status_code(), StatusCode::OK, "{}", done.text());
        let again = server.put(&one).json(&json!({ "speaker": parent("Parent B", "copy", "agrees"), "expected_version": 3 })).await;
        assert_eq!(again.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        let shown = server.get(&list).await.json::<Value>();
        assert_eq!(shown["summary"]["powers_disagree"], json!([]), "one entered in error counts for nothing");
        assert_eq!(shown["speakers"].as_array().unwrap().len(), 3, "it stays");

        let rows: Vec<String> = sqlx::query("SELECT detail::text AS d FROM audit_events WHERE patient_ref = $1 AND detail IS NOT NULL")
            .bind(format!("Patient/{pid}"))
            .fetch_all(&ctx.pool())
            .await
            .unwrap()
            .iter()
            .map(|r| r.get("d"))
            .collect();
        let all = rows.join("\n");
        for secret in ["QUOKKA", "WOMBAT", "Parent A", "blood"] {
            assert!(!all.contains(secret), "{secret}: {all}");
        }
    })
    .await;
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn those_who_treat_record_and_only_a_clinician_marks_one_entered_in_error() {
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
    let sub = |n: u8| format!("0c4f1e2a-0000-4000-8000-00000000f7{n:02x}");
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
                    "POST" => server.post(&path).json(&body),
                    _ => server.put(&path).json(&body),
                };
                let r = r
                    .add_header(HeaderName::from_static("authorization"), auth)
                    .add_header(HeaderName::from_static("x-purpose-of-use"), HeaderValue::from_static("treatment"))
                    .await;
                (r.status_code(), r.text())
            }
        };
        let (status, text) = call("POST", &admin, "/api/patients".into(), patient()).await;
        assert_eq!(status, StatusCode::CREATED, "{text}");
        let pid = serde_json::from_str::<Value>(&text).unwrap()["id"].as_str().unwrap().to_string();
        for n in 1..=2 {
            let (status, text) = call("POST", &admin, format!("/api/patients/{pid}/relationships"),
                json!({ "relationship": { "subject": sub(n), "kind": "care-team", "status": "active" }, "reason": "test" })).await;
            assert_eq!(status, StatusCode::CREATED, "{text}");
        }
        let list = format!("/api/patients/{pid}/speakers");
        let (status, text) = call("POST", &nurse, list.clone(), json!({ "speaker": parent("Parent A", "original", "agrees") })).await;
        assert_eq!(status, StatusCode::CREATED, "{text}");
        let sid = serde_json::from_str::<Value>(&text).unwrap()["id"].as_str().unwrap().to_string();
        assert_eq!(call("GET", &admin, list.clone(), Value::Null).await.0, StatusCode::FORBIDDEN);
        assert_eq!(call("GET", &pharmacist, list.clone(), Value::Null).await.0, StatusCode::FORBIDDEN);
        assert_eq!(call("GET", &nurse, list.clone(), Value::Null).await.0, StatusCode::OK);
        let mistaken = json!({ "speaker": parent("Parent A", "original", "agrees"), "expected_version": 1,
            "state": "entered-in-error", "reason": "wrong patient" });
        assert_eq!(call("PUT", &nurse, format!("{list}/{sid}"), mistaken.clone()).await.0, StatusCode::FORBIDDEN);
        let (status, text) = call("PUT", &doctor, format!("{list}/{sid}"), mistaken).await;
        assert_eq!(status, StatusCode::OK, "{text}");
    })
    .await;
}
