//! The work queue for HL7 messages the feed refused (task OR-3).
//!
//! A refused message is kept as received so a person can see what the sender said and fix the
//! cause (correct the record, register the patient, ask the sender to resend) instead of the
//! message depending on the sender to hold and retry it. A person then closes it with an outcome
//! and a reason. Nothing is changed or deleted: "open" means no resolution row exists, and a
//! refusal can be resolved once.
//!
//! The stored text is patient data. It is shown only to the roles the policy table gives
//! `FeedQueue`, every view is audited, and the audit row names the refusal, never its content.
//! Messages refused as *transient* (`Temporary`) are not queued: the sender is told to retry, and
//! nothing about them is wrong.

use crate::ids;
use crate::patients_store::StoreError;
use crate::store::pg;
use sea_orm::ConnectionTrait;
use serde::{Deserialize, Serialize};

crate::store_errors!("feed queue");

/// The most entries a listing returns.
pub const LIST_LIMIT: u64 = 50;

/// The largest message kept (the MLLP frame cap is 1 MiB; a queue entry is for people to read).
pub const MAX_MESSAGE: usize = 1 << 20;

/// Why the message was refused, in the feed's terms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    /// A message type or text the feed does not handle (acknowledged `AR`).
    Unsupported,
    /// A message that is unacceptable or conflicts with the record (acknowledged `AE`).
    Conflict,
}

impl Kind {
    fn as_str(self) -> &'static str {
        match self {
            Kind::Unsupported => "unsupported",
            Kind::Conflict => "conflict",
        }
    }
}

/// How a person closed a refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Outcome {
    /// The cause was fixed or the message's content was entered another way.
    Handled,
    /// The message needs nothing done.
    Dismissed,
}

impl Outcome {
    fn as_str(self) -> &'static str {
        match self {
            Outcome::Handled => "handled",
            Outcome::Dismissed => "dismissed",
        }
    }
}

/// A person's decision on a refusal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Resolution {
    /// Who.
    pub resolved_by: String,
    /// When (UTC).
    pub resolved_at: String,
    /// How.
    pub outcome: String,
    /// Why.
    pub reason: String,
}

/// One refused message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Entry {
    /// Server id.
    pub id: String,
    /// When the feed refused it (UTC).
    pub received_at: String,
    /// The audit agent of the feed.
    pub agent: String,
    /// The sender's address.
    pub peer: Option<String>,
    /// The HL7 event (`A08`), or `?`.
    pub event: String,
    /// The sender's message control id (MSH-10), or `?`.
    pub control_id: String,
    /// `unsupported` or `conflict`.
    pub kind: String,
    /// Why, in words. Names no patient data.
    pub reasons: Vec<String>,
    /// The message as received. Only on a single-entry read, never in a listing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// How it was closed, if it was.
    pub resolution: Option<Resolution>,
}

const TS: &str = "'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"'";

fn select(with_message: bool) -> String {
    format!(
        "SELECT f.refusal_id::text AS id,
                to_char(f.received_at AT TIME ZONE 'UTC', {TS}) AS received_at,
                f.agent, f.peer, f.event, f.control_id, f.kind, f.reasons::text AS reasons,
                {message} AS message,
                r.resolved_by, r.outcome, r.reason AS resolution_reason,
                to_char(r.resolved_at AT TIME ZONE 'UTC', {TS}) AS resolved_at
         FROM feed_refusals f LEFT JOIN feed_refusal_resolutions r USING (refusal_id)",
        message = if with_message {
            "f.message"
        } else {
            "NULL::text"
        }
    )
}

fn to_entry(row: &sea_orm::QueryResult) -> Result<Entry, StoreError> {
    let get = |c: &str| row.try_get::<String>("", c).map_err(db_err);
    let reasons: Vec<String> = serde_json::from_str(&get("reasons")?).map_err(db_err)?;
    let resolved_by: Option<String> = row.try_get("", "resolved_by").map_err(db_err)?;
    let resolution = match resolved_by {
        Some(resolved_by) => Some(Resolution {
            resolved_by,
            resolved_at: get("resolved_at")?,
            outcome: get("outcome")?,
            reason: get("resolution_reason")?,
        }),
        None => None,
    };
    Ok(Entry {
        id: get("id")?,
        received_at: get("received_at")?,
        agent: get("agent")?,
        peer: row.try_get("", "peer").map_err(db_err)?,
        event: get("event")?,
        control_id: get("control_id")?,
        kind: get("kind")?,
        reasons,
        message: row.try_get("", "message").map_err(db_err)?,
        resolution,
    })
}

