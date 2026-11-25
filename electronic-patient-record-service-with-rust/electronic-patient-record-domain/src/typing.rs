//! Compatibility typing (DM-1, `spec/donor-matching/index.md`): what a laboratory reported that bears on whether blood,
//! a kidney or bone marrow could be given or received. The rules, with no database.
//!
//! **Nothing here matches.** It keeps reports. Whether two people are compatible is decided by a transfusion or
//! histocompatibility laboratory on its own samples and by a transplant team, never from this list: a report typed on the
//! wrong patient, or on a sample taken from the wrong patient, looks exactly like a right one.
//!
//! So every report names its laboratory, its sample and the time the sample was taken, and the list never says what a
//! person's group *is*. It says whether reports from **separately collected** samples agree, because two tubes filled at
//! one identity check repeat one mistake (the two-sample rule; sources in the specification). Agreement is not
//! permission, and the group can change after a stem cell transplant.
//!
//! An HLA type can show that two people are not related as they believe (H-067), and nothing here compares two people.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// The longest laboratory name, sample number, release name or specificity kept.
pub const TEXT_LIMIT: usize = 80;
/// The most antibody specificities kept in one report.
pub const SPECIFICITY_LIMIT: usize = 100;

/// An ABO group, as reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Abo {
    /// Group O.
    O,
    /// Group A.
    A,
    /// Group B.
    B,
    /// Group AB.
    Ab,
}

impl Abo {
    /// Every group.
    pub const ALL: [Abo; 4] = [Abo::O, Abo::A, Abo::B, Abo::Ab];
}

/// An `RhD` type, as reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RhD {
    /// Positive.
    Positive,
    /// Negative.
    Negative,
    /// Weak, partial or otherwise variant: neither of the others.
    Variant,
}

impl RhD {
    /// Every type.
    pub const ALL: [RhD; 3] = [RhD::Positive, RhD::Negative, RhD::Variant];
}

/// An HLA gene.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Locus {
    /// HLA-A.
    A,
    /// HLA-B.
    B,
    /// HLA-C.
    C,
    /// HLA-DRB1.
    DRB1,
    /// HLA-DRB3.
    DRB3,
    /// HLA-DRB4.
    DRB4,
    /// HLA-DRB5.
    DRB5,
    /// HLA-DQA1.
    DQA1,
    /// HLA-DQB1.
    DQB1,
    /// HLA-DPA1.
    DPA1,
    /// HLA-DPB1.
    DPB1,
}

impl Locus {
    /// Every locus.
    pub const ALL: [Locus; 11] = [
        Locus::A,
        Locus::B,
        Locus::C,
        Locus::DRB1,
        Locus::DRB3,
        Locus::DRB4,
        Locus::DRB5,
        Locus::DQA1,
        Locus::DQB1,
        Locus::DPA1,
        Locus::DPB1,
    ];
}

/// Whether an antibody was found. *Not detected* is of one sample on one day: antibodies come and go.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Detection {
    /// Found.
    Detected,
    /// Looked for and not found.
    NotDetected,
    /// The laboratory could not say.
    Indeterminate,
}

impl Detection {
    /// Every answer.
    pub const ALL: [Detection; 3] = [
        Detection::Detected,
        Detection::NotDetected,
        Detection::Indeterminate,
    ];
}

/// What was reported.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "kebab-case",
    deny_unknown_fields
)]
pub enum Finding {
    /// An ABO group.
    Abo {
        /// The group.
        group: Abo,
    },
    /// An `RhD` type.
    Rhd {
        /// The type.
        rhd: RhD,
    },
    /// HLA alleles at one locus.
    Hla {
        /// The gene.
        locus: Locus,
        /// One or two alleles in the laboratory's own resolution (`02:01`). **One is one seen**: never read as two the
        /// same, because a method can miss the other.
        alleles: Vec<String>,
        /// The release of the HLA database the names are from: a name means nothing without it.
        release: String,
    },
    /// Red cell antibodies.
    RedCellAntibody {
        /// Found or not.
        result: Detection,
        /// Which, as reported (`anti-K`). Only when found.
        #[serde(default)]
        specificities: Vec<String>,
    },
    /// HLA antibodies.
    HlaAntibody {
        /// Found or not.
        result: Detection,
        /// Which, as reported. Only when found.
        #[serde(default)]
        specificities: Vec<String>,
    },
    /// Cytomegalovirus antibody (`IgG`).
    CmvIgg {
        /// Found or not.
        result: Detection,
    },
}

