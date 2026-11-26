//! What a tenant has set for refusals of disclosure, and the law it was read from (DR-3;
//! `spec/patient-refusal-of-disclosure-of-records`): a pack checked here with no database, with RF-3's head and checks
//! ([`crate::refusal_rules`]). **Nothing here decides a read.** It says what was set, where it was read, who reviewed it,
//! and, for a key that is not set, the spec's default, marked as a default.
//!
//! - **Closed lists.** A key or a value that is not listed here is refused, and so is a setting with no source.
//! - **The invariants are not settings.** Audit, deletion, "none" on a partial list and the rest of the spec's list have no
//!   key, so a pack cannot name one.
//! - **A pack that contradicts itself does not load**, and the start stops with why.
//! - **A past decision names the version that stood**: [`Pack::version`] is the id, the day of the law and a hash of the
//!   text, so a decision explained later points at exactly the pack it was made under.

#![deny(
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::string_slice
)]

use crate::refusal_rules::{Review, day, text};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const YES_NO: &[&str] = &["yes", "no"];
const ROLES: &[&str] = &[
    "clinician",
    "nurse",
    "pharmacist",
    "allied-health",
    "on-duty-clinician",
];
const WHO_ASKS: &[&str] = &["never", "with-consent", "unless-objected", "by-law-only"];

/// What a setting may hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Takes {
    /// A whole number in this range.
    Number(u16, u16),
    /// One of these words.
    One(&'static [&'static str]),
    /// Any of these words, at least one, none twice.
    Any(&'static [&'static str]),
}

/// Every setting a pack may have, what it takes and the spec's default when it is not set (`None`: no default, which the
/// screen says).
const KEYS: &[(&str, Takes, Option<Fallback>)] = &[
    (
        "model",
        Takes::One(&["opt-out", "opt-in"]),
        Some(Fallback::One("opt-in")),
    ),
    (
        "strength",
        Takes::One(&["binds", "considered", "duty"]),
        Some(Fallback::One("binds")),
    ),
    (
        "scopes",
        Takes::Any(&[
            "care-here",
            "care-elsewhere",
            "emergency-care",
            "research",
            "planning",
            "registry",
            "teaching",
            "insurer",
        ]),
        None,
    ),
    (
        "always-visible",
        Takes::Any(&["name", "birth-date", "number", "death"]),
        Some(Fallback::Any(&["name", "birth-date", "number", "death"])),
    ),
    (
        "flag",
        Takes::One(&["none", "existence", "count"]),
        Some(Fallback::One("count")),
    ),
    (
        "emergency-exists",
        Takes::One(YES_NO),
        Some(Fallback::One("yes")),
    ),
    (
        "emergency-needs",
        Takes::One(&["cannot-be-asked", "a-reason"]),
        Some(Fallback::One("cannot-be-asked")),
    ),
    (
        "emergency-beats-refusal",
        Takes::One(YES_NO),
        Some(Fallback::One("no")),
    ),
    (
        "emergency-beats-label",
        Takes::One(YES_NO),
        Some(Fallback::One("yes")),
    ),
    (
        "emergency-roles",
        Takes::Any(ROLES),
        Some(Fallback::Any(&[
            "clinician",
            "nurse",
            "pharmacist",
            "allied-health",
        ])),
    ),
    (
        "emergency-again-sign-in",
        Takes::One(YES_NO),
        Some(Fallback::One("no")),
    ),
    (
        "emergency-hours",
        Takes::Number(1, 72),
        Some(Fallback::Number(4)),
    ),
    (
        "emergency-notice",
        Takes::One(&["at-once", "on-request", "none"]),
        Some(Fallback::One("on-request")),
    ),
    (
        "patient-may-close-emergency",
        Takes::One(YES_NO),
        Some(Fallback::One("yes")),
    ),
    (
        "author-sees-own",
        Takes::One(YES_NO),
        Some(Fallback::One("yes")),
    ),
    (
        "safety-alerts",
        Takes::One(&["run-without-revealing", "off"]),
        Some(Fallback::One("off")),
    ),
    (
        "family",
        Takes::One(WHO_ASKS),
        Some(Fallback::One("with-consent")),
    ),
    (
        "employer",
        Takes::One(WHO_ASKS),
        Some(Fallback::One("never")),
    ),
    (
        "insurer",
        Takes::One(WHO_ASKS),
        Some(Fallback::One("never")),
    ),
    (
        "police",
        Takes::One(WHO_ASKS),
        Some(Fallback::One("with-consent")),
    ),
    (
        "immigration",
        Takes::One(WHO_ASKS),
        Some(Fallback::One("never")),
    ),
    (
        "carry",
        Takes::One(&["goes", "stays"]),
        Some(Fallback::One("goes")),
    ),
    (
        "forms",
        Takes::Any(&["spoken", "written", "online"]),
        Some(Fallback::Any(&["spoken", "written", "online"])),
    ),
    // For ever when not set: no number is a default here.
    ("log-years", Takes::Number(1, 200), None),
    (
        "patient-log",
        Takes::One(&["shown", "on-request"]),
        Some(Fallback::One("shown")),
    ),
];

