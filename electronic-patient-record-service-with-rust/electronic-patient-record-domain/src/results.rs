//! Results and their sign-off (OR-4 first slice, WL-1): the rules, with no database.
//!
//! A result is a series of versions. A clinician **signs off a version**, not a result: the sign-off
//! names the version it was made on. When a result is amended or corrected after sign-off, the new
//! version is unsigned and the result reads as *changed since sign-off*, with the signed version shown
//! beside it, so a change is never hidden behind an old tick (the Welsh Clinical Portal's results
//! sign-off is the source of the idea: `spec/wales/index.md`).
//!
//! Not built here: routing a result to whoever ordered it, an inbox across patients, escalation of an
//! unsigned abnormal result, and an HL7 ORU feed (OR-4's remaining slices).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The longest test name, value, unit, range or performer kept.
pub const TEXT_LIMIT: usize = 500;

/// Where a version stands in the result's life. A FHIR `Observation`/`DiagnosticReport` status subset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    /// Early, may change.
    Preliminary,
    /// Complete and verified by the reporting service.
    Final,
    /// Changed after it was final, with new information.
    Amended,
    /// Changed after it was final, to correct an error.
    Corrected,
    /// Should never have been reported; final, nothing follows it.
    EnteredInError,
}

impl Status {
    /// Every status, in the order a result usually moves through them.
    pub const ALL: [Status; 5] = [
        Status::Preliminary,
        Status::Final,
        Status::Amended,
        Status::Corrected,
        Status::EnteredInError,
    ];

    /// The stored word (the database's check lists the same words).
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Status::Preliminary => "preliminary",
            Status::Final => "final",
            Status::Amended => "amended",
            Status::Corrected => "corrected",
            Status::EnteredInError => "entered-in-error",
        }
    }
}

/// What the reporting service says about the value. Absent means it said nothing, never "normal".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Interpretation {
    /// Within the reference range.
    Normal,
    /// Outside it.
    Abnormal,
    /// Outside it by enough to act now.
    Critical,
}

/// One version of a result, as the reporting service sent it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultRecord {
    /// The test, in the reporting service's words (for example "Serum potassium").
    pub test: String,
    /// The value as reported, text so that "<0.5" and "see report" survive.
    #[serde(default)]
    pub value: Option<String>,
    /// The unit, as reported.
    #[serde(default)]
    pub unit: Option<String>,
    /// The reference range, as reported.
    #[serde(default)]
    pub reference_range: Option<String>,
    /// The reporting service's flag, if it gave one.
    #[serde(default)]
    pub interpretation: Option<Interpretation>,
    /// Where this version stands.
    pub status: Status,
    /// When the reporting service issued this version, RFC 3339.
    pub issued: String,
    /// Who reported it (a laboratory or a person), as reported.
    #[serde(default)]
    pub performer: Option<String>,
}

/// Why a record or a change was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// The test name is blank.
    TestRequired,
    /// A field is longer than [`TEXT_LIMIT`].
    TooLong,
    /// `issued` is not RFC 3339.
    IssuedInvalid,
    /// A first version must be preliminary, final or entered in error.
    FirstStatus,
    /// After a final version only amended, corrected or entered in error may follow.
    BackToPreliminary,
    /// Nothing follows entered in error.
    AfterError,
    /// A change needs a reason.
    ReasonRequired,
    /// A version entered in error has nothing to sign.
    NothingToSign,
}

impl Refusal {
    /// A stable code for a client.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Refusal::TestRequired => "test_required",
            Refusal::TooLong => "too_long",
            Refusal::IssuedInvalid => "issued_invalid",
            Refusal::FirstStatus => "first_status",
            Refusal::BackToPreliminary => "back_to_preliminary",
            Refusal::AfterError => "after_error",
            Refusal::ReasonRequired => "reason_required",
            Refusal::NothingToSign => "nothing_to_sign",
        }
    }

    /// The words for the writer.
    #[must_use]
    pub fn words(self) -> String {
        match self {
            Refusal::TestRequired => "name the test".into(),
            Refusal::TooLong => format!("a field may be at most {TEXT_LIMIT} characters"),
            Refusal::IssuedInvalid => {
                "issued must be a date and time with its offset (RFC 3339)".into()
            }
            Refusal::FirstStatus => "a new result is preliminary, final or entered in error".into(),
            Refusal::BackToPreliminary => {
                "a final result can only be amended, corrected or entered in error".into()
            }
            Refusal::AfterError => "a result entered in error is final: record a new one".into(),
            Refusal::ReasonRequired => "say why the result changed".into(),
            Refusal::NothingToSign => "a result entered in error has nothing to sign".into(),
        }
    }
}

