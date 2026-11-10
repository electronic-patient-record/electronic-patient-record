//! DB-gated, through the real router with login off (OF-1): a create sent with an
//! `Idempotency-Key` is run once; a retry gets the stored answer and writes nothing.
//!
//! Covered: a first create, an identical retry (same body, `Idempotent-Replayed`, one record, an audit
//! row), a different body under the same key (422), a malformed key (400), a refused first request
//! that releases its key, a key that is still pending (409), an expired key that writes again, and a
//! request without a key (unchanged: it writes twice). Not covered: per-subject separation (login is
//! off here; `http_idempotency_roles.rs` covers it), the hourly purge loop, a crash between the write and the stored
//! answer, and the `/fhir/` creates.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test http_idempotency -- --ignored --test-threads=1
//! ```

use axa_test::prelude::request;
use axum::http::{HeaderName, HeaderValue, StatusCode};
use electronic_patient_record::app::App;
use electronic_patient_record::pool::Pool;
use serde_json::{Value, json};
use sqlx::{AssertSqlSafe, PgPool, Row};

fn letters(label: &str) -> String {
    let mut n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mut s = String::new();
    while n > 0 {
        s.push((b'a' + (n % 26) as u8) as char);
        n /= 26;
    }
    format!("{label}{s}")
}

async fn count(db: &PgPool, sql: &str) -> i64 {
    sqlx::query(AssertSqlSafe(sql.to_string().to_string()))
        .fetch_optional(db)
        .await
        .expect("count")
        .expect("a row")
        .try_get("n")
        .expect("n")
}

fn observation(value: f64) -> Value {
    json!({ "observation": {
        "code": "8867-4", "value": value, "unit": "/min",
        "range": { "low": 60.0, "high": 100.0 },
        "effective_at": "2026-10-26T09:30:00Z", "status": "final" }, "reason": "entered late" })
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_create_with_an_idempotency_key_runs_once() {
    // SAFETY: set before the app boots; the suite runs single-threaded.
    unsafe { std::env::set_var("EPR_REQUIRE_AUTH", "false") };
    request::<App, _, _>(|server, ctx| async move {
        let family = letters("idem");
        let created = server
            .post("/api/patients")
            .json(
                &json!({ "patient": { "names": [{ "family": family, "given": [family] }],
                "birth_date": "1933-04-05" }, "override_reason": "idempotency test" }),
            )
            .await;
        assert_eq!(
            created.status_code(),
            StatusCode::CREATED,
            "{}",
            created.text()
        );
        let id = created.json::<Value>()["id"].as_str().unwrap().to_string();
        let url = format!("/api/patients/{id}/observations");
        let key = HeaderName::from_static("idempotency-key");
        let replayed = HeaderName::from_static("idempotent-replayed");
        let listed = || async {
            server.get(&url).await.json::<Value>()["observations"]
                .as_array()
                .unwrap()
                .len()
        };
        let k = letters("key");

        let first = server
            .post(&url)
            .add_header(key.clone(), HeaderValue::from_str(&k).unwrap())
            .json(&observation(72.0))
            .await;
        assert_eq!(first.status_code(), StatusCode::CREATED, "{}", first.text());
        assert!(first.maybe_header(&replayed).is_none());
        assert_eq!(listed().await, 1);

        // An identical retry: the same answer, nothing written, an audit row.
        let audits = count(
            &ctx.pool(),
            "SELECT count(*)::bigint AS n FROM audit_events_all",
        )
        .await;
        let again = server
            .post(&url)
            .add_header(key.clone(), HeaderValue::from_str(&k).unwrap())
            .json(&observation(72.0))
            .await;
        assert_eq!(again.status_code(), StatusCode::CREATED, "{}", again.text());
        assert_eq!(again.text(), first.text());
        assert_eq!(again.header(&replayed), "true");
        assert_eq!(listed().await, 1, "a replay writes nothing");
        assert!(
            count(
                &ctx.pool(),
                "SELECT count(*)::bigint AS n FROM audit_events_all"
            )
            .await
                > audits,
            "a replay is audited"
        );

        // The same key for a different request.
        let other = server
            .post(&url)
            .add_header(key.clone(), HeaderValue::from_str(&k).unwrap())
            .json(&observation(80.0))
            .await;
        assert_eq!(
            other.status_code(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            other.text()
        );
        assert_eq!(listed().await, 1);

        // A malformed key.
        let bad = server
            .post(&url)
            .add_header(key.clone(), HeaderValue::from_static("not a key"))
            .json(&observation(72.0))
            .await;
        assert_eq!(bad.status_code(), StatusCode::BAD_REQUEST, "{}", bad.text());

        // A first request that is refused releases its key, so the fixed request can use it.
        let k2 = letters("fix");
        let refused = server
            .post(&url)
            .add_header(key.clone(), HeaderValue::from_str(&k2).unwrap())
            .json(&json!({ "observation": { "code": "" } }))
            .await;
        assert!(!refused.status_code().is_success(), "{}", refused.text());
        let fixed = server
            .post(&url)
            .add_header(key.clone(), HeaderValue::from_str(&k2).unwrap())
            .json(&observation(75.0))
            .await;
        assert_eq!(fixed.status_code(), StatusCode::CREATED, "{}", fixed.text());
        assert_eq!(listed().await, 2);

        // A key still running (the first request has not finished, or wrote and could not keep its answer) is a
        // conflict, never a second write, for as long as its lease, which is the key's life (H-043).
        sqlx::raw_sql(AssertSqlSafe(format!(
            "UPDATE axa_idempotency SET state = 'running', lease_until = now() + interval '23 hours' WHERE key = '{k}'"
        )))
        .execute(&ctx.pool())
        .await
        .unwrap();
        let busy = server
            .post(&url)
            .add_header(key.clone(), HeaderValue::from_str(&k).unwrap())
            .json(&observation(72.0))
            .await;
        assert_eq!(busy.status_code(), StatusCode::CONFLICT, "{}", busy.text());
        assert_eq!(listed().await, 2);

        // An expired key is removed and the request runs again.
        sqlx::raw_sql(AssertSqlSafe(format!(
            "UPDATE axa_idempotency SET lease_until = now() - interval '1 second', expires_at = now() - interval '1 second' WHERE key = '{k}'"
        )))
        .execute(&ctx.pool())
        .await
        .unwrap();
        let after = server
            .post(&url)
            .add_header(key.clone(), HeaderValue::from_str(&k).unwrap())
            .json(&observation(72.0))
            .await;
        assert_eq!(after.status_code(), StatusCode::CREATED, "{}", after.text());
        assert!(after.maybe_header(&replayed).is_none());
        assert_eq!(listed().await, 3);

        // Without a key nothing changes: the same request writes twice.
        for _ in 0..2 {
            let r = server.post(&url).json(&observation(72.0)).await;
            assert_eq!(r.status_code(), StatusCode::CREATED, "{}", r.text());
        }
        assert_eq!(listed().await, 5);
    })
    .await;
}
