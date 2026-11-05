//! The queue of referral messages that did not match (task PC-2, hazard H-044).
//!
//! An inbound message (an outcome, a decline, an interim note) names the referral it answers by id. If it
//! names none, names one that does not exist, or asks for a step the state table refuses, it is kept as
//! received for a person to clear. It is **never** dropped, and **never** matched by patient and date. A
//! person closes an entry once, either by matching it by hand to a named referral (which records both ids
//! and changes nothing in the referral) or by dismissing it with a reason. Both tables are append-only.
//!
//! The stored message and note are patient data: they are shown only to the roles the policy table gives
//! the queue, every view is audited, and the audit row names the entry, never its content.

use crate::ids;
use crate::patients_store::StoreError;
use crate::pool::Db;
use crate::referral::{Actor, Event};
use crate::referrals_store::{self, Step, StoredReferral};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::postgres::PgRow;
use sqlx::{AssertSqlSafe, Row};

crate::store_errors!("referral unmatched");

/// The most entries a listing returns.
pub const LIST_LIMIT: u64 = 50;
/// The largest message kept; a longer one is cut.
pub const MAX_MESSAGE: usize = 1 << 20;
const MAX_NOTE: usize = 2000;

/// Why a message did not match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    /// The message carries no referral id.
    NoId,
    /// The id is not a referral on record.
    UnknownId,
    /// The referral exists but the step is not allowed (state table, wrong sender, missing note).
    RefusedByTable,
}

impl Kind {
    /// The kebab-case word stored in `referral_unmatched.kind` (the column has a matching `CHECK`).
    fn as_str(self) -> &'static str {
        match self {
            Kind::NoId => "no-id",
            Kind::UnknownId => "unknown-id",
            Kind::RefusedByTable => "refused-by-table",
        }
    }
}

/// An inbound message.
#[derive(Debug, Clone, Copy)]
pub struct Inbound<'a> {
    /// The sending system's audit agent.
    pub agent: &'a str,
    /// The referral id as received, if any.
    pub referral_ref: Option<&'a str>,
    /// The event word as received (`outcome`, `decline`, `interim-note`, ...), if any.
    pub event: Option<&'a str>,
    /// The sender's note, for the events that need one.
    pub note: Option<&'a str>,
    /// The message as received.
    pub message: &'a str,
}

/// What filing did.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "result", rename_all = "kebab-case")]
pub enum Filed {
    /// The id matched a referral and the step was allowed: the referral moved.
    Moved {
        /// The new version.
        referral: Box<StoredReferral>,
    },
    /// It did not match: it is in the queue.
    Queued {
        /// The queue entry (without its message text).
        entry: Box<Entry>,
    },
}

/// How a person closed an entry.
#[derive(Debug, Clone, Copy)]
pub enum Clearance<'a> {
    /// Matched by hand to this referral (a record of the link; the referral is not changed).
    Matched(&'a str),
    /// Nothing to do.
    Dismissed,
}

/// A person's decision on an entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Cleared {
    /// Who.
    pub cleared_by: String,
    /// When (UTC).
    pub cleared_at: String,
    /// `matched` or `dismissed`.
    pub how: String,
    /// The referral it was matched to, for `matched`.
    pub referral_id: Option<String>,
    /// Why.
    pub reason: String,
}

/// One queue entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Entry {
    /// Server id.
    pub id: String,
    /// When it arrived (UTC).
    pub received_at: String,
    /// The sending system.
    pub agent: String,
    /// Why it did not match.
    pub kind: String,
    /// The referral id as received.
    pub referral_ref: Option<String>,
    /// The event as received.
    pub event: Option<String>,
    /// Why, in words. Names no patient data.
    pub reasons: Vec<String>,
    /// The sender's note and the message as received. Only on a single-entry read, never in a listing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// The message as received. Only on a single-entry read.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// How it was closed, if it was.
    pub cleared: Option<Cleared>,
}

const TS: &str = "'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"'";

/// The `SELECT` for a queue entry joined to its clearance, if any. `with_text` chooses whether the note and
/// message (patient data) are selected; a listing passes `false` so they never leave in bulk.
fn select(with_text: bool) -> String {
    let (note, message) = if with_text {
        ("u.note", "u.message")
    } else {
        ("NULL::text", "NULL::text")
    };
    format!(
        "SELECT u.unmatched_id::text AS id,
                to_char(u.received_at AT TIME ZONE 'UTC', {TS}) AS received_at,
                u.agent, u.kind, u.referral_ref, u.event, u.reasons::text AS reasons,
                {note} AS note, {message} AS message,
                c.cleared_by, c.how, c.referral_id::text AS referral_id, c.reason AS clear_reason,
                to_char(c.cleared_at AT TIME ZONE 'UTC', {TS}) AS cleared_at
         FROM referral_unmatched u LEFT JOIN referral_unmatched_clearances c USING (unmatched_id)"
    )
}