/// Every problem with a record, in a fixed order.
#[must_use]
pub fn problems(r: &ResultRecord) -> Vec<Refusal> {
    let mut out = Vec::new();
    if r.test.trim().is_empty() {
        out.push(Refusal::TestRequired);
    }
    let long = |s: &Option<String>| s.as_ref().is_some_and(|s| s.chars().count() > TEXT_LIMIT);
    if r.test.chars().count() > TEXT_LIMIT
        || long(&r.value)
        || long(&r.unit)
        || long(&r.reference_range)
        || long(&r.performer)
    {
        out.push(Refusal::TooLong);
    }
    if crate::clock::parse_rfc3339(&r.issued).is_none() {
        out.push(Refusal::IssuedInvalid);
    }
    out
}

/// The facts a result's task events carry (TL-3): `test`, the test's name as reported, and `abnormal`, `true` when the
/// reporting service flagged the value abnormal or critical and `false` when it flagged it normal. With no flag there is
/// no `abnormal` fact, never `false`: a rule on `abnormal` then does not match, and silence is not read as normal.
/// `critical` keeps the service's own distinction, under the same rule, so that abnormal and critical are never one thing
/// to a rule. `after_sign_off` is added by the store, which knows the signatures ([`with_sign_off`]).
#[must_use]
pub fn task_facts(record: &ResultRecord) -> BTreeMap<String, String> {
    let mut facts = BTreeMap::from([("test".to_string(), record.test.clone())]);
    if let Some(i) = record.interpretation {
        let abnormal = matches!(i, Interpretation::Abnormal | Interpretation::Critical);
        facts.insert("abnormal".to_string(), abnormal.to_string());
        let critical = i == Interpretation::Critical;
        facts.insert("critical".to_string(), critical.to_string());
    }
    facts
}

/// [`task_facts`] with `after_sign_off`: whether an earlier version had been signed off, so that what its signer acted
/// on may no longer be what the result says.
#[must_use]
pub fn with_sign_off(record: &ResultRecord, after_sign_off: bool) -> BTreeMap<String, String> {
    let mut facts = task_facts(record);
    facts.insert("after_sign_off".to_string(), after_sign_off.to_string());
    facts
}

/// Whether a later version that stands is filed again as news (TL-3): it is abnormal or critical, or an earlier version
/// was signed off, whatever it says now. A rule then decides, and an open task for the same result is not made twice.
#[must_use]
pub fn raises_again(now: &ResultRecord, after_sign_off: bool) -> bool {
    let abnormal = matches!(
        now.interpretation,
        Some(Interpretation::Abnormal | Interpretation::Critical)
    );
    now.status != Status::EnteredInError && (abnormal || after_sign_off)
}

/// Whether a first version may have this status.
#[must_use]
pub fn first_refusal(status: Status) -> Option<Refusal> {
    match status {
        Status::Preliminary | Status::Final | Status::EnteredInError => None,
        Status::Amended | Status::Corrected => Some(Refusal::FirstStatus),
    }
}

/// Whether `next` may follow `was`, with `reason` given for the change.
#[must_use]
pub fn change_refusal(was: Status, next: Status, reason: Option<&str>) -> Option<Refusal> {
    if was == Status::EnteredInError {
        return Some(Refusal::AfterError);
    }
    if was != Status::Preliminary && next == Status::Preliminary {
        return Some(Refusal::BackToPreliminary);
    }
    if reason.is_none_or(|r| r.trim().is_empty()) {
        return Some(Refusal::ReasonRequired);
    }
    None
}

/// Where a result stands against its sign-offs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SignOff {
    /// No version has been signed off.
    Unsigned,
    /// The current version is signed off.
    Signed,
    /// An earlier version was signed off and the result has changed since: it needs signing again.
    ChangedSinceSignOff,
    /// Entered in error: nothing to sign.
    NotApplicable,
}

