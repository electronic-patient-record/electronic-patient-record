//! Watch lists (WL-6): the append-only store behind [`crate::watch_list`]. An entry is version 1 as
//! added and version 2 when its owner ends it; a lapsed entry (its last day passed) is worked out when
//! read. One active entry per owner and patient. Reads of a patient follow merges. The routes are in
//! `controllers/watch_list.rs`.

use crate::ids;
use crate::links;
use crate::patients_store::{self, StoreError};
use crate::pool::Db;
use crate::watch_list::{self, NewEntry, State};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgRow;
use sqlx::{AssertSqlSafe, Row};

crate::store_errors!("watch list");

/// The most entries one person's list returns.
pub const OWN_LIMIT: i64 = 200;

/// What a version stores.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Body {
    reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    until: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    end_reason: Option<String>,
}

/// One entry as read on the patient's record: who is watching, why, and until when.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Entry {
    /// The entry.
    pub id: String,
    /// 1 as added; 2 once ended.
    pub version: i32,
    /// Whose list it is on.
    pub owner: String,
    /// The patient.
    pub patient_id: String,
    /// When it was added (UTC).
    pub since: String,
    /// When this version was written (UTC).
    pub recorded_at: String,
    /// Where it stands today.
    pub state: State,
    /// Why the patient is on the list.
    pub reason: String,
    /// Its last day, if any.
    pub until: Option<String>,
    /// Why it was ended, once ended.
    pub end_reason: Option<String>,
}

/// One entry on one's own list: a pointer, no reason. Open it through the patient's route.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Pointer {
    /// The entry.
    pub id: String,
    /// Its version.
    pub version: i32,
    /// The patient.
    pub patient_id: String,
    /// When it was added (UTC).
    pub since: String,
    /// Its last day, if any.
    pub until: Option<String>,
}

const TS: &str = "'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"'";

fn columns() -> String {
    format!(
        "c.entry_id::text AS id, c.version, c.owner, c.patient_id::text AS patient_id, c.status, c.body,
         to_char(c.recorded_at AT TIME ZONE 'UTC', {TS}) AS at,
         to_char(f.recorded_at AT TIME ZONE 'UTC', {TS}) AS since"
    )
}

const FROM: &str = "FROM watch_entry_current c
    JOIN watch_entry_versions f ON f.entry_id = c.entry_id AND f.version = 1";

fn refuse(message: impl Into<String>) -> StoreError {
    StoreError::BadQuery(message.into())
}

fn row_to_entry(row: &PgRow, today: NaiveDate) -> Result<Entry, StoreError> {
    let body: Body =
        serde_json::from_value(row.try_get("body").map_err(db_err)?).map_err(db_err)?;
    let status: String = row.try_get("status").map_err(db_err)?;
    let stored = State::STORED
        .into_iter()
        .find(|s| s.word() == status)
        .ok_or_else(|| db_err(format!("unexpected status {status}")))?;
    Ok(Entry {
        id: row.try_get("id").map_err(db_err)?,
        version: row.try_get("version").map_err(db_err)?,
        owner: row.try_get("owner").map_err(db_err)?,
        patient_id: row.try_get("patient_id").map_err(db_err)?,
        since: row.try_get("since").map_err(db_err)?,
        recorded_at: row.try_get("at").map_err(db_err)?,
        state: watch_list::state(stored, body.until.as_deref(), today),
        reason: body.reason,
        until: body.until,
        end_reason: body.end_reason,
    })
}

/// Every entry on a patient's record (and records merged into it), current version of each, active
/// first then newest first. Ended and lapsed entries stay in the answer.
///
/// # Errors
///
/// [`StoreError::NotFound`] (no such patient).
pub async fn for_patient(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    today: NaiveDate,
) -> Result<Vec<Entry>, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let patient = patients_store::current(&mut *db, patient_id).await?;
    let linked = ids::uuids(&links::linked_ids(&mut *db, &patient.id).await?);
    let rows = sqlx::query(AssertSqlSafe(format!(
        "SELECT {} {FROM} WHERE c.patient_id = ANY($1::uuid[]) ORDER BY f.recorded_at DESC, c.entry_id",
        columns()
    )))
    .bind(linked)
    .fetch_all(&mut *db)
    .await
    .map_err(db_err)?;
    let mut entries = rows
        .iter()
        .map(|r| row_to_entry(r, today))
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|e| e.state != State::Active);
    Ok(entries)
}

/// Add a patient to `owner`'s list.
///
/// # Errors
///
/// [`StoreError::NotFound`] (no such patient), [`StoreError::BadQuery`] (refused),
/// [`StoreError::Conflict`] (already on that list, and not lapsed: its version is given).
pub async fn add(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    owner: &str,
    entry: &NewEntry,
    today: NaiveDate,
) -> Result<Entry, StoreError> {
    let problems = watch_list::problems(entry, today);
    if !problems.is_empty() {
        return Err(refuse(problems.join("; ")));
    }
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let patient = patients_store::current(&mut *db, patient_id).await?;
    // Two adds at once for the same owner and patient wait for each other, so only one is active.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1))")
        .bind(format!("watch:{owner}:{}", patient.id))
        .execute(&mut *db)
        .await
        .map_err(db_err)?;
    if let Some(open) = for_patient(&mut *db, &patient.id, today)
        .await?
        .into_iter()
        .find(|e| e.owner == owner && e.state == State::Active)
    {
        return Err(StoreError::Conflict {
            current: open.version,
        });
    }
    let body = Body {
        reason: entry.reason.trim().to_string(),
        until: entry.until.clone(),
        end_reason: None,
    };
    let row = sqlx::query(
        "INSERT INTO watch_entry_versions (entry_id, version, owner, patient_id, author, status, body)
         VALUES (gen_random_uuid(), 1, $1, $2::uuid, $1, 'active', $3)
         RETURNING entry_id::text AS id",
    )
    .bind(owner)
    .bind(patient.id.clone())
    .bind(serde_json::to_value(&body).map_err(db_err)?)
    .fetch_one(&mut *db)
    .await
    .map_err(db_err)?;
    let id: String = row.try_get("id").map_err(db_err)?;
    one(&mut *db, &id, today).await
}

