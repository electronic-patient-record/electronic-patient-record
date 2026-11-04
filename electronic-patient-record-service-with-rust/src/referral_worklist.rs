//! The referrer's worklist (hazard H-044): the referrals a person sent that have moved into a state they
//! should look at, until they say they have.
//!
//! **Assumed, awaiting the maintainer (OQ-PC5):** the referrer is the author of version 1. A referral is on
//! that person's worklist when its *current* version is in one of [`ATTENTION`] and nobody has acknowledged
//! *that version*. An acknowledgement is an append (`referral_acknowledgements`), so a later version of the same
//! referral is a new item. The list carries ids, the state and a time: no recipient, reason, question or note.
//! It is a pointer; the referral itself is read through the patient's route, with the full record-level decision.
//!
//! A referral is also listed when it has gone quiet for longer than the configured period ([`Why::NoResponse`],
//! [`Why::NoOutcome`], PC-3, `referral_timeouts.rs`); such an item cannot be acknowledged and leaves the list when
//! the referral moves (or when the sender asks to cancel it).
//!
//! Not here: a worklist for the recipient, a team or a role (the service has no model of a team), reminders, and
//! an appointment date that has passed (`Book` carries no date).

use crate::ids;
use crate::page::Page;
use crate::patients_store::StoreError;
use crate::referral::State;
use crate::referral_timeouts::Timeouts;
use crate::store::pg;
use sea_orm::ConnectionTrait;
use serde::Serialize;

crate::store_errors!("referral worklist");

/// The most items a listing returns.
pub const LIST_LIMIT: u64 = 50;

/// The states that put a referral on its referrer's worklist: the recipient has said or done something the
/// referrer did not ask for or cannot tell from the request. Not here: `requested`, `accepted`,
/// `appointment-booked` and `cancel-requested` (the referral is moving as asked), and `cancelled` (the referrer
/// asked for it).
pub const ATTENTION: [State; 7] = [
    State::Declined,
    State::Redirected,
    State::NoShow,
    State::Deferred,
    State::InformationRequested,
    State::InterimReport,
    State::Completed,
];

/// One referral waiting for its referrer. No clinical text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Item {
    /// The referral id.
    pub referral_id: String,
    /// The patient, so a screen can link to the referral.
    pub patient_id: String,
    /// The version that is waiting.
    pub version: i32,
    /// The state it is in.
    pub state: State,
    /// Why it is on the list: a reply to acknowledge, or a timeout (PC-3).
    pub why: Why,
    /// When that version was written (UTC).
    pub recorded_at: String,
}

/// The `IN (...)` list of [`ATTENTION`] words. Constants of this crate, never user input.
fn attention_sql() -> String {
    ATTENTION
        .iter()
        .map(|s| format!("'{}'", s.as_str()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The states in which a referral with no outcome is timed against [`Timeouts::no_outcome`].
pub const AWAITING_OUTCOME: [State; 3] = [
    State::Accepted,
    State::AppointmentBooked,
    State::InterimReport,
];

/// Why a referral is on the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Why {
    /// The recipient answered in a way that waits for the sender; acknowledge it.
    Reply,
    /// Nobody has answered for longer than the configured period (PC-3). It leaves when the referral moves.
    NoResponse,
    /// It was accepted but no outcome has come for longer than the configured period (PC-3).
    NoOutcome,
}

/// `OR` of one clause per configured priority: the referral's priority and a current version older than that
/// many days. `false` when nothing is configured. The words and numbers come from [`Timeouts::parse`], which
/// accepts only the four priority words and whole numbers of days.
fn overdue_sql(periods: &std::collections::BTreeMap<String, u32>) -> String {
    // `$2` is the time the clock shows (PR-5), so a test moves the clock instead of the rows.
    if periods.is_empty() {
        return "false".into();
    }
    periods
        .iter()
        .map(|(priority, days)| {
            format!(
                "(c.body->>'priority' = '{priority}' AND c.recorded_at < $2::timestamptz - interval '{days} days')"
            )
        })
        .collect::<Vec<_>>()
        .join(" OR ")
}

/// The query shared by the list and the count: this person's referrals with a reason to be on the list, as a
/// sub-select with a `why` column.
fn waiting(timeouts: &Timeouts) -> String {
    let outcome_states = AWAITING_OUTCOME
        .iter()
        .map(|s| format!("'{}'", s.as_str()))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "FROM (
           SELECT c.referral_id, c.patient_id, c.version, c.state, c.recorded_at,
                  CASE
                    WHEN a.referral_id IS NULL AND c.state IN ({attention}) THEN 'reply'
                    WHEN c.state = 'requested' AND ({response}) THEN 'no-response'
                    WHEN c.state IN ({outcome_states}) AND ({outcome}) THEN 'no-outcome'
                  END AS why
           FROM referral_current c
           JOIN referral_versions first ON first.referral_id = c.referral_id AND first.version = 1
           LEFT JOIN referral_acknowledgements a ON a.referral_id = c.referral_id AND a.version = c.version
           WHERE first.author = $1
         ) c WHERE c.why IS NOT NULL",
        attention = attention_sql(),
        response = overdue_sql(&timeouts.no_response),
        outcome = overdue_sql(&timeouts.no_outcome),
    )
}