/// The spec's default for a key that is not set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fallback {
    Number(u16),
    One(&'static str),
    Any(&'static [&'static str]),
}

/// Who the pack is for and who stands behind it: RF-3's head and a data protection officer.
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
    /// The data protection officer who reviewed it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_protection_officer: Option<String>,
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

/// What a setting holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    /// A whole number.
    Number(u16),
    /// One word.
    One(String),
    /// Several words.
    Any(Vec<String>),
}

/// One setting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    /// Which.
    pub key: String,
    /// What it is set to.
    pub value: Value,
    /// Where the law for it was read, or who decided it for the tenant.
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
    version: String,
}

/// A setting in force: what the pack says, or the spec's default where it says nothing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "from", rename_all = "kebab-case")]
pub enum Setting {
    /// Set by the pack.
    Pack {
        /// The value.
        value: Value,
        /// Its source.
        source: String,
    },
    /// Not set: the spec's default, which no lawyer has read for this place.
    Default {
        /// The value.
        value: Value,
    },
    /// Not set, and there is no default: the screen says so.
    NotSet,
}

fn check_rule(name: &str, rule: &Rule) -> Result<(), String> {
    let key = &rule.key;
    let Some((_, takes, _)) = KEYS.iter().find(|(k, ..)| k == key) else {
        return Err(format!(
            "{name}: {key:?} is not a setting this version knows"
        ));
    };
    text(name, &format!("the source of {key:?}"), &rule.source)?;
    if let Some(note) = &rule.note {
        text(name, &format!("the note of {key:?}"), note)?;
    }
    let good = match (takes, &rule.value) {
        (Takes::Number(lo, hi), Value::Number(n)) => (*lo..=*hi).contains(n),
        (Takes::One(allowed), Value::One(word)) => allowed.contains(&word.as_str()),
        (Takes::Any(allowed), Value::Any(words)) => {
            let mut seen = std::collections::BTreeSet::new();
            !words.is_empty()
                && words
                    .iter()
                    .all(|w| allowed.contains(&w.as_str()) && seen.insert(w.as_str()))
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

/// The pack in `source`, called `name` in a message.
///
/// # Errors
///
/// The first thing wrong: the TOML; a head with a bad id, place or day, or a review named in part; a setting that is not
/// known, is there twice, has no source or a value it may not have; or settings that contradict each other.
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
        ("the data protection officer", &head.data_protection_officer),
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
        head.data_protection_officer.is_some(),
        head.clinician.is_some(),
        head.reviewed_on.is_some(),
        head.review_due.is_some(),
    ];
    if named.iter().any(|n| *n) && !named.iter().all(|n| *n) {
        return Err(format!(
            "{name}: a review is a lawyer, a data protection officer, a clinician, reviewed_on and review_due, all or none"
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
    use sha2::Digest;
    let hash = sha2::Sha256::digest(source.as_bytes());
    let short: String = hash.iter().take(6).map(|b| format!("{b:02x}")).collect();
    let pack = Pack {
        version: format!("{}@{}#{short}", head.id, head.law_as_of),
        head,
        rules,
    };
    if let Some(why) = contradiction(&pack) {
        return Err(format!("{name}: {why}"));
    }
    Ok(pack)
}

/// The first way the settings in force, defaults included, contradict each other.
fn contradiction(pack: &Pack) -> Option<&'static str> {
    let word = |k: &str| match pack.setting(k) {
        Setting::Pack {
            value: Value::One(w),
            ..
        }
        | Setting::Default {
            value: Value::One(w),
        } => Some(w),
        _ => None,
    };
    let is = |k: &str, w: &str| word(k).as_deref() == Some(w);
    if is("emergency-beats-refusal", "yes") && is("patient-may-close-emergency", "yes") {
        return Some(
            "emergency-beats-refusal = yes contradicts patient-may-close-emergency = yes: say which wins",
        );
    }
    // With no emergency access, nothing about it can be set.
    if is("emergency-exists", "no")
        && pack
            .rules
            .keys()
            .any(|k| k.starts_with("emergency-") && k != "emergency-exists")
    {
        return Some("emergency-exists = no, so no other emergency- setting can be set");
    }
    // A notice at once names that something was hidden, which flag = none says is never shown.
    if is("flag", "none") && is("emergency-notice", "at-once") {
        return Some(
            "flag = none says nothing hidden is shown, and emergency-notice = at-once shows it: say which wins",
        );
    }
    // A model that refuses unless asked has no use for an opt-out strength of duty.
    if is("model", "opt-in") && is("strength", "duty") {
        return Some(
            "model = opt-in with strength = duty: a duty to consider an objection is an opt-out's",
        );
    }
    None
}

impl Pack {
    /// The id, the day of the law and a hash of the text: what a decision records, so that it is explained later by the
    /// version that stood then.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// The setting under `key`: the pack's, or the spec's default marked as one. A key this version does not know is
    /// [`Setting::NotSet`].
    #[must_use]
    pub fn setting(&self, key: &str) -> Setting {
        if let Some(rule) = self.rules.get(key) {
            return Setting::Pack {
                value: rule.value.clone(),
                source: rule.source.clone(),
            };
        }
        match KEYS
            .iter()
            .find(|(k, ..)| *k == key)
            .and_then(|(_, _, d)| *d)
        {
            Some(Fallback::Number(n)) => Setting::Default {
                value: Value::Number(n),
            },
            Some(Fallback::One(w)) => Setting::Default {
                value: Value::One(w.into()),
            },
            Some(Fallback::Any(ws)) => Setting::Default {
                value: Value::Any(ws.iter().map(|w| (*w).to_string()).collect()),
            },
            None => Setting::NotSet,
        }
    }

    /// Every key, with what is in force: for the screen that shows the rules with their sources.
    #[must_use]
    pub fn settings(&self) -> Vec<(&'static str, Setting)> {
        KEYS.iter().map(|(k, ..)| (*k, self.setting(k))).collect()
    }

    /// Whether the review stands on `today`. All five of the review are needed.
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
        if self.head.lawyer.is_none()
            || self.head.data_protection_officer.is_none()
            || self.head.clinician.is_none()
        {
            return Review::Unreviewed;
        }
        if today > due {
            Review::Lapsed
        } else {
            Review::Current
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEAD: &str = "[pack]\nid = \"x\"\nplace = \"Somewhere\"\nlaw_as_of = \"2026-11-26\"\n";
    const REVIEW: &str = "lawyer = \"A\"\ndata_protection_officer = \"D\"\nclinician = \"B\"\nreviewed_on = \"2026-11-26\"\nreview_due = \"2027-11-26\"\n";

    fn rule(key: &str, value: &str) -> String {
        format!("[[rule]]\nkey = {key:?}\nvalue = {value}\nsource = \"s\"\n")
    }

    fn parsed(rules: &str) -> Result<Pack, String> {
        parse(&format!("{HEAD}{rules}"), "mine")
    }

    fn day(d: &str) -> jiff::civil::Date {
        crate::clock::parse_day(d).unwrap()
    }

    #[test]
    fn an_empty_pack_loads_and_every_setting_is_the_specs_default_said_as_one() {
        let pack = parsed("").unwrap();
        assert_eq!(
            pack.setting("emergency-beats-refusal"),
            Setting::Default {
                value: Value::One("no".into())
            }
        );
        assert_eq!(
            pack.setting("emergency-hours"),
            Setting::Default {
                value: Value::Number(4)
            }
        );
        assert_eq!(
            pack.setting("log-years"),
            Setting::NotSet,
            "for ever is not a number"
        );
        assert_eq!(pack.setting("not-a-key"), Setting::NotSet);
        assert_eq!(pack.settings().len(), KEYS.len());
        assert_eq!(pack.review(day("2026-11-26")), Review::Unreviewed);
    }

    #[test]
    fn a_set_value_says_its_source_and_one_it_may_not_have_is_refused() {
        let pack = parsed(&rule("flag", "\"existence\"")).unwrap();
        assert_eq!(
            pack.setting("flag"),
            Setting::Pack {
                value: Value::One("existence".into()),
                source: "s".into()
            }
        );
        for bad in [
            rule("flag", "\"everything\""),
            rule("emergency-hours", "0"),
            rule("emergency-hours", "\"4\""),
            rule("emergency-roles", "[]"),
            rule("emergency-roles", "[\"nurse\", \"nurse\"]"),
            rule("emergency-roles", "[\"receptionist\"]"),
            "[[rule]]\nkey = \"flag\"\nvalue = \"count\"\nsource = \" \"\n".to_string(),
            format!("{}{}", rule("flag", "\"count\""), rule("flag", "\"none\"")),
        ] {
            assert!(parsed(&bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn the_invariants_have_no_key_so_a_pack_cannot_set_one() {
        for invariant in [
            "audit",
            "delete",
            "none-on-partial",
            "show-to-the-refused",
            "export-hidden",
        ] {
            let e = parsed(&rule(invariant, "\"no\"")).unwrap_err();
            assert!(e.contains("not a setting this version knows"), "{e}");
        }
    }

    #[test]
    fn each_contradiction_is_refused_and_a_default_can_be_contradicted_too() {
        let cases = [
            // The spec's own example: the emergency beats a refusal, and the patient may close the emergency (by default).
            rule("emergency-beats-refusal", "\"yes\""),
            format!(
                "{}{}",
                rule("emergency-exists", "\"no\""),
                rule("emergency-hours", "8")
            ),
            format!(
                "{}{}",
                rule("flag", "\"none\""),
                rule("emergency-notice", "\"at-once\"")
            ),
            rule("strength", "\"duty\""),
        ];
        for case in &cases {
            assert!(parsed(case).is_err(), "{case}");
        }
        // Each stands once what it contradicts is said otherwise.
        let fixed = format!(
            "{}{}",
            rule("emergency-beats-refusal", "\"yes\""),
            rule("patient-may-close-emergency", "\"no\"")
        );
        assert!(parsed(&fixed).is_ok());
        assert!(
            parsed(&format!(
                "{}{}",
                rule("model", "\"opt-out\""),
                rule("strength", "\"duty\"")
            ))
            .is_ok()
        );
    }

    #[test]
    fn a_past_decision_names_the_version_that_stood_and_any_change_is_another() {
        let a = parsed(&rule("flag", "\"count\"")).unwrap();
        let same = parsed(&rule("flag", "\"count\"")).unwrap();
        let changed = parsed(&rule("flag", "\"existence\"")).unwrap();
        assert_eq!(a.version(), same.version());
        assert_ne!(a.version(), changed.version());
        assert!(a.version().starts_with("x@2026-11-26#"), "{}", a.version());
    }

    #[test]
    fn a_review_needs_all_five_and_lapses_on_its_day() {
        assert!(parsed("").is_ok());
        let full = parse(&format!("{HEAD}{REVIEW}"), "mine").unwrap();
        assert_eq!(full.review(day("2027-11-26")), Review::Current);
        assert_eq!(full.review(day("2027-11-27")), Review::Lapsed);
        let partial = REVIEW.replace("data_protection_officer = \"D\"\n", "");
        assert!(parse(&format!("{HEAD}{partial}"), "mine").is_err());
    }
}
