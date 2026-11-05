//! A report that a person has died (BV-1, first slice): the rules for one assertion and the single
//! answer a set of assertions gives, pure. Nothing stores, serves or shows an assertion yet.
//!
//! A death is an assertion with evidence, not a true-or-false field, and it may be wrong. The spec is
//! `spec/bereavement/death-assertion.md`.
//!
//! ```text
//! Reported ─► Verified ─► Certified ─► Registered      (forward only, and steps may be skipped)
//!    └───────────┴────────────┴────────────┴─► WithdrawnInError (final)
//! ```
//!
//! Rules: any report holds outbound contact at once, whoever or whatever made it (OQ-BV1); only a
//! person moves an assertion on, so a feed can report but never confirm or withdraw; every step needs
//! an author and a reason that are not blank; a date of death cannot be in the future; a withdrawn
//! assertion stays in the history and stops counting.
//!
//! Which roles may take each step is not decided here (OQ-BV2).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Where a report came from. A closed list, so that sources can be counted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Source {
    /// The registrar of deaths.
    Registrar,
    /// A national demographics service.
    NationalDemographics,
    /// A hospital.
    Hospital,
    /// A general practice.
    GeneralPractice,
    /// A care home.
    CareHome,
    /// A coroner.
    Coroner,
    /// A family member.
    FamilyMember,
    /// Another system, such as an HL7 feed.
    OtherSystem,
}

/// How far an assertion has got. The order of the variants is the order of the steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    /// Someone or something said so. No person here has confirmed it.
    Reported,
    /// A person established that the patient has died.
    Verified,
    /// The cause has been certified. The cause itself is a separate, later fact.
    Certified,
    /// The death has been registered.
    Registered,
    /// A person found the assertion wrong. Final.
    WithdrawnInError,
}

/// When the person died, or why that is not known. Absent is a reason, never a guess.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Occurred {
    /// `YYYY`, `YYYY-MM`, `YYYY-MM-DD`, or an RFC 3339 instant.
    Known {
        /// The date, as precise as the source was.
        at: String,
    },
    /// Not known.
    NotKnown {
        /// Why not.
        reason: String,
    },
}

/// Who acted. A feed can report, and only a person can do anything else.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "kebab-case")]
pub enum Actor {
    /// A signed-in person.
    Person(String),
    /// An automatic source, such as `mllp:adt-feed`.
    Feed(String),
}

/// A step, always by a named actor with a reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// Where the assertion moves to. [`Status::Reported`] is not a step.
    pub to: Status,
    /// Who took it.
    pub by: Actor,
    /// Why.
    pub reason: String,
}

/// One assertion at one version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Assertion {
    /// Where the report came from.
    pub source: Source,
    /// Who or what made the report.
    pub reported_by: Actor,
    /// When the person died, or why that is not known.
    pub occurred: Occurred,
    /// How far it has got.
    pub status: Status,
    /// Why this version was written.
    pub reason: String,
}

/// Why a report or a step was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Refusal {
    /// The author is blank.
    AuthorRequired,
    /// The reason is blank.
    ReasonRequired,
    /// The date is not one of the accepted forms.
    DateInvalid,
    /// The date is in the future.
    DateInFuture,
    /// A feed may report but not confirm or withdraw.
    PersonRequired,
    /// The step does not move the assertion forward, or the assertion is withdrawn.
    NotAllowed,
}

fn named(actor: &Actor) -> bool {
    match actor {
        Actor::Person(id) | Actor::Feed(id) => !id.trim().is_empty(),
    }
}

/// The date's earliest moment is not after `now`: a partial date is compared at its start, so a
/// death earlier this year or this month is accepted.
fn check_date(at: &str, now: DateTime<Utc>) -> Result<(), Refusal> {
    let start = if crate::patients_store::valid_birth_date(at) {
        let mut parts = at.splitn(3, '-');
        let year: i32 = parts
            .next()
            .and_then(|y| y.parse().ok())
            .ok_or(Refusal::DateInvalid)?;
        let month: u32 = parts
            .next()
            .map_or(Some(1), |m| m.parse().ok())
            .ok_or(Refusal::DateInvalid)?;
        let day: u32 = parts
            .next()
            .map_or(Some(1), |d| d.parse().ok())
            .ok_or(Refusal::DateInvalid)?;
        chrono::NaiveDate::from_ymd_opt(year, month, day)
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .ok_or(Refusal::DateInvalid)?
            .and_utc()
    } else {
        DateTime::parse_from_rfc3339(at)
            .map_err(|_| Refusal::DateInvalid)?
            .to_utc()
    };
    if start > now {
        Err(Refusal::DateInFuture)
    } else {
        Ok(())
    }
}

