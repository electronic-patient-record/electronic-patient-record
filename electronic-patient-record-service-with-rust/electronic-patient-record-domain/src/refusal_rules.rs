//! What a place's law is recorded as saying about refusing treatment (RF-3; `spec/patient-refusal-of-consent`): a pack
//! that belongs to the tenant, checked here with no database. **Nothing here says whether someone may be treated.** A rule
//! is shown beside the facts, with its source, its date and who reviewed it, and a person decides.
//!
//! - **Closed lists.** A key or a value that is not listed here is refused, and so is a rule with no source.
//! - **What is not there is not established.** There is no default rule, and no rule is worked out from another.
//! - **Unreviewed is said.** A pack that does not name its lawyer, its clinician and the day loads, and each rule says so.
//! - **An age is worked out only when it is certain.** A birth date that is estimated, partial or absent may leave it not
//!   known, and not known is nobody's adult and nobody's child.

#![deny(
    clippy::arithmetic_side_effects,
    clippy::default_numeric_fallback,
    clippy::integer_division,
    clippy::modulo_arithmetic,
    clippy::as_conversions,
    clippy::string_slice
)]

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::LazyLock;

/// The pack for England and Wales, as the author read the law; nobody has reviewed it.
pub const ENGLAND_WALES: &str = include_str!("refusal_rules_england_wales.toml");

/// The longest text in a pack.
pub const TEXT_LIMIT: usize = 500;

/// What a rule may hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Takes {
    /// Whole years, 1 to 30.
    Years,
    /// One of these words.
    One(&'static [&'static str]),
    /// Any of these words, at least one, none twice.
    Any(&'static [&'static str]),
}

/// Every rule a pack may have.
const KEYS: &[(&str, Takes)] = &[
    ("adult-age", Takes::Years),
    ("young-person-age", Takes::Years),
    ("advance-age", Takes::Years),
    (
        "capable-refusal",
        Takes::One(&["holds", "holds-with-process", "unsettled"]),
    ),
    (
        "emergency-threats",
        Takes::Any(&[
            "life",
            "limb",
            "serious-deterioration",
            "severe-pain",
            "another-person",
        ]),
    ),
    (
        "emergency-yields-to",
        Takes::One(&["any-known", "in-form-only", "not-stated", "unsettled"]),
    ),
    (
        "advance-force",
        Takes::One(&[
            "binding-in-form",
            "binding",
            "weighed",
            "dying-only",
            "unsettled",
        ]),
    ),
    (
        "advance-form",
        Takes::Any(&[
            "written",
            "signed",
            "witnessed",
            "two-witnesses",
            "doctor",
            "registered",
            "life-at-risk-words",
        ]),
    ),
    (
        "advance-limits",
        Takes::Any(&["terminal", "imminent-death", "pregnancy"]),
    ),
    (
        "withdrawal-spoken",
        Takes::One(&["ends-it", "raises-doubt"]),
    ),
    (
        "withdrawal-written",
        Takes::One(&["ends-it", "raises-doubt"]),
    ),
    (
        "withdrawal-conduct",
        Takes::One(&["ends-it", "raises-doubt"]),
    ),
    (
        "doubt",
        Takes::One(&[
            "life-prevails",
            "does-not-apply",
            "ask-a-court",
            "ask-a-board",
        ]),
    ),
    (
        "minor-override",
        Takes::Any(&[
            "court",
            "parent",
            "committee",
            "minister",
            "child-protection",
        ]),
    ),
    (
        "detention-reaches-unrelated",
        Takes::One(&["yes", "no", "in-part"]),
    ),
    (
        "refusal-record",
        Takes::Any(&["written", "signed", "witnessed", "risks-explained"]),
    ),
];

/// Who the pack is for and who stands behind it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Head {
    /// Lower-case letters, digits and hyphens.
    pub id: String,
    /// The place, as a person reads it.
    pub place: String,
    /// The day the law was read (`YYYY-MM-DD`).
    pub law_as_of: String,
    /// The lawyer who reviewed it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lawyer: Option<String>,
    /// The clinician who reviewed it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clinician: Option<String>,
    /// The day they did.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed_on: Option<String>,
    /// The day it must be looked at again.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_due: Option<String>,
}

/// What a rule holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    /// Whole years.
    Years(u8),
    /// One word.
    One(String),
    /// Several words.
    Any(Vec<String>),
}

