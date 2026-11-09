//! DB-gated, through the real router with login **on** and `EPR_REQUIRE_PERMISSION_BASIS=true` (WL-4):
//! a clinician's read of a patient's clinical content without a permission basis is refused and the
//! refusal audited; with `asked-agreed` or `could-not-ask:<reason>` it is answered and every row of it
//! carries the basis; an unknown basis is a 400, so typed words never reach the trail; and a read of
//! identity, which is not clinical, needs none. One test, because the settings are read once per process.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test http_permission_basis -- --ignored --test-threads=1
//! ```

use authentication_verifier::test_idp::{AUDIENCE, REALM, TestIdp};
use axa_test::prelude::request;
use axum::http::{HeaderName, HeaderValue, StatusCode};
use electronic_patient_record::app::App;
use electronic_patient_record::pool::Pool;
use electronic_patient_record_matcher::{HumanName, Patient};
use serde_json::{Value, json};
use sqlx::{AssertSqlSafe, Row};

fn nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

fn dob(seed: u128) -> String {
    let n = nanos() / 1000 + seed;
    format!(
        "{}-{:02}-{:02}",
        1900 + n % 100,
        1 + n / 100 % 12,
        1 + n / 1200 % 28
    )
}

fn bearer(token: &str) -> (HeaderName, HeaderValue) {
    (
        HeaderName::from_static("authorization"),
        HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
    )
}

fn basis(value: &'static str) -> (HeaderName, HeaderValue) {
    (
        HeaderName::from_static("x-permission-basis"),
        HeaderValue::from_static(value),
    )
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_clinical_read_needs_a_basis_and_every_row_keeps_it() {
    let idp = TestIdp::start().await;
    let roles = ["clinical-admin", "clinician"];
    let map: Value = roles
        .iter()
        .map(|r| ((*r).to_string(), json!({ "role": [r] })))
        .collect();
    // SAFETY: set before the app boots and before any other thread reads them; one test per process.
    unsafe {
        std::env::set_var("EPR_REQUIRE_AUTH", "true");
        std::env::set_var("EPR_REQUIRE_PERMISSION_BASIS", "true");
        std::env::set_var("EPR_KEYCLOAK_URL", &idp.base_url);
        std::env::set_var("EPR_KEYCLOAK_REALM", REALM);
        std::env::set_var("EPR_KEYCLOAK_AUDIENCES", AUDIENCE);
        std::env::set_var("EPR_KEYCLOAK_ROLE_MAP", map.to_string());
    }
    let token = |role: &str, sub: &str| {
        idp.token(|c| {
            c["realm_access"] = json!({ "roles": [role] });
            c["sub"] = json!(sub);
        })
    };
    const CLINICIAN: &str = "0c4f1e2a-0000-4000-8000-0000000000b1";
    let admin = token("clinical-admin", "0c4f1e2a-0000-4000-8000-0000000000b0");
    let clinician = token("clinician", CLINICIAN);

    request::<App, _, _>(|server, ctx| async move {
        let (ah, av) = bearer(&admin);
        let (ch, cv) = bearer(&clinician);
        let family = format!("perm{}", nanos());
        let made = server
            .post("/api/patients")
            .add_header(ah.clone(), av.clone())
            .json(&json!({ "patient": serde_json::to_value(Patient {
                names: vec![HumanName { family: Some(family.clone()), given: vec![family] }],
                birth_date: Some(dob(0)),
                ..Default::default()
            }).unwrap() }))
            .await;
        assert_eq!(made.status_code(), StatusCode::CREATED, "{}", made.text());
        let pid = made.json::<Value>()["id"].as_str().unwrap().to_string();
        let rel = server
            .post(&format!("/api/patients/{pid}/relationships"))
            .add_header(ah.clone(), av.clone())
            .json(&json!({ "relationship": { "subject": CLINICIAN, "kind": "care-team", "status": "active" },
                           "reason": "test" }))
            .await;
        assert_eq!(rel.status_code(), StatusCode::CREATED, "{}", rel.text());
        let allergies = format!("/api/patients/{pid}/allergies");

        // No basis: refused, and the refusal says why.
        let refused = server.get(&allergies).add_header(ch.clone(), cv.clone()).await;
        assert_eq!(refused.status_code(), StatusCode::FORBIDDEN, "{}", refused.text());
        assert_eq!(refused.json::<Value>()["permission_basis_required"], true);

        // An unknown basis, or a typed reason, is a 400 before anything is decided.
        let typed = server
            .get(&allergies)
            .add_header(ch.clone(), cv.clone())
            .add_header(basis("could-not-ask:he was asleep").0, basis("could-not-ask:he was asleep").1)
            .await;
        assert_eq!(typed.status_code(), StatusCode::BAD_REQUEST, "{}", typed.text());

        // Asked and agreed; could not ask, unconscious: both answered.
        for value in ["asked-agreed", "could-not-ask:unconscious"] {
            let (bh, bv) = basis(value);
            let read = server
                .get(&allergies)
                .add_header(ch.clone(), cv.clone())
                .add_header(bh, bv)
                .await;
            assert_eq!(read.status_code(), StatusCode::OK, "{value}: {}", read.text());
        }

        // Identity is not clinical content: no basis needed.
        let who = server
            .get(&format!("/api/patients/{pid}"))
            .add_header(ch.clone(), cv.clone())
            .await;
        assert_eq!(who.status_code(), StatusCode::OK, "{}", who.text());

        // The trail: one refusal naming the missing basis, and each read carrying its basis.
        let rows = sqlx::query(AssertSqlSafe(format!(
            "SELECT outcome, detail::text AS d FROM audit_events WHERE patient_ref = 'Patient/{pid}' AND agent = '{CLINICIAN}'"
        )))
        .fetch_all(&ctx.pool())
        .await
        .unwrap();
        let details: Vec<(i16, String)> = rows
            .iter()
            .map(|r| (r.try_get::<i16, _>("outcome").unwrap_or_default(), r.try_get("d").unwrap()))
            .collect();
        let all = format!("{details:?}");
        assert!(
            details.iter().any(|(o, d)| *o == 4 && d.contains("permission_basis_required")),
            "{all}"
        );
        assert!(details.iter().any(|(_, d)| d.contains("\"permission_basis\": \"asked-agreed\"")), "{all}");
        assert!(
            details
                .iter()
                .any(|(_, d)| d.contains("\"permission_basis\": \"could-not-ask:unconscious\"")),
            "{all}"
        );
        assert!(!all.contains("asleep"), "typed words never reach the trail: {all}");
    })
    .await;
}
