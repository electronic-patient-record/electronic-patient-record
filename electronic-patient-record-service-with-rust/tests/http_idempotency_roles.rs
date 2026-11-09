//! DB-gated, through the real router with login **on** (OF-1): an `Idempotency-Key` belongs to the token
//! subject that sent it. Two clinicians may use the same key without seeing each other's answer, and a
//! retry by the first gets the first's answer. One test, because the token guard is configured once per
//! process.
//!
//! Covered: the same key from two subjects writes twice; a retry by one subject replays its own answer
//! and writes nothing; the replay's audit row names the replaying subject. Not covered: a refused
//! subject reserving a key (authorization runs first, so it cannot), the hourly purge, and `/fhir/`.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test http_idempotency_roles -- --ignored --test-threads=1
//! ```

use authentication_verifier::test_idp::{AUDIENCE, REALM, TestIdp};
use axa_test::prelude::request;
use axum::http::{HeaderName, HeaderValue, StatusCode};
use electronic_patient_record::app::App;
use electronic_patient_record::pool::Pool;
use serde_json::{Value, json};
use sqlx::AssertSqlSafe;
use sqlx::Row;

fn bearer(token: &str) -> (HeaderName, HeaderValue) {
    (
        HeaderName::from_static("authorization"),
        HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
    )
}

fn unique(label: &str) -> String {
    let mut n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mut s = String::from(label);
    while n > 0 {
        s.push((b'a' + (n % 26) as u8) as char);
        n /= 26;
    }
    s
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_key_belongs_to_the_subject_that_sent_it() {
    let idp = TestIdp::start().await;
    let map: Value = json!({ "clinician": { "role": ["clinician"] } });
    // SAFETY: set before the app boots and before any other thread reads them; one test per process.
    unsafe {
        std::env::set_var("EPR_REQUIRE_AUTH", "true");
        std::env::set_var("EPR_KEYCLOAK_URL", &idp.base_url);
        std::env::set_var("EPR_KEYCLOAK_REALM", REALM);
        std::env::set_var("EPR_KEYCLOAK_AUDIENCES", AUDIENCE);
        std::env::set_var("EPR_KEYCLOAK_ROLE_MAP", map.to_string());
    }
    let sub_a = "0c4f1e2a-0000-4000-8000-0000000000e1";
    let sub_b = "0c4f1e2a-0000-4000-8000-0000000000e2";
    let token = |sub: &str| {
        idp.token(|c| {
            c["realm_access"] = json!({ "roles": ["clinician"] });
            c["sub"] = json!(sub);
        })
    };
    let (a, b) = (token(sub_a), token(sub_b));

    request::<App, _, _>(|server, ctx| async move {
        let (ah, av) = bearer(&a);
        let (bh, bv) = bearer(&b);
        let key = HeaderName::from_static("idempotency-key");
        let replayed = HeaderName::from_static("idempotent-replayed");
        let k = unique("shared");
        let title = unique("idem hazard ");
        let body = json!({ "title": title, "severity": 4, "likelihood": 2 });
        let send = |h: HeaderName, v: HeaderValue| {
            let (server, key, k, body) = (&server, key.clone(), k.clone(), body.clone());
            async move {
                server
                    .post("/api/hazards")
                    .add_header(h, v)
                    .add_header(key, HeaderValue::from_str(&k).unwrap())
                    .json(&body)
                    .await
            }
        };
        let titled = || async {
            server
                .get("/api/hazards")
                .add_header(ah.clone(), av.clone())
                .await
                .text()
                .matches(&title)
                .count()
        };

        let first_a = send(ah.clone(), av.clone()).await;
        assert_eq!(
            first_a.status_code(),
            StatusCode::CREATED,
            "{}",
            first_a.text()
        );
        // The other subject, the same key and body: its own record, not A's answer.
        let first_b = send(bh.clone(), bv.clone()).await;
        assert_eq!(
            first_b.status_code(),
            StatusCode::CREATED,
            "{}",
            first_b.text()
        );
        assert!(
            first_b.maybe_header(&replayed).is_none(),
            "B was given A's answer"
        );
        assert_ne!(
            first_a.json::<Value>()["id"],
            first_b.json::<Value>()["id"],
            "one key from two subjects must write two records"
        );
        assert_eq!(titled().await, 2);

        // A retry by A replays A's answer and writes nothing, and the audit row names A.
        let again_a = send(ah.clone(), av.clone()).await;
        assert_eq!(
            again_a.status_code(),
            StatusCode::CREATED,
            "{}",
            again_a.text()
        );
        assert_eq!(again_a.header(&replayed), "true");
        assert_eq!(again_a.text(), first_a.text());
        assert_eq!(titled().await, 2, "a replay writes nothing");
        let row = sqlx::query(
            "SELECT agent FROM audit_events_all WHERE resource_ref = 'IdempotentReplay' \
                 ORDER BY id DESC LIMIT 1",
        )
        .fetch_optional(&ctx.pool())
        .await
        .unwrap()
        .expect("a replay row");
        assert_eq!(row.try_get::<String, _>("agent").unwrap(), sub_a);

        // And B's retry replays B's.
        let again_b = send(bh.clone(), bv.clone()).await;
        assert_eq!(again_b.header(&replayed), "true");
        assert_eq!(again_b.text(), first_b.text());
        assert_eq!(
            sqlx::query(AssertSqlSafe(format!(
                "SELECT count(*)::bigint AS n FROM axa_idempotency WHERE key = '{k}'"
            )))
            .fetch_optional(&ctx.pool())
            .await
            .unwrap()
            .unwrap()
            .try_get::<i64, _>("n")
            .unwrap(),
            2,
            "one row per subject and key"
        );
    })
    .await;
}
