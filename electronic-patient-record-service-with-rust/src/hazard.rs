//! The hazard register (AS-17): the rules for taking a hazard off the active list. Pure: no database, no
//! clock of its own; the caller passes the actor and the time.
//!
//! **Only the Clinical Safety Officer retires a hazard, and must give evidence.** Retiring is not deleting:
//! the register is a series of versions that are never edited or removed (`hazard_store.rs`), so a retired
//! hazard keeps its whole history. Anyone who may write to the register may reopen a retired hazard, with a
//! reason: closing is hard and reopening is easy on purpose.
//!
//! | event | who | needs |
//! |---|---|---|
//! | `Control` | any writer | the hazard is open; a text |
//! | `Retire` | the Clinical Safety Officer only, and not anyone who recorded a control | 1 to 20 evidence items (kind, reference, time taken, SHA-256), a note |
//! | `Reopen` | any writer | the hazard is retired; a reason |
//!
//! The hash proves the evidence did not change after it was recorded. It does not prove the evidence is
//! true; only an independent reviewer or the audit trail narrows that. The register holds no patient data:
//! every text is length-limited and the table is listed in the personal-data inventory.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// The longest title.
pub const TITLE_MAX: usize = 200;
/// The longest control text, note, reason or evidence reference.
pub const TEXT_MAX: usize = 2000;
/// The most evidence items on one retirement.
pub const EVIDENCE_MAX: usize = 20;

/// What a piece of evidence is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceKind {
    /// The change that removed the feature or data flow that caused the hazard.
    RemovalChange,
    /// A test run showing the control works (ideally one that fails when the control is removed).
    TestRun,
    /// A dated review by a named person.
    ReviewRecord,
    /// A query or census showing none of the hazardous condition is left.
    Census,
}

/// One item of evidence that a hazard has been removed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evidence {
    /// What it is.
    pub kind: EvidenceKind,
    /// Where it can be found (a path, a commit, a link). Never patient data.
    pub reference: String,
    /// When the evidence was taken (UTC).
    pub taken_at: DateTime<Utc>,
    /// SHA-256 of the artefact as it was then, 64 lowercase hex digits.
    pub sha256: String,
}

/// Where a hazard stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum State {
    /// On the active list.
    Open,
    /// Taken off the active list by the Clinical Safety Officer, with evidence.
    Retired,
}

impl State {
    /// The word stored in the `state` column.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            State::Open => "open",
            State::Retired => "retired",
        }
    }
}

/// A control someone recorded against a hazard.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Control {
    /// Who recorded it (a token `sub`).
    pub by: String,
    /// What it is.
    pub text: String,
}

/// A hazard as the register holds it after some version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hazard {
    /// A short name.
    pub title: String,
    /// 1 (minor) to 5 (catastrophic), before controls. The raiser's proposal, not validated.
    pub severity: u8,
    /// 1 (very low) to 5 (very high), before controls.
    pub likelihood: u8,
    /// Where it stands.
    pub state: State,
    /// Who raised it.
    pub raised_by: String,
    /// Controls recorded, oldest first.
    pub controls: Vec<Control>,
    /// The evidence of the latest retirement; empty while open.
    pub evidence: Vec<Evidence>,
    /// Who retired it, while retired.
    pub retired_by: Option<String>,
    /// The Clinical Safety Officer's note on the retirement, while retired.
    pub note: Option<String>,
    /// How many times it has been reopened.
    pub reopened: u32,
}

/// Something that can happen to a hazard.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "kebab-case")]
pub enum Event {
    /// Record a control.
    Control {
        /// What the control is.
        text: String,
    },
    /// Take the hazard off the active list.
    Retire {
        /// The evidence it has been removed.
        evidence: Vec<Evidence>,
        /// What was checked and what is left over.
        note: String,
    },
    /// Put a retired hazard back.
    Reopen {
        /// Why.
        reason: String,
    },
}

/// Who is acting. `is_cso` comes from the token's role, never from the request body.
#[derive(Debug, Clone, Copy)]
pub struct Actor<'a> {
    /// The token `sub`.
    pub id: &'a str,
    /// Whether the token holds the `clinical-safety-officer` role.
    pub is_cso: bool,
}