/// The sign-off state of a result at `current`, given the versions that were signed off and its status.
#[must_use]
pub fn sign_off_state(current: i32, status: Status, signed: &[i32]) -> SignOff {
    if status == Status::EnteredInError {
        return SignOff::NotApplicable;
    }
    if signed.contains(&current) {
        SignOff::Signed
    } else if signed.iter().any(|v| *v < current) {
        SignOff::ChangedSinceSignOff
    } else {
        SignOff::Unsigned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(status: Status) -> ResultRecord {
        ResultRecord {
            test: "Serum potassium".into(),
            value: Some("5.9".into()),
            unit: Some("mmol/L".into()),
            reference_range: Some("3.5-5.3".into()),
            interpretation: Some(Interpretation::Abnormal),
            status,
            issued: "2026-11-09T09:00:00Z".into(),
            performer: None,
        }
    }

    #[test]
    fn a_good_record_has_no_problems_and_a_bad_one_lists_each() {
        assert!(problems(&record(Status::Final)).is_empty());
        let mut bad = record(Status::Final);
        bad.test = " ".into();
        bad.issued = "yesterday".into();
        bad.value = Some("x".repeat(TEXT_LIMIT + 1));
        assert_eq!(
            problems(&bad),
            vec![
                Refusal::TestRequired,
                Refusal::TooLong,
                Refusal::IssuedInvalid
            ]
        );
    }

    #[test]
    fn statuses_move_forward_only_and_a_change_needs_a_reason() {
        assert_eq!(first_refusal(Status::Final), None);
        assert_eq!(first_refusal(Status::Corrected), Some(Refusal::FirstStatus));
        assert_eq!(
            change_refusal(Status::Preliminary, Status::Final, Some("complete")),
            None
        );
        assert_eq!(
            change_refusal(Status::Final, Status::Preliminary, Some("x")),
            Some(Refusal::BackToPreliminary)
        );
        assert_eq!(
            change_refusal(Status::Final, Status::Corrected, None),
            Some(Refusal::ReasonRequired)
        );
        assert_eq!(
            change_refusal(Status::Final, Status::Corrected, Some(" ")),
            Some(Refusal::ReasonRequired)
        );
        assert_eq!(
            change_refusal(Status::EnteredInError, Status::Final, Some("x")),
            Some(Refusal::AfterError)
        );
    }

    #[test]
    fn a_sign_off_counts_for_the_version_it_named() {
        assert_eq!(sign_off_state(1, Status::Final, &[]), SignOff::Unsigned);
        assert_eq!(sign_off_state(1, Status::Final, &[1]), SignOff::Signed);
        assert_eq!(
            sign_off_state(2, Status::Corrected, &[1]),
            SignOff::ChangedSinceSignOff
        );
        assert_eq!(
            sign_off_state(2, Status::Corrected, &[1, 2]),
            SignOff::Signed
        );
        assert_eq!(
            sign_off_state(3, Status::EnteredInError, &[1]),
            SignOff::NotApplicable
        );
    }

    #[test]
    fn stored_words_are_the_serde_words() {
        for s in Status::ALL {
            assert_eq!(serde_json::to_value(s).unwrap(), s.word());
        }
    }

    #[test]
    fn task_facts_say_abnormal_only_when_the_service_flagged_it() {
        let mut r = ResultRecord {
            test: "Serum potassium".into(),
            value: Some("6.1".into()),
            unit: None,
            reference_range: None,
            interpretation: None,
            status: Status::Final,
            issued: "2026-11-18T09:00:00Z".into(),
            performer: None,
        };
        let f = task_facts(&r);
        assert_eq!(f.get("test").map(String::as_str), Some("Serum potassium"));
        assert!(!f.contains_key("abnormal"), "no flag is no fact, not false");
        assert!(!f.contains_key("critical"), "nor is it not critical");
        for (i, want, critical) in [
            (Interpretation::Normal, "false", "false"),
            (Interpretation::Abnormal, "true", "false"),
            (Interpretation::Critical, "true", "true"),
        ] {
            r.interpretation = Some(i);
            let f = task_facts(&r);
            assert_eq!(f.get("abnormal").map(String::as_str), Some(want));
            assert_eq!(f.get("critical").map(String::as_str), Some(critical));
        }
        for after in [true, false] {
            let f = with_sign_off(&r, after);
            assert_eq!(
                f.get("after_sign_off").map(String::as_str),
                Some(if after { "true" } else { "false" })
            );
            for fact in f.keys() {
                assert!(
                    crate::task_rules::Event::ResultFiled
                        .facts()
                        .contains(&fact.as_str())
                );
            }
        }
        // What is filed again: anything abnormal, and anything after a signature, unless it is in error.
        for (i, after, again) in [
            (Some(Interpretation::Critical), false, true),
            (Some(Interpretation::Abnormal), false, true),
            (Some(Interpretation::Normal), false, false),
            (None, false, false),
            (Some(Interpretation::Normal), true, true),
            (None, true, true),
        ] {
            r.interpretation = i;
            r.status = Status::Corrected;
            assert_eq!(raises_again(&r, after), again, "{i:?} {after}");
            r.status = Status::EnteredInError;
            assert!(!raises_again(&r, after), "{i:?} {after} in error");
        }
        r.status = Status::Final;
        for fact in task_facts(&r).keys() {
            assert!(
                crate::task_rules::Event::ResultFiled
                    .facts()
                    .contains(&fact.as_str()),
                "{fact} is a fact the event declares"
            );
        }
    }
}
