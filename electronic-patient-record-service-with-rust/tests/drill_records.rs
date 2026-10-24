//! DB-gated: drill and test records (EC-10). A drill record is kept out of ordinary search and out of the
//! duplicate check for real patients, keeps its flag through every version, and can never be merged with a
//! real record.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test drill_records -- --ignored --test-threads=1
//! ```

use electronic_patient_record::links;
use electronic_patient_record::patients_store::{self as store, Search, StoreError};
use electronic_patient_record_matcher::{HumanName, Patient};
use electronic_patient_record_migration::{Migrator, MigratorTrait};
use sea_orm::{Database, DatabaseConnection, TransactionTrait};

async fn db() -> DatabaseConnection {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for --ignored tests");
    let db = Database::connect(url).await.expect("connect");
    Migrator::up(&db, None).await.expect("migrate");
    db
}

fn tag(label: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("Zz{label}{nanos}")
}

fn person(family: &str) -> Patient {
    Patient {
        names: vec![HumanName {
            family: Some(family.into()),
            given: vec!["Sam".into()],
        }],
        birth_date: Some("1975-06-01".into()),
        ..Default::default()
    }
}

fn by_family(family: &str, drill: bool) -> Search {
    Search {
        family: Some(family.into()),
        drill,
        ..Default::default()
    }
}

/// The ids of this test's records among `found`: the duplicate check also matches other tests' rows
/// that share the birth date.
fn ours(found: Vec<store::Stored>, family: &str) -> Vec<String> {
    found
        .into_iter()
        .filter(|s| s.patient.names[0].family.as_deref() == Some(family))
        .map(|s| s.id)
        .collect()
}

fn ids(found: Vec<store::Stored>) -> Vec<String> {
    found.into_iter().map(|s| s.id).collect()
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_drill_record_is_flagged_and_a_real_one_is_not() {
    let db = db().await;
    let family = tag("flag");
    let real = store::create(&db, "t", None, &person(&family))
        .await
        .unwrap();
    let drill = store::create_as(&db, "t", None, &person(&family), true)
        .await
        .unwrap();
    assert!(!real.drill);
    assert!(drill.drill);
    assert!(store::current(&db, &drill.id).await.unwrap().drill);
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn search_shows_real_records_by_default_and_drill_records_only_when_asked() {
    let db = db().await;
    let family = tag("search");
    let real = store::create(&db, "t", None, &person(&family))
        .await
        .unwrap();
    let drill = store::create_as(&db, "t", None, &person(&family), true)
        .await
        .unwrap();
    let ordinary = ids(store::search(&db, &by_family(&family, false))
        .await
        .unwrap());
    assert_eq!(ordinary, vec![real.id.clone()]);
    let drills = ids(store::search(&db, &by_family(&family, true)).await.unwrap());
    assert_eq!(drills, vec![drill.id]);
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn the_duplicate_check_never_offers_a_drill_record_for_a_real_one_or_the_reverse() {
    let db = db().await;
    let family = tag("dup");
    let real = store::create(&db, "t", None, &person(&family))
        .await
        .unwrap();
    let drill = store::create_as(&db, "t", None, &person(&family), true)
        .await
        .unwrap();
    let probe = person(&family);
    let (for_real, _) = store::candidates_in(&db, &probe, false).await.unwrap();
    assert_eq!(ours(for_real, &family), vec![real.id.clone()]);
    let (for_drill, _) = store::candidates_in(&db, &probe, true).await.unwrap();
    assert_eq!(ours(for_drill, &family), vec![drill.id]);
    // The ordinary entry points are the real side.
    assert_eq!(
        ours(store::candidates(&db, &probe).await.unwrap(), &family),
        vec![real.id]
    );
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn every_version_of_a_drill_record_stays_a_drill_record() {
    let db = db().await;
    let family = tag("versions");
    let v1 = store::create_as(&db, "t", None, &person(&family), true)
        .await
        .unwrap();
    let mut changed = person(&family);
    changed.birth_date = Some("1975-06-02".into());
    let v2 = store::update(&db, &v1.id, 1, "t", Some("corrected"), &changed)
        .await
        .unwrap();
    assert_eq!(v2.version, 2);
    assert!(v2.drill, "a drill record cannot become real by an edit");
    assert!(store::current(&db, &v1.id).await.unwrap().drill);
    let real = store::create(&db, "t", None, &person(&family))
        .await
        .unwrap();
    let r2 = store::update(&db, &real.id, 1, "t", None, &changed)
        .await
        .unwrap();
    assert!(!r2.drill);
}

async fn merge(
    db: &DatabaseConnection,
    target: &str,
    source: &str,
) -> Result<links::StoredLink, StoreError> {
    let tx = db.begin().await.unwrap();
    let r = links::merge(&tx, target, source, "admin", "same person").await;
    if r.is_ok() {
        tx.commit().await.unwrap();
    }
    r
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_drill_record_can_never_be_merged_with_a_real_record() {
    let db = db().await;
    let family = tag("merge");
    let real = store::create(&db, "t", None, &person(&family))
        .await
        .unwrap();
    let drill = store::create_as(&db, "t", None, &person(&family), true)
        .await
        .unwrap();
    for (target, source) in [(&real.id, &drill.id), (&drill.id, &real.id)] {
        let e = merge(&db, target, source).await.unwrap_err();
        let StoreError::BadQuery(m) = e else {
            panic!("expected a refusal, got {e:?}")
        };
        assert!(m.contains("drill record and a real record"), "{m}");
    }
    // Two drill records may be merged with each other: the exercise can include a merge.
    let other = store::create_as(&db, "t", None, &person(&family), true)
        .await
        .unwrap();
    assert!(merge(&db, &drill.id, &other.id).await.is_ok());
}
