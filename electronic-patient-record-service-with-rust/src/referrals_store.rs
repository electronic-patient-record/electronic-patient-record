//! Referrals (CA-3, first slice): the append-only store behind the state table in [`crate::referral`].
//! Every step is a new version of the referral with the event that caused it; nothing is edited or
//! deleted, and the database refuses both. There is no delete function here, on purpose. There is no
//! route or screen yet: only this store and its tests.
//!
//! The store enforces what the pure table cannot: the **actor** of each event (the referrer asks for a
//! cancellation, only the recipient confirms it), a **note** where the event needs one (a decline, an
//! outcome, an interim note, a cancel request, ...), a stale `expected_version` as a conflict, and a
//! patient that exists. Whether the caller may act at all is the policy engine's decision, made before
//! this is called.

use crate::ids;
use crate::patients_store::StoreError;
use crate::referral::{self, Actor, Event, State};
use crate::store::pg;
use chrono::{DateTime, SecondsFormat, Utc};
use sea_orm::{ConnectionTrait, TransactionTrait};
use serde::{Deserialize, Serialize};

crate::store_errors!("referrals");

/// The most characters in a recipient name, a reason, a question or a note.
const MAX_SHORT: usize = 200;
const MAX_TEXT: usize = 2000;
/// The most external identifiers on one referral.
const MAX_EXTERNAL: usize = 10;

/// How soon the referrer wants the recipient to act. The words are FHIR's `ServiceRequest.priority`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Priority {
    /// Routine.
    Routine,
    /// Urgent.
    Urgent,
    /// As soon as possible.
    Asap,
    /// Immediately.
    Stat,
}

/// An identifier the referral also carries, such as a national booking reference. The system is a pack's
/// to define; the core does not name any.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalId {
    /// Whose identifier it is.
    pub system: String,
    /// The value.
    pub value: String,
}

/// The request itself. It does not change after it is made; the steps are in the versions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Referral {
    /// The service or clinician the request goes to, as the referrer names it.
    pub recipient: String,
    /// Why the patient is referred.
    pub reason: String,
    /// What the referrer wants to know or have done.
    pub question: String,
    /// How soon.
    pub priority: Priority,
    /// Other identifiers for this referral.
    #[serde(default)]
    pub external_ids: Vec<ExternalId>,
}

/// An event the state table allows from a referral's current state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NextEvent {
    /// The event word to send.
    pub event: &'static str,
    /// Who sends it: `referrer` or `recipient`.
    pub by: &'static str,
    /// Whether it must come with a note.
    pub needs_note: bool,
}

/// The events [`referral::next`] allows from `state`, in the order of [`Event::ALL`]. Read from the table
/// itself, so a screen never carries its own copy.
#[must_use]
pub fn allowed_events(state: State) -> Vec<NextEvent> {
    Event::ALL
        .into_iter()
        .filter(|e| referral::next(state, *e).is_ok())
        .map(|e| NextEvent {
            event: e.as_str(),
            by: match e.actor() {
                Actor::Referrer => "referrer",
                Actor::Recipient => "recipient",
            },
            needs_note: needs_note(e),
        })
        .collect()
}

/// One referral as stored.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StoredReferral {
    /// The referral id: server-assigned, set once, carried by every later message.
    pub id: String,
    /// 1 for the request as made.
    pub version: i32,
    /// Who wrote this version.
    pub author: String,
    /// When this version was written (UTC).
    pub recorded_at: String,
    /// The patient.
    pub patient_id: String,
    /// Where the referral is.
    pub state: State,
    /// The request.
    pub referral: Referral,
    /// What happened in this version: the event, who, the role, when, and the note.
    pub last_event: serde_json::Value,
    /// What the state table allows next. Empty in an end state.
    pub next: Vec<NextEvent>,
}

const TS: &str = "to_char(recorded_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"')";

/// The columns every referral read selects, aliased to the names [`row_to_referral`] reads. `at` is the
/// version's time in UTC to the microsecond.
fn columns() -> String {
    format!(
        "referral_id::text AS id, version, author, patient_id::text AS patient_id, state, {TS} AS at, event, body"
    )
}

