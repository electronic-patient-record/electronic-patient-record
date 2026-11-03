//! DB-gated, through the real router with login **on** (PC-2, hazard H-044): who may send an inbound referral
//! message and who may read and clear the queue. One test, because the token guard is configured once per
//! process.
//!
//! Covered: an integrator's outcome with a known id moves the referral; one with an unknown id, no id, or a
//! step the table refuses is queued (200, `queued`) and moves nothing; a clinician, a receptionist and the
//! administrator may not send; the administrator reads the count, the listing (no text) and one entry (with
//! the text), and clears it once (409 the second time, 422 for a match with no referral or a dismissal with
//! none); an integrator and a clinician may not read the queue (403); the audit rows hold ids, kinds and
//! states but never the message, a note or a reason. Assumed, awaiting the maintainer: which roles these are.
//! Not covered: a message over 1 MiB, concurrent senders, consent, or a referral id mistyped as another real one.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test http_referral_queue -- --ignored --test-threads=1
//! ```

use authentication_verifier::test_idp::{AUDIENCE, REALM, TestIdp};
use axum::http::{HeaderName, HeaderValue, StatusCode};
use electronic_patient_record::app::App;
use loco_rs::testing::prelude::request;
use sea_orm::{ConnectionTrait, Statement};
use serde_json::{Value, json};

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

