//! Death assertions (BV-1): the append-only store behind the rules in [`crate::death`]. Every step is
//! a new version; nothing is edited, so a death recorded in error stays in the history with its
//! reason.
//!
//! Reads follow a merge by provenance: a report on a record merged away counts for the surviving
//! record, and a merged-away record is not one anything writes to. The routes are in
//! `controllers/deaths.rs`.

use crate::death::{self, Actor, Answer, Assertion, Refusal, Step};
use crate::ids;
use crate::links;
use crate::patients_store::{self, StoreError};
use crate::pool::Db;
use serde::Serialize;
use sqlx::postgres::PgRow;
use sqlx::{AssertSqlSafe, Row};

crate::store_errors!("death assertions");

/// One assertion as stored.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StoredAssertion {
    /// Server id, the same for every version.
    pub id: String,
    /// The patient the report was made on.
    pub patient_id: String,
    /// 1 for the report.
    pub version: i32,
    /// Who wrote this version.
    pub author: String,
    /// When this version was written (UTC).
    pub recorded_at: String,
    /// The assertion after this version.
    pub assertion: Assertion,
}

const COLUMNS: &str = "assertion_id::text AS id, patient_id::text AS patient_id, version, author,
    to_char(recorded_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"') AS at, body";

/// Read one row into a [`StoredAssertion`].
///
/// # Errors
///
/// [`StoreError::Db`] when a column is missing or the stored body is unreadable.
fn row_to_assertion(row: &PgRow) -> Result<StoredAssertion, StoreError> {
    let body: serde_json::Value = row.try_get("body").map_err(db_err)?;
    Ok(StoredAssertion {
        id: row.try_get("id").map_err(db_err)?,
        patient_id: row.try_get("patient_id").map_err(db_err)?,
        version: row.try_get("version").map_err(db_err)?,
        author: row.try_get("author").map_err(db_err)?,
        recorded_at: row.try_get("at").map_err(db_err)?,
        assertion: serde_json::from_value(body).map_err(db_err)?,
    })
}

/// A refused step as the writer's to fix.
pub(crate) fn refused(r: Refusal) -> StoreError {
    StoreError::BadQuery(
        match r {
            Refusal::AuthorRequired => "an author is required",
            Refusal::ReasonRequired => "a reason is required",
            Refusal::DateInvalid => "the date is not one this record can hold",
            Refusal::DateInFuture => "the date is in the future",
            Refusal::PersonRequired => "only a person can confirm or withdraw a death",
            Refusal::NotAllowed => "the assertion cannot move to that status",
        }
        .to_string(),
    )
}

/// The id of an actor, as the version's author.
fn author_of(actor: &Actor) -> &str {
    match actor {
        Actor::Person(id) | Actor::Feed(id) => id,
    }
}

/// Store a report made with [`death::report`], at version 1.
///
/// # Errors
///
/// [`StoreError::NotFound`] (no such patient).
pub async fn report(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    assertion: &Assertion,
) -> Result<StoredAssertion, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let patient = patients_store::current(&mut *db, patient_id).await?;
    let row = sqlx::query(AssertSqlSafe(format!(
        "INSERT INTO death_assertion_versions
                   (assertion_id, version, patient_id, author, status, body)
                 VALUES (gen_random_uuid(), 1, $1::uuid, $2, $3, $4)
                 RETURNING {COLUMNS}"
    )))
    .bind(patient.id)
    .bind(author_of(&assertion.reported_by))
    .bind(assertion.status.word())
    .bind(serde_json::to_value(assertion).map_err(db_err)?)
    .fetch_optional(&mut *db)
    .await
    .map_err(db_err)?
    .ok_or_else(|| StoreError::Db("insert returned no row".into()))?;
    row_to_assertion(&row)
}