/// A new assertion. It starts as [`Status::Reported`], whoever made it, and from that moment holds
/// outbound contact.
///
/// # Errors
///
/// The [`Refusal`]: a blank author or reason, or a bad date.
pub fn report(
    source: Source,
    reported_by: Actor,
    occurred: Occurred,
    reason: &str,
    now: DateTime<Utc>,
) -> Result<Assertion, Refusal> {
    if !named(&reported_by) {
        return Err(Refusal::AuthorRequired);
    }
    if reason.trim().is_empty() {
        return Err(Refusal::ReasonRequired);
    }
    match &occurred {
        Occurred::Known { at } => check_date(at, now)?,
        Occurred::NotKnown { reason } if reason.trim().is_empty() => {
            return Err(Refusal::ReasonRequired);
        }
        Occurred::NotKnown { .. } => {}
    }
    Ok(Assertion {
        source,
        reported_by,
        occurred,
        status: Status::Reported,
        reason: reason.to_string(),
    })
}

/// Whether `from` may move to `to`: forward only, never back to a report, nothing after a withdrawal.
#[must_use]
pub fn allowed(from: Status, to: Status) -> bool {
    from != Status::WithdrawnInError && to != Status::Reported && from < to
}

/// The next version of an assertion.
///
/// # Errors
///
/// The [`Refusal`]: a blank author or reason, a feed, or a step that is not allowed.
pub fn advance(assertion: &Assertion, step: &Step) -> Result<Assertion, Refusal> {
    if !named(&step.by) {
        return Err(Refusal::AuthorRequired);
    }
    if step.reason.trim().is_empty() {
        return Err(Refusal::ReasonRequired);
    }
    if !matches!(step.by, Actor::Person(_)) {
        return Err(Refusal::PersonRequired);
    }
    if !allowed(assertion.status, step.to) {
        return Err(Refusal::NotAllowed);
    }
    Ok(Assertion {
        status: step.to,
        reason: step.reason.clone(),
        ..assertion.clone()
    })
}

/// The one answer a screen or a sender is given.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "answer", rename_all = "kebab-case")]
pub enum Answer {
    /// No assertion stands. This never means the person is known to be living.
    NotKnownToHaveDied,
    /// At least one assertion stands.
    Asserted {
        /// The furthest status of those that stand.
        status: Status,
        /// False until a person has confirmed one of them.
        confirmed: bool,
        /// The date from the furthest assertion that has one, as given.
        occurred: Option<String>,
        /// Those that stand give different dates, so a person should look.
        dates_differ: bool,
    },
}

impl Answer {
    /// Whether outbound contact is held. Any assertion that stands holds it (OQ-BV1).
    #[must_use]
    pub fn holds_contact(&self) -> bool {
        matches!(self, Answer::Asserted { .. })
    }
}

