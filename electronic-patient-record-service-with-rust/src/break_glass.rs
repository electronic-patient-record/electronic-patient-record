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
use sqlx::{AssertSqlSafe, Connection, Row};

crate::store_errors!("break-glass");

/// How long a grant lasts. Fixed: the clinician cannot choose a longer one.
pub const GRANT_HOURS: i32 = 4;

/// How long a grant may wait for its review before it is overdue (EC-11), when `EPR_BREAK_GLASS_REVIEW_HOURS`
/// does not say otherwise. **A guess**: no policy has set it.
pub const DEFAULT_REVIEW_DUE_HOURS: i64 = 72;

/// The hours a grant may wait for review: `EPR_BREAK_GLASS_REVIEW_HOURS` when it is a whole number from 1 to
/// 8760, [`DEFAULT_REVIEW_DUE_HOURS`] when it is not set.
///
/// # Errors
///
/// The text naming the setting when it is set to anything else, empty included (RCA-IN-3, RCA-IN-4).
pub fn review_due_hours() -> Result<i64, String> {
    due_hours_from(crate::runtime::setting("EPR_BREAK_GLASS_REVIEW_HOURS").as_deref())
}

/// Check `EPR_BREAK_GLASS_REVIEW_HOURS` at start: a setting that is wrong stops the service (RCA-IN-4).
///
/// # Errors
///
/// As [`review_due_hours`].
pub fn check_settings() -> Result<(), String> {
    review_due_hours().map(|_| ())
}

