//! DB-gated (EH-8): what the store does with a labelled entry, below the routes: what a viewer's list, rules and writes
//! are made of, that a label follows the entry through a merge, and that its rows cannot be changed. Who may label is
//! `labelling.rs`; through the router is `http_entry_labels.rs`.
//!
//! ```text
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test main entry_labels_store:: -- --ignored --test-threads=1
//! ```

use electronic_patient_record::entries_store::{
    self as store, Change, LabelChange, LabelError, Viewer,
};
use electronic_patient_record::labelling::{Actor, Basis, Refusal};
use electronic_patient_record::links;
use electronic_patient_record::masking::Completeness;
use electronic_patient_record::patients_store::{self, StoreError};
use electronic_patient_record::policy::{Decision, DenyReason, Label, Relationship, Role};
use electronic_patient_record::problem::{Certainty, ProblemRecord, Status};
use electronic_patient_record::typing::TypingRecord;
use electronic_patient_record_matcher::{HumanName, Patient};
use serde_json::json;
use sqlx::{AssertSqlSafe, PgPool};
use std::collections::BTreeSet;

async fn db() -> PgPool {
    let url = std::env::var("DB_URL").expect("DB_URL must be set for --ignored tests");
    let db = PgPool::connect(&url).await.expect("connect");
    electronic_patient_record::MIGRATOR
        .run(&db)
        .await
        .expect("migrate");
    db
}

async fn patient(db: &PgPool, family: &str) -> String {
    let p = Patient {
        names: vec![HumanName {
            family: Some(family.into()),
            given: vec!["La".into()],
        }],
        birth_date: Some("1986-02-02".into()),
        ..Default::default()
    };
    patients_store::create(db, "t", None, &p).await.unwrap().id
}

fn family() -> String {
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("Zzlabel{n}")
}

/// Sees what has no label, and nothing that has one.
fn plain(told: bool) -> Viewer<'static> {
    Viewer::new(
        |labels: BTreeSet<Label>| {
            if labels.is_empty() {
                Decision::Allow(Vec::new())
            } else {
                Decision::Deny {
                    reason: DenyReason::NotPermitted,
                    btg_offer: false,
                }
            }
        },
        told,
    )
}

fn all() -> Viewer<'static> {
    Viewer::everything()
}

fn doctor(subject: &str) -> Actor {
    Actor::Person {
        subject: subject.into(),
        roles: BTreeSet::from([Role::Clinician]),
        relationship: Relationship::CareTeam,
    }
}

fn add(label: &str) -> LabelChange {
    LabelChange::Add {
        label: label.into(),
        basis: Basis::Clinical,
    }
}

fn problem(title: &str, due_to: Option<&str>) -> ProblemRecord {
    ProblemRecord {
        title: title.into(),
        coding: None,
        certainty: Certainty::Confirmed,
        status: Status::Active,
        severity: None,
        body_site: None,
        onset: None,
        abated: None,
        due_to: due_to.map(String::from),
    }
}

async fn recorded(db: &PgPool, p: &str, record: &ProblemRecord) -> String {
    store::create(db, p, "dr-1", None, record, &all())
        .await
        .unwrap()
        .id
}

async fn label(db: &PgPool, p: &str, entry: &str, code: &str) {
    let mut tx = db.begin().await.unwrap();
    store::relabel::<ProblemRecord>(&mut *tx, p, entry, &doctor("dr-1"), &add(code), &all())
        .await
        .unwrap();
    tx.commit().await.unwrap();
}