/// Whether the report stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    /// It stands.
    Reported,
    /// Recorded by mistake. Final: it cannot be reopened.
    EnteredInError,
}

impl Status {
    /// Every status (the stored words are checked against the database `CHECK`, PR-3).
    pub const ALL: [Status; 2] = [Status::Reported, Status::EnteredInError];

    /// The stored word.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Reported => "reported",
            Status::EnteredInError => "entered-in-error",
        }
    }

    /// The status a stored word names, or `None` for one the service does not write.
    #[must_use]
    pub fn parse(s: &str) -> Option<Status> {
        Status::ALL.into_iter().find(|v| v.as_str() == s)
    }
}

/// One report. It has no place for another person, so no donor or recipient can be named in it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypingRecord {
    /// What was reported.
    pub finding: Finding,
    /// The laboratory that reported it.
    pub laboratory: String,
    /// The laboratory's number for the sample.
    pub sample: String,
    /// When the sample was taken (RFC 3339), not when it was reported.
    pub collected_at: String,
    /// Whether the report stands.
    pub status: Status,
}

fn line(out: &mut Vec<String>, what: &str, value: &str) {
    if value.trim().is_empty() {
        out.push(format!("the {what} must not be blank"));
    } else if value.chars().count() > TEXT_LIMIT || value.chars().any(char::is_control) {
        out.push(format!(
            "the {what} must be up to {TEXT_LIMIT} characters, without control characters"
        ));
    }
}

/// Whether `a` has the shape of an allele's fields: one to four groups of two to four digits (`02`, `02:01`), then at most
/// one letter that says how it is expressed (`N`, `L`, `S`, `C`, `A`, `Q`) or that it names a group (`G`, `P`). The gene is
/// not repeated, and a list of possible alleles is not one. Nothing is looked up.
#[must_use]
pub fn allele_shape(a: &str) -> bool {
    let fields = a.strip_suffix(['N', 'L', 'S', 'C', 'A', 'Q', 'G', 'P']);
    let parts: Vec<&str> = fields.map_or(a, |f| f).split(':').collect();
    (1..=4).contains(&parts.len())
        && parts
            .iter()
            .all(|p| (2..=4).contains(&p.len()) && p.bytes().all(|b| b.is_ascii_digit()))
}

/// What is wrong with the antibodies of a report.
fn antibodies(out: &mut Vec<String>, result: Detection, which: &[String]) {
    if result != Detection::Detected && !which.is_empty() {
        out.push("specificities are only for an antibody that was detected".into());
    }
    if which.len() > SPECIFICITY_LIMIT {
        out.push(format!("at most {SPECIFICITY_LIMIT} specificities"));
    }
    for s in which.iter().take(SPECIFICITY_LIMIT) {
        line(out, "specificity", s);
    }
}

