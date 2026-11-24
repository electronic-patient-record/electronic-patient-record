//! DB-gated, through the real router with login **on** (TL-3): a team's task list is only for those who hold the
//! team's role, and is audited; a person closes a task with a reason at the version read; the dry run shows what the
//! service's rules would make, drafts marked, and refuses a fact the event does not carry. One test, because the
//! token guard is configured once per process.
//!
//! ```text
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test suite http_tasks:: -- --ignored --test-threads=1
//! ```

use authentication_verifier::test_idp::{AUDIENCE, REALM, TestIdp};
use axa_test::prelude::request_with;
use axum::http::{HeaderName, HeaderValue, StatusCode};
use electronic_patient_record::app::App;
use electronic_patient_record::pool::Pool;
use electronic_patient_record::task_rules::{self, Event};
use electronic_patient_record::tasks_store::{self, Source};
use electronic_patient_record_matcher::{HumanName, Patient};
use serde_json::{Value, json};
use sqlx::Row;
use std::collections::{BTreeMap, BTreeSet};

fn stamp() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
        .to_string()
}

fn bearer(token: &str) -> (HeaderName, HeaderValue) {
    (
        HeaderName::from_static("authorization"),
        HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
    )
}

async fn audit_rows(db: &sqlx::PgPool) -> i64 {
    sqlx::query("SELECT count(*)::bigint AS n FROM audit_events_all WHERE resource_ref = 'Tasks'")
        .fetch_one(db)
        .await
        .unwrap()
        .get("n")
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_teams_tasks_are_for_its_role_closed_with_a_reason_and_the_dry_run_writes_nothing() {
    let idp = TestIdp::start().await;
    let roles = [
        "clinical-admin",
        "clinician",
        "nurse",
        "clinical-safety-officer",
        "auditor",
    ];
    let map: Value = roles
        .iter()
        .map(|r| ((*r).to_string(), json!({ "role": [r] })))
        .collect();
    // This app's own settings: no process environment is written, so the file can join `suite.rs`.
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
    let token = |role: &str, sub: &str| {
        idp.token(|c| {
            c["realm_access"] = json!({ "roles": [role] });
            c["sub"] = json!(sub);
        })
    };
    let admin = bearer(&token(
        "clinical-admin",
        "0c4f1e2a-0000-4000-8000-0000000007a1",
    ));
    let clinician = bearer(&token("clinician", "0c4f1e2a-0000-4000-8000-0000000007c1"));
    let nurse = bearer(&token("nurse", "0c4f1e2a-0000-4000-8000-0000000007e1"));
    let cso = bearer(&token(
        "clinical-safety-officer",
        "0c4f1e2a-0000-4000-8000-0000000007f1",
    ));
    let auditor = bearer(&token("auditor", "0c4f1e2a-0000-4000-8000-0000000007d1"));

    request_with::<App, _, _>(&settings, |server, ctx| async move {
        let db = &ctx.pool();
        let family = format!("tasks{}", stamp());
        let made = server
            .post("/api/patients")
            .add_header(admin.0.clone(), admin.1.clone())
            .json(&json!({ "patient": serde_json::to_value(Patient {
                names: vec![HumanName { family: Some(family.clone()), given: vec![family.clone()] }],
                birth_date: Some("1933-03-03".into()),
                ..Default::default()
            }).unwrap(), "override_reason": "task routes test" }))
            .await;
        assert_eq!(made.status_code(), StatusCode::CREATED, "{}", made.text());
        let pid = made.json::<Value>()["id"].as_str().unwrap().to_string();

        // A task for the clinician team, as an approved rule makes it.
        let rules = task_rules::parse(
            r#"
[[rule]]
id = "http-review"
version = 1
title = "Review the result"
approved_by = "A Clinician"
approved_on = "2026-11-18"
event = "result-filed"
[rule.task]
team = "clinician"
title = "Review the result"
due_working_days = 1
late_reminder = "skip"
"#,
            "test",
        )
        .unwrap();
        let mut conn = db.acquire().await.unwrap();
        let task = tasks_store::create_from_event(
            &mut conn,
            &rules,
            Event::ResultFiled,
            &BTreeMap::new(),
            &pid,
            &Source { kind: "result".into(), id: format!("r-{}", stamp()) },
            jiff::civil::date(2026, 11, 18),
            &BTreeSet::new(),
        )
        .await
        .unwrap()
        .remove(0);
        drop(conn);

        // The clinician lists the clinician team, and the list is audited with the count.
        let before = audit_rows(db).await;
        let listed = server
            .get("/api/tasks?team=clinician")
            .add_header(clinician.0.clone(), clinician.1.clone())
            .await;
        assert_eq!(listed.status_code(), StatusCode::OK, "{}", listed.text());
        let body = listed.json::<Value>();
        assert!(body["items"].as_array().unwrap().iter().any(|t| t["id"] == task.id.as_str()));
        assert_eq!(body["team"], "clinician");
        assert!(audit_rows(db).await > before, "the list is audited");

        // No team is a 422; another role's team, or a team that is no role, is a 403, audited.
        let none = server
            .get("/api/tasks")
            .add_header(clinician.0.clone(), clinician.1.clone())
            .await;
        assert_eq!(none.status_code(), StatusCode::UNPROCESSABLE_ENTITY, "{}", none.text());
        for (who, team) in [(&nurse, "clinician"), (&clinician, "nurse"), (&clinician, "team-x")] {
            let before = audit_rows(db).await;
            let r = server
                .get(&format!("/api/tasks?team={team}"))
                .add_header(who.0.clone(), who.1.clone())
                .await;
            assert_eq!(r.status_code(), StatusCode::FORBIDDEN, "{team}: {}", r.text());
            assert!(audit_rows(db).await > before, "a refused list is audited");
        }
        // The auditor has no task list at all.
        let r = server
            .get("/api/tasks?team=auditor")
            .add_header(auditor.0.clone(), auditor.1.clone())
            .await;
        assert_eq!(r.status_code(), StatusCode::FORBIDDEN, "{}", r.text());

        // Closing: a nurse may not close the clinician team's task; a blank reason is a 422; a stale version a 409.
        let close = |who: &(HeaderName, HeaderValue), body: Value| {
            server
                .post(&format!("/api/tasks/{}/close", task.id))
                .add_header(who.0.clone(), who.1.clone())
                .json(&body)
        };
        let r = close(&nurse, json!({"expected_version": 1, "status": "done", "reason": "seen"})).await;
        assert_eq!(r.status_code(), StatusCode::FORBIDDEN, "{}", r.text());
        let r = close(&clinician, json!({"expected_version": 1, "status": "done", "reason": "  "})).await;
        assert_eq!(r.status_code(), StatusCode::UNPROCESSABLE_ENTITY, "{}", r.text());
        let r = close(&clinician, json!({"expected_version": 2, "status": "done", "reason": "seen"})).await;
        assert_eq!(r.status_code(), StatusCode::CONFLICT, "{}", r.text());
        let r = close(&clinician, json!({"expected_version": 1, "status": "done", "reason": "reviewed, no action"})).await;
        assert_eq!(r.status_code(), StatusCode::OK, "{}", r.text());
        let closed = r.json::<Value>();
        assert_eq!((closed["status"].as_str(), closed["version"].as_i64()), (Some("done"), Some(2)));
        let r = close(&clinician, json!({"expected_version": 2, "status": "cancelled", "reason": "again"})).await;
        assert_eq!(r.status_code(), StatusCode::UNPROCESSABLE_ENTITY, "closed once: {}", r.text());
        let r = server
            .post("/api/tasks/not-a-task/close")
            .add_header(clinician.0.clone(), clinician.1.clone())
            .json(&json!({"expected_version": 1, "status": "done", "reason": "x"}))
            .await;
        assert_eq!(r.status_code(), StatusCode::NOT_FOUND, "{}", r.text());

        // The dry run: the Clinical Safety Officer sees the built-in draft for an abnormal result, marked not
        // approved, due on the site's working days; nothing is written but the audit row.
        let tasks_before: i64 = sqlx::query("SELECT count(*)::bigint AS n FROM task_versions")
            .fetch_one(db).await.unwrap().get("n");
        let r = server
            .post("/api/tasks/dry-run")
            .add_header(cso.0.clone(), cso.1.clone())
            .json(&json!({"event": "result-filed", "facts": {"abnormal": "true"}, "day": "2026-11-20"}))
            .await;
        assert_eq!(r.status_code(), StatusCode::OK, "{}", r.text());
        let answer = r.json::<Value>();
        let planned = answer["planned"].as_array().unwrap();
        let draft = planned.iter().find(|p| p["rule"] == "abnormal-result-review").expect("the built-in draft");
        assert_eq!(draft["approved"], false);
        assert_eq!(draft["due_on"], "2026-11-24", "Friday plus two working days");
        let tasks_after: i64 = sqlx::query("SELECT count(*)::bigint AS n FROM task_versions")
            .fetch_one(db).await.unwrap().get("n");
        assert_eq!(tasks_before, tasks_after, "a dry run makes no task");
        // A normal result matches no rule; a typo in a fact, a bad day, or an unknown event is refused.
        let r = server
            .post("/api/tasks/dry-run")
            .add_header(cso.0.clone(), cso.1.clone())
            .json(&json!({"event": "result-filed", "facts": {"abnormal": "false"}}))
            .await;
        assert_eq!(r.status_code(), StatusCode::OK, "{}", r.text());
        assert!(r.json::<Value>()["planned"].as_array().unwrap().is_empty());
        for bad in [
            json!({"event": "result-filed", "facts": {"abnromal": "true"}}),
            json!({"event": "result-filed", "day": "20/11/2026"}),
        ] {
            let r = server
                .post("/api/tasks/dry-run")
                .add_header(cso.0.clone(), cso.1.clone())
                .json(&bad)
                .await;
            assert_eq!(r.status_code(), StatusCode::UNPROCESSABLE_ENTITY, "{bad}: {}", r.text());
        }
        let r = server
            .post("/api/tasks/dry-run")
            .add_header(cso.0.clone(), cso.1.clone())
            .json(&json!({"event": "message-patient"}))
            .await;
        assert!(r.status_code().is_client_error(), "an unknown event: {}", r.text());
        // A nurse may not run the dry run; the Clinical Safety Officer has no team list.
        let r = server
            .post("/api/tasks/dry-run")
            .add_header(nurse.0.clone(), nurse.1.clone())
            .json(&json!({"event": "result-filed"}))
            .await;
        assert_eq!(r.status_code(), StatusCode::FORBIDDEN, "{}", r.text());
        let r = server
            .get("/api/tasks?team=clinical-safety-officer")
            .add_header(cso.0.clone(), cso.1.clone())
            .await;
        assert_eq!(r.status_code(), StatusCode::FORBIDDEN, "{}", r.text());
    })
    .await;
}
