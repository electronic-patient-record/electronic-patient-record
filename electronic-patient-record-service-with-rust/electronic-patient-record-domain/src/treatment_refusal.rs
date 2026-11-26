//! A refusal of **treatment**, as recorded (RF-4; `spec/patient-refusal-of-consent`). Not a refusal of disclosure, which is
//! `consent.rs`. It is pure, and **there is no function here that answers whether someone may be treated**, whether a
//! refusal is valid, or whether a person can decide.
//!
//! - **"Not known" is always a value**, and never decides anything.
//! - **Who said it is kept apart from whom it is about.** A relative's word, a parent's and an attorney's are not the
//!   patient's.
//! - **A withdrawal that was heard is kept at once**, as heard. Whether it ends the refusal is the place's rule
//!   ([`crate::refusal_rules`]); with no rule it raises doubt.
//! - **A form has facts and no verdict.** [`form_gaps`] lists what the place's form asks for and the record does not say.
//! - **Nothing is removed.** Entered in error is a state.

#![deny(
    clippy::arithmetic_side_effects,
    clippy::default_numeric_fallback,
    clippy::integer_division,
    clippy::modulo_arithmetic,
    clippy::as_conversions,
    clippy::string_slice
)]

use crate::coding::Coding;
use crate::refusal_rules::{Pack, Value};
use serde::{Deserialize, Serialize};

/// The longest words of a refusal, of what is accepted, of where it applies, of where the paper is and of a doubt.
pub const WORDS_LIMIT: usize = 500;

/// What kind of refusal it is. They differ in force and are often confused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    /// Said now, by the person.
    SpokenNow,
    /// Made before, for a time when they cannot say.
    Advance,
    /// A clinician's order, such as no resuscitation. It is not the patient's refusal.
    Order,
    /// By someone with power to decide for them.
    Representative,
    /// By a parent, for a child.
    Parent,
    /// A card or a band found with the person.
    Carried,
    /// Reported by someone with no power to decide.
    Reported,
}

impl Kind {
    /// Every kind (the stored words are checked against the database, PR-3).
    pub const ALL: [Kind; 7] = [
        Kind::SpokenNow,
        Kind::Advance,
        Kind::Order,
        Kind::Representative,
        Kind::Parent,
        Kind::Carried,
        Kind::Reported,
    ];

    /// The stored word, as serde writes it.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::SpokenNow => "spoken-now",
            Kind::Advance => "advance",
            Kind::Order => "order",
            Kind::Representative => "representative",
            Kind::Parent => "parent",
            Kind::Carried => "carried",
            Kind::Reported => "reported",
        }
    }
}

/// Yes, no, or it is not known.
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

/// Whether it says that it applies though life is at risk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LifeAtRisk {
    /// It says so.
    Yes,
    /// It says not.
    No,
    /// It does not say, or that was not asked.
    NotStated,
}

/// What of the paper was seen by whoever recorded it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Seen {
    /// The original.
    Original,
    /// A copy or a photograph.
    Copy,
    /// Not seen, or there is none.
    NotSeen,
}

/// The facts of its form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Form {
    /// In writing.
    pub written: Known,
    /// Signed.
    pub signed: Known,
    /// Witnessed.
    pub witnessed: Known,
    /// How many witnesses, when that is known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub witnesses: Option<u8>,
    /// What was seen.
    pub seen: Seen,
    /// Where the paper is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kept: Option<String>,
}

/// Who said it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Speaker {
    /// The person themselves.
    Patient,
    /// An attorney or agent for health.
    Attorney,
    /// A guardian or deputy, or a court.
    Guardian,
    /// A parent.
    Parent,
    /// A relative or friend with no power to decide.
    Relative,
    /// A clinician.
    Clinician,
    /// Someone else, or nobody knows.
    Other,
}

/// Whether the person it is recorded on is certainly the person it is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IdentityOf {
    /// Certainly.
    Certain,
    /// The record's identity is temporary, or the card may be another person's.
    Uncertain,
}

