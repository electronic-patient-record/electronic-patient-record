//! DB-gated, through the real router with login **on** (hazard H-044): the referrer's worklist. One test, because
//! the token guard is configured once per process.
//!
//! Covered: after an integrator's decline the sending clinician sees the referral on their worklist (ids and
//! state only, no note) and in the count; another clinician sees none of it and cannot acknowledge it (404);
//! the integrator, the receptionist and the administrator are refused the list (403); a stale version is a 409,
//! the current one is acknowledged once (409 the second time) and leaves the list; the audit rows carry ids and
//! states but never the note or the message. Assumed, awaiting the maintainer (OQ-PC5): the sender of
//! version 1 is the referrer and which roles may read the list.
//! Not covered: consent or a care relationship at the list (there is none by design), concurrent
//! acknowledgements, a recipient's or a team's worklist (not built).
//!
//! ```text
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test http_referral_worklist -- --ignored --test-threads=1
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

/// A birth date no earlier run shares, so the duplicate check does not shortlist their patients.
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

const SECRET_NOTE: &str = "Secret-note-lapwing";
const SECRET_MESSAGE: &str = "Secret-message-curlew";

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn the_sender_sees_what_came_back_and_nobody_else_does() {
    let idp = TestIdp::start().await;
    let roles = ["clinical-admin", "clinician", "integrator", "receptionist"];
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
    const SENDER: &str = "0c4f1e2a-0000-4000-8000-0000000000c1";
    const OTHER: &str = "0c4f1e2a-0000-4000-8000-0000000000c4";
    let admin = token("clinical-admin", "0c4f1e2a-0000-4000-8000-0000000000c0");
    let sender = token("clinician", SENDER);
    let other = token("clinician", OTHER);
    let integrator = token("integrator", "0c4f1e2a-0000-4000-8000-0000000000c2");
    let receptionist = token("receptionist", "0c4f1e2a-0000-4000-8000-0000000000c3");

    request::<App, _, _>(|server, ctx| async move {
        let (ah, av) = bearer(&admin);
        let (sh, sv) = bearer(&sender);
        let (oh, ov) = bearer(&other);
        let (ih, iv) = bearer(&integrator);
        let (xh, xv) = bearer(&receptionist);

        let family = format!("worklist{}", nanos());
        let made = server
            .post("/api/patients")
            .add_header(ah.clone(), av.clone())
            .json(&json!({ "patient": {
                "names": [{ "family": family, "given": [family] }], "birth_date": dob() } }))
            .await;
        assert_eq!(made.status_code(), StatusCode::CREATED, "{}", made.text());
        let pid = made.json::<Value>()["id"].as_str().unwrap().to_string();
        let rel = server
            .post(&format!("/api/patients/{pid}/relationships"))
            .add_header(ah.clone(), av.clone())
            .json(&json!({
                "relationship": { "subject": SENDER, "kind": "care-team", "status": "active" },
                "reason": "test" }))
            .await;
        assert_eq!(rel.status_code(), StatusCode::CREATED, "{}", rel.text());
        let sent = server
            .post(&format!("/api/patients/{pid}/referrals"))
            .add_header(sh.clone(), sv.clone())
            .json(&json!({ "referral": {
                "recipient": "Cardiology", "reason": "Chest pain",
                "question": "Please assess", "priority": "routine" } }))
            .await;
        assert_eq!(sent.status_code(), StatusCode::CREATED, "{}", sent.text());
        let rid = sent.json::<Value>()["id"].as_str().unwrap().to_string();

        let count = |h: HeaderName, v: HeaderValue| {
            let server = &server;
            async move {
                let r = server
                    .get("/api/referrals/worklist/count")
                    .add_header(h, v)
                    .await;
                assert_eq!(r.status_code(), StatusCode::OK, "{}", r.text());
                r.json::<Value>()["open"].as_i64().unwrap()
            }
        };
        let before = count(sh.clone(), sv.clone()).await;
        let other_before = count(oh.clone(), ov.clone()).await;

        // The recipient declines (an integrator's inbound message): now it waits for the sender.
        let declined = server
            .post("/api/referral-messages")
            .add_header(ih.clone(), iv.clone())
            .json(&json!({
                "referral_ref": rid, "event": "decline", "note": SECRET_NOTE,
                "message": SECRET_MESSAGE }))
            .await;
        assert_eq!(
            declined.status_code(),
            StatusCode::OK,
            "{}",
            declined.text()
        );
        assert_eq!(declined.json::<Value>()["result"], "moved");

        assert_eq!(count(sh.clone(), sv.clone()).await, before + 1);
        assert_eq!(count(oh.clone(), ov.clone()).await, other_before);
        let list = server
            .get("/api/referrals/worklist")
            .add_header(sh.clone(), sv.clone())
            .await;
        assert_eq!(list.status_code(), StatusCode::OK, "{}", list.text());
        assert!(!list.text().contains(SECRET_NOTE) && !list.text().contains(SECRET_MESSAGE));
        let body = list.json::<Value>();
        assert_eq!(body["limit"], 50);
        assert!(body["more"].is_boolean());
        let item = body["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["referral_id"] == rid.as_str())
            .expect("the declined referral is listed");
        assert_eq!(item["state"], "declined");
        assert_eq!(item["patient_id"], pid.as_str());
        let version = item["version"].as_i64().unwrap();
        let other_list = server
            .get("/api/referrals/worklist")
            .add_header(oh.clone(), ov.clone())
            .await
            .json::<Value>();
        assert!(
            !other_list["items"]
                .as_array()
                .unwrap()
                .iter()
                .any(|i| i["referral_id"] == rid.as_str())
        );

        // Who may read the list: not the integrator, receptionist or administrator.
        for (h, v) in [(&ih, &iv), (&xh, &xv), (&ah, &av)] {
            for path in ["/api/referrals/worklist", "/api/referrals/worklist/count"] {
                let r = server.get(path).add_header(h.clone(), v.clone()).await;
                assert_eq!(
                    r.status_code(),
                    StatusCode::FORBIDDEN,
                    "{path}: {}",
                    r.text()
                );
            }
            let r = server
                .post(&format!("/api/referrals/{rid}/acknowledge"))
                .add_header(h.clone(), v.clone())
                .json(&json!({ "version": version }))
                .await;
            assert_eq!(r.status_code(), StatusCode::FORBIDDEN, "{}", r.text());
        }

        // Acknowledge: someone else's referral is a miss, a stale version a conflict, the current one works once.
        let ack = |h: HeaderName, v: HeaderValue, version: i64| {
            let server = &server;
            let rid = rid.clone();
            async move {
                server
                    .post(&format!("/api/referrals/{rid}/acknowledge"))
                    .add_header(h, v)
                    .json(&json!({ "version": version }))
                    .await
            }
        };
        assert_eq!(
            ack(oh.clone(), ov.clone(), version).await.status_code(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            ack(sh.clone(), sv.clone(), version - 1).await.status_code(),
            StatusCode::CONFLICT
        );
        let done = ack(sh.clone(), sv.clone(), version).await;
        assert_eq!(done.status_code(), StatusCode::OK, "{}", done.text());
        assert_eq!(done.json::<Value>()["acknowledged"], true);
        assert_eq!(
            ack(sh.clone(), sv.clone(), version).await.status_code(),
            StatusCode::CONFLICT
        );
        assert_eq!(count(sh.clone(), sv.clone()).await, before);

        // The audit rows name ids and states, never the text.
        let rows = sqlx::query(
            "SELECT detail::text AS d FROM audit_events_all
                 WHERE resource_ref IN ('ReferralWorklist', 'ReferralMessage')",
        )
        .fetch_all(&ctx.pool())
        .await
        .unwrap();
        assert!(rows.len() >= 8, "every view and acknowledgement is audited");
        for row in rows {
            let d: String = row.try_get("d").unwrap();
            for secret in [SECRET_NOTE, SECRET_MESSAGE] {
                assert!(!d.contains(secret), "an audit row carried text: {d}");
            }
        }
    })
    .await;
}
