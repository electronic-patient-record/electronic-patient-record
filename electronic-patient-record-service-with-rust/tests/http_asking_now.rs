//! DB-gated, through the real router (RF-10): a record of asking now, or of leaving against advice, is kept as made,
//! listed newest first, never changed, and marked entered in error by a clinician only, with a reason; the audit trail
//! holds no word of what was offered or refused.
//!
//! ```text
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test main http_asking_now:: -- --ignored --test-threads=1
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
    let family = format!("an{}", letters());
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

fn asked(at: &str, leaving: bool) -> Value {
    json!({ "offered": "admission for QUOKKA",
        "explained": { "risks": "yes", "benefits": "yes", "alternatives": "not-known", "helped_by": "a telephone interpreter" },
        "refused": "to stay WOMBAT", "accepted": "a blood test", "told_may_change_mind": "yes",
        "in_private": "no", "with_whom": "their partner", "leaving_against_advice": leaving,
        "written": "yes", "signed": "no", "witnessed": "no", "at": at, "by": "Dr A, emergency department" })
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn with_a_pack_in_force_each_record_says_what_the_place_asks_for_and_without_one_nothing_is_said()
 {
    let url: &'static str = Box::leak(
        std::env::var("DB_URL")
            .expect("DB_URL must be set for --ignored tests")
            .into_boxed_str(),
    );
    // The shipped pack for England and Wales has no `refusal-record` rule; this place's adds one.
    let text = format!(
        "{}\n[[rule]]\nkey = \"refusal-record\"\nvalue = [\"written\", \"signed\", \"alternatives-explained\", \"in-private\"]\nsource = \"a test place\"\n",
        electronic_patient_record::refusal_rules::ENGLAND_WALES
    );
    let path = std::env::temp_dir().join(format!("epr-rf3-http-{}.toml", std::process::id()));
    std::fs::write(&path, text).unwrap();
    let file: &'static str = Box::leak(path.to_string_lossy().into_owned().into_boxed_str());
    for with_pack in [true, false] {
        let mut settings = vec![("DB_URL", url), ("EPR_REQUIRE_AUTH", "false")];
        if with_pack {
            settings.push(("EPR_REFUSAL_RULES_FILE", file));
        }
        request_with::<App, _, _>(&settings, |server, _ctx| async move {
            let made = server
                .post("/api/patients")
                .json(&json!({ "patient": person(), "override_reason": "test setup" }))
                .await;
            assert_eq!(made.status_code(), StatusCode::CREATED, "{}", made.text());
            let pid = made.json::<Value>()["id"].as_str().unwrap().to_string();
            let list = format!("/api/patients/{pid}/asking-now");
            let r = server
                .post(&list)
                .json(&json!({ "asked": asked("2026-11-26T09:00:00Z", false) }))
                .await;
            assert_eq!(r.status_code(), StatusCode::CREATED, "{}", r.text());
            let shown = server.get(&list).await.json::<Value>();
            if with_pack {
                assert_eq!(shown["pack"]["place"], "England and Wales");
                assert_eq!(
                    shown["pack"]["review"], "unreviewed",
                    "nobody is named in the shipped pack"
                );
                // Signed "no", alternatives "not known" (never done) and not in private: in the pack's order.
                assert_eq!(
                    shown["records"][0]["pack_gaps"],
                    json!(["signed", "alternatives-explained", "in-private"])
                );
            } else {
                // No file is no rule: nothing is said, which is not "nothing is missing".
                assert_eq!(shown["pack"], Value::Null);
                assert_eq!(shown["records"][0]["pack_gaps"], Value::Null);
            }
        })
        .await;
    }
    let _ = std::fs::remove_file(path);
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_record_of_asking_is_kept_as_made_and_marked_in_error_only_with_a_reason() {
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
        let list = format!("/api/patients/{pid}/asking-now");
        assert_eq!(
            server.get(&list).await.json::<Value>(),
            json!({ "records": [], "left_against_advice": 0, "pack": null })
        );

        let first = server.post(&list).json(&json!({ "asked": asked("2026-11-26T09:00:00Z", false) })).await;
        assert_eq!(first.status_code(), StatusCode::CREATED, "{}", first.text());
        let first_id = first.json::<Value>()["id"].as_str().unwrap().to_string();
        let left = server.post(&list).json(&json!({ "asked": asked("2026-11-26T10:00:00Z", true) })).await;
        assert_eq!(left.status_code(), StatusCode::CREATED, "{}", left.text());
        let left_id = left.json::<Value>()["id"].as_str().unwrap().to_string();

        let shown = server.get(&list).await.json::<Value>();
        assert_eq!(shown["records"].as_array().unwrap().len(), 2);
        assert_eq!(shown["records"][0]["id"], left_id.as_str(), "newest first");
        assert_eq!(shown["records"][0]["from_merged_record"], false);
        assert_eq!(shown["left_against_advice"], 1);

        // Leaving against advice says what was refused; a record with neither refused nor accepted is refused.
        let mut unsaid = asked("2026-11-26T10:00:00Z", true);
        unsaid["refused"] = json!(null);
        assert_eq!(server.post(&list).json(&json!({ "asked": unsaid })).await.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        let future = server.post(&list).json(&json!({ "asked": asked("2999-01-01T00:00:00Z", false) })).await;
        assert_eq!(future.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        // There is no route that changes a record.
        let changed = server.put(&format!("{list}/{first_id}")).json(&json!({ "asked": asked("2026-11-26T09:00:00Z", true) })).await;
        assert!(changed.status_code().is_client_error(), "{}", changed.status_code());

        // Entered in error: a reason and the current version; then it stays, and counts for nothing.
        let eie = format!("{list}/{left_id}/entered-in-error");
        assert_eq!(server.post(&eie).json(&json!({ "expected_version": 1, "reason": " " })).await.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(server.post(&eie).json(&json!({ "expected_version": 2, "reason": "wrong patient" })).await.status_code(), StatusCode::CONFLICT);
        let done = server.post(&eie).json(&json!({ "expected_version": 1, "reason": "wrong patient" })).await;
        assert_eq!(done.status_code(), StatusCode::OK, "{}", done.text());
        let after = server.get(&list).await.json::<Value>();
        assert_eq!(after["records"].as_array().unwrap().len(), 2, "it stays");
        assert_eq!(after["left_against_advice"], 0, "one entered in error counts for nothing");

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
        for secret in ["QUOKKA", "WOMBAT", "interpreter", "wrong patient"] {
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
    let sub = |n: u8| format!("0c4f1e2a-0000-4000-8000-00000000f5{n:02x}");
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
        let list = format!("/api/patients/{pid}/asking-now");
        let (status, text) = call("POST", &nurse, list.clone(), json!({ "asked": asked("2026-11-26T09:00:00Z", true) })).await;
        assert_eq!(status, StatusCode::CREATED, "{text}");
        let rid = serde_json::from_str::<Value>(&text).unwrap()["id"].as_str().unwrap().to_string();
        assert_eq!(call("GET", &admin, list.clone(), Value::Null).await.0, StatusCode::FORBIDDEN);
        assert_eq!(call("GET", &pharmacist, list.clone(), Value::Null).await.0, StatusCode::FORBIDDEN);
        assert_eq!(call("GET", &nurse, list.clone(), Value::Null).await.0, StatusCode::OK);
        let eie = format!("{list}/{rid}/entered-in-error");
        let body = json!({ "expected_version": 1, "reason": "wrong patient" });
        assert_eq!(call("POST", &nurse, eie.clone(), body.clone()).await.0, StatusCode::FORBIDDEN);
        let (status, text) = call("POST", &doctor, eie, body).await;
        assert_eq!(status, StatusCode::OK, "{text}");
    })
    .await;
}
