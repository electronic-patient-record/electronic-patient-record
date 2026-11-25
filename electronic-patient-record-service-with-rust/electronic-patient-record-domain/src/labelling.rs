//! Who puts a security label on an entry and who lifts it (OQ-PI7, decided 2026-11-25; `spec/access-control/masking.md`).
//! It is pure; `entry_labels.rs` in the service keeps what it returns, for the problem list, relative history and typing.
//!
//! - **A treating clinician adds one**, on their own judgement or because the patient asked.
//! - **Another clinician who treats the patient lifts it**, with a reason. Never the one who added it.
//! - **The one for custody is lifted only because the patient asked** (`spec/custodial-health`), and a clinician records it.
//! - **None is added by the service**, but the one for a stay in custody. Any other rule needs a reviewed list.
//! - **Nothing is removed from the history.** Who added, who lifted and why stay, and a label may be added again.
//!
//! A label is added with a [`Basis`] and no words, since words beside a label can say what it hides. A reason for lifting
//! one is words, and is as protected as the entry.

#![deny(
    clippy::arithmetic_side_effects,
    clippy::default_numeric_fallback,
    clippy::integer_division,
    clippy::modulo_arithmetic,
    clippy::as_conversions,
    clippy::string_slice
)]

use crate::policy::{Label, Relationship, Role};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// The longest reason for lifting a label.
pub const REASON_LIMIT: usize = 500;

/// Why a label was added.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Basis {
    /// A clinician's judgement.
    Clinical,
    /// The patient asked.
    PatientAsked,
    /// A stay in custody: the service's own, and the only one.
    Custody,
}

impl Basis {
    /// Every basis.
    pub const ALL: [Basis; 3] = [Basis::Clinical, Basis::PatientAsked, Basis::Custody];

    /// As stored and as sent.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Basis::Clinical => "clinical",
            Basis::PatientAsked => "patient-asked",
            Basis::Custody => "custody",
        }
    }

    /// Read what [`Basis::word`] writes, and nothing else.
    #[must_use]
    pub fn parse(word: &str) -> Option<Basis> {
        Basis::ALL.into_iter().find(|b| b.word() == word)
    }
}

/// Who acts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Actor {
    /// A person: who they are, the roles they hold, and what they are to the patient.
    Person {
        /// The token subject.
        subject: String,
        /// Their roles.
        roles: BTreeSet<Role>,
        /// Their relationship to the patient.
        relationship: Relationship,
    },
    /// The service, by a rule.
    Service,
}

/// One thing that happened to a label, as kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "act", rename_all = "kebab-case")]
pub enum Act {
    /// Added.
    Added {
        /// Which label ([`Label::code`]).
        label: String,
        /// Who; `None` for the service.
        by: Option<String>,
        /// Why.
        basis: Basis,
    },
    /// Lifted.
    Lifted {
        /// Which label.
        label: String,
        /// Who.
        by: String,
        /// On whose decision.
        basis: Basis,
        /// Why, in words.
        reason: String,
    },
}

/// Why an act was refused. The words name no label and quote nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// Only a clinician who treats the patient adds or lifts one.
    NotTreating,
    /// The service adds only the one for custody, and a person does not say custody.
    NotTheirs,
    /// It is there already.
    AlreadyThere,
    /// It is not there.
    NotThere,
    /// The person who added it does not lift it.
    Own,
    /// The one for custody is lifted only because the patient asked.
    Custody,
    /// Lifting needs a reason of 1 to [`REASON_LIMIT`] characters.
    Reason,
}

impl Refusal {
    /// The refusal in words.
    #[must_use]
    pub fn message(self) -> &'static str {
        match self {
            Refusal::NotTreating => "only a clinician who treats the patient adds or lifts a label",
            Refusal::NotTheirs => "a label of this kind is not added this way",
            Refusal::AlreadyThere => "the label is there already",
            Refusal::NotThere => "the label is not there",
            Refusal::Own => "a label is lifted by someone other than the person who added it",
            Refusal::Custody => "a label of this kind is lifted only because the patient asked",
            Refusal::Reason => "lifting a label needs a reason of 1 to 500 characters",
        }
    }
}

/// The act that stands for `label`: the last one, when it added it.
fn standing(history: &[Act], label: Label) -> Option<&Act> {
    history
        .iter()
        .rev()
        .find(|a| match a {
            Act::Added { label: l, .. } | Act::Lifted { label: l, .. } => l == label.code(),
        })
        .filter(|a| matches!(a, Act::Added { .. }))
}

