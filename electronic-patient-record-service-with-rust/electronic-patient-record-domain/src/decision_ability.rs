//! Whether a person can make one decision, now, as judged by one person (RF-8; `spec/patient-refusal-of-consent`).
//!
//! **There is no score and no verdict here.** A judgement is about one decision at one time, by someone named, with their
//! reasons in words. Nothing here answers whether a person "has capacity": [`about_this`] lists the judgements about a
//! decision, newest first, and never merges them; [`disputed`] says when the latest judgements of two people disagree;
//! [`gaps`] says what a judgement does not record. A judgement is **never copied forward**: there is no function that
//! makes a new one from an old one, and [`age_minutes`] lets a screen show how old it is.
//!
//! - **"Not known" is always a value** and decides nothing.
//! - **Help given is recorded**, because a person is not to be treated as unable to decide until what can be done to help
//!   them has been tried (England and Wales, Mental Capacity Act 2005 s.1(3), as read in the spec; other places differ and
//!   that is a pack's to say).

#![deny(
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::string_slice
)]

use serde::{Deserialize, Serialize};

/// The longest words of a judgement's decision, reasons, help or author.
pub const WORDS_LIMIT: usize = 500;

/// Yes, no, or not known.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Known {
    /// Yes.
    Yes,
    /// No.
    No,
    /// Not known.
    NotKnown,
}

/// How the person communicates, if they do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Communicates {
    /// By speech, in a language the judge shares or through an interpreter.
    Speaks,
    /// By other means: writing, a letter board, blinking, gestures. An intubated person may.
    OtherMeans,
    /// Not at all, at the time.
    No,
    /// Not known.
    NotKnown,
}

/// What the judge concluded about this one decision, now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Decides {
    /// They can make this decision now.
    Can,
    /// They cannot make this decision now.
    Cannot,
    /// The judge could not tell.
    NotKnown,
}

/// A kind of help given before the judgement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Help {
    /// An interpreter, in person or remote.
    Interpreter,
    /// A communication aid: a board, a device, writing.
    CommunicationAid,
    /// More time, or another time of day.
    Time,
    /// A quieter place, or fewer people.
    Place,
    /// Someone the person trusts, or an advocate.
    Supporter,
    /// Information in another form: easy read, pictures.
    Information,
    /// Something else, said in words.
    Other,
}

/// One judgement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Judgement {
    /// The decision it is about, in words ("whether to accept blood"). A judgement is about one decision.
    pub about: String,
    /// When it was made (RFC 3339).
    pub at: String,
    /// Whether they were conscious.
    pub conscious: Known,
    /// How they communicated.
    pub communicates: Communicates,
    /// What the judge concluded.
    pub decides: Decides,
    /// What help was given first. Empty is "none recorded", never "none was needed".
    #[serde(default)]
    pub help_given: Vec<Help>,
    /// The help in words. Required when `help_given` holds `other`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help_words: Option<String>,
    /// Why, in the judge's words. Required.
    pub because: String,
    /// Who judged, in words: a name and a role.
    pub by: String,
}

fn words(out: &mut Vec<String>, what: &str, value: &str) {
    if value.trim().is_empty() {
        out.push(format!("{what} must not be blank"));
    } else if value.chars().count() > WORDS_LIMIT {
        out.push(format!("{what} must be up to {WORDS_LIMIT} characters"));
    }
}

/// What is wrong with a judgement. The words name a field and never quote it. Whether the judgement is right is not asked.
#[must_use]
pub fn validate(j: &Judgement, now: jiff::Timestamp) -> Vec<String> {
    let mut out = Vec::new();
    words(&mut out, "about", &j.about);
    words(&mut out, "because", &j.because);
    words(&mut out, "by", &j.by);
    match crate::clock::parse_rfc3339(&j.at) {
        None => out.push("at must be an RFC 3339 time".into()),
        Some(at) if at > now => out.push("at must not be in the future".into()),
        Some(_) => {}
    }
    let other = j.help_given.contains(&Help::Other);
    match (&j.help_words, other) {
        (None, true) => out.push("help_words is required when the help is other".into()),
        (Some(w), _) => words(&mut out, "help_words", w),
        (None, false) => {}
    }
    let mut seen = j.help_given.clone();
    seen.sort();
    seen.dedup();
    if seen.len() != j.help_given.len() {
        out.push("help_given names a kind of help twice".into());
    }
    out
}

