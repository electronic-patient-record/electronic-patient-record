//! Amendment requests (RC-4): the append-only store behind the rules in [`crate::amendment`]. Every
//! step is a new version of the request with the event that caused it; nothing is edited. A request
//! names a record and the version the patient read; it can only be applied when that record really
//! has a later version.
//!
//! Reads follow a merge by provenance, as the records they are about do.

use crate::amendment::{Event, Notice, Refusal, Request, Target};
use crate::ids;
use crate::links;
use crate::patients_store::{self, StoreError};
use crate::store::pg;
use chrono::{DateTime, SecondsFormat, Utc};
use sea_orm::{ConnectionTrait, TransactionTrait};
use serde::Serialize;

crate::store_errors!("amendments");

/// The kinds of record a request may be about (those that exist and are versioned).
pub const KINDS: [&str; 2] = ["Observation", "AllergyIntolerance"];

/// One request as stored.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StoredAmendment {
    /// Server id, the same for every version.
    pub id: String,
    /// The patient the request was made on.
    pub patient_id: String,
    /// 1 for the request as opened.
    pub version: i32,
    /// Who wrote this version.
    pub author: String,
    /// When this version was written (UTC).
    pub recorded_at: String,
    /// The request after this version.
    pub request: Request,
}

const TS: &str = "to_char(recorded_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"')";

fn columns() -> String {
    format!(
        "amendment_id::text AS id, patient_id::text AS patient_id, version, author, {TS} AS at, body"
    )
}

fn row_to_amendment(row: &sea_orm::QueryResult) -> Result<StoredAmendment, StoreError> {
    let body: serde_json::Value = row.try_get("", "body").map_err(db_err)?;
    Ok(StoredAmendment {
        id: row.try_get("", "id").map_err(db_err)?,
        patient_id: row.try_get("", "patient_id").map_err(db_err)?,
        version: row.try_get("", "version").map_err(db_err)?,
        author: row.try_get("", "author").map_err(db_err)?,
        recorded_at: row.try_get("", "at").map_err(db_err)?,
        request: serde_json::from_value(body).map_err(db_err)?,
    })
}

fn refused(r: Refusal) -> StoreError {
    StoreError::BadQuery(match r {
        Refusal::WrongState => "the request is not in a state that allows this".to_string(),
        Refusal::Blank(what) => format!("the {what} must not be blank"),
        Refusal::SelfDecision => "a request cannot be decided by the person who made it".into(),
        Refusal::NotTheRequester => "only the person who made the request may do this".into(),
        Refusal::NotALaterVersion => {
            "the correction must be recorded as a later version than the one disputed".into()
        }
        Refusal::StatementTooLong => "the statement is too long".into(),
        Refusal::AlreadyDisagreed => "a statement of disagreement is already recorded".into(),
        Refusal::AlreadyExtended => "the extension was already taken".into(),
    })
}

/// The current version number of the record a request is about, or `NotFound`.
async fn target_version(
    db: &impl ConnectionTrait,
    patient_id: &str,
    kind: &str,
    id: &str,
) -> Result<i32, StoreError> {
    match kind {
        "Observation" => Ok(crate::observations_store::current(db, patient_id, id)
            .await?
            .version),
        "AllergyIntolerance" => Ok(crate::allergies::current(db, patient_id, id).await?.version),
        other => Err(StoreError::BadQuery(format!(
            "an amendment cannot be requested for {other}"
        ))),
    }
}

/// Open a request about `kind`/`target_id`, made against `version` of it.
///
/// # Errors
///
/// [`StoreError::NotFound`] (no such patient or record), [`StoreError::BadQuery`] (a kind that cannot
/// be amended, a version the record does not have, a blank text).
#[allow(clippy::too_many_arguments)]
pub async fn open(
    db: &impl ConnectionTrait,
    patient_id: &str,
    requester: &str,
    kind: &str,
    target_id: &str,
    version: i32,
    text: &str,
    now: DateTime<Utc>,
) -> Result<StoredAmendment, StoreError> {
    let patient = patients_store::current(db, patient_id).await?;
    let current = target_version(db, &patient.id, kind, target_id).await?;
    if version < 1 || version > current {
        return Err(StoreError::BadQuery(format!(
            "the record has no version {version}"
        )));
    }
    let request = Request::new(
        Target {
            kind: kind.to_string(),
            id: target_id.to_string(),
            version,
        },
        requester,
        text,
        now,
    )
    .map_err(refused)?;
    let row = db
        .query_one_raw(pg(
            &format!(
                "INSERT INTO amendment_versions
                   (amendment_id, version, patient_id, author, state, target_kind, target_id, event, body)
                 VALUES (gen_random_uuid(), 1, $1::uuid, $2, $3, $4, $5::uuid, $6, $7)
                 RETURNING {}",
                columns()
            ),
            vec![
                patient.id.into(),
                requester.into(),
                request.state.word().into(),
                kind.into(),
                target_id.into(),
                serde_json::json!({ "event": "open", "at": now.to_rfc3339_opts(SecondsFormat::Micros, true) }).into(),
                serde_json::to_value(&request).map_err(db_err)?.into(),
            ],
        ))
        .await
        .map_err(db_err)?
        .ok_or_else(|| StoreError::Db("insert returned no row".into()))?;
    row_to_amendment(&row)
}

