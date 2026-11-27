//! Refusals of disclosure that the organisation declined, stored (DR-10): append-only versions.
//!
//! A decline is never changed. The one later version marks it entered in error, with who and why; it stays, and counts
//! for nothing. It is written only for a consent of the patient's that is an active refusal, and only where the tenant's
//! pack allows declining ([`crate::declined_refusal::may_decline`]), and it names the pack's version that stood. **It
//! decides nothing yet**: the refusal still applies.

use crate::decision_ability::State;
use crate::declined_refusal::Declined;
use crate::ids;
use crate::patients_store::{self, StoreError};
use crate::pool::Db;
use serde::Serialize;
use sqlx::postgres::PgRow;
use sqlx::{AssertSqlSafe, Connection, PgPool, Row};

crate::store_errors!("declined_refusals");

/// A declined refusal, as stored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StoredDecline {
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
    /// The consent whose refusal was declined.
    pub consent_id: String,
    /// The version of the tenant's pack that allowed it (`Pack::version`).
    pub pack_version: String,
    /// The decline, as made.
    pub declined: Declined,
}

const COLUMNS: &str = "decline_id::text AS id, patient_id::text AS patient_id, consent_id::text AS consent_id, version, author, state, reason, pack_version, body";

fn row_to_stored(row: &PgRow) -> Result<StoredDecline, StoreError> {
    let body: serde_json::Value = row.try_get("body").map_err(db_err)?;
    let state: String = row.try_get("state").map_err(db_err)?;
    Ok(StoredDecline {
        id: row.try_get("id").map_err(db_err)?,
        patient_id: row.try_get("patient_id").map_err(db_err)?,
        version: row.try_get("version").map_err(db_err)?,
        author: row.try_get("author").map_err(db_err)?,
        state: State::parse(&state).ok_or_else(|| {
            StoreError::Db("an unknown word in declined_refusal_versions.state".into())
        })?,
        reason: row.try_get("reason").map_err(db_err)?,
        consent_id: row.try_get("consent_id").map_err(db_err)?,
        pack_version: row.try_get("pack_version").map_err(db_err)?,
        declined: serde_json::from_value(body).map_err(db_err)?,
    })
}

/// Record that the refusal of `consent_id` was declined, at version 1, under the pack whose version is `pack_version`.
///
/// # Errors
///
/// [`StoreError::NotFound`] for an unknown patient or consent; [`StoreError::BadQuery`] for a consent that is not an active
/// refusal of disclosure; [`StoreError::Db`] when the insert fails.
pub async fn create(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    consent_id: &str,
    pack_version: &str,
    author: &str,
    declined: &Declined,
) -> Result<StoredDecline, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let patient = patients_store::current(&mut *conn, patient_id).await?;
    let consent = crate::consents_store::current(&mut *conn, &patient.id, consent_id).await?;
    if consent.consent.status != crate::consent::Status::Active
        || crate::consent::refusals(&consent.consent).is_none()
    {
        return Err(StoreError::BadQuery(
            "only an active refusal of disclosure can be declined".into(),
        ));
    }
    let row = sqlx::query(AssertSqlSafe(format!(
        "INSERT INTO declined_refusal_versions
                   (decline_id, version, patient_id, consent_id, author, state, ground, pack_version, body)
                 VALUES (gen_random_uuid(), 1, $1::uuid, $2::uuid, $3, 'recorded', $4, $5, $6) RETURNING {COLUMNS}"
    )))
    .bind(patient.id)
    .bind(consent.id)
    .bind(author)
    .bind(declined.ground.as_str())
    .bind(pack_version)
    .bind(serde_json::to_value(declined).map_err(db_err)?)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db_err)?
    .ok_or_else(|| StoreError::Db("insert returned no row".into()))?;
    row_to_stored(&row)
}

