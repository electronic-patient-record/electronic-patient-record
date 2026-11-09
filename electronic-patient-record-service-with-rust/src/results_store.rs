//! Results and sign-offs (OR-4 first slice, WL-1): the append-only store behind [`crate::results`].
//!
//! Reads follow a merge by provenance, as for adjustments: a result filed on a record merged away
//! counts for the surviving record. The routes are in `controllers/results.rs`.

use crate::ids;
use crate::links;
use crate::patients_store::{self, StoreError};
use crate::pool::Db;
use crate::results::{self, Refusal, ResultRecord, SignOff, Status};
use serde::Serialize;
use sqlx::postgres::PgRow;
use sqlx::{AssertSqlSafe, Row};

crate::store_errors!("results");

/// One version of a result as stored.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StoredResult {
    /// Server id, the same for every version.
    pub id: String,
    /// The patient it was filed on.
    pub patient_id: String,
    /// 1 when first filed.
    pub version: i32,
    /// Who filed this version (a feed or a person).
    pub author: String,
    /// When this version was filed (UTC).
    pub recorded_at: String,
    /// Why it changed; absent on a first version.
    pub reason: Option<String>,
    /// The result at this version.
    pub result: ResultRecord,
}

/// A sign-off: who signed which version, and when.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StoredSignOff {
    /// Server id.
    pub id: String,
    /// The result.
    pub result_id: String,
    /// The version that was signed off.
    pub version: i32,
    /// Who signed it off.
    pub signed_by: String,
    /// When (UTC).
    pub signed_at: String,
}

/// A result as a reader sees it: the current version, where it stands against sign-off, and, when it
/// has changed since a sign-off, the version that was signed, so both are shown.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResultView {
    /// The current version.
    pub current: StoredResult,
    /// Unsigned, signed, changed since sign-off, or not applicable.
    pub sign_off: SignOff,
    /// The latest sign-off, of any version.
    pub last_sign_off: Option<StoredSignOff>,
    /// The version that was signed, when the result has changed since: what the signer saw.
    pub signed_version: Option<StoredResult>,
}

const COLUMNS: &str =
    "result_id::text AS id, patient_id::text AS patient_id, version, author, reason,
    to_char(recorded_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"') AS at, body";

const SIGN_COLUMNS: &str = "signoff_id::text AS id, result_id::text AS result_id, result_version,
    signed_by, to_char(recorded_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"') AS at";

fn row_to_stored(row: &PgRow) -> Result<StoredResult, StoreError> {
    let body: serde_json::Value = row.try_get("body").map_err(db_err)?;
    Ok(StoredResult {
        id: row.try_get("id").map_err(db_err)?,
        patient_id: row.try_get("patient_id").map_err(db_err)?,
        version: row.try_get("version").map_err(db_err)?,
        author: row.try_get("author").map_err(db_err)?,
        recorded_at: row.try_get("at").map_err(db_err)?,
        reason: row.try_get("reason").map_err(db_err)?,
        result: serde_json::from_value(body).map_err(db_err)?,
    })
}

fn row_to_sign_off(row: &PgRow) -> Result<StoredSignOff, StoreError> {
    Ok(StoredSignOff {
        id: row.try_get("id").map_err(db_err)?,
        result_id: row.try_get("result_id").map_err(db_err)?,
        version: row.try_get("result_version").map_err(db_err)?,
        signed_by: row.try_get("signed_by").map_err(db_err)?,
        signed_at: row.try_get("at").map_err(db_err)?,
    })
}

/// A refusal as the writer's to fix.
pub(crate) fn refused(r: Refusal) -> StoreError {
    StoreError::BadQuery(r.words())
}

fn check(record: &ResultRecord) -> Result<(), StoreError> {
    match results::problems(record).first() {
        Some(r) => Err(refused(*r)),
        None => Ok(()),
    }
}

/// File a new result at version 1.
///
/// # Errors
///
/// [`StoreError::NotFound`] (no such patient), [`StoreError::BadQuery`] (the record was refused).
pub async fn create(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    author: &str,
    record: &ResultRecord,
) -> Result<StoredResult, StoreError> {
    check(record)?;
    if let Some(r) = results::first_refusal(record.status) {
        return Err(refused(r));
    }
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let patient = patients_store::current(&mut *db, patient_id).await?;
    let row = sqlx::query(AssertSqlSafe(format!(
        "INSERT INTO result_versions (result_id, version, patient_id, author, status, body)
         VALUES (gen_random_uuid(), 1, $1::uuid, $2, $3, $4)
         RETURNING {COLUMNS}"
    )))
    .bind(patient.id)
    .bind(author.to_string())
    .bind(record.status.word())
    .bind(serde_json::to_value(record).map_err(db_err)?)
    .fetch_optional(&mut *db)
    .await
    .map_err(db_err)?
    .ok_or_else(|| StoreError::Db("insert returned no row".into()))?;
    row_to_stored(&row)
}

/// One result's current version, on this patient or a record merged into it.
///
/// # Errors
///
/// [`StoreError::NotFound`] when there is no such result for that patient.
pub async fn current(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    result_id: &str,
) -> Result<StoredResult, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let result_id = ids::uuid(result_id).ok_or(StoreError::NotFound)?;
    let linked = ids::uuids(&links::linked_ids(&mut *db, patient_id).await?);
    let row = sqlx::query(AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM result_current
         WHERE result_id = $1::uuid AND patient_id = ANY($2::uuid[])"
    )))
    .bind(result_id)
    .bind(linked)
    .fetch_optional(&mut *db)
    .await
    .map_err(db_err)?
    .ok_or(StoreError::NotFound)?;
    row_to_stored(&row)
}

