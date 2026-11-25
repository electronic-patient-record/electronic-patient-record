//! One state table for every order (EH-4, `spec/order-entry/state-table.md`): a laboratory or imaging request, a nursing
//! task, a medicine. It is pure, **nothing calls it yet**, and no clinician, pharmacist or laboratory has reviewed it.
//!
//! An order has two sides that move apart, and one word for both hides the dangerous cases: what the orderer **wants**
//! ([`State`]) and what the people who carry it out have **got to** ([`Filling`]). An order can be cancelled and done.
//!
//! - **Asking is checked; a fact is not.** A step that asks for something ([`Event::is_fact`] is false) is refused unless
//!   the table allows it. That something *was done* is never refused, in any state: refusing to record a dose that was
//!   given after a cancel would lose the dose. It is kept and flagged ([`Flag`]).
//! - **A cancel is a request** once the order has been sent. Only the filler's answer ends it, and one that was done
//!   meanwhile cannot be confirmed as cancelled.
//! - **A refusal comes back.** An order the filler refused stays the orderer's, still active, until they end it.
//! - **Done is not finished.** An order completes when the orderer closes it, for example by signing off the result.
//! - **The order's own words are kept** and never generated or changed ([`Order::words`]). A change is another order.
//!
//! Not covered: who may send a step (the policy engine's), times and overdue, doses and timetables, the link to a result,
//! and what is stored.

#![deny(
    clippy::arithmetic_side_effects,
    clippy::default_numeric_fallback,
    clippy::integer_division,
    clippy::modulo_arithmetic,
    clippy::as_conversions,
    clippy::string_slice
)]

use serde::{Deserialize, Serialize};
use std::fmt;

/// What the orderer wants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum State {
    /// Written and not signed.
    Draft,
    /// Signed: wanted.
    Active,
    /// Wanted, and not now.
    OnHold,
    /// The orderer asked to cancel and the filler has not answered. It may still be done.
    CancelRequested,
    /// No longer wanted, and nobody is going to do it. An end.
    Revoked,
    /// Done, and closed by the orderer. An end.
    Completed,
    /// It should never have been ordered (the wrong patient, for example). An end.
    EnteredInError,
}

/// What the people who carry it out have got to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Filling {
    /// They have not been told.
    NotSent,
    /// They have been sent it and have not answered.
    Sent,
    /// They said they will do it.
    Accepted,
    /// They said they will not.
    Refused,
    /// Some of it has been done: a dose, a sample.
    Started,
    /// Some was done and the rest will not be.
    Abandoned,
    /// All of it was done.
    Done,
}

/// Something that happens to an order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Event {
    /// The orderer signs a draft.
    Sign,
    /// The orderer throws a draft away.
    Discard,
    /// The service sends it.
    Send,
    /// The orderer puts it on hold.
    Hold,
    /// The orderer takes it off hold.
    Release,
    /// The orderer asks to cancel.
    RequestCancel,
    /// The orderer says it was a mistake.
    Retract,
    /// The orderer closes one that was done, or that stopped part of the way.
    Close,
    /// The filler will do it.
    Accept,
    /// The filler will not.
    Refuse,
    /// The filler stopped part of the way.
    Abandon,
    /// The filler agrees to the cancel: it has not been done and will not be.
    ConfirmCancel,
    /// The filler says it is too late to cancel, or declines to.
    DeclineCancel,
    /// A fact: some of it was done. It can happen again.
    PartDone,
    /// A fact: it was done.
    Done,
}

/// Who a step comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Actor {
    /// The clinician or team that ordered it.
    Orderer,
    /// The people who carry it out.
    Filler,
    /// The service itself.
    Service,
}

/// Why a recorded fact, or a step, needs a person to look.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Flag {
    /// Done before anyone signed the order, as after a spoken one.
    DoneBeforeSigned,
    /// Done while it was on hold.
    DoneWhileOnHold,
    /// Done after the orderer asked to cancel.
    DoneAfterCancelAsked,
    /// Done after it was cancelled.
    DoneAfterRevoked,
    /// Done after the filler said it would not be.
    DoneAfterRefused,
    /// Done again after it was all done.
    DoneAgain,
    /// Done on an order that should never have been.
    DoneOnMistake,
    /// A mistaken order has left the building: the filler must be told.
    TellFiller,
}

/// A step the table does not allow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Refused {
    /// What the orderer wanted.
    pub state: State,
    /// How far it had got.
    pub filling: Filling,
    /// The step.
    pub event: Event,
}

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "an order that is {:?} and {:?} cannot take {:?}",
            self.state, self.filling, self.event
        )
    }
}

