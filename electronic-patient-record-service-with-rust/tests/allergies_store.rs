//! DB-gated: allergies are append-only versions; "no known allergies" is a recorded statement that
//! cannot contradict the record; the banner summary follows the records.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test allergies_store -- --ignored --test-threads=1
//! ```

use electronic_patient_record::allergies::{
    self as allergies, AllergyRecord, Category, Criticality, Status, Summary,
};
use electronic_patient_record::patients_store::{self, StoreError};
use electronic_patient_record_matcher::{HumanName, Patient};
use electronic_patient_record_migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection};

async fn db() -> DatabaseConnection {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for --ignored tests");
    let db = Database::connect(url).await.expect("connect");
    Migrator::up(&db, None).await.expect("migrate");
    db
}

async fn patient(db: &DatabaseConnection) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let p = Patient {
        names: vec![HumanName {
            family: Some(format!("Zzallergy{nanos}")),
            given: vec!["Al".into()],
        }],
        birth_date: Some("1985-01-01".into()),
        ..Default::default()
    };
    patients_store::create(db, "t", None, &p).await.unwrap().id
}

fn rec(substance: &str, status: Status, criticality: Option<Criticality>) -> AllergyRecord {
    AllergyRecord {
        substance: substance.into(),
        coding: None,
        category: Some(Category::Medication),
        criticality,
        reaction: Some("hives".into()),
        onset: Some("2019-04".into()),
        status,
    }
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn an_allergy_is_versioned_ended_and_can_be_retracted_but_never_reopened() {
    let db = db().await;
    let p = patient(&db).await;
    assert_eq!(
        allergies::summary(&db, &p).await.unwrap(),
        Summary::NotRecorded
    );

    let v1 = allergies::create(
        &db,
        &p,
        "nurse-1",
        Some("history"),
        &rec("Penicillin", Status::Active, Some(Criticality::High)),
    )
    .await
    .unwrap();
    assert_eq!(v1.version, 1);
    let Summary::Active { items } = allergies::summary(&db, &p).await.unwrap() else {
        panic!()
    };
    assert_eq!(items[0].substance, "Penicillin");

    // End it: a new version; the summary no longer calls it active and does not claim none.
    let v2 = allergies::update(
        &db,
        &p,
        &v1.id,
        1,
        "dr-1",
        Some("tolerated challenge"),
        &rec("Penicillin", Status::Inactive, Some(Criticality::High)),
    )
    .await
    .unwrap();
    assert_eq!((v2.version, v2.allergy.status), (2, Status::Inactive));
    assert_eq!(
        allergies::summary(&db, &p).await.unwrap(),
        Summary::NoneActive { inactive: 1 }
    );

    // A stale write is a conflict and changes nothing.
    assert_eq!(
        allergies::update(
            &db,
            &p,
            &v1.id,
            1,
            "dr-2",
            None,
            &rec("Penicillin", Status::Active, None)
        )
        .await,
        Err(StoreError::Conflict { current: 2 })
    );

    // Retract it: entered in error is final.
    let v3 = allergies::update(
        &db,
        &p,
        &v1.id,
        2,
        "dr-1",
        Some("wrong chart"),
        &rec("Penicillin", Status::EnteredInError, None),
    )
    .await
    .unwrap();
    assert_eq!(v3.allergy.status, Status::EnteredInError);
    assert!(matches!(
        allergies::update(
            &db,
            &p,
            &v1.id,
            3,
            "dr-1",
            None,
            &rec("Penicillin", Status::Active, None)
        )
        .await,
        Err(StoreError::BadQuery(_))
    ));
    assert_eq!(
        allergies::summary(&db, &p).await.unwrap(),
        Summary::NotRecorded
    );

    // Three versions remain, with their authors.
    let rows = db
        .query_all_raw(sea_orm::Statement::from_sql_and_values(
            sea_orm::DbBackend::Postgres,
            "SELECT author FROM allergy_versions WHERE allergy_id = $1::uuid ORDER BY version",
            [v1.id.clone().into()],
        ))
        .await
        .unwrap();
    let authors: Vec<String> = rows
        .iter()
        .map(|r| r.try_get("", "author").unwrap())
        .collect();
    assert_eq!(authors, ["nurse-1", "dr-1", "dr-1"]);
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn no_known_allergies_is_a_statement_that_cannot_contradict_the_record() {
    let db = db().await;
    let p = patient(&db).await;
    let a = allergies::create(&db, &p, "n", None, &rec("Latex", Status::Active, None))
        .await
        .unwrap();

    // Refused while an active allergy exists.
    assert!(matches!(
        allergies::assert_none_known(&db, &p, 0, "dr", None, true).await,
        Err(StoreError::BadQuery(_))
    ));
    allergies::update(
        &db,
        &p,
        &a.id,
        1,
        "dr",
        Some("resolved"),
        &rec("Latex", Status::Inactive, None),
    )
    .await
    .unwrap();

    let s = allergies::assert_none_known(&db, &p, 0, "dr", Some("asked the patient"), true)
        .await
        .unwrap();
    assert_eq!(
        (s.version, s.no_known_allergies, s.author.as_str()),
        (1, true, "dr")
    );
    assert!(matches!(
        allergies::summary(&db, &p).await.unwrap(),
        Summary::NoKnown { .. }
    ));
    // A stale statement write is a conflict.
    assert_eq!(
        allergies::assert_none_known(&db, &p, 0, "dr2", None, true).await,
        Err(StoreError::Conflict { current: 1 })
    );

    // A new allergy voids the statement: the summary shows the allergy, not "none".
    let b = allergies::create(
        &db,
        &p,
        "n",
        None,
        &rec("Nuts", Status::Active, Some(Criticality::High)),
    )
    .await
    .unwrap();
    assert!(matches!(
        allergies::summary(&db, &p).await.unwrap(),
        Summary::Active { .. }
    ));
    // Ending it afterwards does not bring the old statement back.
    allergies::update(
        &db,
        &p,
        &b.id,
        1,
        "dr",
        None,
        &rec("Nuts", Status::Inactive, None),
    )
    .await
    .unwrap();
    assert_eq!(
        allergies::summary(&db, &p).await.unwrap(),
        Summary::NoneActive { inactive: 2 }
    );

    // Withdrawing needs a statement to withdraw, and the second statement is version 2.
    let again = allergies::assert_none_known(&db, &p, 1, "dr", None, true)
        .await
        .unwrap();
    assert_eq!(again.version, 2);
    let gone = allergies::assert_none_known(&db, &p, 2, "dr", None, false)
        .await
        .unwrap();
    assert!(!gone.no_known_allergies);
    assert!(matches!(
        allergies::assert_none_known(&db, &p, 3, "dr", None, false).await,
        Err(StoreError::BadQuery(_))
    ));
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn bad_records_unknown_patients_and_races_are_refused() {
    let db = db().await;
    let p = patient(&db).await;
    assert!(matches!(
        allergies::create(&db, &p, "n", None, &rec("   ", Status::Active, None)).await,
        Err(StoreError::BadQuery(_))
    ));
    assert!(matches!(
        allergies::create(
            &db,
            &p,
            "n",
            None,
            &rec("Dust", Status::EnteredInError, None)
        )
        .await,
        Err(StoreError::BadQuery(_))
    ));
    assert_eq!(
        allergies::create(
            &db,
            "00000000-0000-4000-8000-000000000000",
            "n",
            None,
            &rec("Dust", Status::Active, None)
        )
        .await,
        Err(StoreError::NotFound)
    );
    assert_eq!(
        allergies::list(&db, "not-a-uuid").await,
        Err(StoreError::NotFound)
    );
    // Another patient's allergy id is not found under this patient.
    let other = patient(&db).await;
    let a = allergies::create(&db, &other, "n", None, &rec("Dust", Status::Active, None))
        .await
        .unwrap();
    assert_eq!(
        allergies::current(&db, &p, &a.id).await,
        Err(StoreError::NotFound)
    );

    let (ra, rb) = (
        rec("Dust", Status::Inactive, None),
        rec("Dust", Status::EnteredInError, None),
    );
    let (x, y) = tokio::join!(
        allergies::update(&db, &other, &a.id, 1, "a", None, &ra),
        allergies::update(&db, &other, &a.id, 1, "b", None, &rb),
    );
    assert_eq!(
        [x.is_ok(), y.is_ok()].iter().filter(|v| **v).count(),
        1,
        "{x:?} / {y:?}"
    );
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn allergy_tables_cannot_be_changed_or_removed() {
    let db = db().await;
    let p = patient(&db).await;
    let a = allergies::create(&db, &p, "n", None, &rec("Dust", Status::Active, None))
        .await
        .unwrap();
    allergies::assert_none_known(&db, &patient(&db).await, 0, "dr", None, true)
        .await
        .unwrap();
    for sql in [
        format!(
            "UPDATE allergy_versions SET author = 'x' WHERE allergy_id = '{}'",
            a.id
        ),
        format!("DELETE FROM allergy_versions WHERE allergy_id = '{}'", a.id),
        "TRUNCATE allergy_versions".to_string(),
        "UPDATE allergy_assertion_versions SET author = 'x'".to_string(),
        "DELETE FROM allergy_assertion_versions".to_string(),
        "TRUNCATE allergy_assertion_versions".to_string(),
    ] {
        let e = db.execute_unprepared(&sql).await.unwrap_err().to_string();
        assert!(e.contains("append-only"), "{sql}: {e}");
    }
}
