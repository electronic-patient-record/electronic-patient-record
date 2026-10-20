//! DB-gated: the `audit_events` table refuses UPDATE, DELETE and TRUNCATE.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test audit_immutable -- --ignored --test-threads=1
//! ```

use electronic_patient_record_migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection};

async fn connect() -> DatabaseConnection {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for --ignored tests");
    let db = Database::connect(url).await.expect("connect");
    Migrator::up(&db, None).await.expect("migrate");
    db
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn audit_events_are_append_only() {
    let db = connect().await;
    db.execute_unprepared(
        "INSERT INTO audit_events (action, agent, digest) VALUES ('R', 'test-agent', 'd0')",
    )
    .await
    .expect("insert is allowed");

    for statement in [
        "UPDATE audit_events SET agent = 'someone-else'",
        "DELETE FROM audit_events",
        "TRUNCATE audit_events",
    ] {
        let err = db.execute_unprepared(statement).await.expect_err(statement);
        assert!(
            err.to_string().contains("append-only"),
            "{statement}: {err}"
        );
    }
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn audit_action_must_be_a_fhir_audit_action() {
    let db = connect().await;
    let err = db
        .execute_unprepared(
            "INSERT INTO audit_events (action, agent, digest) VALUES ('X', 'a', 'd')",
        )
        .await
        .expect_err("X is not C, R, U, D or E");
    assert!(err.to_string().contains("check"), "{err}");
}
