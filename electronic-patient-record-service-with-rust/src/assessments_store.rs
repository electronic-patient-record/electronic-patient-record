//! Completed assessments (WL-3): the append-only store behind [`crate::assessments`]. Reads follow a merge
//! by provenance, as for results. The routes are in `controllers/assessments.rs`.

use crate::assessments::{self, AssessmentRecord, Form};
use crate::ids;
use crate::links;
use crate::patients_store::{self, StoreError};
use crate::pool::Db;
use serde::Serialize;
use sqlx::postgres::PgRow;
use sqlx::{AssertSqlSafe, Row};

crate::store_errors!("assessments");

/// The form version an assessment was made under, as a reader needs it beside the answers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FormShown {
    /// The form.
    pub id: String,
    /// The version the answers were given under.
    pub version: i32,
    /// Its title.
    pub title: String,
    /// Whether a named clinician approved that version; `false` means a draft.
    pub approved: bool,
    /// The approver, when there is one.
    pub approved_by: Option<String>,
    /// The day, when there is one.
    pub approved_on: Option<String>,
    /// The newest version of the form, so a reader sees that a newer one exists.
    pub latest_version: i32,
}

/// One answer, with the words of the question as that form version asked it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AnswerShown {
    /// The question.
    pub question: String,
    /// What was asked, in that version.
    pub text: String,
    /// The answer, or null.
    pub value: Option<serde_json::Value>,
    /// Why there is no answer.
    pub absent_reason: Option<String>,
}

/// A completed assessment as a reader sees it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StoredAssessment {
    /// Server id.
    pub id: String,
    /// The patient.
    pub patient_id: String,
    /// The record's version (1 here: corrections are not built).
    pub version: i32,
    /// Who recorded it.
    pub author: String,
    /// When (UTC).
    pub recorded_at: String,
    /// The form version it was made under.
    pub form: FormShown,
    /// The answers, in that version's question order.
    pub answers: Vec<AnswerShown>,
}

const COLUMNS: &str = "assessment_id::text AS id, patient_id::text AS patient_id, version, author,
    form_id, form_version,
    to_char(recorded_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"') AS at, body";

fn shown(form: &Form, record: &AssessmentRecord) -> (FormShown, Vec<AnswerShown>) {
    let non_empty = |s: &str| Some(s.trim()).filter(|s| !s.is_empty()).map(str::to_string);
    let head = FormShown {
        id: form.id.clone(),
        version: form.version,
        title: form.title.clone(),
        approved: form.approved(),
        approved_by: non_empty(&form.approved_by),
        approved_on: non_empty(&form.approved_on),
        latest_version: assessments::latest(&form.id).unwrap_or(form.version),
    };
    let answers = form
        .questions
        .iter()
        .map(|q| {
            let a = record.answers.iter().find(|a| a.question == q.id);
            AnswerShown {
                question: q.id.clone(),
                text: q.text.clone(),
                value: a.and_then(|a| a.value.clone()).filter(|v| !v.is_null()),
                absent_reason: a.and_then(|a| a.absent_reason.clone()),
            }
        })
        .collect();
    (head, answers)
}

fn row_to_stored(row: &PgRow) -> Result<StoredAssessment, StoreError> {
    let body: serde_json::Value = row.try_get("body").map_err(db_err)?;
    let record: AssessmentRecord = serde_json::from_value(body).map_err(db_err)?;
    let form_id: String = row.try_get("form_id").map_err(db_err)?;
    let form_version: i32 = row.try_get("form_version").map_err(db_err)?;
    // A stored assessment names a form version the build no longer has only if a version was removed,
    // which the catalogue's rules forbid; it is an error, not a silent blank.
    let form = assessments::form(&form_id, form_version).ok_or_else(|| {
        StoreError::Db(format!(
            "assessment form {form_id} v{form_version} is missing"
        ))
    })?;
    let (form, answers) = shown(form, &record);
    Ok(StoredAssessment {
        id: row.try_get("id").map_err(db_err)?,
        patient_id: row.try_get("patient_id").map_err(db_err)?,
        version: row.try_get("version").map_err(db_err)?,
        author: row.try_get("author").map_err(db_err)?,
        recorded_at: row.try_get("at").map_err(db_err)?,
        form,
        answers,
    })
}

/// Record a completed assessment at version 1.
///
/// # Errors
///
/// [`StoreError::NotFound`] (no such patient), [`StoreError::BadQuery`] (the answers were refused, all
/// problems joined).
pub async fn record(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    author: &str,
    record: &AssessmentRecord,
) -> Result<StoredAssessment, StoreError> {
    let problems = assessments::problems(record);
    if !problems.is_empty() {
        return Err(StoreError::BadQuery(problems.join("; ")));
    }
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let patient = patients_store::current(&mut *db, patient_id).await?;
    let row = sqlx::query(AssertSqlSafe(format!(
        "INSERT INTO assessment_versions (assessment_id, version, patient_id, author, form_id, form_version, body)
         VALUES (gen_random_uuid(), 1, $1::uuid, $2, $3, $4, $5)
         RETURNING {COLUMNS}"
    )))
    .bind(patient.id)
    .bind(author.to_string())
    .bind(record.form.clone())
    .bind(record.form_version)
    .bind(serde_json::to_value(record).map_err(db_err)?)
    .fetch_optional(&mut *db)
    .await
    .map_err(db_err)?
    .ok_or_else(|| StoreError::Db("insert returned no row".into()))?;
    row_to_stored(&row)
}

/// Every assessment for a patient (and records merged into it), newest first.
///
/// # Errors
///
/// [`StoreError::NotFound`] (no such patient).
pub async fn list(
    db: impl Into<Db<'_>>,
    patient_id: &str,
) -> Result<Vec<StoredAssessment>, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let patient = patients_store::current(&mut *db, patient_id).await?;
    let linked = ids::uuids(&links::linked_ids(&mut *db, &patient.id).await?);
    let rows = sqlx::query(AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM assessment_current
         WHERE patient_id = ANY($1::uuid[])
         ORDER BY recorded_at DESC, assessment_id"
    )))
    .bind(linked)
    .fetch_all(&mut *db)
    .await
    .map_err(db_err)?;
    rows.iter().map(row_to_stored).collect()
}
