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
use crate::pool::Db;
use chrono::{DateTime, SecondsFormat, Utc};
use serde::Serialize;
use sqlx::postgres::PgRow;
use sqlx::{AssertSqlSafe, Row};

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

/// The columns every amendment read selects, aliased to the names [`row_to_amendment`] reads.
fn columns() -> String {
    format!(
        "amendment_id::text AS id, patient_id::text AS patient_id, version, author, {TS} AS at, body"
    )
}

/// Read one row into a [`StoredAmendment`].
///
/// # Errors
///
/// [`StoreError::Db`] when a column is missing or the stored body is unreadable.
fn row_to_amendment(row: &PgRow) -> Result<StoredAmendment, StoreError> {
    let body: serde_json::Value = row.try_get("body").map_err(db_err)?;
    Ok(StoredAmendment {
        id: row.try_get("id").map_err(db_err)?,
        patient_id: row.try_get("patient_id").map_err(db_err)?,
        version: row.try_get("version").map_err(db_err)?,
        author: row.try_get("author").map_err(db_err)?,
        recorded_at: row.try_get("at").map_err(db_err)?,
        request: serde_json::from_value(body).map_err(db_err)?,
    })
}

/// A refused amendment step as the writer's to fix: a 422 kind, with a sentence that never repeats the request text.
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
    db: impl Into<Db<'_>>,
    patient_id: &str,
    kind: &str,
    id: &str,
) -> Result<i32, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    match kind {
        "Observation" => Ok(crate::observations_store::current(&mut *db, patient_id, id)
            .await?
            .version),
        "AllergyIntolerance" => Ok(crate::allergies::current(&mut *db, patient_id, id)
            .await?
            .version),
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
    db: impl Into<Db<'_>>,
    patient_id: &str,
    requester: &str,
    kind: &str,
    target_id: &str,
    version: i32,
    text: &str,
    now: DateTime<Utc>,
) -> Result<StoredAmendment, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let patient = patients_store::current(&mut *db, patient_id).await?;
    let current = target_version(&mut *db, &patient.id, kind, target_id).await?;
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
    let row = sqlx::query(AssertSqlSafe(format!(
                "INSERT INTO amendment_versions
                   (amendment_id, version, patient_id, author, state, target_kind, target_id, event, body)
                 VALUES (gen_random_uuid(), 1, $1::uuid, $2, $3, $4, $5::uuid, $6, $7)
                 RETURNING {}",
                columns()
            )))
.bind(patient.id)
.bind(requester)
.bind(request.state.word())
.bind(kind)
.bind(target_id)
.bind(serde_json::json!({ "event": "open", "at": now.to_rfc3339_opts(SecondsFormat::Micros, true) }))
.bind(serde_json::to_value(&request).map_err(db_err)?)
.fetch_optional(&mut *db)
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
    db: impl Into<Db<'_>>,
    patient_id: &str,
) -> Result<Vec<StoredAmendment>, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let patient = patients_store::current(&mut *db, patient_id).await?;
    let linked = ids::uuids(&links::linked_ids(&mut *db, &patient.id).await?);
    let rows = sqlx::query(AssertSqlSafe(format!(
        "SELECT {} FROM amendment_current WHERE patient_id = ANY($1::uuid[])
                 ORDER BY recorded_at DESC, amendment_id",
        columns()
    )))
    .bind(linked)
    .fetch_all(&mut *db)
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
    db: impl Into<Db<'_>>,
    patient_id: &str,
    amendment_id: &str,
) -> Result<StoredAmendment, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let amendment_id = ids::uuid(amendment_id).ok_or(StoreError::NotFound)?;
    let linked = ids::uuids(&links::linked_ids(&mut *db, patient_id).await?);
    let row = sqlx::query(AssertSqlSafe(format!(
        "SELECT {} FROM amendment_current
                 WHERE amendment_id = $1::uuid AND patient_id = ANY($2::uuid[])",
        columns()
    )))
    .bind(amendment_id)
    .bind(linked)
    .fetch_optional(&mut *db)
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
    db: impl Into<Db<'_>>,
    patient_id: &str,
    kind: &str,
    target_id: &str,
) -> Result<Vec<Notice>, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    Ok(list(&mut *db, patient_id)
        .await?
        .into_iter()
        .filter(|a| a.request.target.kind == kind && a.request.target.id == target_id)
        .filter_map(|a| a.request.notice())
        .collect())
}

/// The notices for every record of one kind a patient has, by record id: the same notices as
/// [`notices`], read once. A record with none has no key. A list that shows records of this kind
/// carries this, and fails if it cannot be read: a dispute is never left out.
///
/// # Errors
///
/// [`StoreError::NotFound`] (no such patient).
pub async fn notices_by_target(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    kind: &str,
) -> Result<std::collections::BTreeMap<String, Vec<Notice>>, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let mut out = std::collections::BTreeMap::<String, Vec<Notice>>::new();
    for a in list(&mut *db, patient_id).await? {
        if a.request.target.kind == kind
            && let Some(n) = a.request.notice()
        {
            out.entry(a.request.target.id.clone()).or_default().push(n);
        }
    }
    Ok(out)
}

/// Apply `event` to a request and append the result as the next version.
///
/// # Errors
///
/// [`StoreError::NotFound`], [`StoreError::Conflict`] (stale or racing version),
/// [`StoreError::BadQuery`] (the rule refused it, or an applied version the record does not have).
#[allow(clippy::too_many_arguments)]
pub async fn advance(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    amendment_id: &str,
    expected_version: i32,
    author: &str,
    event: Event,
    now: DateTime<Utc>,
) -> Result<StoredAmendment, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let was = current(&mut *db, patient_id, amendment_id).await?;
    if was.version != expected_version {
        return Err(StoreError::Conflict {
            current: was.version,
        });
    }
    if let Event::Apply { version } = &event {
        let have = target_version(
            &mut *db,
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
        &mut *db,
        STORE,
        sqlx::query(AssertSqlSafe(format!(
                "INSERT INTO amendment_versions
                   (amendment_id, version, patient_id, author, state, target_kind, target_id, event, body)
                 VALUES ($1::uuid, $2, $3::uuid, $4, $5, $6, $7::uuid, $8, $9)
                 RETURNING {}",
                columns()
            )))
.bind(was.id.clone())
.bind(expected_version + 1)
.bind(was.patient_id.clone())
.bind(author)
.bind(next.state.word())
.bind(next.target.kind.clone())
.bind(next.target.id.clone())
.bind(logged)
.bind(serde_json::to_value(&next).map_err(db_err)?),
    )
    .await?;
    match appended {
        crate::store::Appended::Row(row) => row_to_amendment(&row),
        crate::store::Appended::Taken => {
            let current = current(&mut *db, patient_id, amendment_id).await?.version;
            Err(StoreError::Conflict { current })
        }
    }
}