/// What a judgement does not record that a reader would want to know. **Not a verdict**: a judgement with gaps stands as
/// recorded, and one with none is not thereby right.
#[must_use]
pub fn gaps(j: &Judgement) -> Vec<&'static str> {
    let mut out = Vec::new();
    if j.decides == Decides::Cannot && j.help_given.is_empty() {
        out.push("no help is recorded before a judgement that they cannot decide");
    }
    if j.decides == Decides::Cannot
        && j.communicates == Communicates::OtherMeans
        && !j.help_given.contains(&Help::CommunicationAid)
    {
        out.push("they communicate by other means and no communication aid is recorded");
    }
    if j.conscious == Known::NotKnown {
        out.push("whether they were conscious is not known");
    }
    out
}

/// How many whole minutes old a judgement is at `now`, for a screen to show. `None` when its time cannot be read.
#[must_use]
pub fn age_minutes(j: &Judgement, now: jiff::Timestamp) -> Option<i64> {
    let at = crate::clock::parse_rfc3339(&j.at)?;
    let seconds = now.as_second().checked_sub(at.as_second())?;
    Some(seconds.div_euclid(60))
}

/// The same decision, by its words: trimmed, case and inner spacing ignored. Two decisions said differently are two.
fn same_decision(a: &str, b: &str) -> bool {
    let norm = |s: &str| {
        s.split_whitespace()
            .map(str::to_lowercase)
            .collect::<Vec<_>>()
    };
    norm(a) == norm(b)
}

/// The judgements about `about`, newest first. Each is returned as it was made: nothing is merged, carried or worked out.
#[must_use]
pub fn about_this<'a>(all: &'a [Judgement], about: &str) -> Vec<&'a Judgement> {
    let mut found: Vec<(&Judgement, jiff::Timestamp)> = all
        .iter()
        .filter(|j| same_decision(&j.about, about))
        .filter_map(|j| crate::clock::parse_rfc3339(&j.at).map(|at| (j, at)))
        .collect();
    found.sort_by_key(|f| std::cmp::Reverse(f.1));
    found.into_iter().map(|(j, _)| j).collect()
}

