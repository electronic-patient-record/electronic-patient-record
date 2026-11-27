//! Tasks from rules (TL-3, TL-4, TL-14) against Postgres: only an approved rule makes a task, with an audit row, in
//! the caller's transaction; a rolled-back write leaves no task; an event cancels only its own source's tasks; a person
//! closes a task with a reason at the version read; the tables refuse change; each reminder is decided once, and a
//! late one follows its rule. Filing a result raises its event in the same transaction, and signing it off cancels
//! its task; an observation out of range makes one, and entered in error cancels it; an urgent referral makes one, and closing
//! the referral cancels it; a refusal of treatment coming to be doubted makes one, and standing again cancels it.

use electronic_patient_record::observations::Range;
use electronic_patient_record::observations_store::{
    self, ObservationRecord, Status as ObservationStatus,
};
use electronic_patient_record::patients_store::{self, StoreError};
use electronic_patient_record::referral::{Actor, Event as ReferralEvent};
use electronic_patient_record::referrals_store::{self, Priority, Referral, Step};
use electronic_patient_record::results::{Interpretation, ResultRecord, Status as ResultStatus};
use electronic_patient_record::results_store;
use electronic_patient_record::task_reminders;
use electronic_patient_record::task_rules::{self, Event, ReminderNow};
use electronic_patient_record::tasks_store::{self as store, Source, Status};
use electronic_patient_record::treatment_refusal::RefusalRecord;
use electronic_patient_record::treatment_refusals_store::{self as refusals, Edit};
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
        version: None,
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
        version: None,
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
        version: None,
    };
    let other = Source {
        kind: "result".into(),
        id: format!("other-{}", stamp()),
        version: None,
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
        version: None,
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
        version: None,
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

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn an_observation_made_critical_is_news_and_an_empty_range_is_never_kept() {
    let db = db().await;
    let team = format!("team-{}", stamp());
    let rules = task_rules::parse(
        &format!(
            r#"
[[rule]]
id = "critical-observation"
version = 1
title = "Act"
approved_by = "A Clinician"
approved_on = "2026-11-18"
event = "observation-out-of-range"
cancel_on = ["observation-entered-in-error"]
[[rule.when]]
fact = "flag"
any_of = ["critical-high"]
[rule.task]
team = "{team}"
title = "Act on the observation"
due_working_days = 0
late_reminder = "send-once"
"#
        ),
        "test",
    )
    .unwrap();
    let p = patient(&db).await;
    let now: jiff::Timestamp = "2026-11-18T09:00:00Z".parse().unwrap();
    let with = |v: f64, range: Option<Range>, critical: Option<Range>| ObservationRecord {
        range,
        critical,
        ..heart_rate(v, ObservationStatus::Final)
    };
    let upper = |v: f64| {
        Some(Range {
            low: None,
            high: Some(v),
        })
    };
    let empty = Some(Range {
        low: None,
        high: None,
    });
    // High and not critical, then corrected to critically high: one task, though it was out of range already.
    let o = observations_store::create_with_rules(
        &db,
        &p,
        "nurse-1",
        None,
        &with(130.0, upper(100.0), upper(140.0)),
        "UTC",
        now,
        &rules,
    )
    .await
    .unwrap();
    assert!(open_tasks(&db, &team).await.is_empty());
    for (version, v) in [(1, 150.0), (2, 155.0)] {
        observations_store::update_with_rules(
            &db,
            &p,
            &o.id,
            version,
            "nurse-1",
            Some("misread"),
            &with(v, upper(100.0), upper(140.0)),
            "UTC",
            now,
            &rules,
        )
        .await
        .unwrap();
        assert_eq!(open_tasks(&db, &team).await.len(), 1, "{v}");
    }
    // A critical limit alone, crossed (the audit's case): a task.
    observations_store::create_with_rules(
        &db,
        &p,
        "nurse-1",
        None,
        &with(130.0, None, upper(120.0)),
        "UTC",
        now,
        &rules,
    )
    .await
    .unwrap();
    assert_eq!(open_tasks(&db, &team).await.len(), 2);
    // An empty range is kept as absent, and says nothing.
    let bare = observations_store::create(
        &db,
        &p,
        "nurse-1",
        None,
        &with(130.0, empty, empty),
        "UTC",
        now,
    )
    .await
    .unwrap();
    assert_eq!((bare.record.range, bare.record.critical), (None, None));
    assert_eq!(bare.flag, None);
    let stored: serde_json::Value =
        sqlx::query_scalar("SELECT body FROM observation_versions WHERE observation_id = $1::uuid")
            .bind(&bare.id)
            .fetch_one(&db)
            .await
            .unwrap();
    assert!(stored.get("range").is_none() && stored.get("critical").is_none());
    // One kept before this was so (the trigger refuses an edit, so it is written as it then was): read as not assessed.
    let old: String = sqlx::query_scalar(
        "INSERT INTO observation_versions (observation_id, version, patient_id, author, status, effective_at, site_zone, body)
         SELECT gen_random_uuid(), 1, patient_id, author, status, effective_at, site_zone, body || '{\"range\": {}}'
         FROM observation_versions WHERE observation_id = $1::uuid RETURNING observation_id::text",
    )
    .bind(&bare.id)
    .fetch_one(&db)
    .await
    .unwrap();
    let read = observations_store::list(&db, &p)
        .await
        .unwrap()
        .into_iter()
        .find(|x| x.id == old)
        .unwrap();
    assert_eq!(read.flag, None, "never normal");
    let fhir = electronic_patient_record::interop::fhir_observation::to_fhir(&read, &p, &[])
        .unwrap()
        .unwrap();
    let j = serde_json::to_value(&fhir).unwrap();
    assert!(
        j.get("interpretation").is_none() && j.get("referenceRange").is_none(),
        "{j}"
    );
    // It can still be marked in error by a client that sends back what it read.
    let mut retract = read.record.clone();
    retract.status = ObservationStatus::EnteredInError;
    observations_store::update(
        &db,
        &p,
        &old,
        1,
        "dr-1",
        Some("mistake"),
        &retract,
        "UTC",
        now,
    )
    .await
    .unwrap();
}

fn referral_rules(team: &str) -> Vec<task_rules::Rule> {
    task_rules::parse(
        &format!(
            r#"
[[rule]]
id = "triage-urgent-referral"
version = 1
title = "Triage an urgent referral"
approved_by = "A Clinician"
approved_on = "2026-11-18"
event = "referral-stepped"
cancel_on = ["referral-closed"]
[[rule.when]]
fact = "state"
any_of = ["requested"]
[[rule.when]]
fact = "priority"
any_of = ["urgent", "stat"]
[rule.task]
team = "{team}"
title = "Triage the referral"
due_working_days = 1
late_reminder = "send-once"
"#
        ),
        "test",
    )
    .unwrap()
}

fn referral(priority: Priority) -> Referral {
    Referral {
        recipient: "Cardiology outpatients".into(),
        reason: "Exertional chest pain".into(),
        question: "Is this angina?".into(),
        priority,
        external_ids: vec![],
        replaces: None,
    }
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn an_urgent_referral_makes_its_task_and_closing_the_referral_cancels_it() {
    let db = db().await;
    let team = format!("team-{}", stamp());
    let rules = referral_rules(&team);
    let p = patient(&db).await;
    let now = jiff::Timestamp::now();
    let open_for = |all: Vec<store::Task>, id: &str| {
        all.into_iter()
            .filter(|t| t.source.id == id)
            .collect::<Vec<_>>()
    };

    let routine = referrals_store::request_with_rules(
        &db,
        "gp-1",
        &p,
        &referral(Priority::Routine),
        now,
        &rules,
    )
    .await
    .unwrap();
    assert!(open_for(store::for_team(&db, &team).await.unwrap().0, &routine.id).is_empty());

    let urgent = referrals_store::request_with_rules(
        &db,
        "gp-1",
        &p,
        &referral(Priority::Urgent),
        now,
        &rules,
    )
    .await
    .unwrap();
    let open = open_for(store::for_team(&db, &team).await.unwrap().0, &urgent.id);
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].source.kind, "referral");

    // Accepted: a step, which this rule neither makes nor cancels on.
    let step = |event, version| {
        referrals_store::advance(
            &db,
            &urgent.id,
            version,
            Step {
                actor: Actor::Recipient,
                by: "cardiology-1",
                event,
                note: None,
                missed: None,
            },
            now,
        )
    };
    step(ReferralEvent::Accept, 1).await.unwrap();
    assert_eq!(
        open_for(store::for_team(&db, &team).await.unwrap().0, &urgent.id).len(),
        1
    );

    // Completed: closed, so its task is cancelled, naming the event.
    referrals_store::advance(
        &db,
        &urgent.id,
        2,
        Step {
            actor: Actor::Recipient,
            by: "cardiology-1",
            event: ReferralEvent::Outcome,
            note: Some("Stable angina; started on treatment"),
            missed: None,
        },
        now,
    )
    .await
    .unwrap();
    assert!(open_for(store::for_team(&db, &team).await.unwrap().0, &urgent.id).is_empty());
    let task = store::one(&db, &open[0].id).await.unwrap();
    assert_eq!(
        (task.status, task.cancelled_by.as_deref()),
        (Status::Cancelled, Some("event:referral-closed"))
    );
}

