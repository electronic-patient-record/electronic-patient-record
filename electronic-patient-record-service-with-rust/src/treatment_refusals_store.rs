//! Refusals of treatment, stored (RF-5): append-only versions, optimistic updates.
//!
//! The same pattern as `consents_store`, for a different thing: this is a refusal of **treatment**, and
//! `consents_store` holds a refusal of disclosure (RF-2). A change is a new version row; a stale `expected_version` is a
//! [`StoreError::Conflict`]. Nothing here says whether a person may be treated or whether a refusal is valid
//! ([`crate::treatment_refusal`] has no such function), and "none recorded" is never "none".
//!
//! **A merge and an unmerge stop for each refusal** ([`unseen_on_merge`], [`unseen_on_unmerge`]): the caller must name
//! every one that a person looked at, and a refusal the person did not name is a refusal that was not asked about. The
//! refusals of a record merged away are shown with the survivor ([`list_with_merged`]), as what they are: another
//! record's.

use crate::ids;
use crate::patients_store::{self, StoreError};
use crate::pool::Db;
use crate::treatment_refusal::{self, RefusalRecord, State};
use serde::Serialize;
use sqlx::postgres::PgRow;
use sqlx::{AssertSqlSafe, Connection, PgPool, Row};

crate::store_errors!("treatment_refusals");

/// A refusal as stored: the record and its place in the history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StoredRefusal {
    /// The server-assigned id (a UUID), the same for every version.
    pub id: String,
    /// The patient record it is recorded on.
    pub patient_id: String,
    /// 1 for the first version.
    pub version: i32,
    /// Who wrote this version.
    pub author: String,
    /// The record.
    pub refusal: RefusalRecord,
}

const COLUMNS: &str =
    "refusal_id::text AS id, patient_id::text AS patient_id, version, author, body";

const ENDED: &str = "a refusal entered in error is not changed: record a new one";
const WITHDRAWN_NEEDS_A_RULE: &str = "no rule in force says that this withdrawal ends a refusal: record it as doubted, with the withdrawal";

fn row_to_stored(row: &PgRow) -> Result<StoredRefusal, StoreError> {
    let body: serde_json::Value = row.try_get("body").map_err(db_err)?;
    Ok(StoredRefusal {
        id: row.try_get("id").map_err(db_err)?,
        patient_id: row.try_get("patient_id").map_err(db_err)?,
        version: row.try_get("version").map_err(db_err)?,
        author: row.try_get("author").map_err(db_err)?,
        refusal: serde_json::from_value(body).map_err(db_err)?,
    })
}

/// The words of a unit variant, as serde writes them.
fn word<T: Serialize>(value: &T) -> Result<String, StoreError> {
    match serde_json::to_value(value).map_err(db_err)? {
        serde_json::Value::String(s) => Ok(s),
        _ => Err(StoreError::Db("a stored word was not a word".into())),
    }
}

/// A withdrawal that was heard ends a refusal only where a rule in force says so ([`treatment_refusal::after_withdrawal`]).
/// No rule is loaded by the service yet, so a refusal is never stored as withdrawn.
fn check_withdrawal(record: &RefusalRecord) -> Result<(), StoreError> {
    if record.state == State::Withdrawn
        && let Some(w) = &record.withdrawal
        && treatment_refusal::after_withdrawal(w.how, None) != State::Withdrawn
    {
        return Err(StoreError::BadQuery(WITHDRAWN_NEEDS_A_RULE.into()));
    }
    Ok(())
}