impl IdentityOf {
    /// Both values (the stored words are checked against the database, PR-3).
    pub const ALL: [IdentityOf; 2] = [IdentityOf::Certain, IdentityOf::Uncertain];

    /// The stored word, as serde writes it.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            IdentityOf::Certain => "certain",
            IdentityOf::Uncertain => "uncertain",
        }
    }
}

/// How it was withdrawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum How {
    /// Aloud.
    Spoken,
    /// In writing.
    Written,
    /// By what the person did since.
    Conduct,
}

/// A withdrawal that was heard, read or seen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Withdrawal {
    /// How.
    pub how: How,
    /// Who heard it, read it or saw it, in words.
    pub noticed_by: String,
    /// When (RFC 3339).
    pub at: String,
}

/// Where it stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum State {
    /// It stands as recorded.
    Stands,
    /// There is doubt, and why is written.
    Doubted,
    /// Withdrawn.
    Withdrawn,
    /// Recorded by mistake.
    EnteredInError,
}

impl State {
    /// Every state (the stored words are checked against the database, PR-3).
    pub const ALL: [State; 4] = [
        State::Stands,
        State::Doubted,
        State::Withdrawn,
        State::EnteredInError,
    ];

    /// The stored word.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            State::Stands => "stands",
            State::Doubted => "doubted",
            State::Withdrawn => "withdrawn",
            State::EnteredInError => "entered-in-error",
        }
    }

    /// Read what [`State::as_str`] writes, and nothing else.
    #[must_use]
    pub fn parse(word: &str) -> Option<State> {
        State::ALL.into_iter().find(|s| s.as_str() == word)
    }
}

/// A refusal of treatment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefusalRecord {
    /// What kind.
    pub kind: Kind,
    /// What is refused, in the person's words where they are known.
    pub treatment: String,
    /// A code for it, if one was chosen. The words are what is shown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coding: Option<Coding>,
    /// What they would accept instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accepts: Option<String>,
    /// When it is meant to apply.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub circumstances: Option<String>,
    /// Whether it says that it applies though life is at risk.
    pub if_life_at_risk: LifeAtRisk,
    /// The facts of its form.
    pub form: Form,
    /// When it was made: `YYYY`, `YYYY-MM` or `YYYY-MM-DD`. Absent is not known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub made_on: Option<String>,
    /// When the person last confirmed it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confirmed_on: Option<String>,
    /// Who said it.
    pub said_by: Speaker,
    /// Whether an attorney's, guardian's or parent's power was seen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub power_seen: Option<Known>,
    /// Whether it is certainly this person's.
    pub identity: IdentityOf,
    /// Where it stands.
    pub state: State,
    /// Why it is doubted. Required when it is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doubt: Option<String>,
    /// A withdrawal, if one was noticed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub withdrawal: Option<Withdrawal>,
}

fn words(out: &mut Vec<String>, what: &str, value: &str) {
    if value.trim().is_empty() {
        out.push(format!("{what} must not be blank"));
    } else if value.chars().count() > WORDS_LIMIT {
        out.push(format!("{what} must be up to {WORDS_LIMIT} characters"));
    }
}

