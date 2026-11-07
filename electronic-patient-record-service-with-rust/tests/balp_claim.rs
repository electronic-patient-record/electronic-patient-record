//! DB-gated, through the real router (PX-4): a patient read and a registration carry the client's
//! address, the interaction and the server into their audit rows, and so claim a BALP profile, while a
//! search and a row from outside a request do not. A test binary of its own because the server address
//! is read once at boot. The events are checked against the profile in `fhir_validator.rs`.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test balp_claim -- --ignored --test-threads=1
//! ```

use axa::testing::prelude::request;
use axum::http::{HeaderName, HeaderValue, StatusCode};
use electronic_patient_record::app::App;
use electronic_patient_record::audit::AuditRow;
use electronic_patient_record::interop::fhir_audit::{self, Violation};
use electronic_patient_record::pool::Pool;
use sqlx::Row;

const SERVER: &str = "https://epr.example.org/fhir";

/// A family name and a birth date no earlier run shares, so the duplicate check finds nothing.
fn unique() -> (String, String) {
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros();
    let mut letters = String::new();
    let mut left = n;
    while left > 0 {
        letters.push(char::from(b'a' + (left % 26) as u8));
        left /= 26;
    }
    let date = format!(
        "{}-{:02}-{:02}",
        1900 + n % 100,
        1 + n / 100 % 12,
        1 + n / 1200 % 28
    );
    (format!("Balp{letters}"), date)
}

/// The newest row `filter` selects (bound to `value`), as the export would read it.
async fn row(db: &sqlx::PgPool, filter: &str, value: &str) -> AuditRow {
    let sql = format!(
        "SELECT id, action, outcome, agent, patient_ref, resource_ref, purpose_of_use, break_glass,
                network_address, detail
         FROM audit_events WHERE {filter} ORDER BY id DESC LIMIT 1"
    );
    let r = sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind(value)
        .fetch_one(db)
        .await
        .unwrap();
    AuditRow {
        id: r.get("id"),
        at: "2026-11-07T09:00:00.000000Z".into(),
        action: r.get::<String, _>("action").trim().to_string(),
        outcome: r.get("outcome"),
        agent: r.get("agent"),
        patient: r.get("patient_ref"),
        resource: r.get("resource_ref"),
        purpose: r.get("purpose_of_use"),
        break_glass: r.get("break_glass"),
        network_address: r.get("network_address"),
        detail: r.get("detail"),
        digest: None,
        prev_digest: None,
    }
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_registration_and_a_read_claim_their_profiles_and_a_search_does_not() {
    // SAFETY: set before the service boots, and this binary's only test reads it.
    unsafe {
        std::env::set_var("EPR_REQUIRE_AUTH", "false");
        std::env::set_var("EPR_AUDIT_SERVER_ADDRESS", SERVER);
    }
    request::<App, _, _>(|server, ctx| async move {
        let db = ctx.pool();
        let (family, birth_date) = unique();
        let created = server
            .post("/api/patients")
            .json(&serde_json::json!({ "patient": {
                "names": [{ "family": family, "given": ["Ann"] }],
                "birth_date": birth_date,
            }}))
            .await;
        assert_eq!(created.status_code(), StatusCode::CREATED);
        let id = created.json::<serde_json::Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();
        let reference = format!("Patient/{id}");
        // No proxy is trusted here, so what the caller says about its own address is ignored.
        let read = server
            .get(&format!("/api/patients/{id}"))
            .add_header(
                HeaderName::from_static("x-forwarded-for"),
                HeaderValue::from_static("203.0.113.66"),
            )
            .await;
        assert_eq!(read.status_code(), StatusCode::OK);

        for (action, interaction, profile) in [
            ("C", "create", "PatientCreate"),
            ("R", "read", "PatientRead"),
        ] {
            let filter = format!("action = '{action}' AND patient_ref = $1");
            let row = row(&db, &filter, &reference).await;
            let address = row.network_address.clone().expect("a client address");
            assert_ne!(address, "203.0.113.66", "a forged address was recorded");
            let detail = row.detail.clone().unwrap();
            assert_eq!(detail["interaction"], interaction);
            assert_eq!(detail["server"], SERVER);
            assert_eq!(fhir_audit::violations(&row), vec![], "{action}");
            let event = serde_json::to_value(fhir_audit::to_fhir(&row).unwrap()).unwrap();
            let claimed = event["meta"]["profile"][0].as_str().unwrap();
            assert!(claimed.ends_with(profile), "{claimed}");
        }

        // A search does not hold the query, so it says so and is still exported, with no claim.
        let found = server.get(&format!("/api/patients?family={family}")).await;
        assert_eq!(found.status_code(), StatusCode::OK);
        let filter = "detail->>'interaction' = $1 AND resource_ref = 'Patient'";
        let search = row(&db, filter, "search-type").await;
        assert!(fhir_audit::violations(&search).contains(&Violation::QueryNotHeld));
        let event = serde_json::to_value(fhir_audit::to_fhir(&search).unwrap()).unwrap();
        assert!(event.get("meta").is_none());
    })
    .await;
}