/// What is wrong with a report, in words that name a field and never quote it; empty if nothing.
#[must_use]
pub fn validate(r: &TypingRecord, now: jiff::Timestamp) -> Vec<String> {
    let mut out = Vec::new();
    line(&mut out, "laboratory", &r.laboratory);
    line(&mut out, "sample", &r.sample);
    match crate::clock::parse_rfc3339(&r.collected_at) {
        None => out.push("collected_at must be an RFC 3339 time with an offset or Z".into()),
        Some(at) if at > now => out.push("collected_at is in the future".into()),
        Some(_) => {}
    }
    match &r.finding {
        Finding::Abo { .. } | Finding::Rhd { .. } | Finding::CmvIgg { .. } => {}
        Finding::Hla {
            alleles, release, ..
        } => {
            line(&mut out, "release", release);
            if !(1..=2).contains(&alleles.len()) {
                out.push("alleles must be one or two".into());
            } else if !alleles.iter().all(|a| allele_shape(a)) {
                out.push(
                    "an allele must be its fields, such as 02:01, without the gene and not a list"
                        .into(),
                );
            }
        }
        Finding::RedCellAntibody {
            result,
            specificities,
        }
        | Finding::HlaAntibody {
            result,
            specificities,
        } => antibodies(&mut out, *result, specificities),
    }
    out
}

/// What separately collected samples say of one thing. It never says what the person's group is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Agreement {
    /// No report.
    NotReported,
    /// Reports from one collection only. Not checked by a second.
    OneCollection,
    /// Two or more collections, and every report is the same.
    Agree,
    /// Reports differ. Someone must find out why before anything is done.
    Disagree,
}

/// What the list amounts to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Summary {
    /// `recorded`, or `not-recorded` when nothing stands: unknown.
    pub state: State,
    /// The ABO reports.
    pub abo: Agreement,
    /// The `RhD` reports.
    pub rhd: Agreement,
}

/// Whether anything stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum State {
    /// At least one report.
    Recorded,
    /// None: unknown.
    NotRecorded,
}

/// The agreement of `reports`: what each said, and the instant its sample was taken if that can be read. Two samples are
/// separate only when their times are known and differ, whatever their numbers, so tubes filled at one identity check
/// count as one.
fn agreement<T: Ord + Copy>(reports: &[(T, Option<jiff::Timestamp>)]) -> Agreement {
    let said: BTreeSet<T> = reports.iter().map(|(v, _)| *v).collect();
    let taken: BTreeSet<jiff::Timestamp> = reports.iter().filter_map(|(_, at)| *at).collect();
    match (said.len(), taken.len()) {
        (0, _) => Agreement::NotReported,
        (1, 0 | 1) => Agreement::OneCollection,
        (1, _) => Agreement::Agree,
        _ => Agreement::Disagree,
    }
}

