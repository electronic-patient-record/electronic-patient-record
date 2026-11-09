//! Assessment forms as versioned standards (WL-3; the form catalogue for EM-3): the catalogue and the rules,
//! with no database.
//!
//! The Welsh Nursing Care Record publishes each nursing form as a versioned national data standard with
//! a named clinical sign-off (`spec/nhs-wales/index.md`). Here a form version is data in
//! `src/assessment_forms.toml`, read into the build; a completed assessment keeps the form id and version
//! it was made under, and is always shown with that version's questions, so a later version never
//! rewrites what was asked or answered. A form with no approver is a draft, and says so.
//!
//! Every question is answered, or marked absent with a reason: absence is never a "no".

use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

/// The catalogue, as built in.
pub const SOURCE: &str = include_str!("assessment_forms.toml");

/// The longest absent reason kept.
pub const REASON_LIMIT: usize = 200;

/// How a question is answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    /// `true` or `false`.
    YesNo,
    /// One of the question's choices.
    Choice,
}

/// One question of one form version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Question {
    /// Stable within the form, across versions where the question is the same.
    pub id: String,
    /// The words asked.
    pub text: String,
    /// How it is answered.
    pub kind: Kind,
    /// For a choice: the allowed answers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<String>,
}

/// One version of one form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Form {
    /// The form.
    pub id: String,
    /// Its version, from 1.
    pub version: i32,
    /// Its title.
    pub title: String,
    /// The clinical approver; empty when not approved.
    pub approved_by: String,
    /// The day of approval, `YYYY-MM-DD`; empty when not approved.
    pub approved_on: String,
    /// The questions, in order.
    #[serde(rename = "question")]
    pub questions: Vec<Question>,
}

impl Form {
    /// Whether a named clinician approved this version on a stated day.
    #[must_use]
    pub fn approved(&self) -> bool {
        !self.approved_by.trim().is_empty() && !self.approved_on.trim().is_empty()
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    form: Vec<Form>,
}

/// Parse a catalogue.
///
/// # Errors
///
/// A message naming the problem: not TOML of this shape; a version below 1, repeated, or not following
/// the one before; a form with no questions; a blank or repeated question id; a choice question with no
/// choices or a yes-no with some; an approver without a day or a day that is not a date.
pub fn parse(text: &str) -> Result<Vec<Form>, String> {
    let file: File = toml::from_str(text).map_err(|e| format!("assessment forms: {e}"))?;
    let mut forms = file.form;
    forms.sort_by(|a, b| (a.id.as_str(), a.version).cmp(&(b.id.as_str(), b.version)));
    for (i, f) in forms.iter().enumerate() {
        let name = format!("{} v{}", f.id, f.version);
        let previous = i.checked_sub(1).map(|j| &forms[j]).filter(|p| p.id == f.id);
        let expected = previous.map_or(1, |p| p.version + 1);
        if f.version != expected {
            return Err(format!(
                "assessment forms: {name} should be version {expected}"
            ));
        }
        if f.id.trim().is_empty() || f.title.trim().is_empty() {
            return Err(format!("assessment forms: {name} needs an id and a title"));
        }
        if f.questions.is_empty() {
            return Err(format!("assessment forms: {name} has no questions"));
        }
        let mut seen = std::collections::BTreeSet::new();
        for q in &f.questions {
            if q.id.trim().is_empty() || !seen.insert(q.id.as_str()) {
                return Err(format!(
                    "assessment forms: {name} has a blank or repeated question id"
                ));
            }
            match (q.kind, q.choices.is_empty()) {
                (Kind::Choice, true) => {
                    return Err(format!("assessment forms: {name} {} has no choices", q.id));
                }
                (Kind::YesNo, false) => {
                    return Err(format!(
                        "assessment forms: {name} {} is yes-no with choices",
                        q.id
                    ));
                }
                _ => {}
            }
        }
        let by = f.approved_by.trim().is_empty();
        let on = f.approved_on.trim();
        if by != on.is_empty()
            || (!on.is_empty() && chrono::NaiveDate::parse_from_str(on, "%Y-%m-%d").is_err())
        {
            return Err(format!(
                "assessment forms: {name} needs both an approver and a YYYY-MM-DD day, or neither"
            ));
        }
    }
    Ok(forms)
}

static FORMS: LazyLock<Vec<Form>> = LazyLock::new(|| {
    // The built-in catalogue is proved to parse by the tests below.
    parse(SOURCE).unwrap_or_default()
});

/// Every form version, by id then version.
#[must_use]
pub fn catalogue() -> &'static [Form] {
    &FORMS
}

/// One form version.
#[must_use]
pub fn form(id: &str, version: i32) -> Option<&'static Form> {
    FORMS.iter().find(|f| f.id == id && f.version == version)
}

/// The newest version of a form.
#[must_use]
pub fn latest(id: &str) -> Option<i32> {
    FORMS.iter().filter(|f| f.id == id).map(|f| f.version).max()
}

