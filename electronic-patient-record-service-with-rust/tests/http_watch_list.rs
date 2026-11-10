//! DB-gated, through the real router with login **on** (WL-6): a watch list entry needs a reason; one
//! active entry per clinician and patient; only its owner ends it, with a reason; an ended or lapsed
//! entry leaves the owner's list and stays on the patient's record; the owner's list carries no reason;
//! a clinician without a care relationship, and reception, are refused; and no reason reaches the
//! audit. One test, because the token guard is configured once per process.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test http_watch_list -- --ignored --test-threads=1
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
#[ignore = "needs Postgres: set DB_URL"]
async fn a_watch_list_entry_has_a_reason_and_an_end_and_stays_in_the_history() {
    let idp = TestIdp::start().await;
    let roles = ["clinical-admin", "clinician", "receptionist"];
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
    const OWNER: &str = "0c4f1e2a-0000-4000-8000-0000000000e1";
    const COLLEAGUE: &str = "0c4f1e2a-0000-4000-8000-0000000000e2";
    let admin = token("clinical-admin", "0c4f1e2a-0000-4000-8000-0000000000e0");
    let owner = token("clinician", OWNER);
    let colleague = token("clinician", COLLEAGUE);
    let stranger = token("clinician", "0c4f1e2a-0000-4000-8000-0000000000e3");
    let receptionist = token("receptionist", "0c4f1e2a-0000-4000-8000-0000000000e4");

    request::<App, _, _>(|server, ctx| async move {
        let (ah, av) = bearer(&admin);
        let (oh, ov) = bearer(&owner);
        let (ch, cv) = bearer(&colleague);
        let (sh, sv) = bearer(&stranger);
        let (rh, rv) = bearer(&receptionist);
        let family = format!("watch{}", nanos());
        let made = server
            .post("/api/patients")
            .add_header(ah.clone(), av.clone())
            .json(&json!({ "patient": serde_json::to_value(Patient {
                names: vec![HumanName { family: Some(family.clone()), given: vec![family] }],
                birth_date: Some(dob()),
                ..Default::default()
            }).unwrap() }))
            .await;
        assert_eq!(made.status_code(), StatusCode::CREATED, "{}", made.text());
        let pid = made.json::<Value>()["id"].as_str().unwrap().to_string();
        for subject in [OWNER, COLLEAGUE] {
            let rel = server
                .post(&format!("/api/patients/{pid}/relationships"))
                .add_header(ah.clone(), av.clone())
                .json(&json!({ "relationship": { "subject": subject, "kind": "care-team", "status": "active" },
                               "reason": "test" }))
                .await;
            assert_eq!(rel.status_code(), StatusCode::CREATED, "{}", rel.text());
        }
        let url = format!("/api/patients/{pid}/watch-list");

        // A reason is required, and an end day that has passed is refused.
        for entry in [json!({ "reason": " " }), json!({ "reason": "r", "until": "2000-01-01" })] {
            let r = server
                .post(&url)
                .add_header(oh.clone(), ov.clone())
                .json(&json!({ "entry": entry }))
                .await;
            assert_eq!(r.status_code(), StatusCode::UNPROCESSABLE_ENTITY, "{entry}: {}", r.text());
        }
        let added = server
            .post(&url)
            .add_header(oh.clone(), ov.clone())
            .json(&json!({ "entry": { "reason": "Sodium 128 after discharge; recheck Friday" } }))
            .await;
        assert_eq!(added.status_code(), StatusCode::CREATED, "{}", added.text());
        let entry: Value = added.json();
        let entry_id = entry["id"].as_str().unwrap().to_string();
        assert_eq!(entry["state"], "active");
        let twice = server
            .post(&url)
            .add_header(oh.clone(), ov.clone())
            .json(&json!({ "entry": { "reason": "again" } }))
            .await;
        assert_eq!(twice.status_code(), StatusCode::CONFLICT, "{}", twice.text());

        // No care relationship, or reception: refused.
        let r = server
            .post(&url)
            .add_header(sh.clone(), sv.clone())
            .json(&json!({ "entry": { "reason": "curious" } }))
            .await;
        assert_eq!(r.status_code(), StatusCode::FORBIDDEN, "{}", r.text());
        assert_eq!(
            server.get(&url).add_header(rh.clone(), rv.clone()).await.status_code(),
            StatusCode::FORBIDDEN
        );

        // The owner's own list: a pointer, no reason.
        let own = |h: HeaderName, v: HeaderValue| {
            let server = &server;
            async move { server.get("/api/watch-list").add_header(h, v).await.json::<Value>() }
        };
        let mine = own(oh.clone(), ov.clone()).await;
        let item = mine["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["id"] == json!(entry_id))
            .unwrap_or_else(|| panic!("on the owner's list: {mine}"))
            .clone();
        assert!(item.get("reason").is_none(), "no reason on the own list: {item}");
        assert_eq!(item["patient_id"], json!(pid));

        // Only the owner ends it, and only with a reason.
        let end_url = format!("{url}/{entry_id}");
        let by_colleague = server
            .put(&end_url)
            .add_header(ch.clone(), cv.clone())
            .json(&json!({ "expected_version": 1, "end_reason": "not mine" }))
            .await;
        assert_eq!(by_colleague.status_code(), StatusCode::NOT_FOUND, "{}", by_colleague.text());
        for (body, status) in [
            (json!({ "expected_version": 1 }), StatusCode::UNPROCESSABLE_ENTITY),
            (json!({ "expected_version": 2, "end_reason": "sodium normal" }), StatusCode::CONFLICT),
            (json!({ "expected_version": 1, "end_reason": "sodium normal" }), StatusCode::OK),
            (json!({ "expected_version": 2, "end_reason": "again" }), StatusCode::UNPROCESSABLE_ENTITY),
        ] {
            let r = server.put(&end_url).add_header(oh.clone(), ov.clone()).json(&body).await;
            assert_eq!(r.status_code(), status, "{body}: {}", r.text());
        }

        // A lapsed entry: written as it would have been added a while ago, with a last day now passed.
        sqlx::query(
            "INSERT INTO watch_entry_versions (entry_id, version, owner, patient_id, author, status, body)
             VALUES (gen_random_uuid(), 1, $1, $2::uuid, $1, 'active',
                     jsonb_build_object('reason', 'post-op wound check', 'until', '2001-01-01'))",
        )
        .bind(COLLEAGUE)
        .bind(&pid)
        .execute(&ctx.pool())
        .await
        .unwrap();

        // Ended and lapsed entries leave the owners' lists and stay on the patient's record.
        assert!(
            !own(oh.clone(), ov.clone()).await["items"]
                .as_array()
                .unwrap()
                .iter()
                .any(|i| i["patient_id"] == json!(pid)),
            "ended: off the owner's list"
        );
        assert!(
            !own(ch.clone(), cv.clone()).await["items"]
                .as_array()
                .unwrap()
                .iter()
                .any(|i| i["patient_id"] == json!(pid)),
            "lapsed: off the colleague's list"
        );
        let history: Value = server.get(&url).add_header(ch.clone(), cv.clone()).await.json();
        let states: Vec<&str> = history["entries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["state"].as_str().unwrap())
            .collect();
        assert_eq!(states.len(), 2, "{history}");
        assert!(states.contains(&"ended") && states.contains(&"lapsed"), "{states:?}");

        // The owner learns of a lapse: it is on their own `lapsed` list until they end it with a reason.
        let gone = history["entries"].as_array().unwrap().iter()
            .find(|e| e["state"] == "lapsed").unwrap().clone();
        let theirs = own(ch.clone(), cv.clone()).await;
        assert!(
            theirs["lapsed"].as_array().unwrap().iter().any(|i| i["id"] == gone["id"]),
            "lapsed: on the colleague's lapsed list: {theirs}"
        );
        assert!(
            !own(oh.clone(), ov.clone()).await["lapsed"].as_array().unwrap().iter()
                .any(|i| i["id"] == gone["id"]),
            "someone else's lapse is not on mine"
        );
        let r = server
            .put(&format!("{url}/{}", gone["id"].as_str().unwrap()))
            .add_header(ch.clone(), cv.clone())
            .json(&json!({ "expected_version": gone["version"], "end_reason": "wound healed" }))
            .await;
        assert_eq!(r.status_code(), StatusCode::OK, "a lapsed entry can be ended: {}", r.text());
        assert!(
            !own(ch.clone(), cv.clone()).await["lapsed"].as_array().unwrap().iter()
                .any(|i| i["id"] == gone["id"]),
            "ended: off the lapsed list"
        );
        let ended = history["entries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["state"] == "ended")
            .unwrap();
        assert_eq!(ended["end_reason"], "sodium normal");
        assert_eq!(ended["reason"], "Sodium 128 after discharge; recheck Friday");

        // The trail holds ids, versions and states, never a reason.
        let rows = sqlx::query(AssertSqlSafe(format!(
            "SELECT detail::text AS d FROM audit_events
             WHERE resource_ref IN ('WatchList', 'OwnWatchList')
               AND (patient_ref = 'Patient/{pid}' OR patient_ref IS NULL)
               AND recorded_at > now() - interval '10 minutes'"
        )))
        .fetch_all(&ctx.pool())
        .await
        .unwrap();
        assert!(rows.len() >= 5, "adds, an end and reads: {}", rows.len());
        for row in rows {
            let d: String = row.try_get("d").unwrap();
            for secret in ["Sodium", "sodium", "Friday", "wound", "curious"] {
                assert!(!d.contains(secret), "{secret} leaked into the audit row: {d}");
            }
        }
    })
    .await;
}
