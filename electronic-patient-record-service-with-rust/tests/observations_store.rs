//! DB-gated: observations are append-only versions; a mistake is final; the measured time is kept in UTC
//! and the site's zone is kept beside it; the flag is computed when read and never stored; a merge
//! brings the old record's observations along by provenance.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test observations_store -- --ignored --test-threads=1
//! ```

use chrono::{DateTime, Utc};
use electronic_patient_record::links;
use electronic_patient_record::observations::{AbsentReason, Flag, Range};
use electronic_patient_record::observations_store::{self as store, ObservationRecord, Status};
use electronic_patient_record::patients_store::{self, StoreError};
use electronic_patient_record_matcher::{HumanName, Patient};
use electronic_patient_record_migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, Statement, TransactionTrait};

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

async fn person(db: &DatabaseConnection, family: &str) -> String {
    let p = Patient {
        names: vec![HumanName {
            family: Some(family.into()),
            given: vec!["Sam".into()],
        }],
        birth_date: Some("1975-06-01".into()),
        ..Default::default()
    };
    patients_store::create(db, "t", None, &p).await.unwrap().id
}

fn now() -> DateTime<Utc> {
    Utc::now()
}

fn heart(value: f64, at: &str) -> ObservationRecord {
    ObservationRecord {
        code: "8867-4".into(),
        value: Some(value),
        unit: Some("/min".into()),
        absent_reason: None,
        range: None,
        critical: None,
        effective_at: at.into(),
        status: Status::Final,
    }
}

