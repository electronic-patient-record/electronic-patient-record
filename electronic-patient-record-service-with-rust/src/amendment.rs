//! Patient-requested amendment (RC-4, first slice): the rules for one request, pure. Nothing stores,
//! serves or shows a request yet; the caller passes the request, the event and the time, and gets the
//! next request or a [`Refusal`].
//!
//! The clinical record is append-only, so an amendment never edits or removes what was written. An
//! accepted request ends with the correction recorded as a **new version** of the disputed record
//! (`Apply`), and a refused one stays visible beside the record with the patient's statement of
//! disagreement. The shape follows HIPAA 45 CFR 164.526 as I read it; **nobody has reviewed that
//! reading**, and the UK/GDPR rectification route is not modelled.
//!
//! ```text
//! Requested ─ Accept ──► Accepted ─ Apply(version) ──► Applied
//!     │                                                    
//!     ├─ Deny(ground, reason) ──► Denied ─ Disagree(statement) ─► Denied (statement kept)
//!     └─ Withdraw (the requester) ──► Withdrawn
//! ```
//!
//! Rules: the person who decides is never the requester; a denial names one of a closed list of
//! grounds and gives a reason; every decision needs a reason that is not blank; an application must
//! point at a version later than the one disputed; a statement of disagreement is kept once and
//! cannot be edited; nothing leaves a final state.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// Days allowed to act on a request. **A guess from HIPAA's 60; configurable later.**
pub const ACT_WITHIN_DAYS: i64 = 60;
/// The one extension, in days, allowed while undecided, with a written reason.
pub const EXTENSION_DAYS: i64 = 30;
/// The longest statement of disagreement kept, in characters.
pub const STATEMENT_MAX: usize = 4000;

/// The record a request is about: one kind, one id, and the version the patient read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Target {
    /// The kind of record (`Observation`, `AllergyIntolerance`, ...).
    pub kind: String,
    /// Its id.
    pub id: String,
    /// The version the request was made against.
    pub version: i32,
}

/// Why a request was refused. A closed list: a free-text ground cannot be counted or audited.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Ground {
    /// The record was not made by this organisation.
    NotOurs,
    /// The record is accurate and complete as it stands.
    AccurateAndComplete,
    /// The record is not one the requester may inspect or have amended.
    NotAvailableToRequester,
    /// The record was not created by anyone who can be asked.
    Other,
}

/// Where a request stands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub enum State {
    /// Waiting for a decision.
    Requested,
    /// Agreed; the correction is not yet recorded.
    Accepted {
        /// Who agreed.
        by: String,
        /// Why.
        reason: String,
    },
    /// The correction is recorded as this later version of the record. Final.
    Applied {
        /// The version that carries the correction.
        version: i32,
    },
    /// Refused. The patient may add one statement of disagreement.
    Denied {
        /// Who refused.
        by: String,
        /// On which ground.
        ground: Ground,
        /// Why, in words.
        reason: String,
        /// The patient's statement of disagreement, once given.
        disagreement: Option<String>,
    },
    /// The requester withdrew it. Final.
    Withdrawn,
}

/// One request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    /// What it is about.
    pub target: Target,
    /// Who asked (a token subject).
    pub requester: String,
    /// What they want changed, in their words.
    pub text: String,
    /// When it arrived.
    pub received_at: DateTime<Utc>,
    /// The written reason for the one extension, once taken.
    pub extension: Option<String>,
    /// Where it stands.
    pub state: State,
}

/// What can happen to a request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "kebab-case")]
pub enum Event {
    /// Agree to the change.
    Accept {
        /// Who.
        by: String,
        /// Why.
        reason: String,
    },
    /// Refuse the change.
    Deny {
        /// Who.
        by: String,
        /// The ground.
        ground: Ground,
        /// Why.
        reason: String,
    },
    /// The correction has been recorded as this version of the record.
    Apply {
        /// The version.
        version: i32,
    },
    /// The requester withdraws.
    Withdraw {
        /// Who says so (must be the requester).
        by: String,
    },
    /// The requester states that they disagree with a refusal.
    Disagree {
        /// Who says so (must be the requester).
        by: String,
        /// The statement.
        statement: String,
    },
    /// Take the one extension.
    Extend {
        /// The written reason.
        reason: String,
    },
}