const SECRET_MESSAGE: &str = "Secret-message-pelican";
const SECRET_NOTE: &str = "Secret-note-walrus";
const SECRET_REASON: &str = "Secret-reason-badger";

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn who_may_send_and_who_may_clear() {
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
    const CLINICIAN: &str = "0c4f1e2a-0000-4000-8000-0000000000b1";
    let admin = token("clinical-admin", "0c4f1e2a-0000-4000-8000-0000000000b0");
    let clinician = token("clinician", CLINICIAN);
    let integrator = token("integrator", "0c4f1e2a-0000-4000-8000-0000000000b2");
    let receptionist = token("receptionist", "0c4f1e2a-0000-4000-8000-0000000000b3");

    request::<App, _, _>(|server, ctx| async move {
        let (ah, av) = bearer(&admin);
        let (ch, cv) = bearer(&clinician);
        let (ih, iv) = bearer(&integrator);
        let (xh, xv) = bearer(&receptionist);

        // A patient with a referral to move.
        let family = format!("queue{}", nanos());
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
                "relationship": { "subject": CLINICIAN, "kind": "care-team", "status": "active" },
                "reason": "test" }))
            .await;
        assert_eq!(rel.status_code(), StatusCode::CREATED, "{}", rel.text());
        let sent = server
            .post(&format!("/api/patients/{pid}/referrals"))
            .add_header(ch.clone(), cv.clone())
            .json(&json!({ "referral": {
                "recipient": "Cardiology", "reason": "Chest pain",
                "question": "Please assess", "priority": "routine" } }))
            .await;
        assert_eq!(sent.status_code(), StatusCode::CREATED, "{}", sent.text());
        let rid = sent.json::<Value>()["id"].as_str().unwrap().to_string();

        let inbound = |h: HeaderName, v: HeaderValue, body: Value| {
            let server = &server;
            async move {
                server
                    .post("/api/referral-messages")
                    .add_header(h, v)
                    .json(&body)
                    .await
            }
        };

        // Only an integrator sends.
        let body = json!({ "referral_ref": rid, "event": "accept", "message": SECRET_MESSAGE });
        for (h, v) in [(&ch, &cv), (&xh, &xv), (&ah, &av)] {
            let r = inbound(h.clone(), v.clone(), body.clone()).await;
            assert_eq!(r.status_code(), StatusCode::FORBIDDEN, "{}", r.text());
        }
        // A known id moves the referral.
        let accepted = inbound(ih.clone(), iv.clone(), body).await;
        assert_eq!(
            accepted.status_code(),
            StatusCode::OK,
            "{}",
            accepted.text()
        );
        let accepted: Value = accepted.json();
        assert_eq!(
            (accepted["result"].as_str(), accepted["state"].as_str()),
            (Some("moved"), Some("accepted"))
        );
        assert!(
            !accepted.to_string().contains(SECRET_MESSAGE),
            "the message is not repeated"
        );
        // A blank message is a 422, not a queue entry.
        let blank = inbound(ih.clone(), iv.clone(), json!({ "message": "  " })).await;
        assert_eq!(blank.status_code(), StatusCode::UNPROCESSABLE_ENTITY);

        // Each of these is queued and moves nothing.
        let mut queued = Vec::new();
        for body in [
            json!({ "event": "outcome", "note": SECRET_NOTE, "message": SECRET_MESSAGE }),
            json!({ "referral_ref": "11111111-1111-4111-8111-111111111111", "event": "outcome",
                    "note": SECRET_NOTE, "message": SECRET_MESSAGE }),
            json!({ "referral_ref": rid, "event": "request-cancel", "note": SECRET_NOTE,
                    "message": SECRET_MESSAGE }),
        ] {
            let r = inbound(ih.clone(), iv.clone(), body).await;
            assert_eq!(r.status_code(), StatusCode::OK, "{}", r.text());
            let r: Value = r.json();
            assert_eq!(r["result"], "queued", "{r}");
            assert!(!r.to_string().contains(SECRET_MESSAGE));
            queued.push(r["queue_id"].as_str().unwrap().to_string());
        }
        let history = server
            .get(&format!("/api/patients/{pid}/referrals/{rid}"))
            .add_header(ch.clone(), cv.clone())
            .await;
        assert_eq!(
            history.json::<Value>()["versions"]
                .as_array()
                .unwrap()
                .len(),
            2
        );

        // Only the administrator reads the queue.
        for (h, v) in [(&ih, &iv), (&ch, &cv), (&xh, &xv)] {
            for path in ["/api/referral-queue", "/api/referral-queue/count"] {
                let r = server.get(path).add_header(h.clone(), v.clone()).await;
                assert_eq!(r.status_code(), StatusCode::FORBIDDEN, "{path}");
            }
        }
        let count = server
            .get("/api/referral-queue/count")
            .add_header(ah.clone(), av.clone())
            .await;
        assert!(count.json::<Value>()["open"].as_i64().unwrap() >= 3);
        let list = server
            .get("/api/referral-queue")
            .add_header(ah.clone(), av.clone())
            .await;
        assert_eq!(list.status_code(), StatusCode::OK, "{}", list.text());
        assert!(
            !list.text().contains(SECRET_MESSAGE),
            "a listing carries no message text"
        );
        let one = server
            .get(&format!("/api/referral-queue/{}", queued[0]))
            .add_header(ah.clone(), av.clone())
            .await;
        let one: Value = one.json();
        assert_eq!(one["message"], SECRET_MESSAGE);
        assert_eq!(one["kind"], "no-id");

        // Clearing: a match names a referral, a dismissal names none, a reason is needed, and it is once.
        let clear = |id: String, body: Value| {
            let (server, ah, av) = (&server, ah.clone(), av.clone());
            async move {
                server
                    .post(&format!("/api/referral-queue/{id}/clear"))
                    .add_header(ah, av)
                    .json(&body)
                    .await
            }
        };
        for body in [
            json!({ "how": "matched", "reason": "x" }),
            json!({ "how": "dismissed", "referral_id": rid, "reason": "x" }),
            json!({ "how": "dismissed", "reason": "  " }),
        ] {
            let r = clear(queued[0].clone(), body).await;
            assert_eq!(
                r.status_code(),
                StatusCode::UNPROCESSABLE_ENTITY,
                "{}",
                r.text()
            );
        }
        let denied = server
            .post(&format!("/api/referral-queue/{}/clear", queued[0]))
            .add_header(ch.clone(), cv.clone())
            .json(&json!({ "how": "dismissed", "reason": "x" }))
            .await;
        assert_eq!(denied.status_code(), StatusCode::FORBIDDEN);
        let cleared = clear(
            queued[0].clone(),
            json!({ "how": "matched", "referral_id": rid, "reason": SECRET_REASON }),
        )
        .await;
        assert_eq!(cleared.status_code(), StatusCode::OK, "{}", cleared.text());
        let cleared: Value = cleared.json();
        assert_eq!(cleared["cleared"]["how"], "matched");
        assert!(
            cleared.get("message").is_none(),
            "a write does not repeat the message"
        );
        let again = clear(
            queued[0].clone(),
            json!({ "how": "dismissed", "reason": "x" }),
        )
        .await;
        assert_eq!(again.status_code(), StatusCode::CONFLICT);

        // The audit rows name ids, kinds and states, never the text.
        let rows = ctx
            .db
            .query_all_raw(Statement::from_string(
                ctx.db.get_database_backend(),
                "SELECT detail::text AS d FROM audit_events_all
                 WHERE resource_ref IN ('ReferralMessage', 'ReferralQueue')"
                    .to_string(),
            ))
            .await
            .unwrap();
        assert!(
            rows.len() >= 10,
            "every receipt, view and clearance is audited"
        );
        for row in rows {
            let d: String = row.try_get("", "d").unwrap();
            for secret in [SECRET_MESSAGE, SECRET_NOTE, SECRET_REASON] {
                assert!(!d.contains(secret), "an audit row carried text: {d}");
            }
        }
    })
    .await;
}
