//! DB-gated (EH-1, EH-2, EH-3): the problem list and relative history are append-only versions, a cause stays sound, and
//! a read as at a time gives what the record held then, following the merges that stood then.
//!
//! ```text
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test main entries_store:: -- --ignored --test-threads=1
//! ```

use electronic_patient_record::allergies::{self, AllergyRecord, Summary};
use electronic_patient_record::clock;
use electronic_patient_record::entries_store::{self as store, Change, Entry, Stored};
use electronic_patient_record::links;
use electronic_patient_record::patients_store::{self, StoreError};
use electronic_patient_record::problem::{Certainty, ProblemRecord, Status};
use electronic_patient_record::relative_history::RelativeHistoryRecord;
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

fn nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

async fn patient(db: &PgPool) -> String {
    let p = Patient {
        names: vec![HumanName {
            family: Some(format!("Zzentry{}", nanos())),
            given: vec!["En".into()],
        }],
        birth_date: Some("1985-01-01".into()),
        ..Default::default()
    };
    patients_store::create(db, "t", None, &p).await.unwrap().id
}

/// Someone from whom nothing is withheld: what a label withholds is `entry_labels_store.rs`.
fn all() -> store::Viewer<'static> {
    store::Viewer::everything()
}

fn problem(title: &str, status: Status) -> ProblemRecord {
    ProblemRecord {
        title: title.into(),
        coding: None,
        certainty: Certainty::Confirmed,
        status,
        severity: None,
        body_site: None,
        onset: Some("2019-04".into()),
        abated: None,
        due_to: None,
    }
}

fn caused(title: &str, by: &str) -> ProblemRecord {
    ProblemRecord {
        due_to: Some(by.into()),
        ..problem(title, Status::Active)
    }
}

fn change<'a>(version: i32, record: &'a ProblemRecord) -> Change<'a, ProblemRecord> {
    Change {
        expected_version: version,
        author: "dr-2",
        reason: Some("seen in clinic"),
        record,
    }
}

async fn add(db: &PgPool, p: &str, record: &ProblemRecord) -> Stored<ProblemRecord> {
    store::create(db, p, "dr-1", None, record, &all())
        .await
        .unwrap()
}

fn refused<T: std::fmt::Debug>(result: Result<T, StoreError>, says: &str) {
    match result {
        Err(StoreError::BadQuery(m)) => assert!(m.contains(says), "{m}"),
        other => panic!("expected a refusal saying {says:?}, got {other:?}"),
    }
}

/// The time a version was written, as a timestamp.
fn when<R>(s: &Stored<R>) -> jiff::Timestamp {
    clock::parse_rfc3339(&s.recorded_at).unwrap()
}

