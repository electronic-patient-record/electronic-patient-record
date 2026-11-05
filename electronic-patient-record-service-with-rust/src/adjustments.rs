//! Reasonable adjustments and communication needs (PA-6): the rules, pure.
//!
//! What a person needs in order to use care: an interpreter, large print, step-free access, a quiet place
//! to wait. It comes from a fixed catalogue, with an optional short note and a date by which to check it
//! with them again. The spec is `spec/reasonable-adjustments/index.md`.
//!
//! The codes are this project's own. No national code list is used, because the licence of the likely
//! ones has not been checked, and a mapping is not guessed.
//!
//! **An adjustment informs staff and changes nothing by itself.** Nothing reads one to decide what a
//! person is offered, and a need is never a reason to offer less.
//!
//! Rules: a code must be in the catalogue; a spoken interpreter needs the language in the note; a change
//! needs a reason; a review date is a real day, today or later; ending one is final, and it stays in the
//! history.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Every code: `(category, code, the words)`. The UI holds its own translated words for each, and a test
/// fails if the two lists differ.
pub const CATALOGUE: [(&str, &str, &str); 23] = [
    (
        "interpreting",
        "interpreter-spoken",
        "Spoken language interpreter",
    ),
    (
        "interpreting",
        "interpreter-bsl",
        "British Sign Language interpreter",
    ),
    (
        "interpreting",
        "interpreter-deafblind",
        "Deafblind interpreter",
    ),
    ("communication", "easy-read", "Easy read information"),
    ("communication", "large-print", "Large print"),
    ("communication", "braille", "Braille"),
    ("communication", "audio", "Audio information"),
    ("communication", "hearing-loop", "Hearing loop"),
    (
        "communication",
        "pictorial-pain-scale",
        "Prefers a pictorial or simplified pain scale",
    ),
    (
        "communication",
        "text-not-calls",
        "Contact by text, not phone calls",
    ),
    ("physical-access", "step-free", "Step-free access"),
    (
        "physical-access",
        "wheelchair-space",
        "Space for a wheelchair",
    ),
    ("physical-access", "hoist", "Hoist or transfer equipment"),
    (
        "physical-access",
        "accessible-toilet",
        "Accessible toilet close by",
    ),
    (
        "physical-access",
        "assistance-dog",
        "Assistance dog accompanies the patient",
    ),
    ("environment", "quiet-space", "Quiet space to wait"),
    ("environment", "low-lighting", "Low lighting"),
    ("environment", "no-waiting-room", "Avoid the waiting room"),
    (
        "understanding",
        "extra-time",
        "Extra time to take things in",
    ),
    (
        "understanding",
        "carer-present",
        "A carer or supporter present",
    ),
    ("appointments", "longer-appointment", "A longer appointment"),
    (
        "appointments",
        "first-or-last-slot",
        "First or last appointment of a session",
    ),
    (
        "appointments",
        "same-clinician",
        "The same clinician each time",
    ),
];

/// The most characters a note may hold. It is a short pointer, not a place for a history.
pub const NOTE_LIMIT: usize = 200;

/// The category and words of a code, or `None` when it is not in the catalogue.
#[must_use]
pub fn lookup(code: &str) -> Option<(&'static str, &'static str)> {
    CATALOGUE
        .iter()
        .find(|(_, c, _)| *c == code)
        .map(|(category, _, words)| (*category, *words))
}

/// Whether an adjustment still applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    /// It applies.
    Active,
    /// It no longer applies. Final.
    Ended,
}

impl Status {
    /// Every status, for the census that holds these words to the database's list.
    pub const ALL: [Status; 2] = [Status::Active, Status::Ended];

    /// The stored word.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Status::Active => "active",
            Status::Ended => "ended",
        }
    }
}

/// One adjustment at one version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Adjustment {
    /// A code from [`CATALOGUE`]. It never changes: a different need is a new adjustment.
    pub code: String,
    /// A short note, such as the language for a spoken interpreter.
    pub note: Option<String>,
    /// Whether it still applies.
    pub status: Status,
    /// The day by which to check it with the person again (`YYYY-MM-DD`), if one was set.
    pub review_by: Option<String>,
    /// Why this version was written.
    pub reason: String,
}

