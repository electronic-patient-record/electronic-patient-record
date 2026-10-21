//! DB-gated (CQ-9): the `*_current` views give exactly the latest version of each record, and a
//! filter on the patient seeks an index instead of building the current version of every record.
//!
//! ```text
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test current_views -- --ignored --test-threads=1
//! ```

use electronic_patient_record_migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, Statement, TransactionTrait};

async fn db() -> DatabaseConnection {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for --ignored tests");
    let db = Database::connect(url).await.expect("connect");
    Migrator::up(&db, None).await.expect("migrate");
    db
}

/// (view, versions table, record key, the patient column a page filters on)
const VIEWS: [(&str, &str, &str, &str); 4] = [
    (
        "consent_current",
        "consent_versions",
        "consent_id",
        "patient_id",
    ),
    (
        "allergy_current",
        "allergy_versions",
        "allergy_id",
        "patient_id",
    ),
    (
        "relationship_current",
        "relationship_versions",
        "relationship_id",
        "patient_id",
    ),
    (
        "patient_link_current",
        "patient_link_versions",
        "link_id",
        "source_patient_id",
    ),
];

async fn count(db: &impl ConnectionTrait, sql: &str) -> i64 {
    db.query_one_raw(Statement::from_string(sea_orm::DbBackend::Postgres, sql))
        .await
        .unwrap()
        .unwrap()
        .try_get_by_index::<i64>(0)
        .unwrap()
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn each_view_is_the_latest_version_of_each_record() {
    let db = db().await;
    let patients = (
        "patient_current",
        "patient_versions",
        "patient_id",
        "patient_id",
    );
    for (view, table, key, _) in VIEWS.into_iter().chain([patients]) {
        // The definition the views had before: DISTINCT ON the key, highest version first.
        let latest =
            format!("SELECT DISTINCT ON ({key}) * FROM {table} ORDER BY {key}, version DESC");
        let cols = format!("SELECT * FROM {view}");
        let missing = count(
            &db,
            &format!("SELECT count(*) FROM (({latest}) EXCEPT ({cols})) d"),
        )
        .await;
        let extra = count(
            &db,
            &format!("SELECT count(*) FROM (({cols}) EXCEPT ({latest})) d"),
        )
        .await;
        assert_eq!((missing, extra), (0, 0), "{view}");
    }
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_filter_on_the_patient_seeks_an_index() {
    let db = db().await;
    for (view, _, _, column) in VIEWS {
        let tx = db.begin().await.unwrap();
        // Test tables are small enough that a scan would win anyway.
        tx.execute_unprepared("SET LOCAL enable_seqscan = off")
            .await
            .unwrap();
        let plan: Vec<String> = tx
            .query_all_raw(Statement::from_string(
                sea_orm::DbBackend::Postgres,
                format!(
                    "EXPLAIN SELECT * FROM {view}
                     WHERE {column} = '00000000-0000-4000-8000-000000000000'::uuid"
                ),
            ))
            .await
            .unwrap()
            .iter()
            .map(|r| r.try_get_by_index::<String>(0).unwrap())
            .collect();
        assert!(
            plan.iter()
                .any(|l| l.contains(&format!("Index Cond: ({column} =")))
                && !plan.iter().any(|l| l.contains("Unique")),
            "{view}: {plan:#?}"
        );
    }
}

async fn plan(db: &DatabaseConnection, sql: &str) -> Vec<String> {
    let tx = db.begin().await.unwrap();
    tx.execute_unprepared("SET LOCAL enable_seqscan = off")
        .await
        .unwrap();
    tx.query_all_raw(Statement::from_string(
        sea_orm::DbBackend::Postgres,
        format!("EXPLAIN {sql}"),
    ))
    .await
    .unwrap()
    .iter()
    .map(|r| r.try_get_by_index::<String>(0).unwrap())
    .collect()
}

/// Patient search by an identifier or a birth date reaches an index on the versions table.
#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn patient_search_by_identifier_or_birth_date_uses_an_index() {
    let db = db().await;
    for (index, filter) in [
        (
            "patient_versions_identifiers",
            r#"body->'identifiers' @> '[{"system":"https://fhir.nhs.uk/Id/nhs-number","value":"9434765919"}]'::jsonb"#,
        ),
        (
            "patient_versions_birth_date",
            "body->>'birth_date' = '1980-03-14'",
        ),
    ] {
        let p = plan(
            &db,
            &format!("SELECT * FROM patient_current WHERE {filter}"),
        )
        .await;
        assert!(
            p.iter().any(|l| l.contains(index)) && !p.iter().any(|l| l.contains("Unique")),
            "{index}: {p:#?}"
        );
    }
}