/// Record a new refusal for a patient, at version 1.
///
/// # Errors
///
/// [`StoreError::NotFound`] when the patient does not exist; [`StoreError::BadQuery`] for a withdrawal no rule lets end
/// it, or one recorded as entered in error to begin with; [`StoreError::Db`] when the insert fails.
pub async fn create(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    author: &str,
    reason: Option<&str>,
    record: &RefusalRecord,
) -> Result<StoredRefusal, StoreError> {
    check_withdrawal(record)?;
    if record.state == State::EnteredInError {
        return Err(StoreError::BadQuery(
            "a refusal is not recorded as entered in error: it is marked so afterwards".into(),
        ));
    }
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let patient = patients_store::current(&mut *conn, patient_id).await?;
    let row = sqlx::query(AssertSqlSafe(format!(
        "INSERT INTO treatment_refusal_versions
                   (refusal_id, version, patient_id, author, reason, state, kind, identity, body)
                 VALUES (gen_random_uuid(), 1, $1::uuid, $2, $3, $4, $5, $6, $7) RETURNING {COLUMNS}"
    )))
    .bind(patient.id)
    .bind(author)
    .bind(reason.map(str::to_string))
    .bind(record.state.as_str())
    .bind(word(&record.kind)?)
    .bind(word(&record.identity)?)
    .bind(serde_json::to_value(record).map_err(db_err)?)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db_err)?
    .ok_or_else(|| StoreError::Db("insert returned no row".into()))?;
    row_to_stored(&row)
}

/// The current version of every refusal recorded on a patient record, those entered in error and withdrawn included,
/// oldest first.
///
/// # Errors
///
/// [`StoreError::Db`] on a database failure.
pub async fn list(db: &PgPool, patient_id: &str) -> Result<Vec<StoredRefusal>, StoreError> {
    let Some(patient_id) = ids::uuid(patient_id) else {
        return Ok(Vec::new());
    };
    let rows = sqlx::query(AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM treatment_refusal_current
                 WHERE patient_id = $1::uuid ORDER BY recorded_at, refusal_id"
    )))
    .bind(patient_id)
    .fetch_all(db)
    .await
    .map_err(db_err)?;
    rows.iter().map(row_to_stored).collect()
}

/// The refusals of a patient record and of every record merged into it. A refusal does not lapse because two records
/// were joined; its `patient_id` says whose record it is on.
///
/// # Errors
///
/// [`StoreError::Db`] on a database failure.
pub async fn list_with_merged(
    db: &PgPool,
    patient_id: &str,
) -> Result<Vec<StoredRefusal>, StoreError> {
    let mut all = list(db, patient_id).await?;
    for source in crate::links::sources_of(db, patient_id).await? {
        all.extend(list(db, &source).await?);
    }
    Ok(all)
}

/// The current version of one refusal of one patient record.
///
/// # Errors
///
/// [`StoreError::NotFound`] when there is no such refusal on that record.
pub async fn current(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    refusal_id: &str,
) -> Result<StoredRefusal, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let (Some(patient_id), Some(refusal_id)) = (ids::uuid(patient_id), ids::uuid(refusal_id))
    else {
        return Err(StoreError::NotFound);
    };
    let row = sqlx::query(AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM treatment_refusal_current
                 WHERE refusal_id = $1::uuid AND patient_id = $2::uuid"
    )))
    .bind(refusal_id)
    .bind(patient_id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db_err)?
    .ok_or(StoreError::NotFound)?;
    row_to_stored(&row)
}

