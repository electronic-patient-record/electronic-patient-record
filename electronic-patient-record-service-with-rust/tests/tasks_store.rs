//! Tasks from rules (TL-3, TL-4, TL-14) against Postgres: only an approved rule makes a task, with an audit row, in
//! the caller's transaction; a rolled-back write leaves no task; an event cancels only its own source's tasks; a person
//! closes a task with a reason at the version read; the tables refuse change; each reminder is decided once, and a
//! late one follows its rule.

use electronic_patient_record::patients_store::{self, StoreError};
use electronic_patient_record::task_rules::{self, Event, ReminderNow};
use electronic_patient_record::tasks_store::{self as store, Source, Status};
use electronic_patient_record_matcher::{HumanName, Patient};
use jiff::civil::Date;
use sqlx::{Connection, PgPool, Row};
use std::collections::{BTreeMap, BTreeSet};

async fn db() -> PgPool {
    let url = std::env::var("DB_URL").expect("DB_URL must be set for --ignored tests");
    let db = PgPool::connect(&url).await.expect("connect");
    electronic_patient_record::MIGRATOR
        .run(&db)
        .await
        .expect("migrate");
    db
}

fn stamp() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
        .to_string()
}

async fn patient(db: &PgPool) -> String {
    let family = format!("task{}", stamp());
    patients_store::create(
        db,
        "dr-1",
        Some("task store test"),
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

/// Two rules on an abnormal result: one approved, for a team named for this run so lists do not mix; one a draft.
fn rules(team: &str) -> Vec<task_rules::Rule> {
    let text = format!(
        r#"
[[rule]]
id = "approved-review"
version = 1
title = "Approved"
approved_by = "A Clinician"
approved_on = "2026-11-17"
event = "result-filed"
cancel_on = ["result-signed-off"]
[[rule.when]]
fact = "abnormal"
any_of = ["true"]
[rule.task]
team = "{team}"
title = "Review the abnormal result"
due_working_days = 2
remind_working_days_before = [1]
late_reminder = "skip"

[[rule]]
id = "draft-review"
version = 1
title = "Draft"
event = "result-filed"
[rule.task]
team = "{team}"
title = "A draft would also ask"
due_working_days = 1
late_reminder = "send-once"
"#
    );
    task_rules::parse(&text, "test").unwrap()
}

fn d(s: &str) -> Date {
    s.parse().unwrap()
}

fn abnormal() -> BTreeMap<String, String> {
    [("abnormal".to_string(), "true".to_string())]
        .into_iter()
        .collect()
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn only_an_approved_rule_makes_a_task_with_an_audit_row_and_a_rollback_leaves_none() {
    let db = db().await;
    let team = format!("team-{}", stamp());
    let rules = rules(&team);
    let p = patient(&db).await;
    let source = Source {
        kind: "result".into(),
        id: format!("r-{}", stamp()),
    };
    let none = BTreeSet::new();

    // TL-14: the write and its tasks are one transaction; rolled back, nothing is left.
    let mut conn = db.acquire().await.unwrap();
    let mut tx = conn.begin().await.unwrap();
    let made = store::create_from_event(
        &mut tx,
        &rules,
        Event::ResultFiled,
        &abnormal(),
        &p,
        &source,
        d("2026-11-20"),
        &none,
    )
    .await
    .unwrap();
    assert_eq!(made.len(), 1, "the draft makes nothing");
    tx.rollback().await.unwrap();
    assert!(
        store::for_team(&db, &team).await.unwrap().0.is_empty(),
        "rolled back with its write"
    );

    let mut tx = conn.begin().await.unwrap();
    let made = store::create_from_event(
        &mut tx,
        &rules,
        Event::ResultFiled,
        &abnormal(),
        &p,
        &source,
        d("2026-11-20"),
        &none,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let task = &made[0];
    assert_eq!(
        (task.rule.as_str(), task.rule_version, task.status),
        ("approved-review", 1, Status::Open)
    );
    assert_eq!(task.due_on, d("2026-11-24"), "Friday plus two working days");
    assert_eq!(task.reminders, vec![d("2026-11-23")]);
    assert_eq!(task.author, "rule:approved-review@1");
    let audited: i64 =
        sqlx::query_scalar("SELECT count(*) FROM audit_events WHERE resource_ref = $1")
            .bind(format!("Task/{}", task.id))
            .fetch_one(&db)
            .await
            .unwrap();
    assert_eq!(audited, 1);
    // A normal result matches no rule.
    let mut tx = conn.begin().await.unwrap();
    let normal: BTreeMap<String, String> = [("abnormal".to_string(), "false".to_string())]
        .into_iter()
        .collect();
    let other = Source {
        kind: "result".into(),
        id: format!("r-{}", stamp()),
    };
    assert!(
        store::create_from_event(
            &mut tx,
            &rules,
            Event::ResultFiled,
            &normal,
            &p,
            &other,
            d("2026-11-20"),
            &none
        )
        .await
        .unwrap()
        .is_empty()
    );
    tx.commit().await.unwrap();
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn an_event_cancels_only_its_own_sources_open_tasks_and_says_so() {
    let db = db().await;
    let team = format!("team-{}", stamp());
    let rules = rules(&team);
    let p = patient(&db).await;
    let none = BTreeSet::new();
    let mine = Source {
        kind: "result".into(),
        id: format!("mine-{}", stamp()),
    };
    let other = Source {
        kind: "result".into(),
        id: format!("other-{}", stamp()),
    };
    let mut conn = db.acquire().await.unwrap();
    let mut tx = conn.begin().await.unwrap();
    for s in [&mine, &other] {
        store::create_from_event(
            &mut tx,
            &rules,
            Event::ResultFiled,
            &abnormal(),
            &p,
            s,
            d("2026-11-20"),
            &none,
        )
        .await
        .unwrap();
    }
    tx.commit().await.unwrap();

    let mut tx = conn.begin().await.unwrap();
    // An event the rule does not name cancels nothing.
    assert!(
        store::cancel_for_event(&mut tx, Event::ResultFiled, &p, &mine)
            .await
            .unwrap()
            .is_empty()
    );
    let cancelled = store::cancel_for_event(&mut tx, Event::ResultSignedOff, &p, &mine)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(cancelled.len(), 1);
    assert_eq!(cancelled[0].status, Status::Cancelled);
    assert_eq!(
        cancelled[0].cancelled_by.as_deref(),
        Some("event:result-signed-off")
    );
    let open = store::for_team(&db, &team).await.unwrap().0;
    assert_eq!(open.len(), 1, "the other result's task is still open");
    assert_eq!(open[0].source, other);
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_person_closes_a_task_with_a_reason_at_the_version_read_once() {
    let db = db().await;
    let team = format!("team-{}", stamp());
    let p = patient(&db).await;
    let source = Source {
        kind: "result".into(),
        id: format!("r-{}", stamp()),
    };
    let mut conn = db.acquire().await.unwrap();
    let mut tx = conn.begin().await.unwrap();
    let task = store::create_from_event(
        &mut tx,
        &rules(&team),
        Event::ResultFiled,
        &abnormal(),
        &p,
        &source,
        d("2026-11-20"),
        &BTreeSet::new(),
    )
    .await
    .unwrap()
    .remove(0);
    tx.commit().await.unwrap();

    assert!(matches!(
        store::close(&db, &task.id, 1, Status::Done, "dr-2", "  ").await,
        Err(StoreError::BadQuery(_))
    ));
    assert!(matches!(
        store::close(&db, &task.id, 1, Status::Open, "dr-2", "x").await,
        Err(StoreError::BadQuery(_))
    ));
    assert!(matches!(
        store::close(&db, &task.id, 2, Status::Done, "dr-2", "reviewed").await,
        Err(StoreError::Conflict { current: 1 })
    ));
    let done = store::close(
        &db,
        &task.id,
        1,
        Status::Done,
        "dr-2",
        "reviewed, no action",
    )
    .await
    .unwrap();
    assert_eq!((done.version, done.status), (2, Status::Done));
    assert_eq!(done.close_reason.as_deref(), Some("reviewed, no action"));
    assert!(matches!(
        store::close(&db, &task.id, 2, Status::Cancelled, "dr-2", "again").await,
        Err(StoreError::BadQuery(_))
    ));
    assert!(store::for_team(&db, &team).await.unwrap().0.is_empty());
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn the_task_tables_refuse_change() {
    let db = db().await;
    for sql in [
        "UPDATE task_versions SET team = 'x'",
        "DELETE FROM task_versions",
        "TRUNCATE task_versions",
        "UPDATE task_reminder_decisions SET reason = 'x'",
        "DELETE FROM task_reminder_decisions",
        "TRUNCATE task_reminder_runs",
    ] {
        // Each table may be empty, so make sure the trigger fires: TRUNCATE always fires; UPDATE and DELETE fire on rows,
        // and the earlier tests leave rows in task_versions.
        let r = sqlx::query(sql).execute(&db).await;
        if sql.starts_with("TRUNCATE") || sql.contains("task_versions") {
            assert!(r.is_err(), "{sql} was allowed");
        }
    }
    let n: i64 = sqlx::query("SELECT count(*) AS n FROM task_versions")
        .fetch_one(&db)
        .await
        .unwrap()
        .get("n");
    assert!(n >= 0);
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn each_reminder_is_decided_once_and_a_late_one_follows_its_rule() {
    let db = db().await;
    let team = format!("team-{}", stamp());
    let p = patient(&db).await;
    let source = Source {
        kind: "result".into(),
        id: format!("r-{}", stamp()),
    };
    let mut conn = db.acquire().await.unwrap();
    let mut tx = conn.begin().await.unwrap();
    // Made on Friday 2026-11-20: due Tuesday the 24th, reminded Monday the 23rd; the rule skips a late reminder.
    let task = store::create_from_event(
        &mut tx,
        &rules(&team),
        Event::ResultFiled,
        &abnormal(),
        &p,
        &source,
        d("2026-11-20"),
        &BTreeSet::new(),
    )
    .await
    .unwrap()
    .remove(0);
    tx.commit().await.unwrap();

    let mine = |all: Vec<store::Reminder>| {
        all.into_iter()
            .filter(|r| r.task.id == task.id)
            .collect::<Vec<_>>()
    };
    assert!(
        mine(store::reminders_due(&db, d("2026-11-22")).await.unwrap()).is_empty(),
        "not yet its day"
    );
    // On Wednesday, with the sender's last run recorded after the reminder's day, it was not missed: send it.
    store::record_run(&db, d("2026-11-23")).await.unwrap();
    let due = mine(store::reminders_due(&db, d("2026-11-25")).await.unwrap());
    assert_eq!(due.len(), 1);
    assert_eq!(
        (due[0].on, &due[0].now),
        (d("2026-11-23"), &ReminderNow::Send)
    );
    store::decide_reminder(&db, &task.id, d("2026-11-23"), None)
        .await
        .unwrap();
    assert!(
        matches!(
            store::decide_reminder(&db, &task.id, d("2026-11-23"), Some("again")).await,
            Err(StoreError::Conflict { .. })
        ),
        "a reminder is decided once"
    );
    assert!(
        mine(store::reminders_due(&db, d("2026-11-25")).await.unwrap()).is_empty(),
        "decided, so not offered again"
    );
}
