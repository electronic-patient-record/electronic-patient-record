//! DB-gated (hazard H-044): the referrer's worklist. A referral the recipient has answered in a way the
//! referrer did not ask for waits on the sender's list until that version is acknowledged.
//!
//! Covered: a referral that is moving as asked (requested, accepted) is not listed; a request for information
//! is, with the version that waits; the list and the count are the sender's alone; an acknowledgement needs the
//! current version (409 otherwise and when repeated), from the sender (404 from anyone else), in a state that
//! waits (422 otherwise); an acknowledged version leaves the list and a later version that waits comes back;
//! the list carries no clinical text; the table is append-only.
//! Not covered: routes and authorisation (`http_referral_worklist.rs`), concurrent acknowledgements, and a
//! worklist for a recipient or a team (not built).
//!
//! ```text
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test referral_worklist -- --ignored --test-threads=1
//! ```

use chrono::{Duration, Utc};
use electronic_patient_record::clock::{Clock, FixedClock};
use electronic_patient_record::patients_store::{self, StoreError};
use electronic_patient_record::referral::{Actor, Event, State};
use electronic_patient_record::referral_timeouts::Timeouts;
use electronic_patient_record::referral_worklist::{self as wl, Why};
use electronic_patient_record::referrals_store::{self as store, Priority, Referral, Step};
use electronic_patient_record_matcher::{HumanName, Patient};
use electronic_patient_record_migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection};

async fn db() -> DatabaseConnection {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for --ignored tests");
    let db = Database::connect(url).await.expect("connect");
    Migrator::up(&db, None).await.expect("migrate");
    db
}

async fn referral(db: &DatabaseConnection) -> store::StoredReferral {
    referral_by(db, "dr-1").await
}

