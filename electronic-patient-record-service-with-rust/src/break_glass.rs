//! Break-the-glass (task SE-3): a treating clinician with no care relationship declares an
//! emergency, gives a reason, and may read the patient's clinical content for [`GRANT_HOURS`] hours.
//!
//! What a grant lifts, and what it does not, is decided by the policy engine (`policy.rs`): it lifts
//! only the missing care relationship, only for reads of clinical content, only for a treating role,
//! and not while it has expired. It does **not** lift a patient's refusal of consent, and it does not
//! unmask a label. Every row the audit trail writes while a request is served under a grant carries the
//! break-glass flag and the grant's id, and every grant waits in a review queue until someone other than
//! the person who opened it records whether it was appropriate.
//!
//! There is no real-time notification to a privacy officer: nothing here can send one. The unreviewed
//! queue and its count are what exists.

use crate::patients_store::{self, StoreError};
use sea_orm::{ConnectionTrait, DbBackend, Statement, Value};
use serde::{Deserialize, Serialize};

/// How long a grant lasts. Fixed: the clinician cannot choose a longer one.
pub const GRANT_HOURS: i64 = 4;

/// The most grants a listing returns.
pub const LIST_LIMIT: u64 = 50;

/// A grant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Grant {
    /// Server id.
    pub id: String,
    /// The patient.
    pub patient_id: String,
    /// The token subject who opened it.
    pub subject: String,
    /// The reason they gave.
    pub reason: String,
    /// When (UTC).
    pub granted_at: String,
    /// When it ends (UTC).
    pub expires_at: String,
    /// Unix seconds at which it ends.
    #[serde(skip)]
    pub expires_unix: i64,
    /// The review, once there is one.
    pub review: Option<Review>,
}

/// Someone else's verdict on a grant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Review {
    /// Who.
    pub reviewed_by: String,
    /// When (UTC).
    pub reviewed_at: String,
    /// `appropriate` or `inappropriate`.
    pub outcome: String,
    /// Why.
    pub note: String,
}

/// The verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Outcome {
    /// The emergency access was justified.
    Appropriate,
    /// It was not.
    Inappropriate,
}

impl Outcome {
    fn as_str(self) -> &'static str {
        match self {
            Outcome::Appropriate => "appropriate",
            Outcome::Inappropriate => "inappropriate",
        }
    }
}

fn pg(sql: &str, values: Vec<Value>) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, sql, values)
}

fn db_err(e: impl std::fmt::Display) -> StoreError {
    StoreError::Db(e.to_string())
}

const TS: &str = "'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"'";

fn select() -> String {
    format!(
        "SELECT g.grant_id::text AS id, g.patient_id::text AS patient_id, g.subject, g.reason,
                to_char(g.granted_at AT TIME ZONE 'UTC', {TS}) AS granted_at,
                to_char(g.expires_at AT TIME ZONE 'UTC', {TS}) AS expires_at,
                floor(extract(epoch FROM g.expires_at))::bigint AS expires_unix,
                r.reviewed_by, r.outcome, r.note,
                to_char(r.reviewed_at AT TIME ZONE 'UTC', {TS}) AS reviewed_at
         FROM break_glass_grants g LEFT JOIN break_glass_reviews r USING (grant_id)"
    )
}

fn to_grant(row: &sea_orm::QueryResult) -> Result<Grant, StoreError> {
    let get = |c: &str| row.try_get::<String>("", c).map_err(db_err);
    let reviewed_by: Option<String> = row.try_get("", "reviewed_by").map_err(db_err)?;
    let review = match reviewed_by {
        Some(reviewed_by) => Some(Review {
            reviewed_by,
            reviewed_at: get("reviewed_at")?,
            outcome: get("outcome")?,
            note: get("note")?,
        }),
        None => None,
    };
    Ok(Grant {
        id: get("id")?,
        patient_id: get("patient_id")?,
        subject: get("subject")?,
        reason: get("reason")?,
        granted_at: get("granted_at")?,
        expires_at: get("expires_at")?,
        expires_unix: row.try_get("", "expires_unix").map_err(db_err)?,
        review,
    })
}

/// The grant `subject` holds on `patient_id` right now, if any (the latest to expire).
///
/// # Errors
///
/// [`StoreError::Db`].
pub async fn active(
    db: &impl ConnectionTrait,
    patient_id: &str,
    subject: &str,
) -> Result<Option<Grant>, StoreError> {
    let row = db
        .query_one_raw(pg(
            &format!(
                "{} WHERE g.patient_id::text = lower($1) AND g.subject = $2 AND g.expires_at > now()
                 ORDER BY g.expires_at DESC LIMIT 1",
                select()
            ),
            vec![patient_id.into(), subject.into()],
        ))
        .await
        .map_err(db_err)?;
    row.as_ref().map(to_grant).transpose()
}