/// Why a step was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// A retirement by anyone but the Clinical Safety Officer.
    NotTheCso,
    /// The hazard is not in a state that allows this.
    WrongState,
    /// A text that must not be blank.
    Blank(&'static str),
    /// A text over its limit.
    TooLong(&'static str),
    /// A rating outside 1 to 5.
    BadRating,
    /// A retirement with no evidence.
    NoEvidence,
    /// More evidence items than allowed.
    TooMuchEvidence,
    /// A hash that is not 64 lowercase hex digits.
    BadHash,
    /// Evidence said to be taken in the future.
    FromTheFuture,
    /// The Clinical Safety Officer recorded a control on this hazard, so cannot also retire it.
    RecordedTheControl,
}

fn text(what: &'static str, value: &str, max: usize) -> Result<String, Refusal> {
    let v = value.trim();
    if v.is_empty() {
        Err(Refusal::Blank(what))
    } else if v.chars().count() > max {
        Err(Refusal::TooLong(what))
    } else {
        Ok(v.to_string())
    }
}

fn rating(n: u8) -> Result<u8, Refusal> {
    if (1..=5).contains(&n) {
        Ok(n)
    } else {
        Err(Refusal::BadRating)
    }
}

fn is_sha256(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

impl Hazard {
    /// A new, open hazard.
    ///
    /// # Errors
    ///
    /// [`Refusal::Blank`], [`Refusal::TooLong`] or [`Refusal::BadRating`].
    pub fn raise(by: &str, title: &str, severity: u8, likelihood: u8) -> Result<Hazard, Refusal> {
        Ok(Hazard {
            title: text("title", title, TITLE_MAX)?,
            severity: rating(severity)?,
            likelihood: rating(likelihood)?,
            state: State::Open,
            raised_by: by.to_string(),
            controls: Vec::new(),
            evidence: Vec::new(),
            retired_by: None,
            note: None,
            reopened: 0,
        })
    }

    /// `severity × likelihood`.
    #[must_use]
    pub fn score(&self) -> u8 {
        self.severity * self.likelihood
    }

    /// The hazard after `event`, or why it was refused.
    ///
    /// # Errors
    ///
    /// A [`Refusal`]; nothing is changed.
    pub fn apply(
        &self,
        actor: &Actor<'_>,
        event: Event,
        now: DateTime<Utc>,
    ) -> Result<Hazard, Refusal> {
        let mut next = self.clone();
        match event {
            Event::Control { text: t } => {
                if self.state != State::Open {
                    return Err(Refusal::WrongState);
                }
                next.controls.push(Control {
                    by: actor.id.to_string(),
                    text: text("control", &t, TEXT_MAX)?,
                });
            }
            Event::Retire { evidence, note } => {
                if !actor.is_cso {
                    return Err(Refusal::NotTheCso);
                }
                if self.state != State::Open {
                    return Err(Refusal::WrongState);
                }
                if self.controls.iter().any(|c| c.by == actor.id) {
                    return Err(Refusal::RecordedTheControl);
                }
                if evidence.is_empty() {
                    return Err(Refusal::NoEvidence);
                }
                if evidence.len() > EVIDENCE_MAX {
                    return Err(Refusal::TooMuchEvidence);
                }
                let mut kept = Vec::with_capacity(evidence.len());
                for e in evidence {
                    if !is_sha256(&e.sha256) {
                        return Err(Refusal::BadHash);
                    }
                    if e.taken_at > now {
                        return Err(Refusal::FromTheFuture);
                    }
                    kept.push(Evidence {
                        reference: text("evidence reference", &e.reference, TEXT_MAX)?,
                        ..e
                    });
                }
                next.evidence = kept;
                next.note = Some(text("note", &note, TEXT_MAX)?);
                next.retired_by = Some(actor.id.to_string());
                next.state = State::Retired;
            }
            Event::Reopen { reason } => {
                if self.state != State::Retired {
                    return Err(Refusal::WrongState);
                }
                // The reason is checked and kept in the version's event, not in the hazard.
                text("reason", &reason, TEXT_MAX)?;
                next.evidence = Vec::new();
                next.note = None;
                next.retired_by = None;
                next.state = State::Open;
                next.reopened += 1;
            }
        }
        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn now() -> DateTime<Utc> {
        "2026-10-28T12:00:00Z".parse().unwrap()
    }

    fn cso() -> Actor<'static> {
        Actor {
            id: "cso-1",
            is_cso: true,
        }
    }

    fn clinician() -> Actor<'static> {
        Actor {
            id: "dr-1",
            is_cso: false,
        }
    }

    fn evidence() -> Evidence {
        Evidence {
            kind: EvidenceKind::TestRun,
            reference: "tests/example.rs".into(),
            taken_at: "2026-10-27T09:00:00Z".parse().unwrap(),
            sha256: HASH.into(),
        }
    }

    fn retire(evidence: Vec<Evidence>) -> Event {
        Event::Retire {
            evidence,
            note: "control verified; nothing left over".into(),
        }
    }

    fn open() -> Hazard {
        Hazard::raise("dr-1", "Wrong patient", 5, 2).unwrap()
    }

    #[test]
    fn a_hazard_is_raised_open_with_a_score() {
        let h = open();
        assert_eq!(h.state, State::Open);
        assert_eq!(h.score(), 10);
        assert!(h.evidence.is_empty());
    }

    #[test]
    fn ratings_and_texts_are_checked() {
        assert_eq!(Hazard::raise("a", "t", 0, 2), Err(Refusal::BadRating));
        assert_eq!(Hazard::raise("a", "t", 2, 6), Err(Refusal::BadRating));
        assert_eq!(Hazard::raise("a", "  ", 2, 2), Err(Refusal::Blank("title")));
        assert_eq!(
            Hazard::raise("a", &"x".repeat(TITLE_MAX + 1), 2, 2),
            Err(Refusal::TooLong("title"))
        );
    }

    #[test]
    fn only_the_cso_retires_a_hazard() {
        let h = open();
        assert_eq!(
            h.apply(&clinician(), retire(vec![evidence()]), now()),
            Err(Refusal::NotTheCso)
        );
        let done = h.apply(&cso(), retire(vec![evidence()]), now()).unwrap();
        assert_eq!(done.state, State::Retired);
        assert_eq!(done.retired_by.as_deref(), Some("cso-1"));
        assert_eq!(done.evidence.len(), 1);
    }

    #[test]
    fn a_retirement_without_evidence_is_refused_even_for_the_cso() {
        assert_eq!(
            open().apply(&cso(), retire(vec![]), now()),
            Err(Refusal::NoEvidence)
        );
    }

    #[test]
    fn evidence_must_have_a_real_hash_a_reference_and_a_past_time() {
        let h = open();
        for bad in ["", "abc", &"A".repeat(64), &"g".repeat(64), &"a".repeat(65)] {
            let e = Evidence {
                sha256: bad.into(),
                ..evidence()
            };
            assert_eq!(
                h.apply(&cso(), retire(vec![e]), now()),
                Err(Refusal::BadHash),
                "{bad}"
            );
        }
        let blank = Evidence {
            reference: "  ".into(),
            ..evidence()
        };
        assert_eq!(
            h.apply(&cso(), retire(vec![blank]), now()),
            Err(Refusal::Blank("evidence reference"))
        );
        let future = Evidence {
            taken_at: "2026-10-29T00:00:00Z".parse().unwrap(),
            ..evidence()
        };
        assert_eq!(
            h.apply(&cso(), retire(vec![future]), now()),
            Err(Refusal::FromTheFuture)
        );
        let many = vec![evidence(); EVIDENCE_MAX + 1];
        assert_eq!(
            h.apply(&cso(), retire(many), now()),
            Err(Refusal::TooMuchEvidence)
        );
    }

    #[test]
    fn a_note_is_required() {
        let e = Event::Retire {
            evidence: vec![evidence()],
            note: " ".into(),
        };
        assert_eq!(open().apply(&cso(), e, now()), Err(Refusal::Blank("note")));
    }

    #[test]
    fn whoever_recorded_a_control_cannot_retire_the_hazard() {
        let h = open()
            .apply(
                &cso(),
                Event::Control {
                    text: "a second check".into(),
                },
                now(),
            )
            .unwrap();
        assert_eq!(
            h.apply(&cso(), retire(vec![evidence()]), now()),
            Err(Refusal::RecordedTheControl)
        );
        // A different Clinical Safety Officer can.
        let other = Actor {
            id: "cso-2",
            is_cso: true,
        };
        assert!(h.apply(&other, retire(vec![evidence()]), now()).is_ok());
    }

    #[test]
    fn a_retired_hazard_takes_no_control_and_cannot_be_retired_again() {
        let done = open()
            .apply(&cso(), retire(vec![evidence()]), now())
            .unwrap();
        let control = Event::Control { text: "x".into() };
        assert_eq!(
            done.apply(&clinician(), control, now()),
            Err(Refusal::WrongState)
        );
        assert_eq!(
            done.apply(&cso(), retire(vec![evidence()]), now()),
            Err(Refusal::WrongState)
        );
    }

    #[test]
    fn anyone_reopens_with_a_reason_and_the_evidence_is_cleared() {
        let done = open()
            .apply(&cso(), retire(vec![evidence()]), now())
            .unwrap();
        let reopen = |reason: &str| Event::Reopen {
            reason: reason.into(),
        };
        assert_eq!(
            done.apply(&clinician(), reopen(" "), now()),
            Err(Refusal::Blank("reason"))
        );
        let back = done
            .apply(&clinician(), reopen("seen again on ward 4"), now())
            .unwrap();
        assert_eq!(back.state, State::Open);
        assert!(back.evidence.is_empty());
        assert_eq!(back.retired_by, None);
        assert_eq!(back.reopened, 1);
        // An open hazard cannot be reopened.
        assert_eq!(
            back.apply(&clinician(), reopen("again"), now()),
            Err(Refusal::WrongState)
        );
    }

    #[test]
    fn the_role_is_not_taken_from_the_event() {
        // The actor decides; an event has no field a caller could set to claim the role.
        let json = serde_json::json!({
            "event": "retire", "is_cso": true, "evidence": [], "note": "n"
        });
        let parsed: Event = serde_json::from_value(json).unwrap();
        assert_eq!(
            open().apply(&clinician(), parsed, now()),
            Err(Refusal::NotTheCso)
        );
    }
}
