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

use crate::ids;
use crate::patients_store::{self, StoreError};
use crate::pool::Db;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgRow;
use sqlx::{AssertSqlSafe, Row};

crate::store_errors!("break-glass");

/// How long a grant lasts. Fixed: the clinician cannot choose a longer one.
pub const GRANT_HOURS: i64 = 4;

/// How long a grant may wait for its review before it is overdue (EC-11), when `EPR_BREAK_GLASS_REVIEW_HOURS`
/// does not say otherwise. **A guess**: no policy has set it.
pub const DEFAULT_REVIEW_DUE_HOURS: i64 = 72;

/// The hours a grant may wait for review: `EPR_BREAK_GLASS_REVIEW_HOURS` when it is a whole number from 1 to
/// 8760, else [`DEFAULT_REVIEW_DUE_HOURS`].
#[must_use]
pub fn review_due_hours() -> i64 {
    due_hours_from(
        std::env::var("EPR_BREAK_GLASS_REVIEW_HOURS")
            .ok()
            .as_deref(),
    )
}

/// The review deadline in hours from a setting: a whole number from 1 to 8760 (a year), else the default. A bad value
/// falls back rather than stopping the service.
fn due_hours_from(value: Option<&str>) -> i64 {
    value
        .and_then(|v| v.trim().parse::<i64>().ok())
        .filter(|h| (1..=8760).contains(h))
        .unwrap_or(DEFAULT_REVIEW_DUE_HOURS)
}

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
    /// Unreviewed for longer than [`review_due_hours`] (EC-11). Never true once there is a review.
    pub overdue: bool,
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
    /// The word stored in `break_glass_reviews.outcome`.
    fn as_str(self) -> &'static str {
        match self {
            Outcome::Appropriate => "appropriate",
            Outcome::Inappropriate => "inappropriate",
        }
    }
}

const TS: &str = "'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"'";

/// The `SELECT` for a grant joined to its review, if any, with the expiry both as text and as a Unix second for the
/// policy engine.
fn select() -> String {
    format!(
        "SELECT g.grant_id::text AS id, g.patient_id::text AS patient_id, g.subject, g.reason,
                to_char(g.granted_at AT TIME ZONE 'UTC', {TS}) AS granted_at,
                to_char(g.expires_at AT TIME ZONE 'UTC', {TS}) AS expires_at,
                floor(extract(epoch FROM g.expires_at))::bigint AS expires_unix,
                r.reviewed_by, r.outcome, r.note,
                to_char(r.reviewed_at AT TIME ZONE 'UTC', {TS}) AS reviewed_at,
                (r.grant_id IS NULL
                 AND g.granted_at < now() - make_interval(hours => {}::int)) AS overdue
         FROM break_glass_grants g LEFT JOIN break_glass_reviews r USING (grant_id)",
        review_due_hours()
    )
}

/// Read one row of [`select`] into a [`Grant`]; a row with a `reviewed_by` has a review, one without is unreviewed.
///
/// # Errors
///
/// [`StoreError::Db`] when a column is missing.
fn to_grant(row: &PgRow) -> Result<Grant, StoreError> {
    let get = |c: &str| row.try_get::<String, _>(c).map_err(db_err);
    let reviewed_by: Option<String> = row.try_get("reviewed_by").map_err(db_err)?;
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
        expires_unix: row.try_get("expires_unix").map_err(db_err)?,
        review,
        overdue: row.try_get("overdue").map_err(db_err)?,
    })
}

/// The grant `subject` holds on `patient_id` right now, if any (the latest to expire).
///
/// # Errors
///
/// [`StoreError::Db`].
pub async fn active(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    subject: &str,
) -> Result<Option<Grant>, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let Some(patient_id) = ids::uuid(patient_id) else {
        return Ok(None);
    };
    let row = sqlx::query(AssertSqlSafe(format!(
        "{} WHERE g.patient_id = $1::uuid AND g.subject = $2 AND g.expires_at > now()
                 ORDER BY g.expires_at DESC LIMIT 1",
        select()
    )))
    .bind(patient_id)
    .bind(subject)
    .fetch_optional(&mut *db)
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
    db: impl Into<Db<'_>>,
    patient_id: &str,
    subject: &str,
    reason: &str,
) -> Result<(Grant, bool), StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let reason = reason.trim();
    if reason.is_empty() || reason.chars().count() > 500 || reason.chars().any(char::is_control) {
        return Err(StoreError::BadQuery(
            "a reason of 1 to 500 characters is required".into(),
        ));
    }
    let patient = patients_store::current(&mut *db, patient_id).await?;
    if let Some(existing) = active(&mut *db, &patient.id, subject).await? {
        return Ok((existing, false));
    }
    let row = sqlx::query(
        "INSERT INTO break_glass_grants (patient_id, subject, reason, expires_at)
             VALUES ($1::uuid, $2, $3, now() + make_interval(hours => $4::int))
             RETURNING grant_id::text AS id",
    )
    .bind(patient.id.clone())
    .bind(subject)
    .bind(reason)
    .bind(i32::try_from(GRANT_HOURS).unwrap_or(4))
    .fetch_optional(&mut *db)
    .await
    .map_err(db_err)?
    .ok_or_else(|| StoreError::Db("insert returned no row".into()))?;
    let id: String = row.try_get("id").map_err(db_err)?;
    Ok((get(&mut *db, &id).await?, true))
}