/// Every request of a patient, newest first, including those made on a record merged into this one.
///
/// # Errors
///
/// [`StoreError::NotFound`] (no such patient).
pub async fn list(
    db: &impl ConnectionTrait,
    patient_id: &str,
) -> Result<Vec<StoredAmendment>, StoreError> {
    let patient = patients_store::current(db, patient_id).await?;
    let linked = ids::uuids(&links::linked_ids(db, &patient.id).await?);
    let rows = db
        .query_all_raw(pg(
            &format!(
                "SELECT {} FROM amendment_current WHERE patient_id = ANY($1::uuid[])
                 ORDER BY recorded_at DESC, amendment_id",
                columns()
            ),
            vec![linked.into()],
        ))
        .await
        .map_err(db_err)?;
    rows.iter().map(row_to_amendment).collect()
}

/// One request's current version.
///
/// # Errors
///
/// [`StoreError::NotFound`] when there is no such request for that patient.
pub async fn current(
    db: &impl ConnectionTrait,
    patient_id: &str,
    amendment_id: &str,
) -> Result<StoredAmendment, StoreError> {
    let amendment_id = ids::uuid(amendment_id).ok_or(StoreError::NotFound)?;
    let linked = ids::uuids(&links::linked_ids(db, patient_id).await?);
    let row = db
        .query_one_raw(pg(
            &format!(
                "SELECT {} FROM amendment_current
                 WHERE amendment_id = $1::uuid AND patient_id = ANY($2::uuid[])",
                columns()
            ),
            vec![amendment_id.into(), linked.into()],
        ))
        .await
        .map_err(db_err)?
        .ok_or(StoreError::NotFound)?;
    row_to_amendment(&row)
}

/// What a reader of one record must be told: a notice for each request about it that has one, newest
/// first. A withdrawn request has none.
///
/// # Errors
///
/// [`StoreError::NotFound`] (no such patient).
pub async fn notices(
    db: &impl ConnectionTrait,
    patient_id: &str,
    kind: &str,
    target_id: &str,
) -> Result<Vec<Notice>, StoreError> {
    Ok(list(db, patient_id)
        .await?
        .into_iter()
        .filter(|a| a.request.target.kind == kind && a.request.target.id == target_id)
        .filter_map(|a| a.request.notice())
        .collect())
}

/// Apply `event` to a request and append the result as the next version.
///
/// # Errors
///
/// [`StoreError::NotFound`], [`StoreError::Conflict`] (stale or racing version),
/// [`StoreError::BadQuery`] (the rule refused it, or an applied version the record does not have).
#[allow(clippy::too_many_arguments)]
pub async fn advance(
    db: &(impl ConnectionTrait + TransactionTrait),
    patient_id: &str,
    amendment_id: &str,
    expected_version: i32,
    author: &str,
    event: Event,
    now: DateTime<Utc>,
) -> Result<StoredAmendment, StoreError> {
    let was = current(db, patient_id, amendment_id).await?;
    if was.version != expected_version {
        return Err(StoreError::Conflict {
            current: was.version,
        });
    }
    if let Event::Apply { version } = &event {
        let have = target_version(
            db,
            &was.patient_id,
            &was.request.target.kind,
            &was.request.target.id,
        )
        .await?;
        if *version > have {
            return Err(StoreError::BadQuery(format!(
                "the record has no version {version}: record the correction first"
            )));
        }
    }
    let next = was.request.apply(event.clone()).map_err(refused)?;
    let mut logged = serde_json::to_value(&event).map_err(db_err)?;
    logged["by"] = author.into();
    logged["at"] = now.to_rfc3339_opts(SecondsFormat::Micros, true).into();
    let appended = crate::store::append_version(
        db,
        STORE,
        pg(
            &format!(
                "INSERT INTO amendment_versions
                   (amendment_id, version, patient_id, author, state, target_kind, target_id, event, body)
                 VALUES ($1::uuid, $2, $3::uuid, $4, $5, $6, $7::uuid, $8, $9)
                 RETURNING {}",
                columns()
            ),
            vec![
                was.id.clone().into(),
                (expected_version + 1).into(),
                was.patient_id.clone().into(),
                author.into(),
                next.state.word().into(),
                next.target.kind.clone().into(),
                next.target.id.clone().into(),
                logged.into(),
                serde_json::to_value(&next).map_err(db_err)?.into(),
            ],
        ),
    )
    .await?;
    match appended {
        crate::store::Appended::Row(row) => row_to_amendment(&row),
        crate::store::Appended::Taken => {
            let current = current(db, patient_id, amendment_id).await?.version;
            Err(StoreError::Conflict { current })
        }
    }
}
