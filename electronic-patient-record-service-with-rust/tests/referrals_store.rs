//! DB-gated (CA-3, first slice): the referral store keeps every step as a new version; the state table of
//! `referral.rs` holds through it; the actor and the note each event needs are enforced; a stale version is a
//! conflict; and the database itself refuses to edit, delete or truncate a version.
//!
//! Covered: a request, accept and outcome with the full history; a cancellation as a request that only the
//! recipient confirms (and the referrer cannot); a decline after acceptance; a refused transition, a missing
//! note, a stale version and an unknown patient; a patient's list; the three triggers. Not covered: routes,
//! authorisation, the unmatched-outcome queue (PC-2), timeouts (PC-3), concurrent writers.
//!
//! ```text
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test main referrals_store:: -- --ignored --test-threads=1
//! ```
use electronic_patient_record::patients_store::{self, StoreError};
use electronic_patient_record::referral::{Actor, Event, State};
use electronic_patient_record::referrals_store::{self as store, Priority, Referral, Step};
use electronic_patient_record_matcher::{HumanName, Patient};
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

async fn patient(db: &PgPool) -> String {
    let family = format!(
        "ref{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    patients_store::create(
        db,
        "dr-1",
        Some("referral store test"),
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
    .unwrap()
    .id
}

fn referral() -> Referral {
    Referral {
        recipient: "Cardiology outpatients".into(),
        reason: "Exertional chest pain".into(),
        question: "Is this angina?".into(),
        priority: Priority::Routine,
        external_ids: vec![],
        replaces: None,
    }
}

fn step(actor: Actor, event: Event, note: Option<&'static str>) -> Step<'static> {
    Step {
        actor,
        by: match actor {
            Actor::Referrer => "dr-1",
            Actor::Recipient => "clinic-1",
        },
        event,
        note,
        missed: None,
    }
}

fn missed(why: Option<&'static str>) -> Step<'static> {
    Step {
        missed: why,
        ..step(Actor::Recipient, Event::RecordNoShow, None)
    }
}

/// XP-15: a missed appointment has a reason of the list, kept with the words it had; nothing else has one.
#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_missed_appointment_needs_a_reason_and_keeps_its_words() {
    let db = db().await;
    let p = patient(&db).await;
    let now = jiff::Timestamp::now();
    let r = store::request(&db, "dr-1", &p, &referral(), now)
        .await
        .unwrap();
    let go = async |version, s| store::advance(&db, &r.id, version, s, now).await;
    go(1, step(Actor::Recipient, Event::Accept, None))
        .await
        .unwrap();
    // Not allowed here, and refused as that and not for want of a reason.
    match go(2, missed(None)).await {
        Err(StoreError::BadQuery(m)) => assert!(m.contains("cannot take"), "{m}"),
        other => panic!("{other:?}"),
    }
    let booked = go(2, step(Actor::Recipient, Event::Book, None))
        .await
        .unwrap();
    let offered = booked
        .next
        .iter()
        .find(|n| n.event == "record-no-show")
        .unwrap();
    assert!(offered.reasons.iter().any(|g| g.code == "unknown"));
    for (why, says) in [
        (None, "needs a reason"),
        (Some("  "), "needs a reason"),
        (Some("no-such-reason"), "not one of"),
        (Some("Unknown"), "not one of"),
    ] {
        match go(3, missed(why)).await {
            Err(StoreError::BadQuery(m)) => {
                assert!(m.contains(says), "{why:?}: {m}");
                assert!(!m.contains("no-such-reason"), "not quoted: {m}");
            }
            other => panic!("{why:?}: {other:?}"),
        }
    }
    let with_reason = Step {
        missed: Some("unknown"),
        ..step(Actor::Referrer, Event::RequestCancel, Some("why"))
    };
    // Only a missed appointment has one.
    let other = store::request(&db, "dr-1", &p, &referral(), now)
        .await
        .unwrap();
    match store::advance(&db, &other.id, 1, with_reason, now).await {
        Err(StoreError::BadQuery(m)) => assert!(m.contains("only a missed"), "{m}"),
        o => panic!("{o:?}"),
    }
    let gone = go(3, missed(Some("not-brought"))).await.unwrap();
    assert_eq!(gone.state, State::NoShow);
    assert_eq!(
        gone.last_event["missed"],
        serde_json::json!({"code": "not-brought", "words": "Not brought by the person who brings them"})
    );
    let again = go(4, step(Actor::Recipient, Event::Book, None))
        .await
        .unwrap();
    assert!(again.last_event["missed"].is_null());
}

/// One recorded before a reason was asked for says nothing, which is not "unknown" and not the patient's choice.
#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn an_old_missed_appointment_has_no_reason() {
    let db = db().await;
    let p = patient(&db).await;
    let now = jiff::Timestamp::now();
    let r = store::request(&db, "dr-1", &p, &referral(), now)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO referral_versions (referral_id, version, patient_id, author, state, event, body)
         SELECT referral_id, 2, patient_id, 'clinic-1', 'no-show',
                '{\"event\": \"record-no-show\", \"by\": \"clinic-1\", \"note\": null}'::jsonb, body
         FROM referral_versions WHERE referral_id = $1::uuid",
    )
    .bind(r.id.clone())
    .execute(&db)
    .await
    .unwrap();
    let old = store::current(&db, &r.id).await.unwrap();
    assert_eq!(old.state, State::NoShow);
    assert!(old.last_event.get("missed").is_none());
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_referral_runs_from_request_to_outcome_and_keeps_every_step() {
    let db = db().await;
    let now = jiff::Timestamp::now();
    let pid = patient(&db).await;
    let r = store::request(&db, "dr-1", &pid, &referral(), now)
        .await
        .unwrap();
    assert_eq!((r.version, r.state), (1, State::Requested));
    assert_eq!(r.patient_id, pid);
    assert_eq!(r.last_event["event"], "request");

    let r = store::advance(
        &db,
        &r.id,
        1,
        step(Actor::Recipient, Event::Accept, None),
        now,
    )
    .await
    .unwrap();
    assert_eq!((r.version, r.state), (2, State::Accepted));
    let r = store::advance(
        &db,
        &r.id,
        2,
        step(
            Actor::Recipient,
            Event::Outcome,
            Some("Seen; not angina; discharged"),
        ),
        now,
    )
    .await
    .unwrap();
    assert_eq!((r.version, r.state), (3, State::Completed));
    assert_eq!(r.last_event["note"], "Seen; not angina; discharged");
    assert_eq!(r.referral, referral(), "the request itself does not change");

    let history = store::history(&db, &r.id).await.unwrap();
    assert_eq!(
        history.iter().map(|h| h.state).collect::<Vec<_>>(),
        [State::Requested, State::Accepted, State::Completed]
    );
    assert_eq!(store::current(&db, &r.id).await.unwrap().version, 3);
    assert!(
        store::for_patient(&db, &pid)
            .await
            .unwrap()
            .iter()
            .any(|x| x.id == r.id && x.state == State::Completed)
    );
    // An end state takes nothing more.
    let e = store::advance(
        &db,
        &r.id,
        3,
        step(Actor::Recipient, Event::Decline, Some("too late")),
        now,
    )
    .await
    .unwrap_err();
    assert!(matches!(e, StoreError::BadQuery(_)), "{e:?}");
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_cancellation_is_a_request_that_only_the_recipient_confirms() {
    let db = db().await;
    let now = jiff::Timestamp::now();
    let pid = patient(&db).await;
    let r = store::request(&db, "dr-1", &pid, &referral(), now)
        .await
        .unwrap();
    // The referrer asks; the referral is not cancelled.
    let r = store::advance(
        &db,
        &r.id,
        1,
        step(
            Actor::Referrer,
            Event::RequestCancel,
            Some("patient moved away"),
        ),
        now,
    )
    .await
    .unwrap();
    assert_eq!(r.state, State::CancelRequested);
    assert_ne!(r.state, State::Cancelled);
    // The referrer cannot confirm their own request.
    let e = store::advance(
        &db,
        &r.id,
        2,
        step(Actor::Referrer, Event::ConfirmCancel, None),
        now,
    )
    .await
    .unwrap_err();
    assert!(
        matches!(e, StoreError::BadQuery(ref m) if m.contains("recipient")),
        "{e:?}"
    );
    // The recipient confirms.
    let r = store::advance(
        &db,
        &r.id,
        2,
        step(Actor::Recipient, Event::ConfirmCancel, None),
        now,
    )
    .await
    .unwrap();
    assert_eq!((r.version, r.state), (3, State::Cancelled));

    // Or the recipient answers a request with an outcome instead.
    let r2 = store::request(&db, "dr-1", &pid, &referral(), now)
        .await
        .unwrap();
    let r2 = store::advance(
        &db,
        &r2.id,
        1,
        step(
            Actor::Referrer,
            Event::RequestCancel,
            Some("no longer needed"),
        ),
        now,
    )
    .await
    .unwrap();
    let r2 = store::advance(
        &db,
        &r2.id,
        2,
        step(
            Actor::Recipient,
            Event::Outcome,
            Some("Already seen; report attached"),
        ),
        now,
    )
    .await
    .unwrap();
    assert_eq!(r2.state, State::Completed);
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_decline_can_follow_acceptance_and_bad_steps_are_refused() {
    let db = db().await;
    let now = jiff::Timestamp::now();
    let pid = patient(&db).await;
    let r = store::request(&db, "dr-1", &pid, &referral(), now)
        .await
        .unwrap();
    let r = store::advance(
        &db,
        &r.id,
        1,
        step(Actor::Recipient, Event::Accept, None),
        now,
    )
    .await
    .unwrap();
    // A decline needs a note, from the recipient, and a current version.
    let no_note = store::advance(
        &db,
        &r.id,
        2,
        step(Actor::Recipient, Event::Decline, None),
        now,
    )
    .await
    .unwrap_err();
    assert!(
        matches!(no_note, StoreError::BadQuery(ref m) if m.contains("note")),
        "{no_note:?}"
    );
    let blank = store::advance(
        &db,
        &r.id,
        2,
        step(Actor::Recipient, Event::Decline, Some("   ")),
        now,
    )
    .await
    .unwrap_err();
    assert!(matches!(blank, StoreError::BadQuery(_)), "{blank:?}");
    let wrong_actor = store::advance(
        &db,
        &r.id,
        2,
        step(Actor::Referrer, Event::Decline, Some("x")),
        now,
    )
    .await
    .unwrap_err();
    assert!(
        matches!(wrong_actor, StoreError::BadQuery(_)),
        "{wrong_actor:?}"
    );
    let stale = store::advance(
        &db,
        &r.id,
        1,
        step(Actor::Recipient, Event::Decline, Some("cannot take this")),
        now,
    )
    .await
    .unwrap_err();
    assert!(
        matches!(stale, StoreError::Conflict { current: 2 }),
        "{stale:?}"
    );
    let declined = store::advance(
        &db,
        &r.id,
        2,
        step(Actor::Recipient, Event::Decline, Some("cannot take this")),
        now,
    )
    .await
    .unwrap();
    assert_eq!(declined.state, State::Declined);
    // A transition the table does not list: an interim note is not allowed from `requested`.
    let r2 = store::request(&db, "dr-1", &pid, &referral(), now)
        .await
        .unwrap();
    let e = store::advance(
        &db,
        &r2.id,
        1,
        step(Actor::Recipient, Event::InterimNote, Some("update")),
        now,
    )
    .await
    .unwrap_err();
    assert!(matches!(e, StoreError::BadQuery(_)), "{e:?}");
    // The refused steps left the referral where it was.
    assert_eq!(store::current(&db, &r2.id).await.unwrap().version, 1);
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_request_needs_a_real_patient_and_a_whole_request() {
    let db = db().await;
    let now = jiff::Timestamp::now();
    let none = store::request(
        &db,
        "dr-1",
        "00000000-0000-4000-8000-000000000000",
        &referral(),
        now,
    )
    .await
    .unwrap_err();
    assert!(matches!(none, StoreError::NotFound), "{none:?}");
    let pid = patient(&db).await;
    let blank = store::request(
        &db,
        "dr-1",
        &pid,
        &Referral {
            question: " ".into(),
            ..referral()
        },
        now,
    )
    .await
    .unwrap_err();
    assert!(matches!(blank, StoreError::BadQuery(_)), "{blank:?}");
    assert!(matches!(
        store::current(&db, "not-a-uuid").await,
        Err(StoreError::NotFound)
    ));
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn the_database_refuses_to_edit_delete_or_truncate_a_referral_version() {
    let db = db().await;
    let pid = patient(&db).await;
    let r = store::request(&db, "dr-1", &pid, &referral(), jiff::Timestamp::now())
        .await
        .unwrap();
    for sql in [
        format!(
            "UPDATE referral_versions SET author = 'x' WHERE referral_id = '{}'",
            r.id
        ),
        format!(
            "DELETE FROM referral_versions WHERE referral_id = '{}'",
            r.id
        ),
        // CASCADE: acknowledgements reference the versions, and a bare TRUNCATE would be refused for that reason first.
        "TRUNCATE referral_versions CASCADE".to_string(),
    ] {
        let e = sqlx::raw_sql(AssertSqlSafe(sql.to_string()))
            .execute(&db)
            .await
            .unwrap_err();
        assert!(e.to_string().contains("append-only"), "{sql}: {e}");
    }
    assert_eq!(store::current(&db, &r.id).await.unwrap().version, 1);
}
