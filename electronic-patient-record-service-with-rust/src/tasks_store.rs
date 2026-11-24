//! Tasks from rules (TL-3, TL-4, TL-14): the append-only store behind [`crate::task_rules`]. A task is version 1 as a
//! rule made it (`open`), and version 2 when a person closes it (`done`) or an event or a person cancels it
//! (`cancelled`), with why. Each names the rule and version that made it.
//!
//! **All or nothing (TL-14).** [`create_from_event`] and [`cancel_for_event`] take the caller's transaction: the write
//! that raised the event, every task it makes or cancels, and their audit rows commit together or not at all. Only an
//! approved rule makes a task; a draft is the dry run's.
//!
//! **Reminders (TL-4).** Each reminder is decided once ([`decide_reminder`]): sent, or skipped with the reason. The
//! sender's runs are recorded ([`record_run`]), so a reminder whose day fell while nothing ran is known to be late,
//! and its rule says whether it is still sent once or skipped. A reminder here puts the task on its team's list as
//! reminded; nothing is sent to a patient.

use crate::audit::{self, Action, Event as AuditEvent};
use crate::ids;
use crate::patients_store::StoreError;
use crate::pg_time::PgDate;
use crate::pool::Db;
use crate::task_rules::{self, Event, Late, ReminderNow, Rule};
use jiff::civil::Date;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgRow;
use sqlx::{AssertSqlSafe, PgConnection, Row};
use std::collections::{BTreeMap, BTreeSet};

crate::store_errors!("tasks");

/// The most tasks a team's list returns.
pub const TEAM_LIMIT: i64 = 200;

/// The record a task came from: the subject a cancelling event must share.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    /// What it is (`result`, `observation`, `referral`).
    pub kind: String,
    /// Its id.
    pub id: String,
}

/// What a version stores.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Body {
    title: String,
    reminders: Vec<Date>,
    late_reminder: Late,
    cancel_on: Vec<Event>,
    source: Source,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    close_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cancelled_by: Option<String>,
}

/// Where a task stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    /// To do.
    Open,
    /// Done, by a person.
    Done,
    /// Cancelled, by a person or an event.
    Cancelled,
}

impl Status {
    /// The stored word: `open`, `done` or `cancelled`.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Status::Open => "open",
            Status::Done => "done",
            Status::Cancelled => "cancelled",
        }
    }

    fn from_word(w: &str) -> Option<Status> {
        match w {
            "open" => Some(Status::Open),
            "done" => Some(Status::Done),
            "cancelled" => Some(Status::Cancelled),
            _ => None,
        }
    }
}

/// One task, current version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Task {
    /// The task.
    pub id: String,
    /// 1 as made; 2 once closed or cancelled.
    pub version: i32,
    /// The patient.
    pub patient_id: String,
    /// The rule that made it.
    pub rule: String,
    /// That rule's version.
    pub rule_version: i32,
    /// The team (a role) whose task it is.
    pub team: String,
    /// Where it stands.
    pub status: Status,
    /// What to do.
    pub title: String,
    /// The due day, at the site.
    pub due_on: Date,
    /// Its reminder days.
    pub reminders: Vec<Date>,
    /// What a late reminder does.
    pub late_reminder: Late,
    /// The record it came from.
    pub source: Source,
    /// Who or what wrote this version.
    pub author: String,
    /// When this version was written (UTC).
    pub recorded_at: String,
    /// Why it was closed or cancelled.
    pub close_reason: Option<String>,
    /// What cancelled it: `event:<word>` or the person.
    pub cancelled_by: Option<String>,
}

const COLUMNS: &str = "task_id::text AS id, version, patient_id::text AS patient_id, rule_id, rule_version, team, \
     status, due_on::text AS due_on, author, \
     to_char(recorded_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"') AS recorded_at, body";

