//! The referral state table (task PC-1, a draft from `spec/interoperability/closed-loop-referral.md`).
//! It is pure: the caller passes the current [`State`] and an [`Event`] and gets the next state or a
//! [`Refused`]. Nothing in the service uses it yet; CA-3 (referrals with a closed loop) and PA-14 (outcomes
//! beyond accept and reject) are the intended callers.
//!
//! The states and transitions come from one outside, unreviewed source (a proof-of-concept guide built on
//! IHE 360X, which I have not read), so the table is a **proposal**. Three states (`information-requested`,
//! `deferred`, `redirected`) are marked provisional because that source marks them "to confirm".
//!
//! The rules the table keeps:
//!
//! - One stored status, and a transition not in the table is refused. Status is never inferred.
//! - A **decline** can arrive after acceptance, and an **outcome** can end a referral at any live point
//!   after acceptance.
//! - A **cancellation is a request**. The referrer asks ([`Event::RequestCancel`]); only the recipient's
//!   confirmation ([`Event::ConfirmCancel`]) reaches [`State::Cancelled`]. The recipient may instead answer
//!   with an outcome, which completes the referral.
//! - A **redirect** ends the referral; the replacement is a new referral with a new id, made elsewhere.
//! - The loop is closed only by a decline or an outcome ([`State::closes_loop`]).
//!
//! Not covered: who may send an event beyond naming the [`Actor`] each expects (authorisation is the policy
//! engine's), timeouts (PC-3), the referral id and unmatched outcomes (PC-2), and the mapping to FHIR `Task`.

use std::fmt;

/// Where a referral is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum State {
    /// The referrer has sent the request.
    Requested,
    /// The recipient has accepted.
    Accepted,
    /// The recipient has declined, at any time, including after acceptance. An end state.
    Declined,
    /// An appointment is booked.
    AppointmentBooked,
    /// The patient missed the appointment.
    NoShow,
    /// The recipient has sent a note while care continues; it can be repeated.
    InterimReport,
    /// The recipient has sent the outcome. An end state.
    Completed,
    /// The referrer has asked to cancel; the recipient has not yet confirmed.
    CancelRequested,
    /// The recipient confirmed the cancellation. An end state.
    Cancelled,
    /// Provisional: the recipient needs more information before deciding.
    InformationRequested,
    /// Provisional: the recipient has put the referral on hold.
    Deferred,
    /// Provisional: the recipient sent it elsewhere. An end state; a new referral replaces it.
    Redirected,
}

/// Something that happens to a referral.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Event {
    /// The recipient accepts.
    Accept,
    /// The recipient declines.
    Decline,
    /// The recipient books an appointment, or rebooks after a no-show.
    Book,
    /// The recipient records a missed appointment.
    RecordNoShow,
    /// The recipient sends a note while care continues.
    InterimNote,
    /// The recipient sends the outcome.
    Outcome,
    /// The referrer asks to cancel.
    RequestCancel,
    /// The recipient confirms the cancellation.
    ConfirmCancel,
    /// The recipient asks for more information.
    RequestInformation,
    /// The referrer supplies the information asked for.
    ProvideInformation,
    /// The recipient puts the referral on hold.
    Defer,
    /// The recipient takes the referral off hold.
    Resume,
    /// The recipient sends the referral elsewhere.
    Redirect,
}

/// Who sends an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Actor {
    /// The clinician or system that made the request.
    Referrer,
    /// The service the request went to.
    Recipient,
}

/// An event the table does not allow in this state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Refused {
    /// The state the referral was in.
    pub state: State,
    /// The event that was refused.
    pub event: Event,
}

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "a referral that is {} cannot take {:?}",
            self.state.as_str(),
            self.event
        )
    }
}

impl std::error::Error for Refused {}

impl State {
    /// The kebab-case word stored and shown for this state.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            State::Requested => "requested",
            State::Accepted => "accepted",
            State::Declined => "declined",
            State::AppointmentBooked => "appointment-booked",
            State::NoShow => "no-show",
            State::InterimReport => "interim-report",
            State::Completed => "completed",
            State::CancelRequested => "cancel-requested",
            State::Cancelled => "cancelled",
            State::InformationRequested => "information-requested",
            State::Deferred => "deferred",
            State::Redirected => "redirected",
        }
    }

    /// Every state.
    pub const ALL: [State; 12] = [
        State::Requested,
        State::Accepted,
        State::Declined,
        State::AppointmentBooked,
        State::NoShow,
        State::InterimReport,
        State::Completed,
        State::CancelRequested,
        State::Cancelled,
        State::InformationRequested,
        State::Deferred,
        State::Redirected,
    ];

    /// Whether the referral can go no further.
    #[must_use]
    pub fn is_end(self) -> bool {
        matches!(
            self,
            State::Declined | State::Completed | State::Cancelled | State::Redirected
        )
    }

    /// Whether the referrer has now been told what happened: a decline or an outcome. A confirmed
    /// cancellation and a redirect end the referral without closing the loop on the care itself.
    #[must_use]
    pub fn closes_loop(self) -> bool {
        matches!(self, State::Declined | State::Completed)
    }

    /// Whether the source marks this state "to confirm".
    #[must_use]
    pub fn is_provisional(self) -> bool {
        matches!(
            self,
            State::InformationRequested | State::Deferred | State::Redirected
        )
    }
}

