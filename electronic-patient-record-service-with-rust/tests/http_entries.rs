//! DB-gated, through the real router with login **on** (EH-1, EH-2, EH-3): who may record, read and retract a problem or
//! a relative history entry; that a patient and a proxy read none of either; that a read as at a time is decided by
//! **today's** access; that a relative cannot be named; and that the audit trail never holds a title, a condition, a
//! relative or a reason.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test main http_entries:: -- --ignored --test-threads=1
//! ```

use authentication_verifier::test_idp::{AUDIENCE, REALM, TestIdp};
use axa_test::prelude::request_with;
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

fn now() -> String {
    electronic_patient_record::clock::rfc3339_micros(jiff::Timestamp::now())
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn who_may_record_read_and_retract_a_problem_or_a_relative_history_entry() {
    let idp = TestIdp::start().await;
    let roles = [
        "clinical-admin",
        "clinician",
        "nurse",
        "pharmacist",
        "receptionist",
        "patient",
        "proxy",
    ];
    let map: Value = roles
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
    let token = |role: &str, n: u8| {
        idp.token(|c| {
            c["realm_access"] = json!({ "roles": [role] });
            c["sub"] = json!(format!("0c4f1e2a-0000-4000-8000-0000000000{n:02x}"));
        })
    };
    let sub = |n: u8| format!("0c4f1e2a-0000-4000-8000-0000000000{n:02x}");
    let admin = token("clinical-admin", 0xb0);
    let clinician = token("clinician", 0xb1);
    let nurse = token("nurse", 0xb2);
    let pharmacist = token("pharmacist", 0xb3);
    let receptionist = token("receptionist", 0xb4);
    let patient = token("patient", 0xb5);
    let proxy = token("proxy", 0xb6);

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
                let r = r.add_header(h, v).await;
                (r.status_code(), r.text())
            }
        };
        let as_patient = |token: &str, path: String| {
            let (h, v) = bearer(token);
            async move {
                server
                    .get(&path)
                    .add_header(h, v)
                    .add_header(
                        HeaderName::from_static("x-purpose-of-use"),
                        HeaderValue::from_static("patient-access"),
                    )
                    .await
                    .status_code()
            }
        };
        let person = |label: &str, seed: u128| {
            let family = format!("ent{label}{}", nanos());
            json!({ "patient": serde_json::to_value(Patient {
                names: vec![HumanName { family: Some(family.clone()), given: vec![family] }],
                birth_date: Some(dob(seed)),
                ..Default::default()
            }).unwrap() })
        };
        let mut ids = Vec::new();
        for (label, seed) in [("a", 0), ("b", 777)] {
            let (status, text) =
                call("POST", &admin, "/api/patients".into(), person(label, seed)).await;
            assert_eq!(status, StatusCode::CREATED, "{text}");
            let made: Value = serde_json::from_str(&text).unwrap();
            ids.push(made["id"].as_str().unwrap().to_string());
        }
        let (pid, stranger) = (ids[0].clone(), ids[1].clone());
        let expires = i64::try_from(nanos() / 1_000_000_000).unwrap() + 3600;
        for (n, kind, extra) in [
            (0xb1, "care-team", Value::Null),
            (0xb2, "care-team", Value::Null),
            (0xb3, "care-team", Value::Null),
            (0xb5, "self", Value::Null),
            (
                0xb6,
                "proxy",
                json!({ "basis": "parental-responsibility", "scopes": ["identity", "allergies"],
                        "expires_at": expires }),
            ),
        ] {
            let mut relationship = json!({ "subject": sub(n), "kind": kind, "status": "active" });
            if !extra.is_null() {
                relationship["proxy"] = extra;
            }
            let (status, text) = call(
                "POST",
                &admin,
                format!("/api/patients/{pid}/relationships"),
                json!({ "relationship": relationship, "reason": "test" }),
            )
            .await;
            assert_eq!(status, StatusCode::CREATED, "{text}");
        }

        let problem = |status: &str| {
            json!({ "title": "SECRETtitle", "certainty": "confirmed", "status": status,
                    "coding": { "system": "http://snomed.info/sct", "code": "22298006" } })
        };
        let relative = |status: &str| {
            json!({ "genetic": "aunt-uncle", "line": "paternal", "environment": "never", "sex": "female", "member": 1, "condition": "SECRETcondition",
                    "presence": "present", "source": "patient", "status": status })
        };
        type Make<'a> = &'a dyn Fn(&str) -> Value;
        let kinds: [(&str, &str, &str, Make, StatusCode); 2] = [
            ("problems", "active", "Condition", &problem, StatusCode::OK),
            (
                "relative-history",
                "recorded",
                "RelativeHistory",
                &relative,
                StatusCode::FORBIDDEN,
            ),
        ];
        for (kind, standing, resource, record, pharmacist_reads) in kinds {
            let url = format!("/api/patients/{pid}/{kind}");
            let post = |r: Value| json!({ "record": r, "reason": "SECRETreason" });
            // A nurse on the care team records one; a person with no clinical role may not.
            let (status, text) = call("POST", &nurse, url.clone(), post(record(standing))).await;
            assert_eq!(status, StatusCode::CREATED, "{text}");
            let made: Value = serde_json::from_str(&text).unwrap();
            let entry = format!("{url}/{}", made["id"].as_str().unwrap());
            for refused in [&receptionist, &pharmacist, &patient, &proxy] {
                let (status, _) = call("POST", refused, url.clone(), post(record(standing))).await;
                assert_eq!(status, StatusCode::FORBIDDEN, "{kind}");
            }
            // Reads: the list, as at now, and the versions, for the same people.
            let past = format!("{url}/as-at?time={}", now());
            for path in [url.clone(), past.clone(), format!("{entry}/versions")] {
                for (token, expected) in [
                    (&clinician, StatusCode::OK),
                    (&nurse, StatusCode::OK),
                    (&pharmacist, pharmacist_reads),
                    (&receptionist, StatusCode::FORBIDDEN),
                ] {
                    let (status, text) = call("GET", token, path.clone(), Value::Null).await;
                    assert_eq!(status, expected, "{path}: {text}");
                }
                for token in [&patient, &proxy] {
                    assert_eq!(
                        as_patient(token, path.clone()).await,
                        StatusCode::FORBIDDEN,
                        "{path}: a patient's view is not built"
                    );
                }
            }
            // No relationship with the stranger: refused, as at a past time too, with the emergency offer.
            for path in [
                format!("/api/patients/{stranger}/{kind}"),
                format!("/api/patients/{stranger}/{kind}/as-at?time={}", now()),
            ] {
                let (status, text) = call("GET", &clinician, path.clone(), Value::Null).await;
                assert_eq!(status, StatusCode::FORBIDDEN, "{path}");
                let body: Value = serde_json::from_str(&text).unwrap();
                assert_eq!(body["break_glass_offer"], true, "{path}");
            }
            // A bad time, or one to come, is a 400 and says nothing about the record.
            for bad in ["", "?time=", "?time=2026-01-01", "?time=2999-01-01T00:00:00Z"] {
                let (status, _) =
                    call("GET", &clinician, format!("{url}/as-at{bad}"), Value::Null).await;
                assert_eq!(status, StatusCode::BAD_REQUEST, "{bad}");
            }
            // A nurse may change an entry and may not retract it; a clinician may.
            let put = |r: Value, v: i32| {
                json!({ "record": r, "expected_version": v, "reason": "SECRETreason" })
            };
            let (status, text) =
                call("PUT", &nurse, entry.clone(), put(record(standing), 1)).await;
            assert_eq!(status, StatusCode::OK, "{text}");
            let mistake = put(record("entered-in-error"), 2);
            let (status, _) = call("PUT", &nurse, entry.clone(), mistake.clone()).await;
            assert_eq!(status, StatusCode::FORBIDDEN);
            let (status, text) = call("PUT", &clinician, entry.clone(), mistake).await;
            assert_eq!(status, StatusCode::OK, "{text}");
            // The list as it was before: one standing entry, which has changed since.
            let (_, text) = call("GET", &clinician, past, Value::Null).await;
            let then: Value = serde_json::from_str(&text).unwrap();
            assert_eq!(then["entries"][0]["version"], 1, "{text}");
            assert_eq!(then["entries"][0]["changed_since"], true);
            assert_ne!(then["summary"]["state"], "not-recorded");
            let (_, text) = call("GET", &clinician, url.clone(), Value::Null).await;
            let current: Value = serde_json::from_str(&text).unwrap();
            assert_eq!(current["summary"]["state"], "not-recorded", "{text}");
            // An unknown entry, and another patient's, are not found.
            for id in ["00000000-0000-4000-8000-000000000000", "nonsense"] {
                let (status, _) = call(
                    "GET",
                    &clinician,
                    format!("{url}/{id}/versions"),
                    Value::Null,
                )
                .await;
                assert_eq!(status, StatusCode::NOT_FOUND, "{id}");
            }
            // A refused record is 422 and its refusal is audited, by kind only.
            let (status, text) = call(
                "POST",
                &clinician,
                url.clone(),
                post(record("entered-in-error")),
            )
            .await;
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{text}");

            let rows = sqlx::query(AssertSqlSafe(format!(
                "SELECT detail::text AS d, outcome FROM audit_events WHERE resource_ref LIKE '{resource}%'
                     AND (patient_ref = 'Patient/{pid}' OR patient_ref IS NULL)"
            )))
            .fetch_all(&ctx.pool())
            .await
            .unwrap();
            assert!(rows.len() > 8, "{kind}: {}", rows.len());
            for r in &rows {
                let d: String = r.try_get::<Option<String>, _>("d").unwrap().unwrap_or_else(String::new);
                for word in ["SECRET", "22298006", "aunt", "paternal"] {
                    assert!(!d.contains(word), "{kind}: the audit trail holds {word}: {d}");
                }
            }
        }

        // A relative has no name, whatever the client sends.
        let mut named = relative("recorded");
        named["name"] = json!("Aunt Example");
        let (status, _) = call(
            "POST",
            &clinician,
            format!("/api/patients/{pid}/relative-history"),
            json!({ "record": named }),
        )
        .await;
        assert!(status.is_client_error(), "{status}");
        let (_, text) = call(
            "GET",
            &clinician,
            format!("/api/patients/{pid}/relative-history"),
            Value::Null,
        )
        .await;
        assert!(!text.contains("Aunt Example"));

        let newest: i64 = sqlx::query("SELECT coalesce(max(id), 0)::bigint AS n FROM audit_events")
            .fetch_one(&ctx.pool())
            .await
            .unwrap()
            .try_get("n")
            .unwrap();
        // A refused allergy is audited by kind: its words quote the code that was sent, and the trail must not.
        let (status, text) = call(
            "POST",
            &clinician,
            format!("/api/patients/{pid}/allergies"),
            json!({ "allergy": { "substance": "SECRETsubstance", "status": "active",
                    "coding": { "system": "http://snomed.info/sct", "code": "764146008" } } }),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{text}");
        let rows = sqlx::query("SELECT detail::text AS d FROM audit_events WHERE id > $1")
            .bind(newest)
            .fetch_all(&ctx.pool())
            .await
            .unwrap();
        assert!(!rows.is_empty(), "the refusal is audited");
        for r in &rows {
            let d: String = r.try_get::<Option<String>, _>("d").unwrap().unwrap_or_else(String::new);
            assert!(!d.contains("764146008"), "a code in the audit trail: {d}");
        }

        // AG-7: an answer as at a time says what it is and what it leaves out. A pending amendment is one such thing: the
        // list of now tells of it, and the reconstruction does not, and says that it does not.
        let (status, text) = call(
            "POST",
            &clinician,
            format!("/api/patients/{pid}/allergies"),
            json!({ "allergy": { "substance": "Latex", "status": "active" } }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{text}");
        let made: Value = serde_json::from_str(&text).unwrap();
        let (h, v) = bearer(&patient);
        let asked = server
            .post(&format!("/api/patients/{pid}/amendments"))
            .add_header(h, v)
            .add_header(
                HeaderName::from_static("x-purpose-of-use"),
                HeaderValue::from_static("patient-access"),
            )
            .json(&json!({ "kind": "AllergyIntolerance", "target_id": made["id"], "version": 1,
                           "text": "I am not allergic to this" }))
            .await;
        assert_eq!(asked.status_code(), StatusCode::CREATED, "{}", asked.text());
        let (_, now_text) = call("GET", &clinician, format!("/api/patients/{pid}/allergies"), Value::Null).await;
        let current: Value = serde_json::from_str(&now_text).unwrap();
        let id = made["id"].as_str().unwrap();
        assert_eq!(current["amendment_notices"][id][0]["notice"], "pending");
        for path in ["allergies", "problems", "relative-history"] {
            let (status, text) = call(
                "GET",
                &clinician,
                format!("/api/patients/{pid}/{path}/as-at?time={}", now()),
                Value::Null,
            )
            .await;
            assert_eq!(status, StatusCode::OK, "{text}");
            let then: Value = serde_json::from_str(&text).unwrap();
            assert_eq!(then["basis"], "reconstruction-of-stored-versions", "{path}");
            let left_out = then["left_out"].as_array().unwrap();
            for what in ["amendment-notices", "what-a-screen-showed", "whether-a-write-in-progress-had-finished"] {
                assert!(left_out.contains(&json!(what)), "{path}: {what}");
            }
            assert!(!text.contains("pending"), "{path}");
        }

        // The patient still reads their own allergies, as at a past time too; a proxy by its scope.
        for token in [&patient, &proxy] {
            assert_eq!(
                as_patient(
                    token,
                    format!("/api/patients/{pid}/allergies/as-at?time={}", now())
                )
                .await,
                StatusCode::OK
            );
        }
    })
    .await;
}
