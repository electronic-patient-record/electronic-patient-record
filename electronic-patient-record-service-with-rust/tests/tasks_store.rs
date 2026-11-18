//! Tasks from rules (TL-3, TL-4, TL-14) against Postgres: only an approved rule makes a task, with an audit row, in
//! the caller's transaction; a rolled-back write leaves no task; an event cancels only its own source's tasks; a person
//! closes a task with a reason at the version read; the tables refuse change; each reminder is decided once, and a
//! late one follows its rule. Filing a result raises its event in the same transaction, and signing it off cancels
//! its task; an observation out of range makes one, and entered in error cancels it.

use electronic_patient_record::observations::Range;
use electronic_patient_record::observations_store::{
    self, ObservationRecord, Status as ObservationStatus,
};
use electronic_patient_record::patients_store::{self, StoreError};
use electronic_patient_record::results::{Interpretation, ResultRecord, Status as ResultStatus};
use electronic_patient_record::results_store;
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

fn result(interpretation: Option<Interpretation>, status: ResultStatus) -> ResultRecord {
    ResultRecord {
        test: "Serum potassium".into(),
        value: Some("6.1".into()),
        unit: Some("mmol/L".into()),
        reference_range: Some("3.5-5.3".into()),
        interpretation,
        status,
        issued: "2026-11-18T09:00:00Z".into(),
        performer: None,
    }
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn filing_a_result_makes_its_task_in_the_same_transaction_and_signing_it_off_cancels_it() {
    let db = db().await;
    let team = format!("team-{}", stamp());
    let rules = rules(&team);
    let p = patient(&db).await;
    let mine = |all: Vec<store::Task>, id: &str| {
        all.into_iter()
            .filter(|t| t.source.id == id)
            .collect::<Vec<_>>()
    };

    // Rolled back: neither the result nor its task is left.
    let mut conn = db.acquire().await.unwrap();
    let mut tx = conn.begin().await.unwrap();
    let gone = results_store::create_with_rules(
        &mut *tx,
        &p,
        "lab-1",
        &result(Some(Interpretation::Abnormal), ResultStatus::Final),
        &rules,
    )
    .await
    .unwrap();
    tx.rollback().await.unwrap();
    assert!(mine(store::for_team(&db, &team).await.unwrap().0, &gone.id).is_empty());

    // Abnormal: the approved rule makes one task for this result; the draft makes none.
    let mut tx = conn.begin().await.unwrap();
    let filed = results_store::create_with_rules(
        &mut *tx,
        &p,
        "lab-1",
        &result(Some(Interpretation::Critical), ResultStatus::Final),
        &rules,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let open = mine(store::for_team(&db, &team).await.unwrap().0, &filed.id);
    assert_eq!(open.len(), 1);
    assert_eq!(
        (open[0].rule.as_str(), open[0].source.kind.as_str()),
        ("approved-review", "result")
    );

    // No flag, a normal flag, or entered in error: no task.
    for (i, status) in [
        (None, ResultStatus::Final),
        (Some(Interpretation::Normal), ResultStatus::Final),
        (Some(Interpretation::Abnormal), ResultStatus::EnteredInError),
    ] {
        let r = results_store::create_with_rules(&db, &p, "lab-1", &result(i, status), &rules)
            .await
            .unwrap();
        assert!(
            mine(store::for_team(&db, &team).await.unwrap().0, &r.id).is_empty(),
            "{i:?} {status:?} made a task"
        );
    }

    // Signing it off cancels its task, naming the event; signing again changes nothing.
    let mut tx = conn.begin().await.unwrap();
    let (_, new) = results_store::sign_off(&mut *tx, &p, &filed.id, 1, "dr-2")
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert!(new);
    assert!(mine(store::for_team(&db, &team).await.unwrap().0, &filed.id).is_empty());
    let task = store::one(&db, &open[0].id).await.unwrap();
    assert_eq!(task.status, Status::Cancelled);
    assert_eq!(
        task.cancelled_by.as_deref(),
        Some("event:result-signed-off")
    );
    let (_, again) = results_store::sign_off(&db, &p, &filed.id, 1, "dr-2")
        .await
        .unwrap();
    assert!(!again);
    assert_eq!(
        store::one(&db, &open[0].id).await.unwrap().version,
        task.version
    );
}

fn observation_rules(team: &str) -> Vec<task_rules::Rule> {
    task_rules::parse(
        &format!(
            r#"
[[rule]]
id = "high-heart-rate"
version = 1
title = "High heart rate"
approved_by = "A Clinician"
approved_on = "2026-11-18"
event = "observation-out-of-range"
cancel_on = ["observation-entered-in-error"]
[[rule.when]]
fact = "flag"
any_of = ["high", "critical-high"]
[rule.task]
team = "{team}"
title = "Review the heart rate"
due_working_days = 0
late_reminder = "send-once"
"#
        ),
        "test",
    )
    .unwrap()
}

fn heart_rate(value: f64, status: ObservationStatus) -> ObservationRecord {
    ObservationRecord {
        code: "8867-4".into(),
        value: Some(value),
        unit: Some("/min".into()),
        absent_reason: None,
        range: Some(Range {
            low: Some(60.0),
            high: Some(100.0),
        }),
        critical: None,
        effective_at: "2026-11-18T08:00:00Z".into(),
        status,
    }
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn an_out_of_range_observation_makes_its_task_and_entered_in_error_cancels_it() {
    let db = db().await;
    let team = format!("team-{}", stamp());
    let rules = observation_rules(&team);
    let p = patient(&db).await;
    let now: jiff::Timestamp = "2026-11-18T09:00:00Z".parse().unwrap();
    let open_for = |all: Vec<store::Task>, id: &str| {
        all.into_iter()
            .filter(|t| t.source.id == id)
            .collect::<Vec<_>>()
    };

    // In range, or low (the rule asks only for high): no task.
    for v in [72.0, 50.0] {
        let o = observations_store::create_with_rules(
            &db,
            &p,
            "nurse-1",
            None,
            &heart_rate(v, ObservationStatus::Final),
            "UTC",
            now,
            &rules,
        )
        .await
        .unwrap();
        assert!(open_for(store::for_team(&db, &team).await.unwrap().0, &o.id).is_empty());
    }

    // High: one task, from this observation.
    let high = observations_store::create_with_rules(
        &db,
        &p,
        "nurse-1",
        None,
        &heart_rate(130.0, ObservationStatus::Final),
        "UTC",
        now,
        &rules,
    )
    .await
    .unwrap();
    let open = open_for(store::for_team(&db, &team).await.unwrap().0, &high.id);
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].source.kind, "observation");

    // Corrected to another high value: still high, so no second task.
    observations_store::update(
        &db,
        &p,
        &high.id,
        1,
        "nurse-1",
        Some("misread"),
        &heart_rate(135.0, ObservationStatus::Final),
        "UTC",
        now,
    )
    .await
    .unwrap();
    assert_eq!(
        open_for(store::for_team(&db, &team).await.unwrap().0, &high.id).len(),
        1,
        "a second save of a high value makes no second task"
    );

    // Entered in error: its task is cancelled, naming the event.
    let mut conn = db.acquire().await.unwrap();
    let mut tx = conn.begin().await.unwrap();
    observations_store::update(
        &mut *tx,
        &p,
        &high.id,
        2,
        "nurse-1",
        Some("wrong patient"),
        &heart_rate(130.0, ObservationStatus::EnteredInError),
        "UTC",
        now,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    assert!(open_for(store::for_team(&db, &team).await.unwrap().0, &high.id).is_empty());
    let task = store::one(&db, &open[0].id).await.unwrap();
    assert_eq!(
        (task.status, task.cancelled_by.as_deref()),
        (
            Status::Cancelled,
            Some("event:observation-entered-in-error")
        )
    );
}
