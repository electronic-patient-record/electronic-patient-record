//! DB-gated (OS-3), through `axa-outbox`'s relay: an event enqueued in a transaction that rolls back is
//! never delivered; one that commits is delivered once and audited; a delivery the policy or the
//! patient's consent refuses is not made, is audited and is final; a report of death holds an event
//! until it is withdrawn, and holds back no other patient's; a failing sink is tried a fixed number of
//! times and then left for a person, who can release it; a changed event is never published; an
//! oversized payload is refused; an event to a patient that names a suspected condition is refused
//! unless the record says the patient was told (RS-4); and the database refuses to edit or delete the record, and refuses new
//! rows in the closed tables of migration 17.
//!
//! The waits before a retry, a lost relay and the order per key are `axa-outbox`'s and are tested there.
//!
//! ```text
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test outbox_store -- --ignored --test-threads=1
//! ```

use async_trait::async_trait;
use axa::outbox::{Failure, Outbox, OutboxOptions};
use electronic_patient_record::consent::{ConsentRecord, Effect, Kind, Provision, Status};
use electronic_patient_record::consents_store;
use electronic_patient_record::death::{
    self, Actor, Occurred, Source, Status as DeathStatus, Step,
};
use electronic_patient_record::death_store;
use electronic_patient_record::metrics;
use electronic_patient_record::outbox::{self, Delivery, MAX_ATTEMPTS, NewEvent, Sink, Sinks};
use electronic_patient_record::patients_store;
use electronic_patient_record::policy::{Purpose, Resource, Role};
use electronic_patient_record::referrals_store::{self, Priority, Referral};
use electronic_patient_record::suspicion::{
    Grounds, GroundsKind, PatientTold, Source as Raised, Suspicion,
};
use electronic_patient_record::suspicions_store;
use electronic_patient_record_matcher::{HumanName, Patient};
use sqlx::{AssertSqlSafe, PgPool, Row};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use tokio::sync::watch;

fn url() -> String {
    std::env::var("DB_URL").expect("DB_URL must be set for --ignored tests")
}