/// Read one row into a [`StoredReferral`], working out the events the state table allows next ([`allowed_events`]).
///
/// # Errors
///
/// [`StoreError::Db`] when a column is missing or a stored word is not a known state.
fn row_to_referral(row: &sea_orm::QueryResult) -> Result<StoredReferral, StoreError> {
    let body: serde_json::Value = row.try_get("", "body").map_err(db_err)?;
    let state: String = row.try_get("", "state").map_err(db_err)?;
    let state_value: State =
        serde_json::from_value(serde_json::Value::String(state)).map_err(db_err)?;
    Ok(StoredReferral {
        id: row.try_get("", "id").map_err(db_err)?,
        version: row.try_get("", "version").map_err(db_err)?,
        author: row.try_get("", "author").map_err(db_err)?,
        recorded_at: row.try_get("", "at").map_err(db_err)?,
        patient_id: row.try_get("", "patient_id").map_err(db_err)?,
        state: state_value,
        next: allowed_events(state_value),
        referral: serde_json::from_value(body).map_err(db_err)?,
        last_event: row.try_get("", "event").map_err(db_err)?,
    })
}

/// A refusal the writer can fix (a blank field, a missing note, a step the table does not allow): the 422 kind.
fn refuse(message: impl Into<String>) -> StoreError {
    StoreError::BadQuery(message.into())
}

/// Refuse a blank value, or one longer than `max` characters. `what` names the field in the message, which
/// never contains the value itself.
fn text(what: &str, value: &str, max: usize) -> Result<(), StoreError> {
    if value.trim().is_empty() {
        return Err(refuse(format!("the {what} must not be blank")));
    }
    if value.chars().count() > max {
        return Err(refuse(format!("the {what} is too long")));
    }
    Ok(())
}

/// Whether `event` must come with a note saying why or what.
#[must_use]
pub fn needs_note(event: Event) -> bool {
    !matches!(
        event,
        Event::Accept | Event::Book | Event::RecordNoShow | Event::ConfirmCancel | Event::Resume
    )
}

/// Check a request before it is stored: the recipient, reason and question must be present and no longer than
/// their limits, there may be at most `MAX_EXTERNAL` external identifiers, and each needs a system and a value.
///
/// # Errors
///
/// [`StoreError::BadQuery`] naming the first field that fails.
fn check(referral: &Referral) -> Result<(), StoreError> {
    text("recipient", &referral.recipient, MAX_SHORT)?;
    text("reason", &referral.reason, MAX_TEXT)?;
    text("question", &referral.question, MAX_TEXT)?;
    if referral.external_ids.len() > MAX_EXTERNAL {
        return Err(refuse("there are too many external identifiers"));
    }
    for id in &referral.external_ids {
        text("external identifier system", &id.system, MAX_SHORT)?;
        text("external identifier value", &id.value, MAX_SHORT)?;
    }
    Ok(())
}

/// Make a referral: version 1, state `requested`.
///
/// # Errors
///
/// [`StoreError::NotFound`] when the patient does not exist; [`StoreError::BadQuery`] when the request is
/// blank, too long or has too many identifiers.
pub async fn request(
    db: &impl ConnectionTrait,
    author: &str,
    patient_id: &str,
    referral: &Referral,
    now: DateTime<Utc>,
) -> Result<StoredReferral, StoreError> {
    check(referral)?;
    let patient = ids::uuid(patient_id).ok_or(StoreError::NotFound)?;
    crate::patients_store::current(db, &patient).await?;
    let row = db
        .query_one_raw(pg(
            &format!(
                "INSERT INTO referral_versions (referral_id, version, patient_id, author, state, event, body)
                 VALUES (gen_random_uuid(), 1, $1::uuid, $2, $3, $4, $5)
                 RETURNING {}",
                columns()
            ),
            vec![
                patient.into(),
                author.into(),
                State::Requested.as_str().into(),
                serde_json::json!({
                    "event": "request",
                    "by": author,
                    "role": "referrer",
                    "at": now.to_rfc3339_opts(SecondsFormat::Micros, true),
                })
                .into(),
                serde_json::to_value(referral).map_err(db_err)?.into(),
            ],
        ))
        .await
        .map_err(db_err)?
        .ok_or_else(|| StoreError::Db("insert returned no row".into()))?;
    row_to_referral(&row)
}

