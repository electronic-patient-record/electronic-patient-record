//! Asking now, and leaving against advice (RF-10; `spec/patient-refusal-of-consent`, with EC-6): what was offered, what was
//! explained and how, what was refused and what accepted, whether the person was told they may change their mind, and
//! whether it was in private or with whom. One shape for a refusal now and for leaving against advice.
//!
//! **There is no verdict here.** [`pack_gaps`] lists what the place's pack says such a record should hold and this one
//! does not; a record with gaps stands as recorded. "Not known" is a value and never counts as done.

use crate::refusal_rules::{Pack, Value};
use serde::{Deserialize, Serialize};

/// The longest words of any field.
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

/// What was explained, and how.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Explained {
    /// The risks of refusing.
    pub risks: Known,
    /// The benefits of what was offered.
    pub benefits: Known,
    /// The other choices there were.
    pub alternatives: Known,
    /// The language it was explained in, when it is not the record's usual one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Who helped: an interpreter, a supporter. Their role, in words.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub helped_by: Option<String>,
}

/// One time of asking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Asked {
    /// What was offered, in words.
    pub offered: String,
    /// What was explained.
    pub explained: Explained,
    /// What they refused, in their words where known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refused: Option<String>,
    /// What they accepted instead, or of what was offered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accepted: Option<String>,
    /// Whether they were told that they may change their mind, and come back.
    pub told_may_change_mind: Known,
    /// Whether it was in private.
    pub in_private: Known,
    /// Who else was there, by role, when it was not in private.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub with_whom: Option<String>,
    /// They left, or are leaving, against advice (EC-6).
    #[serde(default)]
    pub leaving_against_advice: bool,
    /// Whether the refusal was put in writing.
    pub written: Known,
    /// Whether that writing was signed by them.
    pub signed: Known,
    /// Whether it was witnessed.
    pub witnessed: Known,
    /// When (RFC 3339).
    pub at: String,
    /// Who asked, in words: a name and a role.
    pub by: String,
}

fn words(out: &mut Vec<String>, what: &str, value: &str) {
    if value.trim().is_empty() {
        out.push(format!("{what} must not be blank"));
    } else if value.chars().count() > WORDS_LIMIT {
        out.push(format!("{what} must be up to {WORDS_LIMIT} characters"));
    }
}

/// What is wrong with a record. The words name a field and never quote it.
#[must_use]
pub fn validate(a: &Asked, now: jiff::Timestamp) -> Vec<String> {
    let mut out = Vec::new();
    words(&mut out, "offered", &a.offered);
    words(&mut out, "by", &a.by);
    for (what, value) in [
        ("refused", &a.refused),
        ("accepted", &a.accepted),
        ("with_whom", &a.with_whom),
        ("explained.language", &a.explained.language),
        ("explained.helped_by", &a.explained.helped_by),
    ] {
        if let Some(value) = value {
            words(&mut out, what, value);
        }
    }
    if a.refused.is_none() && a.accepted.is_none() {
        out.push("say what was refused, what was accepted, or both".into());
    }
    if a.leaving_against_advice && a.refused.is_none() {
        out.push("leaving against advice says what they refused".into());
    }
    if a.with_whom.is_some() && a.in_private == Known::Yes {
        out.push("with_whom is for asking that was not in private".into());
    }
    if a.signed == Known::Yes && a.written != Known::Yes {
        out.push("signed is for a refusal that was written".into());
    }
    match crate::clock::parse_rfc3339(&a.at) {
        None => out.push("at must be an RFC 3339 time".into()),
        Some(at) if at > now => out.push("at must not be in the future".into()),
        Some(_) => {}
    }
    out
}

