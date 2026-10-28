//! The hazard register (AS-17): the append-only store behind the rules in [`crate::hazard`]. Every step
//! is a new version of the hazard with the event that caused it; nothing is edited or deleted, and the
//! database refuses both. There is no delete function here, on purpose.

use crate::hazard::{Actor, Event, Hazard, Refusal};
use crate::ids;
use crate::patients_store::StoreError;
use crate::store::pg;
use chrono::{DateTime, SecondsFormat, Utc};
use sea_orm::{ConnectionTrait, TransactionTrait};
use serde::Serialize;

crate::store_errors!("hazards");

/// One hazard as stored.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StoredHazard {
    /// Server id, the same for every version.
    pub id: String,
    /// 1 for the hazard as raised.
    pub version: i32,
    /// Who wrote this version.
    pub author: String,
    /// When this version was written (UTC).
    pub recorded_at: String,
    /// The hazard after this version.
    pub hazard: Hazard,
}

const TS: &str = "to_char(recorded_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"')";

fn columns() -> String {
    format!("hazard_id::text AS id, version, author, {TS} AS at, body")
}

fn row_to_hazard(row: &sea_orm::QueryResult) -> Result<StoredHazard, StoreError> {
    let body: serde_json::Value = row.try_get("", "body").map_err(db_err)?;
    Ok(StoredHazard {
        id: row.try_get("", "id").map_err(db_err)?,
        version: row.try_get("", "version").map_err(db_err)?,
        author: row.try_get("", "author").map_err(db_err)?,
        recorded_at: row.try_get("", "at").map_err(db_err)?,
        hazard: serde_json::from_value(body).map_err(db_err)?,
    })
}

fn refused(r: Refusal) -> StoreError {
    StoreError::BadQuery(match r {
        Refusal::NotTheCso => "only the Clinical Safety Officer can retire a hazard".to_string(),
        Refusal::WrongState => "the hazard is not in a state that allows this".to_string(),
        Refusal::Blank(what) => format!("the {what} must not be blank"),
        Refusal::TooLong(what) => format!("the {what} is too long"),
        Refusal::BadRating => "severity and likelihood must be from 1 to 5".to_string(),
        Refusal::NoEvidence => {
            "a hazard cannot be retired without evidence that it has been removed".to_string()
        }
        Refusal::TooMuchEvidence => "there are too many evidence items".to_string(),
        Refusal::BadHash => {
            "each evidence item needs a SHA-256 of 64 lowercase hex digits".to_string()
        }
        Refusal::FromTheFuture => "evidence cannot have been taken in the future".to_string(),
        Refusal::RecordedTheControl => {
            "whoever recorded a control on this hazard cannot also retire it".to_string()
        }
    })
}

/// Raise a hazard.
///
/// # Errors
///
/// [`StoreError::BadQuery`] when the rule refuses it.
pub async fn raise(
    db: &impl ConnectionTrait,
    author: &str,
    title: &str,
    severity: u8,
    likelihood: u8,
    now: DateTime<Utc>,
) -> Result<StoredHazard, StoreError> {
    let hazard = Hazard::raise(author, title, severity, likelihood).map_err(refused)?;
    let row = db
        .query_one_raw(pg(
            &format!(
                "INSERT INTO hazard_versions (hazard_id, version, author, state, event, body)
                 VALUES (gen_random_uuid(), 1, $1, $2, $3, $4)
                 RETURNING {}",
                columns()
            ),
            vec![
                author.into(),
                hazard.state.word().into(),
                serde_json::json!({
                    "event": "raise",
                    "by": author,
                    "at": now.to_rfc3339_opts(SecondsFormat::Micros, true),
                })
                .into(),
                serde_json::to_value(&hazard).map_err(db_err)?.into(),
            ],
        ))
        .await
        .map_err(db_err)?
        .ok_or_else(|| StoreError::Db("insert returned no row".into()))?;
    row_to_hazard(&row)
}

/// Every hazard's current version, newest first.
///
/// # Errors
///
/// [`StoreError::Db`] when the read fails.
pub async fn list(db: &impl ConnectionTrait) -> Result<Vec<StoredHazard>, StoreError> {
    let rows = db
        .query_all_raw(pg(
            &format!(
                "SELECT {} FROM hazard_current ORDER BY recorded_at DESC, hazard_id",
                columns()
            ),
            vec![],
        ))
        .await
        .map_err(db_err)?;
    rows.iter().map(row_to_hazard).collect()
}

/// One hazard's current version.
///
/// # Errors
///
/// [`StoreError::NotFound`] when there is no such hazard.
pub async fn current(
    db: &impl ConnectionTrait,
    hazard_id: &str,
) -> Result<StoredHazard, StoreError> {
    let id = ids::uuid(hazard_id).ok_or(StoreError::NotFound)?;
    let row = db
        .query_one_raw(pg(
            &format!(
                "SELECT {} FROM hazard_current WHERE hazard_id = $1::uuid",
                columns()
            ),
            vec![id.into()],
        ))
        .await
        .map_err(db_err)?
        .ok_or(StoreError::NotFound)?;
    row_to_hazard(&row)
}

/// Apply `event` to a hazard and append the result as the next version.
///
/// # Errors
///
/// [`StoreError::NotFound`], [`StoreError::Conflict`] (stale or racing version),
/// [`StoreError::BadQuery`] (the rule refused it).
pub async fn advance(
    db: &(impl ConnectionTrait + TransactionTrait),
    hazard_id: &str,
    expected_version: i32,
    actor: &Actor<'_>,
    event: Event,
    now: DateTime<Utc>,
) -> Result<StoredHazard, StoreError> {
    let was = current(db, hazard_id).await?;
    if was.version != expected_version {
        return Err(StoreError::Conflict {
            current: was.version,
        });
    }
    let next = was
        .hazard
        .apply(actor, event.clone(), now)
        .map_err(refused)?;
    let mut logged = serde_json::to_value(&event).map_err(db_err)?;
    logged["by"] = actor.id.into();
    logged["at"] = now.to_rfc3339_opts(SecondsFormat::Micros, true).into();
    let appended = crate::store::append_version(
        db,
        STORE,
        pg(
            &format!(
                "INSERT INTO hazard_versions (hazard_id, version, author, state, event, body)
                 VALUES ($1::uuid, $2, $3, $4, $5, $6)
                 RETURNING {}",
                columns()
            ),
            vec![
                was.id.clone().into(),
                (expected_version + 1).into(),
                actor.id.into(),
                next.state.word().into(),
                logged.into(),
                serde_json::to_value(&next).map_err(db_err)?.into(),
            ],
        ),
    )
    .await?;
    match appended {
        crate::store::Appended::Row(row) => row_to_hazard(&row),
        crate::store::Appended::Taken => {
            let current = current(db, hazard_id).await?.version;
            Err(StoreError::Conflict { current })
        }
    }
}
