//! DB-gated, through the real router with login off (CQ-39): when a write's audit row cannot be written, the
//! write is refused. A trigger on `audit_events` is switched on only around one request, so that request's audit
//! insert fails; the answer must be 500 and no store may gain a row. Then the same request is sent with audit
//! working and must succeed, so the 500 is proved to come from the audit row and not from a bad request.
//!
//! Covered: patient create and update (API and FHIR create), consent, relationship, allergy and the allergy
//! statement, photo put and withdraw, merge and unmerge. **Not covered yet:** FHIR update, the two break-glass
//! writes and resolving a feed refusal (tasks.md CQ-39).
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
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
use serde_json::{Value, json};
use std::future::Future;

const PNG: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 1, 2, 3];

/// The write routes this file drives with a failing audit table, spelled as in `routes::TABLE`.
const COVERED: [&str; 14] = [
    "POST /api/patients",
    "PUT /api/patients/{id}",
    "POST /fhir/R4/Patient",
    "POST /api/patients/{id}/allergies",
    "PUT /api/patients/{id}/allergies/{allergy_id}",
    "PUT /api/patients/{id}/allergy-statement",
    "PUT /api/patients/{id}/photo",
    "POST /api/patients/{id}/photo/withdraw",
    "POST /api/patients/{id}/consents",
    "PUT /api/patients/{id}/consents/{consent_id}",
    "POST /api/patients/{id}/relationships",
    "PUT /api/patients/{id}/relationships/{relationship_id}",
    "POST /api/patients/{id}/merge",
    "POST /api/patients/{id}/unmerge",
];

/// Write routes with no such test yet. Each one is a gap in CQ-39: remove it from here when it is covered.
const NOT_COVERED: [&str; 4] = [
    "PUT /fhir/R4/Patient/{id}",
    "POST /api/patients/{id}/break-glass",
    "POST /api/break-glass/reviews/{id}",
    "POST /api/feed/refusals/{id}/resolve",
];

/// A read in write clothing: it audits, but it changes nothing.
const NOT_A_WRITE: [&str; 1] = ["POST /api/patients/compare"];

/// The routes `refused_then_accepted` has been called with.
static DRIVEN: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

/// Every table a write route changes.
const STORES: [&str; 9] = [
    "patient_versions",
    "consent_versions",
    "relationship_versions",
    "allergy_versions",
    "allergy_assertion_versions",
    "patient_photos",
    "patient_link_versions",
    "break_glass_grants",
    "feed_refusal_resolutions",
];

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

async fn sql(db: &DatabaseConnection, text: &str) {
    db.execute_unprepared(text).await.unwrap();
}

async fn counts(db: &DatabaseConnection) -> Vec<i64> {
    let mut out = Vec::new();
    for table in STORES.iter().chain(["audit_events"].iter()) {
        let row = db
            .query_one_raw(Statement::from_string(
                db.get_database_backend(),
                format!("SELECT count(*) AS n FROM {table}"),
            ))
            .await
            .unwrap()
            .unwrap();
        out.push(row.try_get::<i64>("", "n").unwrap());
    }
    out
}

/// Install the switch and its trigger. The switch starts off.
async fn install(db: &DatabaseConnection) {
    sql(
        db,
        "DROP TRIGGER IF EXISTS epr_test_audit_refuse ON audit_events",
    )
    .await;
    sql(db, "DROP TABLE IF EXISTS epr_test_audit_switch").await;
    sql(
        db,
        "CREATE TABLE epr_test_audit_switch (fail BOOLEAN NOT NULL)",
    )
    .await;
    sql(db, "INSERT INTO epr_test_audit_switch VALUES (false)").await;
    sql(
        db,
        "CREATE OR REPLACE FUNCTION epr_test_audit_refuse() RETURNS trigger AS $$
         BEGIN
           IF (SELECT fail FROM epr_test_audit_switch LIMIT 1) THEN
             RAISE EXCEPTION 'audit row refused by test';
           END IF;
           RETURN NEW;
         END $$ LANGUAGE plpgsql",
    )
    .await;
    sql(
        db,
        "CREATE TRIGGER epr_test_audit_refuse BEFORE INSERT ON audit_events
         FOR EACH ROW EXECUTE FUNCTION epr_test_audit_refuse()",
    )
    .await;
}

async fn uninstall(db: &DatabaseConnection) {
    sql(
        db,
        "DROP TRIGGER IF EXISTS epr_test_audit_refuse ON audit_events",
    )
    .await;
    sql(db, "DROP FUNCTION IF EXISTS epr_test_audit_refuse()").await;
    sql(db, "DROP TABLE IF EXISTS epr_test_audit_switch").await;
}

/// Send the request with the audit insert failing, check it was refused and changed nothing, then
/// send it again with audit working and check it is accepted.
async fn refused_then_accepted<F, Fut>(db: &DatabaseConnection, route: &str, send: F)
where
    F: Fn() -> Fut,
    Fut: Future<Output = (StatusCode, String)>,
{
    DRIVEN.lock().unwrap().push(route.to_string());
    let before = counts(db).await;
    sql(db, "UPDATE epr_test_audit_switch SET fail = true").await;
    let (status, body) = send().await;
    // Off again before anything can fail an assertion and leave it on.
    sql(db, "UPDATE epr_test_audit_switch SET fail = false").await;
    assert_eq!(
        status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "{route}: a write whose audit row cannot be written must be refused: {body}"
    );
    assert_eq!(
        counts(db).await,
        before,
        "{route}: the refused write left a row behind"
    );
    let (status, body) = send().await;
    assert!(
        status.is_success(),
        "{route}: the same request must succeed once audit works ({status}): {body}"
    );
    assert_ne!(
        counts(db).await,
        before,
        "{route}: the accepted write stored nothing"
    );
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
        let mut covered: Vec<String> = COVERED.iter().map(ToString::to_string).collect();
        covered.sort();
        assert_eq!(
            driven, covered,
            "COVERED must list exactly what this test drives"
        );
    })
    .await;
}

/// Census (database-free): every route in the policy table that is not a GET is either driven above, listed
/// as not covered, or a read. A new write route therefore fails here until it is given an audit-failure test
/// or an entry in `NOT_COVERED`, which is a visible gap in CQ-39.
#[test]
fn every_write_route_is_covered_or_listed_as_a_gap() {
    let mut writes: Vec<String> = electronic_patient_record::routes::TABLE
        .iter()
        .filter(|(method, ..)| *method != "GET")
        .map(|(method, template, ..)| format!("{method} {template}"))
        .filter(|r| !NOT_A_WRITE.contains(&r.as_str()))
        .collect();
    writes.sort();
    let mut known: Vec<String> = COVERED
        .iter()
        .chain(NOT_COVERED.iter())
        .map(ToString::to_string)
        .collect();
    known.sort();
    assert_eq!(
        writes, known,
        "a write route was added or removed: give it an audit-failure test (COVERED) or list it (NOT_COVERED)"
    );
}