/// One grant, reviewed or not.
///
/// # Errors
///
/// [`StoreError::NotFound`] (also for an id that is not a UUID).
pub async fn get(db: impl Into<Db<'_>>, id: &str) -> Result<Grant, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let id = ids::uuid(id).ok_or(StoreError::NotFound)?;
    let row = sqlx::query(AssertSqlSafe(format!(
        "{} WHERE g.grant_id = $1::uuid",
        select()
    )))
    .bind(id)
    .fetch_optional(&mut *db)
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
pub async fn unreviewed(db: impl Into<Db<'_>>) -> Result<crate::page::Page<Grant>, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let rows = sqlx::query(AssertSqlSafe(format!(
        "{} WHERE r.grant_id IS NULL ORDER BY g.granted_at, g.grant_id LIMIT {}",
        select(),
        LIST_LIMIT + 1
    )))
    .fetch_all(&mut *db)
    .await
    .map_err(db_err)?;
    let fetched = rows.iter().map(to_grant).collect::<Result<Vec<_>, _>>()?;
    Ok(crate::page::Page::cut(fetched, LIST_LIMIT))
}

/// How many grants nobody has reviewed.
///
/// # Errors
///
/// [`StoreError::Db`].
pub async fn count_unreviewed(db: impl Into<Db<'_>>) -> Result<i64, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let row = sqlx::query(
        "SELECT count(*) AS n FROM break_glass_grants g
             LEFT JOIN break_glass_reviews r USING (grant_id) WHERE r.grant_id IS NULL",
    )
    .fetch_optional(&mut *db)
    .await
    .map_err(db_err)?
    .ok_or_else(|| StoreError::Db("count returned no row".into()))?;
    row.try_get("n").map_err(db_err)
}

/// How many unreviewed grants are overdue (EC-11).
///
/// # Errors
///
/// [`StoreError::Db`].
pub async fn count_overdue(db: impl Into<Db<'_>>) -> Result<i64, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let row = sqlx::query(AssertSqlSafe(format!(
                "SELECT count(*) AS n FROM break_glass_grants g
                 LEFT JOIN break_glass_reviews r USING (grant_id)
                 WHERE r.grant_id IS NULL AND g.granted_at < now() - make_interval(hours => {}::int)",
                review_due_hours()
            )))
.fetch_optional(&mut *db)
        .await
        .map_err(db_err)?
        .ok_or_else(|| StoreError::Db("count returned no row".into()))?;
    row.try_get("n").map_err(db_err)
}

/// Review a grant. A note is required. A grant is reviewed once, and not by the person who opened it.
///
/// # Errors
///
/// [`StoreError::NotFound`]; [`StoreError::BadQuery`] (blank note, or the reviewer opened the grant);
/// [`StoreError::Conflict`] (current 1) when it was already reviewed.
pub async fn review(
    db: impl Into<Db<'_>>,
    id: &str,
    by: &str,
    outcome: Outcome,
    note: &str,
) -> Result<Grant, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let note = note.trim();
    if note.is_empty() || note.chars().count() > 500 {
        return Err(StoreError::BadQuery(
            "a note of 1 to 500 characters is required".into(),
        ));
    }
    let grant = get(&mut *db, id).await?;
    if grant.subject == by {
        return Err(StoreError::BadQuery(
            "a grant is reviewed by someone other than the person who opened it".into(),
        ));
    }
    if grant.review.is_some() {
        return Err(StoreError::Conflict { current: 1 });
    }
    let inserted = sqlx::query(
        "INSERT INTO break_glass_reviews (grant_id, reviewed_by, outcome, note)
             VALUES ($1::uuid, $2, $3, $4) ON CONFLICT (grant_id) DO NOTHING",
    )
    .bind(grant.id.clone())
    .bind(by)
    .bind(outcome.as_str())
    .bind(note)
    .execute(&mut *db)
    .await
    .map_err(db_err)?;
    if inserted.rows_affected() == 0 {
        return Err(StoreError::Conflict { current: 1 });
    }
    get(&mut *db, id).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_review_limit_is_a_whole_number_of_hours_or_the_default() {
        assert_eq!(due_hours_from(None), DEFAULT_REVIEW_DUE_HOURS);
        assert_eq!(due_hours_from(Some("24")), 24);
        assert_eq!(due_hours_from(Some(" 48 ")), 48);
        for bad in ["", "0", "-5", "8761", "2.5", "soon"] {
            assert_eq!(due_hours_from(Some(bad)), DEFAULT_REVIEW_DUE_HOURS, "{bad}");
        }
        assert_eq!(due_hours_from(Some("1")), 1);
        assert_eq!(due_hours_from(Some("8760")), 8760);
    }
}
