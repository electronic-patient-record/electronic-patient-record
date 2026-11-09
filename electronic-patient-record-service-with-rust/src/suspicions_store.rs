//! Suspicions on referrals (RS-1): the append-only store behind [`crate::suspicion`]. One suspicion per
//! referral; version 1 as raised, each later version an amendment with a reason. Every read works out the
//! pathway's plan (`crate::pathways::plan`): the priority and each target's due day, from the point of
//! suspicion or from when the referral was sent, as the pathway says. The routes are in
//! `controllers/suspicions.rs`.

use crate::ids;
use crate::links;
use crate::pathways::{self, ClockFrom, Plan};
use crate::patients_store::StoreError;
use crate::pool::Db;
use crate::referrals_store;
use crate::suspicion::{self, Suspicion};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::postgres::PgRow;
use sqlx::{AssertSqlSafe, Row};

crate::store_errors!("suspicions");

/// One version of a referral's suspicion, as a reader sees it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StoredSuspicion {
    /// The referral it belongs to.
    pub referral_id: String,
    /// 1 as raised; each amendment adds one.
    pub version: i32,
    /// The patient.
    pub patient_id: String,
    /// Who wrote this version.
    pub author: String,
    /// When (UTC).
    pub recorded_at: String,
    /// Why it was amended; `null` on version 1.
    pub reason: Option<String>,
    /// The fields this version changed from the one before; empty on version 1.
    pub changed: Vec<&'static str>,
    /// The suspicion.
    pub suspicion: Suspicion,
    /// The pathway's priority and target days for it, worked out at the time of reading.
    pub plan: Plan,
}

/// A referral's suspicion: every version, oldest first, and the current one.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SuspicionHistory {
    /// The newest version; `null` when the referral carries none.
    pub current: Option<StoredSuspicion>,
    /// Every version, oldest first.
    pub versions: Vec<StoredSuspicion>,
}

const COLUMNS: &str =
    "referral_id::text AS referral_id, version, patient_id::text AS patient_id, author, reason,
    to_char(recorded_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"') AS at, body";

fn refuse(message: impl Into<String>) -> StoreError {
    StoreError::BadQuery(message.into())
}

fn time(s: &str) -> Result<DateTime<Utc>, StoreError> {
    DateTime::parse_from_rfc3339(s)
        .map(|t| t.with_timezone(&Utc))
        .map_err(db_err)
}

/// The referral, when it belongs to this patient; a referral from another record is a miss.
async fn referral_sent(
    db: &mut sqlx::PgConnection,
    patient_id: &str,
    referral_id: &str,
) -> Result<DateTime<Utc>, StoreError> {
    let versions = referrals_store::history(&mut *db, referral_id).await?;
    let first = versions.first().ok_or(StoreError::NotFound)?;
    if first.patient_id != patient_id {
        return Err(StoreError::NotFound);
    }
    time(&first.recorded_at)
}

fn plan_for(s: &Suspicion, sent: DateTime<Utc>, now: DateTime<Utc>) -> Result<Plan, StoreError> {
    // A stored suspicion names a pathway version the build no longer has only if a version was removed,
    // which the catalogue's rules forbid; it is an error, not a silent blank.
    let p = pathways::pathway(&s.pathway, s.pathway_version).ok_or_else(|| {
        StoreError::Db(format!(
            "suspicion pathway {} v{} is missing",
            s.pathway, s.pathway_version
        ))
    })?;
    let start = match p.clock_from {
        ClockFrom::PointOfSuspicion => time(&s.point_of_suspicion)?,
        ClockFrom::ReferralSent => sent,
    };
    pathways::plan(p, &s.choice(), start, now)
        .ok_or_else(|| StoreError::Db("a stored suspicion no longer fits its pathway".into()))
}

fn row_to_stored(
    row: &PgRow,
    previous: Option<&Suspicion>,
    sent: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Result<StoredSuspicion, StoreError> {
    let body: serde_json::Value = row.try_get("body").map_err(db_err)?;
    let s: Suspicion = serde_json::from_value(body).map_err(db_err)?;
    Ok(StoredSuspicion {
        referral_id: row.try_get("referral_id").map_err(db_err)?,
        version: row.try_get("version").map_err(db_err)?,
        patient_id: row.try_get("patient_id").map_err(db_err)?,
        author: row.try_get("author").map_err(db_err)?,
        recorded_at: row.try_get("at").map_err(db_err)?,
        reason: row.try_get("reason").map_err(db_err)?,
        changed: previous.map_or_else(Vec::new, |p| suspicion::changed(p, &s)),
        plan: plan_for(&s, sent, now)?,
        suspicion: s,
    })
}

/// Refuse what the pure rules refuse, and grounds naming a result that is not on this patient's record.
async fn check(
    db: &mut sqlx::PgConnection,
    patient_id: &str,
    s: &Suspicion,
    now: DateTime<Utc>,
) -> Result<(), StoreError> {
    let problems = suspicion::problems(s, now);
    if !problems.is_empty() {
        return Err(refuse(problems.join("; ")));
    }
    if let Some(result) = s.grounds.result_id.as_deref().and_then(ids::uuid) {
        let linked = ids::uuids(&links::linked_ids(&mut *db, patient_id).await?);
        let found = sqlx::query(
            "SELECT 1 FROM result_versions WHERE result_id = $1::uuid AND patient_id = ANY($2::uuid[]) LIMIT 1",
        )
        .bind(result)
        .bind(linked)
        .fetch_optional(&mut *db)
        .await
        .map_err(db_err)?;
        if found.is_none() {
            return Err(refuse("the grounds result is not on this patient's record"));
        }
    }
    Ok(())
}

async fn rows(db: &mut sqlx::PgConnection, referral_id: &str) -> Result<Vec<PgRow>, StoreError> {
    let id = ids::uuid(referral_id).ok_or(StoreError::NotFound)?;
    sqlx::query(AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM referral_suspicion_versions WHERE referral_id = $1::uuid ORDER BY version"
    )))
    .bind(id)
    .fetch_all(&mut *db)
    .await
    .map_err(db_err)
}

