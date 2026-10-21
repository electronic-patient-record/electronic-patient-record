//! DB-gated: the authorization middleware refuses a caller whose roles do not permit the route,
//! refuses a route with no policy entry, and writes each refusal to the audit trail without
//! putting the request body in it. A permitted request passes through untouched.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test authz_audit -- --ignored --test-threads=1
//! ```

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use axum::routing::{get, post};
use electronic_patient_record::auth::Agent;
use electronic_patient_record::authz::{Authz, authorize_mw};
use electronic_patient_record::policy::{Role, Subject};
use electronic_patient_record_migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, Statement};
use tower::ServiceExt;

async fn db() -> DatabaseConnection {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for --ignored tests");
    let db = Database::connect(url).await.expect("connect");
    Migrator::up(&db, None).await.expect("migrate");
    db
}

/// The app as the middleware sees it: a caller with the given roles, already authenticated.
fn app(db: DatabaseConnection, agent: &str, roles: &[Role]) -> Router {
    app_in(db, agent, roles, &[])
}

fn app_in(db: DatabaseConnection, agent: &str, roles: &[Role], organisations: &[&str]) -> Router {
    let agent = Agent(agent.to_string());
    let subject = Subject {
        roles: roles.iter().copied().collect(),
        organisations: organisations.iter().map(|o| (*o).to_string()).collect(),
    };
    Router::new()
        .route("/api/patients/compare", post(|| async { "compared" }))
        .route("/api/unlisted", get(|| async { "should never run" }))
        .layer(axum::middleware::from_fn_with_state(
            Authz { db },
            authorize_mw,
        ))
        .layer(axum::middleware::from_fn(
            move |mut req: axum::extract::Request, next: axum::middleware::Next| {
                let (agent, subject) = (agent.clone(), subject.clone());
                async move {
                    req.extensions_mut().insert(agent);
                    req.extensions_mut().insert(subject);
                    next.run(req).await
                }
            },
        ))
}

async fn call(app: Router, method: Method, path: &str, body: &str) -> StatusCode {
    app.oneshot(
        Request::builder()
            .method(method)
            .uri(path)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
    )
    .await
    .unwrap()
    .status()
}

async fn rows_for(db: &DatabaseConnection, agent: &str) -> Vec<(i16, String, String)> {
    db.query_all_raw(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT outcome, resource_ref, detail::text AS detail FROM audit_events WHERE agent = $1 ORDER BY id",
        [agent.into()],
    ))
    .await
    .unwrap()
    .iter()
    .map(|r| {
        (
            r.try_get("", "outcome").unwrap(),
            r.try_get("", "resource_ref").unwrap(),
            r.try_get("", "detail").unwrap(),
        )
    })
    .collect()
}

fn unique(label: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("test-{label}-{}-{nanos}", std::process::id())
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_role_the_table_does_not_permit_is_refused_and_audited() {
    let db = db().await;
    let agent = unique("auditor");
    let secret = "Zebediah Quux";
    let status = call(
        app(db.clone(), &agent, &[Role::Auditor]),
        Method::POST,
        "/api/patients/compare",
        &format!(r#"{{"a":{{"names":[{{"family":"{secret}"}}]}},"b":{{}}}}"#),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let rows = rows_for(&db, &agent).await;
    assert_eq!(rows.len(), 1);
    let (outcome, resource, detail) = &rows[0];
    assert_eq!(*outcome, 4);
    assert_eq!(resource, "/api/patients/compare");
    assert!(detail.contains("role-not-permitted"), "{detail}");
    assert!(detail.contains("auditor"), "{detail}");
    assert!(
        !detail.contains(secret),
        "the body must not reach the audit row"
    );
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_route_with_no_policy_entry_is_refused_for_every_role_and_audited() {
    let db = db().await;
    let agent = unique("everyone");
    let status = call(
        app(db.clone(), &agent, &Role::ALL),
        Method::GET,
        "/api/unlisted",
        "",
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let rows = rows_for(&db, &agent).await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].1, "unmatched-route");
    assert!(rows[0].2.contains("no-policy-for-route"));
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_permitted_role_passes_through_and_the_gate_writes_nothing() {
    let db = db().await;
    let agent = unique("clinician");
    let status = call(
        app(db.clone(), &agent, &[Role::Clinician]),
        Method::POST,
        "/api/patients/compare",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    // The handler audits its own access; the gate adds nothing for an allow.
    assert!(rows_for(&db, &agent).await.is_empty());
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_row_written_while_handling_a_request_records_the_callers_roles_and_organisations() {
    let db = db().await;
    let agent = unique("receptionist");
    // A refusal: the gate writes the row inside the request.
    let status = call(
        app_in(
            db.clone(),
            &agent,
            &[Role::Receptionist, Role::Nurse],
            &["riverside"],
        ),
        Method::GET,
        "/api/unlisted",
        "",
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let rows = rows_for(&db, &agent).await;
    assert_eq!(rows.len(), 1);
    let detail: serde_json::Value = serde_json::from_str(&rows[0].2).unwrap();
    assert_eq!(
        detail["actor"]["roles"],
        serde_json::json!(["nurse", "receptionist"])
    );
    assert_eq!(
        detail["actor"]["organisations"],
        serde_json::json!(["riverside"])
    );
}