impl std::error::Error for Refused {}

impl State {
    /// Every state.
    pub const ALL: [State; 7] = [
        State::Draft,
        State::Active,
        State::OnHold,
        State::CancelRequested,
        State::Revoked,
        State::Completed,
        State::EnteredInError,
    ];

    /// Whether the orderer is finished with it. A fact can still arrive.
    #[must_use]
    pub fn is_end(self) -> bool {
        matches!(
            self,
            State::Revoked | State::Completed | State::EnteredInError
        )
    }
}

impl Filling {
    /// Every stage.
    pub const ALL: [Filling; 7] = [
        Filling::NotSent,
        Filling::Sent,
        Filling::Accepted,
        Filling::Refused,
        Filling::Started,
        Filling::Abandoned,
        Filling::Done,
    ];

    /// Whether anything was done to the patient.
    #[must_use]
    pub fn touched(self) -> bool {
        matches!(self, Filling::Started | Filling::Abandoned | Filling::Done)
    }
}

impl Event {
    /// Every event.
    pub const ALL: [Event; 15] = [
        Event::Sign,
        Event::Discard,
        Event::Send,
        Event::Hold,
        Event::Release,
        Event::RequestCancel,
        Event::Retract,
        Event::Close,
        Event::Accept,
        Event::Refuse,
        Event::Abandon,
        Event::ConfirmCancel,
        Event::DeclineCancel,
        Event::PartDone,
        Event::Done,
    ];

    /// Whether it says that something happened, and so is never refused.
    #[must_use]
    pub fn is_fact(self) -> bool {
        matches!(self, Event::PartDone | Event::Done)
    }

    /// Who it comes from.
    #[must_use]
    pub fn actor(self) -> Actor {
        match self {
            Event::Sign
            | Event::Discard
            | Event::Hold
            | Event::Release
            | Event::RequestCancel
            | Event::Retract
            | Event::Close => Actor::Orderer,
            Event::Send => Actor::Service,
            Event::Accept
            | Event::Refuse
            | Event::Abandon
            | Event::ConfirmCancel
            | Event::DeclineCancel
            | Event::PartDone
            | Event::Done => Actor::Filler,
        }
    }
}

/// One order: its words, and where each side has got to. The words can be read and not changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Order {
    words: String,
    state: State,
    filling: Filling,
}

impl Order {
    /// A draft, in the orderer's words.
    #[must_use]
    pub fn draft(words: impl Into<String>) -> Order {
        Order {
            words: words.into(),
            state: State::Draft,
            filling: Filling::NotSent,
        }
    }

    /// What was ordered, as the orderer put it.
    #[must_use]
    pub fn words(&self) -> &str {
        &self.words
    }

    /// What the orderer wants.
    #[must_use]
    pub fn state(&self) -> State {
        self.state
    }

    /// What the filler has got to.
    #[must_use]
    pub fn filling(&self) -> Filling {
        self.filling
    }

    /// The order after `event`, and what a person must look at.
    ///
    /// # Errors
    ///
    /// [`Refused`] for a step the table does not allow. Never for a fact.
    pub fn after(&self, event: Event) -> Result<(Order, Option<Flag>), Refused> {
        let (state, filling, flag) = next(self.state, self.filling, event)?;
        let order = Order {
            words: self.words.clone(),
            state,
            filling,
        };
        Ok((order, flag))
    }
}