/// Append version `expected_version + 1`. Earlier versions stay. A doubt, a withdrawal that was heard and "entered in
/// error" are this call with another state.
///
/// # Errors
///
/// [`StoreError::NotFound`] for an unknown refusal or one on another record; [`StoreError::Conflict`] when
/// `expected_version` is not the current version, including a concurrent writer; [`StoreError::BadQuery`] for one
/// already entered in error, or a withdrawal no rule lets end it.
pub async fn update(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    refusal_id: &str,
    expected_version: i32,
    author: &str,
    reason: Option<&str>,
    record: &RefusalRecord,
) -> Result<StoredRefusal, StoreError> {
    check_withdrawal(record)?;
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let mut tx = conn.begin().await.map_err(db_err)?;
    crate::store::one_at_a_time(&mut tx, STORE, &patient_id.to_ascii_lowercase()).await?;
    let db = &mut *tx;
    let now = current(&mut *db, patient_id, refusal_id).await?;
    if now.version != expected_version {
        return Err(StoreError::Conflict {
            current: now.version,
        });
    }
    if now.refusal.state == State::EnteredInError {
        return Err(StoreError::BadQuery(ENDED.into()));
    }
    let new_version = crate::store::next_version(expected_version)?;
    let row = match crate::store::append_version(
        &mut *db,
        STORE,
        sqlx::query(AssertSqlSafe(format!(
            "INSERT INTO treatment_refusal_versions
                   (refusal_id, version, patient_id, author, reason, state, kind, identity, body)
                 VALUES ($1::uuid, $2, $3::uuid, $4, $5, $6, $7, $8, $9) RETURNING {COLUMNS}"
        )))
        .bind(now.id.clone())
        .bind(new_version)
        .bind(now.patient_id.clone())
        .bind(author)
        .bind(reason.map(str::to_string))
        .bind(record.state.as_str())
        .bind(word(&record.kind)?)
        .bind(word(&record.identity)?)
        .bind(serde_json::to_value(record).map_err(db_err)?),
    )
    .await?
    {
        crate::store::Appended::Row(row) => row,
        crate::store::Appended::Taken => {
            let current = current(&mut *db, patient_id, refusal_id).await?.version;
            return Err(StoreError::Conflict { current });
        }
    };
    let stored = row_to_stored(&row)?;
    tx.commit().await.map_err(db_err)?;
    Ok(stored)
}

/// The refusals recorded on `source`, not entered in error, that `seen` does not name: what a merge of `source` into
/// another record has not yet asked a person about. Withdrawn ones are asked about too, because whether a withdrawal ended
/// a refusal is the place's rule and not the software's.
///
/// # Errors
///
/// [`StoreError::Db`] on a database failure.
pub async fn unseen_on_merge(
    db: impl Into<Db<'_>>,
    source: &str,
    seen: &[String],
) -> Result<Vec<String>, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let Some(source) = ids::uuid(source) else {
        return Ok(Vec::new());
    };
    let seen = ids::uuids(seen);
    let rows = sqlx::query(
        "SELECT refusal_id::text AS id FROM treatment_refusal_current
                 WHERE patient_id = $1::uuid AND state <> 'entered-in-error' AND NOT (refusal_id = ANY($2::uuid[]))
                 ORDER BY recorded_at, refusal_id",
    )
    .bind(source)
    .bind(seen)
    .fetch_all(&mut *conn)
    .await
    .map_err(db_err)?;
    rows.iter()
        .map(|r| r.try_get("id").map_err(db_err))
        .collect()
}

/// The refusals recorded on `target` **since** `source` was merged into it, not entered in error, that `seen` does not
/// name: any of them may be the other person's. Empty when `source` is not merged into `target` (the unmerge then fails
/// on its own).
///
/// # Errors
///
/// [`StoreError::Db`] on a database failure.
pub async fn unseen_on_unmerge(
    db: impl Into<Db<'_>>,
    target: &str,
    source: &str,
    seen: &[String],
) -> Result<Vec<String>, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let (Some(target), Some(source)) = (ids::uuid(target), ids::uuid(source)) else {
        return Ok(Vec::new());
    };
    let seen = ids::uuids(seen);
    let rows = sqlx::query(
        "SELECT c.refusal_id::text AS id FROM treatment_refusal_current c
                 WHERE c.patient_id = $1::uuid AND c.state <> 'entered-in-error' AND NOT (c.refusal_id = ANY($3::uuid[]))
                   AND (SELECT min(v.recorded_at) FROM treatment_refusal_versions v WHERE v.refusal_id = c.refusal_id)
                     > (SELECT min(l.recorded_at) FROM patient_link_versions l
                         WHERE l.link_id IN (SELECT link_id FROM patient_link_current
                                              WHERE source_patient_id = $2::uuid AND target_patient_id = $1::uuid
                                                AND status = 'active'))
                 ORDER BY c.recorded_at, c.refusal_id",
    )
    .bind(target)
    .bind(source)
    .bind(seen)
    .fetch_all(&mut *conn)
    .await
    .map_err(db_err)?;
    rows.iter()
        .map(|r| r.try_get("id").map_err(db_err))
        .collect()
}