/// One referral's current version.
///
/// # Errors
///
/// [`StoreError::NotFound`] when there is no such referral.
pub async fn current(
    db: &impl ConnectionTrait,
    referral_id: &str,
) -> Result<StoredReferral, StoreError> {
    let id = ids::uuid(referral_id).ok_or(StoreError::NotFound)?;
    let row = db
        .query_one_raw(pg(
            &format!(
                "SELECT {} FROM referral_current WHERE referral_id = $1::uuid",
                columns()
            ),
            vec![id.into()],
        ))
        .await
        .map_err(db_err)?
        .ok_or(StoreError::NotFound)?;
    row_to_referral(&row)
}

/// A patient's referrals, current version of each, newest first.
///
/// # Errors
///
/// [`StoreError::Db`] when the read fails.
pub async fn for_patient(
    db: &impl ConnectionTrait,
    patient_id: &str,
) -> Result<Vec<StoredReferral>, StoreError> {
    let Some(id) = ids::uuid(patient_id) else {
        return Ok(Vec::new());
    };
    let rows = db
        .query_all_raw(pg(
            &format!(
                "SELECT {} FROM referral_current WHERE patient_id = $1::uuid
                 ORDER BY recorded_at DESC, referral_id",
                columns()
            ),
            vec![id.into()],
        ))
        .await
        .map_err(db_err)?;
    rows.iter().map(row_to_referral).collect()
}

/// Every version of one referral, oldest first: the whole history of the loop.
///
/// # Errors
///
/// [`StoreError::NotFound`] when there is no such referral.
pub async fn history(
    db: &impl ConnectionTrait,
    referral_id: &str,
) -> Result<Vec<StoredReferral>, StoreError> {
    let id = ids::uuid(referral_id).ok_or(StoreError::NotFound)?;
    let rows = db
        .query_all_raw(pg(
            &format!(
                "SELECT {} FROM referral_versions WHERE referral_id = $1::uuid ORDER BY version",
                columns()
            ),
            vec![id.into()],
        ))
        .await
        .map_err(db_err)?;
    if rows.is_empty() {
        return Err(StoreError::NotFound);
    }
    rows.iter().map(row_to_referral).collect()
}

/// One step of the loop: who sends what, with the note it needs.
#[derive(Debug, Clone, Copy)]
pub struct Step<'a> {
    /// The role the sender acts in.
    pub actor: Actor,
    /// The sender: the token `sub`.
    pub by: &'a str,
    /// What happens.
    pub event: Event,
    /// Why or what, for the events that [`needs_note`].
    pub note: Option<&'a str>,
}

