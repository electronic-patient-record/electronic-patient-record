//! Family history (EH-3, `spec/family-history/index.md`): the rules, with no database.
//!
//! An entry is a finding about a **relative**, kept in the patient's record. The openEHR Reference Model makes the
//! subject of every entry explicit, because an entry about a family member that does not say so reads as a finding
//! about the patient (`spec/lessons/openehr-rust/index.md`). So the relationship is required here, the entries are
//! their own kind of record and never appear on the problem list, and a relative is **never named**: the relationship
//! and a number tell two sisters apart (hazard H-065, and OQ-CF3: a relative has not agreed to be in this record).
//!
//! The relationships are of blood, since an inherited risk is what an entry is for. *Absent* ("the mother did not have
//! it") is a recorded finding; *unknown* is another; and an empty list says neither.

use crate::coding::{self, Coding};
use serde::{Deserialize, Serialize};

/// The longest condition kept. Short on purpose: a place for a condition, not for a story about a relative.
pub const CONDITION_LIMIT: usize = 120;
/// The oldest age kept.
pub const AGE_LIMIT: u8 = 130;
/// The most relatives of one kind that can be told apart.
pub const MEMBER_LIMIT: u8 = 20;

/// The relative, by blood.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Relationship {
    /// The biological mother.
    Mother,
    /// The biological father.
    Father,
    /// A full sister.
    Sister,
    /// A full brother.
    Brother,
    /// A half-sister.
    HalfSister,
    /// A half-brother.
    HalfBrother,
    /// A daughter.
    Daughter,
    /// A son.
    Son,
    /// The mother's mother.
    MaternalGrandmother,
    /// The mother's father.
    MaternalGrandfather,
    /// The father's mother.
    PaternalGrandmother,
    /// The father's father.
    PaternalGrandfather,
    /// The mother's sister.
    MaternalAunt,
    /// The mother's brother.
    MaternalUncle,
    /// The father's sister.
    PaternalAunt,
    /// The father's brother.
    PaternalUncle,
    /// A cousin, the side not stated.
    Cousin,
    /// A blood relative; which one is not known.
    UnknownRelative,
}

impl Relationship {
    /// Every relationship.
    pub const ALL: [Relationship; 18] = [
        Relationship::Mother,
        Relationship::Father,
        Relationship::Sister,
        Relationship::Brother,
        Relationship::HalfSister,
        Relationship::HalfBrother,
        Relationship::Daughter,
        Relationship::Son,
        Relationship::MaternalGrandmother,
        Relationship::MaternalGrandfather,
        Relationship::PaternalGrandmother,
        Relationship::PaternalGrandfather,
        Relationship::MaternalAunt,
        Relationship::MaternalUncle,
        Relationship::PaternalAunt,
        Relationship::PaternalUncle,
        Relationship::Cousin,
        Relationship::UnknownRelative,
    ];

    /// Whether a person has at most one, so that a number would be a mistake.
    #[must_use]
    pub fn single(self) -> bool {
        matches!(
            self,
            Relationship::Mother
                | Relationship::Father
                | Relationship::MaternalGrandmother
                | Relationship::MaternalGrandfather
                | Relationship::PaternalGrandmother
                | Relationship::PaternalGrandfather
        )
    }
}

/// Whether the relative had the condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Presence {
    /// They had it.
    Present,
    /// They did not.
    Absent,
    /// It was asked and is not known.
    Unknown,
}

impl Presence {
    /// Every presence.
    pub const ALL: [Presence; 3] = [Presence::Present, Presence::Absent, Presence::Unknown];
}

/// Who said so. A relative's own health record is not a source: using it is a disclosure about the relative, which this
/// does not decide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Source {
    /// The patient.
    Patient,
    /// Someone acting for the patient.
    ActingForPatient,
    /// A genetics service's report about this patient.
    GeneticsService,
    /// Another professional's letter or record about this patient.
    Professional,
}

impl Source {
    /// Every source.
    pub const ALL: [Source; 4] = [
        Source::Patient,
        Source::ActingForPatient,
        Source::GeneticsService,
        Source::Professional,
    ];
}

/// Whether the entry stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    /// It stands.
    Recorded,
    /// Recorded by mistake. Final: it cannot be reopened.
    EnteredInError,
}

impl Status {
    /// Every status (the stored words are checked against the database `CHECK`, PR-3).
    pub const ALL: [Status; 2] = [Status::Recorded, Status::EnteredInError];

    /// The stored word.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Recorded => "recorded",
            Status::EnteredInError => "entered-in-error",
        }
    }

    /// The status a stored word names, or `None` for one the service does not write.
    #[must_use]
    pub fn parse(s: &str) -> Option<Status> {
        Status::ALL.into_iter().find(|v| v.as_str() == s)
    }
}

/// One entry. It has no place for a name, a birth date or a note, and a field it does not know is refused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FamilyHistoryRecord {
    /// The relative.
    pub relationship: Relationship,
    /// Which one, when there are several: 1 for the first sister, 2 for the second. A number and never a name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member: Option<u8>,
    /// The condition in words.
    pub condition: String,
    /// The same, coded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coding: Option<Coding>,
    /// Whether they had it.
    pub presence: Presence,
    /// How old they were when it started, in whole years, if known. Only when they had it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub onset_age: Option<u8>,
    /// Who said so.
    pub source: Source,
    /// Whether the entry stands.
    pub status: Status,
}

