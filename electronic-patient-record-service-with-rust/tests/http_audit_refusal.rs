//! DB-gated, through the real router with login off (CQ-39): when a write's audit row cannot be written, the
//! write is refused. A trigger on `audit_events` is switched on only around one request, so that request's audit
//! insert fails; the answer must be 500 and no store may gain a row. Then the same request is sent with audit
//! working and must succeed, so the 500 is proved to come from the audit row and not from a bad request.
//!
//! Covered here (login off): patients, FHIR create and update, consent, relationship, allergy, the allergy
//! statement, photo put and withdraw, merge and unmerge. Break-glass and the feed queue need login on, so they
//! are in `http_audit_refusal_roles.rs`.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test http_audit_refusal -- --ignored --test-threads=1
//! ```

use axum::http::{HeaderName, HeaderValue, StatusCode};
use electronic_patient_record::app::App;
use electronic_patient_record_matcher::{HumanName, Patient};
use loco_rs::testing::prelude::request;
use serde_json::{Value, json};

mod audit_switch;
use audit_switch::{DRIVEN, LOGIN_OFF, LOGIN_ON, install, refused_then_accepted, uninstall};

const PNG: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 1, 2, 3];

/// A read in write clothing: it audits, but it changes nothing.
const NOT_A_WRITE: [&str; 1] = ["POST /api/patients/compare"];

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
    format!("{label}{letters}")
}

fn patient(family: &str, seed: u128) -> Value {
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
        / 1000
        + seed;
    serde_json::to_value(Patient {
        names: vec![HumanName {
            family: Some(family.into()),
            given: vec![family.into()],
        }],
        birth_date: Some(format!(
            "{}-{:02}-{:02}",
            1900 + n % 100,
            1 + n / 100 % 12,
            1 + n / 1200 % 28
        )),
        ..Default::default()
    })
    .unwrap()
}