/// What is wrong with a record. The words name a field and never quote it. Whether the refusal is right, or applies, is not
/// asked.
#[must_use]
pub fn validate(r: &RefusalRecord, now: jiff::Timestamp, today: jiff::civil::Date) -> Vec<String> {
    let mut out = Vec::new();
    words(&mut out, "treatment", &r.treatment);
    for (what, value) in [
        ("accepts", &r.accepts),
        ("circumstances", &r.circumstances),
        ("form.kept", &r.form.kept),
        ("doubt", &r.doubt),
    ] {
        if let Some(value) = value {
            words(&mut out, what, value);
        }
    }
    let today_text = today.to_string();
    for (what, value) in [("made_on", &r.made_on), ("confirmed_on", &r.confirmed_on)] {
        if let Some(day) = value {
            if !crate::dates::valid_birth_date(day) {
                out.push(format!("{what} must be YYYY, YYYY-MM or YYYY-MM-DD"));
            } else if crate::dates::partial_order(day, &today_text)
                == Some(std::cmp::Ordering::Greater)
            {
                out.push(format!("{what} must not be in the future"));
            }
        }
    }
    if let (Some(made), Some(confirmed)) = (&r.made_on, &r.confirmed_on)
        && crate::dates::partial_order(confirmed, made) == Some(std::cmp::Ordering::Less)
    {
        out.push("confirmed_on must not be before made_on".into());
    }
    let fits = match r.kind {
        Kind::SpokenNow | Kind::Advance | Kind::Carried => r.said_by == Speaker::Patient,
        Kind::Order => r.said_by == Speaker::Clinician,
        Kind::Representative => matches!(r.said_by, Speaker::Attorney | Speaker::Guardian),
        Kind::Parent => r.said_by == Speaker::Parent,
        Kind::Reported => !matches!(r.said_by, Speaker::Patient),
    };
    if !fits {
        out.push("said_by does not fit kind".into());
    }
    let needs_power = matches!(r.kind, Kind::Representative | Kind::Parent);
    if needs_power != r.power_seen.is_some() {
        out.push("power_seen is for a representative or a parent, and is required then".into());
    }
    if r.form.witnesses.is_some() && r.form.witnessed != Known::Yes {
        out.push("form.witnesses is for a witnessed form".into());
    }
    if r.form.witnesses == Some(0) {
        out.push("form.witnesses must be 1 or more".into());
    }
    match (r.state, &r.doubt) {
        (State::Doubted, None) => out.push("doubt is required when it is doubted".into()),
        (State::Stands, Some(_)) => out.push("doubt is for one that is doubted".into()),
        _ => {}
    }
    match (&r.withdrawal, r.state) {
        (None, State::Withdrawn) => {
            out.push("a withdrawal is required when it is withdrawn".into());
        }
        (Some(_), State::Stands) => {
            out.push("one with a withdrawal is doubted or withdrawn".into());
        }
        _ => {}
    }
    if let Some(w) = &r.withdrawal {
        words(&mut out, "withdrawal.noticed_by", &w.noticed_by);
        match crate::clock::parse_rfc3339(&w.at) {
            None => out.push("withdrawal.at must be an RFC 3339 time".into()),
            Some(at) if at > now => out.push("withdrawal.at must not be in the future".into()),
            Some(_) => {}
        }
    }
    // A card found with someone whose identity is not certain is never filed as certainly theirs.
    if r.kind == Kind::Carried && r.identity == IdentityOf::Certain && r.form.seen == Seen::NotSeen
    {
        out.push("a card that was not seen is not certainly theirs".into());
    }
    out
}

/// What a noticed withdrawal makes of a refusal here: its new state, and the doubt to record when it raises one. With no
/// rule, or no pack, it raises doubt, so that the refusal is neither gone nor untouched.
#[must_use]
pub fn after_withdrawal(how: How, pack: Option<&Pack>) -> State {
    let key = match how {
        How::Spoken => "withdrawal-spoken",
        How::Written => "withdrawal-written",
        How::Conduct => "withdrawal-conduct",
    };
    let ends = pack
        .and_then(|p| p.rule(key))
        .is_some_and(|r| r.value == Value::One("ends-it".into()));
    if ends {
        State::Withdrawn
    } else {
        State::Doubted
    }
}

/// What the place's form for an advance refusal asks for and this record does not say is there, by the pack's words. Empty
/// when the pack has no form, which is not the same as a form that is met: ask [`Pack::rule`] first.
#[must_use]
pub fn form_gaps(r: &RefusalRecord, pack: &Pack) -> Vec<String> {
    let Some(Value::Any(wanted)) = pack.rule("advance-form").map(|rule| &rule.value) else {
        return Vec::new();
    };
    wanted
        .iter()
        .filter(|w| {
            let there = match w.as_str() {
                "written" => r.form.written == Known::Yes,
                "signed" => r.form.signed == Known::Yes,
                "witnessed" => r.form.witnessed == Known::Yes,
                "two-witnesses" => r.form.witnesses.is_some_and(|n| n >= 2),
                "life-at-risk-words" => r.if_life_at_risk == LifeAtRisk::Yes,
                // Nothing in the record says these, so they are not said to be there.
                _ => false,
            };
            !there
        })
        .cloned()
        .collect()
}