/// Read one row of [`select`] into an [`Entry`]. A row with a `cleared_by` has a clearance; one without is open.
///
/// # Errors
///
/// [`StoreError::Db`] when a column is missing or the stored reasons are not a JSON array of strings.
fn to_entry(row: &PgRow) -> Result<Entry, StoreError> {
    let get = |c: &str| row.try_get::<String, _>(c).map_err(db_err);
    let opt = |c: &str| row.try_get::<Option<String>, _>(c).map_err(db_err);
    let cleared = match opt("cleared_by")? {
        Some(cleared_by) => Some(Cleared {
            cleared_by,
            cleared_at: get("cleared_at")?,
            how: get("how")?,
            referral_id: opt("referral_id")?,
            reason: get("clear_reason")?,
        }),
        None => None,
    };
    Ok(Entry {
        id: get("id")?,
        received_at: get("received_at")?,
        agent: get("agent")?,
        kind: get("kind")?,
        referral_ref: opt("referral_ref")?,
        event: opt("event")?,
        reasons: serde_json::from_str(&get("reasons")?).map_err(db_err)?,
        note: opt("note")?,
        message: opt("message")?,
        cleared,
    })
}

/// `text` cut to at most `max` bytes, backing up to a character boundary so the result is still valid UTF-8.
fn cut(text: &str, max: usize) -> &str {
    if text.len() <= max {
        return text;
    }
    let mut end = max;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

/// Keep an inbound message that did not match, with the one `reason` it did not, and answer [`Filed::Queued`]
/// without the note or message text. The caller has already decided the message cannot move a referral.
///
/// # Errors
///
/// [`StoreError::Db`] when the insert or the read-back fails.
async fn queue(
    db: impl Into<Db<'_>>,
    inbound: &Inbound<'_>,
    kind: Kind,
    reason: String,
) -> Result<Filed, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let row = sqlx::query(
        "INSERT INTO referral_unmatched (agent, kind, referral_ref, event, reasons, note, message)
             VALUES ($1, $2, $3, $4, $5::jsonb, $6, $7) RETURNING unmatched_id::text AS id",
    )
    .bind(inbound.agent)
    .bind(kind.as_str())
    .bind(inbound.referral_ref.map(|r| cut(r, 200).to_string()))
    .bind(inbound.event.map(|e| cut(e, 100).to_string()))
    .bind(serde_json::to_string(&[reason]).map_err(db_err)?)
    .bind(inbound.note.map(|n| cut(n, MAX_NOTE).to_string()))
    .bind(cut(inbound.message, MAX_MESSAGE))
    .fetch_optional(&mut *db)
    .await
    .map_err(db_err)?
    .ok_or_else(|| StoreError::Db("insert returned no row".into()))?;
    let id: String = row.try_get("id").map_err(db_err)?;
    let entry = get(&mut *db, &id).await?;
    Ok(Filed::Queued {
        entry: Box::new(Entry {
            note: None,
            message: None,
            ..entry
        }),
    })
}

/// File an inbound message: move the referral it answers, or queue it. A message is never dropped.
///
/// # Errors
///
/// [`StoreError::Conflict`] when another step raced this one (the sender may send again); [`StoreError::Db`].
pub async fn file(
    db: impl Into<Db<'_>>,
    inbound: Inbound<'_>,
    now: DateTime<Utc>,
) -> Result<Filed, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let Some(reference) = inbound
        .referral_ref
        .map(str::trim)
        .filter(|r| !r.is_empty())
    else {
        return queue(
            &mut *db,
            &inbound,
            Kind::NoId,
            "the message carries no referral id".into(),
        )
        .await;
    };
    let was = match referrals_store::current(&mut *db, reference).await {
        Ok(r) => r,
        Err(StoreError::NotFound) => {
            return queue(
                &mut *db,
                &inbound,
                Kind::UnknownId,
                "the referral id is not a referral on record".into(),
            )
            .await;
        }
        Err(e) => return Err(e),
    };
    let Some(event) = inbound.event.and_then(Event::parse) else {
        return queue(
            &mut *db,
            &inbound,
            Kind::RefusedByTable,
            "the message names no known event".into(),
        )
        .await;
    };
    if event.actor() != Actor::Recipient {
        return queue(
            &mut *db,
            &inbound,
            Kind::RefusedByTable,
            format!(
                "{} is the referrer's step; an inbound message carries the recipient's",
                event.as_str()
            ),
        )
        .await;
    }
    match referrals_store::advance(
        &mut *db,
        &was.id,
        was.version,
        Step {
            actor: Actor::Recipient,
            by: inbound.agent,
            event,
            note: inbound.note,
        },
        now,
    )
    .await
    {
        Ok(referral) => Ok(Filed::Moved {
            referral: Box::new(referral),
        }),
        // The state table, a missing note or a note that is too long: kept for a person, with the reason.
        Err(StoreError::BadQuery(reason)) => {
            queue(&mut *db, &inbound, Kind::RefusedByTable, reason).await
        }
        Err(e) => Err(e),
    }
}