fn login_off() {
    // SAFETY: set before any thread of this test reads it, and the suite runs single-threaded.
    unsafe { std::env::set_var("EPR_REQUIRE_AUTH", "false") };
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_write_whose_audit_row_cannot_be_written_is_refused_on_every_covered_route() {
    login_off();
    request::<App, _, _>(|server, ctx| async move {
        let db = &ctx.db;
        install(db).await;

        // Two real patients to work on, created with audit working.
        let mut ids = Vec::new();
        for seed in [0, 7_000_000] {
            let made = server
                .post("/api/patients")
                .json(&json!({ "patient": patient(&tag("aud"), seed) }))
                .await;
            assert_eq!(made.status_code(), StatusCode::CREATED, "{}", made.text());
            ids.push(made.json::<Value>()["id"].as_str().unwrap().to_string());
        }
        let id = ids[0].clone();
        let other = ids[1].clone();

        // Patient create (API), and update.
        let new_patient = patient(&tag("audnew"), 14_000_000);
        refused_then_accepted(db, "POST /api/patients", || async {
            let r = server
                .post("/api/patients")
                .json(&json!({ "patient": new_patient }))
                .await;
            (r.status_code(), r.text())
        })
        .await;
        let stored = server
            .get(&format!("/api/patients/{id}"))
            .await
            .json::<Value>();
        let mut changed = stored["patient"].clone();
        changed["birth_date"] = stored["patient"]["birth_date"].clone();
        changed["telecom"] = json!([{ "system": "phone", "value": "01632 960001" }]);
        refused_then_accepted(db, "PUT /api/patients/{id}", || async {
            let r = server
                .put(&format!("/api/patients/{id}"))
                .json(&json!({ "patient": changed, "expected_version": 1 }))
                .await;
            (r.status_code(), r.text())
        })
        .await;

        // FHIR create.
        let fhir = {
            let n = patient(&tag("audfhir"), 21_000_000);
            json!({
                "resourceType": "Patient",
                "name": [{ "family": n["names"][0]["family"], "given": n["names"][0]["given"] }],
                "birthDate": n["birth_date"],
            })
        };
        refused_then_accepted(db, "POST /fhir/R4/Patient", || async {
            let r = server.post("/fhir/R4/Patient").json(&fhir).await;
            (r.status_code(), r.text())
        })
        .await;

        // FHIR update, with If-Match.
        let fhir_read = server
            .get(&format!("/fhir/R4/Patient/{other}"))
            .await
            .json::<Value>();
        let mut fhir_changed = fhir_read.clone();
        fhir_changed["telecom"] = json!([{ "system": "phone", "value": "01632 960002" }]);
        refused_then_accepted(db, "PUT /fhir/R4/Patient/{id}", || async {
            let r = server
                .put(&format!("/fhir/R4/Patient/{other}"))
                .add_header(
                    HeaderName::from_static("if-match"),
                    HeaderValue::from_static("W/\"1\""),
                )
                .json(&fhir_changed)
                .await;
            (r.status_code(), r.text())
        })
        .await;

        // Allergy create, update, and the statement.
        let allergy = json!({ "substance": "penicillin", "status": "active" });
        refused_then_accepted(db, "POST /api/patients/{id}/allergies", || async {
            let r = server
                .post(&format!("/api/patients/{id}/allergies"))
                .json(&json!({ "allergy": allergy }))
                .await;
            (r.status_code(), r.text())
        })
        .await;
        let listed = server
            .get(&format!("/api/patients/{id}/allergies"))
            .await
            .json::<Value>();
        let allergy_id = listed["allergies"][0]["id"]
            .as_str()
            .expect("an allergy id")
            .to_string();
        let ended = json!({ "substance": "penicillin", "status": "inactive" });
        refused_then_accepted(
            db,
            "PUT /api/patients/{id}/allergies/{allergy_id}",
            || async {
                let r = server
                    .put(&format!("/api/patients/{id}/allergies/{allergy_id}"))
                    .json(&json!({ "allergy": ended, "expected_version": 1 }))
                    .await;
                (r.status_code(), r.text())
            },
        )
        .await;
        // The statement needs no active allergy: the other patient has none.
        refused_then_accepted(db, "PUT /api/patients/{id}/allergy-statement", || async {
            let r = server
                .put(&format!("/api/patients/{other}/allergy-statement"))
                .json(&json!({ "none_known": true, "expected_version": 0 }))
                .await;
            (r.status_code(), r.text())
        })
        .await;

        // Observation create and update.
        let observation = json!({
            "code": "8867-4", "value": 72.0, "unit": "/min",
            "effective_at": "2026-10-26T09:30:00Z", "status": "final"
        });
        refused_then_accepted(db, "POST /api/patients/{id}/observations", || async {
            let r = server
                .post(&format!("/api/patients/{id}/observations"))
                .json(&json!({ "observation": observation }))
                .await;
            (r.status_code(), r.text())
        })
        .await;
        let listed = server
            .get(&format!("/api/patients/{id}/observations"))
            .await
            .json::<Value>();
        let observation_id = listed["observations"][0]["id"]
            .as_str()
            .expect("an observation id")
            .to_string();
        let mistaken = json!({
            "code": "8867-4", "value": 72.0, "unit": "/min",
            "effective_at": "2026-10-26T09:30:00Z", "status": "entered-in-error"
        });
        refused_then_accepted(
            db,
            "PUT /api/patients/{id}/observations/{observation_id}",
            || async {
                let r = server
                    .put(&format!("/api/patients/{id}/observations/{observation_id}"))
                    .json(&json!({ "observation": mistaken, "expected_version": 1 }))
                    .await;
                (r.status_code(), r.text())
            },
        )
        .await;

        // Amendment request and a step on it: the requester asks about the observation, then withdraws
        // (with login off one agent is both sides, and may not decide its own request).
        refused_then_accepted(db, "POST /api/patients/{id}/amendments", || async {
            let r = server
                .post(&format!("/api/patients/{id}/amendments"))
                .json(&json!({
                    "kind": "Observation", "target_id": observation_id, "version": 1,
                    "text": "the rate was 62, not 72"
                }))
                .await;
            (r.status_code(), r.text())
        })
        .await;
        let listed = server
            .get(&format!("/api/patients/{id}/amendments"))
            .await
            .json::<Value>();
        let amendment_id = listed["amendments"][0]["id"]
            .as_str()
            .expect("an amendment id")
            .to_string();
        refused_then_accepted(
            db,
            "PUT /api/patients/{id}/amendments/{amendment_id}",
            || async {
                let r = server
                    .put(&format!("/api/patients/{id}/amendments/{amendment_id}"))
                    .json(&json!({ "expected_version": 1, "step": "withdraw" }))
                    .await;
                (r.status_code(), r.text())
            },
        )
        .await;

        // Photo put and withdraw.
        let png = HeaderValue::from_static("image/png");
        let ct = HeaderName::from_static("content-type");
        refused_then_accepted(db, "PUT /api/patients/{id}/photo", || async {
            let r = server
                .put(&format!("/api/patients/{id}/photo?expected_version=0"))
                .add_header(ct.clone(), png.clone())
                .bytes(PNG.to_vec().into())
                .await;
            (r.status_code(), r.text())
        })
        .await;
        refused_then_accepted(db, "POST /api/patients/{id}/photo/withdraw", || async {
            let r = server
                .post(&format!(
                    "/api/patients/{id}/photo/withdraw?expected_version=1"
                ))
                .await;
            (r.status_code(), r.text())
        })
        .await;

        // Consent create and update.
        let consent = json!({ "kind": "general", "status": "active",
            "provisions": [{ "effect": "permit", "purposes": ["treatment"] }] });
        refused_then_accepted(db, "POST /api/patients/{id}/consents", || async {
            let r = server
                .post(&format!("/api/patients/{id}/consents"))
                .json(&json!({ "consent": consent }))
                .await;
            (r.status_code(), r.text())
        })
        .await;
        let consents = server
            .get(&format!("/api/patients/{id}/consents"))
            .await
            .json::<Value>();
        let consent_id = consents["consents"][0]["id"]
            .as_str()
            .expect("a consent id")
            .to_string();
        let inactive = json!({ "kind": "general", "status": "inactive",
            "provisions": [{ "effect": "permit", "purposes": ["treatment"] }] });
        refused_then_accepted(
            db,
            "PUT /api/patients/{id}/consents/{consent_id}",
            || async {
                let r = server
                    .put(&format!("/api/patients/{id}/consents/{consent_id}"))
                    .json(&json!({ "consent": inactive, "expected_version": 1 }))
                    .await;
                (r.status_code(), r.text())
            },
        )
        .await;

        // Relationship create and update.
        let rel = json!({ "subject": "nurse-1", "kind": "care-team", "status": "active" });
        refused_then_accepted(db, "POST /api/patients/{id}/relationships", || async {
            let r = server
                .post(&format!("/api/patients/{id}/relationships"))
                .json(&json!({ "relationship": rel }))
                .await;
            (r.status_code(), r.text())
        })
        .await;
        let rels = server
            .get(&format!("/api/patients/{id}/relationships"))
            .await
            .json::<Value>();
        let rel_id = rels["relationships"][0]["id"]
            .as_str()
            .expect("a relationship id")
            .to_string();
        let ended_rel = json!({ "subject": "nurse-1", "kind": "care-team", "status": "ended" });
        refused_then_accepted(
            db,
            "PUT /api/patients/{id}/relationships/{relationship_id}",
            || async {
                let r = server
                    .put(&format!("/api/patients/{id}/relationships/{rel_id}"))
                    .json(&json!({ "relationship": ended_rel, "expected_version": 1 }))
                    .await;
                (r.status_code(), r.text())
            },
        )
        .await;

        // Merge, then unmerge. The two patients are different people, so merge is refused as a
        // proposal only if the matcher says no; use two records of one person.
        let twin_family = tag("audtwin");
        let twin = patient(&twin_family, 28_000_000);
        let mut twin_ids = Vec::new();
        for override_reason in [None, Some("same person, second record for the test")] {
            let mut body = json!({ "patient": twin });
            if let Some(r) = override_reason {
                body["override_reason"] = json!(r);
            }
            let made = server.post("/api/patients").json(&body).await;
            assert_eq!(made.status_code(), StatusCode::CREATED, "{}", made.text());
            twin_ids.push(made.json::<Value>()["id"].as_str().unwrap().to_string());
        }
        let (target, source) = (twin_ids[0].clone(), twin_ids[1].clone());
        refused_then_accepted(db, "POST /api/patients/{id}/merge", || async {
            let r = server
                .post(&format!("/api/patients/{target}/merge"))
                .json(&json!({ "source": source, "reason": "same person" }))
                .await;
            (r.status_code(), r.text())
        })
        .await;
        refused_then_accepted(db, "POST /api/patients/{id}/unmerge", || async {
            let r = server
                .post(&format!("/api/patients/{target}/unmerge"))
                .json(&json!({ "source": source, "reason": "mistake" }))
                .await;
            (r.status_code(), r.text())
        })
        .await;

        uninstall(db).await;

        let mut driven = DRIVEN.lock().unwrap().clone();
        driven.sort();
        let mut covered: Vec<String> = LOGIN_OFF.iter().map(ToString::to_string).collect();
        covered.sort();
        assert_eq!(
            driven, covered,
            "LOGIN_OFF must list exactly what this test drives"
        );
    })
    .await;
}

/// Census (database-free): every route in the policy table that is not a GET is driven by this file or
/// by `http_audit_refusal_roles.rs`, or is a read. A new write route therefore fails here until it is given an
/// audit-failure test and listed in `audit_switch::LOGIN_OFF` or `LOGIN_ON`.
#[test]
fn every_write_route_is_covered_or_listed_as_a_gap() {
    let mut writes: Vec<String> = electronic_patient_record::routes::TABLE
        .iter()
        .filter(|(method, ..)| *method != "GET")
        .map(|(method, template, ..)| format!("{method} {template}"))
        .filter(|r| !NOT_A_WRITE.contains(&r.as_str()))
        .collect();
    writes.sort();
    let mut known: Vec<String> = LOGIN_OFF
        .iter()
        .chain(LOGIN_ON.iter())
        .map(ToString::to_string)
        .collect();
    known.sort();
    assert_eq!(
        writes, known,
        "a write route was added or removed: give it an audit-failure test and list it in tests/audit_switch/mod.rs"
    );
}
