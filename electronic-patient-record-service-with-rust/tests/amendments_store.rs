//! DB-gated (RC-4): an amendment request is append-only versions; the rules of `amendment.rs` hold
//! through the store; a correction can only be applied when the record really has a later version;
//! nothing can be edited or deleted; a reader's notice follows the request.
//!
//! ```text
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test amendments_store -- --ignored --test-threads=1
//! ```

use chrono::Utc;
use electronic_patient_record::amendment::{Event, Ground, Notice, State};
use electronic_patient_record::amendments_store as store;
use electronic_patient_record::observations_store::{self as obs, ObservationRecord, Status};
use electronic_patient_record::patients_store::{self, StoreError};
use electronic_patient_record_matcher::{HumanName, Patient};
use electronic_patient_record_migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, Statement};

async fn db() -> DatabaseConnection {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for --ignored tests");
    let db = Database::connect(url).await.expect("connect");
    Migrator::up(&db, None).await.expect("migrate");
    db
}

async fn person(db: &DatabaseConnection) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let p = Patient {
        names: vec![HumanName {
            family: Some(format!("Zzam{nanos}")),
            given: vec!["Sam".into()],
        }],
        birth_date: Some("1975-06-01".into()),
        ..Default::default()
    };
    patients_store::create(db, "t", None, &p).await.unwrap().id
}

fn heart(value: f64) -> ObservationRecord {
    ObservationRecord {
        code: "8867-4".into(),
        value: Some(value),
        unit: Some("/min".into()),
        absent_reason: None,
        range: None,
        critical: None,
        effective_at: "2026-10-26T09:30:00Z".into(),
        status: Status::Final,
    }
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_request_runs_its_course_through_the_store_and_every_step_is_kept() {
    let db = db().await;
    let p = person(&db).await;
    let o = obs::create(&db, &p, "nurse", None, &heart(172.0), "UTC", Utc::now())
        .await
        .unwrap();

    // A kind that cannot be amended, a version the record lacks, and a blank text are refused.
    for (kind, version, text) in [
        ("Patient", 1, "x"),
        ("Observation", 2, "x"),
        ("Observation", 0, "x"),
        ("Observation", 1, "  "),
    ] {
        let e = store::open(&db, &p, "pat", kind, &o.id, version, text, Utc::now())
            .await
            .unwrap_err();
        assert!(
            matches!(e, StoreError::BadQuery(_)),
            "{kind} {version}: {e:?}"
        );
    }
    let e = store::open(&db, &p, "pat", "Observation", &p, 1, "x", Utc::now())
        .await
        .unwrap_err();
    assert!(matches!(e, StoreError::NotFound), "{e:?}");

    let a = store::open(
        &db,
        &p,
        "pat",
        "Observation",
        &o.id,
        1,
        "It was 72",
        Utc::now(),
    )
    .await
    .unwrap();
    assert_eq!((a.version, a.request.state.word()), (1, "requested"));
    assert_eq!(
        store::notices(&db, &p, "Observation", &o.id).await.unwrap(),
        vec![Notice::Pending]
    );

    // The rules hold through the store: not by the requester, and a stale version is a conflict.
    let accept = |by: &str| Event::Accept {
        by: by.into(),
        reason: "checked".into(),
    };
    let e = store::advance(&db, &p, &a.id, 1, "pat", accept("pat"), Utc::now())
        .await
        .unwrap_err();
    assert!(matches!(e, StoreError::BadQuery(_)), "{e:?}");
    let e = store::advance(&db, &p, &a.id, 5, "dr", accept("dr"), Utc::now())
        .await
        .unwrap_err();
    assert!(matches!(e, StoreError::Conflict { current: 1 }), "{e:?}");

    let a2 = store::advance(&db, &p, &a.id, 1, "dr", accept("dr"), Utc::now())
        .await
        .unwrap();
    assert_eq!(a2.request.state.word(), "accepted");

    // The correction cannot be applied until the record has it.
    let apply = Event::Apply { version: 2 };
    let e = store::advance(&db, &p, &a.id, 2, "dr", apply.clone(), Utc::now())
        .await
        .unwrap_err();
    assert!(matches!(e, StoreError::BadQuery(_)), "{e:?}");
    obs::update(
        &db,
        &p,
        &o.id,
        1,
        "dr",
        Some("amendment"),
        &heart(72.0),
        "UTC",
        Utc::now(),
    )
    .await
    .unwrap();
    let a3 = store::advance(&db, &p, &a.id, 2, "dr", apply, Utc::now())
        .await
        .unwrap();
    assert_eq!(a3.request.state, State::Applied { version: 2 });
    assert_eq!(
        store::notices(&db, &p, "Observation", &o.id).await.unwrap(),
        vec![Notice::Amended { version: 2 }]
    );
    assert_eq!(store::list(&db, &p).await.unwrap().len(), 1);

    // Every step is kept, with the event that made it.
    let n: i64 = db
        .query_one_raw(Statement::from_string(
            sea_orm::DbBackend::Postgres,
            format!(
                "SELECT count(*) AS n FROM amendment_versions WHERE amendment_id = '{}'::uuid",
                a.id
            ),
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "n")
        .unwrap();
    assert_eq!(n, 3);
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_refusal_keeps_the_patients_statement_with_the_record_and_nothing_can_be_edited() {
    let db = db().await;
    let p = person(&db).await;
    let o = obs::create(&db, &p, "nurse", None, &heart(80.0), "UTC", Utc::now())
        .await
        .unwrap();
    let a = store::open(&db, &p, "pat", "Observation", &o.id, 1, "Wrong", Utc::now())
        .await
        .unwrap();
    let denied = store::advance(
        &db,
        &p,
        &a.id,
        1,
        "dr",
        Event::Deny {
            by: "dr".into(),
            ground: Ground::AccurateAndComplete,
            reason: "monitor print-out".into(),
        },
        Utc::now(),
    )
    .await
    .unwrap();
    let said = store::advance(
        &db,
        &p,
        &a.id,
        denied.version,
        "pat",
        Event::Disagree {
            by: "pat".into(),
            statement: "I was not on that ward".into(),
        },
        Utc::now(),
    )
    .await
    .unwrap();
    assert_eq!(
        store::notices(&db, &p, "Observation", &o.id).await.unwrap(),
        vec![Notice::Refused {
            ground: Ground::AccurateAndComplete,
            disagreement: Some("I was not on that ward".into())
        }]
    );
    // A second statement is refused, and a decided request cannot be withdrawn.
    let e = store::advance(
        &db,
        &p,
        &a.id,
        said.version,
        "pat",
        Event::Withdraw { by: "pat".into() },
        Utc::now(),
    )
    .await
    .unwrap_err();
    assert!(matches!(e, StoreError::BadQuery(_)), "{e:?}");

    // The table refuses edits and deletes.
    for sql in [
        format!(
            "UPDATE amendment_versions SET author = 'x' WHERE amendment_id = '{}'::uuid",
            a.id
        ),
        format!(
            "DELETE FROM amendment_versions WHERE amendment_id = '{}'::uuid",
            a.id
        ),
        "TRUNCATE amendment_versions".to_string(),
    ] {
        let e = db.execute_unprepared(&sql).await.unwrap_err();
        assert!(e.to_string().contains("append-only"), "{sql}: {e}");
    }
}
