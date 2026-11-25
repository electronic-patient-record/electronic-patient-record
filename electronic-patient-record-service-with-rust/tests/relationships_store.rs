//! DB-gated: relationship versions are append-only and optimistic, and `lookup` returns the
//! strongest relationship in force for one subject and one patient only.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test main relationships_store:: -- --ignored --test-threads=1
//! ```

use electronic_patient_record::patients_store::{self, StoreError};
use electronic_patient_record::policy::Relationship;
use electronic_patient_record::proxy::{Basis, Scope, Terms};
use electronic_patient_record::relationships::{self as store, Kind, RelationshipRecord, Status};
use electronic_patient_record_matcher::{HumanName, Patient};
use sqlx::{AssertSqlSafe, PgPool};

const NOW: i64 = 1_790_000_000;

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
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let p = Patient {
        names: vec![HumanName {
            family: Some(format!("Zz{nanos}")),
            given: vec![],
        }],
        ..Default::default()
    };
    patients_store::create(db, "t", None, &p).await.unwrap().id
}

fn rec(subject: &str, kind: Kind, status: Status) -> RelationshipRecord {
    RelationshipRecord {
        subject: subject.into(),
        kind,
        status,
        period: None,
        proxy: (kind == Kind::Proxy).then(|| Terms {
            basis: Basis::PatientRequest,
            scopes: [Scope::Identity].into(),
            expires_at: NOW + 86_400,
        }),
    }
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn lookup_is_per_subject_and_per_patient_and_takes_the_strongest() {
    let db = db().await;
    let (a, b) = (patient(&db).await, patient(&db).await);
    store::create(
        &db,
        &a,
        "admin",
        None,
        &rec("alice", Kind::CareTeam, Status::Active),
    )
    .await
    .unwrap();
    store::create(
        &db,
        &a,
        "admin",
        None,
        &rec("alice", Kind::Proxy, Status::Active),
    )
    .await
    .unwrap();
    store::create(
        &db,
        &a,
        "admin",
        None,
        &rec("bob", Kind::SelfPatient, Status::Active),
    )
    .await
    .unwrap();

    assert_eq!(
        store::lookup(&db, &a, "alice", NOW, Some(Scope::Identity))
            .await
            .unwrap(),
        Relationship::Proxy
    );
    assert_eq!(
        store::lookup(&db, &a, "bob", NOW, Some(Scope::Identity))
            .await
            .unwrap(),
        Relationship::SelfPatient
    );
    assert_eq!(
        store::lookup(&db, &a, "carol", NOW, Some(Scope::Identity))
            .await
            .unwrap(),
        Relationship::None
    );
    // The same subject has nothing to the other patient.
    assert_eq!(
        store::lookup(&db, &b, "alice", NOW, Some(Scope::Identity))
            .await
            .unwrap(),
        Relationship::None
    );
    assert_eq!(
        store::lookup(&db, &a.to_uppercase(), "bob", NOW, Some(Scope::Identity))
            .await
            .unwrap(),
        Relationship::SelfPatient
    );
    assert_eq!(
        store::lookup(&db, "nonsense", "bob", NOW, Some(Scope::Identity))
            .await
            .unwrap(),
        Relationship::None
    );
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn ending_a_relationship_withdraws_access_and_keeps_the_history() {
    let db = db().await;
    let p = patient(&db).await;
    let v1 = store::create(
        &db,
        &p,
        "admin",
        Some("ID checked at desk"),
        &rec("dana", Kind::SelfPatient, Status::Active),
    )
    .await
    .unwrap();
    assert_eq!(
        store::lookup(&db, &p, "dana", NOW, Some(Scope::Identity))
            .await
            .unwrap(),
        Relationship::SelfPatient
    );

    // The subject and kind cannot be rewritten into someone else's access.
    assert!(matches!(
        store::update(
            &db,
            &p,
            &v1.id,
            1,
            "admin",
            None,
            &rec("eve", Kind::SelfPatient, Status::Active)
        )
        .await,
        Err(StoreError::BadQuery(_))
    ));
    assert!(matches!(
        store::update(
            &db,
            &p,
            &v1.id,
            1,
            "admin",
            None,
            &rec("dana", Kind::CareTeam, Status::Active)
        )
        .await,
        Err(StoreError::BadQuery(_))
    ));

    let v2 = store::update(
        &db,
        &p,
        &v1.id,
        1,
        "admin2",
        Some("left"),
        &rec("dana", Kind::SelfPatient, Status::Ended),
    )
    .await
    .unwrap();
    assert_eq!(v2.version, 2);
    assert_eq!(
        store::lookup(&db, &p, "dana", NOW, Some(Scope::Identity))
            .await
            .unwrap(),
        Relationship::None
    );
    assert_eq!(store::list(&db, &p).await.unwrap().len(), 1);
    let rows =
        sqlx::query("SELECT version FROM relationship_versions WHERE relationship_id = $1::uuid")
            .bind(v1.id.clone())
            .fetch_all(&db)
            .await
            .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(
        store::update(
            &db,
            &p,
            &v1.id,
            1,
            "x",
            None,
            &rec("dana", Kind::SelfPatient, Status::Ended)
        )
        .await,
        Err(StoreError::Conflict { current: 2 })
    );
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn needs_a_patient_and_cannot_be_edited_or_removed() {
    let db = db().await;
    assert_eq!(
        store::create(
            &db,
            "00000000-0000-4000-8000-000000000000",
            "a",
            None,
            &rec("x", Kind::Proxy, Status::Active)
        )
        .await,
        Err(StoreError::NotFound)
    );
    let p = patient(&db).await;
    let r = store::create(&db, &p, "a", None, &rec("x", Kind::Proxy, Status::Active))
        .await
        .unwrap();
    for sql in [
        format!(
            "UPDATE relationship_versions SET subject = 'y' WHERE relationship_id = '{}'",
            r.id
        ),
        format!(
            "DELETE FROM relationship_versions WHERE relationship_id = '{}'",
            r.id
        ),
        "TRUNCATE relationship_versions".to_string(),
    ] {
        let e = sqlx::raw_sql(AssertSqlSafe(sql.to_string()))
            .execute(&db)
            .await
            .unwrap_err()
            .to_string();
        assert!(e.contains("append-only"), "{sql}: {e}");
    }
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn own_patients_lists_self_and_proxy_in_force_and_nothing_else() {
    let db = db().await;
    let (mine, theirs, team, ended, both) = (
        patient(&db).await,
        patient(&db).await,
        patient(&db).await,
        patient(&db).await,
        patient(&db).await,
    );
    let me = format!("me-{mine}");
    for (p, kind, status) in [
        (&mine, Kind::SelfPatient, Status::Active),
        (&theirs, Kind::Proxy, Status::Active),
        (&team, Kind::CareTeam, Status::Active),
        (&ended, Kind::SelfPatient, Status::Ended),
        (&both, Kind::Proxy, Status::Active),
        (&both, Kind::SelfPatient, Status::Active),
    ] {
        store::create(&db, p, "admin", None, &rec(&me, kind, status))
            .await
            .unwrap();
    }
    // Someone else's link to `mine` must not appear for me.
    store::create(
        &db,
        &team,
        "admin",
        None,
        &rec("other", Kind::SelfPatient, Status::Active),
    )
    .await
    .unwrap();

    let got = store::own_patients(&db, &me, NOW).await.unwrap();
    let mut want = vec![
        (mine.clone(), Kind::SelfPatient),
        (theirs.clone(), Kind::Proxy),
        (both.clone(), Kind::SelfPatient),
    ];
    want.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(got, want);
    assert!(
        store::own_patients(&db, "nobody", NOW)
            .await
            .unwrap()
            .is_empty()
    );
}

/// Ending one of two would leave the other standing: a proxy whose access was ended would keep it.
#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn one_relationship_at_a_time_so_that_ending_it_ends_it() {
    let db = db().await;
    let p = patient(&db).await;
    let now = jiff::Timestamp::now().as_second();
    let proxy = |expires_at| RelationshipRecord {
        proxy: Some(Terms {
            basis: Basis::PatientRequest,
            scopes: [Scope::Identity].into(),
            expires_at,
        }),
        ..rec("erin", Kind::Proxy, Status::Active)
    };
    let year = 365 * 86_400;
    let first = store::create(&db, &p, "admin", None, &proxy(now + year / 2))
        .await
        .unwrap();
    for again in [proxy(now + year / 2), proxy(now + year / 4)] {
        match store::create(&db, &p, "admin", None, &again).await {
            Err(StoreError::BadQuery(m)) => {
                assert!(m.contains("already has") && !m.contains("erin"), "{m}");
            }
            other => panic!("{other:?}"),
        }
    }
    // Another kind, another person and another patient are not the same relationship.
    for (who, kind) in [("erin", Kind::CareTeam), ("frank", Kind::CareTeam)] {
        store::create(&db, &p, "admin", None, &rec(who, kind, Status::Active))
            .await
            .unwrap();
    }
    let q = patient(&db).await;
    store::create(&db, &q, "admin", None, &proxy(now + year / 2))
        .await
        .unwrap();
    // Ended: the access is gone, and a new one may be recorded.
    let ended = RelationshipRecord {
        status: Status::Ended,
        ..proxy(now + year / 2)
    };
    store::update(&db, &p, &first.id, 1, "admin", Some("asked"), &ended)
        .await
        .unwrap();
    assert_eq!(
        store::lookup(&db, &p, "erin", now, Some(Scope::Identity))
            .await
            .unwrap(),
        Relationship::CareTeam
    );
    let second = store::create(&db, &p, "admin", None, &proxy(now + year / 2))
        .await
        .unwrap();
    // The ended one cannot be brought back beside it.
    match store::update(
        &db,
        &p,
        &first.id,
        2,
        "admin",
        Some("back"),
        &proxy(now + year / 2),
    )
    .await
    {
        Err(StoreError::BadQuery(m)) => assert!(m.contains("already has"), "{m}"),
        other => panic!("{other:?}"),
    }
    // The standing one may change its own terms.
    store::update(
        &db,
        &p,
        &second.id,
        1,
        "admin",
        Some("longer"),
        &proxy(now + year / 3),
    )
    .await
    .unwrap();
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn one_that_is_over_or_does_not_overlap_is_not_in_the_way() {
    use electronic_patient_record::consent::Period;
    let db = db().await;
    let p = patient(&db).await;
    let on = |start: Option<&str>, end: Option<&str>| RelationshipRecord {
        period: Some(Period {
            start: start.map(String::from),
            end: end.map(String::from),
        }),
        ..rec("gina", Kind::ActiveEncounter, Status::Active)
    };
    let make = async |r: RelationshipRecord| store::create(&db, &p, "admin", None, &r).await;
    // Over, though nobody ended it.
    make(on(Some("2001-01-01"), Some("2001-01-31")))
        .await
        .unwrap();
    make(on(Some("2090-01-01"), Some("2090-01-31")))
        .await
        .unwrap();
    make(on(Some("2090-02-01"), None)).await.unwrap();
    make(on(None, Some("2089-12-31"))).await.unwrap();
    for (start, end) in [
        (Some("2090-01-31"), Some("2090-01-31")),
        (Some("2091-01-01"), None),
        (None, None),
        (None, Some("2090-01-01")),
    ] {
        match make(on(start, end)).await {
            Err(StoreError::BadQuery(m)) => assert!(m.contains("already has"), "{m}"),
            other => panic!("{start:?} {end:?}: {other:?}"),
        }
    }
    // A proxy whose terms have run out is over too.
    let now = jiff::Timestamp::now().as_second();
    let proxy = |expires_at| RelationshipRecord {
        proxy: Some(Terms {
            basis: Basis::PatientRequest,
            scopes: [Scope::Identity].into(),
            expires_at,
        }),
        ..rec("hugo", Kind::Proxy, Status::Active)
    };
    make(proxy(now - 60)).await.unwrap();
    make(proxy(now + 86_400)).await.unwrap();
}

/// Two administrators at one moment.
#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn two_people_at_one_moment_record_one_relationship() {
    let db = db().await;
    let p = patient(&db).await;
    let r = rec("iris", Kind::CareTeam, Status::Active);
    let mut first = db.begin().await.unwrap();
    store::create(&mut *first, &p, "admin-1", None, &r)
        .await
        .unwrap();
    let (other, at, same) = (db.clone(), p.clone(), r.clone());
    let second =
        tokio::spawn(async move { store::create(&other, &at, "admin-2", None, &same).await });
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert!(!second.is_finished(), "it waits for the first");
    first.commit().await.unwrap();
    assert!(matches!(
        second.await.unwrap(),
        Err(StoreError::BadQuery(_))
    ));
}