/// What a list amounts to. **"None recorded" is what this record knows, and never "none".**
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub enum Summary {
    /// At least one stands or is doubted.
    Recorded {
        /// How many stand.
        stands: usize,
        /// How many are doubted.
        doubted: usize,
        /// How many of those are not certainly this person's.
        identity_uncertain: usize,
    },
    /// Some were, and all are withdrawn.
    AllWithdrawn {
        /// How many.
        withdrawn: usize,
    },
    /// None is recorded here.
    NoneRecorded,
}

/// What a list amounts to, those entered in error left out.
#[must_use]
pub fn summarise<'a>(records: impl IntoIterator<Item = &'a RefusalRecord>) -> Summary {
    let all: Vec<&RefusalRecord> = records
        .into_iter()
        .filter(|r| r.state != State::EnteredInError)
        .collect();
    let count = |state: State| all.iter().filter(|r| r.state == state).count();
    let (stands, doubted) = (count(State::Stands), count(State::Doubted));
    if stands > 0 || doubted > 0 {
        return Summary::Recorded {
            stands,
            doubted,
            identity_uncertain: all
                .iter()
                .filter(|r| r.state != State::Withdrawn && r.identity == IdentityOf::Uncertain)
                .count(),
        };
    }
    match count(State::Withdrawn) {
        0 => Summary::NoneRecorded,
        withdrawn => Summary::AllWithdrawn { withdrawn },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::refusal_rules;

    fn now() -> jiff::Timestamp {
        "2026-11-25T12:00:00Z".parse().unwrap()
    }

    fn today() -> jiff::civil::Date {
        crate::clock::parse_day("2026-11-25").unwrap()
    }

    fn advance() -> RefusalRecord {
        RefusalRecord {
            kind: Kind::Advance,
            treatment: "Blood and blood products".into(),
            coding: None,
            accepts: Some("Albumin".into()),
            circumstances: None,
            if_life_at_risk: LifeAtRisk::Yes,
            form: Form {
                written: Known::Yes,
                signed: Known::Yes,
                witnessed: Known::Yes,
                witnesses: None,
                seen: Seen::Original,
                kept: Some("In the patient's wallet".into()),
            },
            made_on: Some("2024-03".into()),
            confirmed_on: None,
            said_by: Speaker::Patient,
            power_seen: None,
            identity: IdentityOf::Certain,
            state: State::Stands,
            doubt: None,
            withdrawal: None,
        }
    }

    fn heard(how: How) -> Withdrawal {
        Withdrawal {
            how,
            noticed_by: "Nurse in charge".into(),
            at: "2026-11-25T11:00:00Z".into(),
        }
    }

    fn says(r: &RefusalRecord, what: &str) {
        let out = validate(r, now(), today());
        assert!(out.iter().any(|o| o.contains(what)), "{what}: {out:?}");
    }

    fn fine(r: &RefusalRecord) {
        assert_eq!(validate(r, now(), today()), Vec::<String>::new());
    }

    #[test]
    fn a_refusal_with_everything_unknown_can_be_recorded() {
        let r = RefusalRecord {
            accepts: None,
            if_life_at_risk: LifeAtRisk::NotStated,
            form: Form {
                written: Known::NotKnown,
                signed: Known::NotKnown,
                witnessed: Known::NotKnown,
                witnesses: None,
                seen: Seen::NotSeen,
                kept: None,
            },
            made_on: None,
            ..advance()
        };
        fine(&r);
        fine(&advance());
        let text = serde_json::to_value(&r).unwrap();
        assert_eq!(text["form"]["written"], "not-known");
        assert_eq!(serde_json::from_value::<RefusalRecord>(text).unwrap(), r);
    }

    #[test]
    fn each_thing_wrong_is_named_and_not_quoted() {
        let with = |f: &dyn Fn(&mut RefusalRecord)| {
            let mut r = advance();
            f(&mut r);
            r
        };
        says(&with(&|r| r.treatment = "  ".into()), "treatment");
        says(&with(&|r| r.treatment = "x".repeat(501)), "treatment");
        fine(&with(&|r| r.treatment = "x".repeat(500)));
        says(&with(&|r| r.accepts = Some(" ".into())), "accepts");
        says(&with(&|r| r.made_on = Some("2026-13".into())), "made_on");
        says(&with(&|r| r.made_on = Some("2026-12".into())), "future");
        fine(&with(&|r| r.made_on = Some("2026".into())));
        fine(&with(&|r| r.made_on = Some("2026-11-25".into())));
        says(
            &with(&|r| r.confirmed_on = Some("2023".into())),
            "before made_on",
        );
        fine(&with(&|r| r.confirmed_on = Some("2024-03-09".into())));
        says(&with(&|r| r.said_by = Speaker::Relative), "fit kind");
        says(&with(&|r| r.power_seen = Some(Known::No)), "power_seen");
        says(
            &with(&|r| {
                r.form.witnessed = Known::NotKnown;
                r.form.witnesses = Some(2);
            }),
            "witnesses",
        );
        says(&with(&|r| r.form.witnesses = Some(0)), "1 or more");
        says(&with(&|r| r.state = State::Doubted), "doubt is required");
        says(&with(&|r| r.doubt = Some("x".into())), "doubt is for");
        says(
            &with(&|r| r.state = State::Withdrawn),
            "withdrawal is required",
        );
        says(
            &with(&|r| r.withdrawal = Some(heard(How::Spoken))),
            "doubted or withdrawn",
        );
        for (at, what) in [("soon", "RFC 3339"), ("2026-11-25T12:00:01Z", "future")] {
            says(
                &with(&|r| {
                    r.state = State::Withdrawn;
                    r.withdrawal = Some(Withdrawal {
                        at: at.into(),
                        ..heard(How::Spoken)
                    });
                }),
                what,
            );
        }
        let secret = with(&|r| r.treatment = format!("QUOKKA{}", "x".repeat(500)));
        assert!(
            !validate(&secret, now(), today())
                .concat()
                .contains("QUOKKA")
        );
    }

    #[test]
    fn who_said_it_is_kept_apart_from_whom_it_is_about() {
        for (kind, speaker, power) in [
            (
                Kind::Representative,
                Speaker::Attorney,
                Some(Known::NotKnown),
            ),
            (Kind::Representative, Speaker::Guardian, Some(Known::Yes)),
            (Kind::Parent, Speaker::Parent, Some(Known::No)),
            (Kind::Reported, Speaker::Relative, None),
            (Kind::Order, Speaker::Clinician, None),
            (Kind::SpokenNow, Speaker::Patient, None),
        ] {
            let r = RefusalRecord {
                kind,
                said_by: speaker,
                power_seen: power,
                ..advance()
            };
            fine(&r);
            // The patient's own word is never a representative's, a parent's, a relative's or an order.
            if kind != Kind::SpokenNow {
                says(
                    &RefusalRecord {
                        said_by: Speaker::Patient,
                        ..r
                    },
                    "fit kind",
                );
            }
        }
        // A power that was not asked about is not a power.
        says(
            &RefusalRecord {
                kind: Kind::Parent,
                said_by: Speaker::Parent,
                ..advance()
            },
            "power_seen",
        );
    }

    #[test]
    fn a_card_that_nobody_saw_is_not_certainly_theirs() {
        let mut r = RefusalRecord {
            kind: Kind::Carried,
            ..advance()
        };
        fine(&r);
        r.form.seen = Seen::NotSeen;
        says(&r, "not certainly");
        r.identity = IdentityOf::Uncertain;
        fine(&r);
    }

    #[test]
    fn a_withdrawal_ends_it_only_where_the_place_says_so() {
        let pack = refusal_rules::england_wales().unwrap();
        assert_eq!(after_withdrawal(How::Spoken, Some(pack)), State::Withdrawn);
        assert_eq!(after_withdrawal(How::Written, Some(pack)), State::Withdrawn);
        // Not established there, and with no pack at all: doubt.
        assert_eq!(after_withdrawal(How::Conduct, Some(pack)), State::Doubted);
        for how in [How::Spoken, How::Written, How::Conduct] {
            assert_eq!(after_withdrawal(how, None), State::Doubted);
        }
        let place = refusal_rules::parse(
            "[pack]\nid = \"x\"\nplace = \"X\"\nlaw_as_of = \"2026-11-25\"\n[[rule]]\nkey = \"withdrawal-spoken\"\nvalue = \"raises-doubt\"\nsource = \"s\"\n",
            "x",
        )
        .unwrap();
        assert_eq!(after_withdrawal(How::Spoken, Some(&place)), State::Doubted);
    }

    #[test]
    fn what_a_form_still_lacks_is_listed_and_no_verdict_is_given() {
        let pack = refusal_rules::england_wales().unwrap();
        assert_eq!(form_gaps(&advance(), pack), Vec::<String>::new());
        let mut r = advance();
        r.form.signed = Known::NotKnown;
        r.form.witnessed = Known::No;
        r.if_life_at_risk = LifeAtRisk::NotStated;
        assert_eq!(
            form_gaps(&r, pack),
            ["signed", "witnessed", "life-at-risk-words"]
        );
        let strict = refusal_rules::parse(
            "[pack]\nid = \"x\"\nplace = \"X\"\nlaw_as_of = \"2026-11-25\"\n[[rule]]\nkey = \"advance-form\"\nvalue = [\"two-witnesses\", \"doctor\", \"registered\"]\nsource = \"s\"\n",
            "x",
        )
        .unwrap();
        // Nothing here says that a doctor signed or that it is registered, so neither is said to be there.
        r.form.witnessed = Known::Yes;
        r.form.witnesses = Some(1);
        assert_eq!(
            form_gaps(&r, &strict),
            ["two-witnesses", "doctor", "registered"]
        );
        r.form.witnesses = Some(2);
        assert_eq!(form_gaps(&r, &strict), ["doctor", "registered"]);
        let none = refusal_rules::parse(
            "[pack]\nid = \"x\"\nplace = \"X\"\nlaw_as_of = \"2026-11-25\"\n",
            "x",
        )
        .unwrap();
        assert!(form_gaps(&r, &none).is_empty() && none.rule("advance-form").is_none());
    }

    #[test]
    fn none_recorded_is_not_none_and_a_doubt_is_counted() {
        assert_eq!(summarise([]), Summary::NoneRecorded);
        let of = |state| RefusalRecord { state, ..advance() };
        assert_eq!(
            summarise(&[of(State::EnteredInError)]),
            Summary::NoneRecorded
        );
        assert_eq!(
            summarise(&[of(State::Withdrawn), of(State::EnteredInError)]),
            Summary::AllWithdrawn { withdrawn: 1 }
        );
        let unsure = RefusalRecord {
            identity: IdentityOf::Uncertain,
            ..of(State::Doubted)
        };
        let gone = RefusalRecord {
            identity: IdentityOf::Uncertain,
            ..of(State::Withdrawn)
        };
        assert_eq!(
            summarise(&[of(State::Stands), unsure, gone]),
            Summary::Recorded {
                stands: 1,
                doubted: 1,
                identity_uncertain: 1
            }
        );
        assert_eq!(serde_json::json!(summarise([]))["state"], "none-recorded");
    }

    #[test]
    fn a_stored_state_is_read_as_written_and_nothing_else_is() {
        for s in State::ALL {
            assert_eq!(State::parse(s.as_str()), Some(s));
            assert_eq!(serde_json::json!(s), s.as_str());
        }
        assert_eq!(State::parse("valid"), None);
    }

    #[test]
    fn the_stored_words_of_a_kind_and_an_identity_are_the_serde_words() {
        for k in Kind::ALL {
            assert_eq!(serde_json::json!(k), k.as_str());
        }
        for i in IdentityOf::ALL {
            assert_eq!(serde_json::json!(i), i.as_str());
        }
    }
}
