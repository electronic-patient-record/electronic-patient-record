//! A suspicion on a referral (RS-1, `spec/referral-suspicion/index.md`): the rules, with no database.
//!
//! A referral is usually sent before anything is known, and what sets its urgency is what the referrer
//! suspects. In Wales the Single Cancer Pathway counts from the **point of suspicion**, so that moment is
//! recorded data, not inferred from when the referral arrived. A suspicion holds the condition (the
//! referrer's words, and a code where one is given), the point of suspicion, who or what raised it, the
//! grounds (a criterion met, a result, or a clinical judgement), any instruction the receiving team needs
//! (Parkinson's: refer untreated), and whether the patient has been told.
//!
//! Each suspicion is made under a **pathway** version (`src/pathways.rs`, RS-6): the pathway sets the clock
//! start, the target days and the priority, and may need a band, factors or the person's agreement, which
//! the suspicion then carries.
//!
//! It is append-only: the first version is the suspicion as raised; every later version is an amendment
//! with a reason, because a clock may start from the point of suspicion. An amendment that changes
//! nothing is refused. It ends only with an [`Outcome`] (RS-3): confirmed with the diagnosis, ruled out
//! with who and on what, or withdrawn with a reason; never by the referral closing or a missed appointment.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// The longest condition, code, display or "by".
pub const SHORT: usize = 200;
/// The longest grounds detail, instruction or amendment reason.
pub const LONG: usize = 500;
/// How far ahead of the server's clock a point of suspicion may be, for clock skew.
pub const SKEW_SECONDS: i64 = 300;

/// Who or what raised the suspicion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Source {
    /// A clinician.
    Clinician,
    /// A screening programme.
    ScreeningProgramme,
    /// A test result.
    Result,
}

/// What the suspicion rests on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GroundsKind {
    /// A named criterion was met (for example a NICE NG12 recommendation).
    Criterion,
    /// A result: the result's id is required.
    Result,
    /// A clinical judgement.
    Judgement,
}

/// The grounds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Grounds {
    /// Which kind.
    pub kind: GroundsKind,
    /// The criterion named, the result as the referrer reads it, or the judgement in words.
    pub detail: String,
    /// For `result`: the result (`/api/patients/{id}/results`) it came from, on this patient's record.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result_id: Option<String>,
}

/// Whether the patient has been told what is suspected. A message names the condition only when told
/// (RS-4, not built).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PatientTold {
    /// The suspected condition was discussed with the patient.
    Told,
    /// It was not, or not yet.
    NotTold,
}

/// A code for the suspected condition. The system is the writer's to name; nothing is looked up.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Code {
    /// The code system.
    pub system: String,
    /// The code.
    pub code: String,
    /// Its display, if given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
}

/// A suspicion as sent: the whole of it, every time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Suspicion {
    /// The suspected condition, in the referrer's words (always kept, even with a code).
    pub condition: String,
    /// A code, where one is given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<Code>,
    /// The point of suspicion, RFC 3339.
    pub point_of_suspicion: String,
    /// Who or what raised it.
    pub source: Source,
    /// The clinician, programme or test, named.
    pub by: String,
    /// What it rests on.
    pub grounds: Grounds,
    /// Anything the receiving team must see (for example "refer untreated").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instruction: Option<String>,
    /// Whether the patient has been told.
    pub patient_told: PatientTold,
    /// The pathway it is made under.
    pub pathway: String,
    /// That pathway's version.
    pub pathway_version: i32,
    /// The band chosen, for a pathway with bands.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub band: Option<String>,
    /// The pathway's factors that apply.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub factors: Vec<String>,
    /// The person's agreement to the referral, for a pathway that needs it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agreed: Option<bool>,
}

impl Suspicion {
    /// What the writer chose within the pathway.
    #[must_use]
    pub fn choice(&self) -> crate::pathways::Choice<'_> {
        crate::pathways::Choice {
            band: self.band.as_deref(),
            factors: &self.factors,
            grounds_is_result: self.grounds.kind == GroundsKind::Result,
            agreed: self.agreed,
        }
    }
}

/// The fields a writer can change, in the order they are reported.
pub const FIELDS: [&str; 13] = [
    "condition",
    "code",
    "point_of_suspicion",
    "source",
    "by",
    "grounds",
    "instruction",
    "patient_told",
    "pathway",
    "pathway_version",
    "band",
    "factors",
    "agreed",
];

fn text(out: &mut Vec<String>, what: &str, value: &str, max: usize) {
    if value.trim().is_empty() {
        out.push(format!("the {what} must not be blank"));
    } else if value.chars().count() > max {
        out.push(format!("the {what} is too long"));
    }
}

