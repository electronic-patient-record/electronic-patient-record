//! DB-gated (PC-2, hazard H-044): an inbound referral message that does not match is kept for a person and
//! never dropped; one that matches moves the referral; a person clears an entry once; the tables are
//! append-only.
//!
//! Covered: an outcome with a known id moves the referral to `completed`; no id, an unknown id, a mistyped
//! id, an outcome after `cancelled`, a referrer's step sent inbound, an unknown event and a missing note
//! each queue with a reason and leave the referral as it was; a listing carries no message text; a hand
//! match records both ids and changes nothing in the referral, a dismissal needs a reason, an entry clears
//! once and a hand match to a referral that does not exist is refused; the triggers on both tables.
//! Not covered: routes, authorisation, audit rows, the worklist count shown to a team (PC-3), concurrent
//! writers, a referral id mistyped as another real one (no software can tell).
//!
//! ```text
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test referral_unmatched -- --ignored --test-threads=1
//! ```

use chrono::Utc;
use electronic_patient_record::patients_store::{self, StoreError};
use electronic_patient_record::referral::{Actor, Event, State};
use electronic_patient_record::referral_unmatched::{self as queue, Clearance, Filed, Inbound};
use electronic_patient_record::referrals_store::{self as store, Priority, Referral, Step};
use electronic_patient_record_matcher::{HumanName, Patient};
use electronic_patient_record_migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection};

const SECRET: &str = "Secret-body-quokka";

async fn db() -> DatabaseConnection {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for --ignored tests");
    let db = Database::connect(url).await.expect("connect");
    Migrator::up(&db, None).await.expect("migrate");
    db
}

async fn referral(db: &DatabaseConnection) -> store::StoredReferral {
    let family = format!(
        "unm{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let patient = patients_store::create(
        db,
        "dr-1",
        Some("referral unmatched test"),
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
        "dr-1",
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

fn inbound<'a>(
    reference: Option<&'a str>,
    event: Option<&'a str>,
    note: Option<&'a str>,
) -> Inbound<'a> {
    Inbound {
        agent: "clinic-feed",
        referral_ref: reference,
        event,
        note,
        message: SECRET,
    }
}

