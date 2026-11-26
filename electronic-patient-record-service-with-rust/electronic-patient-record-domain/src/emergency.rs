//! What a person who opens break-the-glass says about the emergency (RF-6, PI-10; `spec/patient-refusal-of-consent`,
//! `spec/access-control`). Four answers, each required on a new grant and each **somebody's word**: what is at risk, how
//! soon, whether the person can be asked now, and, if they could, what they said.
//!
//! **Nothing is worked out here**: not from a diagnosis, an age, a word in the reason or a record. "Not known" is an
//! answer and decides nothing. What a grant lifts does not depend on these answers yet (OQ-PI8 is open); they are held
//! with the grant so that its review can read them.

use serde::{Deserialize, Serialize};

/// The longest words of what the person said.
pub const SAID_LIMIT: usize = 500;

/// What is at risk if nobody reads the record now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Threat {
    /// A life.
    Life,
    /// A limb, or a function as serious.
    Limb,
    /// Neither: a grant for another reason, which is said so.
    Neither,
    /// The person opening it does not know.
    NotKnown,
}

/// How soon harm would come.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TimeToHarm {
    /// Within minutes.
    Minutes,
    /// Within hours.
    Hours,
    /// Within days or longer.
    Days,
    /// The person opening it does not know.
    NotKnown,
}

/// Whether the patient can be asked now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CanBeAsked {
    /// They can, and were.
    Yes,
    /// They cannot.
    No,
    /// The person opening it does not know.
    NotKnown,
}

/// The four answers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Emergency {
    /// What is at risk.
    pub threat: Threat,
    /// How soon.
    pub time_to_harm: TimeToHarm,
    /// Whether the patient can be asked now.
    pub can_be_asked: CanBeAsked,
    /// What they said when they were asked. Required when they can be, and refused when they cannot or it is not known.
    #[serde(default)]
    pub asked_and_said: Option<String>,
}

macro_rules! words {
    ($t:ty { $($v:ident => $w:literal),+ $(,)? }) => {
        impl $t {
            /// Every value, in a fixed order (the stored words are checked against the database, PR-3).
            pub const ALL: &'static [$t] = &[$(<$t>::$v),+];

            /// The stored word, as serde writes it.
            #[must_use]
            pub fn as_str(self) -> &'static str {
                match self { $(<$t>::$v => $w),+ }
            }

            /// Read what [`Self::as_str`] writes, and nothing else.
            #[must_use]
            pub fn parse(word: &str) -> Option<Self> {
                Self::ALL.iter().copied().find(|v| v.as_str() == word)
            }
        }
    };
}

words!(Threat { Life => "life", Limb => "limb", Neither => "neither", NotKnown => "not-known" });
words!(TimeToHarm { Minutes => "minutes", Hours => "hours", Days => "days", NotKnown => "not-known" });
words!(CanBeAsked { Yes => "yes", No => "no", NotKnown => "not-known" });

/// What is wrong with the answers. The words name a field and never quote it.
#[must_use]
pub fn validate(e: &Emergency) -> Vec<String> {
    let mut out = Vec::new();
    match (e.can_be_asked, &e.asked_and_said) {
        (CanBeAsked::Yes, None) => {
            out.push("asked_and_said is required when the patient can be asked".into());
        }
        (CanBeAsked::Yes, Some(said)) => {
            if said.trim().is_empty() {
                out.push("asked_and_said must not be blank".into());
            } else if said.chars().count() > SAID_LIMIT || said.chars().any(char::is_control) {
                out.push(format!(
                    "asked_and_said must be up to {SAID_LIMIT} characters, with no control characters"
                ));
            }
        }
        (CanBeAsked::No | CanBeAsked::NotKnown, Some(_)) => {
            out.push("asked_and_said is for a patient who could be asked".into());
        }
        (CanBeAsked::No | CanBeAsked::NotKnown, None) => {}
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn of(can_be_asked: CanBeAsked, said: Option<&str>) -> Emergency {
        Emergency {
            threat: Threat::NotKnown,
            time_to_harm: TimeToHarm::NotKnown,
            can_be_asked,
            asked_and_said: said.map(str::to_string),
        }
    }

    #[test]
    fn what_they_said_is_required_when_they_could_be_asked_and_only_then() {
        assert!(validate(&of(CanBeAsked::Yes, Some("said yes, read it"))).is_empty());
        assert_eq!(validate(&of(CanBeAsked::Yes, None)).len(), 1);
        assert_eq!(validate(&of(CanBeAsked::Yes, Some("  "))).len(), 1);
        assert_eq!(
            validate(&of(CanBeAsked::Yes, Some(&"x".repeat(SAID_LIMIT + 1)))).len(),
            1
        );
        assert_eq!(validate(&of(CanBeAsked::Yes, Some("a\u{7}b"))).len(), 1);
        for nobody in [CanBeAsked::No, CanBeAsked::NotKnown] {
            assert!(validate(&of(nobody, None)).is_empty(), "{nobody:?}");
            assert_eq!(validate(&of(nobody, Some("x"))).len(), 1, "{nobody:?}");
        }
    }

    #[test]
    fn not_known_is_an_answer_that_is_accepted_everywhere() {
        let e = of(CanBeAsked::NotKnown, None);
        assert!(validate(&e).is_empty());
        assert_eq!(e.threat.as_str(), "not-known");
    }

    #[test]
    fn every_answer_is_required_and_no_other_field_is_taken() {
        for missing in ["threat", "time_to_harm", "can_be_asked"] {
            let mut v = serde_json::json!({"threat": "life", "time_to_harm": "minutes", "can_be_asked": "no"});
            v.as_object_mut().unwrap().remove(missing);
            assert!(serde_json::from_value::<Emergency>(v).is_err(), "{missing}");
        }
        let extra = serde_json::json!({"threat": "life", "time_to_harm": "minutes", "can_be_asked": "no", "age": 3});
        assert!(serde_json::from_value::<Emergency>(extra).is_err());
    }

    #[test]
    fn stored_words_are_the_serde_words_and_read_back() {
        for t in Threat::ALL {
            assert_eq!(serde_json::json!(t), t.as_str());
            assert_eq!(Threat::parse(t.as_str()), Some(*t));
        }
        for t in TimeToHarm::ALL {
            assert_eq!(serde_json::json!(t), t.as_str());
            assert_eq!(TimeToHarm::parse(t.as_str()), Some(*t));
        }
        for c in CanBeAsked::ALL {
            assert_eq!(serde_json::json!(c), c.as_str());
            assert_eq!(CanBeAsked::parse(c.as_str()), Some(*c));
        }
        assert_eq!(Threat::parse("urgent"), None);
    }
}
