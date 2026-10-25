//! DB-gated, through the real router (`controllers::all_routes()`, the route gate and the audit
//! chain, booted by `loco_rs::testing` with login off): the shapes of the duplicate refusal and
//! of the FHIR refusals (CQ-16). Not covered here: consent masking of candidates and the
//! break-glass flag (both are in `http_roles.rs`).
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test http_patients -- --ignored --test-threads=1
//! ```

use axum::http::{HeaderName, HeaderValue, StatusCode};
use electronic_patient_record::app::App;
use electronic_patient_record_matcher::{HumanName, Patient};
use loco_rs::testing::prelude::request;
use sea_orm::{ConnectionTrait, Statement};

/// A family name no earlier run shares: letters only, from the clock, so the matcher's name
/// similarity stays low against patients left in the database by other runs.
fn tag(label: &str) -> String {
    let mut n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mut letters = String::new();
    while n > 0 {
        letters.push((b'a' + (n % 26) as u8) as char);
        n /= 26;
    }
    format!("{label}{letters}")
}

/// A birth date no earlier run shares, so the duplicate check does not shortlist their patients.
fn dob(seed: u128) -> String {
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
        / 1000
        + seed;
    format!(
        "{}-{:02}-{:02}",
        1900 + n % 100,
        1 + n / 100 % 12,
        1 + n / 1200 % 28
    )
}

fn patient(family: &str, dob: &str) -> serde_json::Value {
    serde_json::to_value(Patient {
        names: vec![HumanName {
            family: Some(family.into()),
            given: vec![family.into()],
        }],
        birth_date: Some(dob.into()),
        ..Default::default()
    })
    .unwrap()
}

