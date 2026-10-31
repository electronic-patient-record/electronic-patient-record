//! DB-gated (OS-3): an event enqueued in a transaction that rolls back is never delivered; one that
//! commits is delivered once; a delivery the policy or the patient's consent refuses is not made,
//! is audited and is not retried; a failing sink is retried a fixed number of times; and the database
//! refuses to edit or delete an event or an attempt.
//!
//! ```text
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test outbox_store -- --ignored --test-threads=1
//! ```

use async_trait::async_trait;
use electronic_patient_record::audit::Chain;
use electronic_patient_record::consent::{ConsentRecord, Effect, Kind, Provision, Status};
use electronic_patient_record::consents_store;
use electronic_patient_record::metrics;
use electronic_patient_record::outbox::{self, Event, MAX_FAILURES, NewEvent, Sink, Sinks};
use electronic_patient_record::patients_store;
use electronic_patient_record::policy::{Purpose, Resource, Role};
use electronic_patient_record_matcher::{HumanName, Patient};
use electronic_patient_record_migration::{Migrator, MigratorTrait};
use sea_orm::{
    ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement, TransactionTrait,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

async fn db() -> DatabaseConnection {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for --ignored tests");
    let db = Database::connect(url).await.expect("connect");
    Migrator::up(&db, None).await.expect("migrate");
    db
}

fn nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

/// A sink that counts its calls and answers as told.
struct Counting {
    calls: Arc<AtomicU32>,
    fail: bool,
}

#[async_trait]
impl Sink for Counting {
    async fn deliver(&self, _event: &Event) -> Result<(), String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.fail {
            Err("sink down".into())
        } else {
            Ok(())
        }
    }
}

fn sinks(name: &str, fail: bool) -> (Sinks, Arc<AtomicU32>) {
    let calls = Arc::new(AtomicU32::new(0));
    let mut all = Sinks::new();
    all.insert(
        name.to_string(),
        Box::new(Counting {
            calls: calls.clone(),
            fail,
        }),
    );
    (all, calls)
}

async fn patient(db: &DatabaseConnection) -> String {
    let p = Patient {
        names: vec![HumanName {
            family: Some(format!("Zz{}", nanos())),
            given: vec![],
        }],
        ..Default::default()
    };
    patients_store::create(db, "t", None, &p).await.unwrap().id
}

async fn count(db: &DatabaseConnection, sql: &str, arg: &str) -> i64 {
    db.query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql,
        vec![arg.into()],
    ))
    .await
    .unwrap()
    .unwrap()
    .try_get("", "n")
    .unwrap()
}

async fn attempts(db: &DatabaseConnection, id: &str, outcome: &str) -> i64 {
    db.query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT count(*) AS n FROM outbox_attempts WHERE event_id = $1::uuid AND outcome = $2",
        vec![id.into(), outcome.into()],
    ))
    .await
    .unwrap()
    .unwrap()
    .try_get("", "n")
    .unwrap()
}