/// The labels of an entry, as codes, for `Labels::read` of `masking.rs`. A code this version does not know is kept, so
/// that it withholds and is not lost.
#[must_use]
pub fn current(history: &[Act]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for act in history {
        match act {
            Act::Added { label, .. } => {
                out.insert(label.clone());
            }
            Act::Lifted { label, .. } => {
                out.remove(label);
            }
        }
    }
    out
}

/// The subject of a clinician who treats the patient.
fn treating(actor: &Actor) -> Result<&str, Refusal> {
    match actor {
        Actor::Person {
            subject,
            roles,
            relationship,
        } if roles.contains(&Role::Clinician)
            && relationship.treating()
            && !subject.trim().is_empty() =>
        {
            Ok(subject)
        }
        Actor::Person { .. } | Actor::Service => Err(Refusal::NotTreating),
    }
}

/// Add `label`: the act to keep.
///
/// # Errors
///
/// A [`Refusal`].
pub fn add(history: &[Act], actor: &Actor, label: Label, basis: Basis) -> Result<Act, Refusal> {
    let by = match (actor, basis) {
        // The service's one rule, and its one label.
        (Actor::Service, Basis::Custody) if label == Label::Restricted => None,
        (Actor::Service, _) | (Actor::Person { .. }, Basis::Custody) => {
            return Err(Refusal::NotTheirs);
        }
        (Actor::Person { .. }, Basis::Clinical | Basis::PatientAsked) => {
            Some(treating(actor)?.to_string())
        }
    };
    if standing(history, label).is_some() {
        return Err(Refusal::AlreadyThere);
    }
    Ok(Act::Added {
        label: label.code().to_string(),
        by,
        basis,
    })
}

