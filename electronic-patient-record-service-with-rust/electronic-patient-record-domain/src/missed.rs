//! Why an appointment was not attended (XP-15): a closed list that belongs to the deployment, checked here with no
//! database. A reason is required when a missed appointment is recorded, and may be [`UNKNOWN`].
//!
//! Who was at fault is not asked. "Not brought", "not escorted" and "did not come" are reasons of their own, so that the
//! record does not say that a person failed to come when nobody brought them. A reason can be sensitive ("Afraid of the
//! treatment"): it is in the referral's history and nowhere else, so never in an audit row, a task, a metric or a log.

#![deny(
    clippy::arithmetic_side_effects,
    clippy::default_numeric_fallback,
    clippy::integer_division,
    clippy::modulo_arithmetic,
    clippy::as_conversions,
    clippy::string_slice
)]

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::sync::LazyLock;

/// The built-in list's text.
pub const SOURCE: &str = include_str!("missed_reasons.toml");

/// The reason that is in every list.
pub const UNKNOWN: &str = "unknown";

/// The most reasons a list may hold, and the longest code and words.
pub const MOST: usize = 500;
/// The longest code.
pub const CODE_LIMIT: usize = 40;
/// The longest words.
pub const WORDS_LIMIT: usize = 120;

/// One reason of a list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reason {
    /// Stored. Lower-case letters, digits and hyphens; never reused for another meaning.
    pub code: String,
    /// What a person reads.
    pub words: String,
    /// No longer offered, and kept so that it is not used again for something else.
    #[serde(default)]
    pub retired: bool,
}

/// A reason as given: its code, and the words it had then, so that a later change of the list does not change what was
/// said.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Given {
    /// The code.
    pub code: String,
    /// The words it had.
    pub words: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    reason: Vec<Reason>,
}

/// The reasons in `text`, called `name` in a message.
///
/// # Errors
///
/// The first thing wrong: the TOML; no reasons or too many; a code or words missing, too long or of the wrong shape; a
/// code or words twice; or no [`UNKNOWN`], or a retired one.
pub fn parse(text: &str, name: &str) -> Result<Vec<Reason>, String> {
    let file: File = toml::from_str(text).map_err(|e| format!("{name}: {}", e.message()))?;
    let all = file.reason;
    if all.len() > MOST {
        return Err(format!("{name}: {} reasons; at most {MOST}", all.len()));
    }
    let (mut codes, mut words) = (BTreeSet::new(), BTreeSet::new());
    for r in &all {
        let shape = r
            .code
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            && r.code.starts_with(|c: char| c.is_ascii_lowercase());
        if !shape || r.code.chars().count() > CODE_LIMIT {
            return Err(format!(
                "{name}: the code {:?} must be 1 to {CODE_LIMIT} lower-case letters, digits and hyphens, a letter first",
                r.code
            ));
        }
        let said = r.words.trim();
        if said.is_empty()
            || said.chars().count() > WORDS_LIMIT
            || said != r.words
            || said.chars().any(char::is_control)
        {
            return Err(format!(
                "{name}: the words of {:?} must be 1 to {WORDS_LIMIT} characters with no space around them",
                r.code
            ));
        }
        if !codes.insert(r.code.as_str()) {
            return Err(format!("{name}: the code {:?} is there twice", r.code));
        }
        // Two that read alike could not be told apart when one is chosen. A retired one is not offered.
        if !r.retired && !words.insert(said.to_lowercase()) {
            return Err(format!("{name}: {said:?} is there twice"));
        }
    }
    if !all.iter().any(|r| r.code == UNKNOWN && !r.retired) {
        return Err(format!(
            "{name}: there is no reason with the code {UNKNOWN:?}"
        ));
    }
    Ok(all)
}

/// The built-in list.
///
/// # Errors
///
/// [`parse`]'s (a test keeps it valid).
pub fn built_in() -> Result<&'static [Reason], String> {
    static LIST: LazyLock<Result<Vec<Reason>, String>> =
        LazyLock::new(|| parse(SOURCE, "missed_reasons.toml"));
    LIST.as_deref().map_err(String::clone)
}

