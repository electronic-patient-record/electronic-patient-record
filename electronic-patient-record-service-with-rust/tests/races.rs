//! DB-gated: two people at one moment. A rule that looks at what is already stored, and then writes, holds only if the
//! second writer waits for the first: in each test the first has written and not yet committed, and the second must wait
//! and then be judged by what the first stored. (Emergency grants: `break_glass_store.rs`. Problem causes: `entries_store.rs`.)
//!
//! ```text
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test main races:: -- --ignored --test-threads=1
//! ```

use electronic_patient_record::adjustments::{Adjustment, Status};
use electronic_patient_record::patients_store::{self, StoreError};
use electronic_patient_record::suspicion::{Outcome, Suspicion};
use electronic_patient_record::{adjustments_store, referrals_store, suspicions_store};
use electronic_patient_record_matcher::{HumanName, Patient};
use serde_json::json;
use sqlx::PgPool;
use std::time::Duration;

async fn db() -> PgPool {
    let db = PgPool::connect(&std::env::var("DB_URL").expect("DB_URL must be set"))
        .await
        .expect("connect");
    electronic_patient_record::MIGRATOR
        .run(&db)
        .await
        .expect("migrate");
    db
}

async fn patient(db: &PgPool) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let p = Patient {
        names: vec![HumanName {
            family: Some(format!("Zzrace{nanos}")),
            given: vec!["Em".into()],
        }],
        birth_date: Some("1985-01-01".into()),
        ..Default::default()
    };
    patients_store::create(db, "t", None, &p).await.unwrap().id
}

/// Wait long enough to see that `task` is still waiting, then commit `first` and give the task's answer.
async fn waits<T: Send + 'static>(
    first: sqlx::Transaction<'static, sqlx::Postgres>,
    task: tokio::task::JoinHandle<T>,
) -> T {
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(!task.is_finished(), "the second did not wait for the first");
    first.commit().await.unwrap();
    tokio::time::timeout(Duration::from_secs(5), task)
        .await
        .expect("the second goes on when the first is done")
        .unwrap()
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn one_need_recorded_by_two_people_is_stored_once() {
    let db = db().await;
    let p = patient(&db).await;
    let today = electronic_patient_record::clock::parse_day("2026-11-25").unwrap();
    let need = Adjustment {
        code: "interpreter-spoken".into(),
        note: Some("Polish".into()),
        status: Status::Active,
        review_by: None,
        reason: "they asked".into(),
    };
    let mut first = db.begin().await.unwrap();
    adjustments_store::record(&mut *first, &p, "nurse-1", &need, today)
        .await
        .unwrap();
    let (other, at, same) = (db.clone(), p.clone(), need.clone());
    let second = tokio::spawn(async move {
        adjustments_store::record(&other, &at, "nurse-2", &same, today).await
    });
    // Another patient's need does not wait.
    let elsewhere = patient(&db).await;
    tokio::time::timeout(
        Duration::from_secs(5),
        adjustments_store::record(&db, &elsewhere, "nurse-3", &need, today),
    )
    .await
    .expect("another patient does not wait")
    .unwrap();
    match waits(first, second).await {
        Err(StoreError::BadQuery(m)) => assert!(m.contains("already recorded"), "{m}"),
        other => panic!("{other:?}"),
    }
    let all = adjustments_store::list(&db, &p, today).await.unwrap();
    assert_eq!(all.len(), 1);
}

async fn raised(db: &PgPool) -> (String, String, Suspicion, jiff::Timestamp) {
    let p = patient(db).await;
    let now: jiff::Timestamp = "2026-11-25T12:00:00Z".parse().unwrap();
    let referral = serde_json::from_value(json!({
        "recipient": "Cardiology", "reason": "r", "question": "q", "priority": "urgent"
    }))
    .unwrap();
    let id = referrals_store::request(db, "dr-1", &p, &referral, now)
        .await
        .unwrap()
        .id;
    let s: Suspicion = serde_json::from_value(json!({
        "condition": "Suspected lower gastrointestinal cancer",
        "point_of_suspicion": "2026-11-20T09:30:00Z",
        "source": "clinician", "by": "Dr A",
        "grounds": { "kind": "criterion", "detail": "NG12 criterion met" },
        "patient_told": "told",
        "pathway": "suspected-cancer-wales", "pathway_version": 1
    }))
    .unwrap();
    suspicions_store::raise(db, &p, &id, "dr-1", &s, now)
        .await
        .unwrap();
    (p, id, s, now)
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_suspicion_is_not_amended_while_another_person_ends_it() {
    let db = db().await;
    let (p, id, s, now) = raised(&db).await;
    let ruled_out: Outcome = serde_json::from_value(json!({
        "kind": "ruled-out", "by": "dr-2", "detail": "a test"
    }))
    .unwrap();
    let mut first = db.begin().await.unwrap();
    suspicions_store::end(&mut *first, &p, &id, "dr-2", 1, &ruled_out, now)
        .await
        .unwrap();
    let (other, pp, rr) = (db.clone(), p.clone(), id.clone());
    let changed = Suspicion {
        condition: "another condition".into(),
        ..s
    };
    let second = tokio::spawn(async move {
        suspicions_store::amend(&other, &pp, &rr, "dr-1", 1, Some("seen"), &changed, now).await
    });
    match waits(first, second).await {
        Err(StoreError::BadQuery(m)) => assert!(m.contains("has an outcome"), "{m}"),
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_suspicion_ended_by_two_people_has_one_outcome() {
    let db = db().await;
    let (p, id, _, now) = raised(&db).await;
    let outcome = |kind: &str| -> Outcome {
        serde_json::from_value(json!({
            "kind": kind, "detail": "a test",
            "by": (kind == "ruled-out").then_some("dr-2"),
            "diagnosis": (kind == "confirmed").then_some("the condition"),
        }))
        .unwrap()
    };
    let mut first = db.begin().await.unwrap();
    suspicions_store::end(&mut *first, &p, &id, "dr-2", 1, &outcome("ruled-out"), now)
        .await
        .unwrap();
    let (other, pp, rr, confirmed) = (db.clone(), p.clone(), id.clone(), outcome("confirmed"));
    let second = tokio::spawn(async move {
        suspicions_store::end(&other, &pp, &rr, "dr-3", 1, &confirmed, now).await
    });
    match waits(first, second).await {
        Err(StoreError::BadQuery(m)) => assert!(m.contains("already has an outcome"), "{m}"),
        other => panic!("{other:?}"),
    }
}
