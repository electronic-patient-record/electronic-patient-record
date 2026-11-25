//! Masking (EH-8, the design for SE-4; `spec/access-control/masking.md`): what a list of labelled entries becomes for one
//! viewer, with no database. **Nothing calls it yet and no record carries a label.**
//!
//! - **A whole entry is withheld, never a part.** A title with its code removed still tells.
//! - **It counts and never says what.** The answer for a viewer is the same whatever the withheld entries say: only how
//!   many there are can differ ([`Masked`]).
//! - **It fails closed.** Labels that cannot be read, and a decision that is not a plain allow, withhold the entry.
//! - **The answer stays valid.** An empty or short list that has something withheld is [`Completeness::Partial`] and
//!   never "none", and a shown problem does not point at a withheld one ([`show_problem`]).
//! - **A write does not undo what the writer could not see** ([`keep_withheld_cause`]): a whole record is sent each time,
//!   so someone who was shown no cause would otherwise remove it.
//!
//! Whether a viewer may be told even the count is not decided for a patient or a proxy (OQ-PI5); these lists refuse them.

#![deny(
    clippy::arithmetic_side_effects,
    clippy::default_numeric_fallback,
    clippy::integer_division,
    clippy::modulo_arithmetic,
    clippy::as_conversions,
    clippy::string_slice
)]

use crate::policy::{Decision, Label, Obligation};
use crate::problem::ProblemRecord;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// The labels of one entry, as read from where it is stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Labels {
    /// Every code is a label this version of the service knows. None is an empty set.
    Known(BTreeSet<Label>),
    /// At least one is not. It may be the strictest of all, so the entry is withheld from everyone.
    Unreadable,
}

impl Labels {
    /// Read stored codes. Each must be exactly as [`Label::code`] writes it.
    #[must_use]
    pub fn read<'a>(codes: impl IntoIterator<Item = &'a str>) -> Labels {
        codes
            .into_iter()
            .map(Label::parse)
            .collect::<Option<BTreeSet<Label>>>()
            .map_or(Labels::Unreadable, Labels::Known)
    }
}

/// Whether a decision lets the whole entry be shown: an allow that hides nothing.
#[must_use]
pub fn shows(decision: &Decision) -> bool {
    match decision {
        Decision::Allow(obligations) => {
            !obligations.iter().any(|o| matches!(o, Obligation::Mask(_)))
        }
        Decision::Deny { .. } => false,
    }
}

/// Whether a list is all there is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "completeness", rename_all = "kebab-case")]
pub enum Completeness {
    /// Nothing was withheld from this viewer.
    Whole,
    /// Some entries were. Whatever the rest amounts to is not the whole picture, and an empty rest is not "none".
    Partial {
        /// How many. Never which, or of what kind.
        withheld: usize,
    },
}

/// A list as one viewer may have it. The withheld entries are gone; only their number and ids are kept, and the ids are for
/// the service's own use and not for an answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Masked<T> {
    /// What may be shown, in the list's order.
    pub shown: Vec<T>,
    /// Whether that is all.
    pub completeness: Completeness,
    /// The ids withheld, in lower case. **Never in an answer.**
    pub withheld_ids: BTreeSet<String>,
}

/// Withhold from `entries` what the viewer may not see. `decide` is asked once for each entry whose labels can be read, with
/// those labels, and is the record decision ([`crate::policy::evaluate`]) for this viewer.
#[must_use]
pub fn mask<T>(
    entries: impl IntoIterator<Item = T>,
    id: impl Fn(&T) -> &str,
    labels: impl Fn(&T) -> Labels,
    decide: impl Fn(BTreeSet<Label>) -> Decision,
) -> Masked<T> {
    let mut shown = Vec::new();
    // One for each entry, so that several versions of one entry are counted apart.
    let mut hidden = Vec::new();
    for entry in entries {
        let seen = match labels(&entry) {
            Labels::Known(set) => shows(&decide(set)),
            Labels::Unreadable => false,
        };
        if seen {
            shown.push(entry);
        } else {
            hidden.push(id(&entry).to_ascii_lowercase());
        }
    }
    Masked {
        shown,
        completeness: match hidden.len() {
            0 => Completeness::Whole,
            withheld => Completeness::Partial { withheld },
        },
        withheld_ids: hidden.into_iter().collect(),
    }
}