/// Why an event was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// The request is in a state that does not allow this.
    WrongState,
    /// A reason, statement or request text is blank.
    Blank(&'static str),
    /// The decider is the requester.
    SelfDecision,
    /// Only the requester may do this.
    NotTheRequester,
    /// The version applied is not later than the one disputed.
    NotALaterVersion,
    /// The statement is longer than [`STATEMENT_MAX`].
    StatementTooLong,
    /// A statement of disagreement is already kept.
    AlreadyDisagreed,
    /// The extension was already taken.
    AlreadyExtended,
}

fn blank(s: &str) -> bool {
    s.trim().is_empty()
}

impl State {
    /// The stored word for the state.
    #[must_use]
    pub fn word(&self) -> &'static str {
        match self {
            State::Requested => "requested",
            State::Accepted { .. } => "accepted",
            State::Applied { .. } => "applied",
            State::Denied { .. } => "denied",
            State::Withdrawn => "withdrawn",
        }
    }
}

impl Request {
    /// A new request.
    ///
    /// # Errors
    ///
    /// [`Refusal::Blank`] when the text is blank.
    pub fn new(
        target: Target,
        requester: &str,
        text: &str,
        received_at: DateTime<Utc>,
    ) -> Result<Self, Refusal> {
        if blank(text) {
            return Err(Refusal::Blank("text"));
        }
        Ok(Self {
            target,
            requester: requester.to_string(),
            text: text.trim().to_string(),
            received_at,
            extension: None,
            state: State::Requested,
        })
    }

    /// The instant by which a decision is due: 60 days, 90 after the extension.
    #[must_use]
    pub fn due(&self) -> DateTime<Utc> {
        let days = ACT_WITHIN_DAYS
            + if self.extension.is_some() {
                EXTENSION_DAYS
            } else {
                0
            };
        self.received_at + Duration::days(days)
    }

    /// Whether it is undecided past its due time.
    #[must_use]
    pub fn overdue(&self, now: DateTime<Utc>) -> bool {
        matches!(self.state, State::Requested) && now > self.due()
    }

    /// The request after `event`, or why not. The request is not changed.
    ///
    /// # Errors
    ///
    /// A [`Refusal`] naming the rule broken.
    pub fn apply(&self, event: Event) -> Result<Request, Refusal> {
        let mut next = self.clone();
        match (&self.state, event) {
            (State::Requested, Event::Accept { by, reason }) => {
                self.check_decider(&by, &reason)?;
                next.state = State::Accepted { by, reason };
            }
            (State::Requested, Event::Deny { by, ground, reason }) => {
                self.check_decider(&by, &reason)?;
                next.state = State::Denied {
                    by,
                    ground,
                    reason,
                    disagreement: None,
                };
            }
            (State::Requested, Event::Withdraw { by }) => {
                self.check_requester(&by)?;
                next.state = State::Withdrawn;
            }
            (State::Requested, Event::Extend { reason }) => {
                if self.extension.is_some() {
                    return Err(Refusal::AlreadyExtended);
                }
                if blank(&reason) {
                    return Err(Refusal::Blank("reason"));
                }
                next.extension = Some(reason.trim().to_string());
            }
            (State::Accepted { .. }, Event::Apply { version }) => {
                if version <= self.target.version {
                    return Err(Refusal::NotALaterVersion);
                }
                next.state = State::Applied { version };
            }
            (
                State::Denied {
                    disagreement,
                    by,
                    ground,
                    reason,
                },
                Event::Disagree { by: who, statement },
            ) => {
                self.check_requester(&who)?;
                if disagreement.is_some() {
                    return Err(Refusal::AlreadyDisagreed);
                }
                if blank(&statement) {
                    return Err(Refusal::Blank("statement"));
                }
                if statement.chars().count() > STATEMENT_MAX {
                    return Err(Refusal::StatementTooLong);
                }
                next.state = State::Denied {
                    by: by.clone(),
                    ground: *ground,
                    reason: reason.clone(),
                    disagreement: Some(statement.trim().to_string()),
                };
            }
            _ => return Err(Refusal::WrongState),
        }
        Ok(next)
    }