/// Why a record or a change was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Refusal {
    /// The code is not in the catalogue.
    UnknownCode,
    /// The reason is blank.
    ReasonRequired,
    /// A spoken interpreter needs the language in the note.
    LanguageRequired,
    /// The note is longer than [`NOTE_LIMIT`].
    NoteTooLong,
    /// The review date is not a real `YYYY-MM-DD` day.
    DateInvalid,
    /// The review date has passed.
    DateInPast,
    /// This adjustment has ended.
    Ended,
    /// This need is already recorded and active.
    AlreadyActive,
}

fn check_review_by(review_by: Option<&str>, today: NaiveDate) -> Result<(), Refusal> {
    let Some(day) = review_by else {
        return Ok(());
    };
    if day.len() != 10 {
        return Err(Refusal::DateInvalid);
    }
    let parsed = NaiveDate::parse_from_str(day, "%Y-%m-%d").map_err(|_| Refusal::DateInvalid)?;
    if parsed < today {
        return Err(Refusal::DateInPast);
    }
    Ok(())
}

fn clean_note(note: Option<&str>) -> Result<Option<String>, Refusal> {
    let note = note.map(str::trim).filter(|n| !n.is_empty());
    if note.is_some_and(|n| n.chars().count() > NOTE_LIMIT) {
        return Err(Refusal::NoteTooLong);
    }
    Ok(note.map(str::to_string))
}

/// A new adjustment, active from the start.
///
/// # Errors
///
/// The [`Refusal`]: a code not in the catalogue, a blank reason, a spoken interpreter with no language, a
/// note too long, or a review date that is not a real day from `today` on.
pub fn record(
    code: &str,
    note: Option<&str>,
    review_by: Option<&str>,
    reason: &str,
    today: NaiveDate,
) -> Result<Adjustment, Refusal> {
    if lookup(code).is_none() {
        return Err(Refusal::UnknownCode);
    }
    if reason.trim().is_empty() {
        return Err(Refusal::ReasonRequired);
    }
    let note = clean_note(note)?;
    if code == "interpreter-spoken" && note.is_none() {
        return Err(Refusal::LanguageRequired);
    }
    check_review_by(review_by, today)?;
    Ok(Adjustment {
        code: code.to_string(),
        note,
        status: Status::Active,
        review_by: review_by.map(str::to_string),
        reason: reason.to_string(),
    })
}

/// A change to an adjustment that stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// It was checked with the person and still applies. `review_by` is the next date, or none.
    Reviewed {
        /// The next date, if one is set.
        review_by: Option<String>,
    },
    /// It no longer applies.
    End,
}

/// The next version of an adjustment.
///
/// # Errors
///
/// The [`Refusal`]: a blank reason, an adjustment already ended, or a review date that is not a real day
/// from `today` on.
pub fn change(
    adjustment: &Adjustment,
    change: &Change,
    reason: &str,
    today: NaiveDate,
) -> Result<Adjustment, Refusal> {
    if reason.trim().is_empty() {
        return Err(Refusal::ReasonRequired);
    }
    if adjustment.status == Status::Ended {
        return Err(Refusal::Ended);
    }
    let mut next = adjustment.clone();
    next.reason = reason.to_string();
    match change {
        Change::Reviewed { review_by } => {
            check_review_by(review_by.as_deref(), today)?;
            next.review_by.clone_from(review_by);
        }
        Change::End => {
            next.status = Status::Ended;
            next.review_by = None;
        }
    }
    Ok(next)
}

