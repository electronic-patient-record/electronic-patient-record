//! DB-gated: death assertions (BV-1). The store keeps every version, refuses what the rules refuse,
//! and gives one answer that a merge carries across.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test death_store -- --ignored --test-threads=1
//! ```

use chrono::Utc;
use electronic_patient_record::death::{self, Actor, Answer, Occurred, Source, Status, Step};
use electronic_patient_record::death_store as store;
use electronic_patient_record::links;
use electronic_patient_record::patients_store::{self, StoreError};
use electronic_patient_record_matcher::{HumanName, Patient};
use sqlx::Row;
use sqlx::{AssertSqlSafe, PgPool};

async fn db() -> PgPool {
    let url = std::env::var("DB_URL").expect("DB_URL must be set for --ignored tests");
    let db = PgPool::connect(&url).await.expect("connect");
    electronic_patient_record::MIGRATOR
        .run(&db)
        .await
        .expect("migrate");
    db
}

fn someone() -> Patient {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    Patient {
        names: vec![HumanName {
            family: Some(format!("Zzdeath{nanos}")),
            given: vec!["Sam".into()],
        }],
        birth_date: Some("1975-06-01".into()),
        ..Default::default()
    }
}

async fn person(db: &PgPool) -> String {
    patients_store::create(db, "t", None, &someone())
        .await
        .unwrap()
        .id
}

fn from_feed() -> death::Assertion {
    death::report(
        Source::OtherSystem,
        Actor::Feed("mllp:test".into()),
        Occurred::Known {
            at: "2026-11-01".into(),
        },
        "HL7 ADT",
        Utc::now(),
    )
    .unwrap()
}

fn nurse(to: Status) -> Step {
    Step {
        to,
        by: Actor::Person("nurse-1".into()),
        reason: "checked".into(),
    }
}

#[tokio::test]
#[ignore = "needs DB_URL"]
async fn a_report_holds_contact_until_a_person_withdraws_it_and_the_history_stays() {
    let db = db().await;
    let id = person(&db).await;
    assert_eq!(
        store::answer(&db, &id).await.unwrap(),
        Answer::NotKnownToHaveDied
    );

    let made = store::report(&db, &id, &from_feed()).await.unwrap();
    assert_eq!((made.version, made.author.as_str()), (1, "mllp:test"));
    assert!(store::answer(&db, &id).await.unwrap().holds_contact());

    // A stale version is a conflict, and a feed cannot move it on.
    assert_eq!(
        store::advance(&db, &id, &made.id, 7, &nurse(Status::Verified)).await,
        Err(StoreError::Conflict { current: 1 })
    );
    let by_feed = Step {
        by: Actor::Feed("mllp:test".into()),
        ..nurse(Status::Verified)
    };
    assert!(matches!(
        store::advance(&db, &id, &made.id, 1, &by_feed).await,
        Err(StoreError::BadQuery(_))
    ));

    let gone = store::advance(&db, &id, &made.id, 1, &nurse(Status::WithdrawnInError))
        .await
        .unwrap();
    assert_eq!(gone.version, 2);
    assert_eq!(
        store::answer(&db, &id).await.unwrap(),
        Answer::NotKnownToHaveDied
    );
    // The list still shows it, and both versions are kept.
    assert_eq!(store::list(&db, &id).await.unwrap().len(), 1);
    let row = sqlx::query(
        "SELECT count(*) AS n FROM death_assertion_versions WHERE assertion_id = $1::uuid",
    )
    .bind(made.id.clone())
    .fetch_optional(&db)
    .await
    .unwrap()
    .unwrap();
    assert_eq!(row.try_get::<i64, _>("n").unwrap(), 2);
    // A withdrawal is final.
    assert!(matches!(
        store::advance(&db, &id, &made.id, 2, &nurse(Status::Verified)).await,
        Err(StoreError::BadQuery(_))
    ));
}

#[tokio::test]
#[ignore = "needs DB_URL"]
async fn the_table_is_append_only() {
    let db = db().await;
    let id = person(&db).await;
    store::report(&db, &id, &from_feed()).await.unwrap();
    for sql in [
        "UPDATE death_assertion_versions SET status = 'withdrawn-in-error'",
        "DELETE FROM death_assertion_versions",
        "TRUNCATE death_assertion_versions",
    ] {
        let refused = sqlx::raw_sql(AssertSqlSafe(sql.to_string()))
            .execute(&db)
            .await
            .unwrap_err()
            .to_string();
        assert!(refused.contains("append-only"), "{sql}: {refused}");
    }
}

#[tokio::test]
#[ignore = "needs DB_URL"]
async fn a_report_on_a_merged_away_record_counts_for_the_surviving_one() {
    let db = db().await;
    let shared = someone();
    let target = patients_store::create(&db, "t", None, &shared)
        .await
        .unwrap()
        .id;
    let source = patients_store::create(&db, "t", None, &shared)
        .await
        .unwrap()
        .id;
    store::report(&db, &source, &from_feed()).await.unwrap();
    links::merge(&db, &target, &source, "t", "the same person")
        .await
        .unwrap();
    assert!(store::answer(&db, &target).await.unwrap().holds_contact());
}

#[tokio::test]
#[ignore = "needs DB_URL"]
async fn no_such_patient_or_assertion_is_a_miss() {
    let db = db().await;
    let none = "00000000-0000-4000-8000-000000000000";
    assert_eq!(
        store::report(&db, none, &from_feed()).await,
        Err(StoreError::NotFound)
    );
    let id = person(&db).await;
    assert_eq!(
        store::advance(&db, &id, none, 1, &nurse(Status::Verified)).await,
        Err(StoreError::NotFound)
    );
}