/// The referrals `by` sent that wait for them, oldest first, up to [`LIST_LIMIT`]; `more` says whether any were
/// left behind (PR-1). Timed against the periods [`crate::referral_timeouts::init`] checked.
///
/// # Errors
///
/// [`StoreError::Db`].
pub async fn list(db: &impl ConnectionTrait, by: &str) -> Result<Page<Item>, StoreError> {
    list_with(db, by, &crate::referral_timeouts::current()).await
}

/// [`list`] against given periods.
///
/// # Errors
///
/// [`StoreError::Db`].
pub async fn list_with(
    db: &impl ConnectionTrait,
    by: &str,
    timeouts: &Timeouts,
) -> Result<Page<Item>, StoreError> {
    list_at(db, by, timeouts, chrono::Utc::now()).await
}

/// [`list_with`] judging how long a referral has waited by `at`, the time a [`crate::clock::Clock`] shows.
///
/// # Errors
///
/// [`StoreError::Db`].
pub async fn list_at(
    db: &impl ConnectionTrait,
    by: &str,
    timeouts: &Timeouts,
    at: chrono::DateTime<chrono::Utc>,
) -> Result<Page<Item>, StoreError> {
    let rows = db
        .query_all_raw(pg(
            &format!(
                "SELECT c.referral_id::text AS id, c.patient_id::text AS patient_id, c.version, c.state, c.why,
                        to_char(c.recorded_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"') AS at
                 {} ORDER BY c.recorded_at, c.referral_id LIMIT {}",
                waiting(timeouts),
                LIST_LIMIT + 1
            ),
            vec![by.into(), at.into()],
        ))
        .await
        .map_err(db_err)?;
    let mut items = Vec::with_capacity(rows.len());
    for row in &rows {
        let word: String = row.try_get("", "state").map_err(db_err)?;
        let state = State::ALL
            .into_iter()
            .find(|s| s.as_str() == word)
            .ok_or_else(|| db_err(format!("unexpected state {word}")))?;
        let why: String = row.try_get("", "why").map_err(db_err)?;
        let why = match why.as_str() {
            "reply" => Why::Reply,
            "no-response" => Why::NoResponse,
            "no-outcome" => Why::NoOutcome,
            other => return Err(db_err(format!("unexpected reason {other}"))),
        };
        items.push(Item {
            referral_id: row.try_get("", "id").map_err(db_err)?,
            patient_id: row.try_get("", "patient_id").map_err(db_err)?,
            version: row.try_get("", "version").map_err(db_err)?,
            state,
            why,
            recorded_at: row.try_get("", "at").map_err(db_err)?,
        });
    }
    Ok(Page::cut(items, LIST_LIMIT))
}

/// How many referrals wait for `by`, against the checked periods.
///
/// # Errors
///
/// [`StoreError::Db`].
pub async fn count(db: &impl ConnectionTrait, by: &str) -> Result<i64, StoreError> {
    count_with(db, by, &crate::referral_timeouts::current()).await
}

/// [`count`] against given periods.
///
/// # Errors
///
/// [`StoreError::Db`].
pub async fn count_with(
    db: &impl ConnectionTrait,
    by: &str,
    timeouts: &Timeouts,
) -> Result<i64, StoreError> {
    count_at(db, by, timeouts, chrono::Utc::now()).await
}

/// [`count_with`] judging by `at`.
///
/// # Errors
///
/// [`StoreError::Db`].
pub async fn count_at(
    db: &impl ConnectionTrait,
    by: &str,
    timeouts: &Timeouts,
    at: chrono::DateTime<chrono::Utc>,
) -> Result<i64, StoreError> {
    let row = db
        .query_one_raw(pg(
            &format!("SELECT count(*) AS n {}", waiting(timeouts)),
            vec![by.into(), at.into()],
        ))
        .await
        .map_err(db_err)?
        .ok_or_else(|| db_err("count returned no row"))?;
    row.try_get("", "n").map_err(db_err)
}

/// `by` says they have seen `version` of a referral they sent. Once only.
///
/// # Errors
///
/// [`StoreError::NotFound`] when there is no such referral or `by` did not send it (the two look alike on
/// purpose); [`StoreError::Conflict`] when `version` is no longer the current one, or was already acknowledged;
/// [`StoreError::BadQuery`] when the current state is not one that waits for the referrer.
pub async fn acknowledge(
    db: &impl ConnectionTrait,
    referral_id: &str,
    version: i32,
    by: &str,
) -> Result<(), StoreError> {
    let id = ids::uuid(referral_id).ok_or(StoreError::NotFound)?;
    let row = db
        .query_one_raw(pg(
            "SELECT c.version, c.state, first.author
             FROM referral_current c
             JOIN referral_versions first ON first.referral_id = c.referral_id AND first.version = 1
             WHERE c.referral_id = $1::uuid",
            vec![id.clone().into()],
        ))
        .await
        .map_err(db_err)?
        .ok_or(StoreError::NotFound)?;
    let author: String = row.try_get("", "author").map_err(db_err)?;
    if author != by {
        return Err(StoreError::NotFound);
    }
    let current: i32 = row.try_get("", "version").map_err(db_err)?;
    if current != version {
        return Err(StoreError::Conflict { current });
    }
    let state: String = row.try_get("", "state").map_err(db_err)?;
    if !ATTENTION.iter().any(|s| s.as_str() == state) {
        return Err(StoreError::BadQuery(
            "nothing in this referral waits for the referrer".into(),
        ));
    }
    let done = db
        .execute_raw(pg(
            "INSERT INTO referral_acknowledgements (referral_id, version, acknowledged_by)
             VALUES ($1::uuid, $2, $3) ON CONFLICT DO NOTHING",
            vec![id.into(), version.into(), by.into()],
        ))
        .await
        .map_err(db_err)?;
    if done.rows_affected() == 0 {
        return Err(StoreError::Conflict { current });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_worklist_states_are_the_ones_the_referrer_did_not_ask_for() {
        let words: Vec<_> = ATTENTION.iter().map(|s| s.as_str()).collect();
        for waiting_on_recipient in [
            "requested",
            "accepted",
            "appointment-booked",
            "cancel-requested",
            "cancelled",
        ] {
            assert!(
                !words.contains(&waiting_on_recipient),
                "{waiting_on_recipient}"
            );
        }
        // Every state is either on the worklist or deliberately off it.
        assert_eq!(ATTENTION.len() + 5, State::ALL.len());
        assert!(attention_sql().starts_with("'declined'"));
    }
}