async fn one(
    db: &mut sqlx::PgConnection,
    entry_id: &str,
    today: NaiveDate,
) -> Result<Entry, StoreError> {
    let id = ids::uuid(entry_id).ok_or(StoreError::NotFound)?;
    let row = sqlx::query(AssertSqlSafe(format!(
        "SELECT {} {FROM} WHERE c.entry_id = $1::uuid",
        columns()
    )))
    .bind(id)
    .fetch_optional(&mut *db)
    .await
    .map_err(db_err)?
    .ok_or(StoreError::NotFound)?;
    row_to_entry(&row, today)
}

/// End `owner`'s entry, with a reason. Someone else's entry, or one on another patient's record, is a
/// miss.
///
/// # Errors
///
/// [`StoreError::NotFound`], [`StoreError::Conflict`] (stale or racing version),
/// [`StoreError::BadQuery`] (no reason, or already ended).
#[allow(clippy::too_many_arguments)]
pub async fn end(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    entry_id: &str,
    owner: &str,
    expected_version: i32,
    end_reason: Option<&str>,
    today: NaiveDate,
) -> Result<Entry, StoreError> {
    let problems = watch_list::end_problems(end_reason);
    if !problems.is_empty() {
        return Err(refuse(problems.join("; ")));
    }
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let patient = patients_store::current(&mut *db, patient_id).await?;
    let linked = links::linked_ids(&mut *db, &patient.id).await?;
    let was = one(&mut *db, entry_id, today).await?;
    if was.owner != owner || !linked.contains(&was.patient_id) {
        return Err(StoreError::NotFound);
    }
    if was.state == State::Ended {
        return Err(refuse("the entry has already ended"));
    }
    if was.version != expected_version {
        return Err(StoreError::Conflict {
            current: was.version,
        });
    }
    let body = Body {
        reason: was.reason.clone(),
        until: was.until.clone(),
        end_reason: end_reason.map(|r| r.trim().to_string()),
    };
    let appended = crate::store::append_version(
        &mut *db,
        STORE,
        sqlx::query(
            "INSERT INTO watch_entry_versions (entry_id, version, owner, patient_id, author, status, body)
             VALUES ($1::uuid, $2, $3, $4::uuid, $3, 'ended', $5)
             RETURNING entry_id",
        )
        .bind(was.id.clone())
        .bind(expected_version + 1)
        .bind(owner)
        .bind(was.patient_id.clone())
        .bind(serde_json::to_value(&body).map_err(db_err)?),
    )
    .await?;
    if matches!(appended, crate::store::Appended::Taken) {
        let current = one(&mut *db, entry_id, today).await?.version;
        return Err(StoreError::Conflict { current });
    }
    one(&mut *db, entry_id, today).await
}

/// `owner`'s list: active entries only (not ended, not lapsed), oldest first, up to [`OWN_LIMIT`], as
/// pointers with no reason. `more` is whether any were left out.
///
/// # Errors
///
/// [`StoreError::Db`].
pub async fn own(
    db: impl Into<Db<'_>>,
    owner: &str,
    today: NaiveDate,
) -> Result<(Vec<Pointer>, bool), StoreError> {
    pointers(
        db,
        owner,
        today,
        "(c.body->>'until' IS NULL OR (c.body->>'until')::date >= $2::date)",
        "f.recorded_at, c.entry_id",
    )
    .await
}

/// `owner`'s lapsed entries (WL-6): their last day has passed and the owner has not ended them, oldest
/// last day first, up to [`OWN_LIMIT`]. A lapsed entry leaves the active list, so this is how its owner
/// learns of it; ending it with a reason, or adding the patient again, takes it off.
///
/// # Errors
///
/// [`StoreError::Db`].
pub async fn lapsed(
    db: impl Into<Db<'_>>,
    owner: &str,
    today: NaiveDate,
) -> Result<(Vec<Pointer>, bool), StoreError> {
    pointers(
        db,
        owner,
        today,
        "(c.body->>'until')::date < $2::date",
        "(c.body->>'until')::date, c.entry_id",
    )
    .await
}

async fn pointers(
    db: impl Into<Db<'_>>,
    owner: &str,
    today: NaiveDate,
    condition: &str,
    order: &str,
) -> Result<(Vec<Pointer>, bool), StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let rows = sqlx::query(AssertSqlSafe(format!(
        "SELECT {} {FROM}
         WHERE c.owner = $1 AND c.status = 'active' AND {condition}
         ORDER BY {order} LIMIT {}",
        columns(),
        OWN_LIMIT + 1
    )))
    .bind(owner)
    .bind(today)
    .fetch_all(&mut *db)
    .await
    .map_err(db_err)?;
    let more = rows.len() as i64 > OWN_LIMIT;
    let pointers = rows
        .iter()
        .take(usize::try_from(OWN_LIMIT).unwrap_or(usize::MAX))
        .map(|r| {
            let e = row_to_entry(r, today)?;
            Ok(Pointer {
                id: e.id,
                version: e.version,
                patient_id: e.patient_id,
                since: e.since,
                until: e.until,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    Ok((pointers, more))
}