/// Every assertion about a patient at its current version, newest first, withdrawn ones included,
/// and those made on a record merged into this one.
///
/// # Errors
///
/// [`StoreError::NotFound`] (no such patient).
pub async fn list(
    db: impl Into<Db<'_>>,
    patient_id: &str,
) -> Result<Vec<StoredAssertion>, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let patient = patients_store::current(&mut *db, patient_id).await?;
    let linked = ids::uuids(&links::linked_ids(&mut *db, &patient.id).await?);
    let rows = sqlx::query(AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM death_assertion_current
                 WHERE patient_id = ANY($1::uuid[])
                 ORDER BY recorded_at DESC, assertion_id"
    )))
    .bind(linked)
    .fetch_all(&mut *db)
    .await
    .map_err(db_err)?;
    rows.iter().map(row_to_assertion).collect()
}

/// The one answer for a patient: the read model a banner or a sender asks.
///
/// # Errors
///
/// [`StoreError::NotFound`] (no such patient).
pub async fn answer(db: impl Into<Db<'_>>, patient_id: &str) -> Result<Answer, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let all: Vec<Assertion> = list(&mut *db, patient_id)
        .await?
        .into_iter()
        .map(|s| s.assertion)
        .collect();
    Ok(death::answer(&all))
}

/// A patient as the matcher should see them: the stored identity, with `death_date` from a standing,
/// undisputed assertion. This is the only place the field is filled (OQ-BV3).
///
/// # Errors
///
/// [`StoreError::NotFound`] (no such patient).
pub async fn for_matching(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    patient: &electronic_patient_record_matcher::Patient,
) -> Result<electronic_patient_record_matcher::Patient, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let mut seen = patient.clone();
    seen.death_date = death::match_date(&answer(&mut *db, patient_id).await?);
    Ok(seen)
}

/// One assertion's current version.
///
/// # Errors
///
/// [`StoreError::NotFound`] when there is no such assertion for that patient.
pub async fn current(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    assertion_id: &str,
) -> Result<StoredAssertion, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let assertion_id = ids::uuid(assertion_id).ok_or(StoreError::NotFound)?;
    let linked = ids::uuids(&links::linked_ids(&mut *db, patient_id).await?);
    let row = sqlx::query(AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM death_assertion_current
                 WHERE assertion_id = $1::uuid AND patient_id = ANY($2::uuid[])"
    )))
    .bind(assertion_id)
    .bind(linked)
    .fetch_optional(&mut *db)
    .await
    .map_err(db_err)?
    .ok_or(StoreError::NotFound)?;
    row_to_assertion(&row)
}

/// Append the next version of an assertion.
///
/// # Errors
///
/// [`StoreError::NotFound`], [`StoreError::Conflict`] (stale or racing version),
/// [`StoreError::BadQuery`] (the rules refused the step).
pub async fn advance(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    assertion_id: &str,
    expected_version: i32,
    step: &Step,
) -> Result<StoredAssertion, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let was = current(&mut *db, patient_id, assertion_id).await?;
    if was.version != expected_version {
        return Err(StoreError::Conflict {
            current: was.version,
        });
    }
    let next = death::advance(&was.assertion, step).map_err(refused)?;
    let appended = crate::store::append_version(
        &mut *db,
        STORE,
        sqlx::query(AssertSqlSafe(format!(
            "INSERT INTO death_assertion_versions
                   (assertion_id, version, patient_id, author, status, body)
                 VALUES ($1::uuid, $2, $3::uuid, $4, $5, $6)
                 RETURNING {COLUMNS}"
        )))
        .bind(was.id.clone())
        .bind(expected_version + 1)
        .bind(was.patient_id.clone())
        .bind(author_of(&step.by))
        .bind(next.status.word())
        .bind(serde_json::to_value(&next).map_err(db_err)?),
    )
    .await?;
    match appended {
        crate::store::Appended::Row(row) => row_to_assertion(&row),
        crate::store::Appended::Taken => {
            let current = current(&mut *db, patient_id, assertion_id).await?.version;
            Err(StoreError::Conflict { current })
        }
    }
}