fn row_to_task(row: &PgRow) -> Result<Task, StoreError> {
    let body: Body =
        serde_json::from_value(row.try_get("body").map_err(db_err)?).map_err(db_err)?;
    let status: String = row.try_get("status").map_err(db_err)?;
    let due: String = row.try_get("due_on").map_err(db_err)?;
    Ok(Task {
        id: row.try_get("id").map_err(db_err)?,
        version: row.try_get("version").map_err(db_err)?,
        patient_id: row.try_get("patient_id").map_err(db_err)?,
        rule: row.try_get("rule_id").map_err(db_err)?,
        rule_version: row.try_get("rule_version").map_err(db_err)?,
        team: row.try_get("team").map_err(db_err)?,
        status: Status::from_word(&status)
            .ok_or_else(|| db_err(format!("a task status {status}")))?,
        title: body.title,
        due_on: crate::clock::parse_day(&due).ok_or_else(|| db_err(format!("a due day {due}")))?,
        reminders: body.reminders,
        late_reminder: body.late_reminder,
        source: body.source,
        author: row.try_get("author").map_err(db_err)?,
        recorded_at: row.try_get("recorded_at").map_err(db_err)?,
        close_reason: body.close_reason,
        cancelled_by: body.cancelled_by,
    })
}

async fn audit_row(
    conn: &mut PgConnection,
    action: Action,
    agent: &str,
    task: &Task,
    what: &str,
) -> Result<(), StoreError> {
    let mut row = AuditEvent::new(action, agent.to_string());
    row.patient_ref = Some(format!("Patient/{}", task.patient_id));
    row.resource_ref = Some(format!("Task/{}", task.id));
    row.detail = Some(serde_json::json!({
        "task": what,
        "rule": task.rule,
        "rule_version": task.rule_version,
        "team": task.team,
        "source": task.source.kind,
        "status": task.status.word(),
    }));
    audit::append(&mut *conn, &audit::chain(), &row)
        .await
        .map(|_| ())
        .map_err(db_err)
}

/// One task by id, current version.
///
/// # Errors
///
/// [`StoreError::NotFound`].
pub async fn one(db: impl Into<Db<'_>>, task_id: &str) -> Result<Task, StoreError> {
    let id = ids::uuid(task_id).ok_or(StoreError::NotFound)?;
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let row = sqlx::query(AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM task_current WHERE task_id = $1::uuid"
    )))
    .bind(id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db_err)?
    .ok_or(StoreError::NotFound)?;
    row_to_task(&row)
}

/// Make the tasks `event` calls for under the **approved** rules in `rules`, in the caller's transaction, each with an
/// audit row. `day` is the site day the event fell on. Returns what was made (nothing when no approved rule matches).
///
/// # Errors
///
/// [`StoreError::Db`]: the caller's transaction then rolls back with everything it holds.
#[expect(
    clippy::too_many_arguments,
    reason = "RCA-EXC-6: a store call names each field of the record; one parameter struct for all is a wider change"
)]
pub async fn create_from_event(
    conn: &mut PgConnection,
    rules: &[Rule],
    event: Event,
    facts: &BTreeMap<String, String>,
    patient_id: &str,
    source: &Source,
    day: Date,
    holidays: &BTreeSet<Date>,
) -> Result<Vec<Task>, StoreError> {
    let patient = ids::uuid(patient_id).ok_or(StoreError::NotFound)?;
    let mut made = Vec::new();
    // A date outside the calendar is no plan: nothing is made and the caller's transaction rolls back (RCA-P-3).
    let planned = task_rules::plan(rules, event, facts, day, holidays).ok_or_else(|| {
        StoreError::Db("a task's due or reminder date is outside the calendar".into())
    })?;
    for p in planned.into_iter().filter(|p| p.approved) {
        let author = format!("rule:{}@{}", p.rule, p.version);
        let body = Body {
            title: p.title.clone(),
            reminders: p.reminders.clone(),
            late_reminder: p.late_reminder,
            cancel_on: p.cancel_on.clone(),
            source: source.clone(),
            close_reason: None,
            cancelled_by: None,
        };
        let row = sqlx::query(
            "INSERT INTO task_versions (task_id, version, patient_id, rule_id, rule_version, team, author, status, due_on, body)
             VALUES (gen_random_uuid(), 1, $1::uuid, $2, $3, $4, $5, 'open', $6, $7)
             RETURNING task_id::text AS id",
        )
        .bind(patient.clone())
        .bind(&p.rule)
        .bind(p.version)
        .bind(&p.team)
        .bind(&author)
        .bind(PgDate(p.due_on))
        .bind(serde_json::to_value(&body).map_err(db_err)?)
        .fetch_one(&mut *conn)
        .await
        .map_err(db_err)?;
        let id: String = row.try_get("id").map_err(db_err)?;
        let task = one(&mut *conn, &id).await?;
        audit_row(conn, Action::Create, &author, &task, "made by a rule").await?;
        made.push(task);
    }
    Ok(made)
}