/// An event a receptionist-role recipient may receive: identity data, for operations.
fn event<'a>(
    recipient: &'a str,
    patient: Option<&'a str>,
    payload: &'a serde_json::Value,
) -> NewEvent<'a> {
    NewEvent {
        kind: "patient-registered",
        recipient,
        roles: &[Role::Receptionist],
        purpose: Purpose::Operations,
        resource: Resource::PatientIdentity,
        patient_id: patient,
        payload,
    }
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_rolled_back_event_is_never_delivered_and_a_committed_one_is_delivered_once() {
    let db = db().await;
    let recipient = format!("sink-{}", nanos());
    let payload = serde_json::json!({"n": 1});

    let tx = db.begin().await.unwrap();
    let lost = outbox::enqueue(&tx, event(&recipient, None, &payload))
        .await
        .unwrap();
    tx.rollback().await.unwrap();

    let tx = db.begin().await.unwrap();
    let kept = outbox::enqueue(&tx, event(&recipient, None, &payload))
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let (all, calls) = sinks(&recipient, false);
    let chain = Chain::unkeyed();
    let before = metrics::OUTBOX_DELIVERED.get();
    outbox::run_once(&db, &chain, &all, 100).await.unwrap();
    outbox::run_once(&db, &chain, &all, 100).await.unwrap();

    assert_eq!(calls.load(Ordering::SeqCst), 1, "delivered exactly once");
    assert!(metrics::OUTBOX_DELIVERED.get() > before, "counted");
    assert_eq!(attempts(&db, &kept, "delivered").await, 1);
    assert_eq!(
        count(
            &db,
            "SELECT count(*) AS n FROM outbox_events WHERE id = $1::uuid",
            &lost
        )
        .await,
        0,
        "the rolled back event does not exist"
    );
    let audited = count(
        &db,
        "SELECT count(*) AS n FROM audit_events WHERE resource_ref = $1",
        &format!("outbox/{kept}"),
    )
    .await;
    assert_eq!(audited, 1, "the delivery is audited");
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_patients_refusal_stops_the_delivery_and_is_audited_once() {
    let db = db().await;
    let recipient = format!("sink-{}", nanos());
    let p = patient(&db).await;
    consents_store::create(
        &db,
        &p,
        "t",
        None,
        &ConsentRecord {
            kind: Kind::General,
            status: Status::Active,
            provisions: vec![Provision {
                effect: Effect::Deny,
                actors: vec![],
                purposes: vec!["operations".into()],
                resources: vec![],
                labels: vec![],
                period: None,
            }],
            period: None,
        },
    )
    .await
    .unwrap();
    let payload = serde_json::json!({"n": 2});
    let id = outbox::enqueue(&db, event(&recipient, Some(&p), &payload))
        .await
        .unwrap();

    let (all, calls) = sinks(&recipient, false);
    let chain = Chain::unkeyed();
    outbox::run_once(&db, &chain, &all, 100).await.unwrap();
    outbox::run_once(&db, &chain, &all, 100).await.unwrap();

    assert_eq!(calls.load(Ordering::SeqCst), 0, "the sink was never called");
    assert_eq!(
        attempts(&db, &id, "refused").await,
        1,
        "refused once, not retried"
    );
    assert_eq!(attempts(&db, &id, "delivered").await, 0);
    let audited = count(
        &db,
        "SELECT count(*) AS n FROM audit_events WHERE resource_ref = $1 AND outcome = 4",
        &format!("outbox/{id}"),
    )
    .await;
    assert_eq!(audited, 1, "the refusal is audited");
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_recipient_the_policy_does_not_allow_gets_nothing() {
    let db = db().await;
    let recipient = format!("sink-{}", nanos());
    let payload = serde_json::json!({"n": 3});
    let mut e = event(&recipient, None, &payload);
    e.roles = &[Role::Integrator];
    e.resource = Resource::Observation;
    let id = outbox::enqueue(&db, e).await.unwrap();

    let (all, calls) = sinks(&recipient, false);
    outbox::run_once(&db, &Chain::unkeyed(), &all, 100)
        .await
        .unwrap();

    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(attempts(&db, &id, "refused").await, 1);
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_failing_sink_is_retried_a_fixed_number_of_times_then_left() {
    let db = db().await;
    let recipient = format!("sink-{}", nanos());
    let payload = serde_json::json!({"n": 4});
    let id = outbox::enqueue(&db, event(&recipient, None, &payload))
        .await
        .unwrap();

    let (all, calls) = sinks(&recipient, true);
    for _ in 0..(MAX_FAILURES + 3) {
        outbox::run_once_with(&db, &Chain::unkeyed(), &all, 100, 0)
            .await
            .unwrap();
    }

    assert_eq!(i64::from(calls.load(Ordering::SeqCst)), MAX_FAILURES);
    assert_eq!(attempts(&db, &id, "failed").await, MAX_FAILURES);
    assert_eq!(attempts(&db, &id, "delivered").await, 0);
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn an_event_for_a_recipient_with_no_sink_is_not_lost() {
    let db = db().await;
    let recipient = format!("nobody-{}", nanos());
    let payload = serde_json::json!({"n": 5});
    let id = outbox::enqueue(&db, event(&recipient, None, &payload))
        .await
        .unwrap();

    outbox::run_once(&db, &Chain::unkeyed(), &Sinks::new(), 100)
        .await
        .unwrap();

    assert_eq!(attempts(&db, &id, "failed").await, 1);
    assert_eq!(attempts(&db, &id, "delivered").await, 0);
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn events_and_attempts_cannot_be_edited_deleted_or_truncated() {
    let db = db().await;
    let recipient = format!("sink-{}", nanos());
    let payload = serde_json::json!({"n": 6});
    let id = outbox::enqueue(&db, event(&recipient, None, &payload))
        .await
        .unwrap();
    let (all, _) = sinks(&recipient, false);
    outbox::run_once(&db, &Chain::unkeyed(), &all, 100)
        .await
        .unwrap();

    for sql in [
        format!("UPDATE outbox_events SET kind = 'x' WHERE id = '{id}'"),
        format!("DELETE FROM outbox_events WHERE id = '{id}'"),
        format!("UPDATE outbox_attempts SET outcome = 'failed' WHERE event_id = '{id}'"),
        format!("DELETE FROM outbox_attempts WHERE event_id = '{id}'"),
        "TRUNCATE outbox_attempts".to_string(),
        "TRUNCATE outbox_events CASCADE".to_string(),
    ] {
        let e = db.execute_unprepared(&sql).await.unwrap_err();
        assert!(e.to_string().contains("append-only"), "{sql}: {e}");
    }
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn the_loop_delivers_without_being_asked_again() {
    let db = db().await;
    let recipient = format!("sink-{}", nanos());
    let payload = serde_json::json!({"n": 7});
    let id = outbox::enqueue(&db, event(&recipient, None, &payload))
        .await
        .unwrap();
    let (all, calls) = sinks(&recipient, false);
    let chain: &'static Chain = Box::leak(Box::new(Chain::unkeyed()));
    let task = tokio::spawn(outbox::run_loop(
        db.clone(),
        chain,
        all,
        std::time::Duration::from_millis(50),
    ));
    for _ in 0..100 {
        if attempts(&db, &id, "delivered").await == 1 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    task.abort();
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "delivered once by the loop"
    );
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_failed_event_waits_before_its_retry() {
    let db = db().await;
    let recipient = format!("sink-{}", nanos());
    let payload = serde_json::json!({"n": 8});
    let id = outbox::enqueue(&db, event(&recipient, None, &payload))
        .await
        .unwrap();
    let (all, calls) = sinks(&recipient, true);
    let chain = Chain::unkeyed();
    for _ in 0..3 {
        outbox::run_once_with(&db, &chain, &all, 100, 3600)
            .await
            .unwrap();
    }
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "the retry is not due for an hour"
    );
    assert_eq!(attempts(&db, &id, "failed").await, 1);
}

/// Add `n` failed attempts to an event, each stamped `age_secs` ago (the table is append-only, so the
/// clock is set on insert).
async fn fail_in_the_past(db: &DatabaseConnection, id: &str, n: u32, age_secs: i64) {
    for _ in 0..n {
        db.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO outbox_attempts (event_id, attempted_at, outcome, reason)
             VALUES ($1::uuid, now() - make_interval(secs => $2::double precision), 'failed', 'earlier')",
            vec![id.into(), age_secs.into()],
        ))
        .await
        .unwrap();
    }
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn the_wait_after_a_failure_doubles_with_each_failure() {
    let db = db().await;
    let chain = Chain::unkeyed();
    // With a 30 second base the wait is 30, 60, 120, 240 seconds after the 1st to 4th failure.
    // Each case: (failures so far, age of the last in seconds, due).
    for (failures, age, due) in [
        (1, 20, false),
        (1, 40, true),
        (2, 50, false),
        (2, 70, true),
        (3, 110, false),
        (3, 130, true),
        (4, 230, false),
        (4, 250, true),
    ] {
        let recipient = format!("sink-{}", nanos());
        let payload = serde_json::json!({"n": 9});
        let id = outbox::enqueue(&db, event(&recipient, None, &payload))
            .await
            .unwrap();
        fail_in_the_past(&db, &id, failures, age).await;
        let (all, calls) = sinks(&recipient, false);
        outbox::run_once_with(&db, &chain, &all, 100, 30)
            .await
            .unwrap();
        assert_eq!(
            calls.load(Ordering::SeqCst),
            u32::from(due),
            "{failures} failures, last {age}s ago"
        );
    }
}