/// The entries nobody has cleared, oldest first, without their text.
///
/// # Errors
///
/// [`StoreError::Db`].
pub async fn list_open(db: impl Into<Db<'_>>) -> Result<crate::page::Page<Entry>, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let rows = sqlx::query(AssertSqlSafe(format!(
        "{} WHERE c.unmatched_id IS NULL ORDER BY u.received_at, u.unmatched_id LIMIT {}",
        select(false),
        LIST_LIMIT + 1
    )))
    .fetch_all(&mut *db)
    .await
    .map_err(db_err)?;
    let fetched = rows.iter().map(to_entry).collect::<Result<Vec<_>, _>>()?;
    Ok(crate::page::Page::cut(fetched, LIST_LIMIT))
}

/// How many entries nobody has cleared.
///
/// # Errors
///
/// [`StoreError::Db`].
pub async fn count_open(db: impl Into<Db<'_>>) -> Result<i64, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let row = sqlx::query(
        "SELECT count(*) AS n FROM referral_unmatched u
             LEFT JOIN referral_unmatched_clearances c USING (unmatched_id)
             WHERE c.unmatched_id IS NULL",
    )
    .fetch_optional(&mut *db)
    .await
    .map_err(db_err)?
    .ok_or_else(|| StoreError::Db("count returned no row".into()))?;
    row.try_get("n").map_err(db_err)
}

/// One entry with its note and message, open or cleared.
///
/// # Errors
///
/// [`StoreError::NotFound`] when there is no such entry (or the id is not a UUID).
pub async fn get(db: impl Into<Db<'_>>, id: &str) -> Result<Entry, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let id = ids::uuid(id).ok_or(StoreError::NotFound)?;
    let row = sqlx::query(AssertSqlSafe(format!(
        "{} WHERE u.unmatched_id = $1::uuid",
        select(true)
    )))
    .bind(id)
    .fetch_optional(&mut *db)
    .await
    .map_err(db_err)?
    .ok_or(StoreError::NotFound)?;
    to_entry(&row)
}

/// Clear an entry. A reason is required. An entry clears once.
///
/// # Errors
///
/// [`StoreError::NotFound`] (the entry, or the referral a hand match names); [`StoreError::BadQuery`] for a
/// blank reason or one over 500 characters; [`StoreError::Conflict`] (current 1) when it was already cleared.
pub async fn clear(
    db: impl Into<Db<'_>>,
    id: &str,
    by: &str,
    how: Clearance<'_>,
    reason: &str,
) -> Result<Entry, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let reason = reason.trim();
    if reason.is_empty() || reason.chars().count() > 500 {
        return Err(StoreError::BadQuery(
            "a reason of 1 to 500 characters is required".into(),
        ));
    }
    let existing = get(&mut *db, id).await?;
    if existing.cleared.is_some() {
        return Err(StoreError::Conflict { current: 1 });
    }
    let (word, referral_id) = match how {
        Clearance::Matched(referral) => {
            // A hand match names a referral that exists; it records the link and changes nothing else.
            let found = referrals_store::current(&mut *db, referral).await?;
            ("matched", Some(found.id))
        }
        Clearance::Dismissed => ("dismissed", None),
    };
    // The primary key on the entry settles a race between two people.
    let inserted = sqlx::query("INSERT INTO referral_unmatched_clearances (unmatched_id, cleared_by, how, referral_id, reason)
             VALUES ($1::uuid, $2, $3, $4::uuid, $5) ON CONFLICT (unmatched_id) DO NOTHING")
.bind(existing.id.clone())
.bind(by)
.bind(word)
.bind(referral_id)
.bind(reason)
.execute(&mut *db)
        .await
        .map_err(db_err)?;
    if inserted.rows_affected() == 0 {
        return Err(StoreError::Conflict { current: 1 });
    }
    get(&mut *db, id).await
}
