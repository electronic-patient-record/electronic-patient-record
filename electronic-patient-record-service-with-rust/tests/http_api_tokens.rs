//! DB-gated, through the real router with login **on** (TL-1): only the clinical administrator issues, lists, describes
//! and revokes integration tokens; a token is shown once and never stored or returned in clear; a name is never reused;
//! a scope must be one the integrator role holds; revocation is final; the token tables refuse change; every step is
//! audited without the token; an Idempotency-Key never keeps the token (TL-11). One test, because the token guard is
//! configured once per process.
//!
//! ```text
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test http_api_tokens -- --ignored --test-threads=1
//! ```

use authentication_verifier::test_idp::{AUDIENCE, REALM, TestIdp};
use axa_test::prelude::request;
use axum::http::{HeaderName, HeaderValue, StatusCode};
use electronic_patient_record::app::App;
use electronic_patient_record::pool::Pool;
use serde_json::{Value, json};
use sqlx::Row;

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
    format!("{label}-{letters}")
}

fn bearer(token: &str) -> (HeaderName, HeaderValue) {
    (
        HeaderName::from_static("authorization"),
        HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
    )
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn only_an_administrator_manages_tokens_and_a_token_is_shown_once_and_never_kept() {
    let idp = TestIdp::start().await;
    let roles = ["clinical-admin", "clinician", "integrator"];
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
    let admin = bearer(&token(
        "clinical-admin",
        "0c4f1e2a-0000-4000-8000-0000000008a1",
    ));
    let other_admin = bearer(&token(
        "clinical-admin",
        "0c4f1e2a-0000-4000-8000-0000000008a2",
    ));
    let clinician = bearer(&token("clinician", "0c4f1e2a-0000-4000-8000-0000000008c1"));
    let integrator = bearer(&token("integrator", "0c4f1e2a-0000-4000-8000-0000000008d1"));

    request::<App, _, _>(|server, ctx| async move {
        let db = &ctx.pool();
        let name = tag("referral-feed");
        let issue = |who: &(HeaderName, HeaderValue), body: Value| {
            server
                .post("/api/tokens")
                .add_header(who.0.clone(), who.1.clone())
                .json(&body)
        };

        // Nobody but an administrator issues or lists.
        for who in [&clinician, &integrator] {
            let r = issue(who, json!({"name": name, "scopes": ["referral-queue:create"], "expires_in_days": 30})).await;
            assert_eq!(r.status_code(), StatusCode::FORBIDDEN, "{}", r.text());
            let r = server.get("/api/tokens").add_header(who.0.clone(), who.1.clone()).await;
            assert_eq!(r.status_code(), StatusCode::FORBIDDEN, "{}", r.text());
        }

        // A scope beyond the integrator role, a life out of range, or a bad name is refused.
        for bad in [
            json!({"name": name, "scopes": ["audit-log:export"], "expires_in_days": 30}),
            json!({"name": name, "scopes": ["referral-queue:create"], "expires_in_days": 366}),
            json!({"name": name, "scopes": [], "expires_in_days": 30}),
            json!({"name": "Bad Name", "scopes": ["referral-queue:create"], "expires_in_days": 30}),
        ] {
            let r = issue(&admin, bad.clone()).await;
            assert_eq!(r.status_code(), StatusCode::UNPROCESSABLE_ENTITY, "{bad}: {}", r.text());
        }

        // Issued: the token once, with no-store; then never again.
        let r = issue(
            &admin,
            json!({"name": name, "description": "Inbound referrals", "scopes": ["referral-queue:create"], "expires_in_days": 30}),
        )
        .await;
        assert_eq!(r.status_code(), StatusCode::CREATED, "{}", r.text());
        assert_eq!(r.headers().get("cache-control").and_then(|v| v.to_str().ok()), Some("no-store"));
        let issued = r.json::<Value>();
        let secret_token = issued["token"].as_str().unwrap().to_string();
        let id = issued["id"].as_str().unwrap().to_string();
        assert!(secret_token.starts_with(&format!("epr_{id}_")), "{secret_token}");
        assert_eq!(issued["scopes"], json!(["referral-queue:create"]));
        let secret = secret_token.rsplit('_').next().unwrap().to_string();
        // The stored record verifies the token, with its scope and no other.
        let prefix = electronic_patient_record::api_tokens_store::prefix().expect("a valid prefix");
        let presented = axa::api_tokens::parse(&secret_token, &prefix).expect("a token");
        let (record, record_name) = electronic_patient_record::api_tokens_store::record(db, presented.id())
            .await
            .unwrap()
            .expect("its record");
        assert_eq!(record_name, name);
        let verified = record.verify(&presented, std::time::SystemTime::now()).expect("it verifies");
        assert!(verified.allows("referral-queue:create"));
        assert!(!verified.allows("patient-identity:create"));

        let listed = server.get("/api/tokens").add_header(admin.0.clone(), admin.1.clone()).await;
        assert_eq!(listed.status_code(), StatusCode::OK, "{}", listed.text());
        let text = listed.text();
        assert!(text.contains(&name));
        assert!(!text.contains(&secret), "the list never carries the token");
        let hash: String = sqlx::query("SELECT hash FROM api_tokens WHERE name = $1")
            .bind(&name)
            .fetch_one(db)
            .await
            .unwrap()
            .get("hash");
        assert!(!text.contains(&hash), "the list never carries the hash");
        assert_ne!(hash, secret);
        // Nothing in the database holds the token or its secret in clear: not the token tables, not the audit trail.
        for sql in [
            "SELECT count(*)::bigint AS n FROM api_tokens WHERE hash = $1 OR name = $1",
            "SELECT count(*)::bigint AS n FROM api_token_versions WHERE description LIKE '%' || $1 || '%'",
            "SELECT count(*)::bigint AS n FROM audit_events_all WHERE detail::text LIKE '%' || $1 || '%'",
        ] {
            let n: i64 = sqlx::query(sql).bind(&secret).fetch_one(db).await.unwrap().get("n");
            assert_eq!(n, 0, "{sql}");
        }
        let audited: i64 = sqlx::query(
            "SELECT count(*)::bigint AS n FROM audit_events_all WHERE resource_ref = 'ApiToken' AND detail::text LIKE '%' || $1 || '%'",
        )
        .bind(&name)
        .fetch_one(db)
        .await
        .unwrap()
        .get("n");
        assert!(audited >= 1, "the issue is audited by name");

        // A name is never reused.
        let r = issue(&admin, json!({"name": name, "scopes": ["referral-queue:create"], "expires_in_days": 30})).await;
        assert_eq!(r.status_code(), StatusCode::CONFLICT, "{}", r.text());

        // TL-11: an issue sent with an Idempotency-Key keeps no answer, so the store never holds the token, and a
        // retry under the same key is a new request (409: the name is taken), never a replay of the token.
        let keyed_name = tag("keyed-feed");
        let keyed = |body: Value| {
            server
                .post("/api/tokens")
                .add_header(admin.0.clone(), admin.1.clone())
                .add_header(HeaderName::from_static("idempotency-key"), HeaderValue::from_static("tl-11-issue"))
                .json(&body)
        };
        let body = json!({"name": keyed_name, "scopes": ["referral-queue:create"], "expires_in_days": 1});
        let r = keyed(body.clone()).await;
        assert_eq!(r.status_code(), StatusCode::CREATED, "{}", r.text());
        let keyed_secret = r.json::<Value>()["token"].as_str().unwrap().rsplit('_').next().unwrap().to_string();
        let r = keyed(body).await;
        assert_eq!(r.status_code(), StatusCode::CONFLICT, "{}", r.text());
        assert!(r.headers().get("idempotent-replayed").is_none());
        assert!(!r.text().contains(&keyed_secret), "a retry never shows the token again");
        for s in [&secret, &keyed_secret] {
            let n: i64 = sqlx::query(
                "SELECT count(*)::bigint AS n FROM axa_idempotency WHERE position(convert_to($1, 'UTF8') in body) > 0",
            )
            .bind(s)
            .fetch_one(db)
            .await
            .unwrap()
            .get("n");
            assert_eq!(n, 0, "the idempotency store holds no token");
        }

        // Describe at the version read; a stale version is a 409.
        let describe = |v: i64, d: &str| {
            server
                .post(&format!("/api/tokens/{id}/description"))
                .add_header(admin.0.clone(), admin.1.clone())
                .json(&json!({"expected_version": v, "description": d}))
        };
        assert_eq!(describe(2, "x").await.status_code(), StatusCode::CONFLICT);
        let r = describe(1, "Inbound referrals from the hospital").await;
        assert_eq!(r.status_code(), StatusCode::OK, "{}", r.text());
        assert!(!r.text().contains(&secret), "describing never returns the token");
        assert_eq!(r.json::<Value>()["version"], 2);

        // Another administrator revokes it, with a reason; then it takes no change, and revoking again is refused.
        let revoke = |who: &(HeaderName, HeaderValue), v: i64, reason: &str| {
            server
                .post(&format!("/api/tokens/{id}/revoke"))
                .add_header(who.0.clone(), who.1.clone())
                .json(&json!({"expected_version": v, "reason": reason}))
        };
        assert_eq!(revoke(&other_admin, 2, "  ").await.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(revoke(&clinician, 2, "no").await.status_code(), StatusCode::FORBIDDEN);
        let r = revoke(&other_admin, 2, "the feed moved").await;
        assert_eq!(r.status_code(), StatusCode::OK, "{}", r.text());
        assert!(!r.text().contains(&secret), "revoking never returns the token");
        let revoked = r.json::<Value>();
        assert_eq!((revoked["revoked"].as_bool(), revoked["revoked_reason"].as_str()), (Some(true), Some("the feed moved")));
        assert_eq!(revoke(&other_admin, 3, "again").await.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        let (record, _) = electronic_patient_record::api_tokens_store::record(db, presented.id())
            .await
            .unwrap()
            .expect("its record");
        assert_eq!(
            record.verify(&presented, std::time::SystemTime::now()).unwrap_err(),
            axa::api_tokens::Refused::Revoked
        );
        assert_eq!(describe(3, "after").await.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        let r = server
            .post("/api/tokens/not-a-token/revoke")
            .add_header(admin.0.clone(), admin.1.clone())
            .json(&json!({"expected_version": 1, "reason": "x"}))
            .await;
        assert_eq!(r.status_code(), StatusCode::NOT_FOUND, "{}", r.text());

        // Signing in with a token: it acts as `token:<name>` with the integrator role and only its scopes.
        let finder = tag("patient-finder");
        let r = issue(&admin, json!({"name": finder, "scopes": ["patient-identity:search"], "expires_in_days": 1})).await;
        assert_eq!(r.status_code(), StatusCode::CREATED, "{}", r.text());
        let issued = r.json::<Value>();
        let finder_id = issued["id"].as_str().unwrap().to_string();
        let as_token = bearer(issued["token"].as_str().unwrap());
        let before: i64 = sqlx::query("SELECT count(*)::bigint AS n FROM audit_events_all WHERE agent = $1")
            .bind(format!("token:{finder}"))
            .fetch_one(db)
            .await
            .unwrap()
            .get("n");
        let r = server
            .get("/api/patients?family=nobodyatall")
            .add_header(as_token.0.clone(), as_token.1.clone())
            .await;
        assert_eq!(r.status_code(), StatusCode::OK, "within its scope: {}", r.text());
        let after: i64 = sqlx::query("SELECT count(*)::bigint AS n FROM audit_events_all WHERE agent = $1")
            .bind(format!("token:{finder}"))
            .fetch_one(db)
            .await
            .unwrap()
            .get("n");
        assert!(after > before, "the audit row names the token");
        let used: Option<String> =
            sqlx::query_scalar("SELECT last_used_at::text FROM api_token_uses WHERE token_id = $1::uuid")
                .bind(uuid::Uuid::parse_str(&finder_id).unwrap().to_string())
                .fetch_optional(db)
                .await
                .unwrap();
        assert!(used.is_some(), "its last use is recorded");
        // The integrator role may register a patient, but no scope says so; nor may a token manage tokens.
        let r = server
            .post("/api/patients")
            .add_header(as_token.0.clone(), as_token.1.clone())
            .json(&json!({"patient": {"names": [{"family": "Tokenmade"}], "birth_date": "1950-01-01"}}))
            .await;
        assert_eq!(r.status_code(), StatusCode::FORBIDDEN, "outside its scope: {}", r.text());
        let r = server.get("/api/tokens").add_header(as_token.0.clone(), as_token.1.clone()).await;
        assert_eq!(r.status_code(), StatusCode::FORBIDDEN, "{}", r.text());
        // A malformed, unknown or revoked token is the same 401, saying nothing of why.
        let unknown = format!("epr_{}_{}", "0".repeat(32), "a".repeat(62));
        let r = server
            .post(&format!("/api/tokens/{finder_id}/revoke"))
            .add_header(admin.0.clone(), admin.1.clone())
            .json(&json!({"expected_version": 1, "reason": "test over"}))
            .await;
        assert_eq!(r.status_code(), StatusCode::OK, "{}", r.text());
        let mut bodies = Vec::new();
        for t in ["epr_short".to_string(), unknown, issued["token"].as_str().unwrap().to_string()] {
            let h = bearer(&t);
            let r = server.get("/api/patients?family=nobodyatall").add_header(h.0, h.1).await;
            assert_eq!(r.status_code(), StatusCode::UNAUTHORIZED, "{}", r.text());
            bodies.push(r.text());
        }
        assert!(bodies.iter().all(|b| b == &bodies[0]), "one answer for every refusal: {bodies:?}");
        assert!(!bodies[0].contains("revoked") && !bodies[0].contains("expired"));

        // The tables refuse change.
        for sql in [
            "UPDATE api_tokens SET scopes = ARRAY['patient-identity:update']",
            "DELETE FROM api_tokens",
            "TRUNCATE api_tokens CASCADE",
            "UPDATE api_token_versions SET revoked = false",
            "DELETE FROM api_token_versions",
        ] {
            assert!(sqlx::query(sql).execute(db).await.is_err(), "{sql} was allowed");
        }
    })
    .await;
}