fn build(
    rows: &[PgRow],
    sent: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Result<SuspicionHistory, StoreError> {
    let mut versions: Vec<StoredSuspicion> = Vec::with_capacity(rows.len());
    for row in rows {
        let previous = versions.last().map(|v| &v.suspicion);
        let stored = row_to_stored(row, previous, sent, now)?;
        versions.push(stored);
    }
    Ok(SuspicionHistory {
        current: versions.last().cloned(),
        versions,
    })
}

/// A referral's suspicion, every version oldest first; none is an empty history, not a miss.
///
/// # Errors
///
/// [`StoreError::NotFound`] when the referral does not exist or belongs to another patient.
pub async fn history(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    referral_id: &str,
    now: DateTime<Utc>,
) -> Result<SuspicionHistory, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let sent = referral_sent(&mut *db, patient_id, referral_id).await?;
    let rows = rows(&mut *db, referral_id).await?;
    build(&rows, sent, now)
}

async fn insert(
    db: &mut sqlx::PgConnection,
    referral_id: &str,
    version: i32,
    patient_id: &str,
    author: &str,
    reason: Option<&str>,
    s: &Suspicion,
) -> Result<crate::store::Appended, StoreError> {
    crate::store::append_version(
        &mut *db,
        STORE,
        sqlx::query(AssertSqlSafe(format!(
            "INSERT INTO referral_suspicion_versions
                 (referral_id, version, patient_id, author, reason, pathway_id, pathway_version, body)
             VALUES ($1::uuid, $2, $3::uuid, $4, $5, $6, $7, $8)
             RETURNING {COLUMNS}"
        )))
        .bind(referral_id.to_string())
        .bind(version)
        .bind(patient_id.to_string())
        .bind(author.to_string())
        .bind(reason.map(str::trim).map(str::to_string))
        .bind(s.pathway.clone())
        .bind(s.pathway_version)
        .bind(serde_json::to_value(s).map_err(db_err)?),
    )
    .await
}

/// Raise the suspicion on a referral: version 1.
///
/// # Errors
///
/// [`StoreError::NotFound`] (no such referral on this patient), [`StoreError::BadQuery`] (refused, every
/// problem joined), [`StoreError::Conflict`] (the referral already carries one: amend it instead).
pub async fn raise(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    referral_id: &str,
    author: &str,
    s: &Suspicion,
    now: DateTime<Utc>,
) -> Result<StoredSuspicion, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let sent = referral_sent(&mut *db, patient_id, referral_id).await?;
    let referral_id = ids::uuid(referral_id).ok_or(StoreError::NotFound)?;
    if let Some(last) = rows(&mut *db, &referral_id).await?.last() {
        return Err(StoreError::Conflict {
            current: last.try_get("version").map_err(db_err)?,
        });
    }
    check(&mut *db, patient_id, s, now).await?;
    match insert(&mut *db, &referral_id, 1, patient_id, author, None, s).await? {
        crate::store::Appended::Row(row) => row_to_stored(&row, None, sent, now),
        crate::store::Appended::Taken => Err(StoreError::Conflict { current: 1 }),
    }
}

/// Amend the suspicion: the next version, with a reason, from the version the writer saw.
///
/// # Errors
///
/// [`StoreError::NotFound`] (no such referral, or it carries no suspicion), [`StoreError::Conflict`]
/// (stale or racing version), [`StoreError::BadQuery`] (refused: no reason, nothing changed, or the
/// suspicion's own problems).
#[allow(clippy::too_many_arguments)]
pub async fn amend(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    referral_id: &str,
    author: &str,
    expected_version: i32,
    reason: Option<&str>,
    s: &Suspicion,
    now: DateTime<Utc>,
) -> Result<StoredSuspicion, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let sent = referral_sent(&mut *db, patient_id, referral_id).await?;
    let referral_id = ids::uuid(referral_id).ok_or(StoreError::NotFound)?;
    let history = build(&rows(&mut *db, &referral_id).await?, sent, now)?;
    let was = history.current.ok_or(StoreError::NotFound)?;
    if was.version != expected_version {
        return Err(StoreError::Conflict {
            current: was.version,
        });
    }
    if let Some(r) = suspicion::amendment_refusal(&was.suspicion, s, reason) {
        return Err(refuse(r));
    }
    check(&mut *db, patient_id, s, now).await?;
    match insert(
        &mut *db,
        &referral_id,
        expected_version + 1,
        patient_id,
        author,
        reason,
        s,
    )
    .await?
    {
        crate::store::Appended::Row(row) => row_to_stored(&row, Some(&was.suspicion), sent, now),
        crate::store::Appended::Taken => {
            let current = rows(&mut *db, &referral_id)
                .await?
                .last()
                .map_or(Ok(expected_version + 1), |r| r.try_get("version"))
                .map_err(db_err)?;
            Err(StoreError::Conflict { current })
        }
    }
}