fn refusal(state: &str) -> RefusalRecord {
    serde_json::from_value(serde_json::json!({
        "kind": "spoken-now",
        "treatment": "blood products",
        "if_life_at_risk": "not-stated",
        "form": {"written": "no", "signed": "no", "witnessed": "no", "seen": "not-seen"},
        "said_by": "patient",
        "identity": "certain",
        "state": state,
        "doubt": if state == "doubted" { Some("said in pain") } else { None },
    }))
    .unwrap()
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_refusal_coming_to_be_doubted_makes_its_task_and_standing_again_cancels_it() {
    let db = db().await;
    let team = format!("team-{}", stamp());
    // The built-in draft, approved here and given a team of its own.
    let text = include_str!("../electronic-patient-record-domain/src/task_rules.toml")
        .replace(
            "approved_by = \"\"\napproved_on = \"\"\nevent = \"treatment-refusal-doubted\"",
            "approved_by = \"A Clinician\"\napproved_on = \"2026-11-27\"\nevent = \"treatment-refusal-doubted\"",
        )
        .replace(
            "team = \"clinician\"\ntitle = \"Look at",
            &format!("team = \"{team}\"\ntitle = \"Look at"),
        );
    let rules: Vec<_> = task_rules::parse(&text, "test")
        .unwrap()
        .into_iter()
        .filter(|r| r.id == "doubted-refusal-review")
        .collect();
    assert!(rules[0].approved());
    let p = patient(&db).await;
    let open = || async { store::for_team(&db, &team).await.unwrap().0 };

    // Recorded as standing: nothing. Doubted: one task, from the refusal, with the rule's words only.
    let (doubted, stands) = (refusal("doubted"), refusal("stands"));
    let r = refusals::create_with_rules(&db, &p, "dr-1", None, &stands, &rules)
        .await
        .unwrap();
    assert!(open().await.is_empty());
    let edit = |v, record, why| Edit {
        patient_id: &p,
        refusal_id: &r.id,
        expected_version: v,
        author: "dr-1",
        reason: Some(why),
        record,
    };
    refusals::update_with_rules(&db, &edit(1, &doubted, "said in pain"), &rules)
        .await
        .unwrap();
    let tasks = open().await;
    assert_eq!(tasks.len(), 1);
    assert_eq!(
        (tasks[0].source.kind.as_str(), tasks[0].source.id.as_str()),
        ("treatment-refusal", r.id.as_str())
    );
    assert!(
        !tasks[0].title.contains("blood"),
        "never what is refused: {}",
        tasks[0].title
    );
    // Still doubted, changed again: no second task.
    refusals::update_with_rules(&db, &edit(2, &doubted, "asked again"), &rules)
        .await
        .unwrap();
    assert_eq!(open().await.len(), 1);
    // It stands after all: the task is cancelled, naming the event.
    refusals::update_with_rules(&db, &edit(3, &stands, "asked again, calm"), &rules)
        .await
        .unwrap();
    assert!(open().await.is_empty());
    let task = store::one(&db, &tasks[0].id).await.unwrap();
    assert_eq!(
        (task.status, task.cancelled_by.as_deref()),
        (Status::Cancelled, Some("event:treatment-refusal-resolved"))
    );
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn the_sender_decides_a_due_reminder_once_and_the_list_shows_the_day() {
    let db = db().await;
    let team = format!("team-{}", stamp());
    let p = patient(&db).await;
    let mut conn = db.acquire().await.unwrap();
    let mut tx = conn.begin().await.unwrap();
    // Made on Friday 2026-11-20: due Tuesday the 24th, reminded Monday the 23rd.
    let task = store::create_from_event(
        &mut tx,
        &rules(&team),
        Event::ResultFiled,
        &abnormal(),
        &p,
        &Source {
            kind: "result".into(),
            id: format!("r-{}", stamp()),
            version: None,
        },
        d("2026-11-20"),
        &BTreeSet::new(),
    )
    .await
    .unwrap()
    .remove(0);
    tx.commit().await.unwrap();
    let decision = |db: PgPool, id: String| async move {
        sqlx::query_scalar::<_, String>(
            "SELECT decision FROM task_reminder_decisions WHERE task_id = $1::uuid",
        )
        .bind(id)
        .fetch_all(&db)
        .await
        .unwrap()
    };

    // The day before: nothing to decide for this task.
    task_reminders::run_once(&db, d("2026-11-22"))
        .await
        .unwrap();
    assert!(decision(db.clone(), task.id.clone()).await.is_empty());
    assert!(
        store::reminded_on(&db, std::slice::from_ref(&task.id))
            .await
            .unwrap()
            .is_empty()
    );

    // Its day: sent, once; a second run finds it decided.
    let first = task_reminders::run_once(&db, d("2026-11-23"))
        .await
        .unwrap();
    assert!(first.sent >= 1, "{first:?}");
    assert_eq!(decision(db.clone(), task.id.clone()).await, ["sent"]);
    task_reminders::run_once(&db, d("2026-11-23"))
        .await
        .unwrap();
    assert_eq!(
        decision(db.clone(), task.id.clone()).await,
        ["sent"],
        "never twice"
    );
    assert_eq!(
        store::reminded_on(&db, std::slice::from_ref(&task.id))
            .await
            .unwrap()
            .get(&task.id),
        Some(&d("2026-11-23"))
    );
    assert_eq!(
        store::last_run(&db)
            .await
            .unwrap()
            .map(|l| l >= d("2026-11-23")),
        Some(true)
    );
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_result_amended_to_abnormal_makes_its_task_once_and_entered_in_error_cancels_it() {
    let db = db().await;
    let team = format!("team-{}", stamp());
    let rules = task_rules::parse(
        &format!(
            r#"
[[rule]]
id = "amended-review"
version = 1
title = "Review"
approved_by = "A Clinician"
approved_on = "2026-11-18"
event = "result-filed"
cancel_on = ["result-entered-in-error"]
[[rule.when]]
fact = "abnormal"
any_of = ["true"]
[rule.task]
team = "{team}"
title = "Review the result"
due_working_days = 1
late_reminder = "send-once"
"#
        ),
        "test",
    )
    .unwrap();
    let p = patient(&db).await;
    let open_for = |all: Vec<store::Task>, id: &str| {
        all.into_iter()
            .filter(|t| t.source.id == id)
            .collect::<Vec<_>>()
    };
    let filed = results_store::create_with_rules(
        &db,
        &p,
        "lab-1",
        &result(Some(Interpretation::Normal), ResultStatus::Final),
        &rules,
    )
    .await
    .unwrap();
    assert!(open_for(store::for_team(&db, &team).await.unwrap().0, &filed.id).is_empty());

    // Amended to abnormal: one task. Corrected, still abnormal: no second.
    for (version, status, why) in [
        (1, ResultStatus::Amended, "repeat sample"),
        (2, ResultStatus::Corrected, "unit typo"),
    ] {
        results_store::update_with_rules(
            &db,
            &p,
            &filed.id,
            version,
            "lab-1",
            Some(why),
            &result(Some(Interpretation::Abnormal), status),
            &rules,
        )
        .await
        .unwrap();
        assert_eq!(
            open_for(store::for_team(&db, &team).await.unwrap().0, &filed.id).len(),
            1,
            "{status:?}"
        );
    }
    let task = open_for(store::for_team(&db, &team).await.unwrap().0, &filed.id).remove(0);

    // Entered in error: its task is cancelled, naming the event.
    results_store::update_with_rules(
        &db,
        &p,
        &filed.id,
        3,
        "lab-1",
        Some("wrong patient"),
        &result(Some(Interpretation::Abnormal), ResultStatus::EnteredInError),
        &rules,
    )
    .await
    .unwrap();
    let task = store::one(&db, &task.id).await.unwrap();
    assert_eq!(
        (task.status, task.cancelled_by.as_deref()),
        (Status::Cancelled, Some("event:result-entered-in-error"))
    );
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_failure_in_a_rules_second_action_leaves_no_first_task_and_no_result() {
    let db = db().await;
    let team = format!("team-{}", stamp());
    // Two approved rules on one event, run in id order: the first's task is written, then the second's is refused.
    let rules = task_rules::parse(
        &format!(
            r#"
[[rule]]
id = "tl14-a-first"
version = 1
title = "First"
approved_by = "A Clinician"
approved_on = "2026-11-18"
event = "result-filed"
[rule.task]
team = "{team}"
title = "The first task"
due_working_days = 1
late_reminder = "skip"

[[rule]]
id = "tl14-b-second"
version = 1
title = "Second"
approved_by = "A Clinician"
approved_on = "2026-11-18"
event = "result-filed"
[rule.task]
team = "{team}"
title = "The second task"
due_working_days = 2
late_reminder = "skip"
"#
        ),
        "test",
    )
    .unwrap();
    let p = patient(&db).await;
    for sql in [
        "CREATE OR REPLACE FUNCTION epr_test_tl14_refuse() RETURNS trigger AS $$
         BEGIN
           IF NEW.rule_id = 'tl14-b-second' THEN RAISE EXCEPTION 'the second action fails'; END IF;
           RETURN NEW;
         END $$ LANGUAGE plpgsql",
        "DROP TRIGGER IF EXISTS epr_test_tl14_refuse ON task_versions",
        "CREATE TRIGGER epr_test_tl14_refuse BEFORE INSERT ON task_versions
         FOR EACH ROW EXECUTE FUNCTION epr_test_tl14_refuse()",
    ] {
        sqlx::raw_sql(sqlx::AssertSqlSafe(sql))
            .execute(&db)
            .await
            .unwrap();
    }
    let results_before: i64 =
        sqlx::query_scalar("SELECT count(*) FROM result_versions WHERE patient_id = $1::uuid")
            .bind(&p)
            .fetch_one(&db)
            .await
            .unwrap();

    let mut conn = db.acquire().await.unwrap();
    let mut tx = conn.begin().await.unwrap();
    let filed = results_store::create_with_rules(
        &mut *tx,
        &p,
        "lab-1",
        &result(Some(Interpretation::Abnormal), ResultStatus::Final),
        &rules,
    )
    .await;
    let _ = tx.rollback().await;
    drop(conn);
    for sql in [
        "DROP TRIGGER IF EXISTS epr_test_tl14_refuse ON task_versions",
        "DROP FUNCTION IF EXISTS epr_test_tl14_refuse()",
    ] {
        sqlx::raw_sql(sqlx::AssertSqlSafe(sql))
            .execute(&db)
            .await
            .unwrap();
    }

    assert!(filed.is_err(), "the write fails with its second action");
    assert!(
        store::for_team(&db, &team).await.unwrap().0.is_empty(),
        "the first rule's task went with it"
    );
    let results_after: i64 =
        sqlx::query_scalar("SELECT count(*) FROM result_versions WHERE patient_id = $1::uuid")
            .bind(&p)
            .fetch_one(&db)
            .await
            .unwrap();
    assert_eq!(results_before, results_after, "and so did the result");

    // With the trigger gone, the same write makes both tasks.
    results_store::create_with_rules(
        &db,
        &p,
        "lab-1",
        &result(Some(Interpretation::Abnormal), ResultStatus::Final),
        &rules,
    )
    .await
    .unwrap();
    let made = store::for_team(&db, &team).await.unwrap().0;
    let mut rules_made: Vec<&str> = made.iter().map(|t| t.rule.as_str()).collect();
    rules_made.sort_unstable();
    assert_eq!(rules_made, ["tl14-a-first", "tl14-b-second"]);
}

/// A rule for `team` on abnormal results, approved, cancelled by a signature or an error.
fn review_rule(team: &str, fact: &str) -> Vec<task_rules::Rule> {
    task_rules::parse(
        &format!(
            r#"
[[rule]]
id = "review"
version = 1
title = "Review"
approved_by = "A Clinician"
approved_on = "2026-11-18"
event = "result-filed"
cancel_on = ["result-signed-off", "result-entered-in-error"]
[[rule.when]]
fact = "{fact}"
any_of = ["true"]
[rule.task]
team = "{team}"
title = "Review the result"
due_working_days = 1
late_reminder = "send-once"
"#
        ),
        "test",
    )
    .unwrap()
}

async fn open_tasks(db: &PgPool, team: &str) -> Vec<store::Task> {
    store::for_team(db, team).await.unwrap().0
}

/// File `interpretation` as the next version of `id`, in a transaction as the route does.
async fn correct(
    db: &PgPool,
    p: &str,
    id: &str,
    version: i32,
    interpretation: Option<Interpretation>,
    rules: &[task_rules::Rule],
) -> Result<(), StoreError> {
    let mut tx = db.begin().await.unwrap();
    results_store::update_with_rules(
        &mut *tx,
        p,
        id,
        version,
        "lab-1",
        Some("corrected by the laboratory"),
        &result(interpretation, ResultStatus::Corrected),
        rules,
    )
    .await?;
    tx.commit().await.unwrap();
    Ok(())
}

async fn sign(db: &PgPool, p: &str, id: &str, version: i32) -> Result<bool, StoreError> {
    let mut tx = db.begin().await.unwrap();
    let (_, new) = results_store::sign_off(&mut *tx, p, id, version, "dr-1").await?;
    tx.commit().await.unwrap();
    Ok(new)
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_correction_to_critical_after_sign_off_opens_one_obligation_for_the_new_version() {
    let db = db().await;
    let team = format!("team-{}", stamp());
    let rules = review_rule(&team, "abnormal");
    let p = patient(&db).await;
    let filed = results_store::create_with_rules(
        &db,
        &p,
        "lab-1",
        &result(Some(Interpretation::Abnormal), ResultStatus::Final),
        &rules,
    )
    .await
    .unwrap();
    assert!(sign(&db, &p, &filed.id, 1).await.unwrap());
    assert!(
        open_tasks(&db, &team).await.is_empty(),
        "signing met the first obligation"
    );
    let critical = Some(Interpretation::Critical);
    correct(&db, &p, &filed.id, 1, critical, &rules)
        .await
        .unwrap();
    let open = open_tasks(&db, &team).await;
    assert_eq!(
        open.len(),
        1,
        "the clinician covering the next shift has it"
    );
    assert_eq!(open[0].source.version, Some(2));

    // A replay of the correction makes no second, and the old signature cannot be used again.
    assert!(matches!(
        correct(&db, &p, &filed.id, 1, critical, &rules).await,
        Err(StoreError::Conflict { current: 2 })
    ));
    assert!(matches!(
        sign(&db, &p, &filed.id, 1).await,
        Err(StoreError::Conflict { current: 2 })
    ));
    assert_eq!(open_tasks(&db, &team).await, open);
    // The earlier signature stays visible, on the version it named.
    let view = results_store::list(&db, &p).await.unwrap().remove(0);
    assert_eq!(view.last_sign_off.unwrap().version, 1);
    assert_eq!(view.signed_version.unwrap().version, 1);

    // Still critical and corrected again while the obligation is open: one, not two.
    correct(&db, &p, &filed.id, 2, critical, &rules)
        .await
        .unwrap();
    assert_eq!(open_tasks(&db, &team).await.len(), 1);
    // Signing the version now current meets it.
    assert!(sign(&db, &p, &filed.id, 3).await.unwrap());
    assert!(open_tasks(&db, &team).await.is_empty());
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn the_critical_flag_and_a_change_after_sign_off_are_facts_a_rule_can_name() {
    let db = db().await;
    let p = patient(&db).await;
    // Abnormal, then critical while unsigned: a rule on `critical` fires though `abnormal` was already true.
    let team = format!("critical-{}", stamp());
    let rules = review_rule(&team, "critical");
    let filed = results_store::create_with_rules(
        &db,
        &p,
        "lab-1",
        &result(Some(Interpretation::Abnormal), ResultStatus::Final),
        &rules,
    )
    .await
    .unwrap();
    assert!(open_tasks(&db, &team).await.is_empty());
    let critical = Some(Interpretation::Critical);
    correct(&db, &p, &filed.id, 1, critical, &rules)
        .await
        .unwrap();
    assert_eq!(open_tasks(&db, &team).await.len(), 1);

    // Signed as abnormal and corrected to normal, or to no flag: the treatment it started may be wrong.
    for corrected in [Some(Interpretation::Normal), None] {
        let team = format!("after-{}", stamp());
        let rules = review_rule(&team, "after_sign_off");
        let filed = results_store::create_with_rules(
            &db,
            &p,
            "lab-1",
            &result(Some(Interpretation::Abnormal), ResultStatus::Final),
            &rules,
        )
        .await
        .unwrap();
        assert!(open_tasks(&db, &team).await.is_empty());
        sign(&db, &p, &filed.id, 1).await.unwrap();
        correct(&db, &p, &filed.id, 1, corrected, &rules)
            .await
            .unwrap();
        assert_eq!(open_tasks(&db, &team).await.len(), 1, "{corrected:?}");
    }
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_signature_racing_a_correction_never_cancels_the_new_versions_obligation() {
    let db = db().await;
    for _ in 0..12 {
        let team = format!("race-{}", stamp());
        let rules = review_rule(&team, "abnormal");
        let p = patient(&db).await;
        let filed = results_store::create_with_rules(
            &db,
            &p,
            "lab-1",
            &result(Some(Interpretation::Abnormal), ResultStatus::Final),
            &rules,
        )
        .await
        .unwrap();
        let critical = Some(Interpretation::Critical);
        let (_, corrected) = tokio::join!(
            sign(&db, &p, &filed.id, 1),
            correct(&db, &p, &filed.id, 1, critical, &rules)
        );
        corrected.unwrap();
        assert_eq!(
            open_tasks(&db, &team).await.len(),
            1,
            "version 2 is unsigned"
        );
    }
}
