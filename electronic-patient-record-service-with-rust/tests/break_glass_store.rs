//! DB-gated: break-the-glass grants (task SE-3) last four hours and are not extended by asking again,
//! need a reason, are reviewed once by someone other than the person who opened them, and can not be
//! changed or removed.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test main break_glass_store:: -- --ignored --test-threads=1
//! ```

use electronic_patient_record::break_glass::{self as glass, GRANT_HOURS, Outcome};
use electronic_patient_record::emergency::{CanBeAsked, Emergency, Threat, TimeToHarm};
use electronic_patient_record::patients_store::{self, StoreError};
use electronic_patient_record_matcher::{HumanName, Patient};
use sqlx::{AssertSqlSafe, PgPool, Row};

async fn db() -> PgPool {
    let url = std::env::var("DB_URL").expect("DB_URL must be set for --ignored tests");
    let db = PgPool::connect(&url).await.expect("connect");
    electronic_patient_record::MIGRATOR
        .run(&db)
        .await
        .expect("migrate");
    db
}

/// The answers of a grant where nobody could ask the patient.
fn emergency() -> Emergency {
    Emergency {
        threat: Threat::Life,
        time_to_harm: TimeToHarm::Minutes,
        can_be_asked: CanBeAsked::No,
        asked_and_said: None,
    }
}

fn unique(label: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{label}-{nanos}")
}

