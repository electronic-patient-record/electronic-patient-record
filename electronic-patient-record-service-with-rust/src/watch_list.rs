//! Watch lists (WL-6, `spec/nhs-wales/index.md`): the rules, with no database.
//!
//! A clinician puts a patient on their watch list to keep them in mind: after a worrying result, a
//! discharge, a change in circumstances. The Welsh Clinical Portal's watch lists now record why a
//! patient is on one, and so do these: an entry needs a **reason**, and may carry the day it **ends**.
//! An entry is ended by its owner with a reason, or lapses after its end day; either way it leaves the
//! list and stays in the history, never deleted.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// The longest reason kept.
pub const REASON_LIMIT: usize = 500;

/// A new entry, as sent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewEntry {
    /// Why the patient is on the list.
    pub reason: String,
    /// The last day the entry holds (`YYYY-MM-DD`), if it is not open-ended.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<String>,
}

/// Where an entry stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum State {
    /// On the list.
    Active,
    /// Ended by its owner, with a reason.
    Ended,
    /// Its end day has passed. Worked out when read, never stored.
    Lapsed,
}

impl State {
    /// The words the database stores (`Lapsed` is never stored).
    pub const STORED: [State; 2] = [State::Active, State::Ended];

    /// The word.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            State::Active => "active",
            State::Ended => "ended",
            State::Lapsed => "lapsed",
        }
    }
}

fn reason(out: &mut Vec<String>, what: &str, value: &str) {
    if value.trim().is_empty() {
        out.push(format!("the {what} must not be blank"));
    } else if value.chars().count() > REASON_LIMIT {
        out.push(format!("the {what} is too long"));
    }
}

/// Every problem with a new entry: a blank or long reason, an end day that is not a date or has
/// passed. Words name the field, never quote it.
#[must_use]
pub fn problems(e: &NewEntry, today: NaiveDate) -> Vec<String> {
    let mut out = Vec::new();
    reason(&mut out, "reason", &e.reason);
    if let Some(until) = &e.until {
        match NaiveDate::parse_from_str(until, "%Y-%m-%d") {
            Err(_) => out.push("the end day must be YYYY-MM-DD".into()),
            Ok(d) if d < today => out.push("the end day has passed".into()),
            Ok(_) => {}
        }
    }
    out
}

/// Why ending an entry is refused, if it is: no reason, or a reason too long.
#[must_use]
pub fn end_problems(end_reason: Option<&str>) -> Vec<String> {
    let mut out = Vec::new();
    reason(
        &mut out,
        "reason for ending",
        end_reason.unwrap_or_default(),
    );
    out
}

/// The state of an entry stored as `stored`, with end day `until`, on `today`.
#[must_use]
pub fn state(stored: State, until: Option<&str>, today: NaiveDate) -> State {
    let lapsed = until
        .and_then(|u| NaiveDate::parse_from_str(u, "%Y-%m-%d").ok())
        .is_some_and(|u| u < today);
    match stored {
        State::Active if lapsed => State::Lapsed,
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn an_entry_needs_a_reason_and_a_future_end_if_any() {
        let today = day("2026-11-09");
        let mut e = NewEntry {
            reason: "Low sodium after discharge; recheck".into(),
            until: Some("2026-11-09".into()),
        };
        assert!(problems(&e, today).is_empty(), "today is allowed");
        e.reason = "  ".into();
        e.until = Some("2026-11-08".into());
        assert_eq!(
            problems(&e, today),
            vec!["the reason must not be blank", "the end day has passed"]
        );
        e.reason = "r".into();
        e.until = Some("next week".into());
        assert_eq!(problems(&e, today), vec!["the end day must be YYYY-MM-DD"]);
        assert_eq!(
            end_problems(None),
            vec!["the reason for ending must not be blank"]
        );
        assert!(end_problems(Some("seen in clinic, stable")).is_empty());
    }

    #[test]
    fn an_entry_lapses_the_day_after_its_end() {
        let until = Some("2026-11-09");
        assert_eq!(
            state(State::Active, until, day("2026-11-09")),
            State::Active
        );
        assert_eq!(
            state(State::Active, until, day("2026-11-10")),
            State::Lapsed
        );
        assert_eq!(state(State::Ended, until, day("2026-11-10")), State::Ended);
        assert_eq!(state(State::Active, None, day("2099-01-01")), State::Active);
        let words: Vec<&str> = State::STORED.iter().map(|s| s.word()).collect();
        assert_eq!(words, ["active", "ended"]);
    }
}