/// The table.
///
/// # Errors
///
/// [`Refused`] for a step it does not allow.
pub fn next(
    state: State,
    filling: Filling,
    event: Event,
) -> Result<(State, Filling, Option<Flag>), Refused> {
    use Event as E;
    use Filling as F;
    use State as S;
    let live = matches!(state, S::Active | S::OnHold | S::CancelRequested);
    let wanted = matches!(state, S::Active | S::OnHold);
    let (to, reached) = match (event, state, filling) {
        (E::Sign, S::Draft, _) => (S::Active, filling),
        (E::Discard, S::Draft, F::NotSent) => (S::Revoked, filling),
        (E::Send, S::Active, F::NotSent) => (state, F::Sent),
        (E::Hold, S::Active, _) if filling != F::Done => (S::OnHold, filling),
        (E::Release, S::OnHold, _) => (S::Active, filling),
        // Nobody is going to do it, so there is nobody to ask.
        (E::RequestCancel, _, F::NotSent | F::Refused | F::Abandoned) if wanted => {
            (S::Revoked, filling)
        }
        (E::RequestCancel, _, F::Sent | F::Accepted | F::Started) if wanted => {
            (S::CancelRequested, filling)
        }
        // A draft is thrown away, unless something was done on the strength of it: nobody has to sign what they never
        // ordered in order to be rid of it.
        (E::Retract, _, _)
            if state != S::EnteredInError && (state != S::Draft || filling.touched()) =>
        {
            let flag = (filling != F::NotSent && filling != F::Refused).then_some(Flag::TellFiller);
            return Ok((S::EnteredInError, filling, flag));
        }
        // One done before it was signed is signed first.
        (E::Close, S::Active, F::Done | F::Abandoned) => (S::Completed, filling),
        (E::Accept, _, F::Sent) if live => (state, F::Accepted),
        (E::Refuse, _, F::Sent | F::Accepted) if live => (state, F::Refused),
        (E::Abandon, _, F::Started) if live => (state, F::Abandoned),
        // Not when it was all done meanwhile.
        (E::ConfirmCancel, S::CancelRequested, _) if filling != F::Done => (S::Revoked, filling),
        (E::DeclineCancel, S::CancelRequested, _) => (S::Active, filling),
        (E::PartDone | E::Done, _, _) => {
            let reached = if event == E::Done || filling == F::Done {
                F::Done
            } else {
                F::Started
            };
            return Ok((state, reached, fact_flag(state, filling)));
        }
        _ => {
            return Err(Refused {
                state,
                filling,
                event,
            });
        }
    };
    Ok((to, reached, None))
}

/// What is wrong with something being done now. The orderer's side comes first: that it is not wanted matters more than
/// that the filler had said no.
fn fact_flag(state: State, filling: Filling) -> Option<Flag> {
    match (state, filling) {
        (State::EnteredInError, _) => Some(Flag::DoneOnMistake),
        (State::Revoked, _) => Some(Flag::DoneAfterRevoked),
        (State::CancelRequested, _) => Some(Flag::DoneAfterCancelAsked),
        (State::OnHold, _) => Some(Flag::DoneWhileOnHold),
        (State::Completed, _) | (_, Filling::Done) => Some(Flag::DoneAgain),
        (State::Draft, _) => Some(Flag::DoneBeforeSigned),
        (State::Active, Filling::Refused | Filling::Abandoned) => Some(Flag::DoneAfterRefused),
        (
            State::Active,
            Filling::NotSent | Filling::Sent | Filling::Accepted | Filling::Started,
        ) => None,
    }
}

/// The FHIR R4 `ServiceRequest.status` for a state. A cancel that was only asked for is still `active`.
#[must_use]
pub fn service_request_status(state: State) -> &'static str {
    match state {
        State::Draft => "draft",
        State::Active | State::CancelRequested => "active",
        State::OnHold => "on-hold",
        State::Revoked => "revoked",
        State::Completed => "completed",
        State::EnteredInError => "entered-in-error",
    }
}