/// Lift `label`: the act to keep.
///
/// # Errors
///
/// A [`Refusal`]. Whether it is there is said only to someone who may lift one.
pub fn lift(
    history: &[Act],
    actor: &Actor,
    label: Label,
    basis: Basis,
    reason: &str,
) -> Result<Act, Refusal> {
    let by = treating(actor)?;
    if basis == Basis::Custody {
        return Err(Refusal::NotTheirs);
    }
    let Some(Act::Added {
        by: adder,
        basis: added,
        ..
    }) = standing(history, label)
    else {
        return Err(Refusal::NotThere);
    };
    if *added == Basis::Custody && basis != Basis::PatientAsked {
        return Err(Refusal::Custody);
    }
    if adder.as_deref() == Some(by) {
        return Err(Refusal::Own);
    }
    let reason = reason.trim();
    if reason.is_empty() || reason.chars().count() > REASON_LIMIT {
        return Err(Refusal::Reason);
    }
    Ok(Act::Lifted {
        label: label.code().to_string(),
        by: by.to_string(),
        basis,
        reason: reason.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::masking::Labels;

    fn person(subject: &str, role: Role, relationship: Relationship) -> Actor {
        Actor::Person {
            subject: subject.into(),
            roles: BTreeSet::from([role]),
            relationship,
        }
    }

    fn doctor(subject: &str) -> Actor {
        person(subject, Role::Clinician, Relationship::CareTeam)
    }

    #[test]
    fn only_a_clinician_who_treats_the_patient_adds_or_lifts_one() {
        let there = [add(&[], &doctor("a"), Label::Hiv, Basis::Clinical).unwrap()];
        for role in Role::ALL {
            for relationship in [
                Relationship::None,
                Relationship::CareTeam,
                Relationship::ActiveEncounter,
                Relationship::SelfPatient,
                Relationship::Proxy,
            ] {
                let who = person("b", role, relationship);
                let may = role == Role::Clinician && relationship.treating();
                for basis in [Basis::Clinical, Basis::PatientAsked] {
                    assert_eq!(
                        add(&[], &who, Label::Hiv, basis).is_ok(),
                        may,
                        "{role:?} {relationship:?}"
                    );
                }
                assert_eq!(
                    lift(&there, &who, Label::Hiv, Basis::Clinical, "r").is_ok(),
                    may,
                    "{role:?} {relationship:?}"
                );
                // Someone who may not is not told whether it is there.
                if !may {
                    assert_eq!(
                        lift(&[], &who, Label::Hiv, Basis::Clinical, "r"),
                        lift(&there, &who, Label::Hiv, Basis::Clinical, "r")
                    );
                }
            }
        }
        assert_eq!(
            add(&[], &doctor("  "), Label::Hiv, Basis::Clinical),
            Err(Refusal::NotTreating)
        );
        assert_eq!(
            lift(&there, &Actor::Service, Label::Hiv, Basis::Clinical, "r"),
            Err(Refusal::NotTreating)
        );
    }

    #[test]
    fn the_service_adds_only_the_one_for_custody_and_a_person_never_says_custody() {
        for label in Label::ALL {
            for basis in [Basis::Clinical, Basis::PatientAsked, Basis::Custody] {
                let expected = label == Label::Restricted && basis == Basis::Custody;
                assert_eq!(
                    add(&[], &Actor::Service, label, basis).is_ok(),
                    expected,
                    "{label:?} {basis:?}"
                );
            }
            assert_eq!(
                add(&[], &doctor("a"), label, Basis::Custody),
                Err(Refusal::NotTheirs)
            );
        }
    }

    #[test]
    fn the_person_who_added_it_does_not_lift_it_and_anyone_else_gives_a_reason() {
        let h = [add(&[], &doctor("a"), Label::Psychiatry, Basis::PatientAsked).unwrap()];
        assert_eq!(
            lift(&h, &doctor("a"), Label::Psychiatry, Basis::Clinical, "r"),
            Err(Refusal::Own)
        );
        for reason in ["", "  ", &"x".repeat(501)] {
            assert_eq!(
                lift(&h, &doctor("b"), Label::Psychiatry, Basis::Clinical, reason),
                Err(Refusal::Reason)
            );
        }
        assert_eq!(
            lift(&h, &doctor("b"), Label::Hiv, Basis::Clinical, "r"),
            Err(Refusal::NotThere)
        );
        let lifted = lift(
            &h,
            &doctor("b"),
            Label::Psychiatry,
            Basis::Clinical,
            " no longer wanted ",
        )
        .unwrap();
        assert_eq!(
            lifted,
            Act::Lifted {
                label: "PSY".into(),
                by: "b".into(),
                basis: Basis::Clinical,
                reason: "no longer wanted".into()
            }
        );
        // The one for custody is lifted because the patient asked, and not on anyone's judgement.
        let custody = [add(&[], &Actor::Service, Label::Restricted, Basis::Custody).unwrap()];
        let lifting = |basis| lift(&custody, &doctor("a"), Label::Restricted, basis, "asked");
        assert_eq!(lifting(Basis::Clinical), Err(Refusal::Custody));
        assert_eq!(lifting(Basis::Custody), Err(Refusal::NotTheirs));
        assert!(lifting(Basis::PatientAsked).is_ok());
    }

    #[test]
    fn a_label_may_be_added_again_and_the_history_is_kept() {
        let mut h = vec![add(&[], &doctor("a"), Label::Hiv, Basis::Clinical).unwrap()];
        assert_eq!(
            add(&h, &doctor("b"), Label::Hiv, Basis::Clinical),
            Err(Refusal::AlreadyThere)
        );
        h.push(lift(&h, &doctor("b"), Label::Hiv, Basis::Clinical, "asked").unwrap());
        assert!(current(&h).is_empty());
        // Added again by the one who lifted it: now that is the one who may not lift it.
        h.push(add(&h, &doctor("b"), Label::Hiv, Basis::PatientAsked).unwrap());
        assert_eq!(
            lift(&h, &doctor("b"), Label::Hiv, Basis::Clinical, "r"),
            Err(Refusal::Own)
        );
        assert!(lift(&h, &doctor("a"), Label::Hiv, Basis::Clinical, "r").is_ok());
        assert_eq!(current(&h), BTreeSet::from(["HIV".to_string()]));
        assert_eq!(h.len(), 3);
        let back: Vec<Act> = serde_json::from_value(serde_json::to_value(&h).unwrap()).unwrap();
        assert_eq!(back, h);
    }

    #[test]
    fn the_words_of_a_basis_are_the_ones_it_is_sent_as() {
        for b in Basis::ALL {
            assert_eq!(Basis::parse(b.word()), Some(b));
            assert_eq!(serde_json::json!(b), b.word());
        }
        assert_eq!(Basis::parse("Clinical"), None);
    }

    #[test]
    fn what_stands_is_the_last_thing_that_happened_and_an_unknown_code_is_kept() {
        let mut h = Vec::new();
        for label in Label::ALL {
            h.push(add(&h, &doctor("a"), label, Basis::Clinical).unwrap());
        }
        h.push(lift(&h, &doctor("b"), Label::Hiv, Basis::Clinical, "r").unwrap());
        let now = current(&h);
        assert_eq!(now.len(), Label::ALL.len() - 1);
        assert!(!now.contains("HIV"));
        let newer = [Act::Added {
            label: "SDV".into(),
            by: Some("a".into()),
            basis: Basis::Clinical,
        }];
        let codes = current(&newer);
        assert_eq!(
            Labels::read(codes.iter().map(String::as_str)),
            Labels::Unreadable,
            "it withholds from everyone, and is not lost"
        );
    }
}
