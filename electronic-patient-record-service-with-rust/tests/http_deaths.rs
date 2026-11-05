//! DB-gated, through the real router with login **on** (BV-1, OQ-BV2): any member of staff may report
//! and read the one answer, a clinician or nurse may read the assertions and take a step, only a
//! clinician may withdraw, a patient and a proxy have none of it, the reporter is the token, a stale
//! version is 409, the audit trail never holds a reason, and a proxy and the patient's own login stop
//! once a death is confirmed, not at a report, and come back when it is withdrawn (BV-5). One test, because the token guard is
//! configured once per process.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test http_deaths -- --ignored --test-threads=1
//! ```

use authentication_verifier::test_idp::{AUDIENCE, REALM, TestIdp};
use axa::testing::prelude::request;
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
async fn who_may_report_confirm_and_withdraw() {
    let idp = TestIdp::start().await;
    let roles = [
        "clinician",
        "nurse",
        "receptionist",
        "patient",
        "auditor",
        "clinical-admin",
        "proxy",
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
    let token = |role: &str, sub: &str| {
        idp.token(|c| {
            c["realm_access"] = json!({ "roles": [role] });
            c["sub"] = json!(sub);
        })
    };
    let clinician = token("clinician", "0c4f1e2a-0000-4000-8000-0000000000d1");
    let nurse = token("nurse", "0c4f1e2a-0000-4000-8000-0000000000d2");
    const RECEPTION: &str = "0c4f1e2a-0000-4000-8000-0000000000d3";
    let reception = token("receptionist", RECEPTION);
    const OWN: &str = "0c4f1e2a-0000-4000-8000-0000000000d4";
    let patient = token("patient", OWN);
    let auditor = token("auditor", "0c4f1e2a-0000-4000-8000-0000000000d5");
    let admin = token("clinical-admin", "0c4f1e2a-0000-4000-8000-0000000000d6");
    const PROXY: &str = "0c4f1e2a-0000-4000-8000-0000000000d7";
    let proxy = token("proxy", PROXY);

    request::<App, _, _>(|server, ctx| async move {
        let purpose = HeaderName::from_static("x-purpose-of-use");
        let ops = HeaderValue::from_static("operations");
        let auth = |t: &str| bearer(t);

        let (h, v) = auth(&reception);
        let made = server
            .post("/api/patients")
            .add_header(h, v)
            .add_header(purpose.clone(), ops.clone())
            .json(&json!({ "patient": Patient {
                names: vec![HumanName { family: Some("Deathtest".into()), given: vec![format!("N{}", nanos())] }],
                birth_date: Some(dob()),
                ..Patient::default()
            }, "override_reason": "death route test" }))
            .await;
        assert_eq!(made.status_code(), StatusCode::CREATED, "{}", made.text());
        let id = made.json::<Value>()["id"].as_str().unwrap().to_string();
        let answer = format!("/api/patients/{id}/death");
        let assertions = format!("/api/patients/{id}/death/assertions");

        // A client cannot supply a date of death: only a standing assertion does (OQ-BV3).
        let (h, v) = auth(&reception);
        let refused = server
            .post("/api/patients")
            .add_header(h, v)
            .add_header(purpose.clone(), ops.clone())
            .json(&json!({ "patient": { "names": [{ "family": "Deathtest", "given": ["Z"] }],
                "death_date": "2026-01-01" }, "override_reason": "x" }))
            .await;
        assert_eq!(refused.status_code(), StatusCode::UNPROCESSABLE_ENTITY, "{}", refused.text());
        assert!(refused.text().contains("death_date_not_accepted"), "{}", refused.text());
        let (h, v) = auth(&reception);
        let refused = server
            .post("/api/patients/compare")
            .add_header(h, v)
            .add_header(purpose.clone(), ops.clone())
            .json(&json!({ "a": { "names": [], "death_date": "2026-01-01" }, "b": { "names": [] } }))
            .await;
        assert_eq!(refused.status_code(), StatusCode::UNPROCESSABLE_ENTITY, "{}", refused.text());

        // A proxy with the identity and access log scopes, reading before anything is reported.
        let expires = i64::try_from(nanos() / 1_000_000_000).unwrap() + 3600;
        let (h, v) = auth(&admin);
        let related = server
            .post(&format!("/api/patients/{id}/relationships"))
            .add_header(h, v)
            .add_header(purpose.clone(), ops.clone())
            .json(&json!({ "relationship": { "subject": PROXY, "kind": "proxy", "status": "active",
                "proxy": { "basis": "parental-responsibility",
                    "scopes": ["identity", "access-log"], "expires_at": expires } },
                "reason": "seen" }))
            .await;
        assert_eq!(related.status_code(), StatusCode::CREATED, "{}", related.text());
        // And the patient's own login.
        let (h, v) = auth(&admin);
        let own = server
            .post(&format!("/api/patients/{id}/relationships"))
            .add_header(h, v)
            .add_header(purpose.clone(), ops.clone())
            .json(&json!({ "relationship": { "subject": OWN, "kind": "self", "status": "active" },
                "reason": "seen" }))
            .await;
        assert_eq!(own.status_code(), StatusCode::CREATED, "{}", own.text());
        let portal = HeaderValue::from_static("patient-access");
        let as_proxy = |path: String| {
            let (h, v) = auth(&proxy);
            server.get(&path).add_header(h, v).add_header(purpose.clone(), portal.clone())
        };
        let record = format!("/api/patients/{id}");
        let log = format!("/api/me/patients/{id}/access-log");
        assert_eq!(as_proxy(record.clone()).await.status_code(), StatusCode::OK);
        assert_eq!(as_proxy(log.clone()).await.status_code(), StatusCode::OK);
        assert!(as_proxy("/api/me/patients".into()).await.text().contains(&id));
        let as_self = |path: String| {
            let (h, v) = auth(&patient);
            server.get(&path).add_header(h, v).add_header(purpose.clone(), portal.clone())
        };
        assert_eq!(as_self(record.clone()).await.status_code(), StatusCode::OK);
        assert!(as_self("/api/me/patients".into()).await.text().contains(&id));

        // Nothing yet: not known, which is not the same as living, and nothing is held.
        let (h, v) = auth(&reception);
        let first = server
            .get(&answer)
            .add_header(h, v)
            .add_header(purpose.clone(), ops.clone())
            .await;
        assert_eq!(first.status_code(), StatusCode::OK, "{}", first.text());
        assert_eq!(first.json::<Value>()["answer"], "not-known-to-have-died");
        assert_eq!(first.json::<Value>()["holds_contact"], false);

        // A patient and an auditor have none of this.
        for t in [&auditor, &patient] {
            let (h, v) = auth(t);
            let r = server.get(&answer).add_header(h, v).add_header(purpose.clone(), ops.clone()).await;
            assert_eq!(r.status_code(), StatusCode::FORBIDDEN, "{}", r.text());
        }

        // Reception reports; the body cannot name somebody else.
        const SECRET: &str = "the daughter telephoned reception";
        let (h, v) = auth(&reception);
        let reported = server
            .post(&assertions)
            .add_header(h, v)
            .add_header(purpose.clone(), ops.clone())
            .json(&json!({ "source": "family-member",
                "occurred": { "kind": "known", "at": "2026-11-01" },
                "reason": SECRET,
                "reported_by": { "kind": "person", "id": "somebody-else" } }))
            .await;
        assert_eq!(reported.status_code(), StatusCode::CREATED, "{}", reported.text());
        let reported: Value = reported.json();
        assert_eq!(reported["author"], RECEPTION);
        assert_eq!(reported["assertion"]["reported_by"]["id"], RECEPTION);
        let step = format!("{assertions}/{}", reported["id"].as_str().unwrap());

        // The report holds contact at once, unconfirmed.
        let (h, v) = auth(&reception);
        let held: Value = server.get(&answer).add_header(h, v).add_header(purpose.clone(), ops.clone()).await.json();
        assert_eq!(held["answer"], "asserted");
        assert_eq!(held["confirmed"], false);
        assert_eq!(held["holds_contact"], true);

        // A blank reason and a date in the future are refused.
        let (h, v) = auth(&reception);
        let blank = server
            .post(&assertions)
            .add_header(h, v)
            .add_header(purpose.clone(), ops.clone())
            .json(&json!({ "source": "hospital", "occurred": { "kind": "known", "at": "2026-11-01" }, "reason": " " }))
            .await;
        assert_eq!(blank.status_code(), StatusCode::UNPROCESSABLE_ENTITY, "{}", blank.text());
        let (h, v) = auth(&reception);
        let future = server
            .post(&assertions)
            .add_header(h, v)
            .add_header(purpose.clone(), ops.clone())
            .json(&json!({ "source": "hospital", "occurred": { "kind": "known", "at": "2999-01-01" }, "reason": "r" }))
            .await;
        assert_eq!(future.status_code(), StatusCode::UNPROCESSABLE_ENTITY, "{}", future.text());

        // In FHIR an unconfirmed report is a tag, never `deceased[x]`.
        let (h, v) = auth(&reception);
        let read: Value = server
            .get(&format!("/fhir/R4/Patient/{id}"))
            .add_header(h, v)
            .add_header(purpose.clone(), ops.clone())
            .await
            .json();
        assert_eq!(read["meta"]["tag"][0]["code"], "death-reported");
        assert!(read.get("deceasedDateTime").is_none(), "{read}");

        // A report alone does not stop a proxy: the patient may be alive (BV-5).
        assert_eq!(as_proxy(record.clone()).await.status_code(), StatusCode::OK);
        assert_eq!(as_proxy(log.clone()).await.status_code(), StatusCode::OK);
        assert!(as_proxy("/api/me/patients".into()).await.text().contains(&id));
        // Nor the patient's own login.
        assert_eq!(as_self(record.clone()).await.status_code(), StatusCode::OK);
        assert_eq!(as_self(log.clone()).await.status_code(), StatusCode::OK);
        assert!(as_self("/api/me/patients".into()).await.text().contains(&id));

        // The matcher sees the date of the report, and it is not stored on the identity.
        let stored = electronic_patient_record::patients_store::current(&ctx.pool(), &id).await.unwrap();
        assert_eq!(stored.patient.death_date, None);
        let seen = electronic_patient_record::death_store::for_matching(&ctx.pool(), &id, &stored.patient)
            .await
            .unwrap();
        assert_eq!(seen.death_date.as_deref(), Some("2026-11-01"));

        // Reception can neither read the reasons nor take a step.
        let (h, v) = auth(&reception);
        let denied = server.get(&assertions).add_header(h, v).add_header(purpose.clone(), ops.clone()).await;
        assert_eq!(denied.status_code(), StatusCode::FORBIDDEN);
        let (h, v) = auth(&reception);
        let denied = server
            .put(&step)
            .add_header(h, v)
            .add_header(purpose.clone(), ops.clone())
            .json(&json!({ "expected_version": 1, "to": "verified", "reason": "r" }))
            .await;
        assert_eq!(denied.status_code(), StatusCode::FORBIDDEN);

        // A nurse verifies, and a stale version is a conflict.
        let (h, v) = auth(&nurse);
        let verified = server
            .put(&step)
            .add_header(h, v)
            .add_header(purpose.clone(), ops.clone())
            .json(&json!({ "expected_version": 1, "to": "verified", "reason": "seen by the nurse" }))
            .await;
        assert_eq!(verified.status_code(), StatusCode::OK, "{}", verified.text());
        // Confirmed, a proxy stops, on every path, and the list does not say why.
        assert_eq!(as_proxy(record.clone()).await.status_code(), StatusCode::FORBIDDEN);
        assert_eq!(as_proxy(log.clone()).await.status_code(), StatusCode::FORBIDDEN);
        assert!(!as_proxy("/api/me/patients".into()).await.text().contains(&id));
        // So does the patient's own login.
        assert_eq!(as_self(record.clone()).await.status_code(), StatusCode::FORBIDDEN);
        assert_eq!(as_self(log.clone()).await.status_code(), StatusCode::FORBIDDEN);
        assert!(!as_self("/api/me/patients".into()).await.text().contains(&id));
        let (h, v) = auth(&nurse);
        let stale = server
            .put(&step)
            .add_header(h, v)
            .add_header(purpose.clone(), ops.clone())
            .json(&json!({ "expected_version": 1, "to": "registered", "reason": "r" }))
            .await;
        assert_eq!(stale.status_code(), StatusCode::CONFLICT, "{}", stale.text());

        // FHIR: confirmed, so it is `deceased[x]`, and a read sent back as an update changes nothing
        // about the death. A write cannot change or report one.
        let fhir = format!("/fhir/R4/Patient/{id}");
        let fhir_type = (
            HeaderName::from_static("content-type"),
            HeaderValue::from_static("application/fhir+json"),
        );
        let (h, v) = auth(&reception);
        let read = server.get(&fhir).add_header(h, v).add_header(purpose.clone(), ops.clone()).await;
        assert_eq!(read.status_code(), StatusCode::OK, "{}", read.text());
        let body: Value = read.json();
        assert_eq!(body["deceasedDateTime"], "2026-11-01");
        let if_match = HeaderName::from_static("if-match");
        let (h, v) = auth(&reception);
        let back = server
            .put(&fhir)
            .add_header(h, v)
            .add_header(purpose.clone(), ops.clone())
            .add_header(if_match.clone(), HeaderValue::from_static("W/\"1\""))
            .add_header(fhir_type.0.clone(), fhir_type.1.clone())
            .text(body.to_string())
            .await;
        assert_eq!(back.status_code(), StatusCode::OK, "{}", back.text());
        let mut changed = body.clone();
        changed["deceasedDateTime"] = json!("2026-10-01");
        let (h, v) = auth(&reception);
        let refused = server
            .put(&fhir)
            .add_header(h, v)
            .add_header(purpose.clone(), ops.clone())
            .add_header(if_match.clone(), HeaderValue::from_static("W/\"1\""))
            .add_header(fhir_type.0.clone(), fhir_type.1.clone())
            .text(changed.to_string())
            .await;
        assert_eq!(refused.status_code(), StatusCode::UNPROCESSABLE_ENTITY, "{}", refused.text());
        assert!(refused.text().contains("death/assertions"), "{}", refused.text());

        // A nurse may not withdraw; a clinician may, and the assertion stays in the list.
        let (h, v) = auth(&nurse);
        let denied = server
            .put(&step)
            .add_header(h, v)
            .add_header(purpose.clone(), ops.clone())
            .json(&json!({ "expected_version": 2, "to": "withdrawn-in-error", "reason": "r" }))
            .await;
        assert_eq!(denied.status_code(), StatusCode::FORBIDDEN);
        let (h, v) = auth(&clinician);
        let withdrawn = server
            .put(&step)
            .add_header(h, v)
            .add_header(purpose.clone(), ops.clone())
            .json(&json!({ "expected_version": 2, "to": "withdrawn-in-error", "reason": "wrong patient" }))
            .await;
        assert_eq!(withdrawn.status_code(), StatusCode::OK, "{}", withdrawn.text());
        let (h, v) = auth(&clinician);
        let all: Value = server.get(&assertions).add_header(h, v).add_header(purpose.clone(), ops.clone()).await.json();
        assert_eq!(all["assertions"][0]["assertion"]["status"], "withdrawn-in-error");
        let (h, v) = auth(&reception);
        let gone: Value = server
            .get(&format!("/fhir/R4/Patient/{id}"))
            .add_header(h, v)
            .add_header(purpose.clone(), ops.clone())
            .await
            .json();
        assert!(gone.get("deceasedDateTime").is_none(), "{gone}");
        // Withdrawn, the matcher sees no date.
        let seen = electronic_patient_record::death_store::for_matching(&ctx.pool(), &id, &stored.patient)
            .await
            .unwrap();
        assert_eq!(seen.death_date, None);
        // Withdrawn, the proxy is back, with nothing to restore by hand.
        assert_eq!(as_proxy(record.clone()).await.status_code(), StatusCode::OK);
        assert!(as_proxy("/api/me/patients".into()).await.text().contains(&id));
        assert_eq!(as_self(record.clone()).await.status_code(), StatusCode::OK);
        assert_eq!(as_self(log.clone()).await.status_code(), StatusCode::OK);
        assert!(as_self("/api/me/patients".into()).await.text().contains(&id));
        let (h, v) = auth(&reception);
        let released: Value = server.get(&answer).add_header(h, v).add_header(purpose.clone(), ops.clone()).await.json();
        assert_eq!(released["holds_contact"], false);

        // The audit trail has the writes and none of the words.
        let rows = sqlx::query(AssertSqlSafe(format!(
                    "SELECT detail::text AS d FROM audit_events
                     WHERE resource_ref = 'DeathAssertion' AND patient_ref = 'Patient/{id}'"
                )))
.fetch_all(&ctx.pool())
            .await
            .unwrap();
        assert!(rows.len() >= 3, "{}", rows.len());
        for row in rows {
            let d: String = row.try_get("d").unwrap();
            assert!(!d.contains(SECRET) && !d.contains("2026-11-01"), "{d}");
        }
    })
    .await;
}
