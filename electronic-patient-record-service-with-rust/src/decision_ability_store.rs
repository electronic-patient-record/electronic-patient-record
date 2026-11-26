//! Judgements of whether a person can make one decision, now, stored (RF-8): append-only versions.
//!
//! A judgement is never changed: a new view is a new judgement, so there is no update. The one later version marks a
//! judgement entered in error, with who and why; it stays, and counts for nothing. Nothing here, or in
//! [`crate::decision_ability`], answers whether a person "has capacity". The judgements of a record merged into this one
//! are listed with it, as another record's.

use crate::decision_ability::{Judgement, State};
use crate::ids;
use crate::patients_store::{self, StoreError};
use crate::pool::Db;
use serde::Serialize;
use sqlx::postgres::PgRow;
use sqlx::{AssertSqlSafe, Connection, PgPool, Row};

crate::store_errors!("decision_ability");

/// A judgement as stored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StoredJudgement {
    /// The server-assigned id (a UUID), the same for every version.
    pub id: String,
    /// The patient record it is recorded on.
    pub patient_id: String,
    /// 1 when recorded; 2 when marked entered in error.
    pub version: i32,
    /// Who wrote this version.
    pub author: String,
    /// Where it stands.
    pub state: State,
    /// Why it was marked entered in error; `None` while it stands as recorded.
    pub reason: Option<String>,
    /// The judgement, as made.
    pub judgement: Judgement,
}

const COLUMNS: &str = "judgement_id::text AS id, patient_id::text AS patient_id, version, author, state, reason, body";

fn row_to_stored(row: &PgRow) -> Result<StoredJudgement, StoreError> {
    let body: serde_json::Value = row.try_get("body").map_err(db_err)?;
    let state: String = row.try_get("state").map_err(db_err)?;
    Ok(StoredJudgement {
        id: row.try_get("id").map_err(db_err)?,
        patient_id: row.try_get("patient_id").map_err(db_err)?,
        version: row.try_get("version").map_err(db_err)?,
        author: row.try_get("author").map_err(db_err)?,
        state: State::parse(&state).ok_or_else(|| {
            StoreError::Db("an unknown word in decision_ability_versions.state".into())
        })?,
        reason: row.try_get("reason").map_err(db_err)?,
        judgement: serde_json::from_value(body).map_err(db_err)?,
    })
}

/// Record a judgement on a patient record, at version 1.
///
/// # Errors
///
/// [`StoreError::NotFound`] when the patient does not exist; [`StoreError::Db`] when the insert fails.
pub async fn create(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    author: &str,
    judgement: &Judgement,
) -> Result<StoredJudgement, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let patient = patients_store::current(&mut *conn, patient_id).await?;
    let row = sqlx::query(AssertSqlSafe(format!(
        "INSERT INTO decision_ability_versions (judgement_id, version, patient_id, author, state, decides, body)
                 VALUES (gen_random_uuid(), 1, $1::uuid, $2, 'recorded', $3, $4) RETURNING {COLUMNS}"
    )))
    .bind(patient.id)
    .bind(author)
    .bind(judgement.decides.as_str())
    .bind(serde_json::to_value(judgement).map_err(db_err)?)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db_err)?
    .ok_or_else(|| StoreError::Db("insert returned no row".into()))?;
    row_to_stored(&row)
}

/// The judgements on a patient record and on every record merged into it, current versions, newest judgement first.
///
/// # Errors
///
/// [`StoreError::Db`] on a database failure.
pub async fn list_with_merged(
    db: &PgPool,
    patient_id: &str,
) -> Result<Vec<StoredJudgement>, StoreError> {
    let Some(own) = ids::uuid(patient_id) else {
        return Ok(Vec::new());
    };
    let mut records = crate::links::sources_of(db, &own).await?;
    records.push(own);
    let rows = sqlx::query(AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM decision_ability_current
                 WHERE patient_id = ANY($1::uuid[])
                 ORDER BY judgement_id"
    )))
    .bind(records)
    .fetch_all(db)
    .await
    .map_err(db_err)?;
    let mut all: Vec<StoredJudgement> = rows.iter().map(row_to_stored).collect::<Result<_, _>>()?;
    // By the time the judgement was made, read as a time (an offset in the text would sort wrongly as text).
    all.sort_by_key(|s| std::cmp::Reverse(crate::clock::parse_rfc3339(&s.judgement.at)));
    Ok(all)
}

/// The current version of one judgement on one patient record.
///
/// # Errors
///
/// [`StoreError::NotFound`] when there is no such judgement on that record.
pub async fn current(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    judgement_id: &str,
) -> Result<StoredJudgement, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let (Some(patient_id), Some(judgement_id)) = (ids::uuid(patient_id), ids::uuid(judgement_id))
    else {
        return Err(StoreError::NotFound);
    };
    let row = sqlx::query(AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM decision_ability_current
                 WHERE judgement_id = $1::uuid AND patient_id = $2::uuid"
    )))
    .bind(judgement_id)
    .bind(patient_id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db_err)?
    .ok_or(StoreError::NotFound)?;
    row_to_stored(&row)
}

/// Mark a judgement entered in error, as version `expected_version + 1`, with why. The judgement itself is kept as made.
///
/// # Errors
///
/// [`StoreError::NotFound`]; [`StoreError::Conflict`] when `expected_version` is not current, including a
/// concurrent writer; [`StoreError::BadQuery`] for a blank or over-long reason, or one already marked.
pub async fn mark_entered_in_error(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    judgement_id: &str,
    expected_version: i32,
    author: &str,
    reason: &str,
) -> Result<StoredJudgement, StoreError> {
    let reason = reason.trim();
    if reason.is_empty() || reason.chars().count() > crate::decision_ability::WORDS_LIMIT {
        return Err(StoreError::BadQuery(format!(
            "a reason of 1 to {} characters is required",
            crate::decision_ability::WORDS_LIMIT
        )));
    }
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let mut tx = conn.begin().await.map_err(db_err)?;
    let db = &mut *tx;
    let now = current(&mut *db, patient_id, judgement_id).await?;
    if now.version != expected_version {
        return Err(StoreError::Conflict {
            current: now.version,
        });
    }
    if now.state == State::EnteredInError {
        return Err(StoreError::BadQuery(
            "this judgement is already marked entered in error".into(),
        ));
    }
    let new_version = crate::store::next_version(expected_version)?;
    let row = match crate::store::append_version(
        &mut *db,
        STORE,
        sqlx::query(AssertSqlSafe(format!(
            "INSERT INTO decision_ability_versions
                   (judgement_id, version, patient_id, author, reason, state, decides, body)
                 VALUES ($1::uuid, $2, $3::uuid, $4, $5, 'entered-in-error', $6, $7) RETURNING {COLUMNS}"
        )))
        .bind(now.id.clone())
        .bind(new_version)
        .bind(now.patient_id.clone())
        .bind(author)
        .bind(reason)
        .bind(now.judgement.decides.as_str())
        .bind(serde_json::to_value(&now.judgement).map_err(db_err)?),
    )
    .await?
    {
        crate::store::Appended::Row(row) => row,
        crate::store::Appended::Taken => {
            let current = current(&mut *db, patient_id, judgement_id).await?.version;
            return Err(StoreError::Conflict { current });
        }
    };
    let stored = row_to_stored(&row)?;
    tx.commit().await.map_err(db_err)?;
    Ok(stored)
}
