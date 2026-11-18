//! Task rules as data (TL-3, TL-4, TL-14; `spec/lessons/tellescope/index.md`): when something happens to a patient's
//! record, which team gets a task, due when, with which reminders, and what cancels it. Pure: the catalogue, its checks
//! and the arithmetic; the store and the routes are the service's.
//!
//! **A rule creates a task and nothing else.** It never messages a patient and never changes the record: the file's
//! shape has no place for another action, and an unknown key is refused when the file is read, so a rule that tries
//! is a start-up error, not a surprise. A rule with no approver is a draft: [`plan`] shows what it would do (the dry
//! run) and the service creates nothing from it. Who may approve one is open (OQ-TL1), so every rule built in is a
//! draft.
//!
//! **Working days.** A due day counts working days on the site's calendar: Saturday and Sunday are not working days,
//! nor is any day in the site's holiday list. The event's day is the site's day the event fell on, and a task due in 0
//! working days is due that day, or the next working day when that day is not one.
//!
//! **Reminders and cancellation (TL-4).** A rule lists reminders as working days before the due day, and the events
//! that cancel its open tasks (a result signed off, a referral closed), each recorded with the event that cancelled
//! it. When the service was down past a reminder's day, the rule says whether the late reminder is still sent once or
//! dropped with the reason recorded ([`Late`]); never both, never repeated.
//!
//! **No duplicates (TL-14).** Two rules with the same event, conditions and task would make two tasks for one thing;
//! reading such a file fails, naming both.

use jiff::civil::{Date, Weekday};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

/// The catalogue, as built in.
pub const SOURCE: &str = include_str!("task_rules.toml");

/// The longest due period, in working days.
pub const MAX_DUE_DAYS: u32 = 60;

/// The most reminders one rule may set.
pub const MAX_REMINDERS: usize = 5;

/// What happened, that a rule can start from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Event {
    /// A result was filed for the patient.
    ResultFiled,
    /// A result was signed off.
    ResultSignedOff,
    /// A result filed earlier was marked entered in error.
    ResultEnteredInError,
    /// An observation was recorded outside its range.
    ObservationOutOfRange,
    /// An observation was entered in error.
    ObservationEnteredInError,
    /// A referral moved to a new state.
    ReferralStepped,
    /// A referral was closed (completed, declined, cancelled).
    ReferralClosed,
}

impl Event {
    /// Every event, in order.
    pub const ALL: [Event; 7] = [
        Event::ResultFiled,
        Event::ResultSignedOff,
        Event::ResultEnteredInError,
        Event::ObservationOutOfRange,
        Event::ObservationEnteredInError,
        Event::ReferralStepped,
        Event::ReferralClosed,
    ];

    /// The facts an event carries, which a condition may name. A condition on any other fact is refused when the
    /// file is read: a typo must not make a rule match everything, or nothing, in silence.
    #[must_use]
    pub fn facts(self) -> &'static [&'static str] {
        match self {
            Event::ResultFiled | Event::ResultSignedOff | Event::ResultEnteredInError => {
                &["abnormal", "test"]
            }
            Event::ObservationOutOfRange | Event::ObservationEnteredInError => &["code", "flag"],
            Event::ReferralStepped => &["state", "priority"],
            Event::ReferralClosed => &["state"],
        }
    }

    /// The kebab-case word.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Event::ResultFiled => "result-filed",
            Event::ResultSignedOff => "result-signed-off",
            Event::ResultEnteredInError => "result-entered-in-error",
            Event::ObservationOutOfRange => "observation-out-of-range",
            Event::ObservationEnteredInError => "observation-entered-in-error",
            Event::ReferralStepped => "referral-stepped",
            Event::ReferralClosed => "referral-closed",
        }
    }
}

/// A condition on one fact of the event: it holds when the fact is one of the values.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Condition {
    /// The fact, one of the event's [`Event::facts`].
    pub fact: String,
    /// The values that match; at least one.
    pub any_of: Vec<String>,
}

/// What a late reminder does: one whose day passed while nothing could send it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Late {
    /// Send it once, as soon as possible.
    SendOnce,
    /// Drop it, recording that its day passed.
    Skip,
}

/// The one thing a rule does: a task for a team.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskSpec {
    /// The team: a role word (`clinician`, `nurse`, `clinical-admin`, ...), checked by the service against its roles.
    pub team: String,
    /// What to do, in words a person reads.
    pub title: String,
    /// Working days from the event's day to the due day, 0 to [`MAX_DUE_DAYS`].
    pub due_working_days: u32,
    /// Reminders, as working days before the due day; each at most the due period, none repeated.
    #[serde(default)]
    pub remind_working_days_before: Vec<u32>,
    /// What a reminder whose day passed does.
    pub late_reminder: Late,
}