async fn append(
    conn: &mut PgConnection,
    was: &Task,
    status: Status,
    author: &str,
    close_reason: Option<String>,
    cancelled_by: Option<String>,
) -> Result<Option<Task>, StoreError> {
    let body = Body {
        title: was.title.clone(),
        reminders: was.reminders.clone(),
        late_reminder: was.late_reminder,
        cancel_on: Vec::new(),
        source: was.source.clone(),
        close_reason,
        cancelled_by,
    };
    let new_version = crate::store::next_version(was.version)?;
    let appended = crate::store::append_version(
        &mut *conn,
        STORE,
        sqlx::query(
            "INSERT INTO task_versions (task_id, version, patient_id, rule_id, rule_version, team, author, status, due_on, body)
             VALUES ($1::uuid, $2, $3::uuid, $4, $5, $6, $7, $8, $9, $10)
             RETURNING task_id",
        )
        .bind(was.id.clone())
        .bind(new_version)
        .bind(was.patient_id.clone())
        .bind(&was.rule)
        .bind(was.rule_version)
        .bind(&was.team)
        .bind(author)
        .bind(status.word())
        .bind(PgDate(was.due_on))
        .bind(serde_json::to_value(&body).map_err(db_err)?),
    )
    .await?;
    if matches!(appended, crate::store::Appended::Taken) {
        return Ok(None);
    }
    one(&mut *conn, &was.id).await.map(Some)
}

/// The setting naming a site's own task rules file.
pub const RULES_FILE: &str = "EPR_TASK_RULES_FILE";

/// The most bytes a site's rules file may hold: far more than tens of rules need.
pub const MAX_RULES_FILE: u64 = 1_000_000;

/// The rules in force: the site's, kept in the runtime, or the built-in catalogue.
pub enum Rules {
    /// The site's rules file, read at start.
    Site(std::sync::Arc<Vec<Rule>>),
    /// The built-in catalogue.
    Catalogue(&'static [Rule]),
}

impl std::ops::Deref for Rules {
    type Target = [Rule];

    fn deref(&self) -> &[Rule] {
        match self {
            Rules::Site(rules) => rules,
            Rules::Catalogue(rules) => rules,
        }
    }
}

/// The rules that run in the service: the site's file ([`init_rules`]) when one is set, otherwise the built-in
/// catalogue, every rule of which is a draft until approved (OQ-TL1). Only approved rules make tasks.
///
/// # Errors
///
/// [`StoreError::Db`] when the catalogue does not load (a test keeps it valid).
pub fn rules() -> Result<Rules, StoreError> {
    match crate::runtime::Runtime::current().site_rules.get() {
        Some(site) => Ok(Rules::Site(std::sync::Arc::clone(site))),
        None => task_rules::catalogue()
            .map(Rules::Catalogue)
            .map_err(StoreError::Db),
    }
}

/// The rules in the file at `path`, checked as the built-in catalogue is ([`task_rules::parse`]) and for teams that are
/// roles ([`team_problem`]).
///
/// # Errors
///
/// A message naming the setting and the file when it cannot be read, is larger than [`MAX_RULES_FILE`], does not
/// parse or check, or names a team that is not a role.
pub fn load_rules(path: &std::path::Path) -> Result<Vec<Rule>, String> {
    let shown = path.display();
    let size = std::fs::metadata(path)
        .map_err(|e| format!("{RULES_FILE} `{shown}` cannot be read: {e}"))?
        .len();
    if size > MAX_RULES_FILE {
        return Err(format!(
            "{RULES_FILE} `{shown}` is {size} bytes; at most {MAX_RULES_FILE}"
        ));
    }
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("{RULES_FILE} `{shown}` cannot be read: {e}"))?;
    let rules =
        task_rules::parse(&text, &shown.to_string()).map_err(|e| format!("{RULES_FILE}: {e}"))?;
    if let Some(problem) = team_problem(&rules) {
        return Err(format!("{RULES_FILE} `{shown}`: {problem}"));
    }
    Ok(rules)
}