/// Append the next version of a result: amended, corrected, final after preliminary, or entered in
/// error. Any sign-off stays on the version it named, so the result reads as changed since sign-off.
///
/// # Errors
///
/// [`StoreError::NotFound`], [`StoreError::Conflict`] (stale or racing version),
/// [`StoreError::BadQuery`] (the change was refused).
pub async fn update(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    result_id: &str,
    expected_version: i32,
    author: &str,
    reason: Option<&str>,
    record: &ResultRecord,
) -> Result<StoredResult, StoreError> {
    check(record)?;
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let was = current(&mut *db, patient_id, result_id).await?;
    if was.version != expected_version {
        return Err(StoreError::Conflict {
            current: was.version,
        });
    }
    if let Some(r) = results::change_refusal(was.result.status, record.status, reason) {
        return Err(refused(r));
    }
    let appended = crate::store::append_version(
        &mut *db,
        STORE,
        sqlx::query(AssertSqlSafe(format!(
            "INSERT INTO result_versions (result_id, version, patient_id, author, status, reason, body)
             VALUES ($1::uuid, $2, $3::uuid, $4, $5, $6, $7)
             RETURNING {COLUMNS}"
        )))
        .bind(was.id.clone())
        .bind(expected_version + 1)
        .bind(was.patient_id.clone())
        .bind(author.to_string())
        .bind(record.status.word())
        .bind(reason.map(str::trim).map(str::to_string))
        .bind(serde_json::to_value(record).map_err(db_err)?),
    )
    .await?;
    match appended {
        crate::store::Appended::Row(row) => row_to_stored(&row),
        crate::store::Appended::Taken => {
            let current = current(&mut *db, patient_id, result_id).await?.version;
            Err(StoreError::Conflict { current })
        }
    }
}

/// One version of a result, by number.
async fn version_of(
    db: &mut sqlx::PgConnection,
    result_id: &str,
    version: i32,
) -> Result<StoredResult, StoreError> {
    let row = sqlx::query(AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM result_versions WHERE result_id = $1::uuid AND version = $2"
    )))
    .bind(result_id.to_string())
    .bind(version)
    .fetch_optional(&mut *db)
    .await
    .map_err(db_err)?
    .ok_or(StoreError::NotFound)?;
    row_to_stored(&row)
}