/// A problem as it may be shown when the problems in `withheld_ids` may not: it does not say that one of them causes it.
/// That it has a cause at all is left out, because "caused by something restricted" is part of what is restricted.
#[must_use]
pub fn show_problem(mut record: ProblemRecord, withheld_ids: &BTreeSet<String>) -> ProblemRecord {
    record.due_to = record
        .due_to
        .filter(|cause| !withheld_ids.contains(&cause.to_ascii_lowercase()));
    record
}

/// The record to store when a writer who was shown `stored` through [`show_problem`] sends `sent`. A cause they could not
/// see, and did not replace with one they could, stays.
#[must_use]
pub fn keep_withheld_cause(
    mut sent: ProblemRecord,
    stored: &ProblemRecord,
    withheld_ids: &BTreeSet<String>,
) -> ProblemRecord {
    if sent.due_to.is_none()
        && let Some(cause) = &stored.due_to
        && withheld_ids.contains(&cause.to_ascii_lowercase())
    {
        sent.due_to = Some(cause.clone());
    }
    sent
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::{Action, Consent, Purpose, Relationship, Request, Resource, Role, Subject};
    use crate::problem::{self, Certainty, Link, Status};

    #[derive(Debug, Clone, PartialEq)]
    struct Entry {
        id: String,
        labels: Vec<&'static str>,
        record: ProblemRecord,
    }

    fn entry(id: &str, labels: &[&'static str], title: &str, due_to: Option<&str>) -> Entry {
        Entry {
            id: id.into(),
            labels: labels.to_vec(),
            record: ProblemRecord {
                title: title.into(),
                coding: None,
                certainty: Certainty::Confirmed,
                status: Status::Active,
                severity: None,
                body_site: None,
                onset: None,
                abated: None,
                due_to: due_to.map(String::from),
            },
        }
    }

    fn decide(role: Role) -> impl Fn(BTreeSet<Label>) -> Decision {
        move |labels| {
            crate::policy::evaluate(&Request {
                subject: Subject {
                    roles: BTreeSet::from([role]),
                    ..Subject::default()
                },
                resource: Resource::Condition,
                action: Action::Read,
                labels,
                relationship: Relationship::CareTeam,
                purpose: Purpose::Treatment,
                consent: Consent::NotRecorded,
                break_glass: None,
                now: 0,
            })
        }
    }

    fn masked(role: Role, entries: &[Entry]) -> Masked<Entry> {
        mask(
            entries.to_vec(),
            |e| e.id.as_str(),
            |e| Labels::read(e.labels.iter().copied()),
            decide(role),
        )
    }

    /// Everything a viewer would be given, as text.
    fn answer(role: Role, entries: &[Entry]) -> String {
        let m = masked(role, entries);
        let shown: Vec<(String, ProblemRecord)> = m
            .shown
            .iter()
            .map(|e| {
                (
                    e.id.clone(),
                    show_problem(e.record.clone(), &m.withheld_ids),
                )
            })
            .collect();
        let links: Vec<Link> = shown
            .iter()
            .map(|(id, r)| Link {
                id: id.clone(),
                standing: true,
                refuted: false,
                due_to: r.due_to.clone(),
            })
            .collect();
        serde_json::json!({
            "shown": shown,
            "completeness": m.completeness,
            "summary": problem::summarise(shown.iter().map(|(_, r)| r)),
            "causes": problem::causes(&links),
        })
        .to_string()
    }

    #[test]
    fn a_label_that_cannot_be_read_withholds_the_entry_from_everyone() {
        assert_eq!(Labels::read([]), Labels::Known(BTreeSet::new()));
        assert_eq!(
            Labels::read(["PSY", "HIV"]),
            Labels::Known(BTreeSet::from([Label::Psychiatry, Label::Hiv]))
        );
        for codes in [&["psy"][..], &["PSY", "STD"], &[""], &[" PSY"]] {
            assert_eq!(Labels::read(codes.iter().copied()), Labels::Unreadable);
            let one = [Entry {
                labels: codes.to_vec(),
                ..entry("a", &[], "t", None)
            }];
            for role in Role::ALL {
                assert!(masked(role, &one).shown.is_empty(), "{role:?} {codes:?}");
            }
        }
    }

    #[test]
    fn an_entry_is_whole_or_withheld_and_the_count_is_right() {
        let list = [
            entry("a", &[], "Asthma", None),
            entry("b", &["SUD"], "Dependence", None),
            entry("c", &["PSY", "SUD"], "Both", None),
            entry("d", &["PSY"], "Depression", None),
        ];
        let seen = |role| {
            let m = masked(role, &list);
            let ids: Vec<String> = m.shown.iter().map(|e| e.id.clone()).collect();
            (ids.join(""), m.completeness)
        };
        // A nurse sees PSY and not SUD: the entry with both is withheld whole.
        assert_eq!(
            seen(Role::Nurse),
            ("ad".into(), Completeness::Partial { withheld: 2 })
        );
        // A clinician sees SUD only with the patient's consent, which is not recorded here.
        assert_eq!(
            seen(Role::Clinician),
            ("ad".into(), Completeness::Partial { withheld: 2 })
        );
        assert_eq!(
            masked(Role::Nurse, &list[..1]).completeness,
            Completeness::Whole
        );
        // A role with no access at all sees none, and is not told the list is whole.
        assert_eq!(
            seen(Role::Receptionist),
            (String::new(), Completeness::Partial { withheld: 4 })
        );
    }

    #[test]
    fn an_empty_rest_is_never_whole() {
        let hidden = [entry("a", &["V"], "t", None)];
        let m = masked(Role::Nurse, &hidden);
        assert!(m.shown.is_empty());
        assert_eq!(m.completeness, Completeness::Partial { withheld: 1 });
        assert_eq!(
            serde_json::json!(m.completeness),
            serde_json::json!({"completeness": "partial", "withheld": 1_u32})
        );
    }

    /// The answer does not depend on what is withheld, for any role and any label: only on how much.
    #[test]
    fn what_is_withheld_changes_nothing_in_the_answer() {
        let mut compared = 0_usize;
        for role in Role::ALL {
            for label in Label::ALL {
                let with = |title: &str, cause: Option<&str>, id: &str| {
                    let mut secret = entry(id, &[label.code()], title, cause);
                    if title == "Other" {
                        secret.record.certainty = Certainty::Refuted;
                        secret.record.status = Status::Inactive;
                        secret.record.onset = Some("1999".into());
                    }
                    [
                        entry("a", &[], "Asthma", Some(id)),
                        secret,
                        entry("c", &[], "Cough", Some("a")),
                    ]
                };
                let one = with("Secret", None, "b");
                if masked(role, &one).completeness == Completeness::Whole {
                    continue;
                }
                compared += 1;
                let first = answer(role, &one);
                for other in [
                    with("Other", None, "b"),
                    with("Secret", Some("c"), "b"),
                    with("Secret", None, "zzz"),
                ] {
                    assert_eq!(first, answer(role, &other), "{role:?} {label:?}");
                }
                assert!(
                    !first.contains("Secret") && !first.contains("\"b\""),
                    "{first}"
                );
            }
        }
        // Not an empty loop: six labels and seventeen roles, most of which see none of them.
        assert!(compared > 60, "{compared}");
    }

    #[test]
    fn a_shown_problem_does_not_point_at_a_withheld_one() {
        let list = [
            entry("s", &["V"], "Syndrome", None),
            entry("l", &[], "Lung disease", Some("S")),
        ];
        let m = masked(Role::Nurse, &list);
        assert_eq!(
            show_problem(list[1].record.clone(), &m.withheld_ids).due_to,
            None
        );
        // Someone who sees both sees the link.
        assert_eq!(
            show_problem(list[1].record.clone(), &BTreeSet::new()).due_to,
            Some("S".into())
        );
    }

    #[test]
    fn a_writer_who_could_not_see_a_cause_does_not_remove_it() {
        let withheld = BTreeSet::from(["s".to_string()]);
        let stored = entry("l", &[], "Lung disease", Some("S")).record;
        let sent = ProblemRecord {
            title: "Lung disease, worse".into(),
            ..show_problem(stored.clone(), &withheld)
        };
        assert_eq!(sent.due_to, None);
        let kept = keep_withheld_cause(sent.clone(), &stored, &withheld);
        assert_eq!(kept.due_to, Some("S".into()));
        assert_eq!(kept.title, "Lung disease, worse");
        // One they could see and took away is taken away; one they put in its place is theirs.
        assert_eq!(
            keep_withheld_cause(sent.clone(), &stored, &BTreeSet::new()).due_to,
            None
        );
        let replaced = ProblemRecord {
            due_to: Some("x".into()),
            ..sent
        };
        assert_eq!(
            keep_withheld_cause(replaced, &stored, &withheld).due_to,
            Some("x".into())
        );
    }
}
