//! DB-gated, through the real router with login off (RF-5): a refusal of treatment is recorded, doubted and entered in
//! error as new versions; a withdrawal that was heard is never stored as ending one (no rule is loaded); "none recorded"
//! is what an empty list says; the audit trail holds no word of what was refused; and a merge or an unmerge stops for each
//! refusal until a person has named it.
//!
//! ```text
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test main http_treatment_refusals:: -- --ignored --test-threads=1
//! ```

use axa_test::prelude::request_with;
use axum::http::StatusCode;
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