/// A referral sent by `author`.
async fn referral_by(db: &DatabaseConnection, author: &str) -> store::StoredReferral {
    let family = format!(
        "wl{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let patient = patients_store::create(
        db,
        author,
        Some("referral worklist test"),
        &Patient {
            names: vec![HumanName {
                family: Some(family.clone()),
                given: vec![family],
            }],
            birth_date: Some("1950-01-01".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    store::request(
        db,
        author,
        &patient.id,
        &Referral {
            recipient: "Cardiology outpatients".into(),
            reason: "Exertional chest pain".into(),
            question: "Is this angina?".into(),
            priority: Priority::Routine,
            external_ids: vec![],
        },
        Utc::now(),
    )
    .await
    .unwrap()
}

async fn step(
    db: &DatabaseConnection,
    id: &str,
    version: i32,
    actor: Actor,
    by: &str,
    event: Event,
    note: Option<&str>,
) -> store::StoredReferral {
    store::advance(
        db,
        id,
        version,
        Step {
            actor,
            by,
            event,
            note,
        },
        Utc::now(),
    )
    .await
    .unwrap()
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_referral_waits_for_its_sender_until_that_version_is_acknowledged() {
    let db = db().await;
    let r = referral(&db).await;
    let ids = |page: &electronic_patient_record::page::Page<wl::Item>| -> Vec<String> {
        page.iter().map(|i| i.referral_id.clone()).collect()
    };

    // Moving as asked: not listed.
    assert!(!ids(&wl::list(&db, "dr-1").await.unwrap()).contains(&r.id));
    let accepted = step(
        &db,
        &r.id,
        1,
        Actor::Recipient,
        "cardio-1",
        Event::Accept,
        None,
    )
    .await;
    assert_eq!(accepted.state, State::Accepted);
    assert!(!ids(&wl::list(&db, "dr-1").await.unwrap()).contains(&r.id));

    // A decline waits; the list holds the version and state, not the text.
    let before = wl::count(&db, "dr-1").await.unwrap();
    let declined = step(
        &db,
        &r.id,
        2,
        Actor::Recipient,
        "cardio-1",
        Event::Decline,
        Some("Secret-note-heron"),
    )
    .await;
    assert_eq!(declined.version, 3);
    let list = wl::list(&db, "dr-1").await.unwrap();
    let item = list.iter().find(|i| i.referral_id == r.id).expect("listed");
    assert_eq!((item.version, item.state), (3, State::Declined));
    assert_eq!(item.patient_id, r.patient_id);
    assert!(
        !serde_json::to_string(item)
            .unwrap()
            .contains("Secret-note-heron")
    );
    assert_eq!(wl::count(&db, "dr-1").await.unwrap(), before + 1);

    // It is the sender's alone.
    assert!(!ids(&wl::list(&db, "someone-else").await.unwrap()).contains(&r.id));
    assert!(matches!(
        wl::acknowledge(&db, &r.id, 3, "someone-else").await,
        Err(StoreError::NotFound)
    ));
    assert!(matches!(
        wl::acknowledge(&db, "not-a-uuid", 3, "dr-1").await,
        Err(StoreError::NotFound)
    ));

    // Only the current version, once.
    assert!(matches!(
        wl::acknowledge(&db, &r.id, 2, "dr-1").await,
        Err(StoreError::Conflict { current: 3 })
    ));
    wl::acknowledge(&db, &r.id, 3, "dr-1").await.unwrap();
    assert!(matches!(
        wl::acknowledge(&db, &r.id, 3, "dr-1").await,
        Err(StoreError::Conflict { .. })
    ));
    assert!(!ids(&wl::list(&db, "dr-1").await.unwrap()).contains(&r.id));
    assert_eq!(wl::count(&db, "dr-1").await.unwrap(), before);
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_later_version_that_waits_comes_back_and_a_state_that_does_not_wait_cannot_be_acknowledged()
 {
    let db = db().await;
    let r = referral(&db).await;
    // Version 1 (requested) is not a state that waits.
    assert!(matches!(
        wl::acknowledge(&db, &r.id, 1, "dr-1").await,
        Err(StoreError::BadQuery(_))
    ));

    let asked = step(
        &db,
        &r.id,
        1,
        Actor::Recipient,
        "cardio-1",
        Event::RequestInformation,
        Some("which side?"),
    )
    .await;
    assert_eq!(asked.state, State::InformationRequested);
    assert!(
        wl::list(&db, "dr-1")
            .await
            .unwrap()
            .iter()
            .any(|i| i.referral_id == r.id && i.version == 2)
    );
    wl::acknowledge(&db, &r.id, 2, "dr-1").await.unwrap();

    let answered = step(
        &db,
        &r.id,
        2,
        Actor::Referrer,
        "dr-1",
        Event::ProvideInformation,
        Some("left"),
    )
    .await;
    assert_eq!(answered.state, State::Requested);
    assert!(
        !wl::list(&db, "dr-1")
            .await
            .unwrap()
            .iter()
            .any(|i| i.referral_id == r.id)
    );

    // The recipient asks again: version 4 is a new item although version 2 was acknowledged.
    step(
        &db,
        &r.id,
        3,
        Actor::Recipient,
        "cardio-1",
        Event::RequestInformation,
        Some("since when?"),
    )
    .await;
    assert!(
        wl::list(&db, "dr-1")
            .await
            .unwrap()
            .iter()
            .any(|i| i.referral_id == r.id && i.version == 4)
    );
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn the_database_refuses_to_edit_delete_or_truncate_an_acknowledgement() {
    let db = db().await;
    let r = referral(&db).await;
    step(
        &db,
        &r.id,
        1,
        Actor::Recipient,
        "cardio-1",
        Event::RequestInformation,
        Some("which side?"),
    )
    .await;
    wl::acknowledge(&db, &r.id, 2, "dr-1").await.unwrap();
    for sql in [
        format!(
            "UPDATE referral_acknowledgements SET acknowledged_by = 'x' WHERE referral_id = '{}'",
            r.id
        ),
        format!(
            "DELETE FROM referral_acknowledgements WHERE referral_id = '{}'",
            r.id
        ),
        "TRUNCATE referral_acknowledgements".to_string(),
    ] {
        let e = db.execute_unprepared(&sql).await.unwrap_err();
        assert!(e.to_string().contains("append-only"), "{sql}: {e}");
    }
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_referral_that_has_gone_quiet_is_listed_only_for_a_priority_with_a_period() {
    // PC-3, hazard H-044. Priority is `routine` in the helper's referral.
    let db = db().await;
    let sender = format!(
        "dr-clock-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let r = referral_by(&db, &sender).await;
    // The clock starts at the real time and is moved; no row is rewritten (PR-5).
    let clock = FixedClock::new(Utc::now());
    let ids = |page: &electronic_patient_record::page::Page<wl::Item>| -> Vec<(String, Why)> {
        page.iter()
            .map(|i| (i.referral_id.clone(), i.why))
            .collect()
    };
    let none = Timeouts::default();
    let routine_2 = Timeouts::parse(Some(
        r#"{"no_response_days":{"routine":2},"no_outcome_days":{"routine":5}}"#,
    ))
    .unwrap();
    let urgent_only = Timeouts::parse(Some(r#"{"no_response_days":{"urgent":1}}"#)).unwrap();

    // New: not quiet yet.
    assert!(
        !ids(&wl::list_at(&db, &sender, &routine_2, clock.now())
            .await
            .unwrap())
        .iter()
        .any(|(i, _)| *i == r.id)
    );
    // Old and unanswered: listed with a reason, but only where a period is set for its priority.
    clock.advance(Duration::days(3));
    assert!(
        ids(&wl::list_at(&db, &sender, &routine_2, clock.now())
            .await
            .unwrap())
        .contains(&(r.id.clone(), Why::NoResponse))
    );
    assert!(
        !ids(&wl::list_at(&db, &sender, &none, clock.now()).await.unwrap())
            .iter()
            .any(|(i, _)| *i == r.id)
    );
    assert!(
        !ids(&wl::list_at(&db, &sender, &urgent_only, clock.now())
            .await
            .unwrap())
        .iter()
        .any(|(i, _)| *i == r.id)
    );
    let before = wl::count_at(&db, &sender, &none, clock.now())
        .await
        .unwrap();
    assert_eq!(
        wl::count_at(&db, &sender, &routine_2, clock.now())
            .await
            .unwrap(),
        before + 1
    );
    // It is the sender's alone, and a timeout cannot be acknowledged.
    assert!(
        !ids(&wl::list_at(&db, "someone-else", &routine_2, clock.now())
            .await
            .unwrap())
        .iter()
        .any(|(i, _)| *i == r.id)
    );
    assert!(matches!(
        wl::acknowledge(&db, &r.id, 1, &sender).await,
        Err(StoreError::BadQuery(_))
    ));

    // Accepted: the clock restarts and the reason changes to no outcome once that is old too.
    step(
        &db,
        &r.id,
        1,
        Actor::Recipient,
        "cardio-1",
        Event::Accept,
        None,
    )
    .await;
    assert!(
        !ids(&wl::list_at(&db, &sender, &routine_2, clock.now())
            .await
            .unwrap())
        .iter()
        .any(|(i, _)| *i == r.id)
    );
    clock.advance(Duration::days(6));
    assert!(
        ids(&wl::list_at(&db, &sender, &routine_2, clock.now())
            .await
            .unwrap())
        .contains(&(r.id.clone(), Why::NoOutcome))
    );

    // An outcome ends it: a completed referral waits as a reply, not a timeout.
    step(
        &db,
        &r.id,
        2,
        Actor::Recipient,
        "cardio-1",
        Event::Outcome,
        Some("seen"),
    )
    .await;
    assert!(
        ids(&wl::list_at(&db, &sender, &routine_2, clock.now())
            .await
            .unwrap())
        .contains(&(r.id.clone(), Why::Reply))
    );
}
