//! Reasonable adjustments (PA-6): the append-only store behind the rules in [`crate::adjustments`]. Every
//! change is a new version; nothing is edited, so what a person needed stays in the history.
//!
//! Reads follow a merge by provenance: an adjustment on a record merged away counts for the surviving
//! record. The routes are in `controllers/adjustments.rs`.

use crate::adjustments::{self, Adjustment, Change, Refusal};
use crate::ids;
use crate::links;
use crate::patients_store::{self, StoreError};
use crate::store::pg;
use chrono::NaiveDate;
use sea_orm::{ConnectionTrait, TransactionTrait};
use serde::Serialize;

crate::store_errors!("adjustments");

/// One adjustment as stored, with what a reader needs beside it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StoredAdjustment {
    /// Server id, the same for every version.
    pub id: String,
    /// The patient it was recorded on.
    pub patient_id: String,
    /// 1 when first recorded.
    pub version: i32,
    /// Who wrote this version.
    pub author: String,
    /// When this version was written (UTC). For an ended adjustment, when it ended.
    pub recorded_at: String,
    /// The adjustment after this version.
    pub adjustment: Adjustment,
    /// The category of its code.
    pub category: String,
    /// The English words for its code.
    pub display: String,
    /// Whether the date to check it with the person has passed.
    pub overdue: bool,
}