fn just_before(t: jiff::Timestamp) -> jiff::Timestamp {
    t - jiff::SignedDuration::from_micros(1)
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_problem_is_versioned_with_a_reason_and_a_mistake_is_final() {
    let db = db().await;
    let p = patient(&db).await;
    let v1 = add(&db, &p, &problem("Asthma", Status::Active)).await;
    assert_eq!((v1.version, v1.reason.clone()), (1, None));

    // A change needs a reason, and the version it was made from.
    let resolved = ProblemRecord {
        abated: Some("2025".into()),
        ..problem("Asthma", Status::Resolved)
    };
    refused(
        store::update(
            &db,
            &p,
            &v1.id,
            Change {
                reason: None,
                ..change(1, &resolved)
            },
            &all(),
        )
        .await,
        "a reason is required",
    );
    assert!(matches!(
        store::update(&db, &p, &v1.id, change(7, &resolved), &all()).await,
        Err(StoreError::Conflict { current: 1 })
    ));
    let v2 = store::update(&db, &p, &v1.id, change(1, &resolved), &all())
        .await
        .unwrap();
    assert_eq!((v2.version, v2.author.as_str()), (2, "dr-2"));
    assert!(when(&v2) > when(&v1), "a later version has a later time");

    let history = store::history::<ProblemRecord>(&db, &p, &v1.id, &all())
        .await
        .unwrap();
    assert_eq!(
        history.iter().map(|v| v.version).collect::<Vec<_>>(),
        [1, 2],
        "oldest first"
    );
    assert_eq!(
        history[0].record.status,
        Status::Active,
        "nothing is edited"
    );

    let mistake = problem("Asthma", Status::EnteredInError);
    refused(
        store::create(&db, &p, "dr-1", None, &mistake, &all()).await,
        "cannot start as entered in error",
    );
    store::update(&db, &p, &v1.id, change(2, &mistake), &all())
        .await
        .unwrap();
    refused(
        store::update(&db, &p, &v1.id, change(3, &resolved), &all()).await,
        "is final",
    );
    refused(
        store::update(&db, &p, &v1.id, change(3, &mistake), &all()).await,
        "is final",
    );
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn the_database_refuses_to_change_or_remove_a_version() {
    let db = db().await;
    let p = patient(&db).await;
    add(&db, &p, &problem("Asthma", Status::Active)).await;
    let entry: RelativeHistoryRecord = serde_json::from_value(serde_json::json!({
        "genetic": "full-sibling", "environment": "childhood", "member": 1, "condition": "Asthma", "presence": "present",
        "source": "patient", "status": "recorded"
    }))
    .unwrap();
    store::create(&db, &p, "dr-1", None, &entry, &all())
        .await
        .unwrap();
    for table in ["problem_versions", "relative_history_versions"] {
        for sql in [
            format!("UPDATE {table} SET author = 'x' WHERE patient_id = '{p}'"),
            format!("DELETE FROM {table} WHERE patient_id = '{p}'"),
            format!("TRUNCATE {table}"),
        ] {
            let e = sqlx::raw_sql(AssertSqlSafe(sql.clone()))
                .execute(&db)
                .await
                .expect_err(&sql);
            assert!(e.to_string().contains("append-only"), "{sql}: {e}");
        }
    }
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_cause_is_on_the_same_list_never_loops_and_cannot_be_retracted_from_under_its_effects() {
    let db = db().await;
    let p = patient(&db).await;
    let other = patient(&db).await;
    let syndrome = add(&db, &p, &problem("Syndrome", Status::Active)).await;
    let lung = add(&db, &p, &caused("Lung disease", &syndrome.id)).await;
    let thyroid = add(&db, &p, &caused("Thyroid disease", &syndrome.id)).await;
    let elsewhere = add(&db, &other, &problem("Asthma", Status::Active)).await;

    refused(
        store::create(&db, &p, "dr-1", None, &caused("X", &elsewhere.id), &all()).await,
        "no problem on this patient's list",
    );
    refused(
        store::update(
            &db,
            &p,
            &syndrome.id,
            change(1, &caused("Syndrome", &lung.id)),
            &all(),
        )
        .await,
        "through others, its own cause",
    );
    refused(
        store::update(
            &db,
            &p,
            &syndrome.id,
            change(1, &caused("Syndrome", &syndrome.id.to_uppercase())),
            &all(),
        )
        .await,
        "its own cause",
    );
    // Two problems name it, so it cannot be retracted until they do not.
    let mistake = problem("Syndrome", Status::EnteredInError);
    refused(
        store::update(&db, &p, &syndrome.id, change(1, &mistake), &all()).await,
        "2 problem(s) name this as their cause",
    );
    for effect in [&lung, &thyroid] {
        let alone = problem(&effect.record.title, Status::Active);
        store::update(&db, &p, &effect.id, change(1, &alone), &all())
            .await
            .unwrap();
    }
    store::update(&db, &p, &syndrome.id, change(1, &mistake), &all())
        .await
        .unwrap();
    refused(
        store::create(&db, &p, "dr-1", None, &caused("Y", &syndrome.id), &all()).await,
        "entered in error",
    );
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn two_writers_cannot_together_make_a_loop() {
    let db = db().await;
    for _ in 0..8 {
        let p = patient(&db).await;
        let a = add(&db, &p, &problem("A", Status::Active)).await;
        let b = add(&db, &p, &problem("B", Status::Active)).await;
        let (a_by_b, b_by_a) = (caused("A", &b.id), caused("B", &a.id));
        let write = |id: String, record: ProblemRecord| {
            let (db, p) = (db.clone(), p.clone());
            async move {
                let mut tx = db.begin().await.unwrap();
                let r = store::update(&mut *tx, &p, &id, change(1, &record), &all()).await;
                match r {
                    Ok(_) => tx.commit().await.is_ok(),
                    Err(StoreError::BadQuery(_)) => false,
                    Err(e) => panic!("{e}"),
                }
            }
        };
        let (x, y) = tokio::join!(write(a.id.clone(), a_by_b), write(b.id.clone(), b_by_a));
        assert!(x ^ y, "exactly one of the two is accepted: {x} {y}");
    }
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_list_as_at_a_time_is_what_the_record_held_then() {
    let db = db().await;
    let p = patient(&db).await;
    let v1 = add(&db, &p, &problem("Asthma", Status::Active)).await;
    let later = add(&db, &p, &problem("Gout", Status::Active)).await;
    let v2 = store::update(
        &db,
        &p,
        &v1.id,
        change(1, &problem("Asthma", Status::EnteredInError)),
        &all(),
    )
    .await
    .unwrap();

    let at = async |t| {
        store::list_at::<ProblemRecord>(&db, &p, t, &all())
            .await
            .unwrap()
            .entries
            .into_iter()
            .map(|s| (s.stored.record.title, s.stored.version, s.changed_since))
            .collect::<Vec<_>>()
    };
    assert!(
        at(just_before(when(&v1))).await.is_empty(),
        "before anything"
    );
    // To the microsecond it was written, and one before the next write.
    for t in [when(&v1), just_before(when(&later))] {
        assert_eq!(at(t).await, [("Asthma".into(), 1, true)]);
    }
    // Retracted later, so it still shows as it was, and says it has changed since.
    assert_eq!(
        at(just_before(when(&v2))).await,
        [("Asthma".into(), 1, true), ("Gout".into(), 1, false)]
    );
    let now = at(jiff::Timestamp::now()).await;
    assert_eq!(now.len(), 2);
    assert!(now.contains(&("Asthma".into(), 2, false)), "{now:?}");
    assert!(matches!(
        store::list_at::<ProblemRecord>(
            &db,
            "00000000-0000-4000-8000-000000000000",
            jiff::Timestamp::now(),
            &all()
        )
        .await,
        Err(StoreError::NotFound)
    ));
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn as_at_a_time_follows_the_merges_that_stood_then() {
    let db = db().await;
    let same = Patient {
        names: vec![HumanName {
            family: Some(format!("Zzmerge{}", nanos())),
            given: vec!["Em".into()],
        }],
        birth_date: Some("1971-02-03".into()),
        ..Default::default()
    };
    let mut ids = Vec::new();
    for _ in 0..2 {
        ids.push(
            patients_store::create(&db, "t", None, &same)
                .await
                .unwrap()
                .id,
        );
    }
    let (kept, gone) = (ids[0].clone(), ids[1].clone());
    add(&db, &kept, &problem("On the survivor", Status::Active)).await;
    add(&db, &gone, &problem("On the other", Status::Active)).await;
    let merge = links::merge(&db, &kept, &gone, "admin", "same person")
        .await
        .unwrap();
    let merged_at = clock::parse_rfc3339(&merge.recorded_at).unwrap();

    let titles = async |t| {
        store::list_at::<ProblemRecord>(&db, &kept, t, &all())
            .await
            .unwrap()
            .entries
            .into_iter()
            .map(|s| s.stored.record.title)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        titles(just_before(merged_at)).await,
        ["On the survivor"],
        "not merged yet"
    );
    let during = jiff::Timestamp::now();
    assert_eq!(
        titles(during).await,
        ["On the survivor", "On the other"],
        "merged"
    );
    links::unmerge(&db, &kept, &gone, "admin", "different people")
        .await
        .unwrap();
    assert_eq!(
        titles(jiff::Timestamp::now()).await,
        ["On the survivor"],
        "unmerged"
    );
    assert_eq!(
        titles(during).await,
        ["On the survivor", "On the other"],
        "what a reader saw while they were merged"
    );
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_cause_left_on_the_other_record_by_an_unmerge_is_said_and_a_refuted_one_too() {
    let db = db().await;
    let (kept, gone) = (patient(&db).await, patient(&db).await);
    let causes = async |p: &str| {
        let all = store::list::<ProblemRecord>(&db, p, &all())
            .await
            .unwrap()
            .entries;
        ProblemRecord::summary(&all.iter().collect::<Vec<_>>())["causes"].clone()
    };
    let syndrome = add(&db, &gone, &problem("Syndrome", Status::Active)).await;
    links::merge(&db, &kept, &gone, "admin", "same person")
        .await
        .unwrap();
    let lung = add(&db, &kept, &caused("Lung disease", &syndrome.id)).await;
    let joined = serde_json::json!({
        "groups": [{"cause": syndrome.id, "effects": [lung.id]}],
        "cause_not_on_list": [], "cause_refuted": []
    });
    assert_eq!(causes(&kept).await, joined);

    let refuted = ProblemRecord {
        certainty: Certainty::Refuted,
        ..problem("Syndrome", Status::Inactive)
    };
    store::update(&db, &kept, &syndrome.id, change(1, &refuted), &all())
        .await
        .unwrap();
    assert_eq!(
        causes(&kept).await["cause_refuted"],
        serde_json::json!([lung.id])
    );

    links::unmerge(&db, &kept, &gone, "admin", "different people")
        .await
        .unwrap();
    assert_eq!(
        causes(&kept).await,
        serde_json::json!({"groups": [], "cause_not_on_list": [lung.id], "cause_refuted": []})
    );
    assert_eq!(causes(&gone).await["groups"], serde_json::json!([]));
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn the_allergies_as_at_a_time_keep_their_statement_and_banner() {
    let db = db().await;
    let p = patient(&db).await;
    let start = jiff::Timestamp::now();
    let said = allergies::assert_none_known(&db, &p, 0, "nurse-1", None, true)
        .await
        .unwrap();
    let said_at = clock::parse_rfc3339(&said.recorded_at).unwrap();
    let record = AllergyRecord {
        substance: "Penicillin".into(),
        coding: None,
        category: None,
        criticality: None,
        reaction: None,
        onset: None,
        status: allergies::Status::Active,
    };
    let recorded = allergies::create(&db, &p, "nurse-1", None, &record)
        .await
        .unwrap();
    let recorded_at = clock::parse_rfc3339(&recorded.recorded_at).unwrap();
    let state = async |t| {
        let all = allergies::list_at(&db, &p, t).await.unwrap();
        let then: Vec<_> = all.iter().map(|a| a.stored.clone()).collect();
        let statement = allergies::statement_at(&db, &p, t).await.unwrap();
        allergies::summarise(&then, statement.as_ref())
    };
    assert_eq!(
        state(just_before(said_at).min(start)).await,
        Summary::NotRecorded
    );
    assert!(matches!(
        state(just_before(recorded_at)).await,
        Summary::NoKnown { .. }
    ));
    assert!(matches!(state(recorded_at).await, Summary::Active { .. }));
    assert!(!allergies::list_at(&db, &p, recorded_at).await.unwrap()[0].changed_since);
    let ended = AllergyRecord {
        status: allergies::Status::Inactive,
        ..record
    };
    allergies::update(&db, &p, &recorded.id, 1, "dr-1", None, &ended)
        .await
        .unwrap();
    let then = allergies::list_at(&db, &p, recorded_at).await.unwrap();
    assert!(then[0].changed_since);
    assert_eq!(then[0].stored.allergy.status, allergies::Status::Active);
}

/// AG-7. A limit, held so that nobody mistakes it for a promise: a version's time is when it was **written**, and nobody
/// else could see it until its transaction committed. An as-at answer for a moment in between includes a version that no
/// reader had. The answer says so (`left_out`), and an allergy's window is wider, from when its transaction began.
#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_write_in_progress_at_the_time_is_included_though_nobody_could_see_it() {
    let db = db().await;
    let p = patient(&db).await;
    let kept = add(&db, &p, &problem("Asthma", Status::Active)).await;

    let mut tx = db.begin().await.unwrap();
    // The transaction has begun and nothing is written: the allergy will carry this time and the problem will not.
    sqlx::query("SELECT now()").execute(&mut *tx).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(40)).await;
    let before_any_write = jiff::Timestamp::now();
    tokio::time::sleep(std::time::Duration::from_millis(40)).await;
    let correction = problem("Asthma, corrected", Status::Active);
    store::update(&mut *tx, &p, &kept.id, change(1, &correction), &all())
        .await
        .unwrap();
    let record = AllergyRecord {
        substance: "Latex".into(),
        coding: None,
        category: None,
        criticality: None,
        reaction: None,
        onset: None,
        status: allergies::Status::Active,
    };
    allergies::create(&mut *tx, &p, "nurse-1", None, &record)
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(40)).await;
    let written_not_committed = jiff::Timestamp::now();
    // What a reader saw at that moment: the old problem, and no allergy.
    let seen = store::list::<ProblemRecord>(&db, &p, &all())
        .await
        .unwrap()
        .entries;
    assert_eq!(seen[0].record.title, "Asthma");
    assert!(allergies::list(&db, &p).await.unwrap().is_empty());
    tx.commit().await.unwrap();

    let title = async |t| {
        store::list_at::<ProblemRecord>(&db, &p, t, &all())
            .await
            .unwrap()
            .entries[0]
            .stored
            .record
            .title
            .clone()
    };
    let allergy_count = async |t| allergies::list_at(&db, &p, t).await.unwrap().len();
    assert_eq!(
        title(before_any_write).await,
        "Asthma",
        "before it was written"
    );
    assert_eq!(
        allergy_count(before_any_write).await,
        1,
        "the allergy's time is when its transaction began"
    );
    assert_eq!(
        title(written_not_committed).await,
        "Asthma, corrected",
        "stored, and seen by nobody"
    );
    assert_eq!(allergy_count(written_not_committed).await, 1);
}
