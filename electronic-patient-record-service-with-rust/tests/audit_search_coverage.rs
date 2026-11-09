//! DB-gated, through the real router with login off (OF-2): a successful search or list leaves at
//! least one audit row. `authz::authorize_mw` audits only refusals, so this walks every route in
//! `routes::TABLE` whose policy action is `Search` and holds each to it.
//!
//! Covered: each `Search` route is called once and must answer success and add an audit row. A
//! `Search` route that is neither called here nor listed in `EXEMPT` with a reason fails the test,
//! so a new search route cannot be added unaudited. Not covered: that the row's detail is free of
//! the searched values (the patient search test in `src/controllers/patients.rs` and the audit
//! tests check the detail for their routes), the `Read` list routes (`/api/hazards`,
//! `/api/me/patients`), and what a row says about a search that returns nothing.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test audit_search_coverage -- --ignored --test-threads=1
//! ```

use axa::testing::prelude::request;
use axum::http::StatusCode;
use electronic_patient_record::app::App;
use electronic_patient_record::policy::Action;
use electronic_patient_record::pool::Pool;
use electronic_patient_record::routes;
use sqlx::{PgPool, Row};

/// Search routes this test does not call, with the reason.
const EXEMPT: &[(&str, &str)] = &[(
    "/api/patients/compare",
    "pure: it compares two patients the caller supplied and reads no stored record",
)];

/// The call for a search route: its path with a query that matches nothing in particular.
fn call(template: &str, patient: &str) -> Option<String> {
    Some(match template {
        "/api/patients/{id}/adjustments" => format!("/api/patients/{patient}/adjustments"),
        "/api/patients/{id}/results" => format!("/api/patients/{patient}/results"),
        "/api/patients/{id}/assessments" => format!("/api/patients/{patient}/assessments"),
        "/fhir/R4/Flag" => format!("/fhir/R4/Flag?patient={patient}"),
        "/fhir/R5/Flag" => format!("/fhir/R5/Flag?patient={patient}"),
        "/api/patients/{id}/death/assertions" => {
            format!("/api/patients/{patient}/death/assertions")
        }
        "/api/patients" => "/api/patients?family=nobody".into(),
        "/fhir/R4/Patient" => "/fhir/R4/Patient?family=nobody".into(),
        "/fhir/R5/Patient" => "/fhir/R5/Patient?family=nobody".into(),
        "/api/audit"
        | "/fhir/R4/AuditEvent"
        | "/api/feed/refusals"
        | "/api/feed/refusals/count"
        | "/api/death-reports"
        | "/api/death-reports/count"
        | "/api/overdue-reviews"
        | "/api/overdue-reviews/count"
        | "/api/referrals/worklist"
        | "/api/referrals/worklist/count"
        | "/api/referral-queue"
        | "/api/referral-queue/count"
        | "/api/break-glass/reviews"
        | "/api/break-glass/reviews/count" => template.into(),
        _ => return None,
    })
}

async fn audit_rows(db: &PgPool) -> i64 {
    let row = sqlx::query("SELECT count(*)::bigint AS n FROM audit_events_all")
        .fetch_optional(db)
        .await
        .expect("count")
        .expect("one row");
    row.try_get("n").expect("n")
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_successful_search_or_list_leaves_an_audit_row() {
    // SAFETY: set before the app boots; the suite runs single-threaded.
    unsafe { std::env::set_var("EPR_REQUIRE_AUTH", "false") };
    request::<App, _, _>(|server, ctx| async move {
        let mut unaudited = Vec::new();
        let mut unlisted = Vec::new();
        let mut seen = 0;
        let made = server
            .post("/api/patients")
            .json(
                &serde_json::json!({ "patient": { "names": [{ "family": "Searchcoverage" }],
                "birth_date": "1911-02-03" }, "override_reason": "search coverage" }),
            )
            .await;
        assert_eq!(made.status_code(), StatusCode::CREATED, "{}", made.text());
        let patient = made.json::<serde_json::Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();
        for (method, template, _, action) in routes::TABLE {
            if *method != "GET" && *method != "POST" || *action != Action::Search {
                continue;
            }
            if EXEMPT.iter().any(|(t, _)| t == template) {
                continue;
            }
            let Some(path) = call(template, &patient) else {
                unlisted.push(*template);
                continue;
            };
            let before = audit_rows(&ctx.pool()).await;
            let response = server.get(&path).await;
            let after = audit_rows(&ctx.pool()).await;
            assert_eq!(
                response.status_code(),
                StatusCode::OK,
                "{template}: expected 200, got {} {}",
                response.status_code(),
                response.text()
            );
            seen += 1;
            if after <= before {
                unaudited.push(*template);
            }
        }
        assert!(
            unlisted.is_empty(),
            "search routes this test neither calls nor exempts: {unlisted:?}"
        );
        assert!(seen >= 9, "only {seen} search routes were walked");
        assert!(
            unaudited.is_empty(),
            "successful searches with no audit row: {unaudited:?}"
        );
    })
    .await;
}