async fn count_rows(db: &DatabaseConnection, patient: &str) -> i64 {
    db.query_one_raw(Statement::from_string(
        sea_orm::DbBackend::Postgres,
        format!(
            "SELECT count(*) AS n FROM observation_versions WHERE patient_id = '{patient}'::uuid"
        ),
    ))
    .await
    .unwrap()
    .unwrap()
    .try_get::<i64>("", "n")
    .unwrap()
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn an_observation_is_versioned_corrected_and_a_mistake_is_final() {
    let db = db().await;
    let p = person(&db, &tag("obs")).await;
    let v1 = store::create(
        &db,
        &p,
        "nurse-1",
        Some("on arrival"),
        &heart(72.0, "2026-10-26T09:30:00Z"),
        "Europe/London",
        now(),
    )
    .await
    .unwrap();
    assert_eq!((v1.version, v1.record.status), (1, Status::Final));
    assert_eq!(v1.site_zone, "Europe/London");

    // A correction is a new version carrying a reason; the old one is still in the table.
    let v2 = store::update(
        &db,
        &p,
        &v1.id,
        1,
        "dr",
        Some("misread the monitor"),
        &heart(78.0, "2026-10-26T09:30:00Z"),
        "Europe/London",
        now(),
    )
    .await
    .unwrap();
    assert_eq!((v2.version, v2.record.value), (2, Some(78.0)));
    assert_eq!(count_rows(&db, &p).await, 2);
    assert_eq!(store::current(&db, &p, &v1.id).await.unwrap().version, 2);

    // A stale writer is told the current version and writes nothing.
    assert_eq!(
        store::update(
            &db,
            &p,
            &v1.id,
            1,
            "x",
            None,
            &heart(80.0, "2026-10-26T09:30:00Z"),
            "UTC",
            now()
        )
        .await,
        Err(StoreError::Conflict { current: 2 })
    );
    assert_eq!(count_rows(&db, &p).await, 2);

    // A mistake is final: it cannot be reopened, only a new observation recorded.
    let mut mistake = heart(78.0, "2026-10-26T09:30:00Z");
    mistake.status = Status::EnteredInError;
    let v3 = store::update(
        &db,
        &p,
        &v1.id,
        2,
        "dr",
        Some("wrong patient"),
        &mistake,
        "UTC",
        now(),
    )
    .await
    .unwrap();
    assert_eq!(v3.record.status, Status::EnteredInError);
    assert!(matches!(
        store::update(
            &db,
            &p,
            &v1.id,
            3,
            "dr",
            None,
            &heart(78.0, "2026-10-26T09:30:00Z"),
            "UTC",
            now()
        )
        .await,
        Err(StoreError::BadQuery(_))
    ));
    assert_eq!(count_rows(&db, &p).await, 3);

    // The history keeps every version, oldest first, each with its reason and its own value.
    let h = store::history(&db, &p, &v1.id).await.unwrap();
    assert_eq!(
        h.iter()
            .map(|v| (v.stored.version, v.stored.record.value, v.reason.as_deref()))
            .collect::<Vec<_>>(),
        vec![
            (1, Some(72.0), Some("on arrival")),
            (2, Some(78.0), Some("misread the monitor")),
            (3, Some(78.0), Some("wrong patient")),
        ]
    );
    assert_eq!(h[2].stored.record.status, Status::EnteredInError);
    // Another patient's observation, or an id that is not one, is not found.
    let other = person(&db, &tag("other")).await;
    assert_eq!(
        store::history(&db, &other, &v1.id).await,
        Err(StoreError::NotFound)
    );
    assert_eq!(
        store::history(&db, &p, "nope").await,
        Err(StoreError::NotFound)
    );
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn the_time_is_kept_in_utc_and_the_list_is_newest_measurement_first() {
    let db = db().await;
    let p = person(&db, &tag("time")).await;
    // 05:30 in New York (UTC-4) is 09:30 UTC.
    let a = store::create(
        &db,
        &p,
        "n",
        None,
        &heart(70.0, "2026-10-26T05:30:00-04:00"),
        "America/New_York",
        now(),
    )
    .await
    .unwrap();
    assert_eq!(a.record.effective_at, "2026-10-26T09:30:00.000000Z");
    let b = store::create(
        &db,
        &p,
        "n",
        None,
        &heart(71.0, "2026-10-26T10:45:00Z"),
        "America/New_York",
        now(),
    )
    .await
    .unwrap();
    let c = store::create(
        &db,
        &p,
        "n",
        None,
        &heart(69.0, "2026-10-25T23:00:00Z"),
        "America/New_York",
        now(),
    )
    .await
    .unwrap();
    let order: Vec<String> = store::list(&db, &p)
        .await
        .unwrap()
        .into_iter()
        .map(|o| o.id)
        .collect();
    assert_eq!(order, [b.id, a.id, c.id]);
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn the_flag_is_computed_when_read_and_never_stored() {
    let db = db().await;
    let p = person(&db, &tag("flag")).await;
    let mut r = heart(130.0, "2026-10-26T09:30:00Z");
    r.range = Some(Range {
        low: Some(60.0),
        high: Some(100.0),
    });
    r.critical = Some(Range {
        low: Some(40.0),
        high: Some(140.0),
    });
    let high = store::create(&db, &p, "n", None, &r, "UTC", now())
        .await
        .unwrap();
    assert_eq!(high.flag, Some(Flag::High));
    assert_eq!(
        store::current(&db, &p, &high.id).await.unwrap().flag,
        Some(Flag::High)
    );

    // The stored body has no flag, so a corrected rule cannot leave an old one behind.
    let stored = db
        .query_one_raw(Statement::from_string(
            sea_orm::DbBackend::Postgres,
            format!("SELECT body::text AS b FROM observation_versions WHERE observation_id = '{}'::uuid", high.id),
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<String>("", "b")
        .unwrap();
    assert!(!stored.contains("flag"), "{stored}");

    // No range: not assessed, never normal. No value: not assessed too, and the reason is kept.
    let bare = store::create(
        &db,
        &p,
        "n",
        None,
        &heart(70.0, "2026-10-26T09:31:00Z"),
        "UTC",
        now(),
    )
    .await
    .unwrap();
    assert_eq!(bare.flag, None);
    let mut none = heart(0.0, "2026-10-26T09:32:00Z");
    none.value = None;
    none.unit = None;
    none.absent_reason = Some(AbsentReason::Declined);
    let declined = store::create(&db, &p, "n", None, &none, "UTC", now())
        .await
        .unwrap();
    assert_eq!((declined.record.value, declined.flag), (None, None));
    assert_eq!(declined.record.absent_reason, Some(AbsentReason::Declined));
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_refused_record_writes_nothing_and_an_unknown_patient_is_not_found() {
    let db = db().await;
    let p = person(&db, &tag("refuse")).await;
    let mut wrong_unit = heart(98.6, "2026-10-26T09:30:00Z");
    wrong_unit.code = "8310-5".into();
    wrong_unit.unit = Some("[degF]".into());
    for bad in [
        wrong_unit,
        heart(0.0, "2026-10-26T09:30:00Z"),
        heart(72.0, "not a time"),
        heart(72.0, "2999-01-01T00:00:00Z"),
    ] {
        assert!(matches!(
            store::create(&db, &p, "n", None, &bad, "UTC", now()).await,
            Err(StoreError::BadQuery(_))
        ));
    }
    let mut starts_in_error = heart(72.0, "2026-10-26T09:30:00Z");
    starts_in_error.status = Status::EnteredInError;
    assert!(matches!(
        store::create(&db, &p, "n", None, &starts_in_error, "UTC", now()).await,
        Err(StoreError::BadQuery(_))
    ));
    assert_eq!(count_rows(&db, &p).await, 0);
    assert_eq!(
        store::create(
            &db,
            "00000000-0000-4000-8000-000000000000",
            "n",
            None,
            &heart(72.0, "2026-10-26T09:30:00Z"),
            "UTC",
            now()
        )
        .await,
        Err(StoreError::NotFound)
    );
    assert_eq!(
        store::list(&db, "not-a-uuid").await,
        Err(StoreError::NotFound)
    );
    // Another patient's observation id is not found under this patient.
    let other = person(&db, &tag("other")).await;
    let o = store::create(
        &db,
        &other,
        "n",
        None,
        &heart(72.0, "2026-10-26T09:30:00Z"),
        "UTC",
        now(),
    )
    .await
    .unwrap();
    assert_eq!(
        store::current(&db, &p, &o.id).await,
        Err(StoreError::NotFound)
    );
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn two_writers_of_the_same_version_get_one_winner() {
    let db = db().await;
    let p = person(&db, &tag("race")).await;
    let o = store::create(
        &db,
        &p,
        "n",
        None,
        &heart(72.0, "2026-10-26T09:30:00Z"),
        "UTC",
        now(),
    )
    .await
    .unwrap();
    let (ra, rb) = (
        heart(73.0, "2026-10-26T09:30:00Z"),
        heart(74.0, "2026-10-26T09:30:00Z"),
    );
    let (x, y) = tokio::join!(
        store::update(&db, &p, &o.id, 1, "a", None, &ra, "UTC", now()),
        store::update(&db, &p, &o.id, 1, "b", None, &rb, "UTC", now()),
    );
    assert_eq!(
        [x.is_ok(), y.is_ok()].iter().filter(|v| **v).count(),
        1,
        "{x:?} / {y:?}"
    );
    assert_eq!(count_rows(&db, &p).await, 2);
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn observation_tables_cannot_be_changed_or_removed() {
    let db = db().await;
    let p = person(&db, &tag("immutable")).await;
    let o = store::create(
        &db,
        &p,
        "n",
        None,
        &heart(72.0, "2026-10-26T09:30:00Z"),
        "UTC",
        now(),
    )
    .await
    .unwrap();
    for sql in [
        format!(
            "UPDATE observation_versions SET author = 'x' WHERE observation_id = '{}'",
            o.id
        ),
        format!(
            "DELETE FROM observation_versions WHERE observation_id = '{}'",
            o.id
        ),
        "TRUNCATE observation_versions".to_string(),
    ] {
        let e = db.execute_unprepared(&sql).await.unwrap_err().to_string();
        assert!(e.contains("append-only"), "{sql}: {e}");
    }
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn observations_follow_a_merge_by_provenance_and_return_on_unmerge() {
    let db = db().await;
    let family = tag("merge");
    let (t, s) = (person(&db, &family).await, person(&db, &family).await);
    let on_source = store::create(
        &db,
        &s,
        "n",
        None,
        &heart(72.0, "2026-10-26T09:30:00Z"),
        "UTC",
        now(),
    )
    .await
    .unwrap();
    let on_target = store::create(
        &db,
        &t,
        "n",
        None,
        &heart(80.0, "2026-10-26T10:30:00Z"),
        "UTC",
        now(),
    )
    .await
    .unwrap();

    let tx = db.begin().await.unwrap();
    links::merge(&tx, &t, &s, "admin", "same person")
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let both = store::list(&db, &t).await.unwrap();
    assert_eq!(both.len(), 2);
    assert!(
        both.iter()
            .any(|o| o.id == on_source.id && o.patient_id == s)
    );
    assert!(
        both.iter()
            .any(|o| o.id == on_target.id && o.patient_id == t)
    );
    // The merged-away record still reads as itself.
    assert_eq!(store::list(&db, &s).await.unwrap().len(), 1);
    // An observation found through the survivor is corrected there and keeps its own patient.
    let v2 = store::update(
        &db,
        &t,
        &on_source.id,
        1,
        "dr",
        Some("typo"),
        &heart(73.0, "2026-10-26T09:30:00Z"),
        "UTC",
        now(),
    )
    .await
    .unwrap();
    assert_eq!((v2.version, v2.patient_id.as_str()), (2, s.as_str()));

    let tx = db.begin().await.unwrap();
    links::unmerge(&tx, &t, &s, "admin", "mistake")
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(store::list(&db, &t).await.unwrap().len(), 1);
    assert_eq!(store::list(&db, &s).await.unwrap().len(), 1);
}
