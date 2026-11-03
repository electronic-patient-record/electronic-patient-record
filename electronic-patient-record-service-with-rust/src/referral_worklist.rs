//! The referrer's worklist (hazard H-044): the referrals a person sent that have moved into a state they
//! should look at, until they say they have.
//!
//! **Assumed, awaiting the maintainer (OQ-PC5):** the referrer is the author of version 1. A referral is on
//! that person's worklist when its *current* version is in one of [`ATTENTION`] and nobody has acknowledged
//! *that version*. An acknowledgement is an append (`referral_acknowledgements`), so a later version of the same
//! referral is a new item. The list carries ids, the state and a time: no recipient, reason, question or note.
//! It is a pointer; the referral itself is read through the patient's route, with the full record-level decision.
//!
//! Not here: a worklist for the recipient, a team or a role (the service has no model of a team), reminders,
//! and timeouts for a referral that has had no reply (PC-3).

use crate::ids;
use crate::page::Page;
use crate::patients_store::StoreError;
use crate::referral::State;
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
    /// The state that put it here.
    pub state: State,
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

/// The part of the query shared by the list and the count: this person's referrals, current version in
/// [`ATTENTION`], not acknowledged.
fn waiting() -> String {
    format!(
        "FROM referral_current c
         JOIN referral_versions first ON first.referral_id = c.referral_id AND first.version = 1
         LEFT JOIN referral_acknowledgements a ON a.referral_id = c.referral_id AND a.version = c.version
         WHERE first.author = $1 AND a.referral_id IS NULL AND c.state IN ({})",
        attention_sql()
    )
}

/// The referrals `by` sent that wait for them, oldest first, up to [`LIST_LIMIT`]; `more` says whether any were
/// left behind (PR-1).
///
/// # Errors
///
/// [`StoreError::Db`].
pub async fn list(db: &impl ConnectionTrait, by: &str) -> Result<Page<Item>, StoreError> {
    let rows = db
        .query_all_raw(pg(
            &format!(
                "SELECT c.referral_id::text AS id, c.patient_id::text AS patient_id, c.version, c.state,
                        to_char(c.recorded_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"') AS at
                 {} ORDER BY c.recorded_at, c.referral_id LIMIT {}",
                waiting(),
                LIST_LIMIT + 1
            ),
            vec![by.into()],
        ))
        .await
        .map_err(db_err)?;
    let mut items = Vec::with_capacity(rows.len());
    for row in &rows {
        let word: String = row.try_get("", "state").map_err(db_err)?;
        let state = ATTENTION
            .into_iter()
            .find(|s| s.as_str() == word)
            .ok_or_else(|| db_err(format!("unexpected state {word}")))?;
        items.push(Item {
            referral_id: row.try_get("", "id").map_err(db_err)?,
            patient_id: row.try_get("", "patient_id").map_err(db_err)?,
            version: row.try_get("", "version").map_err(db_err)?,
            state,
            recorded_at: row.try_get("", "at").map_err(db_err)?,
        });
    }
    Ok(Page::cut(items, LIST_LIMIT))
}

/// How many referrals wait for `by`.
///
/// # Errors
///
/// [`StoreError::Db`].
pub async fn count(db: &impl ConnectionTrait, by: &str) -> Result<i64, StoreError> {
    let row = db
        .query_one_raw(pg(
            &format!("SELECT count(*) AS n {}", waiting()),
            vec![by.into()],
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