async fn patient(db: &PgPool) -> String {
    patients_store::create(
        db,
        "tester",
        None,
        &Patient {
            names: vec![HumanName {
                family: Some(unique("Glass")),
                given: vec!["Test".into()],
            }],
            birth_date: Some("1980-01-01".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap()
    .id
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_grant_lasts_four_hours_and_asking_again_does_not_extend_it() {
    let db = db().await;
    let (p, who) = (patient(&db).await, unique("dr"));
    assert!(glass::active(&db, &p, &who).await.unwrap().is_none());

    let (g, created) = glass::open(&db, &p, &who, "  collapsed in the corridor ", &emergency())
        .await
        .unwrap();
    assert!(created);
    assert_eq!(g.reason, "collapsed in the corridor");
    assert_eq!(g.patient_id, p);
    let now = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    )
    .unwrap();
    let left = g.expires_unix - now;
    assert!(
        (i64::from(GRANT_HOURS) * 3600 - 60..=i64::from(GRANT_HOURS) * 3600).contains(&left),
        "{left}"
    );
    assert_eq!(
        glass::active(&db, &p, &who).await.unwrap().unwrap().id,
        g.id
    );

    // Again: the same grant, same end time.
    let (again, created) = glass::open(&db, &p, &who, "a different reason", &emergency())
        .await
        .unwrap();
    assert!(!created);
    assert_eq!(
        (again.id.as_str(), again.expires_unix),
        (g.id.as_str(), g.expires_unix)
    );

    // Someone else, or another patient, has no grant.
    assert!(
        glass::active(&db, &p, "someone-else")
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        glass::active(&db, &patient(&db).await, &who)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn an_expired_grant_is_not_active_and_a_new_one_can_be_opened() {
    let db = db().await;
    let (p, who) = (patient(&db).await, unique("dr"));
    sqlx::query(
        "INSERT INTO break_glass_grants (patient_id, subject, reason, granted_at, expires_at)
         VALUES ($1::uuid, $2, 'old', now() - interval '5 hours', now() - interval '1 hour')",
    )
    .bind(p.clone())
    .bind(who.clone())
    .execute(&db)
    .await
    .unwrap();
    assert!(glass::active(&db, &p, &who).await.unwrap().is_none());
    let (_, created) = glass::open(&db, &p, &who, "new emergency", &emergency())
        .await
        .unwrap();
    assert!(created);
}

/// PR-5: the edges of the four hours and of the review limit, by moving the clock and not by writing rows in the past.
#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_grant_ends_at_four_hours_to_the_microsecond_and_is_overdue_just_after_the_limit() {
    use electronic_patient_record::clock::{Clock, FixedClock};
    use jiff::SignedDuration;
    let db = db().await;
    let (p, who) = (patient(&db).await, unique("dr"));
    // A time of its own: the rows of other tests are near the real one.
    let clock = FixedClock::new("2031-05-06T07:08:09.123456Z".parse().unwrap());
    let moment = SignedDuration::from_micros(1);
    let start = clock.now();
    let (g, created) = glass::open_at(&db, &p, &who, "collapsed", &emergency(), Some(clock.now()))
        .await
        .unwrap();
    assert!(created);
    assert_eq!(g.granted_at, "2031-05-06T07:08:09.123456Z");
    assert_eq!(g.expires_at, "2031-05-06T11:08:09.123456Z");

    clock.advance(SignedDuration::from_hours(i64::from(GRANT_HOURS)) - moment);
    let held = glass::active_at(&db, &p, &who, Some(clock.now()))
        .await
        .unwrap()
        .expect("still held a microsecond before the end");
    assert_eq!(held.id, g.id);
    // Asking again at the last moment gives the same end.
    let (again, created) = glass::open_at(&db, &p, &who, "again", &emergency(), Some(clock.now()))
        .await
        .unwrap();
    assert!(!created);
    assert_eq!(again.expires_at, g.expires_at);

    clock.advance(moment);
    assert!(
        glass::active_at(&db, &p, &who, Some(clock.now()))
            .await
            .unwrap()
            .is_none(),
        "over at four hours exactly"
    );
    let (next, created) = glass::open_at(&db, &p, &who, "another", &emergency(), Some(clock.now()))
        .await
        .unwrap();
    assert!(created);
    assert_ne!(next.id, g.id);

    // The review: due at the limit, overdue only after it, and no longer once reviewed.
    let due = start + SignedDuration::from_hours(glass::DEFAULT_REVIEW_DUE_HOURS);
    let overdue_at = async |t| glass::get_at(&db, &g.id, Some(t)).await.unwrap().overdue;
    assert!(!overdue_at(due).await, "at the limit");
    assert!(overdue_at(due + moment).await, "a microsecond after it");
    let counted = async |t| glass::count_overdue_at(&db, Some(t)).await.unwrap();
    assert_eq!(counted(due + moment).await, counted(due).await + 1);
    let listed = glass::unreviewed_at(&db, Some(due + moment)).await.unwrap();
    if let Some(found) = listed.iter().find(|x| x.id == g.id) {
        assert!(found.overdue);
    }
    glass::review(&db, &g.id, "auditor", Outcome::Appropriate, "emergency")
        .await
        .unwrap();
    assert!(!overdue_at(due + moment).await);
}

/// Two requests at one moment, as from a double press: the second waits for the first and is given its grant. Each pair
/// of person and patient waits only for itself.
#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn two_requests_at_one_moment_open_one_grant() {
    let db = db().await;
    let (p, who) = (patient(&db).await, unique("dr"));
    let mut first = db.begin().await.unwrap();
    let (g, created) = glass::open(&mut *first, &p, &who, "collapsed", &emergency())
        .await
        .unwrap();
    assert!(created);
    let (other, at, name) = (db.clone(), p.clone(), who.clone());
    let second = tokio::spawn(async move {
        let mut tx = other.begin().await.unwrap();
        let got = glass::open(&mut *tx, &at, &name, "collapsed", &emergency())
            .await
            .unwrap();
        tx.commit().await.unwrap();
        got
    });
    // Someone else, for the same patient, does not wait for it.
    let (_, theirs) = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        glass::open(&db, &p, &unique("other"), "collapsed", &emergency()),
    )
    .await
    .expect("another person does not wait")
    .unwrap();
    assert!(theirs);
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert!(!second.is_finished(), "it waits for the first");
    first.commit().await.unwrap();
    let (again, created) = second.await.unwrap();
    assert!(!created);
    assert_eq!(again.id, g.id);
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_grant_needs_a_reason_and_a_patient() {
    let db = db().await;
    let (p, who) = (patient(&db).await, unique("dr"));
    for bad in ["", "   ", &"x".repeat(501), "line\u{0}break"] {
        assert!(
            matches!(
                glass::open(&db, &p, &who, bad, &emergency()).await,
                Err(StoreError::BadQuery(_))
            ),
            "{bad:?}"
        );
    }
    assert_eq!(
        glass::open(
            &db,
            "00000000-0000-4000-8000-000000000000",
            &who,
            "why",
            &emergency()
        )
        .await
        .map(|_| ()),
        Err(StoreError::NotFound)
    );
    assert!(glass::active(&db, &p, &who).await.unwrap().is_none());
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_grant_waits_for_review_by_someone_else_and_is_reviewed_once() {
    let db = db().await;
    let (p, who, auditor) = (patient(&db).await, unique("dr"), unique("auditor"));
    let waiting = glass::count_unreviewed(&db).await.unwrap();
    let (g, _) = glass::open(&db, &p, &who, "emergency", &emergency())
        .await
        .unwrap();
    assert_eq!(glass::count_unreviewed(&db).await.unwrap(), waiting + 1);
    assert!(
        glass::unreviewed(&db)
            .await
            .unwrap()
            .iter()
            .all(|x| x.review.is_none())
    );

    // Not by the person who opened it; not without a note.
    assert!(matches!(
        glass::review(&db, &g.id, &who, Outcome::Appropriate, "me").await,
        Err(StoreError::BadQuery(_))
    ));
    assert!(matches!(
        glass::review(&db, &g.id, &auditor, Outcome::Appropriate, "  ").await,
        Err(StoreError::BadQuery(_))
    ));
    assert_eq!(glass::get(&db, &g.id).await.unwrap().review, None);

    let reviewed = glass::review(
        &db,
        &g.id,
        &auditor,
        Outcome::Inappropriate,
        "no emergency found",
    )
    .await
    .unwrap();
    let r = reviewed.review.unwrap();
    assert_eq!(
        (r.reviewed_by.as_str(), r.outcome.as_str()),
        (auditor.as_str(), "inappropriate")
    );
    assert_eq!(glass::count_unreviewed(&db).await.unwrap(), waiting);
    assert!(matches!(
        glass::review(&db, &g.id, "another", Outcome::Appropriate, "again").await,
        Err(StoreError::Conflict { .. })
    ));
    // A reviewed grant still lets its holder in until it ends: review is after the fact.
    assert!(glass::active(&db, &p, &who).await.unwrap().is_some());
    assert_eq!(
        glass::review(&db, "not-a-uuid", &auditor, Outcome::Appropriate, "x")
            .await
            .map(|_| ()),
        Err(StoreError::NotFound)
    );
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn grants_and_reviews_cannot_be_changed_or_removed() {
    let db = db().await;
    let (p, who) = (patient(&db).await, unique("dr"));
    let (g, _) = glass::open(&db, &p, &who, "emergency", &emergency())
        .await
        .unwrap();
    glass::review(&db, &g.id, "auditor", Outcome::Appropriate, "fine")
        .await
        .unwrap();
    for sql in [
        format!(
            "UPDATE break_glass_grants SET expires_at = now() + interval '1 year' WHERE grant_id = '{}'",
            g.id
        ),
        format!("DELETE FROM break_glass_grants WHERE grant_id = '{}'", g.id),
        format!(
            "UPDATE break_glass_reviews SET outcome = 'inappropriate' WHERE grant_id = '{}'",
            g.id
        ),
        format!(
            "DELETE FROM break_glass_reviews WHERE grant_id = '{}'",
            g.id
        ),
        "TRUNCATE break_glass_reviews".to_string(),
        "TRUNCATE break_glass_grants CASCADE".to_string(),
    ] {
        let e = sqlx::raw_sql(AssertSqlSafe(sql.to_string()))
            .execute(&db)
            .await
            .unwrap_err()
            .to_string();
        assert!(e.contains("append-only"), "{sql}: {e}");
    }
}

async fn old_grant(
    db: impl Into<electronic_patient_record::pool::Db<'_>>,
    patient: &str,
    subject: &str,
    hours_ago: i64,
) -> String {
    let mut conn = db.into().acquire().await.unwrap();
    let db = &mut *conn;
    let row = sqlx::query(AssertSqlSafe(format!(
                "INSERT INTO break_glass_grants (patient_id, subject, reason, granted_at, expires_at)
                 VALUES ('{patient}'::uuid, '{subject}', 'old emergency',
                         now() - interval '{hours_ago} hours', now() - interval '{hours_ago} hours' + interval '4 hours')
                 RETURNING grant_id::text AS id"
            )))
.fetch_optional(&mut *db)
        .await
        .unwrap()
        .unwrap();
    row.try_get("id").unwrap()
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_grant_unreviewed_for_longer_than_the_limit_is_overdue_until_it_is_reviewed() {
    let db = db().await;
    let p = patient(&db).await;
    let before = glass::count_overdue(&db).await.unwrap();

    // Inside the limit: waiting, not overdue.
    let (fresh, _) = glass::open(&db, &p, &unique("dr"), "collapsed", &emergency())
        .await
        .unwrap();
    // Just inside, and well outside.
    let inside = old_grant(&db, &p, &unique("dr"), glass::DEFAULT_REVIEW_DUE_HOURS - 1).await;
    let outside = old_grant(&db, &p, &unique("dr"), glass::DEFAULT_REVIEW_DUE_HOURS + 1).await;

    let waiting = glass::unreviewed(&db).await.unwrap();
    let find = |id: &str| {
        waiting
            .iter()
            .find(|g| g.id == id)
            .unwrap_or_else(|| panic!("{id}"))
            .overdue
    };
    assert!(!find(&fresh.id));
    assert!(!find(&inside), "inside the limit");
    assert!(find(&outside), "past the limit");
    assert_eq!(glass::count_overdue(&db).await.unwrap(), before + 1);

    // A review ends the overdue state.
    glass::review(
        &db,
        &outside,
        "an-auditor",
        Outcome::Appropriate,
        "emergency",
    )
    .await
    .unwrap();
    assert_eq!(glass::count_overdue(&db).await.unwrap(), before);
    assert!(!glass::get(&db, &outside).await.unwrap().overdue);
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_review_list_longer_than_the_list_limit_says_it_was_cut_short() {
    // PR-1 (hazard H-044): oldest first and capped, so the newest unreviewed access is the one hidden.
    // Done in a transaction that is rolled back, so other tests' counts of unreviewed grants are untouched.
    let db = db().await;
    let p = patient(&db).await;
    let mut txn = db.begin().await.unwrap();
    let mut open = glass::count_unreviewed(&mut *txn).await.unwrap();
    while open < i64::try_from(glass::LIST_LIMIT).unwrap() + 1 {
        old_grant(&mut *txn, &p, &unique("dr"), 1).await;
        open += 1;
    }
    let page = glass::unreviewed(&mut *txn).await.unwrap();
    assert_eq!(page.len() as u64, glass::LIST_LIMIT);
    assert!(
        page.more,
        "{open} are unreviewed but the list does not say it is cut"
    );
    txn.rollback().await.unwrap();
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_grant_holds_the_openers_answers_and_one_from_before_they_were_asked_says_so() {
    let db = db().await;
    let p = patient(&db).await;
    // What the patient said is kept, trimmed, when they could be asked; asking again changes none of it.
    let asked = Emergency {
        threat: Threat::Limb,
        time_to_harm: TimeToHarm::Hours,
        can_be_asked: CanBeAsked::Yes,
        asked_and_said: Some("  said to go ahead  ".into()),
    };
    let who = unique("dr");
    let (g, _) = glass::open(&db, &p, &who, "crush injury", &asked)
        .await
        .unwrap();
    let held = g.emergency.clone().expect("the answers are held");
    assert_eq!(held.threat, Threat::Limb);
    assert_eq!(held.asked_and_said.as_deref(), Some("said to go ahead"));
    let (again, created) = glass::open(&db, &p, &who, "again", &emergency())
        .await
        .unwrap();
    assert!(!created);
    assert_eq!(again.emergency, g.emergency);
    // The review reads them.
    assert_eq!(glass::get(&db, &g.id).await.unwrap().emergency, g.emergency);

    // An answer that does not fit is refused, and nothing is written.
    let said_but_unasked = Emergency {
        asked_and_said: Some("x".into()),
        ..emergency()
    };
    assert!(matches!(
        glass::open(&db, &p, &unique("dr"), "why", &said_but_unasked).await,
        Err(StoreError::BadQuery(_))
    ));
    let unsaid = Emergency {
        can_be_asked: CanBeAsked::Yes,
        ..emergency()
    };
    assert!(matches!(
        glass::open(&db, &p, &unique("dr"), "why", &unsaid).await,
        Err(StoreError::BadQuery(_))
    ));

    // The database refuses half an answer, and a grant from before the question reads as not asked.
    let half = sqlx::query(
        "INSERT INTO break_glass_grants (patient_id, subject, reason, expires_at, threat)
             VALUES ($1::uuid, 'x', 'r', now() + interval '1 hour', 'life')",
    )
    .bind(&p)
    .execute(&db)
    .await;
    assert!(half.is_err(), "a threat with no other answer was stored");
    let old: String = sqlx::query(
        "INSERT INTO break_glass_grants (patient_id, subject, reason, expires_at)
             VALUES ($1::uuid, $2, 'before RF-6', now() + interval '1 hour') RETURNING grant_id::text AS id",
    )
    .bind(&p)
    .bind(unique("old"))
    .fetch_one(&db)
    .await
    .unwrap()
    .get("id");
    assert_eq!(glass::get(&db, &old).await.unwrap().emergency, None);
}
