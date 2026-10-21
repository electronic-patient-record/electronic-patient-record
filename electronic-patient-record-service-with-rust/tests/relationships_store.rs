//! DB-gated: relationship versions are append-only and optimistic, and `lookup` returns the
//! strongest relationship in force for one subject and one patient only.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test relationships_store -- --ignored --test-threads=1
//! ```

use electronic_patient_record::patients_store::{self, StoreError};
use electronic_patient_record::policy::Relationship;
use electronic_patient_record::relationships::{self as store, Kind, RelationshipRecord, Status};
use electronic_patient_record_matcher::{HumanName, Patient};
use electronic_patient_record_migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection};

const NOW: i64 = 1_790_000_000;

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
            family: Some(format!("Zz{nanos}")),
            given: vec![],
        }],
        ..Default::default()
    };
    patients_store::create(db, "t", None, &p).await.unwrap().id
}

fn rec(subject: &str, kind: Kind, status: Status) -> RelationshipRecord {
    RelationshipRecord {
        subject: subject.into(),
        kind,
        status,
        period: None,
    }
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn lookup_is_per_subject_and_per_patient_and_takes_the_strongest() {
    let db = db().await;
    let (a, b) = (patient(&db).await, patient(&db).await);
    store::create(
        &db,
        &a,
        "admin",
        None,
        &rec("alice", Kind::CareTeam, Status::Active),
    )
    .await
    .unwrap();
    store::create(
        &db,
        &a,
        "admin",
        None,
        &rec("alice", Kind::Proxy, Status::Active),
    )
    .await
    .unwrap();
    store::create(
        &db,
        &a,
        "admin",
        None,
        &rec("bob", Kind::SelfPatient, Status::Active),
    )
    .await
    .unwrap();

    assert_eq!(
        store::lookup(&db, &a, "alice", NOW).await.unwrap(),
        Relationship::Proxy
    );
    assert_eq!(
        store::lookup(&db, &a, "bob", NOW).await.unwrap(),
        Relationship::SelfPatient
    );
    assert_eq!(
        store::lookup(&db, &a, "carol", NOW).await.unwrap(),
        Relationship::None
    );
    // The same subject has nothing to the other patient.
    assert_eq!(
        store::lookup(&db, &b, "alice", NOW).await.unwrap(),
        Relationship::None
    );
    assert_eq!(
        store::lookup(&db, &a.to_uppercase(), "bob", NOW)
            .await
            .unwrap(),
        Relationship::SelfPatient
    );
    assert_eq!(
        store::lookup(&db, "nonsense", "bob", NOW).await.unwrap(),
        Relationship::None
    );
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn ending_a_relationship_withdraws_access_and_keeps_the_history() {
    let db = db().await;
    let p = patient(&db).await;
    let v1 = store::create(
        &db,
        &p,
        "admin",
        Some("ID checked at desk"),
        &rec("dana", Kind::SelfPatient, Status::Active),
    )
    .await
    .unwrap();
    assert_eq!(
        store::lookup(&db, &p, "dana", NOW).await.unwrap(),
        Relationship::SelfPatient
    );

    // The subject and kind cannot be rewritten into someone else's access.
    assert!(matches!(
        store::update(
            &db,
            &p,
            &v1.id,
            1,
            "admin",
            None,
            &rec("eve", Kind::SelfPatient, Status::Active)
        )
        .await,
        Err(StoreError::BadQuery(_))
    ));
    assert!(matches!(
        store::update(
            &db,
            &p,
            &v1.id,
            1,
            "admin",
            None,
            &rec("dana", Kind::CareTeam, Status::Active)
        )
        .await,
        Err(StoreError::BadQuery(_))
    ));

    let v2 = store::update(
        &db,
        &p,
        &v1.id,
        1,
        "admin2",
        Some("left"),
        &rec("dana", Kind::SelfPatient, Status::Ended),
    )
    .await
    .unwrap();
    assert_eq!(v2.version, 2);
    assert_eq!(
        store::lookup(&db, &p, "dana", NOW).await.unwrap(),
        Relationship::None
    );
    assert_eq!(store::list(&db, &p).await.unwrap().len(), 1);
    let rows = db
        .query_all_raw(sea_orm::Statement::from_sql_and_values(
            sea_orm::DbBackend::Postgres,
            "SELECT version FROM relationship_versions WHERE relationship_id = $1::uuid",
            [v1.id.clone().into()],
        ))
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(
        store::update(
            &db,
            &p,
            &v1.id,
            1,
            "x",
            None,
            &rec("dana", Kind::SelfPatient, Status::Ended)
        )
        .await,
        Err(StoreError::Conflict { current: 2 })
    );
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn needs_a_patient_and_cannot_be_edited_or_removed() {
    let db = db().await;
    assert_eq!(
        store::create(
            &db,
            "00000000-0000-4000-8000-000000000000",
            "a",
            None,
            &rec("x", Kind::Proxy, Status::Active)
        )
        .await,
        Err(StoreError::NotFound)
    );
    let p = patient(&db).await;
    let r = store::create(&db, &p, "a", None, &rec("x", Kind::Proxy, Status::Active))
        .await
        .unwrap();
    for sql in [
        format!(
            "UPDATE relationship_versions SET subject = 'y' WHERE relationship_id = '{}'",
            r.id
        ),
        format!(
            "DELETE FROM relationship_versions WHERE relationship_id = '{}'",
            r.id
        ),
        "TRUNCATE relationship_versions".to_string(),
    ] {
        let e = db.execute_unprepared(&sql).await.unwrap_err().to_string();
        assert!(e.contains("append-only"), "{sql}: {e}");
    }
}