/// Every problem with a suspicion, as words that name the field and never quote it.
#[must_use]
pub fn problems(s: &Suspicion, now: DateTime<Utc>) -> Vec<String> {
    let mut out = Vec::new();
    text(&mut out, "condition", &s.condition, SHORT);
    text(&mut out, "by", &s.by, SHORT);
    text(&mut out, "grounds detail", &s.grounds.detail, LONG);
    if let Some(c) = &s.code {
        text(&mut out, "code system", &c.system, SHORT);
        text(&mut out, "code", &c.code, SHORT);
        if let Some(d) = &c.display {
            text(&mut out, "code display", d, SHORT);
        }
    }
    if let Some(i) = &s.instruction {
        text(&mut out, "instruction", i, LONG);
    }
    match DateTime::parse_from_rfc3339(&s.point_of_suspicion) {
        Err(_) => out.push("the point of suspicion must be an RFC 3339 time".into()),
        Ok(t) if t.with_timezone(&Utc) > now + chrono::Duration::seconds(SKEW_SECONDS) => {
            out.push("the point of suspicion is in the future".into());
        }
        Ok(_) => {}
    }
    match (s.grounds.kind, &s.grounds.result_id) {
        (GroundsKind::Result, None) => {
            out.push("grounds of kind result need the result_id".into());
        }
        (GroundsKind::Result, Some(id)) if crate::ids::uuid(id).is_none() => {
            out.push("the grounds result_id is not a result id".into());
        }
        (GroundsKind::Criterion | GroundsKind::Judgement, Some(_)) => {
            out.push("only grounds of kind result carry a result_id".into());
        }
        _ => {}
    }
    match crate::pathways::pathway(&s.pathway, s.pathway_version) {
        None => out.push(format!(
            "there is no pathway {} version {}",
            s.pathway, s.pathway_version
        )),
        Some(p) => out.extend(crate::pathways::problems(p, &s.choice())),
    }
    out
}

/// The fields that differ between two versions, in [`FIELDS`] order.
#[must_use]
pub fn changed(was: &Suspicion, now: &Suspicion) -> Vec<&'static str> {
    let (a, b) = (
        serde_json::to_value(was).unwrap_or_default(),
        serde_json::to_value(now).unwrap_or_default(),
    );
    FIELDS
        .into_iter()
        .filter(|f| a.get(f) != b.get(f))
        .collect()
}

/// Why an amendment is refused, if it is: no reason, a reason too long, or nothing changed.
#[must_use]
pub fn amendment_refusal(was: &Suspicion, now: &Suspicion, reason: Option<&str>) -> Option<String> {
    let reason = reason.map(str::trim).unwrap_or_default();
    if reason.is_empty() {
        return Some("an amendment needs a reason".into());
    }
    if reason.chars().count() > LONG {
        return Some("the reason is too long".into());
    }
    if changed(was, now).is_empty() {
        return Some("the amendment changes nothing".into());
    }
    None
}

/// How a suspicion ends (RS-3). Nothing else ends it: not the referral closing, being cancelled or
/// redirected, and never a missed test or appointment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OutcomeKind {
    /// The suspected condition is confirmed: the diagnosis is required.
    Confirmed,
    /// It is ruled out: who ruled it out, and on what, are required.
    RuledOut,
    /// A clinician withdrew it: the reason is required.
    Withdrawn,
}

impl OutcomeKind {
    /// Every kind, in a fixed order.
    pub const ALL: [OutcomeKind; 3] = [
        OutcomeKind::Confirmed,
        OutcomeKind::RuledOut,
        OutcomeKind::Withdrawn,
    ];

    /// The stored word.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            OutcomeKind::Confirmed => "confirmed",
            OutcomeKind::RuledOut => "ruled-out",
            OutcomeKind::Withdrawn => "withdrawn",
        }
    }
}

/// The outcome of a suspicion, as sent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Outcome {
    /// Which outcome.
    pub kind: OutcomeKind,
    /// For `confirmed`: the diagnosis, in words.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnosis: Option<String>,
    /// For `confirmed`: a code for the diagnosis, if given (not looked up).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<Code>,
    /// For `ruled-out`: who ruled it out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,
    /// What it rests on (a test, an examination, a specialist opinion), or why it was withdrawn.
    pub detail: String,
}