async fn db() -> PgPool {
    let db = PgPool::connect(&url()).await.expect("connect");
    electronic_patient_record::MIGRATOR
        .run(&db)
        .await
        .expect("migrate");
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
    async fn deliver(&self, _event: &Delivery) -> Result<(), Failure> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.fail {
            Err(Failure::rejected("sink-down"))
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

/// The service's settings, with waits short enough for a test.
fn quick() -> OutboxOptions {
    OutboxOptions {
        poll_interval: Duration::from_millis(20),
        retry_backoff: Duration::from_millis(10),
        hold_interval: Duration::from_millis(50),
        ..outbox::options(Duration::from_millis(20))
    }
}

/// A relay over `sinks`, running until it is stopped.
struct Relay {
    stop: watch::Sender<bool>,
    task: tokio::task::JoinHandle<()>,
}

impl Relay {
    async fn start(sinks: Sinks) -> Relay {
        let outbox = Outbox::connect_existing(&url(), quick())
            .await
            .expect("the outbox tables, indexes and triggers are there");
        let (stop, stopped) = watch::channel(false);
        let task = tokio::spawn(outbox::run(outbox, sinks, stopped));
        Relay { stop, task }
    }

    async fn stop(self) {
        self.stop.send(true).unwrap();
        tokio::time::timeout(Duration::from_secs(10), self.task)
            .await
            .expect("the relay stops")
            .unwrap();
    }
}

/// Wait up to ten seconds for `n` attempts of `outcome` on `id`, and say how many there are.
async fn wait_for(db: &PgPool, id: &str, outcome: &str, n: i64) -> i64 {
    for _ in 0..200 {
        if attempts(db, id, outcome).await >= n {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    attempts(db, id, outcome).await
}

/// Let a relay run a while longer, for what it should not do.
async fn settle() {
    tokio::time::sleep(Duration::from_millis(400)).await;
}

async fn patient(db: &PgPool) -> String {
    let p = Patient {
        names: vec![HumanName {
            family: Some(format!("Zz{}", nanos())),
            given: vec![],
        }],
        ..Default::default()
    };
    patients_store::create(db, "t", None, &p).await.unwrap().id
}

async fn count(db: &PgPool, sql: &str, arg: &str) -> i64 {
    sqlx::query(AssertSqlSafe(sql.to_string()))
        .bind(arg)
        .fetch_optional(db)
        .await
        .unwrap()
        .unwrap()
        .try_get("n")
        .unwrap()
}

async fn attempts(db: &PgPool, id: &str, outcome: &str) -> i64 {
    sqlx::query(
        "SELECT count(*) AS n FROM axa_outbox_attempts WHERE event_id = $1::uuid AND outcome = $2",
    )
    .bind(id)
    .bind(outcome)
    .fetch_optional(db)
    .await
    .unwrap()
    .unwrap()
    .try_get("n")
    .unwrap()
}

async fn audited(db: &PgPool, id: &str, outcome: i16) -> i64 {
    sqlx::query("SELECT count(*) AS n FROM audit_events WHERE resource_ref = $1 AND outcome = $2")
        .bind(format!("outbox/{id}"))
        .bind(outcome)
        .fetch_optional(db)
        .await
        .unwrap()
        .unwrap()
        .try_get("n")
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
        names_suspicion_of: None,
    }
}

async fn enqueue(db: &PgPool, new: NewEvent<'_>) -> String {
    let mut tx = db.begin().await.unwrap();
    let id = outbox::enqueue(&mut tx, new).await.unwrap();
    tx.commit().await.unwrap();
    id
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_rolled_back_event_is_never_delivered_and_a_committed_one_is_delivered_once() {
    let db = db().await;
    let recipient = format!("sink-{}", nanos());
    let payload = serde_json::json!({"n": 1});

    let mut tx = db.begin().await.unwrap();
    let lost = outbox::enqueue(&mut tx, event(&recipient, None, &payload))
        .await
        .unwrap();
    tx.rollback().await.unwrap();
    let kept = enqueue(&db, event(&recipient, None, &payload)).await;

    let (all, calls) = sinks(&recipient, false);
    let before = metrics::OUTBOX_DELIVERED.get();
    let relay = Relay::start(all).await;
    assert_eq!(wait_for(&db, &kept, "delivered", 1).await, 1);
    settle().await;
    relay.stop().await;

    assert_eq!(calls.load(Ordering::SeqCst), 1, "delivered exactly once");
    assert!(metrics::OUTBOX_DELIVERED.get() > before, "counted");
    assert_eq!(
        count(
            &db,
            "SELECT count(*) AS n FROM axa_outbox_events WHERE event_id = $1::uuid",
            &lost
        )
        .await,
        0,
        "the rolled back event does not exist"
    );
    assert_eq!(audited(&db, &kept, 0).await, 1, "the delivery is audited");
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
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
    let id = enqueue(&db, event(&recipient, Some(&p), &payload)).await;

    let (all, calls) = sinks(&recipient, false);
    let relay = Relay::start(all).await;
    assert_eq!(wait_for(&db, &id, "refused", 1).await, 1);
    settle().await;
    relay.stop().await;

    assert_eq!(calls.load(Ordering::SeqCst), 0, "the sink was never called");
    assert_eq!(
        attempts(&db, &id, "refused").await,
        1,
        "refused once, final"
    );
    assert_eq!(attempts(&db, &id, "delivered").await, 0);
    assert_eq!(audited(&db, &id, 4).await, 1, "the refusal is audited");
}

/// BV-3: any report holds an event, whoever made it; a person confirming it keeps the hold; a
/// withdrawal releases it, and the event is then delivered once. Another patient's event goes.
#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_report_of_death_holds_an_event_until_it_is_withdrawn() {
    let db = db().await;
    let recipient = format!("sink-{}", nanos());
    let id = patient(&db).await;
    let other = patient(&db).await;
    let payload = serde_json::json!({"n": 1});

    let made = death::report(
        Source::OtherSystem,
        Actor::Feed("mllp:adt-feed".into()),
        Occurred::NotKnown {
            reason: "the message gave no date".into(),
        },
        "HL7 ADT A08 C1",
        chrono::Utc::now(),
    )
    .unwrap();
    let stored = death_store::report(&db, &id, &made).await.unwrap();
    let held = enqueue(&db, event(&recipient, Some(&id), &payload)).await;
    let free = enqueue(&db, event(&recipient, Some(&other), &payload)).await;

    let (all, calls) = sinks(&recipient, false);
    let relay = Relay::start(all).await;
    assert_eq!(
        wait_for(&db, &free, "delivered", 1).await,
        1,
        "the other patient's goes"
    );
    settle().await;
    assert_eq!(calls.load(Ordering::SeqCst), 1, "only the other patient's");
    let any = "SELECT count(*) AS n FROM axa_outbox_attempts WHERE event_id = $1::uuid";
    assert_eq!(
        count(&db, any, &held).await,
        0,
        "held: no attempt, not refused"
    );

    // Confirmed, it is still held.
    let by = Actor::Person("nurse-1".into());
    let step = |to, reason: &str| Step {
        to,
        by: by.clone(),
        reason: reason.into(),
    };
    death_store::advance(
        &db,
        &id,
        &stored.id,
        1,
        &step(DeathStatus::Verified, "seen"),
    )
    .await
    .unwrap();
    settle().await;
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(count(&db, any, &held).await, 0);

    // Withdrawn, it goes, once.
    let wrong = step(DeathStatus::WithdrawnInError, "the wrong record");
    death_store::advance(&db, &id, &stored.id, 2, &wrong)
        .await
        .unwrap();
    assert_eq!(wait_for(&db, &held, "delivered", 1).await, 1);
    settle().await;
    relay.stop().await;
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_recipient_the_policy_does_not_allow_gets_nothing() {
    let db = db().await;
    let recipient = format!("sink-{}", nanos());
    let payload = serde_json::json!({"n": 3});
    let mut e = event(&recipient, None, &payload);
    e.roles = &[Role::Integrator];
    e.resource = Resource::Observation;
    let id = enqueue(&db, e).await;

    let (all, calls) = sinks(&recipient, false);
    let relay = Relay::start(all).await;
    assert_eq!(wait_for(&db, &id, "refused", 1).await, 1);
    relay.stop().await;
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_failing_sink_is_tried_a_fixed_number_of_times_then_left_for_a_person() {
    let db = db().await;
    let recipient = format!("sink-{}", nanos());
    let payload = serde_json::json!({"n": 4});
    let id = enqueue(&db, event(&recipient, None, &payload)).await;

    let (all, calls) = sinks(&recipient, true);
    let relay = Relay::start(all).await;
    assert_eq!(wait_for(&db, &id, "failed", 1).await, 1);
    settle().await;
    relay.stop().await;

    assert_eq!(calls.load(Ordering::SeqCst), MAX_ATTEMPTS.unsigned_abs());
    assert_eq!(
        attempts(&db, &id, "rejected").await,
        i64::from(MAX_ATTEMPTS - 1)
    );
    assert_eq!(attempts(&db, &id, "delivered").await, 0);
}

/// Not lost: an event for a sink that does not exist yet fails, and a person can put it back once it
/// does, with who and why on record.
#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn an_event_for_a_recipient_with_no_sink_is_not_lost() {
    let db = db().await;
    let recipient = format!("nobody-{}", nanos());
    let payload = serde_json::json!({"n": 5});
    let id = enqueue(&db, event(&recipient, None, &payload)).await;

    let relay = Relay::start(Sinks::new()).await;
    assert_eq!(wait_for(&db, &id, "failed", 1).await, 1);
    relay.stop().await;
    assert_eq!(attempts(&db, &id, "delivered").await, 0);

    let mut tx = db.begin().await.unwrap();
    let event_id = uuid::Uuid::parse_str(&id).unwrap();
    assert!(
        axa::outbox::release(&mut tx, event_id, "admin-1", "sink-added")
            .await
            .unwrap()
    );
    tx.commit().await.unwrap();
    let (all, calls) = sinks(&recipient, false);
    let relay = Relay::start(all).await;
    assert_eq!(wait_for(&db, &id, "delivered", 1).await, 1);
    relay.stop().await;
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let by: String = sqlx::query_scalar(
        "SELECT by FROM axa_outbox_attempts WHERE event_id = $1 AND outcome = 'released'",
    )
    .bind(event_id)
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(by, "admin-1");
}

/// An event whose stored payload was changed, by someone who turned the guard off to do it, is failed
/// as `tampered`, is never published, and is an audit row.
#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_changed_event_is_never_published() {
    let db = db().await;
    let recipient = format!("sink-{}", nanos());
    let payload = serde_json::json!({"n": 8});
    let id = enqueue(&db, event(&recipient, None, &payload)).await;
    // Wider roles, written behind the guard's back.
    sqlx::raw_sql(AssertSqlSafe(format!(
        "BEGIN;
         ALTER TABLE axa_outbox_payloads DISABLE TRIGGER axa_outbox_payloads_guard;
         UPDATE axa_outbox_payloads SET payload = jsonb_set(payload, '{{route,roles}}', '[\"admin\"]')
           WHERE event_id = '{id}';
         ALTER TABLE axa_outbox_payloads ENABLE ALWAYS TRIGGER axa_outbox_payloads_guard;
         COMMIT;"
    )))
    .execute(&db)
    .await
    .unwrap();

    let (all, calls) = sinks(&recipient, false);
    let relay = Relay::start(all).await;
    assert_eq!(wait_for(&db, &id, "failed", 1).await, 1);
    settle().await;
    relay.stop().await;
    assert_eq!(calls.load(Ordering::SeqCst), 0, "never published");
    let reason: String = sqlx::query_scalar(
        "SELECT reason FROM axa_outbox_attempts WHERE event_id = $1::uuid AND outcome = 'failed'",
    )
    .bind(&id)
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(reason, "tampered");
    assert_eq!(
        audited(&db, &id, 8).await,
        1,
        "the changed event is audited"
    );
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn an_oversized_payload_is_refused_with_its_transaction() {
    let db = db().await;
    let payload = serde_json::json!({ "text": "x".repeat(70 * 1024) });
    let mut tx = db.begin().await.unwrap();
    let refused = outbox::enqueue(&mut tx, event("sink", None, &payload)).await;
    assert!(refused.is_err(), "{refused:?}");
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn the_record_cannot_be_edited_deleted_or_truncated_and_the_old_tables_are_closed() {
    let db = db().await;
    let recipient = format!("sink-{}", nanos());
    let payload = serde_json::json!({"n": 6});
    let id = enqueue(&db, event(&recipient, None, &payload)).await;
    let (all, _) = sinks(&recipient, false);
    let relay = Relay::start(all).await;
    assert_eq!(wait_for(&db, &id, "delivered", 1).await, 1);
    relay.stop().await;

    for sql in [
        format!("UPDATE axa_outbox_events SET kind = 'x' WHERE event_id = '{id}'"),
        format!("DELETE FROM axa_outbox_events WHERE event_id = '{id}'"),
        format!("UPDATE axa_outbox_attempts SET outcome = 'failed' WHERE event_id = '{id}'"),
        format!("DELETE FROM axa_outbox_attempts WHERE event_id = '{id}'"),
        format!("UPDATE axa_outbox_payloads SET payload = '{{}}' WHERE event_id = '{id}'"),
        format!("DELETE FROM axa_outbox_payloads WHERE event_id = '{id}'"),
        "TRUNCATE axa_outbox_attempts".to_string(),
        "TRUNCATE axa_outbox_events CASCADE".to_string(),
        "TRUNCATE axa_outbox_payloads".to_string(),
    ] {
        let e = sqlx::raw_sql(AssertSqlSafe(sql.clone()))
            .execute(&db)
            .await
            .unwrap_err();
        assert!(e.to_string().contains("refused"), "{sql}: {e}");
    }
    for sql in [
        "INSERT INTO outbox_events (id, kind, recipient, recipient_roles, purpose, resource, payload)
           VALUES (gen_random_uuid(), 'k', 'r', '{}', 'operations', 'PatientIdentity', '{}')",
    ] {
        let e = sqlx::raw_sql(AssertSqlSafe(sql.to_string()))
            .execute(&db)
            .await
            .unwrap_err();
        assert!(e.to_string().contains("closed"), "{sql}: {e}");
    }
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn depth_counts_owed_and_stuck_events() {
    let db = db().await;
    let recipient = format!("nobody-{}", nanos());
    let payload = serde_json::json!({"n": 9});
    let outbox = Outbox::connect_existing(&url(), quick()).await.unwrap();
    let (pending_before, _) = outbox::depth(&outbox).await.unwrap();
    let id = enqueue(&db, event(&recipient, None, &payload)).await;
    let (pending, _) = outbox::depth(&outbox).await.unwrap();
    assert!(pending > pending_before, "a new event is owed");

    let relay = Relay::start(Sinks::new()).await;
    assert_eq!(wait_for(&db, &id, "failed", 1).await, 1);
    relay.stop().await;
    let (_, stuck) = outbox::depth(&outbox).await.unwrap();
    assert!(
        stuck >= 1,
        "a failed event is stuck until a person releases it"
    );
}

/// A sink that says when a publish has begun, then waits to be let go (or for ever).
struct Gated {
    entered: Arc<tokio::sync::Notify>,
    go: Arc<tokio::sync::Notify>,
    calls: Arc<AtomicU32>,
}

#[async_trait]
impl Sink for Gated {
    async fn deliver(&self, _event: &Delivery) -> Result<(), Failure> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.entered.notify_one();
        self.go.notified().await;
        Ok(())
    }
}

fn gated(
    name: &str,
) -> (
    Sinks,
    Arc<tokio::sync::Notify>,
    Arc<tokio::sync::Notify>,
    Arc<AtomicU32>,
) {
    let entered = Arc::new(tokio::sync::Notify::new());
    let go = Arc::new(tokio::sync::Notify::new());
    let calls = Arc::new(AtomicU32::new(0));
    let mut all = Sinks::new();
    all.insert(
        name.to_string(),
        Box::new(Gated {
            entered: entered.clone(),
            go: go.clone(),
            calls: calls.clone(),
        }),
    );
    (all, entered, go, calls)
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_stop_lets_the_publish_in_progress_finish_and_sends_nothing_more() {
    let db = db().await;
    let recipient = format!("gated-{}", nanos());
    let payload = serde_json::json!({"n": 10});
    let first = enqueue(&db, event(&recipient, None, &payload)).await;
    let second = enqueue(&db, event(&recipient, None, &payload)).await;
    let (all, entered, go, calls) = gated(&recipient);
    let outbox = Outbox::connect_existing(&url(), quick()).await.unwrap();
    let running = outbox::Running::start(outbox, all, Duration::from_secs(60));

    tokio::time::timeout(Duration::from_secs(10), entered.notified())
        .await
        .expect("a publish begins");
    let stopping = tokio::spawn(running.stop(outbox::STOP_WAIT));
    settle().await;
    assert!(!stopping.is_finished(), "the stop waits for the publish");
    go.notify_one();
    assert!(stopping.await.unwrap(), "the relay stopped in time");

    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "nothing is sent after the stop"
    );
    let delivered =
        attempts(&db, &first, "delivered").await + attempts(&db, &second, "delivered").await;
    assert_eq!(delivered, 1, "the publish in progress is recorded");
    let charged = attempts(&db, &first, "rejected").await
        + attempts(&db, &second, "rejected").await
        + attempts(&db, &first, "failed").await
        + attempts(&db, &second, "failed").await;
    assert_eq!(charged, 0, "the stop charges nothing");
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_relay_that_does_not_stop_in_time_is_abandoned_and_says_so() {
    let db = db().await;
    let recipient = format!("stuck-{}", nanos());
    let payload = serde_json::json!({"n": 11});
    let id = enqueue(&db, event(&recipient, None, &payload)).await;
    let (all, entered, _go, _calls) = gated(&recipient);
    let outbox = Outbox::connect_existing(&url(), quick()).await.unwrap();
    let running = outbox::Running::start(outbox, all, Duration::from_secs(60));

    tokio::time::timeout(Duration::from_secs(10), entered.notified())
        .await
        .expect("a publish begins");
    let waited = std::time::Instant::now();
    assert!(
        !running.stop(Duration::from_millis(300)).await,
        "not stopped"
    );
    assert!(
        waited.elapsed() < Duration::from_secs(5),
        "the wait is bounded"
    );
    assert_eq!(
        attempts(&db, &id, "delivered").await,
        0,
        "nothing is recorded as sent"
    );
}

#[tokio::test]
async fn with_no_relay_kept_a_stop_is_immediate() {
    assert!(outbox::stop_kept().await);
}

/// The reasons the relay gave for refusing an event.
async fn refusal_reasons(db: &PgPool, id: &str) -> Vec<String> {
    sqlx::query(
        "SELECT reason FROM axa_outbox_attempts WHERE event_id = $1::uuid AND outcome = 'refused'",
    )
    .bind(id)
    .fetch_all(db)
    .await
    .unwrap()
    .iter()
    .map(|r| {
        r.try_get::<Option<String>, _>("reason")
            .unwrap()
            .unwrap_or_default()
    })
    .collect()
}

/// RS-4, at the outbound gate: a message to the patient that names the suspected condition is refused
/// while the suspicion says `not-told`, and for another patient's referral; once the record says the
/// patient was told it passes this check (and meets the read policy, which today refuses every patient
/// recipient because the relay holds no relationship).
#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_message_naming_an_untold_suspicion_never_reaches_the_patient() {
    let db = db().await;
    let recipient = format!("sink-{}", nanos());
    let (p, other) = (patient(&db).await, patient(&db).await);
    let now = chrono::Utc::now();
    let referral = referrals_store::request(
        &db,
        "t",
        &p,
        &Referral {
            recipient: "Colorectal clinic".into(),
            reason: "bleeding".into(),
            question: "please assess".into(),
            priority: Priority::Urgent,
            external_ids: vec![],
            replaces: None,
        },
        now,
    )
    .await
    .unwrap();
    let mut s = Suspicion {
        condition: "Suspected lower gastrointestinal cancer".into(),
        code: None,
        subtype: None,
        point_of_suspicion: "2026-11-09T09:30:00Z".into(),
        source: Raised::Clinician,
        by: "Dr A".into(),
        grounds: Grounds {
            kind: GroundsKind::Criterion,
            detail: "NG12".into(),
            result_id: None,
        },
        instruction: None,
        patient_told: PatientTold::NotTold,
        pathway: "suspected-cancer-wales".into(),
        pathway_version: 1,
        band: None,
        factors: vec![],
        agreed: None,
    };
    suspicions_store::raise(&db, &p, &referral.id, "t", &s, now)
        .await
        .unwrap();
    let letter =
        serde_json::json!({"letter": "You have been referred because cancer is suspected."});
    let send = |patient: String| {
        let db = db.clone();
        let recipient = recipient.clone();
        let letter = letter.clone();
        let referral = referral.id.clone();
        async move {
            enqueue(
                &db,
                NewEvent {
                    kind: "referral-letter",
                    recipient: &recipient,
                    roles: &[Role::Patient],
                    purpose: Purpose::PatientAccess,
                    resource: Resource::Referral,
                    patient_id: Some(&patient),
                    payload: &letter,
                    names_suspicion_of: Some(&referral),
                },
            )
            .await
        }
    };
    let untold = send(p.clone()).await;
    let elsewhere = send(other.clone()).await;
    let (all, calls) = sinks(&recipient, false);
    let relay = Relay::start(all).await;
    for id in [&untold, &elsewhere] {
        assert_eq!(wait_for(&db, id, "refused", 1).await, 1, "{id}");
    }
    settle().await;
    relay.stop().await;
    assert_eq!(refusal_reasons(&db, &untold).await, ["suspicion-not-told"]);
    assert_eq!(
        refusal_reasons(&db, &elsewhere).await,
        ["suspicion-not-told"]
    );

    // The gate reads the record when the event is sent: once it says the patient was told, the
    // suspicion no longer stops the message.
    s.patient_told = PatientTold::Told;
    suspicions_store::amend(
        &db,
        &p,
        &referral.id,
        "t",
        1,
        Some("discussed in clinic"),
        &s,
        now,
    )
    .await
    .unwrap();
    let told = send(p.clone()).await;
    let (all, more) = sinks(&recipient, false);
    let relay = Relay::start(all).await;
    assert_eq!(wait_for(&db, &told, "refused", 1).await, 1);
    settle().await;
    relay.stop().await;
    assert_eq!(
        calls.load(Ordering::SeqCst) + more.load(Ordering::SeqCst),
        0
    );
    let told_reasons = refusal_reasons(&db, &told).await;
    assert_eq!(told_reasons.len(), 1);
    assert_ne!(
        told_reasons[0], "suspicion-not-told",
        "told: it passes the suspicion check"
    );
}