impl Event {
    /// Every event.
    pub const ALL: [Event; 13] = [
        Event::Accept,
        Event::Decline,
        Event::Book,
        Event::RecordNoShow,
        Event::InterimNote,
        Event::Outcome,
        Event::RequestCancel,
        Event::ConfirmCancel,
        Event::RequestInformation,
        Event::ProvideInformation,
        Event::Defer,
        Event::Resume,
        Event::Redirect,
    ];

    /// Who sends this event.
    #[must_use]
    pub fn actor(self) -> Actor {
        match self {
            Event::RequestCancel | Event::ProvideInformation => Actor::Referrer,
            _ => Actor::Recipient,
        }
    }
}

/// The state after `event`, or [`Refused`] if the table does not allow it in `state`.
///
/// # Errors
///
/// [`Refused`] for any pair not in the table, including every event on an end state.
pub fn next(state: State, event: Event) -> Result<State, Refused> {
    use Event as E;
    use State as S;
    let to = match (state, event) {
        (S::Requested, E::Accept) => S::Accepted,
        (S::Requested, E::Decline) => S::Declined,
        (S::Requested, E::RequestInformation) => S::InformationRequested,
        (S::Requested, E::Redirect) => S::Redirected,
        (S::Requested | S::Accepted | S::Deferred, E::RequestCancel) => S::CancelRequested,
        (S::Accepted, E::Book) => S::AppointmentBooked,
        (S::Accepted | S::AppointmentBooked, E::InterimNote) => S::InterimReport,
        (
            S::Accepted | S::AppointmentBooked | S::InterimReport | S::CancelRequested,
            E::Outcome,
        ) => S::Completed,
        (S::Accepted | S::NoShow, E::Decline) => S::Declined,
        (S::Accepted, E::Defer) => S::Deferred,
        (S::AppointmentBooked, E::RecordNoShow) => S::NoShow,
        (S::NoShow, E::Book) => S::AppointmentBooked,
        (S::InterimReport, E::InterimNote) => S::InterimReport,
        (S::CancelRequested, E::ConfirmCancel) => S::Cancelled,
        (S::InformationRequested, E::ProvideInformation) => S::Requested,
        (S::Deferred, E::Resume) => S::Accepted,
        _ => return Err(Refused { state, event }),
    };
    Ok(to)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeSet, HashSet};

    /// The table as the spec lists it, written out again so a change to `next` must change this too.
    const LISTED: &[(State, Event, State)] = &[
        (State::Requested, Event::Accept, State::Accepted),
        (State::Requested, Event::Decline, State::Declined),
        (
            State::Requested,
            Event::RequestInformation,
            State::InformationRequested,
        ),
        (State::Requested, Event::Redirect, State::Redirected),
        (
            State::Requested,
            Event::RequestCancel,
            State::CancelRequested,
        ),
        (State::Accepted, Event::Book, State::AppointmentBooked),
        (State::Accepted, Event::InterimNote, State::InterimReport),
        (State::Accepted, Event::Outcome, State::Completed),
        (State::Accepted, Event::Decline, State::Declined),
        (State::Accepted, Event::Defer, State::Deferred),
        (
            State::Accepted,
            Event::RequestCancel,
            State::CancelRequested,
        ),
        (State::AppointmentBooked, Event::RecordNoShow, State::NoShow),
        (
            State::AppointmentBooked,
            Event::InterimNote,
            State::InterimReport,
        ),
        (State::AppointmentBooked, Event::Outcome, State::Completed),
        (State::NoShow, Event::Book, State::AppointmentBooked),
        (State::NoShow, Event::Decline, State::Declined),
        (
            State::InterimReport,
            Event::InterimNote,
            State::InterimReport,
        ),
        (State::InterimReport, Event::Outcome, State::Completed),
        (
            State::CancelRequested,
            Event::ConfirmCancel,
            State::Cancelled,
        ),
        (State::CancelRequested, Event::Outcome, State::Completed),
        (
            State::InformationRequested,
            Event::ProvideInformation,
            State::Requested,
        ),
        (State::Deferred, Event::Resume, State::Accepted),
        (
            State::Deferred,
            Event::RequestCancel,
            State::CancelRequested,
        ),
    ];

    #[test]
    fn every_pair_matches_the_listed_table_and_nothing_else_is_allowed() {
        for s in State::ALL {
            for e in Event::ALL {
                let listed = LISTED.iter().find(|(a, b, _)| *a == s && *b == e);
                match (next(s, e), listed) {
                    (Ok(got), Some((_, _, want))) => assert_eq!(got, *want, "{s:?} {e:?}"),
                    (Err(r), None) => assert_eq!(r, Refused { state: s, event: e }),
                    (got, listed) => panic!("{s:?} {e:?}: {got:?} against {listed:?}"),
                }
            }
        }
        let pairs: HashSet<_> = LISTED.iter().map(|(a, b, _)| (*a, *b)).collect();
        assert_eq!(pairs.len(), LISTED.len(), "a pair is listed twice");
    }

    #[test]
    fn an_end_state_takes_no_event() {
        for s in State::ALL.into_iter().filter(|s| s.is_end()) {
            for e in Event::ALL {
                assert!(next(s, e).is_err(), "{s:?} took {e:?}");
            }
        }
        let ends: BTreeSet<_> = State::ALL
            .into_iter()
            .filter(|s| s.is_end())
            .map(State::as_str)
            .collect();
        assert_eq!(
            ends,
            BTreeSet::from(["cancelled", "completed", "declined", "redirected"])
        );
    }

    #[test]
    fn a_cancellation_is_a_request_and_only_the_recipients_confirmation_cancels() {
        // Cancelled is entered by exactly one pair.
        let into: Vec<_> = LISTED
            .iter()
            .filter(|(_, _, to)| *to == State::Cancelled)
            .collect();
        assert_eq!(
            into,
            [&(
                State::CancelRequested,
                Event::ConfirmCancel,
                State::Cancelled
            )]
        );
        assert_eq!(Event::ConfirmCancel.actor(), Actor::Recipient);
        assert_eq!(Event::RequestCancel.actor(), Actor::Referrer);
        // The referrer's cancel never lands on Cancelled, from any state.
        for s in State::ALL {
            if let Ok(to) = next(s, Event::RequestCancel) {
                assert_eq!(to, State::CancelRequested);
            }
        }
        // The recipient may answer a request with an outcome instead.
        assert_eq!(
            next(State::CancelRequested, Event::Outcome),
            Ok(State::Completed)
        );
    }

    #[test]
    fn a_decline_can_follow_acceptance_and_only_a_decline_or_outcome_closes_the_loop() {
        assert_eq!(next(State::Accepted, Event::Decline), Ok(State::Declined));
        let closing: Vec<_> = State::ALL.into_iter().filter(|s| s.closes_loop()).collect();
        assert_eq!(closing, [State::Declined, State::Completed]);
    }

    #[test]
    fn every_live_state_can_reach_an_end_and_every_state_is_reachable() {
        // Reachable from Requested.
        let mut seen = HashSet::from([State::Requested]);
        let mut frontier = vec![State::Requested];
        while let Some(s) = frontier.pop() {
            for e in Event::ALL {
                if let Ok(to) = next(s, e)
                    && seen.insert(to)
                {
                    frontier.push(to);
                }
            }
        }
        assert_eq!(seen.len(), State::ALL.len(), "unreachable: {:?}", {
            let all: HashSet<_> = State::ALL.into_iter().collect();
            all.difference(&seen).copied().collect::<Vec<_>>()
        });
        // No dead end: from any state an end can be reached.
        for start in State::ALL {
            let mut seen = HashSet::from([start]);
            let mut frontier = vec![start];
            let mut ended = start.is_end();
            while let Some(s) = frontier.pop() {
                for e in Event::ALL {
                    if let Ok(to) = next(s, e) {
                        ended |= to.is_end();
                        if seen.insert(to) {
                            frontier.push(to);
                        }
                    }
                }
            }
            assert!(ended, "{start:?} cannot reach an end");
        }
    }

    #[test]
    fn an_interim_note_repeats_and_information_goes_back_to_requested() {
        let mut s = State::Accepted;
        for _ in 0..3 {
            s = next(s, Event::InterimNote).unwrap();
            assert_eq!(s, State::InterimReport);
        }
        assert_eq!(next(s, Event::Outcome), Ok(State::Completed));
        let asked = next(State::Requested, Event::RequestInformation).unwrap();
        assert_eq!(next(asked, Event::ProvideInformation), Ok(State::Requested));
        assert_eq!(Event::ProvideInformation.actor(), Actor::Referrer);
    }

    #[test]
    fn the_provisional_states_are_the_three_the_source_marks_to_confirm() {
        let provisional: Vec<_> = State::ALL
            .into_iter()
            .filter(|s| s.is_provisional())
            .map(State::as_str)
            .collect();
        assert_eq!(
            provisional,
            ["information-requested", "deferred", "redirected"]
        );
    }

    #[test]
    fn the_stored_words_are_unique_kebab_case_and_match_serde() {
        let mut words = BTreeSet::new();
        for s in State::ALL {
            let json = serde_json::to_value(s).unwrap();
            assert_eq!(json, serde_json::json!(s.as_str()), "{s:?}");
            let back: State = serde_json::from_value(json).unwrap();
            assert_eq!(back, s);
            assert!(words.insert(s.as_str()), "{s:?} repeats a word");
        }
        let refused = next(State::Completed, Event::Accept).unwrap_err();
        assert_eq!(
            refused.to_string(),
            "a referral that is completed cannot take Accept"
        );
    }
}
