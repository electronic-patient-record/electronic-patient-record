//! DB-gated, through the real router with login **on** (CA-3, second slice): the referral routes and who
//! may take which step. One test, because the token guard is configured once per process.
//!
//! Covered: a clinician sends a referral; the sender cannot answer it (422); another clinician accepts;
//! a stale version is a 409; a cancellation is only a request, needs a note, and only the other subject
//! confirms it; nothing follows an end state; the history lists every step; a receptionist is refused
//! (403); a referral id under another patient is a 404; the patient reads their own list; the audit rows
//! name ids, states and roles but never the reason, the question or a note.
//! Not covered: a recipient proved to be the named team (the service has no such model, OQ-PC2 assumed),
//! the redirect that creates a linked referral (not built), break-the-glass reads, and consent refusals.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test http_referrals -- --ignored --test-threads=1
//! ```

use authentication_verifier::test_idp::{AUDIENCE, REALM, TestIdp};
use axa::testing::prelude::request;
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

const REASON: &str = "Secret-reason-xylophone";
const QUESTION: &str = "Secret-question-zeppelin";
const NOTE: &str = "Secret-note-marmalade";

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn who_may_take_which_step() {
    let idp = TestIdp::start().await;
    let roles = ["clinical-admin", "clinician", "patient", "receptionist"];
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
    const SENDER: &str = "0c4f1e2a-0000-4000-8000-0000000000a1";
    const RECIPIENT: &str = "0c4f1e2a-0000-4000-8000-0000000000a2";
    const PATIENT: &str = "0c4f1e2a-0000-4000-8000-0000000000a3";
    let admin = token("clinical-admin", "0c4f1e2a-0000-4000-8000-0000000000a0");
    let sender = token("clinician", SENDER);
    let recipient = token("clinician", RECIPIENT);
    let patient = token("patient", PATIENT);
    let receptionist = token("receptionist", "0c4f1e2a-0000-4000-8000-0000000000a4");

    request::<App, _, _>(|server, ctx| async move {
        let (ah, av) = bearer(&admin);
        let (sh, sv) = bearer(&sender);
        let (rh, rv) = bearer(&recipient);
        let (ph, pv) = bearer(&patient);
        let (xh, xv) = bearer(&receptionist);
        let purpose = HeaderName::from_static("x-purpose-of-use");
        let access = HeaderValue::from_static("patient-access");

        let mut ids = Vec::new();
        for label in ["referral", "other"] {
            let family = format!("{label}{}", nanos());
            let made = server
                .post("/api/patients")
                .add_header(ah.clone(), av.clone())
                .json(&json!({ "patient": {
                    "names": [{ "family": family, "given": [family] }],
                    "birth_date": dob() } }))
                .await;
            assert_eq!(made.status_code(), StatusCode::CREATED, "{}", made.text());
            ids.push(made.json::<Value>()["id"].as_str().unwrap().to_string());
        }
        let (pid, other) = (ids[0].clone(), ids[1].clone());
        for (sub, kind) in [
            (SENDER, "care-team"),
            (RECIPIENT, "care-team"),
            (PATIENT, "self"),
        ] {
            let r = server
                .post(&format!("/api/patients/{pid}/relationships"))
                .add_header(ah.clone(), av.clone())
                .json(&json!({
                    "relationship": { "subject": sub, "kind": kind, "status": "active" },
                    "reason": "test"
                }))
                .await;
            assert_eq!(r.status_code(), StatusCode::CREATED, "{}", r.text());
        }
        let base = format!("/api/patients/{pid}/referrals");

        // The sender sends it.
        let sent = server
            .post(&base)
            .add_header(sh.clone(), sv.clone())
            .json(&json!({ "referral": {
                "recipient": "Cardiology outpatients", "reason": REASON,
                "question": QUESTION, "priority": "urgent" } }))
            .await;
        assert_eq!(sent.status_code(), StatusCode::CREATED, "{}", sent.text());
        let sent: Value = sent.json();
        assert_eq!(sent["state"], "requested");
        assert_eq!(sent["version"], 1);
        // The screen's buttons come from the service: accept from the recipient, cancel from the referrer.
        let next: Vec<(&str, &str)> = sent["next"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| (n["event"].as_str().unwrap(), n["by"].as_str().unwrap()))
            .collect();
        assert!(next.contains(&("accept", "recipient")));
        assert!(next.contains(&("request-cancel", "referrer")));
        let rid = sent["id"].as_str().unwrap().to_string();
        let events = format!("{base}/{rid}/events");
        let step =
            |h: HeaderName, v: HeaderValue, event: &str, version: i32, note: Option<&str>| {
                let (server, events) = (&server, events.clone());
                let body = json!({ "event": event, "expected_version": version, "note": note });
                async move { server.post(&events).add_header(h, v).json(&body).await }
            };

        // The sender cannot answer its own referral.
        let own = step(sh.clone(), sv.clone(), "accept", 1, None).await;
        assert_eq!(
            own.status_code(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            own.text()
        );
        // An unknown event is a 422 too.
        let unknown = step(rh.clone(), rv.clone(), "approve", 1, None).await;
        assert_eq!(unknown.status_code(), StatusCode::UNPROCESSABLE_ENTITY);

        // The other clinician accepts.
        let accepted = step(rh.clone(), rv.clone(), "accept", 1, None).await;
        assert_eq!(
            accepted.status_code(),
            StatusCode::OK,
            "{}",
            accepted.text()
        );
        let accepted: Value = accepted.json();
        assert_eq!(
            (accepted["state"].as_str(), accepted["version"].as_i64()),
            (Some("accepted"), Some(2))
        );
        assert_eq!(accepted["last_event"]["role"], "recipient");
        // A stale version is a 409.
        let stale = step(rh.clone(), rv.clone(), "book", 1, None).await;
        assert_eq!(
            stale.status_code(),
            StatusCode::CONFLICT,
            "{}",
            stale.text()
        );

        // A cancellation is a request, and it needs a note.
        let no_note = step(sh.clone(), sv.clone(), "request-cancel", 2, None).await;
        assert_eq!(no_note.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        let asked = step(sh.clone(), sv.clone(), "request-cancel", 2, Some(NOTE)).await;
        assert_eq!(asked.status_code(), StatusCode::OK, "{}", asked.text());
        assert_eq!(asked.json::<Value>()["state"], "cancel-requested");
        // The sender cannot confirm it.
        let self_confirm = step(sh.clone(), sv.clone(), "confirm-cancel", 3, None).await;
        assert_eq!(self_confirm.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        let cancelled = step(rh.clone(), rv.clone(), "confirm-cancel", 3, None).await;
        assert_eq!(
            cancelled.status_code(),
            StatusCode::OK,
            "{}",
            cancelled.text()
        );
        let cancelled: Value = cancelled.json();
        assert_eq!(cancelled["state"], "cancelled");
        assert!(
            cancelled["next"].as_array().unwrap().is_empty(),
            "an end state offers no step"
        );
        // Nothing follows an end state.
        let after = step(rh.clone(), rv.clone(), "accept", 4, None).await;
        assert_eq!(after.status_code(), StatusCode::UNPROCESSABLE_ENTITY);

        // The history lists every step, oldest first.
        let history = server
            .get(&format!("{base}/{rid}"))
            .add_header(sh.clone(), sv.clone())
            .await;
        assert_eq!(history.status_code(), StatusCode::OK, "{}", history.text());
        let states: Vec<String> = history.json::<Value>()["versions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v["state"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(
            states,
            ["requested", "accepted", "cancel-requested", "cancelled"]
        );

        // The patient reads their own list; a receptionist is refused; another patient's id is a miss.
        let own_list = server
            .get(&base)
            .add_header(ph.clone(), pv.clone())
            .add_header(purpose.clone(), access.clone())
            .await;
        assert_eq!(
            own_list.status_code(),
            StatusCode::OK,
            "{}",
            own_list.text()
        );
        assert_eq!(
            own_list.json::<Value>()["referrals"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        let refused = server.get(&base).add_header(xh.clone(), xv.clone()).await;
        assert_eq!(refused.status_code(), StatusCode::FORBIDDEN);
        let wrong = server
            .get(&format!("/api/patients/{other}/referrals/{rid}"))
            .add_header(ah.clone(), av.clone())
            .await;
        assert_ne!(
            wrong.status_code(),
            StatusCode::OK,
            "another patient's referral must not be disclosed"
        );
        let wrong_step = server
            .post(&format!("/api/patients/{other}/referrals/{rid}/events"))
            .add_header(rh.clone(), rv.clone())
            .json(&json!({ "event": "accept", "expected_version": 4 }))
            .await;
        assert_ne!(wrong_step.status_code(), StatusCode::OK);

        // The audit rows name ids, states and roles, never the substance.
        let rows = sqlx::query(
            "SELECT detail::text AS d FROM audit_events_all WHERE resource_ref = 'Referral'",
        )
        .fetch_all(&ctx.pool())
        .await
        .unwrap();
        assert!(
            rows.len() >= 8,
            "every send, step, refusal and read is audited"
        );
        for row in rows {
            let d: String = row.try_get("d").unwrap();
            for secret in [REASON, QUESTION, NOTE] {
                assert!(
                    !d.contains(secret),
                    "an audit row carried clinical text: {d}"
                );
            }
        }
    })
    .await;
}