/// Whether the date to check this with the person has passed. An ended adjustment is never overdue, and
/// one with no date is not overdue, which says only that no date was set.
#[must_use]
pub fn overdue(adjustment: &Adjustment, today: NaiveDate) -> bool {
    adjustment.status == Status::Active
        && adjustment
            .review_by
            .as_deref()
            .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
            .is_some_and(|d| d < today)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    fn today() -> NaiveDate {
        day("2026-11-05")
    }

    fn large_print() -> Adjustment {
        record(
            "large-print",
            None,
            Some("2027-11-05"),
            "they asked",
            today(),
        )
        .unwrap()
    }

    #[test]
    fn the_stored_word_is_the_serialised_word() {
        for status in Status::ALL {
            assert_eq!(serde_json::to_value(status).unwrap(), status.word());
        }
    }

    #[test]
    fn the_catalogue_has_no_repeated_code() {
        let mut codes: Vec<_> = CATALOGUE.iter().map(|(_, c, _)| *c).collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), CATALOGUE.len());
    }

    #[test]
    fn a_new_adjustment_is_active() {
        let a = large_print();
        assert_eq!(a.status, Status::Active);
        assert_eq!(a.review_by.as_deref(), Some("2027-11-05"));
    }

    #[test]
    fn a_code_must_be_in_the_catalogue() {
        assert_eq!(
            record("high-risk", None, None, "r", today()),
            Err(Refusal::UnknownCode)
        );
    }

    #[test]
    fn a_reason_is_needed_every_time() {
        assert_eq!(
            record("braille", None, None, "  ", today()),
            Err(Refusal::ReasonRequired)
        );
        assert_eq!(
            change(&large_print(), &Change::End, " ", today()),
            Err(Refusal::ReasonRequired)
        );
    }

    #[test]
    fn a_spoken_interpreter_needs_the_language() {
        assert_eq!(
            record("interpreter-spoken", Some("  "), None, "r", today()),
            Err(Refusal::LanguageRequired)
        );
        let a = record("interpreter-spoken", Some(" Polish "), None, "r", today()).unwrap();
        assert_eq!(a.note.as_deref(), Some("Polish"));
    }

    #[test]
    fn a_blank_note_is_no_note() {
        let a = record("braille", Some("  "), None, "r", today()).unwrap();
        assert_eq!(a.note, None);
    }

    #[test]
    fn a_note_is_short() {
        let long = "x".repeat(NOTE_LIMIT + 1);
        assert_eq!(
            record("braille", Some(&long), None, "r", today()),
            Err(Refusal::NoteTooLong)
        );
        assert!(record("braille", Some(&long[1..]), None, "r", today()).is_ok());
    }

    #[test]
    fn a_review_date_is_a_real_day_from_today_on() {
        for bad in ["2027", "2027-02-30", "not a date", "27-01-01", "2027-1-1"] {
            assert_eq!(
                record("braille", None, Some(bad), "r", today()),
                Err(Refusal::DateInvalid),
                "{bad}"
            );
        }
        assert_eq!(
            record("braille", None, Some("2026-11-04"), "r", today()),
            Err(Refusal::DateInPast)
        );
        assert!(record("braille", None, Some("2026-11-05"), "r", today()).is_ok());
    }

    #[test]
    fn a_review_sets_the_next_date_and_keeps_the_rest() {
        let a = large_print();
        let reviewed = change(
            &a,
            &Change::Reviewed {
                review_by: Some("2028-01-01".into()),
            },
            "still needed",
            today(),
        )
        .unwrap();
        assert_eq!(reviewed.review_by.as_deref(), Some("2028-01-01"));
        assert_eq!(reviewed.status, Status::Active);
        assert_eq!(reviewed.code, a.code);
        assert_eq!(reviewed.reason, "still needed");
    }

    #[test]
    fn a_review_cannot_set_a_date_that_has_passed() {
        let refused = change(
            &large_print(),
            &Change::Reviewed {
                review_by: Some("2020-01-01".into()),
            },
            "r",
            today(),
        );
        assert_eq!(refused, Err(Refusal::DateInPast));
    }

    #[test]
    fn ending_is_final_and_clears_the_date() {
        let ended = change(&large_print(), &Change::End, "no longer needed", today()).unwrap();
        assert_eq!(ended.status, Status::Ended);
        assert_eq!(ended.review_by, None);
        for next in [Change::End, Change::Reviewed { review_by: None }] {
            assert_eq!(
                change(&ended, &next, "r", today()),
                Err(Refusal::Ended),
                "{next:?}"
            );
        }
    }

    #[test]
    fn overdue_means_active_with_a_date_that_has_passed() {
        let a = large_print();
        assert!(!overdue(&a, day("2027-11-05")), "the day itself is in time");
        assert!(overdue(&a, day("2027-11-06")));
        let none = record("braille", None, None, "r", today()).unwrap();
        assert!(!overdue(&none, day("2099-01-01")));
        let mut ended = a.clone();
        ended.status = Status::Ended;
        assert!(!overdue(&ended, day("2099-01-01")));
    }
}
