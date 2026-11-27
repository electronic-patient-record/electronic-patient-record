//! A refusal of disclosure that the organisation declined (DR-10; `spec/patient-refusal-of-disclosure-of-records`): who
//! declined it, on what ground, when the patient was told and how, and how they may complain. **Only where the tenant's pack
//! allows it**: under `strength = binds`, the spec's default, a refusal cannot be declined, and [`may_decline`] says so.
//! Nothing here decides whether a refusal should be declined.

use crate::disclosure_rules::{Pack, Setting, Value};
use serde::{Deserialize, Serialize};

/// The longest words of any field.
pub const WORDS_LIMIT: usize = 500;

/// The ground, as a closed list; the words are in `because`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Ground {
    /// The law requires the disclosure.
    Law,
    /// Another person would be at serious risk.
    RiskToAnother,
    /// The refusal cannot be carried out safely here, and the patient was told what that means.
    CannotBeDoneSafely,
}

impl Ground {
    /// Every ground (the stored words are checked against the database, PR-3).
    pub const ALL: [Ground; 3] = [
        Ground::Law,
        Ground::RiskToAnother,
        Ground::CannotBeDoneSafely,
    ];

    /// The stored word, as serde writes it.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Ground::Law => "law",
            Ground::RiskToAnother => "risk-to-another",
            Ground::CannotBeDoneSafely => "cannot-be-done-safely",
        }
    }
}

/// A declined refusal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Declined {
    /// Who declined it, by name and role.
    pub by: String,
    /// The ground.
    pub ground: Ground,
    /// Why, in words the patient was given.
    pub because: String,
    /// When the patient was told (RFC 3339).
    pub told_at: String,
    /// How they were told: in person, by letter, by telephone, in words.
    pub told_how: String,
    /// How they may complain, and to whom, as they were told.
    pub complain_to: String,
}

/// Whether the pack lets a refusal be declined at all: never under `binds`, and `binds` is the default.
#[must_use]
pub fn may_decline(pack: Option<&Pack>) -> bool {
    let Some(pack) = pack else {
        return false;
    };
    match pack.setting("strength") {
        Setting::Pack {
            value: Value::One(w),
            ..
        }
        | Setting::Default {
            value: Value::One(w),
        } => w != "binds",
        // Anything else is not a strength this version can read: it does not allow declining.
        _ => false,
    }
}

fn words(out: &mut Vec<String>, what: &str, value: &str) {
    if value.trim().is_empty() {
        out.push(format!("{what} must not be blank"));
    } else if value.chars().count() > WORDS_LIMIT {
        out.push(format!("{what} must be up to {WORDS_LIMIT} characters"));
    }
}

/// What is wrong with a declined refusal under `pack`. The first problem, when the pack does not allow it, is that.
#[must_use]
pub fn validate(d: &Declined, pack: Option<&Pack>, now: jiff::Timestamp) -> Vec<String> {
    let mut out = Vec::new();
    if !may_decline(pack) {
        out.push("a refusal cannot be declined here: the pack's strength is binds".into());
    }
    words(&mut out, "by", &d.by);
    words(&mut out, "because", &d.because);
    words(&mut out, "told_how", &d.told_how);
    words(&mut out, "complain_to", &d.complain_to);
    match crate::clock::parse_rfc3339(&d.told_at) {
        None => out.push("told_at must be an RFC 3339 time".into()),
        Some(at) if at > now => out.push("told_at must not be in the future".into()),
        Some(_) => {}
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> jiff::Timestamp {
        crate::clock::parse_rfc3339("2026-11-26T12:00:00Z").unwrap()
    }

    fn pack(strength: Option<&str>) -> Pack {
        let rule = strength.map_or(String::new(), |s| {
            format!("[[rule]]\nkey = \"strength\"\nvalue = {s:?}\nsource = \"s\"\n")
        });
        crate::disclosure_rules::parse(
            &format!("[pack]\nid = \"x\"\nplace = \"P\"\nlaw_as_of = \"2026-11-26\"\n{rule}"),
            "mine",
        )
        .unwrap()
    }

    fn declined() -> Declined {
        Declined {
            by: "Dr F, Caldicott guardian".into(),
            ground: Ground::RiskToAnother,
            because: "a child in the house is at risk".into(),
            told_at: "2026-11-26T10:00:00Z".into(),
            told_how: "in person, with a letter".into(),
            complain_to: "the privacy officer, then the regulator".into(),
        }
    }

    #[test]
    fn it_is_impossible_under_binds_set_or_by_default_and_with_no_pack() {
        for p in [None, Some(pack(None)), Some(pack(Some("binds")))] {
            assert!(!may_decline(p.as_ref()));
            let problems = validate(&declined(), p.as_ref(), now());
            assert_eq!(problems.len(), 1);
            assert!(problems[0].contains("binds"));
        }
    }

    #[test]
    fn where_the_pack_allows_it_it_says_who_why_when_told_and_how_to_complain() {
        for strength in ["considered", "duty"] {
            // A duty to consider an objection is an opt-out's (DR-3 refuses it beside the default opt-in).
            let p = if strength == "duty" {
                crate::disclosure_rules::parse(
                    "[pack]\nid = \"x\"\nplace = \"P\"\nlaw_as_of = \"2026-11-26\"\n[[rule]]\nkey = \"strength\"\nvalue = \"duty\"\nsource = \"s\"\n[[rule]]\nkey = \"model\"\nvalue = \"opt-out\"\nsource = \"s\"\n",
                    "mine",
                )
                .unwrap()
            } else {
                pack(Some(strength))
            };
            assert!(may_decline(Some(&p)), "{strength}");
            assert!(validate(&declined(), Some(&p), now()).is_empty());
        }
        let p = pack(Some("considered"));
        let unsaid = Declined {
            complain_to: " ".into(),
            told_at: "tomorrow".into(),
            ..declined()
        };
        assert_eq!(validate(&unsaid, Some(&p), now()).len(), 2);
    }
}
