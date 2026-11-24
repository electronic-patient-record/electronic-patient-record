//! Relative history: inherited and shared-risk history (EH-3, `spec/relative-history/index.md`). The rules, with no
//! database.
//!
//! An entry is a finding about a **relative**, kept in the patient's record. The openEHR Reference Model makes the
//! subject of every entry explicit, because an entry about a relative that does not say so reads as a finding about
//! the patient (`spec/lessons/openehr-rust/index.md`). So the entries are their own kind of record and never appear on
//! the problem list, and a relative is **never named** (hazard H-065, OQ-CF3: they have not agreed to be here).
//!
//! A word such as "mother" or "brother" mixes facts that matter for different reasons, so they are asked apart and none
//! is guessed from another:
//!
//! 1. the **genetic** link, which an inherited condition follows;
//! 2. the **gestational** link, which an infection or exposure in pregnancy follows (with a donated egg or a surrogate
//!    the two are different people);
//! 3. the **environment** they shared, which an exposure follows whoever the parents are.
//!
//! The legal or social role (adoptive, step, foster) is left out: it does not help to assess a risk and it tells of an
//! adoption. *Absent* ("they did not have it") is a recorded finding; *unknown* is another; an empty list says neither.

use crate::coding::{self, Coding};
use serde::{Deserialize, Serialize};

/// The longest condition kept. Short on purpose: a place for a condition, not for a story about a relative.
pub const CONDITION_LIMIT: usize = 120;
/// The oldest age kept.
pub const AGE_LIMIT: u8 = 130;
/// The most relatives of one kind that can be told apart.
pub const MEMBER_LIMIT: u8 = 20;

/// How the relative and the patient share genes, if they do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GeneticLink {
    /// The egg or the sperm was theirs: a birth parent or a donor. [`Line`] says which.
    GeneticParent,
    /// Both genetic parents shared. Not a twin.
    FullSibling,
    /// One genetic parent shared. [`Line`] says which.
    HalfSibling,
    /// A twin from the same egg.
    IdenticalTwin,
    /// A twin from another egg.
    FraternalTwin,
    /// A child of the patient's egg or sperm.
    GeneticChild,
    /// A genetic parent's genetic parent.
    Grandparent,
    /// A genetic parent's sibling.
    AuntUncle,
    /// A genetic parent's sibling's child.
    Cousin,
    /// They share none by descent: adoptive, step or foster, a partner, a carrier of the pregnancy only.
    #[serde(rename = "none")]
    Unrelated,
    /// Asked and not known: "a brother", with nothing to say of which kind.
    Unknown,
}

impl GeneticLink {
    /// Every link.
    pub const ALL: [GeneticLink; 11] = [
        GeneticLink::GeneticParent,
        GeneticLink::FullSibling,
        GeneticLink::HalfSibling,
        GeneticLink::IdenticalTwin,
        GeneticLink::FraternalTwin,
        GeneticLink::GeneticChild,
        GeneticLink::Grandparent,
        GeneticLink::AuntUncle,
        GeneticLink::Cousin,
        GeneticLink::Unrelated,
        GeneticLink::Unknown,
    ];

    /// Whether the link has a side, so that [`Line`] is required.
    #[must_use]
    pub fn has_line(self) -> bool {
        matches!(
            self,
            GeneticLink::GeneticParent
                | GeneticLink::HalfSibling
                | GeneticLink::Grandparent
                | GeneticLink::AuntUncle
                | GeneticLink::Cousin
        )
    }
}

/// Through which genetic parent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Line {
    /// Through the one whose egg it was.
    Maternal,
    /// Through the one whose sperm it was.
    Paternal,
    /// Asked and not known.
    Unknown,
}

impl Line {
    /// Every line.
    pub const ALL: [Line; 3] = [Line::Maternal, Line::Paternal, Line::Unknown];
}

/// Whether the relative carried the pregnancy the patient was born from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Gestational {
    /// They did.
    Yes,
    /// They did not.
    No,
    /// Asked and not known.
    Unknown,
}

impl Gestational {
    /// Every answer.
    pub const ALL: [Gestational; 3] = [Gestational::Yes, Gestational::No, Gestational::Unknown];
}