/// Read the rules once, at start-up: the file `EPR_TASK_RULES_FILE` names, which then replaces the built-in catalogue,
/// or, unset or blank, the catalogue. Returns the rules that will run, so the start can log what runs.
///
/// # Errors
///
/// [`load_rules`]' for a file; for the catalogue, a message when it does not load or names a team that is not a role.
pub fn init_rules() -> Result<Rules, String> {
    let setting = crate::runtime::setting(RULES_FILE);
    match setting.as_deref().map(str::trim).filter(|v| !v.is_empty()) {
        Some(path) => {
            let rules = load_rules(std::path::Path::new(path))?;
            let runtime = crate::runtime::Runtime::current();
            let _ = runtime.site_rules.set(std::sync::Arc::new(rules));
            runtime
                .site_rules
                .get()
                .map(|site| Rules::Site(std::sync::Arc::clone(site)))
                .ok_or_else(|| format!("{RULES_FILE}: the rules were not kept"))
        }
        None => {
            let rules = task_rules::catalogue()?;
            if let Some(problem) = team_problem(rules) {
                return Err(problem);
            }
            Ok(Rules::Catalogue(rules))
        }
    }
}

/// The first rule in `rules` whose team is not a role word, as a message: a task for a team nobody can hold would
/// sit on no one's list (OQ-TL3: a team is a role). `None` when every team is a role.
#[must_use]
pub fn team_problem(rules: &[Rule]) -> Option<String> {
    rules
        .iter()
        .find(|r| {
            !crate::policy::Role::ALL
                .iter()
                .any(|role| role.as_str() == r.task.team)
        })
        .map(|r| {
            format!(
                "task rule {} version {}: team `{}` is not a role, so nobody could see its tasks",
                r.id, r.version, r.task.team
            )
        })
}

/// What one event did to tasks.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Raised {
    /// Tasks it made.
    pub made: Vec<Task>,
    /// Tasks it cancelled.
    pub cancelled: Vec<Task>,
}

/// Raise `event` from `source` in the caller's transaction (TL-3, TL-14): first cancel the source's open tasks that the
/// event cancels, then make the tasks the approved `rules` call for, due on the site's working days counted from the
/// site's today ([`crate::site`]). Cancelling first means a new event replaces what an older one asked, and never
/// cancels a task it has just made. If either step fails, the caller's write rolls back with it.
///
/// # Errors
///
/// [`StoreError::Db`], [`StoreError::NotFound`] (a patient id that is not a UUID).
pub async fn raise(
    conn: &mut PgConnection,
    rules: &[Rule],
    event: Event,
    facts: &BTreeMap<String, String>,
    patient_id: &str,
    source: &Source,
) -> Result<Raised, StoreError> {
    let cancelled = cancel_for_event(conn, event, patient_id, source).await?;
    let made = create_from_event(
        conn,
        rules,
        event,
        facts,
        patient_id,
        source,
        crate::site::today(),
        &crate::site::holidays(),
    )
    .await?;
    Ok(Raised { made, cancelled })
}

/// Cancel the open tasks from `source` whose rule is cancelled by `event`, in the caller's transaction, each with an
/// audit row naming the event. Returns what was cancelled.
///
/// # Errors
///
/// [`StoreError::Db`].
pub async fn cancel_for_event(
    conn: &mut PgConnection,
    event: Event,
    patient_id: &str,
    source: &Source,
) -> Result<Vec<Task>, StoreError> {
    let patient = ids::uuid(patient_id).ok_or(StoreError::NotFound)?;
    let rows = sqlx::query(AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM task_current
         WHERE patient_id = $1::uuid AND status = 'open'
           AND body->'source'->>'kind' = $2 AND body->'source'->>'id' = $3
           AND body->'cancel_on' ? $4
         ORDER BY task_id"
    )))
    .bind(patient)
    .bind(&source.kind)
    .bind(&source.id)
    .bind(event.word())
    .fetch_all(&mut *conn)
    .await
    .map_err(db_err)?;
    let mut cancelled = Vec::new();
    for row in &rows {
        let was = row_to_task(row)?;
        let by = format!("event:{}", event.word());
        if let Some(task) = append(
            conn,
            &was,
            Status::Cancelled,
            &by,
            Some(format!("cancelled: {}", event.word())),
            Some(by.clone()),
        )
        .await?
        {
            audit_row(conn, Action::Update, &by, &task, "cancelled by an event").await?;
            cancelled.push(task);
        }
    }
    Ok(cancelled)
}

