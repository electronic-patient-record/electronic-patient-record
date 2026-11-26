//! DB-gated, through the real router with login **on** (EH-8): on each route of the three lists, that a labelled entry does
//! not appear to someone who may not see it; who may add and lift a label; that break-the-glass shows it and is flagged;
//! and that the audit trail names no label and holds no reason.
//!
//! ```text
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test main http_entry_labels:: -- --ignored --test-threads=1
//! ```

use authentication_verifier::test_idp::{AUDIENCE, REALM, TestIdp};
use axa_test::prelude::request_with;
use axum::http::{HeaderName, HeaderValue, StatusCode};
use electronic_patient_record::app::App;
use electronic_patient_record::pool::Pool;
use electronic_patient_record_matcher::{HumanName, Patient};
use serde_json::{Value, json};
use sqlx::Row;

fn nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

/// A birth date no earlier run shares, so the duplicate check does not shortlist their patients.
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

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_labelled_entry_does_not_appear_and_who_may_label_and_what_break_the_glass_shows() {
    let idp = TestIdp::start().await;
    let map: Value = ["clinical-admin", "clinician", "nurse", "pharmacist"]
        .iter()
        .map(|r| ((*r).to_string(), json!({ "role": [r] })))
        .collect();
    let db_url = std::env::var("DB_URL").expect("DB_URL must be set for --ignored tests");
    let role_map = map.to_string();
    let settings: Vec<(&str, &str)> = vec![
        ("DB_URL", &db_url),
        ("EPR_REQUIRE_AUTH", "true"),
        ("EPR_KEYCLOAK_URL", &idp.base_url),
        ("EPR_KEYCLOAK_REALM", REALM),
        ("EPR_KEYCLOAK_AUDIENCES", AUDIENCE),
        ("EPR_KEYCLOAK_ROLE_MAP", &role_map),
    ];
    let sub = |n: u8| format!("0c4f1e2a-0000-4000-8000-00000000f1{n:02x}");
    let token = |role: &str, n: u8| {
        idp.token(|c| {
            c["realm_access"] = json!({ "roles": [role] });
            c["sub"] = json!(sub(n));
        })
    };
    let admin = token("clinical-admin", 0);
    let (doctor, other) = (token("clinician", 1), token("clinician", 2));
    // A nurse on the team, one who will break the glass on the team, and one with no relationship.
    let (nurse, urgent, stranger) = (token("nurse", 3), token("nurse", 4), token("nurse", 5));

    request_with::<App, _, _>(&settings, |server, ctx| async move {
        let server = &server;
        let call = |method: &'static str, token: &str, path: String, body: Value| {
            let (h, v) = bearer(token);
            async move {
                let r = match method {
                    "GET" => server.get(&path),
                    "POST" => server.post(&path).json(&body),
                    _ => server.put(&path).json(&body),
                };
                let r = r
                    .add_header(h, v)
                    .add_header(
                        HeaderName::from_static("x-purpose-of-use"),
                        HeaderValue::from_static("treatment"),
                    )
                    .await;
                (r.status_code(), r.text())
            }
        };
        let get = |token: &str, path: String| call("GET", token, path, Value::Null);
        let family = format!("lab{}", nanos());
        let (status, text) = call("POST", &admin, "/api/patients".into(),
            json!({ "patient": serde_json::to_value(Patient {
                names: vec![HumanName { family: Some(family.clone()), given: vec![family] }],
                birth_date: Some(dob(3)),
                ..Default::default()
            }).unwrap() })).await;
        assert_eq!(status, StatusCode::CREATED, "{text}");
        let made: Value = serde_json::from_str(&text).unwrap();
        let pid = made["id"].as_str().unwrap().to_string();
        for n in 1..=4 {
            let (status, text) = call("POST", &admin, format!("/api/patients/{pid}/relationships"),
                json!({ "relationship": { "subject": sub(n), "kind": "care-team", "status": "active" }, "reason": "test" })).await;
            assert_eq!(status, StatusCode::CREATED, "{text}");
        }
        for token in [&urgent, &stranger] {
            let (status, text) = call("POST", token, format!("/api/patients/{pid}/break-glass"), json!({ "reason": "collapsed", "emergency": { "threat": "life", "time_to_harm": "minutes", "can_be_asked": "no" } })).await;
            assert_eq!(status, StatusCode::CREATED, "{text}");
        }
        let kinds = [
            ("problems", "nothing", json!({ "title": "QUOKKA", "certainty": "confirmed", "status": "active" })),
            ("relative-history", "nothing", json!({ "genetic": "full-sibling", "environment": "childhood",
                "condition": "QUOKKA", "presence": "present", "source": "patient", "status": "recorded" })),
            ("compatibility-typing", "abo", json!({ "finding": { "kind": "abo", "group": "o" }, "laboratory": "QUOKKA",
                "sample": "S1", "collected_at": "2026-11-20T09:00:00Z", "status": "reported" })),
        ];
        for (path, agrees, record) in kinds {
            let url = format!("/api/patients/{pid}/{path}");
            let (status, text) = call("POST", &doctor, url.clone(), json!({ "record": record })).await;
            assert_eq!(status, StatusCode::CREATED, "{text}");
            let made: Value = serde_json::from_str(&text).unwrap();
            let id = made["id"].as_str().unwrap().to_string();
            let at_first = electronic_patient_record::clock::rfc3339_micros(jiff::Timestamp::now());
            let one = format!("{url}/{id}");
            let labels = format!("{one}/labels");
            let change = |act: &str, reason: Option<&str>| {
                let mut body = json!({ "act": act, "label": "V", "basis": "clinical" });
                if let Some(r) = reason {
                    body["reason"] = json!(r);
                }
                body
            };

            // With a grant and nothing hidden, the grant is not used: the count below leaves this read out.
            assert_eq!(get(&urgent, url.clone()).await.0, StatusCode::OK);
            // A nurse on the team sees it and may not label it. A pharmacist has no relationship.
            let (_, text) = get(&nurse, url.clone()).await;
            assert!(text.contains("QUOKKA") && text.contains("\"whole\""), "{text}");
            assert_eq!(call("POST", &nurse, labels.clone(), change("add", None)).await.0, StatusCode::FORBIDDEN);
            // Nor a stranger who broke the glass: that is for reading.
            assert_eq!(call("POST", &stranger, labels.clone(), change("add", None)).await.0, StatusCode::FORBIDDEN);
            // A person does not say custody, or an unknown label.
            for (key, value) in [("basis", "custody"), ("label", "v")] {
                let mut body = change("add", None);
                body[key] = json!(value);
                let (status, text) = call("POST", &doctor, labels.clone(), body).await;
                assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{text}");
            }
            assert_eq!(call("POST", &doctor, labels.clone(), change("add", None)).await.0, StatusCode::OK);

            // It is gone for the nurse from every route, as at a time before the label too.
            let reads = [url.clone(), format!("{url}/as-at?time={at_first}")];
            for at in &reads {
                let (status, text) = get(&nurse, at.clone()).await;
                assert_eq!(status, StatusCode::OK, "{text}");
                assert!(!text.contains("QUOKKA") && !text.contains(&id), "{at}: {text}");
                let body: Value = serde_json::from_str(&text).unwrap();
                assert_eq!((&body["completeness"], &body["withheld"]), (&json!("partial"), &json!(1)), "{at}");
                assert_eq!(body["summary"]["state"], "not-stated", "{at}");
                if agrees == "abo" {
                    assert_eq!(body["summary"]["abo"], "not-stated");
                }
            }
            for at in [format!("{one}/versions"), labels.clone()] {
                assert_eq!(get(&nurse, at).await.0, StatusCode::NOT_FOUND);
            }
            // A write is answered as for an id that is not there (a nurse may write a problem and not a typing).
            let mut said = Vec::new();
            for id in [id.as_str(), "00000000-0000-4000-8000-000000000000"] {
                let body = json!({ "record": record, "expected_version": 9, "reason": "r" });
                said.push(call("PUT", &nurse, format!("{url}/{id}"), body).await);
            }
            assert_eq!(said[0], said[1]);
            if path == "problems" {
                assert_eq!(said[0].0, StatusCode::NOT_FOUND, "{}", said[0].1);
            }

            // Break-the-glass shows it, to a nurse on the team and to a stranger, and is flagged.
            for (token, n) in [(&urgent, 4), (&stranger, 5)] {
                for at in &reads {
                    let (_, text) = get(token, at.clone()).await;
                    assert!(text.contains("QUOKKA") && text.contains("\"whole\""), "{n} {at}: {text}");
                }
                for at in [format!("{one}/versions"), labels.clone()] {
                    assert_eq!(get(token, at).await.0, StatusCode::OK);
                }
                let flagged: i64 = sqlx::query(
                    "SELECT count(*) AS n FROM audit_events WHERE agent = $1 AND patient_ref = $2 AND action = 'R'
                     AND outcome = 0 AND break_glass AND resource_ref LIKE $3")
                    .bind(sub(n)).bind(format!("Patient/{pid}")).bind(match path {
                        "problems" => "Condition%", "relative-history" => "%elative%", _ => "%yping%" })
                    .fetch_one(&ctx.pool()).await.unwrap().get("n");
                assert_eq!(flagged, 4, "{n} {path}: every read that showed it");
            }
            // Where nothing was hidden from the first nurse, nothing is flagged.
            let flagged: i64 = sqlx::query("SELECT count(*) AS n FROM audit_events WHERE agent = $1 AND patient_ref = $2 AND break_glass")
                .bind(sub(3)).bind(format!("Patient/{pid}"))
                .fetch_one(&ctx.pool()).await.unwrap().get("n");
            assert_eq!(flagged, 0);

            // Lifting: not by the one who added it, not by a nurse who sees it in an emergency, and with a reason.
            assert_eq!(call("POST", &doctor, labels.clone(), change("lift", Some("r"))).await.0, StatusCode::FORBIDDEN);
            assert_eq!(call("POST", &urgent, labels.clone(), change("lift", Some("r"))).await.0, StatusCode::FORBIDDEN);
            assert_eq!(call("POST", &other, labels.clone(), change("lift", Some("  "))).await.0, StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(call("POST", &other, labels.clone(), change("lift", None)).await.0, StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(call("POST", &other, labels.clone(), change("add", None)).await.0, StatusCode::CONFLICT);
            assert_eq!(call("POST", &other, labels.clone(), change("lift", Some("WOMBAT"))).await.0, StatusCode::OK);
            let (_, text) = get(&nurse, labels.clone()).await;
            let acts: Value = serde_json::from_str(&text).unwrap();
            assert_eq!(acts["acts"][0]["by"], json!(sub(1)));
            assert_eq!((&acts["acts"][1]["by"], &acts["acts"][1]["reason"]), (&json!(sub(2)), &json!("WOMBAT")));
            let (_, text) = get(&nurse, url.clone()).await;
            assert!(text.contains("QUOKKA") && text.contains("\"whole\""), "{text}");
        }
        // The audit trail: that an entry's labels changed, how many were withheld, and no label, reason or content.
        let rows: Vec<String> = sqlx::query("SELECT detail::text AS d FROM audit_events WHERE patient_ref = $1 AND detail IS NOT NULL")
            .bind(format!("Patient/{pid}"))
            .fetch_all(&ctx.pool()).await.unwrap().iter().map(|r| r.get("d")).collect();
        let all = rows.join("\n");
        assert!(all.contains("\"withheld\": 1") && all.contains("\"labels\": \"changed\""), "{all}");
        for secret in ["QUOKKA", "WOMBAT", "\"V\"", "clinical\""] {
            assert!(!all.contains(secret), "{secret}: {all}");
        }
    })
    .await;
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_consent_that_names_a_label_decides_for_the_entries_that_carry_it() {
    let idp = TestIdp::start().await;
    let map: Value = ["clinical-admin", "clinician", "nurse", "pharmacist"]
        .iter()
        .map(|r| ((*r).to_string(), json!({ "role": [r] })))
        .collect();
    let db_url = std::env::var("DB_URL").expect("DB_URL must be set for --ignored tests");
    let role_map = map.to_string();
    let settings: Vec<(&str, &str)> = vec![
        ("DB_URL", &db_url),
        ("EPR_REQUIRE_AUTH", "true"),
        ("EPR_KEYCLOAK_URL", &idp.base_url),
        ("EPR_KEYCLOAK_REALM", REALM),
        ("EPR_KEYCLOAK_AUDIENCES", AUDIENCE),
        ("EPR_KEYCLOAK_ROLE_MAP", &role_map),
    ];
    let sub = |n: u8| format!("0c4f1e2a-0000-4000-8000-00000000f1{n:02x}");
    let token = |role: &str, n: u8| {
        idp.token(|c| {
            c["realm_access"] = json!({ "roles": [role] });
            c["sub"] = json!(sub(n));
        })
    };
    let admin = token("clinical-admin", 0x10);
    let (doctor, urgent) = (token("clinician", 0x11), token("clinician", 0x12));

    request_with::<App, _, _>(&settings, |server, _ctx| async move {
        let server = &server;
        let call = |method: &'static str, token: &str, path: String, body: Value| {
            let (h, v) = bearer(token);
            async move {
                let r = match method {
                    "GET" => server.get(&path),
                    "POST" => server.post(&path).json(&body),
                    _ => server.put(&path).json(&body),
                };
                let r = r
                    .add_header(h, v)
                    .add_header(
                        HeaderName::from_static("x-purpose-of-use"),
                        HeaderValue::from_static("treatment"),
                    )
                    .await;
                (r.status_code(), r.text())
            }
        };
        let get = |token: &str, path: String| call("GET", token, path, Value::Null);
        let family = format!("con{}", nanos());
        let (status, text) = call(
            "POST",
            &admin,
            "/api/patients".into(),
            json!({ "patient": serde_json::to_value(Patient {
                names: vec![HumanName { family: Some(family.clone()), given: vec![family] }],
                birth_date: Some(dob(5)),
                ..Default::default()
            }).unwrap() }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{text}");
        let made: Value = serde_json::from_str(&text).unwrap();
        let pid = made["id"].as_str().unwrap().to_string();
        for n in [0x11, 0x12] {
            let (status, text) = call(
                "POST",
                &admin,
                format!("/api/patients/{pid}/relationships"),
                json!({ "relationship": { "subject": sub(n), "kind": "care-team", "status": "active" }, "reason": "test" }),
            )
            .await;
            assert_eq!(status, StatusCode::CREATED, "{text}");
        }
        let url = format!("/api/patients/{pid}/problems");
        for (title, label) in [("WALLABY", "SUD"), ("BANDICOOT", "HIV")] {
            let record = json!({ "title": title, "certainty": "confirmed", "status": "active" });
            let (_, text) = call("POST", &doctor, url.clone(), json!({ "record": record })).await;
            let made: Value = serde_json::from_str(&text).unwrap();
            let (status, text) = call(
                "POST",
                &doctor,
                format!("{url}/{}/labels", made["id"].as_str().unwrap()),
                json!({ "act": "add", "label": label, "basis": "patient-asked" }),
            )
            .await;
            assert_eq!(status, StatusCode::OK, "{text}");
        }
        let sees = async |token: &str| {
            let (status, text) = get(token, url.clone()).await;
            assert_eq!(status, StatusCode::OK, "{text}");
            (text.contains("WALLABY"), text.contains("BANDICOOT"))
        };
        // Substance use needs the patient's consent, for the one who labelled it too.
        assert_eq!(sees(&doctor).await, (false, true));
        // In an emergency it does not.
        let (status, text) = call(
            "POST",
            &urgent,
            format!("/api/patients/{pid}/break-glass"),
            json!({ "reason": "collapsed", "emergency": { "threat": "life", "time_to_harm": "minutes", "can_be_asked": "no" } }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{text}");
        assert_eq!(sees(&urgent).await, (true, true));

        let consent = async |effect: &str, label: &str| {
            let (status, text) = call(
                "POST",
                &admin,
                format!("/api/patients/{pid}/consents"),
                json!({ "consent": { "kind": "general", "status": "active", "provisions": [{ "effect": effect,
                    "purposes": ["treatment"], "labels": [label] }] }, "reason": "asked" }),
            )
            .await;
            assert_eq!(status, StatusCode::CREATED, "{text}");
        };
        consent("permit", "SUD").await;
        assert_eq!(sees(&doctor).await, (true, true));
        // A refusal for one label withholds the entries with it and no other, in an emergency too (OQ-PI8).
        consent("deny", "HIV").await;
        assert_eq!(sees(&doctor).await, (true, false));
        assert_eq!(sees(&urgent).await, (true, false));
        let (_, text) = get(&doctor, url.clone()).await;
        assert!(text.contains("\"withheld\":1"), "{text}");
    })
    .await;
}