/// One version of one rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    /// The rule.
    pub id: String,
    /// Its version, from 1.
    pub version: i32,
    /// What it is for.
    pub title: String,
    /// The approver; empty for a draft.
    #[serde(default)]
    pub approved_by: String,
    /// The day it was approved, `YYYY-MM-DD`; empty for a draft.
    #[serde(default)]
    pub approved_on: String,
    /// What starts it.
    pub event: Event,
    /// Every condition must hold; none means every such event.
    #[serde(default, rename = "when")]
    pub conditions: Vec<Condition>,
    /// The task it creates.
    pub task: TaskSpec,
    /// Events on the same subject that cancel its open tasks.
    #[serde(default)]
    pub cancel_on: Vec<Event>,
}

impl Rule {
    /// Whether a person has approved this version.
    #[must_use]
    pub fn approved(&self) -> bool {
        !self.approved_by.trim().is_empty()
    }

    /// Whether `event` with `facts` starts this rule.
    #[must_use]
    pub fn matches(&self, event: Event, facts: &BTreeMap<String, String>) -> bool {
        self.event == event
            && self
                .conditions
                .iter()
                .all(|c| facts.get(&c.fact).is_some_and(|v| c.any_of.contains(v)))
    }

    /// What makes two rules the same work: event, conditions (in any order, values in any order) and task.
    fn identity(&self) -> (Event, Vec<(String, Vec<String>)>, TaskSpec) {
        let mut conditions: Vec<(String, Vec<String>)> = self
            .conditions
            .iter()
            .map(|c| {
                let mut v = c.any_of.clone();
                v.sort();
                v.dedup();
                (c.fact.clone(), v)
            })
            .collect();
        conditions.sort();
        let mut task = self.task.clone();
        task.remind_working_days_before.sort_unstable();
        (self.event, conditions, task)
    }
}

/// The catalogue file's shape.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    #[serde(default, rename = "rule")]
    rules: Vec<Rule>,
}

/// Read and check a catalogue: what [`SOURCE`] must satisfy.
///
/// # Errors
///
/// A sentence naming the rule for: an unknown key or action (the only action is a task); a version run that is not
/// 1, 2, 3 ...; an approver without a day or a day without an approver; a condition on a fact the event does not carry,
/// or with no values; a due period over [`MAX_DUE_DAYS`]; a reminder after the due period, repeated, or more than
/// [`MAX_REMINDERS`]; a rule cancelled by its own event; an empty team or title; two rules that are the same work.
pub fn parse(text: &str, name: &str) -> Result<Vec<Rule>, String> {
    let file: File = toml::from_str(text).map_err(|e| format!("task rules: {name}: {e}"))?;
    let rules = file.rules;
    let mut versions: BTreeMap<&str, Vec<i32>> = BTreeMap::new();
    for r in &rules {
        let at = format!("task rules: {name}: rule {} version {}", r.id, r.version);
        if r.id.trim().is_empty() || r.title.trim().is_empty() {
            return Err(format!("{at}: needs an id and a title"));
        }
        if r.task.team.trim().is_empty() || r.task.title.trim().is_empty() {
            return Err(format!("{at}: the task needs a team and a title"));
        }
        let by = r.approved_by.trim().is_empty();
        let on = r.approved_on.trim();
        if by != on.is_empty() || (!on.is_empty() && crate::clock::parse_day(on).is_none()) {
            return Err(format!(
                "{at}: needs both an approver and a YYYY-MM-DD day, or neither"
            ));
        }
        let facts = r.event.facts();
        for c in &r.conditions {
            if !facts.contains(&c.fact.as_str()) {
                return Err(format!(
                    "{at}: {} carries no fact {} (it carries {})",
                    r.event.word(),
                    c.fact,
                    facts.join(", ")
                ));
            }
            if c.any_of.is_empty() || c.any_of.iter().any(|v| v.trim().is_empty()) {
                return Err(format!("{at}: the condition on {} needs values", c.fact));
            }
        }
        if r.task.due_working_days > MAX_DUE_DAYS {
            return Err(format!(
                "{at}: due in {} working days; at most {MAX_DUE_DAYS}",
                r.task.due_working_days
            ));
        }
        let reminders = &r.task.remind_working_days_before;
        let distinct: BTreeSet<u32> = reminders.iter().copied().collect();
        if reminders.len() > MAX_REMINDERS
            || distinct.len() != reminders.len()
            || reminders.iter().any(|d| *d > r.task.due_working_days)
        {
            return Err(format!(
                "{at}: reminders must be distinct, at most {MAX_REMINDERS}, and within the due period"
            ));
        }
        if r.cancel_on.contains(&r.event) {
            return Err(format!("{at}: is cancelled by the event that starts it"));
        }
        versions.entry(r.id.as_str()).or_default().push(r.version);
    }
    for (id, mut v) in versions {
        v.sort_unstable();
        if v.iter()
            .enumerate()
            .any(|(i, n)| i64::from(*n) != i as i64 + 1)
        {
            return Err(format!(
                "task rules: {name}: rule {id}: versions must run 1, 2, 3 ... with none missing or repeated"
            ));
        }
    }
    // TL-14: the same work twice. Versions of one rule are its history, so only each rule's latest is compared.
    let latest: Vec<&Rule> = latest_versions(&rules);
    for (i, a) in latest.iter().enumerate() {
        for b in &latest[i + 1..] {
            if a.identity() == b.identity() {
                return Err(format!(
                    "task rules: {name}: rules {} and {} are the same event, conditions and task",
                    a.id, b.id
                ));
            }
        }
    }
    Ok(rules)
}