const COLUMNS: &str = "adjustment_id::text AS id, patient_id::text AS patient_id, version, author,
    to_char(recorded_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"') AS at, body";

/// Read one row into a [`StoredAdjustment`]. A code no longer in the catalogue is still shown, under its
/// code: a need does not vanish because a list changed.
///
/// # Errors
///
/// [`StoreError::Db`] when a column is missing or the stored body is unreadable.
fn row_to_stored(
    row: &sea_orm::QueryResult,
    today: NaiveDate,
) -> Result<StoredAdjustment, StoreError> {
    let body: serde_json::Value = row.try_get("", "body").map_err(db_err)?;
    let adjustment: Adjustment = serde_json::from_value(body).map_err(db_err)?;
    let (category, display) = adjustments::lookup(&adjustment.code)
        .map_or(("other", adjustment.code.as_str()), |(c, d)| (c, d));
    Ok(StoredAdjustment {
        id: row.try_get("", "id").map_err(db_err)?,
        patient_id: row.try_get("", "patient_id").map_err(db_err)?,
        version: row.try_get("", "version").map_err(db_err)?,
        author: row.try_get("", "author").map_err(db_err)?,
        recorded_at: row.try_get("", "at").map_err(db_err)?,
        category: category.to_string(),
        display: display.to_string(),
        overdue: adjustments::overdue(&adjustment, today),
        adjustment,
    })
}

/// A refused record or change as the writer's to fix.
pub(crate) fn refused(r: Refusal) -> StoreError {
    StoreError::BadQuery(match r {
        Refusal::UnknownCode => "that adjustment is not in the catalogue".to_string(),
        Refusal::ReasonRequired => "a reason is required".to_string(),
        Refusal::LanguageRequired => "say which language the interpreter is for".to_string(),
        Refusal::NoteTooLong => format!(
            "a note may be at most {} characters",
            adjustments::NOTE_LIMIT
        ),
        Refusal::DateInvalid => "the review date must be a real day, YYYY-MM-DD".to_string(),
        Refusal::DateInPast => "the review date has passed".to_string(),
        Refusal::Ended => "this adjustment has ended".to_string(),
        Refusal::AlreadyActive => "this need is already recorded".to_string(),
    })
}

/// Every adjustment for a patient at its current version, newest first, ended ones included, and those
/// made on a record merged into this one.
///
/// # Errors
///
/// [`StoreError::NotFound`] (no such patient).
pub async fn list(
    db: &impl ConnectionTrait,
    patient_id: &str,
    today: NaiveDate,
) -> Result<Vec<StoredAdjustment>, StoreError> {
    let patient = patients_store::current(db, patient_id).await?;
    let linked = ids::uuids(&links::linked_ids(db, &patient.id).await?);
    let rows = db
        .query_all_raw(pg(
            &format!(
                "SELECT {COLUMNS} FROM adjustment_current
                 WHERE patient_id = ANY($1::uuid[])
                 ORDER BY recorded_at DESC, adjustment_id"
            ),
            vec![linked.into()],
        ))
        .await
        .map_err(db_err)?;
    rows.iter().map(|r| row_to_stored(r, today)).collect()
}

/// Store a new adjustment at version 1. The same need twice while the first stands is refused. Two
/// writers at the same moment can both pass this check; the second is a duplicate a person can end.
///
/// # Errors
///
/// [`StoreError::NotFound`] (no such patient), [`StoreError::BadQuery`] (already recorded).
pub async fn record(
    db: &impl ConnectionTrait,
    patient_id: &str,
    author: &str,
    adjustment: &Adjustment,
    today: NaiveDate,
) -> Result<StoredAdjustment, StoreError> {
    let standing = list(db, patient_id, today).await?;
    let repeated = standing.iter().any(|s| {
        s.adjustment.status == adjustments::Status::Active
            && s.adjustment.code == adjustment.code
            && s.adjustment.note == adjustment.note
    });
    if repeated {
        return Err(refused(Refusal::AlreadyActive));
    }
    let patient = patients_store::current(db, patient_id).await?;
    let row = db
        .query_one_raw(pg(
            &format!(
                "INSERT INTO adjustment_versions
                   (adjustment_id, version, patient_id, author, status, body)
                 VALUES (gen_random_uuid(), 1, $1::uuid, $2, $3, $4)
                 RETURNING {COLUMNS}"
            ),
            vec![
                patient.id.into(),
                author.to_string().into(),
                adjustment.status.word().into(),
                serde_json::to_value(adjustment).map_err(db_err)?.into(),
            ],
        ))
        .await
        .map_err(db_err)?
        .ok_or_else(|| StoreError::Db("insert returned no row".into()))?;
    row_to_stored(&row, today)
}

/// One adjustment's current version.
///
/// # Errors
///
/// [`StoreError::NotFound`] when there is no such adjustment for that patient.
pub async fn current(
    db: &impl ConnectionTrait,
    patient_id: &str,
    adjustment_id: &str,
    today: NaiveDate,
) -> Result<StoredAdjustment, StoreError> {
    let adjustment_id = ids::uuid(adjustment_id).ok_or(StoreError::NotFound)?;
    let linked = ids::uuids(&links::linked_ids(db, patient_id).await?);
    let row = db
        .query_one_raw(pg(
            &format!(
                "SELECT {COLUMNS} FROM adjustment_current
                 WHERE adjustment_id = $1::uuid AND patient_id = ANY($2::uuid[])"
            ),
            vec![adjustment_id.into(), linked.into()],
        ))
        .await
        .map_err(db_err)?
        .ok_or(StoreError::NotFound)?;
    row_to_stored(&row, today)
}

/// Append the next version of an adjustment.
///
/// # Errors
///
/// [`StoreError::NotFound`], [`StoreError::Conflict`] (stale or racing version),
/// [`StoreError::BadQuery`] (the rules refused the change).
#[allow(clippy::too_many_arguments)]
pub async fn change(
    db: &(impl ConnectionTrait + TransactionTrait),
    patient_id: &str,
    adjustment_id: &str,
    expected_version: i32,
    author: &str,
    what: &Change,
    reason: &str,
    today: NaiveDate,
) -> Result<StoredAdjustment, StoreError> {
    let was = current(db, patient_id, adjustment_id, today).await?;
    if was.version != expected_version {
        return Err(StoreError::Conflict {
            current: was.version,
        });
    }
    let next = adjustments::change(&was.adjustment, what, reason, today).map_err(refused)?;
    let appended = crate::store::append_version(
        db,
        STORE,
        pg(
            &format!(
                "INSERT INTO adjustment_versions
                   (adjustment_id, version, patient_id, author, status, body)
                 VALUES ($1::uuid, $2, $3::uuid, $4, $5, $6)
                 RETURNING {COLUMNS}"
            ),
            vec![
                was.id.clone().into(),
                (expected_version + 1).into(),
                was.patient_id.clone().into(),
                author.to_string().into(),
                next.status.word().into(),
                serde_json::to_value(&next).map_err(db_err)?.into(),
            ],
        ),
    )
    .await?;
    match appended {
        crate::store::Appended::Row(row) => row_to_stored(&row, today),
        crate::store::Appended::Taken => {
            let current = current(db, patient_id, adjustment_id, today).await?.version;
            Err(StoreError::Conflict { current })
        }
    }
}
