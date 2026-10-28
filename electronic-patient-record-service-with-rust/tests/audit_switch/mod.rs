//! Shared by the audit-refusal tests (CQ-39): a trigger on `audit_events` that fails the insert while a switch
//! is on, the row counts of every store a write route changes, and `refused_then_accepted`, which sends one
//! request with the audit insert failing and again with it working.
#![allow(dead_code)]

use axum::http::StatusCode;
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
use std::future::Future;

/// The write routes driven with login off (`http_audit_refusal.rs`), spelled as in `routes::TABLE`.
pub const LOGIN_OFF: [&str; 21] = [
    "POST /api/patients",
    "PUT /api/patients/{id}",
    "POST /fhir/R4/Patient",
    "PUT /fhir/R4/Patient/{id}",
    "POST /api/patients/{id}/allergies",
    "PUT /api/patients/{id}/allergies/{allergy_id}",
    "PUT /api/patients/{id}/allergy-statement",
    "POST /api/patients/{id}/observations",
    "PUT /api/patients/{id}/observations/{observation_id}",
    "POST /api/patients/{id}/amendments",
    "PUT /api/patients/{id}/amendments/{amendment_id}",
    "POST /api/hazards",
    "PUT /api/hazards/{hazard_id}",
    "PUT /api/patients/{id}/photo",
    "POST /api/patients/{id}/photo/withdraw",
    "POST /api/patients/{id}/consents",
    "PUT /api/patients/{id}/consents/{consent_id}",
    "POST /api/patients/{id}/relationships",
    "PUT /api/patients/{id}/relationships/{relationship_id}",
    "POST /api/patients/{id}/merge",
    "POST /api/patients/{id}/unmerge",
];

/// The write routes driven with login on (`http_audit_refusal_roles.rs`).
pub const LOGIN_ON: [&str; 3] = [
    "POST /api/patients/{id}/break-glass",
    "POST /api/break-glass/reviews/{id}",
    "POST /api/feed/refusals/{id}/resolve",
];

/// The routes `refused_then_accepted` has been called with.
pub static DRIVEN: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

/// Every table a write route changes.
pub const STORES: [&str; 13] = [
    "patient_versions",
    "consent_versions",
    "relationship_versions",
    "allergy_versions",
    "allergy_assertion_versions",
    "observation_versions",
    "amendment_versions",
    "hazard_versions",
    "patient_photos",
    "patient_link_versions",
    "break_glass_grants",
    "break_glass_reviews",
    "feed_refusal_resolutions",
];

pub async fn sql(db: &DatabaseConnection, text: &str) {
    db.execute_unprepared(text).await.unwrap();
}

pub async fn counts(db: &DatabaseConnection) -> Vec<i64> {
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
pub async fn install(db: &DatabaseConnection) {
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

pub async fn uninstall(db: &DatabaseConnection) {
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
pub async fn refused_then_accepted<F, Fut>(db: &DatabaseConnection, route: &str, send: F)
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