/// One answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Answer {
    /// The question.
    pub question: String,
    /// The answer: a boolean for yes-no, a string for a choice; null with a reason when not answered.
    #[serde(default)]
    pub value: Option<serde_json::Value>,
    /// Why there is no answer (required when `value` is null).
    #[serde(default)]
    pub absent_reason: Option<String>,
}

/// A completed assessment as sent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssessmentRecord {
    /// The form.
    pub form: String,
    /// The form version it was made under.
    pub form_version: i32,
    /// One answer per question, in any order.
    pub answers: Vec<Answer>,
}

/// Every problem with a record against its form version, as words.
#[must_use]
pub fn problems(r: &AssessmentRecord) -> Vec<String> {
    let Some(form) = form(&r.form, r.form_version) else {
        return vec![format!(
            "there is no form {} version {}",
            r.form, r.form_version
        )];
    };
    let mut out = Vec::new();
    for a in &r.answers {
        if !form.questions.iter().any(|q| q.id == a.question) {
            out.push(format!(
                "{} is not a question of this form version",
                a.question
            ));
        }
    }
    for q in &form.questions {
        let given: Vec<&Answer> = r.answers.iter().filter(|a| a.question == q.id).collect();
        let [a] = given.as_slice() else {
            out.push(format!("answer {} once", q.id));
            continue;
        };
        match (&a.value, a.absent_reason.as_deref().map(str::trim)) {
            (None | Some(serde_json::Value::Null), Some(reason)) if !reason.is_empty() => {
                if reason.chars().count() > REASON_LIMIT {
                    out.push(format!("the reason for {} is too long", q.id));
                }
            }
            (None | Some(serde_json::Value::Null), _) => {
                out.push(format!("{} has no answer: give one, or say why not", q.id));
            }
            (Some(_), Some(_)) => out.push(format!("{} has an answer and a reason", q.id)),
            (Some(v), None) => {
                let ok = match q.kind {
                    Kind::YesNo => v.is_boolean(),
                    Kind::Choice => v.as_str().is_some_and(|s| q.choices.iter().any(|c| c == s)),
                };
                if !ok {
                    out.push(format!(
                        "{} has an answer this question does not take",
                        q.id
                    ));
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn answer(q: &str, v: serde_json::Value) -> Answer {
        Answer {
            question: q.into(),
            value: Some(v),
            absent_reason: None,
        }
    }

    #[test]
    fn the_built_in_catalogue_parses_with_two_versions_of_one_form_both_drafts() {
        let forms = parse(SOURCE).expect("src/assessment_forms.toml must parse");
        assert_eq!(latest("skin-check"), Some(2));
        assert_eq!(form("skin-check", 1).unwrap().questions.len(), 2);
        assert_eq!(form("skin-check", 2).unwrap().questions.len(), 3);
        assert!(
            forms.iter().all(|f| !f.approved()),
            "nobody has approved them"
        );
    }

    #[test]
    fn each_question_is_answered_once_or_absent_with_a_reason() {
        let mut r = AssessmentRecord {
            form: "skin-check".into(),
            form_version: 1,
            answers: vec![
                answer("intact", json!(true)),
                answer("redness", json!(false)),
            ],
        };
        assert!(problems(&r).is_empty(), "{:?}", problems(&r));
        r.answers[1] = Answer {
            question: "redness".into(),
            value: None,
            absent_reason: Some("declined to be examined".into()),
        };
        assert!(problems(&r).is_empty());
        r.answers[1].absent_reason = None;
        assert_eq!(
            problems(&r),
            vec!["redness has no answer: give one, or say why not"]
        );
        r.answers[1] = answer("redness", json!("sometimes"));
        assert_eq!(
            problems(&r),
            vec!["redness has an answer this question does not take"]
        );
        r.answers.push(answer("where", json!("heels")));
        assert!(
            problems(&r)
                .iter()
                .any(|p| p.contains("where is not a question"))
        );
        r.form_version = 9;
        assert_eq!(problems(&r), vec!["there is no form skin-check version 9"]);
    }

    #[test]
    fn a_bad_catalogue_is_refused() {
        let q = "[[form.question]]\nid = \"a\"\ntext = \"t\"\nkind = \"yes-no\"\n";
        let f = |v: i32, by: &str, on: &str| {
            format!(
                "[[form]]\nid = \"x\"\nversion = {v}\ntitle = \"T\"\napproved_by = \"{by}\"\napproved_on = \"{on}\"\n{q}"
            )
        };
        for (text, words) in [
            (f(2, "", ""), "should be version 1"),
            (f(1, "", "") + &f(3, "", ""), "should be version 2"),
            (f(1, "Dr A", ""), "approver"),
            (f(1, "Dr A", "soon"), "approver"),
            (
                "[[form]]\nid = \"x\"\nversion = 1\ntitle = \"T\"\napproved_by = \"\"\napproved_on = \"\"\nquestion = []\n"
                    .to_string(),
                "no questions",
            ),
        ] {
            let err = parse(&text).expect_err(&text);
            assert!(err.contains(words), "{text}: {err}");
        }
        assert!(parse(&f(1, "Dr A", "2026-11-09")).unwrap()[0].approved());
    }
}