/// One rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    /// Which.
    pub key: String,
    /// What it is recorded as.
    pub value: Value,
    /// Where that was read.
    pub source: String,
    /// What a reader should know beside it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    pack: Head,
    #[serde(default)]
    rule: Vec<Rule>,
}

/// A pack that was read and checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pack {
    /// Its head.
    pub head: Head,
    rules: BTreeMap<String, Rule>,
}

/// Whether a pack's people and days are there and stand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Review {
    /// Nobody is named, or not all of it.
    Unreviewed,
    /// Reviewed, and the day it was due again has passed.
    Lapsed,
    /// Reviewed, and not due.
    Current,
}

/// How old someone is, to a pack's ages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgeGroup {
    /// At or over the adult age.
    Adult,
    /// Under it, and at or over the young person's.
    YoungPerson,
    /// Under both.
    Child,
    /// It cannot be said.
    NotKnown,
}

/// Text that is there, not too long, with no space around it and no control character.
fn text(name: &str, what: &str, value: &str) -> Result<(), String> {
    let plain = !value.trim().is_empty()
        && value.trim() == value
        && value.chars().count() <= TEXT_LIMIT
        && !value.chars().any(char::is_control);
    if plain {
        Ok(())
    } else {
        Err(format!(
            "{name}: {what} must be 1 to {TEXT_LIMIT} characters with no space around them"
        ))
    }
}

fn day(name: &str, what: &str, value: &str) -> Result<(), String> {
    match crate::clock::parse_day(value) {
        Some(_) => Ok(()),
        None => Err(format!("{name}: {what} must be a day, YYYY-MM-DD")),
    }
}

fn check_rule(name: &str, rule: &Rule) -> Result<(), String> {
    let key = &rule.key;
    let Some((_, takes)) = KEYS.iter().find(|(k, _)| k == key) else {
        return Err(format!("{name}: {key:?} is not a rule this version knows"));
    };
    text(name, &format!("the source of {key:?}"), &rule.source)?;
    if let Some(note) = &rule.note {
        text(name, &format!("the note of {key:?}"), note)?;
    }
    let known = |allowed: &[&str], word: &str| allowed.contains(&word);
    let good = match (takes, &rule.value) {
        (Takes::Years, Value::Years(y)) => (1..=30).contains(y),
        (Takes::One(allowed), Value::One(word)) => known(allowed, word),
        (Takes::Any(allowed), Value::Any(words)) => {
            let mut seen = std::collections::BTreeSet::new();
            !words.is_empty()
                && words
                    .iter()
                    .all(|w| known(allowed, w) && seen.insert(w.as_str()))
        }
        _ => false,
    };
    if good {
        Ok(())
    } else {
        Err(format!(
            "{name}: the value of {key:?} is not one it may have"
        ))
    }
}

/// The pack in `text`, called `name` in a message.
///
/// # Errors
///
/// The first thing wrong: the TOML; a head with a bad id, place or day, or a reviewer named in part; a rule that is not
/// known, is there twice, has no source or a value it may not have; or ages that contradict each other.
pub fn parse(source: &str, name: &str) -> Result<Pack, String> {
    let file: File = toml::from_str(source).map_err(|e| format!("{name}: {}", e.message()))?;
    let head = file.pack;
    let shape = head.id.starts_with(|c: char| c.is_ascii_lowercase())
        && head
            .id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && head.id.chars().count() <= 40;
    if !shape {
        return Err(format!(
            "{name}: the id must be 1 to 40 lower-case letters, digits and hyphens, a letter first"
        ));
    }
    text(name, "the place", &head.place)?;
    day(name, "law_as_of", &head.law_as_of)?;
    for (what, value) in [
        ("the lawyer", &head.lawyer),
        ("the clinician", &head.clinician),
    ] {
        if let Some(value) = value {
            text(name, what, value)?;
        }
    }
    for (what, value) in [
        ("reviewed_on", &head.reviewed_on),
        ("review_due", &head.review_due),
    ] {
        if let Some(value) = value {
            day(name, what, value)?;
        }
    }
    let named = [
        head.lawyer.is_some(),
        head.clinician.is_some(),
        head.reviewed_on.is_some(),
        head.review_due.is_some(),
    ];
    if named.iter().any(|n| *n) && !named.iter().all(|n| *n) {
        return Err(format!(
            "{name}: a review is a lawyer, a clinician, reviewed_on and review_due, all or none"
        ));
    }
    let mut rules = BTreeMap::new();
    for rule in file.rule {
        check_rule(name, &rule)?;
        let key = rule.key.clone();
        if rules.insert(key.clone(), rule).is_some() {
            return Err(format!("{name}: {key:?} is there twice"));
        }
    }
    let pack = Pack { head, rules };
    if let (Some(adult), Some(young)) = (pack.years("adult-age"), pack.years("young-person-age"))
        && young >= adult
    {
        return Err(format!("{name}: young-person-age must be under adult-age"));
    }
    if pack.years("young-person-age").is_some() && pack.years("adult-age").is_none() {
        return Err(format!("{name}: young-person-age needs adult-age"));
    }
    // A form is what makes a binding one bind; without it, "in form" says nothing.
    if pack.word("advance-force") == Some("binding-in-form")
        && !pack.rules.contains_key("advance-form")
    {
        return Err(format!("{name}: binding-in-form needs advance-form"));
    }
    if pack.word("emergency-yields-to") == Some("in-form-only")
        && !pack.rules.contains_key("advance-form")
    {
        return Err(format!("{name}: in-form-only needs advance-form"));
    }
    Ok(pack)
}