/// The summary of the current versions, those entered in error left out.
#[must_use]
pub fn summarise<'a>(records: impl IntoIterator<Item = &'a TypingRecord>) -> Summary {
    let mut any = false;
    let (mut abo, mut rhd) = (Vec::new(), Vec::new());
    for r in records
        .into_iter()
        .filter(|r| r.status != Status::EnteredInError)
    {
        any = true;
        let at = crate::clock::parse_rfc3339(&r.collected_at);
        match &r.finding {
            Finding::Abo { group } => abo.push((*group, at)),
            Finding::Rhd { rhd: t } => rhd.push((*t, at)),
            _ => {}
        }
    }
    Summary {
        state: if any {
            State::Recorded
        } else {
            State::NotRecorded
        },
        abo: agreement(&abo),
        rhd: agreement(&rhd),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> jiff::Timestamp {
        "2026-11-25T12:00:00Z".parse().unwrap()
    }

    fn rec(finding: Finding, sample: &str, at: &str) -> TypingRecord {
        TypingRecord {
            finding,
            laboratory: "Transfusion laboratory".into(),
            sample: sample.into(),
            collected_at: at.into(),
            status: Status::Reported,
        }
    }

    fn abo(group: Abo, sample: &str, at: &str) -> TypingRecord {
        rec(Finding::Abo { group }, sample, at)
    }

    const T1: &str = "2026-11-20T09:00:00Z";
    const T2: &str = "2026-11-21T09:00:00Z";

    fn hla(alleles: &[&str], release: &str) -> TypingRecord {
        rec(
            Finding::Hla {
                locus: Locus::DRB1,
                alleles: alleles.iter().map(|a| (*a).to_string()).collect(),
                release: release.into(),
            },
            "S1",
            T1,
        )
    }

    #[test]
    fn the_words_round_trip_and_an_unknown_one_is_refused() {
        for s in Status::ALL {
            assert_eq!(Status::parse(s.as_str()), Some(s));
            assert_eq!(serde_json::to_value(s).unwrap(), s.as_str());
        }
        assert_eq!(Status::parse("confirmed"), None);
    }

    #[test]
    fn one_sample_is_never_agreement_and_neither_are_two_tubes_at_one_time() {
        let summary = |all: &[TypingRecord]| summarise(all).abo;
        assert_eq!(summary(&[]), Agreement::NotReported);
        assert_eq!(summary(&[abo(Abo::A, "S1", T1)]), Agreement::OneCollection);
        assert_eq!(
            summary(&[abo(Abo::A, "S1", T1), abo(Abo::A, "S2", T1)]),
            Agreement::OneCollection,
            "two tubes, one identity check"
        );
        // The same instant written with another offset is the same collection.
        assert_eq!(
            summary(&[
                abo(Abo::A, "S1", T1),
                abo(Abo::A, "S2", "2026-11-20T10:00:00+01:00")
            ]),
            Agreement::OneCollection
        );
        assert_eq!(
            summary(&[abo(Abo::A, "S1", T1), abo(Abo::A, "S2", T2)]),
            Agreement::Agree
        );
    }

    #[test]
    fn any_difference_is_disagreement_however_many_agree() {
        let mut all = vec![
            abo(Abo::A, "S1", T1),
            abo(Abo::A, "S2", T2),
            abo(Abo::O, "S3", "2026-11-22T09:00:00Z"),
        ];
        assert_eq!(summarise(&all).abo, Agreement::Disagree);
        // One collection that contradicts itself.
        assert_eq!(
            summarise(&[abo(Abo::A, "S1", T1), abo(Abo::B, "S1", T1)]).abo,
            Agreement::Disagree
        );
        // One entered in error never happened.
        all[2].status = Status::EnteredInError;
        assert_eq!(summarise(&all).abo, Agreement::Agree);
    }

    #[test]
    fn abo_and_rhd_are_judged_apart_and_an_empty_list_is_unknown() {
        let none = summarise([]);
        assert_eq!(none.state, State::NotRecorded);
        let all = [
            abo(Abo::O, "S1", T1),
            abo(Abo::O, "S2", T2),
            rec(Finding::Rhd { rhd: RhD::Positive }, "S1", T1),
            rec(Finding::Rhd { rhd: RhD::Variant }, "S2", T2),
            rec(
                Finding::CmvIgg {
                    result: Detection::NotDetected,
                },
                "S1",
                T1,
            ),
        ];
        let s = summarise(&all);
        assert_eq!(
            (s.state, s.abo, s.rhd),
            (State::Recorded, Agreement::Agree, Agreement::Disagree)
        );
        // A stored time that cannot be read never makes a second collection, beside another or a real one.
        for other in ["later", T1] {
            assert_eq!(
                summarise(&[abo(Abo::O, "S1", "soon"), abo(Abo::O, "S2", other)]).abo,
                Agreement::OneCollection,
                "{other}"
            );
        }
    }

    #[test]
    fn a_report_names_its_laboratory_sample_and_collection() {
        assert!(validate(&abo(Abo::A, "S1", T1), now()).is_empty());
        let one = |edit: fn(&mut TypingRecord), says: &str| {
            let mut r = abo(Abo::A, "S1", T1);
            edit(&mut r);
            let found = validate(&r, now());
            assert_eq!(found.len(), 1, "{found:?}");
            assert!(found[0].contains(says), "{found:?}");
        };
        one(
            |r| r.laboratory = " ".into(),
            "laboratory must not be blank",
        );
        one(
            |r| r.sample = "x".repeat(TEXT_LIMIT + 1),
            "sample must be up to",
        );
        one(
            |r| r.collected_at = "2026-11-20".into(),
            "collected_at must be",
        );
        one(
            |r| r.collected_at = "2026-11-25T12:00:01Z".into(),
            "in the future",
        );
    }

    #[test]
    fn an_allele_is_its_fields_and_not_the_gene_or_a_list() {
        for good in [
            "02",
            "02:01",
            "01:01:01:01",
            "24:09N",
            "01:01:01G",
            "02:01P",
            "15:1234",
        ] {
            assert!(allele_shape(good), "{good}");
        }
        for bad in [
            "",
            "2",
            "A*02:01",
            "02:01/02:07",
            "02:XX",
            "02:",
            ":01",
            "02:01:01:01:01",
            "02:01NN",
            "02:01n",
            "12345",
        ] {
            assert!(!allele_shape(bad), "{bad}");
        }
        assert!(
            validate(&hla(&["15:01"], "3.60.0"), now()).is_empty(),
            "one seen"
        );
        assert!(validate(&hla(&["15:01", "04:01"], "3.60.0"), now()).is_empty());
        assert_eq!(validate(&hla(&[], "3.60.0"), now()).len(), 1);
        assert_eq!(
            validate(&hla(&["15:01", "04:01", "01:01"], "3.60.0"), now()).len(),
            1
        );
        assert_eq!(validate(&hla(&["DRB1*15:01"], "3.60.0"), now()).len(), 1);
        assert_eq!(
            validate(&hla(&["15:01"], " "), now()).len(),
            1,
            "a release is required"
        );
    }

    #[test]
    fn specificities_belong_to_an_antibody_that_was_found() {
        let antibody = |result, specificities: Vec<String>| {
            rec(
                Finding::RedCellAntibody {
                    result,
                    specificities,
                },
                "S1",
                T1,
            )
        };
        let k = || vec!["anti-K".to_string()];
        assert!(validate(&antibody(Detection::Detected, k()), now()).is_empty());
        assert!(validate(&antibody(Detection::Detected, vec![]), now()).is_empty());
        for other in [Detection::NotDetected, Detection::Indeterminate] {
            assert!(validate(&antibody(other, vec![]), now()).is_empty());
            assert_eq!(validate(&antibody(other, k()), now()).len(), 1, "{other:?}");
        }
        assert_eq!(
            validate(&antibody(Detection::Detected, vec![" ".into()]), now()).len(),
            1
        );
        let many = vec!["anti-K".to_string(); SPECIFICITY_LIMIT + 1];
        assert_eq!(
            validate(&antibody(Detection::Detected, many), now()).len(),
            1
        );
    }

    #[test]
    fn a_report_has_no_place_for_another_person_or_a_conclusion() {
        let base = serde_json::to_value(abo(Abo::Ab, "S1", T1)).unwrap();
        assert_eq!(
            base["finding"],
            serde_json::json!({ "kind": "abo", "group": "ab" })
        );
        assert!(serde_json::from_value::<TypingRecord>(base.clone()).is_ok());
        for field in [
            "donor",
            "recipient",
            "patient_id",
            "match",
            "compatible",
            "crossmatch",
            "note",
        ] {
            let mut r = base.clone();
            r[field] = serde_json::json!("x");
            assert!(
                serde_json::from_value::<TypingRecord>(r).is_err(),
                "{field}"
            );
            let mut f = base.clone();
            f["finding"][field] = serde_json::json!("x");
            assert!(
                serde_json::from_value::<TypingRecord>(f).is_err(),
                "finding.{field}"
            );
        }
        for kind in ["crossmatch", "compatible", "universal-donor"] {
            let mut r = base.clone();
            r["finding"]["kind"] = serde_json::json!(kind);
            assert!(serde_json::from_value::<TypingRecord>(r).is_err(), "{kind}");
        }
    }
}