fn change<'a>(v: i32, record: &'a ProblemRecord) -> Change<'a, ProblemRecord> {
    Change {
        expected_version: v,
        author: "n-1",
        reason: Some("reviewed"),
        record,
    }
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_labelled_entry_is_not_there_for_someone_who_may_not_see_it() {
    let db = db().await;
    let p = patient(&db, &family()).await;
    let hidden = recorded(&db, &p, &problem("Secret", None)).await;
    let effect = recorded(&db, &p, &problem("Rash", Some(&hidden))).await;
    label(&db, &p, &hidden, "V").await;
    let viewer = plain(true);

    let view = store::list::<ProblemRecord>(&db, &p, &viewer)
        .await
        .unwrap();
    assert_eq!(view.entries.len(), 1);
    assert_eq!(
        view.entries[0].record.due_to, None,
        "it does not point at it"
    );
    assert_eq!(view.completeness, Completeness::Partial { withheld: 1 });
    let summary = view.summary();
    assert_eq!(summary["partial"], true);
    assert_eq!(summary["state"], "active", "what is shown is still true");
    assert_eq!(summary["causes"]["cause_not_on_list"], json!([]));
    // Not told: the same whether or not anything is withheld.
    let untold = store::list::<ProblemRecord>(&db, &p, &plain(false))
        .await
        .unwrap();
    assert_eq!(untold.completeness, Completeness::Unstated);
    // As at a time **before** the label, by today's label.
    let now = jiff::Timestamp::now();
    let then = store::list_at::<ProblemRecord>(&db, &p, now, &viewer)
        .await
        .unwrap();
    assert_eq!(then.entries.len(), 1);
    assert_eq!(then.entries[0].stored.record.due_to, None);
    assert_eq!(then.summary()["partial"], true);
    // Its history, its labels and a write to it: as for an id that is not there.
    let missing = "00000000-0000-4000-8000-000000000000";
    for id in [hidden.as_str(), missing] {
        assert!(matches!(
            store::history::<ProblemRecord>(&db, &p, id, &viewer).await,
            Err(StoreError::NotFound)
        ));
        assert!(matches!(
            store::label_history::<ProblemRecord>(&db, &p, id, &viewer).await,
            Err(StoreError::NotFound)
        ));
        // With a stale version too: the version is not told.
        for v in [1, 9] {
            assert!(matches!(
                store::update(&db, &p, id, change(v, &problem("x", None)), &viewer).await,
                Err(StoreError::NotFound)
            ));
        }
        let mut tx = db.begin().await.unwrap();
        assert!(matches!(
            store::relabel::<ProblemRecord>(&mut *tx, &p, id, &doctor("b"), &add("V"), &viewer)
                .await,
            Err(LabelError::Store(StoreError::NotFound))
        ));
    }
    // Naming it as a cause is refused in the same words.
    let mut said = Vec::new();
    for id in [hidden.as_str(), missing] {
        match store::create(&db, &p, "n-1", None, &problem("X", Some(id)), &viewer).await {
            Err(StoreError::BadQuery(m)) => said.push(m),
            other => panic!("{other:?}"),
        }
    }
    assert_eq!(said[0], said[1]);
    // The history of what points at it does not either.
    for v in store::history::<ProblemRecord>(&db, &p, &effect, &viewer)
        .await
        .unwrap()
    {
        assert_eq!(v.record.due_to, None);
    }
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_writer_who_could_not_see_a_cause_keeps_it_and_may_retract_what_a_hidden_entry_names() {
    let db = db().await;
    let p = patient(&db, &family()).await;
    let hidden = recorded(&db, &p, &problem("Secret", None)).await;
    let effect = recorded(&db, &p, &problem("Rash", Some(&hidden))).await;
    label(&db, &p, &hidden, "V").await;
    let viewer = plain(true);
    let answer = store::update(
        &db,
        &p,
        &effect,
        change(1, &problem("Rash, worse", None)),
        &viewer,
    )
    .await
    .unwrap();
    assert_eq!(answer.record.due_to, None, "the answer does not tell");
    let whole = store::list::<ProblemRecord>(&db, &p, &all()).await.unwrap();
    let kept = whole.entries.iter().find(|s| s.id == effect).unwrap();
    assert_eq!(kept.record.due_to.as_deref(), Some(hidden.as_str()));
    assert_eq!(kept.record.title, "Rash, worse");

    // A hidden effect does not stop a retraction, and is then listed for those who see it.
    let cause = recorded(&db, &p, &problem("Cause", None)).await;
    let secret = recorded(&db, &p, &problem("Secret effect", Some(&cause))).await;
    label(&db, &p, &secret, "V").await;
    let retracted = ProblemRecord {
        status: Status::EnteredInError,
        ..problem("Cause", None)
    };
    assert!(matches!(
        store::update(&db, &p, &cause, change(1, &retracted), &all()).await,
        Err(StoreError::BadQuery(_))
    ));
    store::update(&db, &p, &cause, change(1, &retracted), &viewer)
        .await
        .unwrap();
    let whole = store::list::<ProblemRecord>(&db, &p, &all()).await.unwrap();
    assert_eq!(
        whole.summary()["causes"]["cause_not_on_list"],
        json!([secret])
    );
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn nothing_is_claimed_that_a_withheld_entry_could_change() {
    let db = db().await;
    let p = patient(&db, &family()).await;
    let abo = |sample: &str, at: &str, group: &str| -> TypingRecord {
        serde_json::from_value(
            json!({ "finding": { "kind": "abo", "group": group }, "laboratory": "Lab",
            "sample": sample, "collected_at": at, "status": "reported" }),
        )
        .unwrap()
    };
    let mut ids = Vec::new();
    for (sample, at, group) in [
        ("S1", "2026-11-20T09:00:00Z", "o"),
        ("S2", "2026-11-21T09:00:00Z", "o"),
        ("S3", "2026-11-22T09:00:00Z", "a"),
    ] {
        ids.push(
            store::create(&db, &p, "lab", None, &abo(sample, at, group), &all())
                .await
                .unwrap()
                .id,
        );
    }
    let seen = async || {
        store::list::<TypingRecord>(&db, &p, &plain(true))
            .await
            .unwrap()
            .summary()
    };
    assert_eq!(seen().await["abo"], "disagree");
    let mut tx = db.begin().await.unwrap();
    store::relabel::<TypingRecord>(&mut *tx, &p, &ids[2], &doctor("a"), &add("R"), &all())
        .await
        .unwrap();
    tx.commit().await.unwrap();
    // The two left agree, and the one withheld does not: nothing is said.
    assert_eq!(seen().await["abo"], "not-stated");
    assert_eq!(seen().await["rhd"], "not-stated");
    for id in &ids[..2] {
        let mut tx = db.begin().await.unwrap();
        store::relabel::<TypingRecord>(&mut *tx, &p, id, &doctor("a"), &add("R"), &all())
            .await
            .unwrap();
        tx.commit().await.unwrap();
    }
    assert_eq!(
        seen().await["state"],
        "not-stated",
        "an empty rest is not none"
    );
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_label_follows_its_entry_through_a_merge_and_an_unmerge() {
    let db = db().await;
    let f = family();
    let (kept, gone) = (patient(&db, &f).await, patient(&db, &f).await);
    let entry = recorded(&db, &gone, &problem("Secret", None)).await;
    label(&db, &gone, &entry, "V").await;
    links::merge(&db, &kept, &gone, "admin", "same person")
        .await
        .unwrap();
    let on = async |p: &str| {
        let v = store::list::<ProblemRecord>(&db, p, &plain(true))
            .await
            .unwrap();
        (v.entries.len(), v.completeness)
    };
    assert_eq!(on(&kept).await, (0, Completeness::Partial { withheld: 1 }));
    // Lifted on the surviving record, it is lifted on its own after an unmerge.
    let mut tx = db.begin().await.unwrap();
    let lift = LabelChange::Lift {
        label: "V".into(),
        basis: Basis::PatientAsked,
        reason: "asked".into(),
    };
    store::relabel::<ProblemRecord>(&mut *tx, &kept, &entry, &doctor("dr-2"), &lift, &all())
        .await
        .unwrap();
    tx.commit().await.unwrap();
    links::unmerge(&db, &kept, &gone, "admin", "different people")
        .await
        .unwrap();
    assert_eq!(on(&gone).await, (1, Completeness::Whole));
    assert_eq!(on(&kept).await, (0, Completeness::Whole));
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_code_this_version_does_not_know_withholds_from_everyone() {
    let db = db().await;
    let p = patient(&db, &family()).await;
    let entry = recorded(&db, &p, &problem("Secret", None)).await;
    sqlx::raw_sql(AssertSqlSafe(format!(
        "INSERT INTO entry_labels (kind, entry_id, seq, patient_id, act, label, author, basis)
         VALUES ('problem', '{entry}', 1, '{p}', 'added', 'SDV', 'dr-1', 'clinical')"
    )))
    .execute(&db)
    .await
    .unwrap();
    let view = store::list::<ProblemRecord>(&db, &p, &all()).await.unwrap();
    assert_eq!(view.completeness, Completeness::Partial { withheld: 1 });
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn two_people_adding_one_label_at_one_moment_add_it_once() {
    let db = db().await;
    let p = patient(&db, &family()).await;
    let entry = recorded(&db, &p, &problem("Secret", None)).await;
    let mut first = db.begin().await.unwrap();
    store::relabel::<ProblemRecord>(&mut *first, &p, &entry, &doctor("a"), &add("V"), &all())
        .await
        .unwrap();
    let second = {
        let (db, p, entry) = (db.clone(), p.clone(), entry.clone());
        tokio::spawn(async move {
            let mut tx = db.begin().await.unwrap();
            store::relabel::<ProblemRecord>(&mut *tx, &p, &entry, &doctor("b"), &add("V"), &all())
                .await
        })
    };
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert!(!second.is_finished(), "it waits for the first");
    first.commit().await.unwrap();
    assert!(matches!(
        second.await.unwrap(),
        Err(LabelError::Refused(Refusal::AlreadyThere))
    ));
    let acts = store::label_history::<ProblemRecord>(&db, &p, &entry, &all())
        .await
        .unwrap();
    assert_eq!(acts.len(), 1);
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn the_database_refuses_to_change_or_remove_what_happened_to_a_label() {
    let db = db().await;
    let p = patient(&db, &family()).await;
    let entry = recorded(&db, &p, &problem("Secret", None)).await;
    label(&db, &p, &entry, "V").await;
    for sql in [
        format!("UPDATE entry_labels SET label = 'R' WHERE entry_id = '{entry}'"),
        format!("DELETE FROM entry_labels WHERE entry_id = '{entry}'"),
        "TRUNCATE entry_labels".to_string(),
    ] {
        let e = sqlx::raw_sql(AssertSqlSafe(sql.clone()))
            .execute(&db)
            .await
            .unwrap_err();
        assert!(e.to_string().contains("append-only"), "{sql}: {e}");
    }
}