/// Every problem with an outcome, as words naming the field and never quoting it.
#[must_use]
pub fn outcome_problems(o: &Outcome) -> Vec<String> {
    let mut out = Vec::new();
    text(&mut out, "outcome detail", &o.detail, LONG);
    match o.kind {
        OutcomeKind::Confirmed => match &o.diagnosis {
            Some(d) => text(&mut out, "diagnosis", d, SHORT),
            None => out.push("a confirmed suspicion needs the diagnosis".into()),
        },
        OutcomeKind::RuledOut | OutcomeKind::Withdrawn => {
            if o.diagnosis.is_some() || o.code.is_some() {
                out.push("only a confirmed suspicion carries a diagnosis".into());
            }
        }
    }
    match (o.kind, &o.by) {
        (OutcomeKind::RuledOut, Some(by)) => text(&mut out, "outcome by", by, SHORT),
        (OutcomeKind::RuledOut, None) => {
            out.push("a ruled-out suspicion needs who ruled it out".into());
        }
        (_, Some(by)) => text(&mut out, "outcome by", by, SHORT),
        (_, None) => {}
    }
    if let Some(c) = &o.code {
        text(&mut out, "diagnosis code system", &c.system, SHORT);
        text(&mut out, "diagnosis code", &c.code, SHORT);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-11-09T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    fn lower_gi() -> Suspicion {
        Suspicion {
            condition: "Suspected lower gastrointestinal cancer".into(),
            code: None,
            point_of_suspicion: "2026-11-09T09:30:00Z".into(),
            source: Source::Clinician,
            by: "Dr A, general practice".into(),
            grounds: Grounds {
                kind: GroundsKind::Criterion,
                detail: "NG12: unexplained rectal bleeding, age 50 or over".into(),
                result_id: None,
            },
            instruction: None,
            patient_told: PatientTold::Told,
            pathway: "suspected-cancer-wales".into(),
            pathway_version: 1,
            band: None,
            factors: Vec::new(),
            agreed: None,
        }
    }

    #[test]
    fn a_complete_suspicion_has_no_problems() {
        assert!(problems(&lower_gi(), now()).is_empty());
    }

    #[test]
    fn each_field_rule_is_named_without_quoting_the_value() {
        let mut s = lower_gi();
        s.condition = " ".into();
        s.point_of_suspicion = "yesterday".into();
        s.grounds.kind = GroundsKind::Result;
        s.pathway_version = 9;
        let p = problems(&s, now());
        assert_eq!(
            p,
            vec![
                "the condition must not be blank",
                "the point of suspicion must be an RFC 3339 time",
                "grounds of kind result need the result_id",
                "there is no pathway suspected-cancer-wales version 9",
            ]
        );
        let mut s = lower_gi();
        s.agreed = Some(true);
        assert_eq!(
            problems(&s, now()),
            vec!["pathway suspected-cancer-wales does not record agreement"],
            "the pathway's own rules apply"
        );
        let mut s = lower_gi();
        s.point_of_suspicion = "2026-11-10T09:00:00Z".into();
        s.grounds.result_id = Some("0c4f1e2a-0000-4000-8000-000000000001".into());
        assert_eq!(
            problems(&s, now()),
            vec![
                "the point of suspicion is in the future",
                "only grounds of kind result carry a result_id",
            ]
        );
        s.point_of_suspicion = "2026-11-09T12:04:00Z".into();
        s.grounds.kind = GroundsKind::Result;
        assert!(problems(&s, now()).is_empty(), "within the skew");
    }

    #[test]
    fn each_outcome_carries_what_it_needs() {
        let base = Outcome {
            kind: OutcomeKind::Confirmed,
            diagnosis: None,
            code: None,
            by: None,
            detail: "colonoscopy and biopsy".into(),
        };
        assert_eq!(
            outcome_problems(&base),
            vec!["a confirmed suspicion needs the diagnosis"]
        );
        let confirmed = Outcome {
            diagnosis: Some("Adenocarcinoma of the sigmoid colon".into()),
            ..base.clone()
        };
        assert!(outcome_problems(&confirmed).is_empty());
        let ruled_out = Outcome {
            kind: OutcomeKind::RuledOut,
            ..confirmed.clone()
        };
        assert_eq!(
            outcome_problems(&ruled_out),
            vec![
                "only a confirmed suspicion carries a diagnosis",
                "a ruled-out suspicion needs who ruled it out",
            ]
        );
        let withdrawn = Outcome {
            kind: OutcomeKind::Withdrawn,
            detail: " ".into(),
            ..base
        };
        assert_eq!(
            outcome_problems(&withdrawn),
            vec!["the outcome detail must not be blank"]
        );
        let words: Vec<&str> = OutcomeKind::ALL.iter().map(|k| k.word()).collect();
        assert_eq!(words, ["confirmed", "ruled-out", "withdrawn"]);
        for k in OutcomeKind::ALL {
            assert_eq!(serde_json::to_value(k).unwrap(), k.word());
        }
    }

    #[test]
    fn an_amendment_needs_a_reason_and_a_change() {
        let was = lower_gi();
        let mut now_ = was.clone();
        assert_eq!(
            amendment_refusal(&was, &now_, Some("typo")).as_deref(),
            Some("the amendment changes nothing")
        );
        now_.point_of_suspicion = "2026-11-08T16:00:00Z".into();
        now_.patient_told = PatientTold::NotTold;
        assert_eq!(
            amendment_refusal(&was, &now_, Some("  ")).as_deref(),
            Some("an amendment needs a reason")
        );
        assert_eq!(
            amendment_refusal(&was, &now_, Some("first seen the day before")),
            None
        );
        assert_eq!(
            changed(&was, &now_),
            vec!["point_of_suspicion", "patient_told"]
        );
    }
}