/// A message to queue.
#[derive(Debug, Clone, Copy)]
pub struct New<'a> {
    /// The feed's audit agent.
    pub agent: &'a str,
    /// The sender's address.
    pub peer: Option<&'a str>,
    /// The HL7 event, or `?`.
    pub event: &'a str,
    /// The sender's control id, or `?`.
    pub control_id: &'a str,
    /// Why it was refused, in kind.
    pub kind: Kind,
    /// Why, in words.
    pub reasons: &'a [String],
    /// The message as received.
    pub message: &'a str,
}

/// Keep a refused message. A message over [`MAX_MESSAGE`] bytes is cut.
///
/// # Errors
///
/// [`StoreError::Db`] when the insert fails.
pub async fn record(db: &impl ConnectionTrait, new: New<'_>) -> Result<String, StoreError> {
    let message = new.message;
    let mut text = message;
    if text.len() > MAX_MESSAGE {
        let mut end = MAX_MESSAGE;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text = &text[..end];
    }
    let row = db
        .query_one_raw(pg(
            "INSERT INTO feed_refusals (agent, peer, event, control_id, kind, reasons, message)
             VALUES ($1, $2, $3, $4, $5, $6::jsonb, $7) RETURNING refusal_id::text AS id",
            vec![
                new.agent.into(),
                new.peer.map(str::to_string).into(),
                new.event.into(),
                new.control_id.into(),
                new.kind.as_str().into(),
                serde_json::to_string(new.reasons).map_err(db_err)?.into(),
                text.into(),
            ],
        ))
        .await
        .map_err(db_err)?
        .ok_or_else(|| StoreError::Db("insert returned no row".into()))?;
    row.try_get("", "id").map_err(db_err)
}

/// The refusals nobody has closed, oldest first, without their message text.
///
/// # Errors
///
/// [`StoreError::Db`].
pub async fn list_open(db: &impl ConnectionTrait) -> Result<Vec<Entry>, StoreError> {
    let rows = db
        .query_all_raw(pg(
            &format!(
                "{} WHERE r.refusal_id IS NULL ORDER BY f.received_at, f.refusal_id LIMIT {LIST_LIMIT}",
                select(false)
            ),
            vec![],
        ))
        .await
        .map_err(db_err)?;
    rows.iter().map(to_entry).collect()
}

/// How many refusals nobody has closed.
///
/// # Errors
///
/// [`StoreError::Db`].
pub async fn count_open(db: &impl ConnectionTrait) -> Result<i64, StoreError> {
    let row = db
        .query_one_raw(pg(
            "SELECT count(*) AS n FROM feed_refusals f
             LEFT JOIN feed_refusal_resolutions r USING (refusal_id)
             WHERE r.refusal_id IS NULL",
            vec![],
        ))
        .await
        .map_err(db_err)?
        .ok_or_else(|| StoreError::Db("count returned no row".into()))?;
    row.try_get("", "n").map_err(db_err)
}

/// One refusal with its message text, open or closed.
///
/// # Errors
///
/// [`StoreError::NotFound`] when there is no such refusal (or the id is not a UUID).
pub async fn get(db: &impl ConnectionTrait, id: &str) -> Result<Entry, StoreError> {
    let id = ids::uuid(id).ok_or(StoreError::NotFound)?;
    let row = db
        .query_one_raw(pg(
            &format!("{} WHERE f.refusal_id = $1::uuid", select(true)),
            vec![id.into()],
        ))
        .await
        .map_err(db_err)?
        .ok_or(StoreError::NotFound)?;
    to_entry(&row)
}

/// Close a refusal. A reason is required. A refusal closes once.
///
/// # Errors
///
/// [`StoreError::NotFound`]; [`StoreError::BadQuery`] for a blank reason or one over 500
/// characters; [`StoreError::Conflict`] (current 1) when it was already closed.
pub async fn resolve(
    db: &impl ConnectionTrait,
    id: &str,
    by: &str,
    outcome: Outcome,
    reason: &str,
) -> Result<Entry, StoreError> {
    let reason = reason.trim();
    if reason.is_empty() || reason.chars().count() > 500 {
        return Err(StoreError::BadQuery(
            "a reason of 1 to 500 characters is required".into(),
        ));
    }
    let existing = get(db, id).await?;
    if existing.resolution.is_some() {
        return Err(StoreError::Conflict { current: 1 });
    }
    // The primary key on the refusal settles a race between two people.
    let inserted = db
        .execute_raw(pg(
            "INSERT INTO feed_refusal_resolutions (refusal_id, resolved_by, outcome, reason)
             VALUES ($1::uuid, $2, $3, $4) ON CONFLICT (refusal_id) DO NOTHING",
            vec![
                existing.id.clone().into(),
                by.into(),
                outcome.as_str().into(),
                reason.into(),
            ],
        ))
        .await
        .map_err(db_err)?;
    if inserted.rows_affected() == 0 {
        return Err(StoreError::Conflict { current: 1 });
    }
    get(db, id).await
}