/// Login off, set before the app boots (the guard reads it once).
fn login_off() {
    // SAFETY: set before any thread of this test reads it, and the suite runs single-threaded.
    unsafe { std::env::set_var("EPR_REQUIRE_AUTH", "false") };
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_likely_duplicate_is_409_with_candidates_and_an_override_needs_a_reason() {
    login_off();
    request::<App, _, _>(|server, _ctx| async move {
        let family = tag("dup");
        let born = dob(0);
        let body = serde_json::json!({ "patient": patient(&family, &born) });
        let first = server.post("/api/patients").json(&body).await;
        assert_eq!(first.status_code(), StatusCode::CREATED, "{}", first.text());

        let again = server.post("/api/patients").json(&body).await;
        assert_eq!(again.status_code(), StatusCode::CONFLICT);
        let json: serde_json::Value = again.json();
        let candidates = json["candidates"]
            .as_array()
            .expect("candidates in the 409");
        assert!(!candidates.is_empty());
        assert!(candidates[0].get("score").is_some());

        let blank = serde_json::json!({
            "patient": patient(&family, &born), "override_reason": "  "
        });
        let refused = server.post("/api/patients").json(&blank).await;
        assert_eq!(refused.status_code(), StatusCode::CONFLICT);

        let with_reason = serde_json::json!({
            "patient": patient(&family, &born), "override_reason": "twins"
        });
        let ok = server.post("/api/patients").json(&with_reason).await;
        assert_eq!(ok.status_code(), StatusCode::CREATED, "{}", ok.text());
    })
    .await;
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn fhir_refusals_are_operation_outcomes_and_never_carry_candidates() {
    login_off();
    request::<App, _, _>(|server, _ctx| async move {
        let family = tag("fhir");
        let created = server
            .post("/api/patients")
            .json(&serde_json::json!({ "patient": patient(&family, &dob(777)) }))
            .await;
        assert_eq!(created.status_code(), StatusCode::CREATED);
        let id = created.json::<serde_json::Value>()["id"]
            .as_str()
            .expect("id")
            .to_string();

        // The read carries the site's time zone for the banner's age (RC-9): UTC unless `EPR_SITE_TIME_ZONE` is set.
        let view = server.get(&format!("/api/patients/{id}")).await;
        assert_eq!(view.status_code(), StatusCode::OK);
        assert_eq!(view.json::<serde_json::Value>()["site_time_zone"], "UTC");

        // Update without If-Match: 428 OperationOutcome.
        let read = server.get(&format!("/fhir/R4/Patient/{id}")).await;
        assert_eq!(read.status_code(), StatusCode::OK);
        assert_eq!(read.header("etag"), "W/\"1\"");
        let resource: serde_json::Value = read.json();
        let put = server
            .put(&format!("/fhir/R4/Patient/{id}"))
            .json(&resource)
            .await;
        assert_eq!(put.status_code(), StatusCode::PRECONDITION_REQUIRED);
        assert_eq!(
            put.json::<serde_json::Value>()["resourceType"],
            "OperationOutcome"
        );

        // A stale If-Match: 412.
        let stale = server
            .put(&format!("/fhir/R4/Patient/{id}"))
            .add_header(
                HeaderName::from_static("if-match"),
                HeaderValue::from_static("W/\"7\""),
            )
            .json(&resource)
            .await;
        assert_eq!(stale.status_code(), StatusCode::PRECONDITION_FAILED);

        // A search parameter the facade does not read is a 400 naming it, never a silently
        // unfiltered list (IO-1); result-shaping `_count` still passes.
        for url in [
            format!("/fhir/R4/Patient?family={family}&gender=female"),
            format!("/fhir/R4/AllergyIntolerance?patient={id}&category=food"),
            "/fhir/R4/AuditEvent?_id=1".to_string(),
        ] {
            let refused = server.get(&url).await;
            assert_eq!(refused.status_code(), StatusCode::BAD_REQUEST, "{url}");
            let body: serde_json::Value = refused.json();
            assert_eq!(body["resourceType"], "OperationOutcome", "{url}");
            assert!(body.to_string().contains("not supported"), "{url}: {body}");
        }
        let counted = server
            .get(&format!("/fhir/R4/Patient?family={family}&_count=5"))
            .await;
        assert_eq!(counted.status_code(), StatusCode::OK);

        // A duplicate through FHIR: 409, an OperationOutcome, no candidates.
        let mut dup = resource.clone();
        dup.as_object_mut().unwrap().remove("id");
        let conflict = server.post("/fhir/R4/Patient").json(&dup).await;
        assert_eq!(conflict.status_code(), StatusCode::CONFLICT);
        let outcome: serde_json::Value = conflict.json();
        assert_eq!(outcome["resourceType"], "OperationOutcome");
        assert!(!outcome.to_string().contains(&family));

        // An element the record cannot hold: 422, not stored with it dropped.
        let mut marital = dup;
        marital["maritalStatus"] = serde_json::json!({ "text": "married" });
        marital["name"][0]["family"] = serde_json::json!(tag("fhir2"));
        let unprocessable = server.post("/fhir/R4/Patient").json(&marital).await;
        assert_eq!(
            unprocessable.status_code(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
    })
    .await;
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn the_request_id_is_echoed_and_written_into_the_audit_row() {
    login_off();
    request::<App, _, _>(|server, ctx| async move {
        let family = tag("rid");
        let body = serde_json::json!({ "patient": patient(&family, &dob(31)) });
        // A caller's id is kept, minus characters outside `[\w-@]`. Unique per run: the database persists.
        let supplied = format!("trace-{}drop", tag("id"));
        let response = server
            .post("/api/patients")
            .add_header(
                HeaderName::from_static("x-request-id"),
                HeaderValue::from_str(&supplied.replace("drop", ";drop")).unwrap(),
            )
            .json(&body)
            .await;
        assert_eq!(response.status_code(), StatusCode::CREATED);
        assert_eq!(response.header("x-request-id"), supplied.as_str());
        let rows = ctx
            .db
            .query_all_raw(Statement::from_sql_and_values(
                ctx.db.get_database_backend(),
                "SELECT detail->>'request_id' AS rid FROM audit_events WHERE detail->>'request_id' = $1",
                [supplied.clone().into()],
            ))
            .await
            .unwrap();
        assert_eq!(rows.len(), 1, "the registration's audit row carries the id");

        // With none supplied the middleware makes one, and the row has the same.
        let made = server.post("/api/patients").json(&serde_json::json!({
            "patient": patient(&tag("rid2"), &dob(32))
        })).await;
        let id = made.header("x-request-id").to_str().unwrap().to_string();
        assert!(!id.is_empty());
        let rows = ctx
            .db
            .query_all_raw(Statement::from_sql_and_values(
                ctx.db.get_database_backend(),
                "SELECT 1 FROM audit_events WHERE detail->>'request_id' = $1",
                [id.into()],
            ))
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
    })
    .await;
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_drill_registration_is_kept_apart_from_real_patients() {
    login_off();
    request::<App, _, _>(|server, _ctx| async move {
        let family = tag("drill");
        let born = dob(7);
        let real = server
            .post("/api/patients")
            .json(&serde_json::json!({ "patient": patient(&family, &born) }))
            .await;
        assert_eq!(real.status_code(), StatusCode::CREATED, "{}", real.text());
        let real: serde_json::Value = real.json();
        assert_eq!(real["drill"], false);

        // The same person as a drill record is not a duplicate of the real one.
        let drill = server
            .post("/api/patients")
            .json(&serde_json::json!({ "patient": patient(&family, &born), "drill": true }))
            .await;
        assert_eq!(drill.status_code(), StatusCode::CREATED, "{}", drill.text());
        let drill: serde_json::Value = drill.json();
        assert_eq!(drill["drill"], true);

        let ids = |v: &serde_json::Value| -> Vec<String> {
            v["results"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| r["id"].as_str().unwrap().to_string())
                .collect()
        };
        let ordinary: serde_json::Value = server
            .get(&format!("/api/patients?family={family}"))
            .await
            .json();
        assert_eq!(ids(&ordinary), vec![real["id"].as_str().unwrap()]);
        let drills: serde_json::Value = server
            .get(&format!("/api/patients?family={family}&drill=true"))
            .await
            .json();
        assert_eq!(ids(&drills), vec![drill["id"].as_str().unwrap()]);

        // The FHIR facade lists real patients only, and a drill read says what it is.
        let bundle: serde_json::Value = server
            .get(&format!("/fhir/R4/Patient?family={family}"))
            .await
            .json();
        let in_bundle = bundle["entry"].as_array().map_or(0, Vec::len);
        assert_eq!(in_bundle, 1, "{bundle}");
        let read = server
            .get(&format!(
                "/fhir/R4/Patient/{}",
                drill["id"].as_str().unwrap()
            ))
            .await;
        let read: serde_json::Value = read.json();
        assert_eq!(read["meta"]["tag"][0]["code"], "drill");
    })
    .await;
}