/// What the place's pack (`refusal-record`) says a record of refusing now should hold and this one does not show as done,
/// by the pack's words. Empty when the pack has no such rule, which is not the same as one that is met: ask
/// [`Pack::rule`] first. "Not known" is never done.
#[must_use]
pub fn pack_gaps(a: &Asked, pack: &Pack) -> Vec<String> {
    let Some(Value::Any(wanted)) = pack.rule("refusal-record").map(|r| &r.value) else {
        return Vec::new();
    };
    wanted
        .iter()
        .filter(|w| {
            let done = match w.as_str() {
                "written" => a.written == Known::Yes,
                "signed" => a.signed == Known::Yes,
                "witnessed" => a.witnessed == Known::Yes,
                "described" => a.refused.is_some(),
                "risks-explained" => a.explained.risks == Known::Yes,
                "benefits-explained" => a.explained.benefits == Known::Yes,
                "alternatives-explained" => a.explained.alternatives == Known::Yes,
                "may-change-mind" => a.told_may_change_mind == Known::Yes,
                "in-private" => a.in_private == Known::Yes,
                // A word this version lists and cannot see in a record is not said to be done.
                _ => false,
            };
            !done
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> jiff::Timestamp {
        crate::clock::parse_rfc3339("2026-11-26T12:00:00Z").unwrap()
    }

    fn asked() -> Asked {
        Asked {
            offered: "admission for chest pain".into(),
            explained: Explained {
                risks: Known::Yes,
                benefits: Known::Yes,
                alternatives: Known::NotKnown,
                language: Some("Polish".into()),
                helped_by: Some("a telephone interpreter".into()),
            },
            refused: Some("to stay in hospital".into()),
            accepted: Some("a blood test before leaving".into()),
            told_may_change_mind: Known::Yes,
            in_private: Known::No,
            with_whom: Some("their partner".into()),
            leaving_against_advice: true,
            written: Known::Yes,
            signed: Known::No,
            witnessed: Known::No,
            at: "2026-11-26T11:00:00Z".into(),
            by: "Dr E, emergency department".into(),
        }
    }

    fn pack(words: &str) -> Pack {
        crate::refusal_rules::parse(
            &format!(
                "[pack]\nid = \"x\"\nplace = \"P\"\nlaw_as_of = \"2026-11-26\"\n[[rule]]\nkey = \"refusal-record\"\nvalue = [{words}]\nsource = \"s\"\n"
            ),
            "mine",
        )
        .unwrap()
    }

    #[test]
    fn leaving_against_advice_with_an_interpreter_is_one_record_and_valid() {
        assert!(validate(&asked(), now()).is_empty());
        let json = serde_json::to_value(asked()).unwrap();
        let back: Asked = serde_json::from_value(json).unwrap();
        assert_eq!(back, asked());
    }

    #[test]
    fn the_us_rules_items_are_covered_by_a_pack_and_not_known_is_not_done() {
        // The US emergency rule, as the spec reads it (42 CFR 489.24(d)(3)): a description of what was refused, the risks
        // and benefits explained, and reasonable steps for a written refusal. As a pack's words:
        let us = pack("\"described\", \"risks-explained\", \"benefits-explained\", \"written\"");
        assert!(pack_gaps(&asked(), &us).is_empty());
        let unsure = Asked {
            explained: Explained {
                benefits: Known::NotKnown,
                ..asked().explained
            },
            written: Known::No,
            ..asked()
        };
        assert_eq!(
            pack_gaps(&unsure, &us),
            vec!["benefits-explained", "written"]
        );
    }

    #[test]
    fn a_places_own_items_are_covered_too_and_no_rule_is_not_a_rule_met() {
        let own =
            pack("\"may-change-mind\", \"in-private\", \"signed\", \"alternatives-explained\"");
        assert_eq!(
            pack_gaps(&asked(), &own),
            vec!["in-private", "signed", "alternatives-explained"]
        );
        // The built-in pack for England and Wales sets no refusal-record: no gaps, and the rule is absent, not met.
        let ew = crate::refusal_rules::england_wales().unwrap();
        assert!(ew.rule("refusal-record").is_none());
        assert!(pack_gaps(&asked(), ew).is_empty());
    }

    #[test]
    fn what_does_not_fit_is_refused_and_named() {
        let neither = Asked {
            refused: None,
            accepted: None,
            leaving_against_advice: false,
            ..asked()
        };
        assert_eq!(validate(&neither, now()).len(), 1);
        let left = Asked {
            refused: None,
            ..asked()
        };
        assert_eq!(validate(&left, now()).len(), 1);
        let private_with = Asked {
            in_private: Known::Yes,
            ..asked()
        };
        assert_eq!(validate(&private_with, now()).len(), 1);
        let signed_unwritten = Asked {
            written: Known::No,
            signed: Known::Yes,
            ..asked()
        };
        assert_eq!(validate(&signed_unwritten, now()).len(), 1);
        let future = Asked {
            at: "2027-01-01T00:00:00Z".into(),
            offered: " ".into(),
            ..asked()
        };
        assert_eq!(validate(&future, now()).len(), 2);
    }
}
