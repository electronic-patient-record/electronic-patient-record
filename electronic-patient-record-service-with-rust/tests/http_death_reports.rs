//! DB-gated, through the real router with login **on** (OQ-BV4, H-046): the list of reports of a death that
//! wait for a person. One test, because the token guard is configured once per process.
//!
//! Covered: a report appears, oldest first, as a pointer with no reason; clinician, nurse, administrator and
//! on-duty clinician read it and the count, which names the accountable role; every other staff role is refused
//! (403); a verified report and a withdrawn one leave the list; the on-duty clinician alone can verify and
//! withdraw; a nurse still cannot withdraw; the audit rows carry the count, never the reason.
//! Not covered: a report on a record merged away, a list that is cut short, and who holds the on-duty role, which
//! the service does not know.
//!
//! ```text
//! DB_URL=postgres://postgres:postgres@localhost:5543/electronic_patient_record_test \
//!   cargo test --test http_death_reports -- --ignored --test-threads=1
//! ```

use authentication_verifier::test_idp::{AUDIENCE, REALM, TestIdp};
use axa_test::prelude::request;
use axum::http::{HeaderName, HeaderValue, StatusCode};
use electronic_patient_record::app::App;
use electronic_patient_record::pool::Pool;
use serde_json::{Value, json};
use sqlx::Row;

fn nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

fn bearer(token: &str) -> (HeaderName, HeaderValue) {
    (
        HeaderName::from_static("authorization"),
        HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
    )
}

const SECRET: &str = "Secret-reason-kittiwake";

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn every_alerted_role_sees_what_waits_and_nobody_else_does() {
    let idp = TestIdp::start().await;
    let roles = [
        "clinical-admin",
        "clinician",
        "nurse",
        "on-duty-clinician",
        "receptionist",
        "pharmacist",
        "auditor",
    ];
    let map: Value = roles
        .iter()
        .map(|r| ((*r).to_string(), json!({ "role": [r] })))
        .collect();
    // SAFETY: set before the app boots and before any other thread reads them; one test per process.
    unsafe {
        std::env::set_var("EPR_REQUIRE_AUTH", "true");
        std::env::set_var("EPR_KEYCLOAK_URL", &idp.base_url);
        std::env::set_var("EPR_KEYCLOAK_REALM", REALM);
        std::env::set_var("EPR_KEYCLOAK_AUDIENCES", AUDIENCE);
        std::env::set_var("EPR_KEYCLOAK_ROLE_MAP", map.to_string());
    }
    let token = |role: &str, n: u8| {
        idp.token(|c| {
            c["realm_access"] = json!({ "roles": [role] });
            c["sub"] = json!(format!("0c4f1e2a-0000-4000-8000-0000000000{n:02}"));
        })
    };
    let admin = bearer(&token("clinical-admin", 10));
    let clinician = bearer(&token("clinician", 11));
    let nurse = bearer(&token("nurse", 12));
    let on_duty = bearer(&token("on-duty-clinician", 13));
    let refused = [
        bearer(&token("receptionist", 14)),
        bearer(&token("pharmacist", 15)),
        bearer(&token("auditor", 16)),
    ];

    request::<App, _, _>(|server, ctx| async move {
        let family = format!("deathreports{}", nanos());
        let made = server
            .post("/api/patients")
            .add_header(admin.0.clone(), admin.1.clone())
            .json(&json!({ "patient": {
                "names": [{ "family": family, "given": [family] }], "birth_date": "1931-02-03" } }))
            .await;
        assert_eq!(made.status_code(), StatusCode::CREATED, "{}", made.text());
        let pid = made.json::<Value>()["id"].as_str().unwrap().to_string();

        let report = || {
            let (server, pid, clinician) = (&server, pid.clone(), clinician.clone());
            async move {
                let r = server
                    .post(&format!("/api/patients/{pid}/death/assertions"))
                    .add_header(clinician.0, clinician.1)
                    .json(&json!({
                        "source": "family-member",
                        "occurred": { "kind": "not-known", "reason": "not given" },
                        "reason": SECRET }))
                    .await;
                assert_eq!(r.status_code(), StatusCode::CREATED, "{}", r.text());
                r.json::<Value>()["id"].as_str().unwrap().to_string()
            }
        };
        let first = report().await;
        let second = report().await;

        let mine = |list: &Value| -> Vec<String> {
            list["items"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|i| i["patient_id"] == pid.as_str())
                .map(|i| i["assertion_id"].as_str().unwrap().to_string())
                .collect()
        };
        for who in [&admin, &clinician, &nurse, &on_duty] {
            let list = server
                .get("/api/death-reports")
                .add_header(who.0.clone(), who.1.clone())
                .await;
            assert_eq!(list.status_code(), StatusCode::OK, "{}", list.text());
            assert!(!list.text().contains(SECRET));
            let body = list.json::<Value>();
            assert_eq!(body["accountable_role"], "on-duty-clinician");
            assert_eq!(mine(&body), vec![first.clone(), second.clone()]);
            let n = server
                .get("/api/death-reports/count")
                .add_header(who.0.clone(), who.1.clone())
                .await;
            assert_eq!(n.status_code(), StatusCode::OK, "{}", n.text());
            assert!(n.json::<Value>()["open"].as_i64().unwrap() >= 2);
        }
        for who in &refused {
            for path in ["/api/death-reports", "/api/death-reports/count"] {
                let r = server
                    .get(path)
                    .add_header(who.0.clone(), who.1.clone())
                    .await;
                assert_eq!(r.status_code(), StatusCode::FORBIDDEN, "{}", r.text());
            }
        }

        // The on-duty role alone can do the work it is accountable for.
        let step = |id: String, to: &'static str, who: (HeaderName, HeaderValue)| {
            let (server, pid) = (&server, pid.clone());
            async move {
                server
                    .put(&format!("/api/patients/{pid}/death/assertions/{id}"))
                    .add_header(who.0, who.1)
                    .json(&json!({ "expected_version": 1, "to": to, "reason": "checked" }))
                    .await
            }
        };
        let no = step(second.clone(), "withdrawn-in-error", nurse.clone()).await;
        assert_eq!(no.status_code(), StatusCode::FORBIDDEN, "{}", no.text());
        let yes = step(first.clone(), "verified", on_duty.clone()).await;
        assert_eq!(yes.status_code(), StatusCode::OK, "{}", yes.text());
        let list = server
            .get("/api/death-reports")
            .add_header(on_duty.0.clone(), on_duty.1.clone())
            .await
            .json::<Value>();
        assert_eq!(mine(&list), vec![second.clone()]);

        let out = step(second.clone(), "withdrawn-in-error", on_duty.clone()).await;
        assert_eq!(out.status_code(), StatusCode::OK, "{}", out.text());
        let list = server
            .get("/api/death-reports")
            .add_header(on_duty.0.clone(), on_duty.1.clone())
            .await
            .json::<Value>();
        assert!(mine(&list).is_empty());

        let rows = sqlx::query(
            "SELECT detail::text AS d FROM audit_events_all WHERE resource_ref = 'DeathReports'",
        )
        .fetch_all(&ctx.pool())
        .await
        .unwrap();
        assert!(!rows.is_empty());
        for row in rows {
            assert!(!row.get::<String, _>("d").contains(SECRET));
        }
    })
    .await;
}