/// Apply a [`Step`] to a referral and append the result as the next version.
///
/// # Errors
///
/// [`StoreError::NotFound`]; [`StoreError::Conflict`] (a stale or racing version);
/// [`StoreError::BadQuery`] (the actor is not the one this event expects, the state table does not allow
/// it, or a needed note is missing or too long).
pub async fn advance(
    db: &(impl ConnectionTrait + TransactionTrait),
    referral_id: &str,
    expected_version: i32,
    step: Step<'_>,
    now: DateTime<Utc>,
) -> Result<StoredReferral, StoreError> {
    let Step {
        actor,
        by,
        event,
        note,
    } = step;
    let was = current(db, referral_id).await?;
    if was.version != expected_version {
        return Err(StoreError::Conflict {
            current: was.version,
        });
    }
    if event.actor() != actor {
        return Err(refuse(format!(
            "only the {} can send {}",
            match event.actor() {
                Actor::Referrer => "referrer",
                Actor::Recipient => "recipient",
            },
            event.as_str()
        )));
    }
    match (
        needs_note(event),
        note.map(str::trim).filter(|n| !n.is_empty()),
    ) {
        (true, None) => return Err(refuse(format!("{} needs a note", event.as_str()))),
        (true, Some(n)) => text("note", n, MAX_TEXT)?,
        (false, _) => {}
    }
    let state = referral::next(was.state, event).map_err(|r| refuse(r.to_string()))?;
    let logged = serde_json::json!({
        "event": event.as_str(),
        "by": by,
        "role": match actor { Actor::Referrer => "referrer", Actor::Recipient => "recipient" },
        "at": now.to_rfc3339_opts(SecondsFormat::Micros, true),
        "note": note.map(str::trim).filter(|n| !n.is_empty()),
    });
    let appended = crate::store::append_version(
        db,
        STORE,
        pg(
            &format!(
                "INSERT INTO referral_versions (referral_id, version, patient_id, author, state, event, body)
                 VALUES ($1::uuid, $2, $3::uuid, $4, $5, $6, $7)
                 RETURNING {}",
                columns()
            ),
            vec![
                was.id.clone().into(),
                (expected_version + 1).into(),
                was.patient_id.clone().into(),
                by.into(),
                state.as_str().into(),
                logged.into(),
                serde_json::to_value(&was.referral).map_err(db_err)?.into(),
            ],
        ),
    )
    .await?;
    match appended {
        crate::store::Appended::Row(row) => row_to_referral(&row),
        crate::store::Appended::Taken => {
            let current = current(db, referral_id).await?.version;
            Err(StoreError::Conflict { current })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_allowed_events_are_the_state_tables() {
        for state in State::ALL {
            let listed: Vec<&str> = allowed_events(state).iter().map(|n| n.event).collect();
            let table: Vec<&str> = Event::ALL
                .into_iter()
                .filter(|e| referral::next(state, *e).is_ok())
                .map(Event::as_str)
                .collect();
            assert_eq!(listed, table, "{state:?}");
            assert_eq!(listed.is_empty(), state.is_end(), "{state:?}");
        }
        let requested = allowed_events(State::Requested);
        assert!(
            requested
                .iter()
                .any(|n| n.event == "accept" && n.by == "recipient" && !n.needs_note)
        );
        assert!(
            requested
                .iter()
                .any(|n| n.event == "request-cancel" && n.by == "referrer" && n.needs_note)
        );
    }

    fn referral() -> Referral {
        Referral {
            recipient: "Cardiology outpatients".into(),
            reason: "Exertional chest pain".into(),
            question: "Is this angina?".into(),
            priority: Priority::Routine,
            external_ids: vec![],
        }
    }

    #[test]
    fn a_blank_or_oversize_request_is_refused() {
        assert!(check(&referral()).is_ok());
        for bad in [
            Referral {
                recipient: " ".into(),
                ..referral()
            },
            Referral {
                reason: String::new(),
                ..referral()
            },
            Referral {
                question: "q".repeat(MAX_TEXT + 1),
                ..referral()
            },
            Referral {
                external_ids: vec![ExternalId {
                    system: "s".into(),
                    value: " ".into(),
                }],
                ..referral()
            },
            Referral {
                external_ids: (0..=MAX_EXTERNAL)
                    .map(|n| ExternalId {
                        system: "s".into(),
                        value: n.to_string(),
                    })
                    .collect(),
                ..referral()
            },
        ] {
            assert!(
                matches!(check(&bad), Err(StoreError::BadQuery(_))),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn the_events_that_need_a_note_are_the_ones_that_say_something() {
        let without: Vec<_> = Event::ALL.into_iter().filter(|e| !needs_note(*e)).collect();
        assert_eq!(
            without,
            [
                Event::Accept,
                Event::Book,
                Event::RecordNoShow,
                Event::ConfirmCancel,
                Event::Resume
            ]
        );
    }

    #[test]
    fn priority_words_are_fhirs() {
        for (p, word) in [
            (Priority::Routine, "routine"),
            (Priority::Urgent, "urgent"),
            (Priority::Asap, "asap"),
            (Priority::Stat, "stat"),
        ] {
            assert_eq!(serde_json::to_value(p).unwrap(), serde_json::json!(word));
        }
    }
}