/// When the relative and the patient lived in the same place, in the patient's life.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Environment {
    /// In the patient's childhood.
    Childhood,
    /// In the patient's adult life.
    Adulthood,
    /// In both.
    Both,
    /// Never.
    Never,
    /// Asked and not known.
    Unknown,
}

impl Environment {
    /// Every environment.
    pub const ALL: [Environment; 5] = [
        Environment::Childhood,
        Environment::Adulthood,
        Environment::Both,
        Environment::Never,
        Environment::Unknown,
    ];

    /// Whether they are known to have shared one.
    #[must_use]
    pub fn shared(self) -> bool {
        matches!(
            self,
            Environment::Childhood | Environment::Adulthood | Environment::Both
        )
    }
}

/// The relative's sex at birth, where an inheritance depends on it. Absent means not stated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Sex {
    /// Female.
    Female,
    /// Male.
    Male,
}

impl Sex {
    /// Each sex.
    pub const ALL: [Sex; 2] = [Sex::Female, Sex::Male];
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
pub struct RelativeHistoryRecord {
    /// How they share genes. Required and never guessed.
    pub genetic: GeneticLink,
    /// Through which genetic parent. Required where the link has a side, and refused where it has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<Line>,
    /// Whether they carried the pregnancy. Required for a genetic parent whose line is not paternal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gestational: Option<Gestational>,
    /// When they shared an environment. Required and never guessed.
    pub environment: Environment,
    /// Their sex at birth, if it matters and is known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sex: Option<Sex>,
    /// Which one, when there are several: 1 for the first, 2 for the second. A number and never a name.
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

impl RelativeHistoryRecord {
    /// Whether the entry bears on inherited risk: `Some(false)` for a relative who shares no genes, which must never be
    /// counted towards it, and `None` when the link is not known.
    #[must_use]
    pub fn inherited(&self) -> Option<bool> {
        match self.genetic {
            GeneticLink::Unrelated => Some(false),
            GeneticLink::Unknown => None,
            _ => Some(true),
        }
    }
}

/// What is wrong with the links between the relative and the patient.
fn link_faults(r: &RelativeHistoryRecord) -> Vec<String> {
    let mut out = Vec::new();
    match r.line {
        None if r.genetic.has_line() => {
            out.push(
                "line is required for this genetic link: maternal, paternal or unknown".into(),
            );
        }
        Some(_) if !r.genetic.has_line() => {
            out.push("line is only for a genetic link that has a side".into());
        }
        _ => {}
    }
    // Any other relative may have carried the pregnancy: an aunt or a grandmother can be a surrogate.
    let carried_by_none = matches!(
        r.genetic,
        GeneticLink::IdenticalTwin | GeneticLink::FraternalTwin | GeneticLink::GeneticChild
    ) || (r.genetic == GeneticLink::GeneticParent
        && r.line == Some(Line::Paternal));
    match r.gestational {
        Some(_) if carried_by_none => {
            out.push(
                "gestational is not for a relative who cannot have carried the pregnancy".into(),
            );
        }
        None if r.genetic == GeneticLink::GeneticParent
            && matches!(r.line, Some(Line::Maternal | Line::Unknown)) =>
        {
            out.push("gestational is required for this parent: yes, no or unknown".into());
        }
        _ => {}
    }
    if r.genetic == GeneticLink::Unrelated
        && r.gestational != Some(Gestational::Yes)
        && !r.environment.shared()
    {
        out.push(
            "no genetic link, pregnancy or shared environment: there is nothing to record".into(),
        );
    }
    out
}

/// What is wrong with a record, in words that name a field and never quote it; empty if nothing.
#[must_use]
pub fn validate(r: &RelativeHistoryRecord) -> Vec<String> {
    let mut out = link_faults(r);
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
    if r.member.is_some_and(|n| !(1..=MEMBER_LIMIT).contains(&n)) {
        out.push(format!("member must be 1 to {MEMBER_LIMIT}"));
    }
    if r.onset_age.is_some_and(|a| a > AGE_LIMIT) {
        out.push(format!("onset_age must be up to {AGE_LIMIT}"));
    }
    if r.onset_age.is_some() && r.presence != Presence::Present {
        out.push("onset_age is only for a condition the relative had".into());
    }
    out
}

/// What a list amounts to. An empty one is unknown, never "nothing in the relatives".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub enum Summary {
    /// At least one entry stands.
    Recorded {
        /// About a relative who shares genes.
        inherited: usize,
        /// About one who shares none: never counted towards inherited risk.
        not_inherited: usize,
        /// About one whose genetic link is not known.
        link_unknown: usize,
    },
    /// Nothing is recorded.
    NotRecorded,
}

/// The summary of the current versions, those entered in error left out.
#[must_use]
pub fn summarise<'a>(records: impl IntoIterator<Item = &'a RelativeHistoryRecord>) -> Summary {
    let standing: Vec<Option<bool>> = records
        .into_iter()
        .filter(|r| r.status != Status::EnteredInError)
        .map(RelativeHistoryRecord::inherited)
        .collect();
    if standing.is_empty() {
        return Summary::NotRecorded;
    }
    let count = |wanted: Option<bool>| standing.iter().filter(|i| **i == wanted).count();
    Summary::Recorded {
        inherited: count(Some(true)),
        not_inherited: count(Some(false)),
        link_unknown: count(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(
        genetic: GeneticLink,
        line: Option<Line>,
        gestational: Option<Gestational>,
        environment: Environment,
    ) -> RelativeHistoryRecord {
        RelativeHistoryRecord {
            genetic,
            line,
            gestational,
            environment,
            sex: None,
            member: None,
            condition: "Thyroid disease".into(),
            coding: None,
            presence: Presence::Present,
            onset_age: None,
            source: Source::Patient,
            status: Status::Recorded,
        }
    }

    fn sibling() -> RelativeHistoryRecord {
        rec(GeneticLink::FullSibling, None, None, Environment::Childhood)
    }

    #[test]
    fn the_words_round_trip_and_an_unknown_one_is_refused() {
        for s in Status::ALL {
            assert_eq!(Status::parse(s.as_str()), Some(s));
            assert_eq!(serde_json::to_value(s).unwrap(), s.as_str());
        }
        assert_eq!(Status::parse("active"), None);
        assert_eq!(
            serde_json::to_value(GeneticLink::Unrelated).unwrap(),
            "none"
        );
    }

    #[test]
    fn the_worked_examples_of_the_specification_are_accepted() {
        use Environment::{Childhood, Never};
        use GeneticLink::{GeneticParent, HalfSibling, IdenticalTwin, Unrelated};
        use Gestational::{No, Yes};
        use Line::{Maternal, Paternal};
        for (who, r, inherited) in [
            (
                "a birth mother who raised the patient",
                rec(GeneticParent, Some(Maternal), Some(Yes), Childhood),
                Some(true),
            ),
            (
                "an egg donor",
                rec(GeneticParent, Some(Maternal), Some(No), Never),
                Some(true),
            ),
            (
                "a sperm donor",
                rec(GeneticParent, Some(Paternal), None, Never),
                Some(true),
            ),
            (
                "a surrogate",
                rec(Unrelated, None, Some(Yes), Never),
                Some(false),
            ),
            (
                "an adoptive father",
                rec(Unrelated, None, None, Childhood),
                Some(false),
            ),
            (
                "a half-brother by the same mother, raised apart",
                rec(HalfSibling, Some(Maternal), None, Never),
                Some(true),
            ),
            (
                "an identical twin",
                rec(IdenticalTwin, None, None, Childhood),
                Some(true),
            ),
            (
                "a grandmother who carried the pregnancy",
                rec(GeneticLink::Grandparent, Some(Maternal), Some(Yes), Never),
                Some(true),
            ),
            (
                "a brother, of which kind is not known",
                rec(GeneticLink::Unknown, None, None, Environment::Unknown),
                None,
            ),
        ] {
            assert!(validate(&r).is_empty(), "{who}: {:?}", validate(&r));
            assert_eq!(r.inherited(), inherited, "{who}");
        }
    }

    #[test]
    fn no_link_is_ever_guessed() {
        let base = serde_json::to_value(sibling()).unwrap();
        assert!(serde_json::from_value::<RelativeHistoryRecord>(base.clone()).is_ok());
        for field in ["genetic", "environment", "presence", "source", "status"] {
            let mut without = base.clone();
            without.as_object_mut().unwrap().remove(field);
            assert!(
                serde_json::from_value::<RelativeHistoryRecord>(without).is_err(),
                "{field}"
            );
        }
        // The mixed words, and the legal or social role, are not offered.
        for word in ["mother", "brother", "sister", "adoptive", "step-sibling"] {
            let mut mixed = base.clone();
            mixed["genetic"] = serde_json::json!(word);
            assert!(
                serde_json::from_value::<RelativeHistoryRecord>(mixed).is_err(),
                "{word}"
            );
        }
    }

    #[test]
    fn a_relative_cannot_be_named() {
        let base = serde_json::to_value(sibling()).unwrap();
        for field in ["name", "birth_date", "note", "patient_id", "relationship"] {
            let mut named = base.clone();
            named[field] = serde_json::json!("x");
            assert!(
                serde_json::from_value::<RelativeHistoryRecord>(named).is_err(),
                "{field}"
            );
        }
    }

    #[test]
    fn a_side_is_required_exactly_where_there_is_one() {
        for genetic in GeneticLink::ALL {
            let mut r = rec(genetic, None, None, Environment::Childhood);
            let wants =
                |r: &RelativeHistoryRecord| validate(r).iter().any(|p| p.starts_with("line"));
            assert_eq!(wants(&r), genetic.has_line(), "{genetic:?} with none");
            for line in Line::ALL {
                r.line = Some(line);
                assert_eq!(wants(&r), !genetic.has_line(), "{genetic:?} {line:?}");
            }
        }
    }

    #[test]
    fn who_carried_the_pregnancy_is_asked_of_those_who_could_have() {
        let says = |r: &RelativeHistoryRecord| {
            validate(r)
                .iter()
                .filter(|p| p.starts_with("gestational"))
                .count()
        };
        for (line, required) in [(Line::Maternal, 1), (Line::Unknown, 1), (Line::Paternal, 0)] {
            let r = rec(
                GeneticLink::GeneticParent,
                Some(line),
                None,
                Environment::Childhood,
            );
            assert_eq!(says(&r), required, "{line:?}");
        }
        for (genetic, line) in [
            (GeneticLink::IdenticalTwin, None),
            (GeneticLink::FraternalTwin, None),
            (GeneticLink::GeneticChild, None),
            (GeneticLink::GeneticParent, Some(Line::Paternal)),
        ] {
            for answer in Gestational::ALL {
                let r = rec(genetic, line, Some(answer), Environment::Never);
                assert_eq!(says(&r), 1, "{genetic:?} {answer:?}");
            }
        }
    }

    #[test]
    fn someone_with_no_link_of_any_kind_is_not_recorded() {
        for (gestational, environment, ok) in [
            (None, Environment::Never, false),
            (None, Environment::Unknown, false),
            (Some(Gestational::No), Environment::Never, false),
            (Some(Gestational::Unknown), Environment::Unknown, false),
            (Some(Gestational::Yes), Environment::Never, true),
            (None, Environment::Childhood, true),
            (None, Environment::Adulthood, true),
            (None, Environment::Both, true),
        ] {
            let r = rec(GeneticLink::Unrelated, None, gestational, environment);
            assert_eq!(
                validate(&r).is_empty(),
                ok,
                "{gestational:?} {environment:?}"
            );
        }
    }

    #[test]
    fn relatives_of_one_kind_are_told_apart_by_number() {
        for (member, ok) in [
            (0, false),
            (1, true),
            (MEMBER_LIMIT, true),
            (MEMBER_LIMIT + 1, false),
        ] {
            let mut r = sibling();
            r.member = Some(member);
            assert_eq!(validate(&r).is_empty(), ok, "{member}");
        }
    }

    #[test]
    fn an_age_belongs_to_a_condition_the_relative_had() {
        let mut r = sibling();
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
        let mut r = sibling();
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

    #[test]
    fn a_relative_who_shares_no_genes_is_never_counted_as_inherited() {
        assert_eq!(summarise([]), Summary::NotRecorded);
        let mut mistaken = sibling();
        mistaken.status = Status::EnteredInError;
        assert_eq!(summarise([&mistaken]), Summary::NotRecorded);
        let all = [
            sibling(),
            rec(GeneticLink::Unrelated, None, None, Environment::Childhood),
            rec(GeneticLink::Unknown, None, None, Environment::Unknown),
            mistaken,
        ];
        assert_eq!(
            summarise(&all),
            Summary::Recorded {
                inherited: 1,
                not_inherited: 1,
                link_unknown: 1
            }
        );
    }
}