/// Close (`done`) or cancel a task, by a person, with a reason, at the version they read.
///
/// # Errors
///
/// [`StoreError::NotFound`], [`StoreError::BadQuery`] (no reason, already closed, or `status` is `open`),
/// [`StoreError::Conflict`] (stale version).
pub async fn close(
    db: impl Into<Db<'_>>,
    task_id: &str,
    expected_version: i32,
    status: Status,
    by: &str,
    reason: &str,
) -> Result<Task, StoreError> {
    if status == Status::Open {
        return Err(StoreError::BadQuery(
            "a task is closed as done or cancelled".into(),
        ));
    }
    if reason.trim().is_empty() || reason.chars().count() > 500 {
        return Err(StoreError::BadQuery(
            "a reason is needed, at most 500 characters".into(),
        ));
    }
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let mut tx = sqlx::Connection::begin(&mut *conn).await.map_err(db_err)?;
    let was = one(&mut *tx, task_id).await?;
    if was.status != Status::Open {
        return Err(StoreError::BadQuery("the task is already closed".into()));
    }
    if was.version != expected_version {
        return Err(StoreError::Conflict {
            current: was.version,
        });
    }
    let cancelled_by = (status == Status::Cancelled).then(|| by.to_string());
    let Some(task) = append(
        &mut tx,
        &was,
        status,
        by,
        Some(reason.trim().to_string()),
        cancelled_by,
    )
    .await?
    else {
        let current = one(&mut *tx, task_id).await?.version;
        return Err(StoreError::Conflict { current });
    };
    audit_row(&mut tx, Action::Update, by, &task, "closed by a person").await?;
    tx.commit().await.map_err(db_err)?;
    Ok(task)
}

/// A team's open tasks, due first, up to [`TEAM_LIMIT`]; `more` is whether any were left out.
///
/// # Errors
///
/// [`StoreError::Db`].
pub async fn for_team(db: impl Into<Db<'_>>, team: &str) -> Result<(Vec<Task>, bool), StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let rows = sqlx::query(AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM task_current WHERE team = $1 AND status = 'open'
         ORDER BY due_on, recorded_at, task_id LIMIT $2"
    )))
    .bind(team)
    .bind(TEAM_LIMIT + 1)
    .fetch_all(&mut *conn)
    .await
    .map_err(db_err)?;
    let more = i64::try_from(rows.len()).map_or(true, |n| n > TEAM_LIMIT);
    let tasks = rows
        .iter()
        .take(usize::try_from(TEAM_LIMIT).unwrap_or(200))
        .map(row_to_task)
        .collect::<Result<Vec<_>, _>>()?;
    Ok((tasks, more))
}

/// A reminder that is due a decision today.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Reminder {
    /// The task.
    pub task: Task,
    /// The reminder's day.
    pub on: Date,
    /// What to do with it now.
    pub now: ReminderNow,
}

/// The day the reminder sender last ran, if it ever has.
///
/// # Errors
///
/// [`StoreError::Db`].
pub async fn last_run(db: impl Into<Db<'_>>) -> Result<Option<Date>, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let day: Option<String> =
        sqlx::query_scalar("SELECT max(ran_on)::text FROM task_reminder_runs")
            .fetch_one(&mut *conn)
            .await
            .map_err(db_err)?;
    Ok(day.as_deref().and_then(crate::clock::parse_day))
}

/// Every reminder of an open task whose day is today or past and that has not been decided, with what to do now.
///
/// # Errors
///
/// [`StoreError::Db`].
pub async fn reminders_due(
    db: impl Into<Db<'_>>,
    today: Date,
) -> Result<Vec<Reminder>, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let last = last_run(&mut *conn).await?;
    let rows = sqlx::query(AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM task_current WHERE status = 'open' ORDER BY due_on, task_id"
    )))
    .fetch_all(&mut *conn)
    .await
    .map_err(db_err)?;
    let decided: Vec<(String, String)> =
        sqlx::query_as("SELECT task_id::text, remind_on::text FROM task_reminder_decisions")
            .fetch_all(&mut *conn)
            .await
            .map_err(db_err)?;
    let decided: BTreeSet<(String, String)> = decided.into_iter().collect();
    let mut out = Vec::new();
    for row in &rows {
        let task = row_to_task(row)?;
        for on in task.reminders.iter().copied().filter(|d| *d <= today) {
            if decided.contains(&(task.id.clone(), on.to_string())) {
                continue;
            }
            let now = task_rules::reminder_now(on, today, last, task.late_reminder);
            out.push(Reminder {
                task: task.clone(),
                on,
                now,
            });
        }
    }
    Ok(out)
}