/// The built-in pack for England and Wales.
///
/// # Errors
///
/// [`parse`]'s (a test keeps it valid).
pub fn england_wales() -> Result<&'static Pack, String> {
    static PACK: LazyLock<Result<Pack, String>> =
        LazyLock::new(|| parse(ENGLAND_WALES, "refusal_rules_england_wales.toml"));
    PACK.as_ref().map_err(String::clone)
}

impl Pack {
    /// The rule under `key`, or `None` when it is not established. `None` is never a rule.
    #[must_use]
    pub fn rule(&self, key: &str) -> Option<&Rule> {
        self.rules.get(key)
    }

    /// Every rule, by key.
    pub fn rules(&self) -> impl Iterator<Item = &Rule> {
        self.rules.values()
    }

    /// The keys a pack may have and this one does not: what a screen lists as not established.
    #[must_use]
    pub fn not_established(&self) -> Vec<&'static str> {
        KEYS.iter()
            .map(|(k, _)| *k)
            .filter(|k| !self.rules.contains_key(*k))
            .collect()
    }

    fn years(&self, key: &str) -> Option<u8> {
        match self.rules.get(key)?.value {
            Value::Years(y) => Some(y),
            Value::One(_) | Value::Any(_) => None,
        }
    }

    fn word(&self, key: &str) -> Option<&str> {
        match &self.rules.get(key)?.value {
            Value::One(w) => Some(w),
            Value::Years(_) | Value::Any(_) => None,
        }
    }

    /// Whether the review stands on `today`.
    #[must_use]
    pub fn review(&self, today: jiff::civil::Date) -> Review {
        let Some(due) = self
            .head
            .review_due
            .as_deref()
            .and_then(crate::clock::parse_day)
        else {
            return Review::Unreviewed;
        };
        if self.head.lawyer.is_none() || self.head.clinician.is_none() {
            return Review::Unreviewed;
        }
        if today > due {
            Review::Lapsed
        } else {
            Review::Current
        }
    }

    /// The age group on `today` of someone born on `birth_date` (`YYYY`, `YYYY-MM` or `YYYY-MM-DD`). Not known when there
    /// is none, when it is `estimated`, when it is in the future, when the pack has no ages, and when a partial date could
    /// fall in two groups.
    #[must_use]
    pub fn age_group(
        &self,
        birth_date: Option<&str>,
        estimated: bool,
        today: jiff::civil::Date,
    ) -> AgeGroup {
        let Some((earliest, latest)) = birth_date.filter(|_| !estimated).and_then(span) else {
            return AgeGroup::NotKnown;
        };
        if latest > today {
            return AgeGroup::NotKnown;
        }
        let (Some(oldest), Some(youngest)) = (age(earliest, today), age(latest, today)) else {
            return AgeGroup::NotKnown;
        };
        let group = |years: u8| {
            let Some(adult) = self.years("adult-age") else {
                return AgeGroup::NotKnown;
            };
            match self.years("young-person-age") {
                _ if years >= adult => AgeGroup::Adult,
                Some(young) if years >= young => AgeGroup::YoungPerson,
                Some(_) | None => AgeGroup::Child,
            }
        };
        if group(oldest) == group(youngest) {
            group(oldest)
        } else {
            AgeGroup::NotKnown
        }
    }
}

