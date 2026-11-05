//! DB-gated, through the real router with login **on** (PA-6): anyone who meets patients may read and
//! record an adjustment, a patient, a proxy and an auditor have none of it, the author is the token, a
//! stale version is 409, ending is final and stays in the history, an overdue review shows, the audit
//! trail holds no code, note or reason, and the table refuses change. One test, because the token guard
//! is configured once per process.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test http_adjustments -- --ignored --test-threads=1
//! ```

use authentication_verifier::test_idp::{AUDIENCE, REALM, TestIdp};
use axum::http::{HeaderName, HeaderValue, StatusCode};
use electronic_patient_record::app::App;
use electronic_patient_record_matcher::{HumanName, Patient};
use loco_rs::testing::prelude::request;
use sea_orm::{ConnectionTrait, Statement};
use serde_json::{Value, json};

fn nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

fn dob() -> String {
    let n = nanos() / 1000;
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

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn who_may_record_review_and_end_an_adjustment() {
    let idp = TestIdp::start().await;
    let roles = ["nurse", "receptionist", "patient", "auditor", "proxy"];
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
    let token = |role: &str, sub: &str| {
        idp.token(|c| {
            c["realm_access"] = json!({ "roles": [role] });
            c["sub"] = json!(sub);
        })
    };
    let nurse = token("nurse", "0c4f1e2a-0000-4000-8000-0000000000e1");
    const RECEPTION: &str = "0c4f1e2a-0000-4000-8000-0000000000e2";
    let reception = token("receptionist", RECEPTION);
    let patient = token("patient", "0c4f1e2a-0000-4000-8000-0000000000e3");
    let auditor = token("auditor", "0c4f1e2a-0000-4000-8000-0000000000e4");
    let proxy = token("proxy", "0c4f1e2a-0000-4000-8000-0000000000e5");

    request::<App, _, _>(|server, ctx| async move {
        let purpose = HeaderName::from_static("x-purpose-of-use");
        let ops = HeaderValue::from_static("operations");

        let (h, v) = bearer(&reception);
        let made = server
            .post("/api/patients")
            .add_header(h, v)
            .add_header(purpose.clone(), ops.clone())
            .json(&json!({ "patient": Patient {
                names: vec![HumanName {
                    family: Some("Adjusttest".into()),
                    given: vec![format!("N{}", nanos())],
                }],
                birth_date: Some(dob()),
                ..Patient::default()
            }, "override_reason": "adjustment route test" }))
            .await;
        assert_eq!(made.status_code(), StatusCode::CREATED, "{}", made.text());
        let id = made.json::<Value>()["id"].as_str().unwrap().to_string();
        let path = format!("/api/patients/{id}/adjustments");

        let get = |t: String| {
            let (h, v) = bearer(&t);
            server
                .get(&path)
                .add_header(h, v)
                .add_header(purpose.clone(), ops.clone())
        };
        let post = |t: String, body: Value| {
            let (h, v) = bearer(&t);
            server
                .post(&path)
                .add_header(h, v)
                .add_header(purpose.clone(), ops.clone())
                .json(&body)
        };

        // Nothing yet, and the people with no route are refused.
        let empty = get(reception.clone()).await;
        assert_eq!(empty.status_code(), StatusCode::OK, "{}", empty.text());
        assert_eq!(empty.json::<Value>()["adjustments"], json!([]));
        for who in [&patient, &auditor, &proxy] {
            let refused = get(who.clone()).await;
            assert_eq!(
                refused.status_code(),
                StatusCode::FORBIDDEN,
                "{}",
                refused.text()
            );
            let refused = post(who.clone(), json!({ "code": "braille", "reason": "x" })).await;
            assert_eq!(
                refused.status_code(),
                StatusCode::FORBIDDEN,
                "{}",
                refused.text()
            );
        }

        // Reception records one. The author is the token, not the body.
        let body = json!({ "code": "interpreter-spoken", "note": "Polish",
            "review_by": "2099-01-01", "reason": "the daughter telephoned" });
        let made = post(reception.clone(), body.clone()).await;
        assert_eq!(made.status_code(), StatusCode::CREATED, "{}", made.text());
        let made: Value = made.json();
        assert_eq!(made["author"], RECEPTION);
        assert_eq!(made["display"], "Spoken language interpreter");
        assert_eq!(made["category"], "interpreting");
        assert_eq!(made["overdue"], false);
        let one = format!("{path}/{}", made["id"].as_str().unwrap());

        // The same need twice is refused, a second language is not, and the rules reach the route.
        let twice = post(reception.clone(), body).await;
        assert_eq!(
            twice.status_code(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            twice.text()
        );
        let welsh = post(
            reception.clone(),
            json!({ "code": "interpreter-spoken", "note": "Welsh", "reason": "they asked" }),
        )
        .await;
        assert_eq!(welsh.status_code(), StatusCode::CREATED, "{}", welsh.text());
        for bad in [
            json!({ "code": "interpreter-spoken", "reason": "no language" }),
            json!({ "code": "not-a-code", "reason": "unknown" }),
            json!({ "code": "braille", "reason": "  " }),
            json!({ "code": "braille", "review_by": "2001-01-01", "reason": "past" }),
        ] {
            let refused = post(nurse.clone(), bad).await;
            assert_eq!(
                refused.status_code(),
                StatusCode::UNPROCESSABLE_ENTITY,
                "{}",
                refused.text()
            );
        }

        let put = |t: String, body: Value| {
            let (h, v) = bearer(&t);
            server
                .put(&one)
                .add_header(h, v)
                .add_header(purpose.clone(), ops.clone())
                .json(&body)
        };

        // A nurse reviews it; a stale version is a conflict.
        let reviewed = put(
            nurse.clone(),
            json!({ "expected_version": 1, "action": "review",
                "review_by": "2100-01-01", "reason": "checked with them" }),
        )
        .await;
        assert_eq!(
            reviewed.status_code(),
            StatusCode::OK,
            "{}",
            reviewed.text()
        );
        let reviewed: Value = reviewed.json();
        assert_eq!(reviewed["version"], 2);
        assert_eq!(reviewed["adjustment"]["review_by"], "2100-01-01");
        assert_eq!(
            reviewed["adjustment"]["note"], "Polish",
            "a review keeps the note"
        );
        let stale = put(
            nurse.clone(),
            json!({ "expected_version": 1, "action": "end", "reason": "stale" }),
        )
        .await;
        assert_eq!(
            stale.status_code(),
            StatusCode::CONFLICT,
            "{}",
            stale.text()
        );

        // Ending is final, and it stays in the list.
        let ended = put(
            nurse.clone(),
            json!({ "expected_version": 2, "action": "end", "reason": "no longer needed" }),
        )
        .await;
        assert_eq!(ended.status_code(), StatusCode::OK, "{}", ended.text());
        let again = put(
            nurse.clone(),
            json!({ "expected_version": 3, "action": "review", "reason": "too late" }),
        )
        .await;
        assert_eq!(
            again.status_code(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            again.text()
        );
        let listed: Value = get(nurse.clone()).await.json();
        let rows = listed["adjustments"].as_array().unwrap();
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().any(|r| r["adjustment"]["status"] == "ended"));

        // A review date that has passed shows as overdue. The route refuses to write one, so the test
        // puts it in the table, as time would.
        ctx.db
            .execute_raw(Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                "INSERT INTO adjustment_versions
                   (adjustment_id, version, patient_id, author, status, body)
                 VALUES (gen_random_uuid(), 1, $1::uuid, 'test', 'active', $2)",
                vec![
                    id.clone().into(),
                    json!({ "code": "braille", "note": null, "status": "active",
                        "review_by": "2020-01-01", "reason": "set long ago" })
                    .into(),
                ],
            ))
            .await
            .unwrap();
        let listed: Value = get(nurse.clone()).await.json();
        let old = listed["adjustments"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["adjustment"]["code"] == "braille")
            .unwrap();
        assert_eq!(old["overdue"], true);

        // The same list as FHIR, in both releases, with the same refusals.
        let fhir = |t: String, base: &str| {
            let (h, v) = bearer(&t);
            server
                .get(&format!("/fhir/{base}/Flag?patient={id}"))
                .add_header(h, v)
                .add_header(purpose.clone(), ops.clone())
        };
        for base in ["R4", "R5"] {
            let got = fhir(nurse.clone(), base).await;
            assert_eq!(got.status_code(), StatusCode::OK, "{}", got.text());
            let got: Value = got.json();
            let flags = got["entry"].as_array().unwrap();
            assert_eq!(flags.len(), 3, "{got}");
            assert!(flags.iter().any(|e| e["resource"]["status"] == "inactive"));
            assert!(!got.to_string().contains("daughter"), "a reason went out");
            let refused = fhir(patient.clone(), base).await;
            assert_eq!(
                refused.status_code(),
                StatusCode::FORBIDDEN,
                "{}",
                refused.text()
            );
        }

        // The audit trail says something happened, and nothing of what the person needs.
        let audited = ctx
            .db
            .query_all_raw(Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                "SELECT detail::text AS detail FROM audit_events_all
                 WHERE resource_ref = 'Adjustment' AND patient_ref = $1",
                vec![format!("Patient/{id}").into()],
            ))
            .await
            .unwrap();
        assert!(audited.len() >= 5, "only {} audit rows", audited.len());
        for row in audited {
            let detail: String = row.try_get("", "detail").unwrap();
            for secret in ["Polish", "interpreter", "daughter", "no longer needed"] {
                assert!(!detail.contains(secret), "{secret} in {detail}");
            }
        }

        // The table refuses change.
        let change = ctx
            .db
            .execute_unprepared(&format!(
                "UPDATE adjustment_versions SET author = 'x' WHERE patient_id = '{id}'"
            ))
            .await;
        assert!(change.is_err(), "an adjustment was edited");
    })
    .await;
}
