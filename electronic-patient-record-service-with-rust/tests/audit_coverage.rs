//! DB-gated, through the real router with login off (OS-9): a successful read of a patient's
//! record leaves at least one audit row. `authz::authorize_mw` audits only refusals, so this
//! walks the policy table and holds each patient-scoped GET to it.
//!
//! Covered: every `GET` in `routes::TABLE` whose path is `/api/patients/{id}...` or
//! `/fhir/{R4,R5}/Patient/{id}`, `/fhir/R4|R5/AllergyIntolerance` and `/Observation` (with a
//! `patient` search). Not covered: writes, routes with an id other than the patient's, the
//! audit, feed, break-glass and hazard routes (they read the system, not a record), `/api/me`.
//! A route needing a fixture this test does not make is listed in `SKIPPED` with the reason (none now);
//! the test fails on a patient-scoped GET that is neither exercised nor listed.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test audit_coverage -- --ignored --test-threads=1
//! ```

use axum::http::StatusCode;
use electronic_patient_record::app::App;
use electronic_patient_record::routes;
use electronic_patient_record_matcher::{HumanName, Patient};
use loco_rs::testing::prelude::request;
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};

/// Patient-scoped GETs this test does not call, with the reason.
const SKIPPED: &[(&str, &str)] = &[];

const PNG: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 1, 2, 3];

fn letters(label: &str) -> String {
    let mut n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mut s = String::new();
    while n > 0 {
        s.push((b'a' + (n % 26) as u8) as char);
        n /= 26;
    }
    format!("{label}{s}")
}

async fn audit_rows(db: &DatabaseConnection) -> i64 {
    let row = db
        .query_one_raw(Statement::from_string(
            db.get_database_backend(),
            "SELECT count(*)::bigint AS n FROM audit_events_all".to_string(),
        ))
        .await
        .expect("count")
        .expect("one row");
    row.try_get("", "n").expect("n")
}

fn patient_scoped(template: &str) -> bool {
    template.starts_with("/api/patients/{id}")
        || template.starts_with("/fhir/R4/Patient/{id}")
        || template.starts_with("/fhir/R5/Patient/{id}")
        || template.starts_with("/fhir/R4/AllergyIntolerance")
        || template.starts_with("/fhir/R5/AllergyIntolerance")
        || template.starts_with("/fhir/R4/Observation")
        || template.starts_with("/fhir/R5/Observation")
        || template.starts_with("/fhir/R4/Flag")
        || template.starts_with("/fhir/R5/Flag")
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_successful_read_of_a_patient_record_leaves_an_audit_row() {
    // SAFETY: set before the app boots; the suite runs single-threaded.
    unsafe { std::env::set_var("EPR_REQUIRE_AUTH", "false") };
    request::<App, _, _>(|server, ctx| async move {
        let family = letters("cov");
        let patient = serde_json::to_value(Patient {
            names: vec![HumanName {
                family: Some(family.clone()),
                given: vec![family.clone()],
            }],
            birth_date: Some("1931-02-03".into()),
            ..Default::default()
        })
        .unwrap();
        let created = server
            .post("/api/patients")
            .json(&serde_json::json!({ "patient": patient, "override_reason": "coverage test" }))
            .await;
        assert_eq!(
            created.status_code(),
            StatusCode::CREATED,
            "{}",
            created.text()
        );
        let id = created.json::<serde_json::Value>()["id"]
            .as_str()
            .expect("id")
            .to_string();

        let stored = server
            .put(&format!("/api/patients/{id}/photo?expected_version=0"))
            .add_header(
                axum::http::HeaderName::from_static("content-type"),
                axum::http::HeaderValue::from_static("image/png"),
            )
            .bytes(PNG.to_vec().into())
            .await;
        assert!(stored.status_code().is_success(), "{}", stored.text());
        let made = server
            .post(&format!("/api/patients/{id}/observations"))
            .json(&serde_json::json!({ "observation": {
                "code": "8867-4", "value": 72.0, "unit": "/min",
                "range": { "low": 60.0, "high": 100.0 },
                "effective_at": "2026-10-26T09:30:00Z", "status": "final" }, "reason": "entered late" }))
            .await;
        assert_eq!(made.status_code(), StatusCode::CREATED, "{}", made.text());
        let observation_id = made.json::<serde_json::Value>()["id"]
            .as_str()
            .expect("observation id")
            .to_string();
        let sent = server
            .post(&format!("/api/patients/{id}/referrals"))
            .json(&serde_json::json!({ "referral": {
                "recipient": "Cardiology outpatients", "reason": "chest pain",
                "question": "please assess", "priority": "routine" } }))
            .await;
        assert_eq!(sent.status_code(), StatusCode::CREATED, "{}", sent.text());
        let referral_id = sent.json::<serde_json::Value>()["id"]
            .as_str()
            .expect("referral id")
            .to_string();

        let mut unaudited = Vec::new();
        let mut seen = 0;
        for (method, template, _, _) in routes::TABLE {
            if *method != "GET" || !patient_scoped(template) {
                continue;
            }
            if SKIPPED.iter().any(|(t, _)| t == template) {
                continue;
            }
            let path = if template.contains("{id}") {
                template
                    .replace("{id}", &id)
                    .replace("{observation_id}", &observation_id)
                    .replace("{referral_id}", &referral_id)
            } else {
                format!("{template}?patient={id}")
            };
            assert!(
                !path.contains('{'),
                "{template}: a parameter this test cannot fill; fill it or list it in SKIPPED"
            );
            let before = audit_rows(&ctx.db).await;
            let response = server.get(&path).await;
            let after = audit_rows(&ctx.db).await;
            if !response.status_code().is_success() {
                panic!(
                    "{template}: expected success, got {}",
                    response.status_code()
                );
            }
            seen += 1;
            if after <= before {
                unaudited.push(*template);
            }
        }
        assert!(seen > 0, "the walk saw no routes");
        assert!(
            unaudited.is_empty(),
            "successful reads with no audit row: {unaudited:?}"
        );
    })
    .await;
}
