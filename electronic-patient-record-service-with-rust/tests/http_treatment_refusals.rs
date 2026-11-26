//! DB-gated, through the real router with login off (RF-5): a refusal of treatment is recorded, doubted and entered in
//! error as new versions; a withdrawal that was heard is never stored as ending one (no rule is loaded); "none recorded"
//! is what an empty list says; the audit trail holds no word of what was refused; and a merge or an unmerge stops for each
//! refusal until a person has named it. With login on: who may record, read and mark one entered in error.
//!
//! ```text
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test main http_treatment_refusals:: -- --ignored --test-threads=1
//! ```

use authentication_verifier::test_idp::{AUDIENCE, REALM, TestIdp};
use axa_test::prelude::request_with;
use axum::http::{HeaderName, HeaderValue, StatusCode};
use electronic_patient_record::app::App;
use electronic_patient_record::pool::Pool;
use electronic_patient_record_matcher::{HumanName, Patient};
use serde_json::{Value, json};
use sqlx::Row;

fn settings() -> Vec<(&'static str, &'static str)> {
    let url: &'static str = Box::leak(
        std::env::var("DB_URL")
            .expect("DB_URL must be set for --ignored tests")
            .into_boxed_str(),
    );
    vec![("DB_URL", url), ("EPR_REQUIRE_AUTH", "false")]
}

fn nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

/// The same person under a family name of letters unique to this call, so the matcher agrees to merge two of them.
fn person(family: &str) -> Value {
    serde_json::to_value(Patient {
        names: vec![HumanName {
            family: Some(family.into()),
            given: vec!["Sam".into()],
        }],
        birth_date: Some("1975-06-01".into()),
        ..Default::default()
    })
    .unwrap()
}

fn letters() -> String {
    let mut n = nanos();
    let mut s = String::new();
    while n > 0 {
        s.push((b'a' + (n % 26) as u8) as char);
        n /= 26;
    }
    s
}

fn refusal(treatment: &str, state: &str) -> Value {
    let mut r = json!({ "kind": "spoken-now", "treatment": treatment, "if_life_at_risk": "not-stated",
        "form": { "written": "no", "signed": "no", "witnessed": "no", "seen": "not-seen" },
        "said_by": "patient", "identity": "certain", "state": state });
    if state == "doubted" {
        r["doubt"] = json!("said in pain");
    }
    r
}

