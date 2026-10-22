//! Consent storage (task SE-2): append-only versions, optimistic updates.
//!
//! The same pattern as `patients_store`: a change is a new version row, never an update; a stale
//! `expected_version`, or a concurrent writer that got there first, is a
//! [`StoreError::Conflict`]. Only the *current* version of each consent counts for a decision.

use crate::consent::ConsentRecord;
use crate::ids;
use crate::patients_store::{self, StoreError};
use crate::store::pg;
use sea_orm::{ConnectionTrait, DatabaseConnection, TransactionTrait};
use serde::Serialize;
use std::collections::HashMap;

crate::store_errors!("consents");

/// A consent as stored: the record and its place in the history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StoredConsent {
    /// The server-assigned id (a UUID), the same for every version.
    pub id: String,
    /// The patient it belongs to.
    pub patient_id: String,
    /// 1 for the first version.
    pub version: i32,
    /// Who wrote this version.
    pub author: String,
    /// The record.
    pub consent: ConsentRecord,
}

const COLUMNS: &str =
    "consent_id::text AS id, patient_id::text AS patient_id, version, author, body";

fn row_to_stored(row: &sea_orm::QueryResult) -> Result<StoredConsent, StoreError> {
    let body: serde_json::Value = row.try_get("", "body").map_err(db_err)?;
    Ok(StoredConsent {
        id: row.try_get("", "id").map_err(db_err)?,
        patient_id: row.try_get("", "patient_id").map_err(db_err)?,
        version: row.try_get("", "version").map_err(db_err)?,
        author: row.try_get("", "author").map_err(db_err)?,
        consent: serde_json::from_value(body).map_err(db_err)?,
    })
}

/// Record a new consent for a patient, at version 1.
///
/// # Errors
///
/// [`StoreError::NotFound`] when the patient does not exist; [`StoreError::Db`] when the insert
/// fails.
pub async fn create(
    db: &impl ConnectionTrait,
    patient_id: &str,
    author: &str,
    reason: Option<&str>,
    record: &ConsentRecord,
) -> Result<StoredConsent, StoreError> {
    let patient = patients_store::current(db, patient_id).await?;
    let row = sea_orm::ConnectionTrait::query_one_raw(
        db,
        pg(
            &format!(
                "INSERT INTO consent_versions
                   (consent_id, version, patient_id, author, reason, status, body)
                 VALUES (gen_random_uuid(), 1, $1::uuid, $2, $3, $4, $5) RETURNING {COLUMNS}"
            ),
            vec![
                patient.id.into(),
                author.into(),
                reason.map(str::to_string).into(),
                record.status.as_str().into(),
                serde_json::to_value(record).map_err(db_err)?.into(),
            ],
        ),
    )
    .await
    .map_err(db_err)?
    .ok_or_else(|| StoreError::Db("insert returned no row".into()))?;
    row_to_stored(&row)
}

/// The current version of every consent of a patient, withdrawn ones included, oldest first.
///
/// # Errors
///
/// [`StoreError::Db`] on a database failure.
pub async fn list(
    db: &DatabaseConnection,
    patient_id: &str,
) -> Result<Vec<StoredConsent>, StoreError> {
    let Some(patient_id) = ids::uuid(patient_id) else {
        return Ok(Vec::new());
    };
    let rows = sea_orm::ConnectionTrait::query_all_raw(
        db,
        pg(
            &format!(
                "SELECT {COLUMNS} FROM consent_current
                 WHERE patient_id = $1::uuid ORDER BY recorded_at, consent_id"
            ),
            vec![patient_id.into()],
        ),
    )
    .await
    .map_err(db_err)?;
    rows.iter().map(row_to_stored).collect()
}

/// The current, **active** consents of each of `patient_ids`, keyed by patient id. A patient with
/// none has no entry. One query, however many patients.
///
/// # Errors
///
/// [`StoreError::Db`] on a database failure.
pub async fn active_for(
    db: &impl ConnectionTrait,
    patient_ids: &[String],
) -> Result<HashMap<String, Vec<StoredConsent>>, StoreError> {
    let mut by_patient: HashMap<String, Vec<StoredConsent>> = HashMap::new();
    let patient_ids = ids::uuids(patient_ids);
    if patient_ids.is_empty() {
        return Ok(by_patient);
    }
    let rows = sea_orm::ConnectionTrait::query_all_raw(
        db,
        pg(
            &format!(
                "SELECT {COLUMNS} FROM consent_current
                 WHERE status = 'active' AND patient_id = ANY($1::uuid[])
                 ORDER BY consent_id"
            ),
            vec![patient_ids.into()],
        ),
    )
    .await
    .map_err(db_err)?;
    for row in &rows {
        let stored = row_to_stored(row)?;
        by_patient
            .entry(stored.patient_id.clone())
            .or_default()
            .push(stored);
    }
    Ok(by_patient)
}