/// Every result for a patient (and records merged into it) at its current version, newest filed
/// first, each with where it stands against sign-off.
///
/// # Errors
///
/// [`StoreError::NotFound`] (no such patient).
pub async fn list(db: impl Into<Db<'_>>, patient_id: &str) -> Result<Vec<ResultView>, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let patient = patients_store::current(&mut *db, patient_id).await?;
    let linked = ids::uuids(&links::linked_ids(&mut *db, &patient.id).await?);
    let rows = sqlx::query(AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM result_current
         WHERE patient_id = ANY($1::uuid[])
         ORDER BY recorded_at DESC, result_id"
    )))
    .bind(linked.clone())
    .fetch_all(&mut *db)
    .await
    .map_err(db_err)?;
    let currents = rows
        .iter()
        .map(row_to_stored)
        .collect::<Result<Vec<_>, _>>()?;
    let sign_rows = sqlx::query(AssertSqlSafe(format!(
        "SELECT {SIGN_COLUMNS} FROM result_signoffs
         WHERE patient_id = ANY($1::uuid[])
         ORDER BY result_version"
    )))
    .bind(linked)
    .fetch_all(&mut *db)
    .await
    .map_err(db_err)?;
    let sign_offs = sign_rows
        .iter()
        .map(row_to_sign_off)
        .collect::<Result<Vec<_>, _>>()?;
    let mut out = Vec::with_capacity(currents.len());
    for current in currents {
        let mine: Vec<&StoredSignOff> = sign_offs
            .iter()
            .filter(|s| s.result_id == current.id)
            .collect();
        let versions: Vec<i32> = mine.iter().map(|s| s.version).collect();
        let state = results::sign_off_state(current.version, current.result.status, &versions);
        let last = mine.last().map(|s| (*s).clone());
        let signed_version = match (&state, &last) {
            (SignOff::ChangedSinceSignOff, Some(s)) => {
                Some(version_of(&mut *db, &current.id, s.version).await?)
            }
            _ => None,
        };
        out.push(ResultView {
            current,
            sign_off: state,
            last_sign_off: last,
            signed_version,
        });
    }
    Ok(out)
}

/// Sign off `version` of a result. Only the current version can be signed: an older one is a
/// [`StoreError::Conflict`] naming the current, because the signer has not seen the change. Signing a
/// version that is already signed returns that sign-off and `false`.
///
/// # Errors
///
/// [`StoreError::NotFound`], [`StoreError::Conflict`], [`StoreError::BadQuery`] (entered in error).
pub async fn sign_off(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    result_id: &str,
    version: i32,
    signer: &str,
) -> Result<(StoredSignOff, bool), StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let current = current(&mut *db, patient_id, result_id).await?;
    if current.version != version {
        return Err(StoreError::Conflict {
            current: current.version,
        });
    }
    if current.result.status == Status::EnteredInError {
        return Err(refused(Refusal::NothingToSign));
    }
    let inserted = sqlx::query(AssertSqlSafe(format!(
        "INSERT INTO result_signoffs (result_id, result_version, patient_id, signed_by)
         VALUES ($1::uuid, $2, $3::uuid, $4)
         ON CONFLICT (result_id, result_version) DO NOTHING
         RETURNING {SIGN_COLUMNS}"
    )))
    .bind(current.id.clone())
    .bind(version)
    .bind(current.patient_id.clone())
    .bind(signer.to_string())
    .fetch_optional(&mut *db)
    .await
    .map_err(db_err)?;
    if let Some(row) = inserted {
        return Ok((row_to_sign_off(&row)?, true));
    }
    let row = sqlx::query(AssertSqlSafe(format!(
        "SELECT {SIGN_COLUMNS} FROM result_signoffs WHERE result_id = $1::uuid AND result_version = $2"
    )))
    .bind(current.id)
    .bind(version)
    .fetch_one(&mut *db)
    .await
    .map_err(db_err)?;
    Ok((row_to_sign_off(&row)?, false))
}
