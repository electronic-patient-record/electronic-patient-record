//! Who speaks for a person, stored (RF-9): append-only versions, optimistic updates.
//!
//! One entry per person (two parents are two), with what of their paper was seen and their views. A change, a view that
//! moved or paper seen later, is a new version; a stale `expected_version` is a [`StoreError::Conflict`]. Entered in error
//! is a state, with why, and ends changes. Nothing here, or in [`crate::who_speaks`], decides who wins.

use crate::ids;
use crate::patients_store::{self, StoreError};
use crate::pool::Db;
use crate::who_speaks::Speaker;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgRow;
use sqlx::{AssertSqlSafe, Connection, PgPool, Row};

crate::store_errors!("speakers");

/// Where a stored entry stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum State {
    /// As recorded.
    Stands,
    /// Recorded by mistake. It stays, and counts for nothing.
    EnteredInError,
}

impl State {
    /// Both states (the stored words are checked against the database, PR-3).
    pub const ALL: [State; 2] = [State::Stands, State::EnteredInError];

    /// The stored word.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            State::Stands => "stands",
            State::EnteredInError => "entered-in-error",
        }
    }

    fn parse(word: &str) -> Option<State> {
        State::ALL.into_iter().find(|s| s.as_str() == word)
    }
}

/// An entry as stored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StoredSpeaker {
    /// The server-assigned id (a UUID), the same for every version.
    pub id: String,
    /// The patient record it is recorded on.
    pub patient_id: String,
    /// 1 for the first version.
    pub version: i32,
    /// Who wrote this version.
    pub author: String,
    /// Where it stands.
    pub state: State,
    /// Why this version was written; required to mark one entered in error.
    pub reason: Option<String>,
    /// The entry.
    pub speaker: Speaker,
}

const COLUMNS: &str =
    "speaker_id::text AS id, patient_id::text AS patient_id, version, author, state, reason, body";

fn row_to_stored(row: &PgRow) -> Result<StoredSpeaker, StoreError> {
    let body: serde_json::Value = row.try_get("body").map_err(db_err)?;
    let state: String = row.try_get("state").map_err(db_err)?;
    Ok(StoredSpeaker {
        id: row.try_get("id").map_err(db_err)?,
        patient_id: row.try_get("patient_id").map_err(db_err)?,
        version: row.try_get("version").map_err(db_err)?,
        author: row.try_get("author").map_err(db_err)?,
        state: State::parse(&state)
            .ok_or_else(|| StoreError::Db("an unknown word in speaker_versions.state".into()))?,
        reason: row.try_get("reason").map_err(db_err)?,
        speaker: serde_json::from_value(body).map_err(db_err)?,
    })
}

/// Record a new entry on a patient record, at version 1.
///
/// # Errors
///
/// [`StoreError::NotFound`] when the patient does not exist; [`StoreError::Db`] when the insert fails.
pub async fn create(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    author: &str,
    reason: Option<&str>,
    speaker: &Speaker,
) -> Result<StoredSpeaker, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let patient = patients_store::current(&mut *conn, patient_id).await?;
    let row = sqlx::query(AssertSqlSafe(format!(
        "INSERT INTO speaker_versions (speaker_id, version, patient_id, author, reason, state, role, body)
                 VALUES (gen_random_uuid(), 1, $1::uuid, $2, $3, 'stands', $4, $5) RETURNING {COLUMNS}"
    )))
    .bind(patient.id)
    .bind(author)
    .bind(reason.map(str::to_string))
    .bind(speaker.role.as_str())
    .bind(serde_json::to_value(speaker).map_err(db_err)?)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db_err)?
    .ok_or_else(|| StoreError::Db("insert returned no row".into()))?;
    row_to_stored(&row)
}