/// The FHIR R4 `MedicationRequest.status`. One that was revoked after some was given is `stopped`, and one of which none
/// was given is `cancelled`.
#[must_use]
pub fn medication_request_status(state: State, filling: Filling) -> &'static str {
    match state {
        State::Revoked if filling.touched() => "stopped",
        State::Revoked => "cancelled",
        other => service_request_status(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet, VecDeque};

    type At = (State, Filling);

    /// The steps the table allows, written out again: from, step, to. A step that is not here is refused.
    fn listed() -> BTreeMap<(At, Event), At> {
        use Event as E;
        use Filling as F;
        use State as S;
        let mut out = BTreeMap::new();
        let mut put = |from: At, e: Event, to: At| {
            assert_eq!(out.insert((from, e), to), None, "{from:?} {e:?} twice");
        };
        for f in F::ALL {
            put((S::Draft, f), E::Sign, (S::Active, f));
            put((S::OnHold, f), E::Release, (S::Active, f));
            put((S::CancelRequested, f), E::DeclineCancel, (S::Active, f));
            for s in [
                S::Active,
                S::OnHold,
                S::CancelRequested,
                S::Revoked,
                S::Completed,
            ] {
                put((s, f), E::Retract, (S::EnteredInError, f));
            }
            if f.touched() {
                put((S::Draft, f), E::Retract, (S::EnteredInError, f));
            }
            if f != F::Done {
                put((S::Active, f), E::Hold, (S::OnHold, f));
                put((S::CancelRequested, f), E::ConfirmCancel, (S::Revoked, f));
            }
        }
        put((S::Draft, F::NotSent), E::Discard, (S::Revoked, F::NotSent));
        put((S::Active, F::NotSent), E::Send, (S::Active, F::Sent));
        for f in [F::Done, F::Abandoned] {
            put((S::Active, f), E::Close, (S::Completed, f));
        }
        for s in [S::Active, S::OnHold] {
            for f in [F::NotSent, F::Refused, F::Abandoned] {
                put((s, f), E::RequestCancel, (S::Revoked, f));
            }
            for f in [F::Sent, F::Accepted, F::Started] {
                put((s, f), E::RequestCancel, (S::CancelRequested, f));
            }
        }
        for s in [S::Active, S::OnHold, S::CancelRequested] {
            put((s, F::Sent), E::Accept, (s, F::Accepted));
            put((s, F::Sent), E::Refuse, (s, F::Refused));
            put((s, F::Accepted), E::Refuse, (s, F::Refused));
            put((s, F::Started), E::Abandon, (s, F::Abandoned));
        }
        out
    }

    fn every() -> impl Iterator<Item = (State, Filling, Event)> {
        State::ALL.into_iter().flat_map(|s| {
            Filling::ALL
                .into_iter()
                .flat_map(move |f| Event::ALL.into_iter().map(move |e| (s, f, e)))
        })
    }

    /// Every place an order can get to from a draft.
    fn reached() -> BTreeSet<At> {
        let start = Order::draft("w");
        let mut seen = BTreeSet::from([(start.state(), start.filling())]);
        let mut queue = VecDeque::from([start]);
        while let Some(order) = queue.pop_front() {
            for event in Event::ALL {
                if let Ok((to, _)) = order.after(event)
                    && seen.insert((to.state(), to.filling()))
                {
                    queue.push_back(to);
                }
            }
        }
        seen
    }

    #[test]
    fn a_step_is_allowed_when_it_is_listed_and_refused_when_it_is_not() {
        let listed = listed();
        for (s, f, e) in every().filter(|(_, _, e)| !e.is_fact()) {
            let got = next(s, f, e).ok().map(|(s, f, _)| (s, f));
            assert_eq!(got, listed.get(&((s, f), e)).copied(), "{s:?} {f:?} {e:?}");
        }
        assert_eq!(Event::ALL.len(), 15, "a new event needs rows here");
    }

    #[test]
    fn a_fact_is_never_refused_and_never_moves_what_the_orderer_wants() {
        for (s, f, e) in every().filter(|(_, _, e)| e.is_fact()) {
            let (to, reached, _) = next(s, f, e).unwrap();
            assert_eq!(to, s);
            assert!(reached.touched(), "{s:?} {f:?} {e:?}");
            // What was done stays done.
            if f == Filling::Done {
                assert_eq!(reached, Filling::Done);
            }
        }
    }

    #[test]
    fn something_done_that_should_not_have_been_is_flagged() {
        use Filling as F;
        use State as S;
        let flag = |s, f| next(s, f, Event::Done).unwrap().2;
        // The one case with nothing to say: wanted, and not refused or done already.
        for (s, f) in State::ALL
            .into_iter()
            .flat_map(|s| Filling::ALL.map(|f| (s, f)))
        {
            let fine =
                s == S::Active && matches!(f, F::NotSent | F::Sent | F::Accepted | F::Started);
            assert_eq!(flag(s, f).is_none(), fine, "{s:?} {f:?}");
        }
        assert_eq!(flag(S::Draft, F::NotSent), Some(Flag::DoneBeforeSigned));
        assert_eq!(flag(S::OnHold, F::Accepted), Some(Flag::DoneWhileOnHold));
        assert_eq!(
            flag(S::CancelRequested, F::Accepted),
            Some(Flag::DoneAfterCancelAsked)
        );
        assert_eq!(flag(S::Revoked, F::Refused), Some(Flag::DoneAfterRevoked));
        assert_eq!(flag(S::Active, F::Refused), Some(Flag::DoneAfterRefused));
        assert_eq!(flag(S::Active, F::Done), Some(Flag::DoneAgain));
        assert_eq!(flag(S::Completed, F::Abandoned), Some(Flag::DoneAgain));
        assert_eq!(flag(S::EnteredInError, F::Done), Some(Flag::DoneOnMistake));
    }

    #[test]
    fn a_mistaken_order_that_was_sent_says_to_tell_the_filler() {
        let retract = |s, f| next(s, f, Event::Retract).unwrap().2;
        assert_eq!(retract(State::Active, Filling::NotSent), None);
        assert_eq!(retract(State::Active, Filling::Refused), None);
        for f in [
            Filling::Sent,
            Filling::Accepted,
            Filling::Started,
            Filling::Abandoned,
            Filling::Done,
        ] {
            assert_eq!(
                retract(State::Completed, f),
                Some(Flag::TellFiller),
                "{f:?}"
            );
        }
    }

    #[test]
    fn an_end_takes_nothing_but_a_fact_or_being_called_a_mistake() {
        for (s, f, e) in every().filter(|(s, _, _)| s.is_end()) {
            let allowed = next(s, f, e).is_ok();
            let expected = e.is_fact() || (e == Event::Retract && s != State::EnteredInError);
            assert_eq!(allowed, expected, "{s:?} {f:?} {e:?}");
        }
    }

    #[test]
    fn nothing_that_was_all_done_is_confirmed_as_cancelled() {
        let done = next(State::CancelRequested, Filling::Accepted, Event::Done).unwrap();
        assert!(next(done.0, done.1, Event::ConfirmCancel).is_err());
        // The way out: the filler says it is too late, and the orderer closes it.
        let (s, f, _) = next(done.0, done.1, Event::DeclineCancel).unwrap();
        assert_eq!(next(s, f, Event::Close).unwrap().0, State::Completed);
    }

    #[test]
    fn a_refused_order_stays_the_orderers_until_they_end_it() {
        let (s, f, _) = next(State::Active, Filling::Sent, Event::Refuse).unwrap();
        assert_eq!((s, f), (State::Active, Filling::Refused));
        assert!(next(s, f, Event::Close).is_err());
        assert_eq!(next(s, f, Event::RequestCancel).unwrap().0, State::Revoked);
    }

    #[test]
    fn no_order_is_stuck_and_only_a_done_or_stopped_one_completes() {
        let reached = reached();
        assert!(reached.len() > 30, "{}", reached.len());
        for (s, f) in &reached {
            if *s == State::Completed {
                assert!(f.touched(), "completed and {f:?}");
            }
            if *s == State::Draft {
                assert!(
                    !matches!(f, Filling::Sent | Filling::Accepted | Filling::Refused),
                    "a draft was sent"
                );
            }
            // Some steps, and no fact, lead to an end.
            let mut seen = BTreeSet::from([(*s, *f)]);
            let mut queue = VecDeque::from([(*s, *f)]);
            let mut ends = false;
            while let Some((s, f)) = queue.pop_front() {
                ends |= s.is_end();
                for e in Event::ALL.into_iter().filter(|e| !e.is_fact()) {
                    if let Ok((s, f, _)) = next(s, f, e)
                        && e != Event::Retract
                        && seen.insert((s, f))
                    {
                        queue.push_back((s, f));
                    }
                }
            }
            assert!(ends, "{s:?} {f:?} is stuck without calling it a mistake");
        }
    }

    #[test]
    fn the_orders_own_words_never_change() {
        let words = "Amoxicillin 500 mg three times a day, stop if rash\n";
        let mut order = Order::draft(words);
        for event in [
            Event::Sign,
            Event::Send,
            Event::Accept,
            Event::PartDone,
            Event::Done,
        ] {
            order = order.after(event).unwrap().0;
            assert_eq!(order.words(), words);
        }
        let json = serde_json::to_value(&order).unwrap();
        assert_eq!(json["words"], words);
        assert_eq!(json["state"], "active");
        assert_eq!(json["filling"], "done");
        let refused = order.after(Event::Sign).unwrap_err();
        assert!(refused.to_string().contains("cannot take Sign"));
    }

    #[test]
    fn the_fhir_words_keep_an_unanswered_cancel_active_and_tell_stopped_from_cancelled() {
        assert_eq!(service_request_status(State::CancelRequested), "active");
        for (s, f) in State::ALL
            .into_iter()
            .flat_map(|s| Filling::ALL.map(|f| (s, f)))
        {
            let word = medication_request_status(s, f);
            assert!(
                [
                    "active",
                    "on-hold",
                    "cancelled",
                    "completed",
                    "entered-in-error",
                    "stopped",
                    "draft"
                ]
                .contains(&word)
            );
            assert_eq!(word == "stopped", s == State::Revoked && f.touched());
        }
    }

    #[test]
    fn each_step_comes_from_one_side() {
        for e in Event::ALL {
            assert_eq!(
                e.actor() == Actor::Filler,
                e.is_fact()
                    || matches!(
                        e,
                        Event::Accept
                            | Event::Refuse
                            | Event::Abandon
                            | Event::ConfirmCancel
                            | Event::DeclineCancel
                    )
            );
        }
        assert_eq!(Event::Send.actor(), Actor::Service);
        assert_eq!(Event::RequestCancel.actor(), Actor::Orderer);
    }
}
