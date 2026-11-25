//! DB-gated, through the real router with login off (AG-6): opening a merged chart is recorded against **the chart that
//! was opened**, whichever record its first row came from, and the records that contributed are kept beside it for an
//! authorised investigation. The patient-facing log reads only the patient reference, so a list whose newest row came
//! from a merged-in record used to be missing from the survivor's log.
//!
//! ```text
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test main audit_attribution:: -- --ignored --test-threads=1
//! ```

use axa_test::prelude::request_with;
use axum::http::StatusCode;
use electronic_patient_record::app::App;
use electronic_patient_record::audit;
use electronic_patient_record::pool::Pool;
use electronic_patient_record_matcher::{HumanName, Patient};
use serde_json::{Value, json};
use sqlx::{PgPool, Row};

async fn newest(db: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT coalesce(max(id), 0)::bigint FROM audit_events")
        .fetch_one(db)
        .await
        .unwrap()
}

/// The rows after `since`: whom each is about, and its detail.
async fn since(db: &PgPool, since: i64) -> Vec<(Option<String>, Value)> {
    sqlx::query("SELECT patient_ref, detail FROM audit_events WHERE id > $1 ORDER BY id")
        .bind(since)
        .fetch_all(db)
        .await
        .unwrap()
        .iter()
        .map(|r| {
            (
                r.try_get("patient_ref").unwrap(),
                r.try_get::<Option<Value>, _>("detail")
                    .unwrap()
                    .unwrap_or(Value::Null),
            )
        })
        .collect()
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_merged_chart_is_audited_as_the_chart_that_was_opened() {
    let url = std::env::var("DB_URL").expect("DB_URL must be set for --ignored tests");
    let settings = [("DB_URL", url.as_str()), ("EPR_REQUIRE_AUTH", "false")];
    request_with::<App, _, _>(&settings, |server, ctx| async move {
        let server = &server;
        let db = ctx.pool();
        // Both orders: the merged-in record's reading is the newest, then the oldest.
        for (kept_at, gone_at) in [("09:00", "10:00"), ("10:00", "09:00")] {
            let family = format!(
                "attr{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            );
            let person = serde_json::to_value(Patient {
                names: vec![HumanName {
                    family: Some(family.clone()),
                    given: vec![family],
                }],
                birth_date: Some("1950-05-06".into()),
                ..Default::default()
            })
            .unwrap();
            let mut ids = Vec::new();
            for _ in 0..2 {
                let made = server
                    .post("/api/patients")
                    .json(&json!({ "patient": person, "override_reason": "attribution test" }))
                    .await;
                assert_eq!(made.status_code(), StatusCode::CREATED, "{}", made.text());
                ids.push(made.json::<Value>()["id"].as_str().unwrap().to_string());
            }
            let (kept, gone) = (ids[0].clone(), ids[1].clone());
            let mut recorded = std::collections::BTreeMap::new();
            for (id, at) in [(&kept, kept_at), (&gone, gone_at)] {
                for (path, body) in [
                    ("observations", json!({ "observation": { "code": "8867-4", "value": 72.0, "unit": "/min",
                        "effective_at": format!("2026-10-26T{at}:00Z"), "status": "final" }, "reason": "late" })),
                    ("allergies", json!({ "allergy": { "substance": "latex", "status": "active" } })),
                    ("results", json!({ "result": { "test": "Serum potassium", "status": "final",
                        "issued": "2026-10-26T09:00:00Z" } })),
                    ("problems", json!({ "record": { "title": "Asthma", "certainty": "confirmed", "status": "active" } })),
                    ("relative-history", json!({ "record": { "genetic": "full-sibling", "environment": "childhood",
                        "condition": "Asthma", "presence": "present", "source": "patient", "status": "recorded" } })),
                    ("adjustments", json!({ "code": "large-print", "reason": "they asked" })),
                ] {
                    let r = server.post(&format!("/api/patients/{id}/{path}")).json(&body).await;
                    assert_eq!(r.status_code(), StatusCode::CREATED, "{path}: {}", r.text());
                    recorded.insert((id.clone(), path), r.json::<Value>()["id"].as_str().unwrap().to_string());
                }
            }
            let merge = server
                .post(&format!("/api/patients/{kept}/merge"))
                .json(&json!({ "source": gone, "reason": "one person" }))
                .await;
            assert!(merge.status_code().is_success(), "{}", merge.text());

            let opened = format!("Patient/{kept}");
            for path in [
                "observations", "allergies", "results", "problems", "relative-history", "links", "amendments",
                "assessments", "adjustments",
            ] {
                let before = newest(&db).await;
                let r = server.get(&format!("/api/patients/{kept}/{path}")).await;
                assert_eq!(r.status_code(), StatusCode::OK, "{path}: {}", r.text());
                let rows = since(&db, before).await;
                assert!(!rows.is_empty(), "{path}");
                for (about, detail) in &rows {
                    assert_eq!(about.as_deref(), Some(opened.as_str()), "{path} {kept_at}: {detail}");
                }
                // The record that contributed is kept, where one did.
                if recorded.contains_key(&(gone.clone(), path)) {
                    assert_eq!(rows[0].1["records"], json!([gone]), "{path}: {}", rows[0].1);
                }
            }
            // One entry's history, and a change to an entry that was written on the merged-in record.
            let theirs = &recorded[&(gone.clone(), "observations")];
            let before = newest(&db).await;
            let versions = server
                .get(&format!("/api/patients/{kept}/observations/{theirs}/versions"))
                .await;
            assert_eq!(versions.status_code(), StatusCode::OK);
            for (path, body) in [
                ("observations", json!({ "observation": { "code": "8867-4", "value": 73.0, "unit": "/min",
                    "effective_at": format!("2026-10-26T{gone_at}:00Z"), "status": "final" }, "expected_version": 1, "reason": "r" })),
                ("allergies", json!({ "allergy": { "substance": "latex", "status": "inactive" }, "expected_version": 1 })),
                ("results", json!({ "result": { "test": "Serum potassium", "status": "corrected",
                    "issued": "2026-10-26T09:00:00Z" }, "expected_version": 1, "reason": "r" })),
                ("problems", json!({ "record": { "title": "Asthma", "certainty": "confirmed", "status": "resolved" },
                    "expected_version": 1, "reason": "r" })),
            ] {
                let id = &recorded[&(gone.clone(), path)];
                let r = server.put(&format!("/api/patients/{kept}/{path}/{id}")).json(&body).await;
                assert_eq!(r.status_code(), StatusCode::OK, "{path}: {}", r.text());
            }
            let rows = since(&db, before).await;
            assert_eq!(rows.len(), 5);
            for (about, detail) in &rows {
                assert_eq!(about.as_deref(), Some(opened.as_str()), "{detail}");
                assert_eq!(detail["records"], json!([gone]), "{detail}");
            }
            // The survivor's own log has the opening of the list, and shows nothing of the records behind it.
            let log = audit::patient_log(&db, &kept, "nobody", &audit::PatientLogFilter::default(), 100)
                .await
                .unwrap();
            assert!(log.iter().filter(|r| r.resource.as_deref() == Some("Observation")).count() >= 3);

            // After an unmerge: the opening of the survivor's chart stays in its log, and what happens next on the other
            // record is not in it.
            let survivor_rows = log.len();
            let unmerge = server
                .post(&format!("/api/patients/{kept}/unmerge"))
                .json(&json!({ "source": gone, "reason": "two people" }))
                .await;
            assert!(unmerge.status_code().is_success(), "{}", unmerge.text());
            let before = newest(&db).await;
            server.get(&format!("/api/patients/{gone}/observations")).await;
            server.get(&format!("/api/patients/{kept}/observations")).await;
            let rows = since(&db, before).await;
            assert_eq!(rows[0].0.as_deref(), Some(format!("Patient/{gone}").as_str()));
            assert_eq!(rows[1].0.as_deref(), Some(opened.as_str()));
            assert!(rows.iter().all(|(_, d)| d.get("records").is_none()), "{rows:?}");
            let log = audit::patient_log(&db, &kept, "nobody", &audit::PatientLogFilter::default(), 100)
                .await
                .unwrap();
            assert!(log.len() > survivor_rows, "kept, and added to");
        }
        // An empty list is an access too.
        let r = server
            .post("/api/patients")
            .json(&json!({ "patient": { "names": [{ "family": "Empty", "given": ["E"] }], "birth_date": "1951-01-01" },
                           "override_reason": "attribution test" }))
            .await;
        let id = r.json::<Value>()["id"].as_str().unwrap().to_string();
        let before = newest(&db).await;
        server.get(&format!("/api/patients/{id}/observations")).await;
        let rows = since(&db, before).await;
        assert_eq!(rows[0].0.as_deref(), Some(format!("Patient/{id}").as_str()));
    })
    .await;
}