/// The first and last day that a partial date may be.
fn span(date: &str) -> Option<(jiff::civil::Date, jiff::civil::Date)> {
    if !crate::dates::valid_birth_date(date) {
        return None;
    }
    match date.len() {
        4 => Some((
            crate::clock::parse_day(&format!("{date}-01-01"))?,
            crate::clock::parse_day(&format!("{date}-12-31"))?,
        )),
        7 => {
            let first = crate::clock::parse_day(&format!("{date}-01"))?;
            Some((first, first.last_of_month()))
        }
        _ => crate::clock::parse_day(date).map(|d| (d, d)),
    }
}

/// Whole years between a birthday and `today`; `None` past 255 or when it cannot be worked out. Someone born on 29 February
/// has their birthday on 1 March in other years, so that nobody is taken for older than they are.
fn age(born: jiff::civil::Date, today: jiff::civil::Date) -> Option<u8> {
    let years = i32::from(today.year()).checked_sub(i32::from(born.year()))?;
    let had_birthday = (today.month(), today.day()) >= (born.month(), born.day());
    let whole = if had_birthday {
        years
    } else {
        years.checked_sub(1)?
    };
    u8::try_from(whole).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEAD: &str = "[pack]\nid = \"x\"\nplace = \"Somewhere\"\nlaw_as_of = \"2026-11-25\"\n";
    const REVIEW: &str = "lawyer = \"A\"\nclinician = \"B\"\nreviewed_on = \"2026-11-25\"\nreview_due = \"2027-11-25\"\n";

    fn rule(key: &str, value: &str) -> String {
        format!("[[rule]]\nkey = {key:?}\nvalue = {value}\nsource = \"s\"\n")
    }

    fn with(rules: &str) -> Pack {
        parse(&format!("{HEAD}{rules}"), "mine").unwrap()
    }

    fn today() -> jiff::civil::Date {
        crate::clock::parse_day("2026-11-25").unwrap()
    }

    #[test]
    fn the_built_in_pack_is_valid_and_says_that_nobody_reviewed_it() {
        let pack = england_wales().unwrap();
        assert_eq!(pack.review(today()), Review::Unreviewed);
        assert_eq!(pack.head.place, "England and Wales");
        // It establishes less than it could, and says so.
        assert!(!pack.not_established().is_empty());
        for r in pack.rules() {
            assert!(r.source.contains("http"), "{}", r.key);
        }
    }

    #[test]
    fn what_is_not_there_is_not_established_and_nothing_is_worked_out() {
        let pack = with("");
        assert_eq!(pack.rules().count(), 0);
        assert_eq!(pack.not_established().len(), KEYS.len());
        assert_eq!(pack.rule("capable-refusal"), None);
        assert_eq!(
            pack.age_group(Some("1950-01-01"), false, today()),
            AgeGroup::NotKnown
        );
    }

    #[test]
    fn each_thing_wrong_is_refused_and_said() {
        let r = |k: &str, v: &str| format!("{HEAD}{}", rule(k, v));
        for (text, says) in [
            (String::new(), "missing field"),
            (HEAD.replace("\"x\"", "\"X\""), "the id"),
            (HEAD.replace("\"x\"", "\"\""), "the id"),
            (HEAD.replace("Somewhere", " x"), "the place"),
            (HEAD.replace("2026-11-25", "2026-13-01"), "law_as_of"),
            (format!("{HEAD}colour = \"red\"\n"), "unknown field"),
            (format!("{HEAD}lawyer = \"A\"\n"), "all or none"),
            (
                format!("{HEAD}{}", REVIEW.replace("2027-11-25", "soon")),
                "review_due",
            ),
            (r("may-treat", "\"yes\""), "not a rule"),
            (r("capable-refusal", "\"always\""), "not one it may have"),
            (r("capable-refusal", "[\"holds\"]"), "not one it may have"),
            (r("adult-age", "0"), "not one it may have"),
            (r("adult-age", "31"), "not one it may have"),
            (r("adult-age", "\"18\""), "not one it may have"),
            (r("advance-form", "[]"), "not one it may have"),
            (
                r("advance-form", "[\"written\", \"written\"]"),
                "not one it may have",
            ),
            (
                r("capable-refusal", "\"holds\"").replace("source = \"s\"", "source = \" \""),
                "the source",
            ),
            (
                r("capable-refusal", "\"holds\"").replace("source = \"s\"\n", ""),
                "missing field",
            ),
            (
                format!(
                    "{}{}",
                    r("doubt", "\"life-prevails\""),
                    rule("doubt", "\"ask-a-court\"")
                ),
                "is there twice",
            ),
            (
                r("advance-force", "\"binding-in-form\""),
                "needs advance-form",
            ),
            (
                r("emergency-yields-to", "\"in-form-only\""),
                "needs advance-form",
            ),
            (r("young-person-age", "16"), "needs adult-age"),
            (
                format!("{}{}", r("young-person-age", "18"), rule("adult-age", "18")),
                "under adult-age",
            ),
        ] {
            let e = parse(&text, "mine").unwrap_err();
            assert!(e.starts_with("mine: ") && e.contains(says), "{says}: {e}");
        }
    }

    #[test]
    fn a_review_stands_until_its_day_and_no_longer() {
        let pack = parse(&format!("{HEAD}{REVIEW}"), "mine").unwrap();
        for (day, expected) in [
            ("2026-11-25", Review::Current),
            ("2027-11-25", Review::Current),
            ("2027-11-26", Review::Lapsed),
        ] {
            let day = crate::clock::parse_day(day).unwrap();
            assert_eq!(pack.review(day), expected, "{day}");
        }
    }

    #[test]
    fn an_age_group_is_said_only_when_it_is_certain() {
        let pack = with(&format!(
            "{}{}",
            rule("adult-age", "18"),
            rule("young-person-age", "16")
        ));
        let group = |born: &str| pack.age_group(Some(born), false, today());
        for (born, expected) in [
            ("2008-11-25", AgeGroup::Adult),
            ("2008-11-26", AgeGroup::YoungPerson),
            ("2010-11-25", AgeGroup::YoungPerson),
            ("2010-11-26", AgeGroup::Child),
            ("2026-11-25", AgeGroup::Child),
            ("1950", AgeGroup::Adult),
            ("2015-06", AgeGroup::Child),
            // It could be either.
            ("2008", AgeGroup::NotKnown),
            ("2008-11", AgeGroup::NotKnown),
            ("2010", AgeGroup::NotKnown),
            ("2026-11-26", AgeGroup::NotKnown),
            ("2026", AgeGroup::NotKnown),
            ("not a date", AgeGroup::NotKnown),
        ] {
            assert_eq!(group(born), expected, "{born}");
        }
        assert_eq!(
            pack.age_group(Some("1950-01-01"), true, today()),
            AgeGroup::NotKnown,
            "an estimated birth date says nothing"
        );
        assert_eq!(pack.age_group(None, false, today()), AgeGroup::NotKnown);
        // Born on 29 February: not older on 28 February.
        let leap = |day: &str| {
            let day = crate::clock::parse_day(day).unwrap();
            pack.age_group(Some("2008-02-29"), false, day)
        };
        assert_eq!(leap("2026-02-28"), AgeGroup::YoungPerson);
        assert_eq!(leap("2026-03-01"), AgeGroup::Adult);
    }

    #[test]
    fn a_pack_with_no_young_persons_age_has_adults_and_children() {
        let pack = with(&rule("adult-age", "16"));
        assert_eq!(
            pack.age_group(Some("2010-11-25"), false, today()),
            AgeGroup::Adult
        );
        assert_eq!(
            pack.age_group(Some("2010-11-26"), false, today()),
            AgeGroup::Child
        );
    }

    #[test]
    fn a_value_is_sent_as_it_is_written() {
        let pack = with(&format!(
            "{}{}",
            rule("advance-form", "[\"written\", \"signed\"]"),
            rule("advance-age", "18")
        ));
        assert_eq!(
            serde_json::json!(pack.rule("advance-form").unwrap().value),
            serde_json::json!(["written", "signed"])
        );
        assert_eq!(
            serde_json::json!(pack.rule("advance-age").unwrap().value),
            18_u8
        );
    }
}