/// The reasons of `list` that may be given now, in the list's order.
#[must_use]
pub fn offered(list: &[Reason]) -> Vec<Given> {
    list.iter()
        .filter(|r| !r.retired)
        .map(|r| Given {
            code: r.code.clone(),
            words: r.words.clone(),
        })
        .collect()
}

/// The reason with this code, to be stored; `None` when it is not in the list or no longer offered.
#[must_use]
pub fn given(list: &[Reason], code: &str) -> Option<Given> {
    offered(list).into_iter().find(|g| g.code == code)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(reasons: &[(&str, &str)]) -> String {
        reasons
            .iter()
            .map(|(c, w)| format!("[[reason]]\ncode = {c:?}\nwords = {w:?}\n"))
            .collect()
    }

    #[test]
    fn the_built_in_list_is_valid_and_tells_not_brought_from_did_not_come() {
        let all = built_in().unwrap();
        for code in [UNKNOWN, "did-not-come", "not-brought", "not-escorted"] {
            assert!(given(all, code).is_some(), "{code}");
        }
        assert_eq!(offered(all).len(), all.len());
    }

    #[test]
    fn a_list_of_a_tenants_own_may_be_long() {
        let mut text = list(&[
            ("unknown", "Unknown"),
            ("no-car", "Unable to drive to hospital"),
        ]);
        for i in 0..400_u32 {
            text.push_str(&list(&[(&format!("r-{i}"), &format!("Reason {i}"))]));
        }
        let all = parse(&text, "mine").unwrap();
        assert_eq!(all.len(), 402);
        assert_eq!(
            given(&all, "no-car").unwrap().words,
            "Unable to drive to hospital"
        );
    }

    #[test]
    fn a_list_with_something_wrong_is_refused_and_says_what() {
        let u = ("unknown", "Unknown");
        for (text, says) in [
            (String::new(), "missing field"),
            (list(&[("other", "Other")]), "no reason with the code"),
            (
                format!("{}retired = true\n", list(&[u])),
                "no reason with the code",
            ),
            (list(&[u, u]), "\"unknown\" is there twice"),
            (list(&[u, ("a", "Same"), ("b", "same")]), "is there twice"),
            (list(&[u, ("A", "x")]), "the code \"A\""),
            (list(&[u, ("", "x")]), "the code \"\""),
            (list(&[u, ("1a", "x")]), "the code \"1a\""),
            (list(&[u, (&"a".repeat(41), "x")]), "must be 1 to 40"),
            (list(&[u, ("a", " x")]), "the words of \"a\""),
            (list(&[u, ("a", "")]), "the words of \"a\""),
            (list(&[u, ("a", "x\ny")]), "the words of \"a\""),
            (list(&[u, ("a", &"x".repeat(121))]), "must be 1 to 120"),
            (
                format!("{}fault = \"patient\"\n", list(&[u])),
                "unknown field",
            ),
        ] {
            let e = parse(&text, "mine").unwrap_err();
            assert!(e.starts_with("mine: ") && e.contains(says), "{e}");
        }
        let many: String = (0..501_u32)
            .map(|i| list(&[(&format!("r-{i}"), &format!("R {i}"))]))
            .collect();
        assert!(parse(&many, "mine").unwrap_err().contains("at most 500"));
    }

    #[test]
    fn a_retired_reason_is_kept_and_not_offered() {
        let text = format!(
            "{}{}retired = true\n",
            list(&[("unknown", "Unknown")]),
            list(&[("old", "Old")])
        );
        let all = parse(&text, "mine").unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(offered(&all).len(), 1);
        assert!(given(&all, "old").is_none());
        // Its words may be used again, its code not.
        assert!(parse(&format!("{text}{}", list(&[("new", "Old")])), "mine").is_ok());
        assert!(parse(&format!("{text}{}", list(&[("old", "New")])), "mine").is_err());
    }
}