/// Whether the latest judgements of two or more people about one decision disagree on whether the person can decide.
/// "Not known" disagrees with neither. Older judgements by the same person do not count: a person who changed their view
/// is not in dispute with themselves.
#[must_use]
pub fn disputed(about: &[&Judgement]) -> bool {
    let mut latest: Vec<(&str, Decides, jiff::Timestamp)> = Vec::new();
    for j in about {
        let Some(at) = crate::clock::parse_rfc3339(&j.at) else {
            continue;
        };
        let by = j.by.trim();
        match latest.iter_mut().find(|(b, ..)| b.eq_ignore_ascii_case(by)) {
            Some(entry) if entry.2 < at => *entry = (by, j.decides, at),
            Some(_) => {}
            None => latest.push((by, j.decides, at)),
        }
    }
    let said = |d: Decides| latest.iter().any(|(_, x, _)| *x == d);
    said(Decides::Can) && said(Decides::Cannot)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> jiff::Timestamp {
        crate::clock::parse_rfc3339("2026-11-26T12:00:00Z").unwrap()
    }

    fn judged(at: &str, by: &str, decides: Decides) -> Judgement {
        Judgement {
            about: "whether to accept blood".into(),
            at: at.into(),
            conscious: Known::Yes,
            communicates: Communicates::Speaks,
            decides,
            help_given: vec![Help::Time],
            help_words: None,
            because: "explained the risks back in their own words".into(),
            by: by.into(),
        }
    }

    #[test]
    fn a_fluctuating_person_has_each_judgement_as_made_and_the_newest_is_not_carried_back() {
        // Lucid in the morning, delirious at night: two judgements, both kept, the newest first, neither merged.
        let morning = judged("2026-11-26T09:00:00Z", "Dr A, registrar", Decides::Can);
        let night = Judgement {
            communicates: Communicates::No,
            because: "drowsy and could not follow the question".into(),
            ..judged("2026-11-26T02:00:00Z", "Dr A, registrar", Decides::Cannot)
        };
        let all = [night.clone(), morning.clone()];
        let found = about_this(&all, "Whether to accept  BLOOD ");
        assert_eq!(found, vec![&morning, &night]);
        // One person who changed their view is not a dispute.
        assert!(!disputed(&found));
        // The morning judgement is three hours old at noon: a screen shows it so, and it is never renewed here.
        assert_eq!(age_minutes(&morning, now()), Some(180));
        // Another decision is another list.
        assert!(about_this(&all, "whether to have surgery").is_empty());
    }

    #[test]
    fn an_intubated_person_who_communicates_by_other_means_is_not_recorded_as_unable_to_communicate()
     {
        let intubated = Judgement {
            communicates: Communicates::OtherMeans,
            help_given: vec![Help::CommunicationAid],
            because: "answered each question on a letter board, consistently".into(),
            ..judged("2026-11-26T10:00:00Z", "Nurse B", Decides::Can)
        };
        assert!(validate(&intubated, now()).is_empty());
        assert!(gaps(&intubated).is_empty());
        // Judged unable, by other means, with no aid recorded: said as a gap, not refused and not turned round.
        let unaided = Judgement {
            decides: Decides::Cannot,
            help_given: vec![],
            ..intubated
        };
        assert_eq!(gaps(&unaided).len(), 2);
        assert!(validate(&unaided, now()).is_empty());
    }

    #[test]
    fn an_interpreted_judgement_records_the_interpreter_and_other_help_needs_its_words() {
        let interpreted = Judgement {
            help_given: vec![Help::Interpreter, Help::Other],
            help_words: Some("their daughter was asked to leave the room".into()),
            ..judged("2026-11-26T11:00:00Z", "Dr C", Decides::Can)
        };
        assert!(validate(&interpreted, now()).is_empty());
        let unsaid = Judgement {
            help_words: None,
            ..interpreted.clone()
        };
        assert_eq!(validate(&unsaid, now()).len(), 1);
        let twice = Judgement {
            help_given: vec![Help::Interpreter, Help::Interpreter],
            help_words: None,
            ..interpreted
        };
        assert_eq!(validate(&twice, now()).len(), 1);
    }

    #[test]
    fn two_people_who_disagree_are_a_dispute_and_not_known_is_no_side() {
        let a = judged("2026-11-26T09:00:00Z", "Dr A", Decides::Can);
        let b = judged("2026-11-26T10:00:00Z", "Dr B", Decides::Cannot);
        assert!(disputed(&[&a, &b]));
        // Dr B later agrees: their latest view counts, so it is no longer disputed.
        let b_later = judged("2026-11-26T11:00:00Z", "dr b", Decides::Can);
        assert!(!disputed(&[&a, &b, &b_later]));
        let unsure = judged("2026-11-26T11:30:00Z", "Dr D", Decides::NotKnown);
        assert!(!disputed(&[&a, &unsure]));
    }

    #[test]
    fn a_judgement_needs_its_decision_reasons_author_and_a_real_past_time() {
        let mut j = judged("2026-11-27T00:00:00Z", " ", Decides::NotKnown);
        j.because = String::new();
        j.about = "x".repeat(WORDS_LIMIT + 1);
        let problems = validate(&j, now());
        assert_eq!(problems.len(), 4, "{problems:?}");
        assert!(
            problems.iter().all(|p| !p.contains("xxx")),
            "a problem quotes a field"
        );
        j.at = "yesterday".into();
        assert!(validate(&j, now()).iter().any(|p| p.contains("RFC 3339")));
        assert_eq!(age_minutes(&j, now()), None);
        let unknown = Judgement {
            conscious: Known::NotKnown,
            ..judged("2026-11-26T09:00:00Z", "Dr A", Decides::NotKnown)
        };
        assert_eq!(
            gaps(&unknown),
            vec!["whether they were conscious is not known"]
        );
    }

    #[test]
    fn nothing_extra_is_taken_and_every_answer_is_required() {
        let v = serde_json::to_value(judged("2026-11-26T09:00:00Z", "Dr A", Decides::Can)).unwrap();
        for missing in [
            "about",
            "at",
            "conscious",
            "communicates",
            "decides",
            "because",
            "by",
        ] {
            let mut w = v.clone();
            w.as_object_mut().unwrap().remove(missing);
            assert!(serde_json::from_value::<Judgement>(w).is_err(), "{missing}");
        }
        let mut score = v;
        score["score"] = serde_json::json!(3);
        assert!(
            serde_json::from_value::<Judgement>(score).is_err(),
            "a score was taken"
        );
    }
}