async fn new_patient(server: &axa_test::TestServer, family: &str) -> String {
    let r = server
        .post("/api/patients")
        .json(&json!({ "patient": person(family), "override_reason": "test setup" }))
        .await;
    assert_eq!(r.status_code(), StatusCode::CREATED, "{}", r.text());
    r.json::<Value>()["id"].as_str().unwrap().to_string()
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_refusal_is_recorded_doubted_and_entered_in_error_and_none_recorded_is_not_none() {
    request_with::<App, _, _>(&settings(), |server, ctx| async move {
        let id = new_patient(&server, &format!("rf{}", letters())).await;
        let path = format!("/api/patients/{id}/treatment-refusals");

        let empty = server.get(&path).await.json::<Value>();
        assert_eq!(empty["summary"]["state"], "none-recorded");
        assert_eq!(empty["refusals"], json!([]));

        let made = server
            .post(&path)
            .json(&json!({ "refusal": refusal("blood products", "stands") }))
            .await;
        assert_eq!(made.status_code(), StatusCode::CREATED, "{}", made.text());
        let made = made.json::<Value>();
        let rid = made["id"].as_str().unwrap().to_string();
        assert_eq!(made["version"], 1);

        let listed = server.get(&path).await.json::<Value>();
        assert_eq!(listed["summary"]["state"], "recorded");
        assert_eq!(listed["summary"]["stands"], 1);
        assert_eq!(listed["refusals"][0]["from_merged_record"], false);

        // A blank treatment, and a doubt with no reason, are refused with their field named.
        for bad in [refusal(" ", "stands"), {
            let mut r = refusal("x", "stands");
            r["state"] = json!("doubted");
            r
        }] {
            let r = server.post(&path).json(&json!({ "refusal": bad })).await;
            assert_eq!(r.status_code(), StatusCode::UNPROCESSABLE_ENTITY, "{}", r.text());
        }
        // One is never recorded as entered in error to begin with.
        let r = server
            .post(&path)
            .json(&json!({ "refusal": refusal("x", "entered-in-error") }))
            .await;
        assert_eq!(r.status_code(), StatusCode::UNPROCESSABLE_ENTITY);

        // A doubt is a new version; a stale version is a 409.
        let one = format!("{path}/{rid}");
        let doubt = server
            .put(&one)
            .json(&json!({ "refusal": refusal("blood products", "doubted"), "expected_version": 1 }))
            .await;
        assert_eq!(doubt.status_code(), StatusCode::OK, "{}", doubt.text());
        assert_eq!(doubt.json::<Value>()["version"], 2);
        let stale = server
            .put(&one)
            .json(&json!({ "refusal": refusal("blood products", "doubted"), "expected_version": 1 }))
            .await;
        assert_eq!(stale.status_code(), StatusCode::CONFLICT);

        // A withdrawal that was heard is kept, as doubted. No rule is loaded that lets it end the refusal.
        let mut heard = refusal("blood products", "withdrawn");
        heard["withdrawal"] = json!({ "how": "spoken", "noticed_by": "the night nurse", "at": "2026-01-01T10:00:00Z" });
        let r = server
            .put(&one)
            .json(&json!({ "refusal": heard, "expected_version": 2 }))
            .await;
        assert_eq!(r.status_code(), StatusCode::UNPROCESSABLE_ENTITY, "{}", r.text());
        heard["state"] = json!("doubted");
        heard["doubt"] = json!("said to have changed their mind");
        let r = server
            .put(&one)
            .json(&json!({ "refusal": heard, "expected_version": 2 }))
            .await;
        assert_eq!(r.status_code(), StatusCode::OK, "{}", r.text());

        // Entered in error is a state; after it nothing changes and the list says none recorded.
        let mut mistaken = refusal("blood products", "entered-in-error");
        mistaken["doubt"] = json!(null);
        let r = server
            .put(&one)
            .json(&json!({ "refusal": mistaken, "expected_version": 3 }))
            .await;
        assert_eq!(r.status_code(), StatusCode::OK, "{}", r.text());
        let r = server
            .put(&one)
            .json(&json!({ "refusal": refusal("blood products", "stands"), "expected_version": 4 }))
            .await;
        assert_eq!(r.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        let after = server.get(&path).await.json::<Value>();
        assert_eq!(after["summary"]["state"], "none-recorded");

        // An unknown patient and an unknown refusal are 404, not an empty list.
        let r = server
            .get("/api/patients/00000000-0000-4000-8000-000000000000/treatment-refusals")
            .await;
        assert_eq!(r.status_code(), StatusCode::NOT_FOUND);
        let r = server
            .put(&format!("{path}/00000000-0000-4000-8000-000000000000"))
            .json(&json!({ "refusal": refusal("x", "stands"), "expected_version": 1 }))
            .await;
        assert_eq!(r.status_code(), StatusCode::NOT_FOUND);

        // What was refused is in no audit row for this patient.
        let rows = sqlx::query("SELECT detail::text AS d FROM audit_events WHERE patient_ref = $1 AND detail IS NOT NULL")
            .bind(format!("Patient/{id}"))
            .fetch_all(&ctx.pool())
            .await
            .unwrap();
        assert!(!rows.is_empty());
        for row in rows {
            let d: String = row.get("d");
            assert!(!d.contains("blood"), "an audit row holds what was refused: {d}");
        }
    })
    .await;
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_merge_and_an_unmerge_stop_for_each_refusal_until_a_person_names_it() {
    request_with::<App, _, _>(&settings(), |server, _ctx| async move {
        let family = format!("rm{}", letters());
        let (target, source) = (
            new_patient(&server, &family).await,
            new_patient(&server, &family).await,
        );
        let on_source = server
            .post(&format!("/api/patients/{source}/treatment-refusals"))
            .json(&json!({ "refusal": refusal("blood products", "stands") }))
            .await
            .json::<Value>();
        let on_source_id = on_source["id"].as_str().unwrap().to_string();

        // One that is entered in error asks nothing; the one that stands stops the merge.
        let merge = format!("/api/patients/{target}/merge");
        let stopped = server
            .post(&merge)
            .json(&json!({ "source": source, "reason": "same person" }))
            .await;
        assert_eq!(stopped.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        let body = stopped.json::<Value>();
        assert_eq!(body["refusal"], "refusals_not_seen");
        assert_eq!(body["refusals_unseen"], json!([on_source_id]));
        assert!(!body.to_string().contains("blood"), "the refusal leaked into its stop");

        // Naming a refusal that is not there does not satisfy it.
        let still = server
            .post(&merge)
            .json(&json!({ "source": source, "reason": "same person",
                "refusals_seen": ["00000000-0000-4000-8000-000000000000"] }))
            .await;
        assert_eq!(still.status_code(), StatusCode::UNPROCESSABLE_ENTITY);

        let merged = server
            .post(&merge)
            .json(&json!({ "source": source, "reason": "same person", "refusals_seen": [on_source_id] }))
            .await;
        assert_eq!(merged.status_code(), StatusCode::CREATED, "{}", merged.text());

        // The survivor shows the other record's refusal, as another record's.
        let shown = server
            .get(&format!("/api/patients/{target}/treatment-refusals"))
            .await
            .json::<Value>();
        assert_eq!(shown["summary"]["state"], "recorded");
        assert_eq!(shown["refusals"][0]["from_merged_record"], true);

        // A refusal recorded on the survivor since the merge may be the other person's: the unmerge asks.
        let since = server
            .post(&format!("/api/patients/{target}/treatment-refusals"))
            .json(&json!({ "refusal": refusal("a blood transfusion", "stands") }))
            .await
            .json::<Value>();
        let since_id = since["id"].as_str().unwrap().to_string();
        let unmerge = format!("/api/patients/{target}/unmerge");
        let stopped = server
            .post(&unmerge)
            .json(&json!({ "source": source, "reason": "mistake" }))
            .await;
        assert_eq!(stopped.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(stopped.json::<Value>()["refusals_unseen"], json!([since_id]));
        let done = server
            .post(&unmerge)
            .json(&json!({ "source": source, "reason": "mistake", "refusals_seen": [since_id] }))
            .await;
        assert_eq!(done.status_code(), StatusCode::OK, "{}", done.text());
    })
    .await;
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn only_those_who_treat_read_one_and_only_a_clinician_marks_one_entered_in_error() {
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
    let sub = |n: u8| format!("0c4f1e2a-0000-4000-8000-00000000f2{n:02x}");
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
                    .add_header(
                        HeaderName::from_static("x-purpose-of-use"),
                        HeaderValue::from_static("treatment"),
                    )
                    .await;
                (r.status_code(), r.text())
            }
        };
        let (status, text) = call(
            "POST",
            &admin,
            "/api/patients".into(),
            json!({ "patient": person(&format!("rr{}", letters())), "override_reason": "test setup" }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{text}");
        let pid = serde_json::from_str::<Value>(&text).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();
        // The doctor and the nurse are on the team; the pharmacist is not.
        for n in 1..=2 {
            let (status, text) = call("POST", &admin, format!("/api/patients/{pid}/relationships"),
                json!({ "relationship": { "subject": sub(n), "kind": "care-team", "status": "active" }, "reason": "test" })).await;
            assert_eq!(status, StatusCode::CREATED, "{text}");
        }
        let list = format!("/api/patients/{pid}/treatment-refusals");

        // A nurse records one, as a refusal is said to whoever is there.
        let (status, text) = call("POST", &nurse, list.clone(), json!({ "refusal": refusal("blood products", "stands") })).await;
        assert_eq!(status, StatusCode::CREATED, "{text}");
        let rid = serde_json::from_str::<Value>(&text).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();
        let one = format!("{list}/{rid}");

        // An administrator, who has no care relationship, and a pharmacist off the team read none.
        assert_eq!(call("GET", &admin, list.clone(), Value::Null).await.0, StatusCode::FORBIDDEN);
        assert_eq!(call("GET", &pharmacist, list.clone(), Value::Null).await.0, StatusCode::FORBIDDEN);
        assert_eq!(call("GET", &nurse, list.clone(), Value::Null).await.0, StatusCode::OK);

        // A nurse may doubt it, and may not mark it entered in error; a clinician may.
        let mistaken = json!({ "refusal": refusal("blood products", "entered-in-error"), "expected_version": 1 });
        assert_eq!(call("PUT", &nurse, one.clone(), mistaken.clone()).await.0, StatusCode::FORBIDDEN);
        let (status, text) = call("PUT", &doctor, one.clone(), mistaken).await;
        assert_eq!(status, StatusCode::OK, "{text}");
    })
    .await;
}