    fn check_decider(&self, by: &str, reason: &str) -> Result<(), Refusal> {
        if by == self.requester {
            return Err(Refusal::SelfDecision);
        }
        if blank(reason) {
            return Err(Refusal::Blank("reason"));
        }
        Ok(())
    }

    fn check_requester(&self, by: &str) -> Result<(), Refusal> {
        if by == self.requester {
            Ok(())
        } else {
            Err(Refusal::NotTheRequester)
        }
    }

    /// What a reader of the disputed record must be told, if anything. A request nobody has decided
    /// is told as undecided; a withdrawn one is not told at all.
    #[must_use]
    pub fn notice(&self) -> Option<Notice> {
        match &self.state {
            State::Requested | State::Accepted { .. } => Some(Notice::Pending),
            State::Applied { version } => Some(Notice::Amended { version: *version }),
            State::Denied {
                ground,
                disagreement,
                ..
            } => Some(Notice::Refused {
                ground: *ground,
                disagreement: disagreement.clone(),
            }),
            State::Withdrawn => None,
        }
    }
}

/// What a reader is shown beside the disputed record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "notice", rename_all = "kebab-case")]
pub enum Notice {
    /// An amendment has been asked for and the record is not yet corrected.
    Pending,
    /// The record was corrected in this later version.
    Amended {
        /// The version.
        version: i32,
    },
    /// The request was refused; the patient's statement goes with the record.
    Refused {
        /// The ground.
        ground: Ground,
        /// The patient's statement, if they gave one.
        disagreement: Option<String>,
    },
}

impl Notice {
    /// The notice in plain English, as a reader of the record is shown it (the UI says the same).
    /// The patient's statement is given as written.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Notice::Pending => "The patient has asked for this record to be corrected. No decision has been recorded.".into(),
            Notice::Amended { version } => format!("The patient asked for this record to be corrected. It was corrected in version {version}."),
            Notice::Refused { ground, disagreement } => {
                let ground = match ground {
                    Ground::NotOurs => "this organisation did not make the record.",
                    Ground::AccurateAndComplete => "the record is accurate and complete as it stands.",
                    Ground::NotAvailableToRequester => "the record is not one the requester may have amended.",
                    Ground::Other => "another ground.",
                };
                let said = match disagreement.as_deref().map(str::trim) {
                    Some(s) if !s.is_empty() => format!("The patient's statement of disagreement: {s}"),
                    _ => "The patient has given no statement.".into(),
                };
                format!("The patient asked for this record to be corrected. The request was refused: {ground} {said}")
            }
        }
    }

    /// The notice as a FHIR `Annotation`: the English text a person reads, and a coded extension a
    /// machine can act on (state, the version that corrected it, the ground of a refusal). The
    /// extension URL is a placeholder under `.example` until the project owns a canonical base.
    #[must_use]
    pub fn fhir_annotation(&self) -> serde_json::Value {
        let (state, mut parts) = match self {
            Notice::Pending => ("pending", vec![]),
            Notice::Amended { version } => (
                "amended",
                vec![serde_json::json!({ "url": "version", "valueInteger": version })],
            ),
            Notice::Refused { ground, .. } => (
                "refused",
                vec![serde_json::json!({
                    "url": "ground",
                    "valueCode": serde_json::to_value(ground).ok().and_then(|g| g.as_str().map(str::to_owned)).unwrap_or_default()
                })],
            ),
        };
        parts.insert(0, serde_json::json!({ "url": "state", "valueCode": state }));
        serde_json::json!({
            "extension": [{ "url": AMENDMENT_NOTICE_URL, "extension": parts }],
            "text": self.describe(),
        })
    }
}