/// What is wrong with a record, in words that name a field and never quote it; empty if nothing.
#[must_use]
pub fn validate(r: &FamilyHistoryRecord) -> Vec<String> {
    let mut out = Vec::new();
    if r.condition.trim().is_empty() {
        out.push("the condition must not be blank".into());
    } else if r.condition.chars().count() > CONDITION_LIMIT
        || r.condition.chars().any(char::is_control)
    {
        out.push(format!(
            "the condition must be up to {CONDITION_LIMIT} characters, without control characters"
        ));
    }
    if let Some(c) = &r.coding {
        out.extend(coding::problems(c));
    }
    match r.member {
        Some(_) if r.relationship.single() => {
            out.push("a number is only for a relative there can be several of".into());
        }
        Some(n) if !(1..=MEMBER_LIMIT).contains(&n) => {
            out.push(format!("member must be 1 to {MEMBER_LIMIT}"));
        }
        _ => {}
    }
    if r.onset_age.is_some_and(|a| a > AGE_LIMIT) {
        out.push(format!("onset_age must be up to {AGE_LIMIT}"));
    }
    if r.onset_age.is_some() && r.presence != Presence::Present {
        out.push("onset_age is only for a condition the relative had".into());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(relationship: Relationship) -> FamilyHistoryRecord {
        FamilyHistoryRecord {
            relationship,
            member: None,
            condition: "Thyroid disease".into(),
            coding: None,
            presence: Presence::Present,
            onset_age: None,
            source: Source::Patient,
            status: Status::Recorded,
        }
    }

    #[test]
    fn the_words_round_trip_and_an_unknown_one_is_refused() {
        for s in Status::ALL {
            assert_eq!(Status::parse(s.as_str()), Some(s));
            assert_eq!(serde_json::to_value(s).unwrap(), s.as_str());
        }
        assert_eq!(Status::parse("active"), None);
    }

    #[test]
    fn the_relative_is_required_and_cannot_be_named() {
        let base = serde_json::to_value(rec(Relationship::Mother)).unwrap();
        assert!(serde_json::from_value::<FamilyHistoryRecord>(base.clone()).is_ok());
        let mut without = base.clone();
        without.as_object_mut().unwrap().remove("relationship");
        assert!(serde_json::from_value::<FamilyHistoryRecord>(without).is_err());
        for field in ["name", "birth_date", "note", "patient_id"] {
            let mut named = base.clone();
            named[field] = serde_json::json!("x");
            assert!(
                serde_json::from_value::<FamilyHistoryRecord>(named).is_err(),
                "{field}"
            );
        }
        let mut adoptive = base;
        adoptive["relationship"] = serde_json::json!("adoptive-mother");
        assert!(
            serde_json::from_value::<FamilyHistoryRecord>(adoptive).is_err(),
            "only relatives by blood"
        );
    }

    #[test]
    fn a_plain_record_passes_for_every_relative() {
        for r in Relationship::ALL {
            assert!(validate(&rec(r)).is_empty(), "{r:?}");
        }
    }

    #[test]
    fn two_sisters_are_told_apart_by_number_and_one_mother_has_none() {
        for (relationship, member, ok) in [
            (Relationship::Sister, 1, true),
            (Relationship::Sister, MEMBER_LIMIT, true),
            (Relationship::Sister, 0, false),
            (Relationship::Sister, MEMBER_LIMIT + 1, false),
            (Relationship::Mother, 1, false),
            (Relationship::PaternalGrandfather, 1, false),
        ] {
            let mut r = rec(relationship);
            r.member = Some(member);
            assert_eq!(validate(&r).is_empty(), ok, "{relationship:?} {member}");
        }
    }

    #[test]
    fn an_age_belongs_to_a_condition_the_relative_had() {
        let mut r = rec(Relationship::Father);
        r.onset_age = Some(AGE_LIMIT);
        assert!(validate(&r).is_empty());
        r.onset_age = Some(AGE_LIMIT + 1);
        assert_eq!(validate(&r).len(), 1);
        for presence in [Presence::Absent, Presence::Unknown] {
            r.onset_age = Some(40);
            r.presence = presence;
            assert_eq!(validate(&r).len(), 1, "{presence:?}");
        }
    }

    #[test]
    fn the_condition_is_short_and_a_bad_code_is_named() {
        let mut r = rec(Relationship::Cousin);
        r.condition = " ".into();
        assert_eq!(validate(&r), ["the condition must not be blank"]);
        r.condition = "x".repeat(CONDITION_LIMIT + 1);
        assert_eq!(validate(&r).len(), 1);
        r.condition = "a\nb".into();
        assert_eq!(validate(&r).len(), 1);
        r.condition = "Thyroid disease".into();
        r.coding = Some(Coding {
            system: "x".into(),
            code: "1".into(),
        });
        assert_eq!(validate(&r).len(), 1);
    }
}