/// Open a grant. A subject that already holds one on this patient gets that one back, unchanged:
/// asking again does not extend it.
///
/// # Errors
///
/// [`StoreError::NotFound`] (no such patient), [`StoreError::BadQuery`] (blank or over-long reason),
/// [`StoreError::Db`].
pub async fn open(
    db: &impl ConnectionTrait,
    patient_id: &str,
    subject: &str,
    reason: &str,
) -> Result<(Grant, bool), StoreError> {
    let reason = reason.trim();
    if reason.is_empty() || reason.chars().count() > 500 || reason.chars().any(char::is_control) {
        return Err(StoreError::BadQuery(
            "a reason of 1 to 500 characters is required".into(),
        ));
    }
    let patient = patients_store::current(db, patient_id).await?;
    if let Some(existing) = active(db, &patient.id, subject).await? {
        return Ok((existing, false));
    }
    let row = db
        .query_one_raw(pg(
            "INSERT INTO break_glass_grants (patient_id, subject, reason, expires_at)
             VALUES ($1::uuid, $2, $3, now() + make_interval(hours => $4::int))
             RETURNING grant_id::text AS id",
            vec![
                patient.id.clone().into(),
                subject.into(),
                reason.into(),
                i32::try_from(GRANT_HOURS).unwrap_or(4).into(),
            ],
        ))
        .await
        .map_err(db_err)?
        .ok_or_else(|| StoreError::Db("insert returned no row".into()))?;
    let id: String = row.try_get("", "id").map_err(db_err)?;
    Ok((get(db, &id).await?, true))
}

/// One grant, reviewed or not.
///
/// # Errors
///
/// [`StoreError::NotFound`] (also for an id that is not a UUID).
pub async fn get(db: &impl ConnectionTrait, id: &str) -> Result<Grant, StoreError> {
    let row = db
        .query_one_raw(pg(
            &format!("{} WHERE g.grant_id::text = lower($1)", select()),
            vec![id.into()],
        ))
        .await
        .map_err(db_err)?
        .ok_or(StoreError::NotFound)?;
    to_grant(&row)
}

/// Grants nobody has reviewed, oldest first.
///
/// # Errors
///
/// [`StoreError::Db`].
pub async fn unreviewed(db: &impl ConnectionTrait) -> Result<Vec<Grant>, StoreError> {
    let rows = db
        .query_all_raw(pg(
            &format!(
                "{} WHERE r.grant_id IS NULL ORDER BY g.granted_at, g.grant_id LIMIT {LIST_LIMIT}",
                select()
            ),
            vec![],
        ))
        .await
        .map_err(db_err)?;
    rows.iter().map(to_grant).collect()
}

/// How many grants nobody has reviewed.
///
/// # Errors
///
/// [`StoreError::Db`].
pub async fn count_unreviewed(db: &impl ConnectionTrait) -> Result<i64, StoreError> {
    let row = db
        .query_one_raw(pg(
            "SELECT count(*) AS n FROM break_glass_grants g
             LEFT JOIN break_glass_reviews r USING (grant_id) WHERE r.grant_id IS NULL",
            vec![],
        ))
        .await
        .map_err(db_err)?
        .ok_or_else(|| StoreError::Db("count returned no row".into()))?;
    row.try_get("", "n").map_err(db_err)
}

/// Review a grant. A note is required. A grant is reviewed once, and not by the person who opened it.
///
/// # Errors
///
/// [`StoreError::NotFound`]; [`StoreError::BadQuery`] (blank note, or the reviewer opened the grant);
/// [`StoreError::Conflict`] (current 1) when it was already reviewed.
pub async fn review(
    db: &impl ConnectionTrait,
    id: &str,
    by: &str,
    outcome: Outcome,
    note: &str,
) -> Result<Grant, StoreError> {
    let note = note.trim();
    if note.is_empty() || note.chars().count() > 500 {
        return Err(StoreError::BadQuery(
            "a note of 1 to 500 characters is required".into(),
        ));
    }
    let grant = get(db, id).await?;
    if grant.subject == by {
        return Err(StoreError::BadQuery(
            "a grant is reviewed by someone other than the person who opened it".into(),
        ));
    }
    if grant.review.is_some() {
        return Err(StoreError::Conflict { current: 1 });
    }
    let inserted = db
        .execute_raw(pg(
            "INSERT INTO break_glass_reviews (grant_id, reviewed_by, outcome, note)
             VALUES ($1::uuid, $2, $3, $4) ON CONFLICT (grant_id) DO NOTHING",
            vec![
                grant.id.clone().into(),
                by.into(),
                outcome.as_str().into(),
                note.into(),
            ],
        ))
        .await
        .map_err(db_err)?;
    if inserted.rows_affected() == 0 {
        return Err(StoreError::Conflict { current: 1 });
    }
    get(db, id).await
}