fn latest_versions(rules: &[Rule]) -> Vec<&Rule> {
    let mut latest: BTreeMap<&str, &Rule> = BTreeMap::new();
    for r in rules {
        let slot = latest.entry(r.id.as_str()).or_insert(r);
        if r.version > slot.version {
            *slot = r;
        }
    }
    latest.into_values().collect()
}

static CATALOGUE: LazyLock<Result<Vec<Rule>, String>> =
    LazyLock::new(|| parse(SOURCE, "task_rules.toml"));

/// The built-in catalogue, checked once.
///
/// # Errors
///
/// [`parse`]'s, for the built-in file (a test keeps it valid).
pub fn catalogue() -> Result<&'static [Rule], String> {
    CATALOGUE.as_deref().map_err(Clone::clone)
}

/// Whether `day` is a working day: not Saturday or Sunday, and not in `holidays`.
#[must_use]
pub fn working_day(day: Date, holidays: &BTreeSet<Date>) -> bool {
    !matches!(day.weekday(), Weekday::Saturday | Weekday::Sunday) && !holidays.contains(&day)
}

/// `days` working days after `from`. 0 is `from` itself when it is a working day, else the next working day; 1 is the
/// next working day after that, and so on. Saturating at the calendar's end.
#[must_use]
pub fn add_working_days(from: Date, days: u32, holidays: &BTreeSet<Date>) -> Date {
    let next = |d: Date| d.tomorrow().unwrap_or(d);
    let mut day = from;
    // At most a long holiday run past any start: bounded so a calendar of all holidays cannot loop for ever.
    let mut guard = 0;
    while !working_day(day, holidays) && guard < 366 {
        day = next(day);
        guard += 1;
    }
    for _ in 0..days {
        day = next(day);
        let mut guard = 0;
        while !working_day(day, holidays) && guard < 366 {
            day = next(day);
            guard += 1;
        }
    }
    day
}

/// `days` working days before `due`, never before `from` (a reminder is not set in the past of the task).
#[must_use]
pub fn sub_working_days(due: Date, days: u32, from: Date, holidays: &BTreeSet<Date>) -> Date {
    let mut day = due;
    let mut left = days;
    while left > 0 && day > from {
        day = day.yesterday().unwrap_or(day);
        if working_day(day, holidays) {
            left -= 1;
        }
    }
    day.max(from)
}

/// A task a rule would create.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Planned {
    /// The rule.
    pub rule: String,
    /// Its version.
    pub version: i32,
    /// Whether that version is approved: a draft's task is shown, never created.
    pub approved: bool,
    /// The team.
    pub team: String,
    /// What to do.
    pub title: String,
    /// The due day.
    pub due_on: Date,
    /// The reminder days, earliest first, none repeated.
    pub reminders: Vec<Date>,
    /// What a late reminder does.
    pub late_reminder: Late,
    /// The events that cancel it.
    pub cancel_on: Vec<Event>,
}

