//! DB-gated: `down` refuses to drop clinical tables outside development (task CQ-15).
//!
//! ```text
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test migration_down -- --ignored --test-threads=1
//! ```

use electronic_patient_record_migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, Database, Statement};

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn down_in_production_is_an_error_and_the_tables_remain() {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for --ignored tests");
    let db = Database::connect(url).await.expect("connect");
    Migrator::up(&db, None).await.expect("migrate");
    // Even with the flag, production refuses; this is the only test in this binary, so the
    // environment is ours.
    // SAFETY: the only test in this binary, and no other thread reads the environment yet.
    unsafe {
        std::env::set_var("LOCO_ENV", "production");
        std::env::set_var("EPR_ALLOW_DESTRUCTIVE_DOWN", "true");
    }
    let e = Migrator::down(&db, Some(12)).await.unwrap_err();
    assert!(e.to_string().contains("allowed only with"), "{e}");
    for table in [
        "audit_events",
        "patient_versions",
        "consent_versions",
        "patient_link_versions",
        "break_glass_grants",
    ] {
        let r = db
            .query_one_raw(Statement::from_string(
                db.get_database_backend(),
                format!("SELECT to_regclass('{table}')::text AS t"),
            ))
            .await
            .unwrap()
            .unwrap();
        let t: Option<String> = r.try_get("", "t").unwrap();
        assert!(t.is_some(), "{table} was dropped");
    }
}