/// The review deadline in hours from a setting. Absent is the default (RCA-IN-3, declared in EC-11); a value
/// that is set but is not a whole number from 1 to 8760 (a year) is an error, never the default.
fn due_hours_from(value: Option<&str>) -> Result<i64, String> {
    let Some(text) = value else {
        return Ok(DEFAULT_REVIEW_DUE_HOURS); // EC-11: absent means the default
    };
    text.trim()
        .parse::<i64>()
        .ok()
        .filter(|h| (1..=8760).contains(h))
        .ok_or_else(|| {
            format!(
                "EPR_BREAK_GLASS_REVIEW_HOURS must be a whole number from 1 to 8760, not {text:?}"
            )
        })
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

/// The time as SQL: the database's own when `at` is `None`, which is what the service always asks for, so that several
/// replicas whose clocks differ agree on when a grant ends. A test gives the time instead of writing a row in the past
/// (PR-5).
fn time(at: Option<jiff::Timestamp>) -> String {
    at.map_or_else(
        || "now()".to_string(),
        |t| format!("'{}'::timestamptz", crate::clock::rfc3339_micros(t)),
    )
}

/// The `SELECT` for a grant joined to its review, if any, with the expiry both as text and as a Unix second for the
/// policy engine.
fn select(at: Option<jiff::Timestamp>) -> Result<String, StoreError> {
    let hours = review_due_hours().map_err(StoreError::Db)?;
    Ok(format!(
        "SELECT g.grant_id::text AS id, g.patient_id::text AS patient_id, g.subject, g.reason,
                to_char(g.granted_at AT TIME ZONE 'UTC', {TS}) AS granted_at,
                to_char(g.expires_at AT TIME ZONE 'UTC', {TS}) AS expires_at,
                floor(extract(epoch FROM g.expires_at))::bigint AS expires_unix,
                r.reviewed_by, r.outcome, r.note,
                to_char(r.reviewed_at AT TIME ZONE 'UTC', {TS}) AS reviewed_at,
                (r.grant_id IS NULL
                 AND g.granted_at < {} - make_interval(hours => {hours}::int)) AS overdue
         FROM break_glass_grants g LEFT JOIN break_glass_reviews r USING (grant_id)",
        time(at)
    ))
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
    active_at(db, patient_id, subject, None).await
}

/// [`active`] at a time; `None` is now, by the database.
///
/// # Errors
///
/// [`StoreError::Db`].
pub async fn active_at(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    subject: &str,
    at: Option<jiff::Timestamp>,
) -> Result<Option<Grant>, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let Some(patient_id) = ids::uuid(patient_id) else {
        return Ok(None);
    };
    let row = sqlx::query(AssertSqlSafe(format!(
        "{} WHERE g.patient_id = $1::uuid AND g.subject = $2 AND g.expires_at > {}
                 ORDER BY g.expires_at DESC LIMIT 1",
        select(at)?,
        time(at)
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
    open_at(db, patient_id, subject, reason, None).await
}

/// [`open`] at a time; `None` is now, by the database.
///
/// # Errors
///
/// As [`open`].
pub async fn open_at(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    subject: &str,
    reason: &str,
    at: Option<jiff::Timestamp>,
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
    // Two requests at one moment would each find no grant and make one. The lock is the transaction's, so the second
    // waits until the first is committed, in the caller's transaction when there is one, and then finds its grant.
    let mut tx = db.begin().await.map_err(db_err)?;
    let db = &mut *tx;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(format!("break-glass:{}:{subject}", patient.id))
        .execute(&mut *db)
        .await
        .map_err(db_err)?;
    if let Some(existing) = active_at(&mut *db, &patient.id, subject, at).await? {
        tx.commit().await.map_err(db_err)?;
        return Ok((existing, false));
    }
    let now = time(at);
    let row = sqlx::query(AssertSqlSafe(format!(
        "INSERT INTO break_glass_grants (patient_id, subject, reason, granted_at, expires_at)
             VALUES ($1::uuid, $2, $3, {now}, {now} + make_interval(hours => $4::int))
             RETURNING grant_id::text AS id"
    )))
    .bind(patient.id.clone())
    .bind(subject)
    .bind(reason)
    .bind(GRANT_HOURS)
    .fetch_optional(&mut *db)
    .await
    .map_err(db_err)?
    .ok_or_else(|| StoreError::Db("insert returned no row".into()))?;
    let id: String = row.try_get("id").map_err(db_err)?;
    let grant = get_at(&mut *db, &id, at).await?;
    tx.commit().await.map_err(db_err)?;
    Ok((grant, true))
}

/// One grant, reviewed or not.
///
/// # Errors
///
/// [`StoreError::NotFound`] (also for an id that is not a UUID).
pub async fn get(db: impl Into<Db<'_>>, id: &str) -> Result<Grant, StoreError> {
    get_at(db, id, None).await
}

/// [`get`] at a time, which decides whether it is overdue; `None` is now, by the database.
///
/// # Errors
///
/// As [`get`].
pub async fn get_at(
    db: impl Into<Db<'_>>,
    id: &str,
    at: Option<jiff::Timestamp>,
) -> Result<Grant, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let id = ids::uuid(id).ok_or(StoreError::NotFound)?;
    let row = sqlx::query(AssertSqlSafe(format!(
        "{} WHERE g.grant_id = $1::uuid",
        select(at)?
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
    unreviewed_at(db, None).await
}

/// [`unreviewed`] at a time, which decides what is overdue; `None` is now, by the database.
///
/// # Errors
///
/// [`StoreError::Db`].
pub async fn unreviewed_at(
    db: impl Into<Db<'_>>,
    at: Option<jiff::Timestamp>,
) -> Result<crate::page::Page<Grant>, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let rows = sqlx::query(AssertSqlSafe(format!(
        "{} WHERE r.grant_id IS NULL ORDER BY g.granted_at, g.grant_id LIMIT {}",
        select(at)?,
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
    count_overdue_at(db, None).await
}

/// [`count_overdue`] at a time; `None` is now, by the database.
///
/// # Errors
///
/// [`StoreError::Db`].
pub async fn count_overdue_at(
    db: impl Into<Db<'_>>,
    at: Option<jiff::Timestamp>,
) -> Result<i64, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let hours = review_due_hours().map_err(StoreError::Db)?;
    let row = sqlx::query(AssertSqlSafe(format!(
                "SELECT count(*) AS n FROM break_glass_grants g
                 LEFT JOIN break_glass_reviews r USING (grant_id)
                 WHERE r.grant_id IS NULL AND g.granted_at < {} - make_interval(hours => {hours}::int)",
                time(at)
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
    fn the_review_limit_is_a_whole_number_of_hours_when_set_and_the_default_only_when_absent() {
        assert_eq!(due_hours_from(None), Ok(DEFAULT_REVIEW_DUE_HOURS));
        assert_eq!(due_hours_from(Some("24")), Ok(24));
        assert_eq!(due_hours_from(Some(" 48 ")), Ok(48));
        assert_eq!(due_hours_from(Some("1")), Ok(1));
        assert_eq!(due_hours_from(Some("8760")), Ok(8760));
        for bad in ["", "0", "-5", "8761", "2.5", "soon", "99999999999999999999"] {
            let e = due_hours_from(Some(bad)).unwrap_err();
            assert!(
                e.starts_with("EPR_BREAK_GLASS_REVIEW_HOURS must be"),
                "{bad}: {e}"
            );
        }
    }
}
