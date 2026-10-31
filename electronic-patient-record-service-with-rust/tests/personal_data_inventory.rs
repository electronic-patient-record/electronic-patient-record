//! `governance/personal-data-inventory.csv` classifies EVERY column of the database and stays true
//! to it (idea from the patient-administration-system project). Adding a column without
//! classifying it, or classifying one that does not exist, fails the database test, so personal
//! data cannot arrive by accident. The shape test needs no database.
//!
//! ```text
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test personal_data_inventory -- --ignored --test-threads=1
//! ```

use electronic_patient_record_migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, Database, DbBackend, Statement};
use std::collections::BTreeSet;
use std::path::Path;

const CATEGORIES: [&str; 8] = [
    "identifier",
    "staff-identifier",
    "health",
    "biometric",
    "preference",
    "free-text",
    "staff-trace",
    "none",
];
const RETENTION: [&str; 3] = ["record", "audit", "feed"];

fn inventory() -> Vec<[String; 5]> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("governance/personal-data-inventory.csv");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut lines = text.lines();
    assert_eq!(
        lines.next(),
        Some("table,column,category,purpose,retention_class"),
        "header"
    );
    lines
        .map(|l| {
            let f: Vec<&str> = l.split(',').collect();
            assert_eq!(f.len(), 5, "malformed row (no commas in a purpose): {l}");
            std::array::from_fn(|i| f[i].to_string())
        })
        .collect()
}

#[test]
fn inventory_rows_are_well_formed_and_unique() {
    let mut seen = BTreeSet::new();
    for [t, c, cat, purpose, ret] in inventory() {
        assert!(
            CATEGORIES.contains(&cat.as_str()),
            "{t}.{c}: unknown category {cat:?}"
        );
        assert!(
            RETENTION.contains(&ret.as_str()),
            "{t}.{c}: unknown retention class {ret:?}"
        );
        assert!(!purpose.trim().is_empty(), "{t}.{c}: no purpose");
        assert!(
            seen.insert((t.clone(), c.clone())),
            "{t}.{c} is listed twice"
        );
    }
    assert!(!seen.is_empty());
}

// A partition of `audit_events` holds the parent's columns, which are classified under the parent (OS-13);
// every column of every other table, and of the parent, must still be listed.
#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn every_database_column_is_classified_and_no_stale_row_remains() {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for --ignored tests");
    let db = Database::connect(url).await.expect("connect");
    Migrator::up(&db, None).await.expect("migrate");
    let rows = db
        .query_all_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT table_name, column_name FROM information_schema.columns \
             WHERE table_schema = 'public' AND table_name NOT LIKE 'seaql%' \
               AND table_name NOT IN (SELECT relname::text FROM pg_class WHERE relispartition)"
                .to_string(),
        ))
        .await
        .unwrap();
    let actual: BTreeSet<(String, String)> = rows
        .iter()
        .map(|r| {
            (
                r.try_get("", "table_name").unwrap(),
                r.try_get("", "column_name").unwrap(),
            )
        })
        .collect();
    let listed: BTreeSet<(String, String)> =
        inventory().into_iter().map(|[t, c, ..]| (t, c)).collect();
    let unclassified: Vec<_> = actual.difference(&listed).collect();
    let stale: Vec<_> = listed.difference(&actual).collect();
    assert!(
        unclassified.is_empty(),
        "columns not in governance/personal-data-inventory.csv (classify them): {unclassified:?}"
    );
    assert!(
        stale.is_empty(),
        "inventory rows for columns that no longer exist: {stale:?}"
    );
}