/// What `event`, with `facts`, on site day `day`, would create under the latest version of each rule in `rules`,
/// approved or not (the caller creates only the approved ones). Pure, so it is the dry run too.
#[must_use]
pub fn plan(
    rules: &[Rule],
    event: Event,
    facts: &BTreeMap<String, String>,
    day: Date,
    holidays: &BTreeSet<Date>,
) -> Vec<Planned> {
    latest_versions(rules)
        .into_iter()
        .filter(|r| r.matches(event, facts))
        .map(|r| {
            let due_on = add_working_days(day, r.task.due_working_days, holidays);
            let mut reminders: Vec<Date> = r
                .task
                .remind_working_days_before
                .iter()
                .map(|d| sub_working_days(due_on, *d, day, holidays))
                .collect();
            reminders.sort();
            reminders.dedup();
            Planned {
                rule: r.id.clone(),
                version: r.version,
                approved: r.approved(),
                team: r.task.team.clone(),
                title: r.task.title.clone(),
                due_on,
                reminders,
                late_reminder: r.task.late_reminder,
                cancel_on: r.cancel_on.clone(),
            }
        })
        .collect()
}

/// What to do with one reminder now (TL-4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "decision")]
pub enum ReminderNow {
    /// Not yet its day.
    Wait,
    /// Send it now.
    Send,
    /// Its day passed and the rule says skip: drop it, recording why.
    Skip {
        /// Why, in words.
        reason: String,
    },
}

/// Whether a reminder due on `on` is sent `today`, given the day its sender last ran (`last_run`): a reminder whose day
/// fell after the last run and before today was missed while nothing ran, and is late.
#[must_use]
pub fn reminder_now(on: Date, today: Date, last_run: Option<Date>, late: Late) -> ReminderNow {
    if on > today {
        return ReminderNow::Wait;
    }
    let missed = on < today && last_run.is_none_or(|run| run < on);
    match (missed, late) {
        (true, Late::Skip) => ReminderNow::Skip {
            reason: format!("its day ({on}) passed while reminders were not running"),
        },
        _ => ReminderNow::Send,
    }
}