/// The extension that codes an amendment notice. A placeholder base: not yet published or registered.
pub const AMENDMENT_NOTICE_URL: &str =
    "https://electronic-patient-record.example/fhir/StructureDefinition/amendment-notice";

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn t(day: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, day, 9, 0, 0).unwrap()
    }

    fn open() -> Request {
        Request::new(
            Target {
                kind: "Observation".into(),
                id: "o1".into(),
                version: 2,
            },
            "patient-1",
            "  The heart rate is wrong  ",
            t(1),
        )
        .unwrap()
    }

    fn accept(by: &str, reason: &str) -> Event {
        Event::Accept {
            by: by.into(),
            reason: reason.into(),
        }
    }

    fn deny() -> Event {
        Event::Deny {
            by: "dr-1".into(),
            ground: Ground::AccurateAndComplete,
            reason: "checked the monitor print-out".into(),
        }
    }

    #[test]
    fn a_blank_request_is_refused_and_the_text_is_trimmed() {
        let target = open().target;
        assert_eq!(
            Request::new(target, "p", "  ", t(1)),
            Err(Refusal::Blank("text"))
        );
        assert_eq!(open().text, "The heart rate is wrong");
    }

    #[test]
    fn accepted_then_applied_ends_final_and_the_reader_is_told_the_version() {
        let r = open().apply(accept("dr-1", "agreed")).unwrap();
        assert_eq!(r.notice(), Some(Notice::Pending));
        let r = r.apply(Event::Apply { version: 3 }).unwrap();
        assert_eq!(r.notice(), Some(Notice::Amended { version: 3 }));
        assert_eq!(
            r.apply(Event::Apply { version: 4 }),
            Err(Refusal::WrongState)
        );
        assert_eq!(r.apply(deny()), Err(Refusal::WrongState));
    }

    #[test]
    fn nobody_decides_their_own_request_and_a_decision_needs_a_reason() {
        let r = open();
        assert_eq!(
            r.apply(accept("patient-1", "yes")),
            Err(Refusal::SelfDecision)
        );
        assert_eq!(r.apply(accept("dr-1", "  ")), Err(Refusal::Blank("reason")));
        assert_eq!(
            r.apply(Event::Deny {
                by: "dr-1".into(),
                ground: Ground::Other,
                reason: String::new()
            }),
            Err(Refusal::Blank("reason"))
        );
    }

    #[test]
    fn the_correction_must_be_a_later_version_than_the_one_disputed() {
        let r = open().apply(accept("dr-1", "agreed")).unwrap();
        assert_eq!(
            r.apply(Event::Apply { version: 2 }),
            Err(Refusal::NotALaterVersion)
        );
        assert_eq!(
            r.apply(Event::Apply { version: 1 }),
            Err(Refusal::NotALaterVersion)
        );
    }

    #[test]
    fn a_refusal_keeps_one_unedited_statement_of_disagreement_that_readers_see() {
        let r = open().apply(deny()).unwrap();
        assert_eq!(
            r.notice(),
            Some(Notice::Refused {
                ground: Ground::AccurateAndComplete,
                disagreement: None
            })
        );
        let say = |who: &str, s: &str| Event::Disagree {
            by: who.into(),
            statement: s.into(),
        };
        assert_eq!(r.apply(say("dr-1", "x")), Err(Refusal::NotTheRequester));
        assert_eq!(
            r.apply(say("patient-1", " ")),
            Err(Refusal::Blank("statement"))
        );
        assert_eq!(
            r.apply(say("patient-1", &"x".repeat(STATEMENT_MAX + 1))),
            Err(Refusal::StatementTooLong)
        );
        let r = r.apply(say("patient-1", " I was there. ")).unwrap();
        assert_eq!(
            r.notice(),
            Some(Notice::Refused {
                ground: Ground::AccurateAndComplete,
                disagreement: Some("I was there.".into())
            })
        );
        assert_eq!(
            r.apply(say("patient-1", "again")),
            Err(Refusal::AlreadyDisagreed)
        );
    }

    #[test]
    fn only_the_requester_withdraws_and_only_before_a_decision_and_a_withdrawal_is_not_shown() {
        let r = open();
        assert_eq!(
            r.apply(Event::Withdraw { by: "dr-1".into() }),
            Err(Refusal::NotTheRequester)
        );
        let w = r
            .apply(Event::Withdraw {
                by: "patient-1".into(),
            })
            .unwrap();
        assert_eq!(w.notice(), None);
        assert_eq!(w.apply(accept("dr-1", "x")), Err(Refusal::WrongState));
        let decided = r.apply(deny()).unwrap();
        assert_eq!(
            decided.apply(Event::Withdraw {
                by: "patient-1".into()
            }),
            Err(Refusal::WrongState)
        );
    }

    #[test]
    fn the_due_time_is_60_days_and_90_after_the_one_extension_which_needs_a_reason() {
        let r = open();
        assert_eq!(r.due(), t(1) + Duration::days(60));
        assert!(!r.overdue(t(1) + Duration::days(60)));
        assert!(r.overdue(t(1) + Duration::days(61)));
        assert_eq!(
            r.apply(Event::Extend { reason: " ".into() }),
            Err(Refusal::Blank("reason"))
        );
        let e = r
            .apply(Event::Extend {
                reason: "records off site".into(),
            })
            .unwrap();
        assert_eq!(e.due(), t(1) + Duration::days(90));
        assert!(!e.overdue(t(1) + Duration::days(61)));
        assert_eq!(
            e.apply(Event::Extend {
                reason: "again".into()
            }),
            Err(Refusal::AlreadyExtended)
        );
        // A decided request is never overdue.
        assert!(!r.apply(deny()).unwrap().overdue(t(1) + Duration::days(400)));
    }

    #[test]
    fn applying_does_not_change_the_request_it_was_called_on() {
        let r = open();
        let _ = r.apply(deny()).unwrap();
        assert_eq!(r.state, State::Requested);
    }

    #[test]
    fn a_notice_in_words_never_drops_the_ground_or_the_statement() {
        assert!(
            Notice::Pending
                .describe()
                .contains("No decision has been recorded.")
        );
        assert!(
            Notice::Amended { version: 3 }
                .describe()
                .contains("version 3")
        );
        let refused = Notice::Refused {
            ground: Ground::AccurateAndComplete,
            disagreement: Some("  I was not feverish.  ".into()),
        }
        .describe();
        assert!(refused.contains("accurate and complete"));
        assert!(refused.ends_with("The patient's statement of disagreement: I was not feverish."));
        let none = Notice::Refused {
            ground: Ground::Other,
            disagreement: Some("  ".into()),
        }
        .describe();
        assert!(none.ends_with("The patient has given no statement."));
    }

    #[test]
    fn a_notice_is_coded_for_a_machine_as_well_as_told_in_words() {
        let code = |n: &Notice| n.fhir_annotation()["extension"][0]["extension"].clone();
        assert_eq!(
            code(&Notice::Pending),
            serde_json::json!([{ "url": "state", "valueCode": "pending" }])
        );
        assert_eq!(
            code(&Notice::Amended { version: 4 }),
            serde_json::json!([{ "url": "state", "valueCode": "amended" }, { "url": "version", "valueInteger": 4 }])
        );
        let refused = Notice::Refused {
            ground: Ground::AccurateAndComplete,
            disagreement: Some("no".into()),
        };
        assert_eq!(
            code(&refused),
            serde_json::json!([{ "url": "state", "valueCode": "refused" }, { "url": "ground", "valueCode": "accurate-and-complete" }])
        );
        assert_eq!(refused.fhir_annotation()["text"], refused.describe());
    }
}