/// The declines on a patient record and on every record merged into it, current versions, newest first.
///
/// # Errors
///
/// [`StoreError::Db`] on a database failure.
pub async fn list_with_merged(
    db: &PgPool,
    patient_id: &str,
) -> Result<Vec<StoredDecline>, StoreError> {
    let Some(own) = ids::uuid(patient_id) else {
        return Ok(Vec::new());
    };
    let mut records = crate::links::sources_of(db, &own).await?;
    records.push(own);
    let rows = sqlx::query(AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM declined_refusal_current
                 WHERE patient_id = ANY($1::uuid[])
                 ORDER BY decline_id"
    )))
    .bind(records)
    .fetch_all(db)
    .await
    .map_err(db_err)?;
    let mut all: Vec<StoredDecline> = rows.iter().map(row_to_stored).collect::<Result<_, _>>()?;
    // By the time the record was made, read as a time (an offset in the text would sort wrongly as text).
    all.sort_by_key(|s| std::cmp::Reverse(crate::clock::parse_rfc3339(&s.declined.told_at)));
    Ok(all)
}

/// The current version of one decline on one patient record.
///
/// # Errors
///
/// [`StoreError::NotFound`] when there is no such decline on that record.
pub async fn current(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    decline_id: &str,
) -> Result<StoredDecline, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let (Some(patient_id), Some(decline_id)) = (ids::uuid(patient_id), ids::uuid(decline_id))
    else {
        return Err(StoreError::NotFound);
    };
    let row = sqlx::query(AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM declined_refusal_current
                 WHERE decline_id = $1::uuid AND patient_id = $2::uuid"
    )))
    .bind(decline_id)
    .bind(patient_id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db_err)?
    .ok_or(StoreError::NotFound)?;
    row_to_stored(&row)
}

/// Mark a decline entered in error, as version `expected_version + 1`, with why. The decline itself is kept as made.
///
/// # Errors
///
/// [`StoreError::NotFound`]; [`StoreError::Conflict`] when `expected_version` is not current, including a
/// concurrent writer; [`StoreError::BadQuery`] for a blank or over-long reason, or one already marked.
pub async fn mark_entered_in_error(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    decline_id: &str,
    expected_version: i32,
    author: &str,
    reason: &str,
) -> Result<StoredDecline, StoreError> {
    let reason = reason.trim();
    if reason.is_empty() || reason.chars().count() > crate::declined_refusal::WORDS_LIMIT {
        return Err(StoreError::BadQuery(format!(
            "a reason of 1 to {} characters is required",
            crate::declined_refusal::WORDS_LIMIT
        )));
    }
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let mut tx = conn.begin().await.map_err(db_err)?;
    let db = &mut *tx;
    let now = current(&mut *db, patient_id, decline_id).await?;
    if now.version != expected_version {
        return Err(StoreError::Conflict {
            current: now.version,
        });
    }
    if now.state == State::EnteredInError {
        return Err(StoreError::BadQuery(
            "this decline is already marked entered in error".into(),
        ));
    }
    let new_version = crate::store::next_version(expected_version)?;
    let row = match crate::store::append_version(
        &mut *db,
        STORE,
        sqlx::query(AssertSqlSafe(format!(
            "INSERT INTO declined_refusal_versions
                   (decline_id, version, patient_id, consent_id, author, reason, state, ground, pack_version, body)
                 VALUES ($1::uuid, $2, $3::uuid, $4::uuid, $5, $6, 'entered-in-error', $7, $8, $9) RETURNING {COLUMNS}"
        )))
        .bind(now.id.clone())
        .bind(new_version)
        .bind(now.patient_id.clone())
        .bind(now.consent_id.clone())
        .bind(author)
        .bind(reason)
        .bind(now.declined.ground.as_str())
        .bind(now.pack_version.clone())
        .bind(serde_json::to_value(&now.declined).map_err(db_err)?),
    )
    .await?
    {
        crate::store::Appended::Row(row) => row,
        crate::store::Appended::Taken => {
            let current = current(&mut *db, patient_id, decline_id).await?.version;
            return Err(StoreError::Conflict { current });
        }
    };
    let stored = row_to_stored(&row)?;
    tx.commit().await.map_err(db_err)?;
    Ok(stored)
}
