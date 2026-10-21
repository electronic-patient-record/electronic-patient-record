//! DB-gated: break-the-glass grants (task SE-3) last four hours and are not extended by asking again,
//! need a reason, are reviewed once by someone other than the person who opened them, and can not be
//! changed or removed.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test break_glass_store -- --ignored --test-threads=1
//! ```

use electronic_patient_record::break_glass::{self as glass, GRANT_HOURS, Outcome};
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

fn unique(label: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{label}-{nanos}")
}

async fn patient(db: &DatabaseConnection) -> String {
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
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_grant_lasts_four_hours_and_asking_again_does_not_extend_it() {
    let db = db().await;
    let (p, who) = (patient(&db).await, unique("dr"));
    assert!(glass::active(&db, &p, &who).await.unwrap().is_none());

    let (g, created) = glass::open(&db, &p, &who, "  collapsed in the corridor ")
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
        (GRANT_HOURS * 3600 - 60..=GRANT_HOURS * 3600).contains(&left),
        "{left}"
    );
    assert_eq!(
        glass::active(&db, &p, &who).await.unwrap().unwrap().id,
        g.id
    );

    // Again: the same grant, same end time.
    let (again, created) = glass::open(&db, &p, &who, "a different reason")
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
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn an_expired_grant_is_not_active_and_a_new_one_can_be_opened() {
    let db = db().await;
    let (p, who) = (patient(&db).await, unique("dr"));
    db.execute_raw(Statement::from_sql_and_values(
        sea_orm::DbBackend::Postgres,
        "INSERT INTO break_glass_grants (patient_id, subject, reason, granted_at, expires_at)
         VALUES ($1::uuid, $2, 'old', now() - interval '5 hours', now() - interval '1 hour')",
        [p.clone().into(), who.clone().into()],
    ))
    .await
    .unwrap();
    assert!(glass::active(&db, &p, &who).await.unwrap().is_none());
    let (_, created) = glass::open(&db, &p, &who, "new emergency").await.unwrap();
    assert!(created);
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_grant_needs_a_reason_and_a_patient() {
    let db = db().await;
    let (p, who) = (patient(&db).await, unique("dr"));
    for bad in ["", "   ", &"x".repeat(501), "line\u{0}break"] {
        assert!(
            matches!(
                glass::open(&db, &p, &who, bad).await,
                Err(StoreError::BadQuery(_))
            ),
            "{bad:?}"
        );
    }
    assert_eq!(
        glass::open(&db, "00000000-0000-4000-8000-000000000000", &who, "why")
            .await
            .map(|_| ()),
        Err(StoreError::NotFound)
    );
    assert!(glass::active(&db, &p, &who).await.unwrap().is_none());
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_grant_waits_for_review_by_someone_else_and_is_reviewed_once() {
    let db = db().await;
    let (p, who, auditor) = (patient(&db).await, unique("dr"), unique("auditor"));
    let waiting = glass::count_unreviewed(&db).await.unwrap();
    let (g, _) = glass::open(&db, &p, &who, "emergency").await.unwrap();
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
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn grants_and_reviews_cannot_be_changed_or_removed() {
    let db = db().await;
    let (p, who) = (patient(&db).await, unique("dr"));
    let (g, _) = glass::open(&db, &p, &who, "emergency").await.unwrap();
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
        let e = db.execute_unprepared(&sql).await.unwrap_err().to_string();
        assert!(e.contains("append-only"), "{sql}: {e}");
    }
}