/// The entries of a patient record and of every record merged into it, current versions, oldest first.
///
/// # Errors
///
/// [`StoreError::Db`] on a database failure.
pub async fn list_with_merged(
    db: &PgPool,
    patient_id: &str,
) -> Result<Vec<StoredSpeaker>, StoreError> {
    let Some(own) = ids::uuid(patient_id) else {
        return Ok(Vec::new());
    };
    let mut records = crate::links::sources_of(db, &own).await?;
    records.push(own);
    let rows = sqlx::query(AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM speaker_current WHERE patient_id = ANY($1::uuid[]) ORDER BY recorded_at, speaker_id"
    )))
    .bind(records)
    .fetch_all(db)
    .await
    .map_err(db_err)?;
    rows.iter().map(row_to_stored).collect()
}

/// The current version of one entry on one patient record.
///
/// # Errors
///
/// [`StoreError::NotFound`] when there is no such entry on that record.
pub async fn current(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    speaker_id: &str,
) -> Result<StoredSpeaker, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let (Some(patient_id), Some(speaker_id)) = (ids::uuid(patient_id), ids::uuid(speaker_id))
    else {
        return Err(StoreError::NotFound);
    };
    let row = sqlx::query(AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM speaker_current WHERE speaker_id = $1::uuid AND patient_id = $2::uuid"
    )))
    .bind(speaker_id)
    .bind(patient_id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db_err)?
    .ok_or(StoreError::NotFound)?;
    row_to_stored(&row)
}

/// A change to an entry: a new version.
#[derive(Debug, Clone)]
pub struct Change<'a> {
    /// The version it was made from.
    pub expected_version: i32,
    /// Who made it.
    pub author: &'a str,
    /// Why; required to mark one entered in error.
    pub reason: Option<&'a str>,
    /// The whole new entry.
    pub speaker: &'a Speaker,
    /// Where it stands after.
    pub state: State,
}

/// Append version `expected_version + 1` with `speaker` and `state`. Earlier versions stay. Marking one entered in error
/// needs a reason, and nothing changes one after it.
///
/// # Errors
///
/// [`StoreError::NotFound`]; [`StoreError::Conflict`] when `expected_version` is not current, including a concurrent
/// writer; [`StoreError::BadQuery`] for one already entered in error, or marked so with no reason.
pub async fn update(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    speaker_id: &str,
    change: &Change<'_>,
) -> Result<StoredSpeaker, StoreError> {
    let Change {
        expected_version,
        author,
        reason,
        speaker,
        state,
    } = *change;
    let reason = reason.map(str::trim).filter(|r| !r.is_empty());
    if state == State::EnteredInError && reason.is_none() {
        return Err(StoreError::BadQuery(
            "a reason is required to mark one entered in error".into(),
        ));
    }
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let mut tx = conn.begin().await.map_err(db_err)?;
    let db = &mut *tx;
    let now = current(&mut *db, patient_id, speaker_id).await?;
    if now.version != expected_version {
        return Err(StoreError::Conflict {
            current: now.version,
        });
    }
    if now.state == State::EnteredInError {
        return Err(StoreError::BadQuery(
            "an entry entered in error is not changed: record a new one".into(),
        ));
    }
    let new_version = crate::store::next_version(expected_version)?;
    let row = match crate::store::append_version(
        &mut *db,
        STORE,
        sqlx::query(AssertSqlSafe(format!(
            "INSERT INTO speaker_versions (speaker_id, version, patient_id, author, reason, state, role, body)
                 VALUES ($1::uuid, $2, $3::uuid, $4, $5, $6, $7, $8) RETURNING {COLUMNS}"
        )))
        .bind(now.id.clone())
        .bind(new_version)
        .bind(now.patient_id.clone())
        .bind(author)
        .bind(reason.map(str::to_string))
        .bind(state.as_str())
        .bind(speaker.role.as_str())
        .bind(serde_json::to_value(speaker).map_err(db_err)?),
    )
    .await?
    {
        crate::store::Appended::Row(row) => row,
        crate::store::Appended::Taken => {
            let current = current(&mut *db, patient_id, speaker_id).await?.version;
            return Err(StoreError::Conflict { current });
        }
    };
    let stored = row_to_stored(&row)?;
    tx.commit().await.map_err(db_err)?;
    Ok(stored)
}