/// Whether `event` cancels an open task made by `planned`'s rule.
#[must_use]
pub fn cancels(cancel_on: &[Event], event: Event) -> bool {
    cancel_on.contains(&event)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Date {
        s.parse().unwrap()
    }

    fn facts(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    const GOOD: &str = r#"
[[rule]]
id = "abnormal-result"
version = 1
title = "An abnormal result needs a clinician"
event = "result-filed"
cancel_on = ["result-signed-off"]
[[rule.when]]
fact = "abnormal"
any_of = ["true"]
[rule.task]
team = "clinician"
title = "Review the abnormal result"
due_working_days = 2
remind_working_days_before = [1]
late_reminder = "send-once"
"#;

    #[test]
    fn the_built_in_catalogue_reads_and_every_rule_in_it_is_a_draft() {
        let rules = catalogue().unwrap();
        assert!(!rules.is_empty(), "a draft example ships, for the dry run");
        assert!(rules.iter().all(|r| !r.approved()), "OQ-TL1: none approved");
    }

    #[test]
    fn a_rule_that_tries_anything_but_a_task_is_refused_when_read() {
        for extra in [
            "\n[rule.message]\nto = \"patient\"\n",
            "\n[rule.update]\nfield = \"status\"\n",
            "\nsend_sms = true\n",
        ] {
            let text = GOOD.replacen("[rule.task]", &format!("{extra}[rule.task]"), 1);
            assert!(parse(&text, "t").is_err(), "{extra}");
        }
        // An unknown key inside the task is refused too.
        let text = GOOD.replace(
            "late_reminder = \"send-once\"",
            "late_reminder = \"send-once\"\nnotify_patient = true",
        );
        assert!(parse(&text, "t").is_err());
        assert_eq!(parse(GOOD, "t").unwrap().len(), 1);
    }

    #[test]
    fn a_condition_on_a_fact_the_event_lacks_or_with_no_values_is_refused() {
        let e = parse(
            &GOOD.replace("fact = \"abnormal\"", "fact = \"abnormall\""),
            "t",
        )
        .unwrap_err();
        assert!(
            e.contains("carries no fact abnormall") && e.contains("abnormal, test"),
            "{e}"
        );
        assert!(parse(&GOOD.replace("any_of = [\"true\"]", "any_of = []"), "t").is_err());
    }

    #[test]
    fn approval_due_period_reminders_and_self_cancellation_are_checked() {
        for (why, from, to) in [
            (
                "approver without a day",
                "title = \"An abnormal",
                "approved_by = \"A Person\"\ntitle = \"An abnormal",
            ),
            (
                "a day that is not a day",
                "title = \"An abnormal",
                "approved_by = \"A\"\napproved_on = \"2026-02-30\"\ntitle = \"An abnormal",
            ),
            ("too long", "due_working_days = 2", "due_working_days = 61"),
            (
                "reminder after the due period",
                "remind_working_days_before = [1]",
                "remind_working_days_before = [3]",
            ),
            (
                "reminder repeated",
                "remind_working_days_before = [1]",
                "remind_working_days_before = [1, 1]",
            ),
            (
                "cancelled by its own event",
                "cancel_on = [\"result-signed-off\"]",
                "cancel_on = [\"result-filed\"]",
            ),
            ("no team", "team = \"clinician\"", "team = \"\""),
        ] {
            assert!(parse(&GOOD.replace(from, to), "t").is_err(), "{why}");
        }
        let approved = GOOD.replace(
            "title = \"An abnormal",
            "approved_by = \"A Person\"\napproved_on = \"2026-11-17\"\ntitle = \"An abnormal",
        );
        assert!(parse(&approved, "t").unwrap()[0].approved());
    }

    #[test]
    fn versions_run_from_one_with_none_missing() {
        let two = format!("{GOOD}\n{}", GOOD.replace("version = 1", "version = 3"));
        assert!(parse(&two, "t").unwrap_err().contains("versions must run"));
        let next = format!("{GOOD}\n{}", GOOD.replace("version = 1", "version = 2"));
        assert_eq!(
            parse(&next, "t").unwrap().len(),
            2,
            "versions of one rule are its history, not duplicates"
        );
    }

    #[test]
    fn two_rules_that_are_the_same_work_are_refused_naming_both() {
        let twin = GOOD.replace("id = \"abnormal-result\"", "id = \"abnormal-again\"");
        let e = parse(&format!("{GOOD}\n{twin}"), "t").unwrap_err();
        assert!(
            e.contains("abnormal-again") && e.contains("abnormal-result"),
            "{e}"
        );
        // Conditions in another order or with values reordered are still the same work.
        let shuffled = twin.replace("any_of = [\"true\"]", "any_of = [\"true\", \"true\"]");
        assert!(parse(&format!("{GOOD}\n{shuffled}"), "t").is_err());
        // Another team is other work.
        let other = twin.replace("team = \"clinician\"", "team = \"nurse\"");
        assert!(parse(&format!("{GOOD}\n{other}"), "t").is_ok());
    }

    #[test]
    fn working_days_skip_weekends_and_holidays() {
        let none = BTreeSet::new();
        // Friday 2026-11-20 plus one working day is Monday the 23rd; plus zero is itself.
        assert_eq!(add_working_days(d("2026-11-20"), 1, &none), d("2026-11-23"));
        assert_eq!(add_working_days(d("2026-11-20"), 0, &none), d("2026-11-20"));
        // An event on Saturday is counted from Monday.
        assert_eq!(add_working_days(d("2026-11-21"), 0, &none), d("2026-11-23"));
        assert_eq!(add_working_days(d("2026-11-21"), 2, &none), d("2026-11-25"));
        // Christmas and Boxing Day as holidays: Thursday the 24th plus one is Monday the 28th.
        let holidays: BTreeSet<Date> = [d("2026-12-25")].into_iter().collect();
        assert_eq!(
            add_working_days(d("2026-12-24"), 1, &holidays),
            d("2026-12-28")
        );
        // A reminder one working day before Monday's due day is the Friday, never before the event's day.
        assert_eq!(
            sub_working_days(d("2026-11-23"), 1, d("2026-11-19"), &none),
            d("2026-11-20")
        );
        assert_eq!(
            sub_working_days(d("2026-11-23"), 5, d("2026-11-20"), &none),
            d("2026-11-20")
        );
    }

    #[test]
    fn the_plan_is_the_latest_version_of_each_matching_rule_with_its_dates() {
        let rules = parse(GOOD, "t").unwrap();
        let none = BTreeSet::new();
        let p = plan(
            &rules,
            Event::ResultFiled,
            &facts(&[("abnormal", "true")]),
            d("2026-11-20"),
            &none,
        );
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].due_on, d("2026-11-24"));
        assert_eq!(p[0].reminders, vec![d("2026-11-23")]);
        assert!(!p[0].approved);
        assert!(
            plan(
                &rules,
                Event::ResultFiled,
                &facts(&[("abnormal", "false")]),
                d("2026-11-20"),
                &none
            )
            .is_empty()
        );
        assert!(
            plan(
                &rules,
                Event::ResultFiled,
                &facts(&[]),
                d("2026-11-20"),
                &none
            )
            .is_empty(),
            "a missing fact does not match"
        );
        assert!(
            plan(
                &rules,
                Event::ObservationOutOfRange,
                &facts(&[("abnormal", "true")]),
                d("2026-11-20"),
                &none
            )
            .is_empty()
        );
        let newer = format!(
            "{GOOD}\n{}",
            GOOD.replace("version = 1", "version = 2")
                .replace("due_working_days = 2", "due_working_days = 5")
        );
        let p = plan(
            &parse(&newer, "t").unwrap(),
            Event::ResultFiled,
            &facts(&[("abnormal", "true")]),
            d("2026-11-20"),
            &none,
        );
        assert_eq!(
            (p.len(), p[0].version, p[0].due_on),
            (1, 2, d("2026-11-27"))
        );
    }

    #[test]
    fn a_late_reminder_is_sent_once_or_skipped_with_a_reason_never_both() {
        let on = d("2026-11-20");
        assert_eq!(
            reminder_now(on, d("2026-11-19"), None, Late::Skip),
            ReminderNow::Wait
        );
        assert_eq!(
            reminder_now(on, on, Some(d("2026-11-19")), Late::Skip),
            ReminderNow::Send,
            "on its day"
        );
        // The sender last ran before the reminder's day and today is after it: missed.
        let skipped = reminder_now(on, d("2026-11-24"), Some(d("2026-11-19")), Late::Skip);
        assert!(
            matches!(&skipped, ReminderNow::Skip { reason } if reason.contains("2026-11-20")),
            "{skipped:?}"
        );
        assert_eq!(
            reminder_now(on, d("2026-11-24"), Some(d("2026-11-19")), Late::SendOnce),
            ReminderNow::Send
        );
        // Run on its day, so not missed.
        assert_eq!(
            reminder_now(on, d("2026-11-24"), Some(on), Late::Skip),
            ReminderNow::Send
        );
        assert!(cancels(&[Event::ResultSignedOff], Event::ResultSignedOff));
        assert!(!cancels(&[Event::ResultSignedOff], Event::ResultFiled));
    }

    // Pins from the 2026-11-17 mutation run (docs/maintainability/mutation-testing.md).
    #[test]
    fn each_event_word_is_its_serde_name() {
        for e in Event::ALL {
            let json = serde_json::to_string(&e).unwrap();
            assert_eq!(json, format!("\"{}\"", e.word()));
        }
    }

    #[test]
    fn an_empty_id_or_title_alone_is_refused() {
        assert!(parse(&GOOD.replace("id = \"abnormal-result\"", "id = \"\""), "t").is_err());
        assert!(
            parse(
                &GOOD.replace(
                    "title = \"An abnormal result needs a clinician\"",
                    "title = \" \""
                ),
                "t"
            )
            .is_err()
        );
    }

    #[test]
    fn the_limits_are_inclusive() {
        let at_most = GOOD.replace("due_working_days = 2", "due_working_days = 60");
        assert!(parse(&at_most, "t").is_ok(), "60 working days is allowed");
        let five = at_most.replace(
            "remind_working_days_before = [1]",
            "remind_working_days_before = [1, 2, 3, 4, 5]",
        );
        assert!(parse(&five, "t").is_ok(), "five reminders are allowed");
        let six = at_most.replace(
            "remind_working_days_before = [1]",
            "remind_working_days_before = [1, 2, 3, 4, 5, 6]",
        );
        assert!(parse(&six, "t").is_err(), "six are not");
        let on_the_day = GOOD.replace(
            "remind_working_days_before = [1]",
            "remind_working_days_before = [2]",
        );
        assert!(
            parse(&on_the_day, "t").is_ok(),
            "a reminder the whole due period before is allowed"
        );
    }

    #[test]
    fn a_calendar_of_nothing_but_holidays_still_ends() {
        let start = d("2026-01-01");
        let mut holidays = BTreeSet::new();
        let mut day = start;
        // Longer than the guard's reach (366 days at the start and after each working day counted), so the bound shows:
        // without the guard the answer would be the first working day after the run, years later.
        for _ in 0..1500 {
            holidays.insert(day);
            day = day.tomorrow().unwrap();
        }
        // Bounded: the answer is some day within the guard, not a loop for ever.
        let due = add_working_days(start, 2, &holidays);
        assert!(due > start && due < d("2029-02-01"), "{due}");
    }
}