/// The answer for all of a patient's assertions, each at its current version.
#[must_use]
pub fn answer(assertions: &[Assertion]) -> Answer {
    let mut standing: Vec<&Assertion> = assertions
        .iter()
        .filter(|a| a.status != Status::WithdrawnInError)
        .collect();
    standing.sort_by_key(|a| std::cmp::Reverse(a.status));
    let Some(first) = standing.first() else {
        return Answer::NotKnownToHaveDied;
    };
    let mut dates = standing.iter().filter_map(|a| match &a.occurred {
        Occurred::Known { at } => Some(at.clone()),
        Occurred::NotKnown { .. } => None,
    });
    let occurred = dates.next();
    let dates_differ = dates.any(|d| Some(&d) != occurred.as_ref());
    Answer::Asserted {
        status: first.status,
        confirmed: first.status != Status::Reported,
        occurred,
        dates_differ,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Status::{Certified, Registered, Reported, Verified, WithdrawnInError as Withdrawn};
    use chrono::TimeZone;

    const ALL: [Status; 5] = [Reported, Verified, Certified, Registered, Withdrawn];

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 5, 12, 0, 0).unwrap()
    }

    fn nurse() -> Actor {
        Actor::Person("nurse-1".into())
    }

    fn feed() -> Actor {
        Actor::Feed("mllp:adt-feed".into())
    }

    fn known(at: &str) -> Occurred {
        Occurred::Known { at: at.into() }
    }

    fn at(status: Status, occurred: Occurred) -> Assertion {
        Assertion {
            source: Source::Hospital,
            reported_by: nurse(),
            occurred,
            status,
            reason: "r".into(),
        }
    }

    /// The whole state table, written out: every pair, so a new status has to be placed.
    #[test]
    fn the_state_table() {
        let table = [
            (Reported, [false, true, true, true, true]),
            (Verified, [false, false, true, true, true]),
            (Certified, [false, false, false, true, true]),
            (Registered, [false, false, false, false, true]),
            (Withdrawn, [false, false, false, false, false]),
        ];
        for (from, row) in table {
            for (to, expected) in ALL.into_iter().zip(row) {
                assert_eq!(allowed(from, to), expected, "{from:?} to {to:?}");
                let step = Step {
                    to,
                    by: nurse(),
                    reason: "r".into(),
                };
                let result = advance(&at(from, known("2026-11-01")), &step);
                assert_eq!(result.is_ok(), expected, "{from:?} to {to:?}");
            }
        }
    }

    #[test]
    fn any_report_holds_contact_at_once_even_from_a_feed() {
        let made = report(
            Source::OtherSystem,
            feed(),
            known("2026-11-01"),
            "ADT",
            now(),
        )
        .unwrap();
        assert_eq!(made.status, Reported);
        let told = answer(&[made]);
        assert!(told.holds_contact());
        assert!(matches!(
            told,
            Answer::Asserted {
                confirmed: false,
                ..
            }
        ));
    }

    #[test]
    fn a_feed_can_report_but_never_confirm_or_withdraw() {
        for to in [Verified, Certified, Registered, Withdrawn] {
            let step = Step {
                to,
                by: feed(),
                reason: "r".into(),
            };
            assert_eq!(
                advance(&at(Reported, known("2026")), &step),
                Err(Refusal::PersonRequired),
            );
        }
    }

    #[test]
    fn a_person_confirming_a_report_keeps_the_hold() {
        let step = Step {
            to: Verified,
            by: nurse(),
            reason: "seen".into(),
        };
        let done = advance(&at(Reported, known("2026-11-01")), &step).unwrap();
        let told = answer(&[done]);
        assert!(told.holds_contact());
        assert!(matches!(
            told,
            Answer::Asserted {
                confirmed: true,
                ..
            }
        ));
    }

    #[test]
    fn a_withdrawal_releases_the_hold_only_when_nothing_else_stands() {
        let wrong = at(Withdrawn, known("2026-11-01"));
        assert_eq!(
            answer(std::slice::from_ref(&wrong)),
            Answer::NotKnownToHaveDied
        );
        assert!(!answer(std::slice::from_ref(&wrong)).holds_contact());
        let other = at(Reported, known("2026-11-02"));
        assert!(answer(&[wrong, other]).holds_contact());
    }

    #[test]
    fn the_furthest_assertion_answers_and_differing_dates_are_said() {
        let told = answer(&[
            at(Reported, known("2026-11-02")),
            at(Registered, known("2026-11-01")),
        ]);
        assert_eq!(
            told,
            Answer::Asserted {
                status: Registered,
                confirmed: true,
                occurred: Some("2026-11-01".into()),
                dates_differ: true,
            }
        );
    }

    #[test]
    fn no_assertion_is_not_a_statement_that_the_person_is_living() {
        assert!(!answer(&[]).holds_contact());
        let text = serde_json::to_string(&answer(&[])).unwrap();
        assert_eq!(text, r#"{"answer":"not-known-to-have-died"}"#);
    }

    #[test]
    fn a_date_may_be_partial_or_an_instant_but_not_later_than_now() {
        let try_at = |d: &str| report(Source::Hospital, nurse(), known(d), "r", now()).map(|_| ());
        for good in ["2026", "2026-11", "2026-11-05", "2026-11-05T11:00:00Z"] {
            assert_eq!(try_at(good), Ok(()), "{good}");
        }
        assert_eq!(try_at("2026-11-06"), Err(Refusal::DateInFuture));
        assert_eq!(try_at("2026-12"), Err(Refusal::DateInFuture));
        assert_eq!(try_at("2027"), Err(Refusal::DateInFuture));
        assert_eq!(try_at("2026-11-05T13:00:00Z"), Err(Refusal::DateInFuture));
        assert_eq!(try_at("last week"), Err(Refusal::DateInvalid));
    }

    #[test]
    fn a_blank_author_or_reason_is_refused() {
        let date = known("2026");
        let blank = Actor::Person(" ".into());
        assert_eq!(
            report(Source::Hospital, blank.clone(), date.clone(), "r", now()),
            Err(Refusal::AuthorRequired)
        );
        assert_eq!(
            report(Source::Hospital, nurse(), date.clone(), " ", now()),
            Err(Refusal::ReasonRequired)
        );
        let empty = Occurred::NotKnown { reason: " ".into() };
        assert_eq!(
            report(Source::Hospital, nurse(), empty, "r", now()),
            Err(Refusal::ReasonRequired)
        );
        let a = at(Reported, date);
        let step = Step {
            to: Verified,
            by: blank,
            reason: "r".into(),
        };
        assert_eq!(advance(&a, &step), Err(Refusal::AuthorRequired));
        let step = Step {
            to: Verified,
            by: nurse(),
            reason: String::new(),
        };
        assert_eq!(advance(&a, &step), Err(Refusal::ReasonRequired));
    }
}