/// The active consents that apply to each of `patient_ids`: its own and those of every record merged
/// into it (a person's refusal does not lapse because two records were joined). Two queries however
/// many patients (CQ-10). A patient with neither has no entry; one with merged-in records but no
/// consent has an empty entry. Keys are lower case.
///
/// # Errors
///
/// [`StoreError::Db`] on a database failure.
pub async fn active_with_merged(
    db: &impl ConnectionTrait,
    patient_ids: &[String],
) -> Result<HashMap<String, Vec<StoredConsent>>, StoreError> {
    let ids = ids::uuids(patient_ids);
    let sources = crate::links::sources_of_many(db, &ids).await?;
    let mut all = ids.clone();
    all.extend(sources.values().flatten().cloned());
    all.sort();
    all.dedup();
    let consents = active_for(db, &all).await?;
    let mut out = HashMap::new();
    for id in &ids {
        let own = consents.get(id).cloned();
        let merged = sources.get(id);
        if own.is_none() && merged.is_none() {
            continue;
        }
        let mut list = own.unwrap_or_default();
        for source in merged.into_iter().flatten() {
            list.extend(consents.get(source).cloned().unwrap_or_default());
        }
        out.insert(id.clone(), list);
    }
    Ok(out)
}

/// The current version of one consent of one patient.
///
/// # Errors
///
/// [`StoreError::NotFound`] when there is no such consent for that patient.
pub async fn current(
    db: &impl ConnectionTrait,
    patient_id: &str,
    consent_id: &str,
) -> Result<StoredConsent, StoreError> {
    let (Some(patient_id), Some(consent_id)) = (ids::uuid(patient_id), ids::uuid(consent_id))
    else {
        return Err(StoreError::NotFound);
    };
    let row = sea_orm::ConnectionTrait::query_one_raw(
        db,
        pg(
            &format!(
                "SELECT {COLUMNS} FROM consent_current
                 WHERE consent_id = $1::uuid AND patient_id = $2::uuid"
            ),
            vec![consent_id.into(), patient_id.into()],
        ),
    )
    .await
    .map_err(db_err)?
    .ok_or(StoreError::NotFound)?;
    row_to_stored(&row)
}

/// Append version `expected_version + 1`. Earlier versions stay. Withdrawing a consent is this
/// call with `status: inactive`.
///
/// # Errors
///
/// [`StoreError::NotFound`] for an unknown consent or one belonging to another patient;
/// [`StoreError::Conflict`] when `expected_version` is not the current version, including when a
/// concurrent writer got there first.
pub async fn update(
    db: &(impl ConnectionTrait + TransactionTrait),
    patient_id: &str,
    consent_id: &str,
    expected_version: i32,
    author: &str,
    reason: Option<&str>,
    record: &ConsentRecord,
) -> Result<StoredConsent, StoreError> {
    let now = current(db, patient_id, consent_id).await?;
    if now.version != expected_version {
        return Err(StoreError::Conflict {
            current: now.version,
        });
    }
    // A concurrent writer took this version: the primary key refuses the second (CQ-12).
    let row = match crate::store::append_version(
        db,
        STORE,
        pg(
            &format!(
                "INSERT INTO consent_versions
                   (consent_id, version, patient_id, author, reason, status, body)
                 VALUES ($1::uuid, $2, $3::uuid, $4, $5, $6, $7) RETURNING {COLUMNS}"
            ),
            vec![
                now.id.clone().into(),
                (expected_version + 1).into(),
                now.patient_id.clone().into(),
                author.into(),
                reason.map(str::to_string).into(),
                record.status.as_str().into(),
                serde_json::to_value(record).map_err(db_err)?.into(),
            ],
        ),
    )
    .await?
    {
        crate::store::Appended::Row(row) => row,
        crate::store::Appended::Taken => {
            let current = current(db, patient_id, consent_id).await?.version;
            return Err(StoreError::Conflict { current });
        }
    };
    row_to_stored(&row)
}
