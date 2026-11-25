//! Where the security labels of an entry are kept (EH-8): the table `entry_labels`, one row for each thing that happened.
//! Who may add and lift one is [`crate::labelling`]'s rule; what a label withholds is `entries_store.rs`'s. Nothing here
//! decides either. A row that cannot be read fails the read: a label that was lost would show what it hides.

use crate::ids;
use crate::labelling::{Act, Basis};
use crate::patients_store::StoreError;
use serde::Serialize;
use sqlx::{PgConnection, Row};
use std::collections::BTreeMap;

const STORE: &str = "entry labels";

/// One thing that happened to a label of an entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StoredAct {
    /// 1 for the first on the entry.
    pub seq: i32,
    /// When (UTC).
    pub recorded_at: String,
    /// What.
    #[serde(flatten)]
    pub act: Act,
}

fn failed<E: std::fmt::Display + 'static>(e: E) -> StoreError {
    crate::store::db_err(STORE, e)
}

fn act_of(row: &sqlx::postgres::PgRow) -> Result<Act, StoreError> {
    let word: String = row.try_get("act").map_err(failed)?;
    let label: String = row.try_get("label").map_err(failed)?;
    let by: Option<String> = row.try_get("author").map_err(failed)?;
    let reason: Option<String> = row.try_get("reason").map_err(failed)?;
    let basis: String = row.try_get("basis").map_err(failed)?;
    let basis = Basis::parse(&basis).ok_or_else(|| failed("unknown stored basis"))?;
    match (word.as_str(), by, reason) {
        ("added", by, None) => Ok(Act::Added { label, by, basis }),
        ("lifted", Some(by), Some(reason)) => Ok(Act::Lifted {
            label,
            by,
            basis,
            reason,
        }),
        _ => Err(failed("unreadable stored label")),
    }
}

/// Everything that happened to the labels of the entries of one kind that are on these records, by entry id, oldest first.
///
/// # Errors
///
/// A database error, or a row that cannot be read.
pub async fn of_patients(
    db: &mut PgConnection,
    kind: &str,
    patient_ids: &[String],
) -> Result<BTreeMap<String, Vec<StoredAct>>, StoreError> {
    let rows = sqlx::query(
        "SELECT entry_id::text AS entry_id, seq, act, label, author, basis, reason,
                to_char(recorded_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"') AS at
         FROM entry_labels WHERE kind = $1 AND patient_id = ANY($2::uuid[]) ORDER BY entry_id, seq",
    )
    .bind(kind.to_string())
    .bind(ids::uuids(patient_ids))
    .fetch_all(db)
    .await
    .map_err(failed)?;
    let mut out: BTreeMap<String, Vec<StoredAct>> = BTreeMap::new();
    for row in &rows {
        let entry: String = row.try_get("entry_id").map_err(failed)?;
        out.entry(entry).or_default().push(StoredAct {
            seq: row.try_get("seq").map_err(failed)?,
            recorded_at: row.try_get("at").map_err(failed)?,
            act: act_of(row)?,
        });
    }
    Ok(out)
}

/// Keep `act` as the next on `entry_id`. The caller holds the lock on the patient's list.
///
/// # Errors
///
/// A database error.
pub async fn append(
    db: &mut PgConnection,
    kind: &str,
    entry_id: &str,
    patient_id: &str,
    act: &Act,
) -> Result<(), StoreError> {
    let (word, label, author, basis, reason) = match act {
        Act::Added { label, by, basis } => ("added", label, by.clone(), basis, None),
        Act::Lifted {
            label,
            by,
            basis,
            reason,
        } => (
            "lifted",
            label,
            Some(by.clone()),
            basis,
            Some(reason.clone()),
        ),
    };
    sqlx::query(
        "INSERT INTO entry_labels (kind, entry_id, seq, patient_id, act, label, author, basis, reason)
         SELECT $1, $2::uuid, COALESCE(MAX(seq), 0) + 1, $3::uuid, $4, $5, $6, $7, $8
         FROM entry_labels WHERE kind = $1 AND entry_id = $2::uuid",
    )
    .bind(kind.to_string())
    .bind(entry_id.to_string())
    .bind(patient_id.to_string())
    .bind(word)
    .bind(label.clone())
    .bind(author)
    .bind(basis.word())
    .bind(reason)
    .execute(db)
    .await
    .map(|_| ())
    .map_err(failed)
}