fn queued(filed: Filed) -> queue::Entry {
    match filed {
        Filed::Queued { entry } => *entry,
        other @ Filed::Moved { .. } => panic!("expected the queue, got {other:?}"),
    }
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_message_with_a_known_id_moves_the_referral() {
    let db = db().await;
    let r = referral(&db).await;
    let accepted = store::advance(
        &db,
        &r.id,
        1,
        Step {
            actor: Actor::Recipient,
            by: "clinic-feed",
            event: Event::Accept,
            note: None,
        },
        Utc::now(),
    )
    .await
    .unwrap();
    let open_before = queue::count_open(&db).await.unwrap();
    let filed = queue::file(
        &db,
        inbound(
            Some(&accepted.id),
            Some("outcome"),
            Some("Seen; not angina"),
        ),
        Utc::now(),
    )
    .await
    .unwrap();
    let Filed::Moved { referral } = filed else {
        panic!("expected the referral to move");
    };
    assert_eq!((referral.state, referral.version), (State::Completed, 3));
    assert_eq!(referral.last_event["by"], "clinic-feed");
    assert_eq!(
        queue::count_open(&db).await.unwrap(),
        open_before,
        "nothing was queued"
    );
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_message_that_does_not_match_is_queued_with_a_reason_and_changes_nothing() {
    let db = db().await;
    let r = referral(&db).await;
    let cancelled = {
        let asked = store::advance(
            &db,
            &r.id,
            1,
            Step {
                actor: Actor::Referrer,
                by: "dr-1",
                event: Event::RequestCancel,
                note: Some("not needed"),
            },
            Utc::now(),
        )
        .await
        .unwrap();
        store::advance(
            &db,
            &asked.id,
            2,
            Step {
                actor: Actor::Recipient,
                by: "clinic-feed",
                event: Event::ConfirmCancel,
                note: None,
            },
            Utc::now(),
        )
        .await
        .unwrap()
    };
    assert_eq!(cancelled.state, State::Cancelled);
    let live = referral(&db).await;
    let unknown = "11111111-1111-4111-8111-111111111111";

    let cases: [(Inbound<'_>, &str); 7] = [
        (inbound(None, Some("outcome"), Some("x")), "no-id"),
        (inbound(Some("  "), Some("outcome"), Some("x")), "no-id"),
        (
            inbound(Some(unknown), Some("outcome"), Some("x")),
            "unknown-id",
        ),
        (
            inbound(Some("not-a-uuid"), Some("outcome"), Some("x")),
            "unknown-id",
        ),
        // After the end state, the table refuses.
        (
            inbound(Some(&cancelled.id), Some("outcome"), Some("x")),
            "refused-by-table",
        ),
        // The referrer's step, an unknown event and a missing note.
        (
            inbound(Some(&live.id), Some("request-cancel"), Some("x")),
            "refused-by-table",
        ),
        (
            inbound(Some(&live.id), Some("approve"), Some("x")),
            "refused-by-table",
        ),
    ];
    let mut mine = Vec::new();
    for (message, kind) in cases {
        let entry = queued(queue::file(&db, message, Utc::now()).await.unwrap());
        mine.push(entry.id.clone());
        assert_eq!(entry.kind, kind, "{message:?}");
        assert_eq!(entry.reasons.len(), 1);
        assert!(entry.cleared.is_none());
        assert!(
            entry.message.is_none(),
            "a queued answer carries no message text"
        );
    }
    // A step that needs a note and has none is queued too, and says why.
    let accepted = store::advance(
        &db,
        &live.id,
        1,
        Step {
            actor: Actor::Recipient,
            by: "clinic-feed",
            event: Event::Accept,
            note: None,
        },
        Utc::now(),
    )
    .await
    .unwrap();
    let entry = queued(
        queue::file(
            &db,
            inbound(Some(&accepted.id), Some("outcome"), None),
            Utc::now(),
        )
        .await
        .unwrap(),
    );
    assert!(
        entry.reasons[0].contains("needs a note"),
        "{:?}",
        entry.reasons
    );

    // The referrals did not move.
    assert_eq!(
        store::current(&db, &cancelled.id).await.unwrap().version,
        cancelled.version
    );
    assert_eq!(store::current(&db, &live.id).await.unwrap().version, 2);

    // A listing carries no message text; a single read does.
    let open = queue::list_open(&db).await.unwrap();
    assert!(open.len() >= 8);
    assert!(open.iter().all(|e| e.message.is_none() && e.note.is_none()));
    // Read one of this test's own entries, not whichever is oldest: other tests leave entries too.
    let one = queue::get(&db, &mine[0]).await.unwrap();
    assert_eq!(one.message.as_deref(), Some(SECRET));
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_person_clears_an_entry_once_by_matching_it_or_dismissing_it() {
    let db = db().await;
    let r = referral(&db).await;
    let entry = queued(
        queue::file(&db, inbound(None, Some("outcome"), Some("x")), Utc::now())
            .await
            .unwrap(),
    );
    // A reason is required; a hand match names a referral that exists.
    assert!(matches!(
        queue::clear(&db, &entry.id, "admin-1", Clearance::Dismissed, "  ").await,
        Err(StoreError::BadQuery(_))
    ));
    assert!(matches!(
        queue::clear(
            &db,
            &entry.id,
            "admin-1",
            Clearance::Matched("11111111-1111-4111-8111-111111111111"),
            "found it"
        )
        .await,
        Err(StoreError::NotFound)
    ));
    let before = store::current(&db, &r.id).await.unwrap();
    let cleared = queue::clear(
        &db,
        &entry.id,
        "admin-1",
        Clearance::Matched(&r.id),
        "the letter is the cardiology outcome",
    )
    .await
    .unwrap();
    let c = cleared.cleared.expect("cleared");
    assert_eq!(
        (c.cleared_by.as_str(), c.how.as_str()),
        ("admin-1", "matched")
    );
    assert_eq!(c.referral_id.as_deref(), Some(r.id.as_str()));
    // A hand match records the link and changes nothing in the referral.
    assert_eq!(
        store::current(&db, &r.id).await.unwrap().version,
        before.version
    );
    // Once.
    assert!(matches!(
        queue::clear(&db, &entry.id, "admin-2", Clearance::Dismissed, "again").await,
        Err(StoreError::Conflict { .. })
    ));
    assert!(
        !queue::list_open(&db)
            .await
            .unwrap()
            .iter()
            .any(|e| e.id == entry.id)
    );

    let other = queued(
        queue::file(&db, inbound(None, None, None), Utc::now())
            .await
            .unwrap(),
    );
    let dismissed = queue::clear(
        &db,
        &other.id,
        "admin-1",
        Clearance::Dismissed,
        "a test message",
    )
    .await
    .unwrap();
    let c = dismissed.cleared.expect("cleared");
    assert_eq!((c.how.as_str(), c.referral_id), ("dismissed", None));
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn the_database_refuses_to_edit_delete_or_truncate_the_queue() {
    let db = db().await;
    let entry = queued(
        queue::file(&db, inbound(None, None, None), Utc::now())
            .await
            .unwrap(),
    );
    queue::clear(&db, &entry.id, "admin-1", Clearance::Dismissed, "test")
        .await
        .unwrap();
    for sql in [
        format!(
            "UPDATE referral_unmatched SET agent = 'x' WHERE unmatched_id = '{}'",
            entry.id
        ),
        format!(
            "DELETE FROM referral_unmatched WHERE unmatched_id = '{}'",
            entry.id
        ),
        "TRUNCATE referral_unmatched CASCADE".to_string(),
        format!(
            "UPDATE referral_unmatched_clearances SET reason = 'x' WHERE unmatched_id = '{}'",
            entry.id
        ),
        format!(
            "DELETE FROM referral_unmatched_clearances WHERE unmatched_id = '{}'",
            entry.id
        ),
        "TRUNCATE referral_unmatched_clearances".to_string(),
    ] {
        let e = db.execute_unprepared(&sql).await.unwrap_err();
        assert!(e.to_string().contains("append-only"), "{sql}: {e}");
    }
    assert!(queue::get(&db, &entry.id).await.unwrap().cleared.is_some());
}