/// The latest day each of `task_ids` was reminded (a reminder decided `sent`), for a list to show. A task never
/// reminded is absent.
///
/// # Errors
///
/// [`StoreError::Db`].
pub async fn reminded_on(
    db: impl Into<Db<'_>>,
    task_ids: &[String],
) -> Result<BTreeMap<String, Date>, StoreError> {
    // Only well-formed ids: one that is not a UUID would fail the cast for the whole list.
    let task_ids: Vec<String> = task_ids.iter().filter_map(|id| ids::uuid(id)).collect();
    if task_ids.is_empty() {
        return Ok(BTreeMap::new());
    }
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT task_id::text, max(remind_on)::text FROM task_reminder_decisions
         WHERE decision = 'sent' AND task_id = ANY($1::text[]::uuid[]) GROUP BY task_id",
    )
    .bind(&task_ids)
    .fetch_all(&mut *conn)
    .await
    .map_err(db_err)?;
    Ok(rows
        .into_iter()
        .filter_map(|(id, day)| crate::clock::parse_day(&day).map(|d| (id, d)))
        .collect())
}

/// Record a reminder's one decision: `None` reason is sent, `Some` is skipped with it. A second decision for the same
/// task and day is refused by the table, so no reminder is sent twice.
///
/// # Errors
///
/// [`StoreError::Conflict`] (already decided), [`StoreError::Db`].
pub async fn decide_reminder(
    db: impl Into<Db<'_>>,
    task_id: &str,
    on: Date,
    skipped: Option<&str>,
) -> Result<(), StoreError> {
    let id = ids::uuid(task_id).ok_or(StoreError::NotFound)?;
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let inserted = sqlx::query(
        "INSERT INTO task_reminder_decisions (task_id, remind_on, decision, reason)
         VALUES ($1::uuid, $2, $3, $4) ON CONFLICT DO NOTHING",
    )
    .bind(id)
    .bind(PgDate(on))
    .bind(if skipped.is_some() { "skipped" } else { "sent" })
    .bind(skipped)
    .execute(&mut *conn)
    .await
    .map_err(db_err)?
    .rows_affected();
    if inserted == 0 {
        return Err(StoreError::Conflict { current: 1 });
    }
    Ok(())
}

/// Record that the reminder sender ran on site day `day`.
///
/// # Errors
///
/// [`StoreError::Db`].
pub async fn record_run(db: impl Into<Db<'_>>, day: Date) -> Result<(), StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    sqlx::query("INSERT INTO task_reminder_runs (ran_on) VALUES ($1)")
        .bind(PgDate(day))
        .execute(&mut *conn)
        .await
        .map_err(db_err)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_site_rules_file_is_checked_like_the_catalogue_and_any_fault_names_the_setting() {
        let dir = std::env::temp_dir().join(format!("epr-task-rules-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = |name: &str, text: &str| {
            let p = dir.join(name);
            std::fs::write(&p, text).unwrap();
            p
        };
        let good = file(
            "good.toml",
            r#"
[[rule]]
id = "site-review"
version = 1
title = "Review"
approved_by = "A Clinician"
approved_on = "2026-11-18"
event = "result-filed"
[rule.task]
team = "clinician"
title = "Review the result"
due_working_days = 1
late_reminder = "skip"
"#,
        );
        let rules = load_rules(&good).unwrap();
        assert_eq!((rules.len(), rules[0].approved()), (1, true));
        let ward = file(
            "ward.toml",
            &std::fs::read_to_string(&good)
                .unwrap()
                .replace("\"clinician\"", "\"ward-7\""),
        );
        let messages = file(
            "message.toml",
            "[[rule]]\nid = \"x\"\naction = \"message-patient\"\n",
        );
        for (path, says) in [
            (dir.join("missing.toml"), "cannot be read"),
            (ward, "ward-7"),
            (messages, "message.toml"),
        ] {
            let e = load_rules(&path).unwrap_err();
            assert!(e.contains(RULES_FILE) && e.contains(says), "{e}");
        }
        let big = file(
            "big.toml",
            &"#".repeat(usize::try_from(MAX_RULES_FILE).unwrap() + 1),
        );
        assert!(load_rules(&big).unwrap_err().contains("at most"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn every_built_in_rule_is_for_a_role_and_a_team_that_is_none_is_named() {
        assert_eq!(team_problem(&rules().unwrap()), None);
        let mut odd = rules().unwrap().to_vec();
        odd[0].task.team = "ward-7".into();
        let p = team_problem(&odd).unwrap();
        assert!(p.contains("ward-7") && p.contains(&odd[0].id), "{p}");
    }
}
