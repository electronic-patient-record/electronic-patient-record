//! Refusals of treatment, stored (RF-5): append-only versions, optimistic updates.
//!
//! The same pattern as `consents_store`, for a different thing: this is a refusal of **treatment**, and
//! `consents_store` holds a refusal of disclosure (RF-2). A change is a new version row; a stale `expected_version` is a
//! [`StoreError::Conflict`]. Nothing here says whether a person may be treated or whether a refusal is valid
//! ([`crate::treatment_refusal`] has no such function), and "none recorded" is never "none".
//!
//! **A merge and an unmerge stop for each refusal** ([`unseen_on_merge`], [`unseen_on_unmerge`]): the caller must name
//! every one that a person looked at, and a refusal the person did not name is a refusal that was not asked about. The
//! refusals of a record merged away are shown with the survivor ([`list_with_merged`]), as what they are: another
//! record's.
//!
//! **A doubt raises a task event** (TL-3, RF-7): a refusal coming to be doubted raises `treatment-refusal-doubted`, and a
//! doubted one leaving doubt raises `treatment-refusal-resolved` ([`treatment_refusal::task_event`]), in the writer's
//! transaction, with the refusal as the task's source. Only an approved rule makes a task from it.

use crate::ids;
use crate::patients_store::{self, StoreError};
use crate::pool::Db;
use crate::task_rules::Rule;
use crate::tasks_store::{self, Source};
use crate::treatment_refusal::{self, RefusalRecord, State};
use serde::Serialize;
use sqlx::postgres::PgRow;
use sqlx::{AssertSqlSafe, Connection, PgPool, Postgres, Row, Transaction};

crate::store_errors!("treatment_refusals");

/// A refusal as stored: the record and its place in the history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StoredRefusal {
    /// The server-assigned id (a UUID), the same for every version.
    pub id: String,
    /// The patient record it is recorded on.
    pub patient_id: String,
    /// 1 for the first version.
    pub version: i32,
    /// Who wrote this version.
    pub author: String,
    /// The record.
    pub refusal: RefusalRecord,
}

const COLUMNS: &str =
    "refusal_id::text AS id, patient_id::text AS patient_id, version, author, body";

const ENDED: &str = "a refusal entered in error is not changed: record a new one";
const WITHDRAWN_NEEDS_A_RULE: &str = "no rule in force says that this withdrawal ends a refusal: record it as doubted, with the withdrawal";

fn row_to_stored(row: &PgRow) -> Result<StoredRefusal, StoreError> {
    let body: serde_json::Value = row.try_get("body").map_err(db_err)?;
    Ok(StoredRefusal {
        id: row.try_get("id").map_err(db_err)?,
        patient_id: row.try_get("patient_id").map_err(db_err)?,
        version: row.try_get("version").map_err(db_err)?,
        author: row.try_get("author").map_err(db_err)?,
        refusal: serde_json::from_value(body).map_err(db_err)?,
    })
}

/// A withdrawal that was heard ends a refusal only where a rule in force says so ([`treatment_refusal::after_withdrawal`]).
/// No rule is loaded by the service yet, so a refusal is never stored as withdrawn.
fn check_withdrawal(record: &RefusalRecord) -> Result<(), StoreError> {
    if record.state == State::Withdrawn
        && let Some(w) = &record.withdrawal
        && treatment_refusal::after_withdrawal(w.how, None) != State::Withdrawn
    {
        return Err(StoreError::BadQuery(WITHDRAWN_NEEDS_A_RULE.into()));
    }
    Ok(())
}

/// Record a new refusal for a patient, at version 1.
///
/// # Errors
///
/// [`StoreError::NotFound`] when the patient does not exist; [`StoreError::BadQuery`] for a withdrawal no rule lets end
/// it, or one recorded as entered in error to begin with; [`StoreError::Db`] when the insert fails.
pub async fn create(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    author: &str,
    reason: Option<&str>,
    record: &RefusalRecord,
) -> Result<StoredRefusal, StoreError> {
    create_with_rules(
        db,
        patient_id,
        author,
        reason,
        record,
        &tasks_store::rules()?,
    )
    .await
}

/// [`create`] with the task rules given, which a test sets. One recorded as doubted to begin with raises
/// `treatment-refusal-doubted`.
///
/// # Errors
///
/// As [`create`].
pub async fn create_with_rules(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    author: &str,
    reason: Option<&str>,
    record: &RefusalRecord,
    rules: &[Rule],
) -> Result<StoredRefusal, StoreError> {
    check_withdrawal(record)?;
    if record.state == State::EnteredInError {
        return Err(StoreError::BadQuery(
            "a refusal is not recorded as entered in error: it is marked so afterwards".into(),
        ));
    }
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let patient = patients_store::current(&mut *conn, patient_id).await?;
    let mut tx = conn.begin().await.map_err(db_err)?;
    // The same key as [`update`] and the merge checks: a merge that is asking about this record's refusals waits for
    // this one, or this one waits for the merge.
    crate::store::one_at_a_time(&mut tx, STORE, &patient.id).await?;
    let row = sqlx::query(AssertSqlSafe(format!(
        "INSERT INTO treatment_refusal_versions
                   (refusal_id, version, patient_id, author, reason, state, kind, identity, body)
                 VALUES (gen_random_uuid(), 1, $1::uuid, $2, $3, $4, $5, $6, $7) RETURNING {COLUMNS}"
    )))
    .bind(patient.id)
    .bind(author)
    .bind(reason.map(str::to_string))
    .bind(record.state.as_str())
    .bind(record.kind.as_str())
    .bind(record.identity.as_str())
    .bind(serde_json::to_value(record).map_err(db_err)?)
    .fetch_optional(&mut *tx)
    .await
    .map_err(db_err)?
    .ok_or_else(|| StoreError::Db("insert returned no row".into()))?;
    let stored = row_to_stored(&row)?;
    raise(&mut tx, rules, None, &stored).await?;
    tx.commit().await.map_err(db_err)?;
    Ok(stored)
}

/// Raise the task event a change from `before` to `stored` makes, if any ([`treatment_refusal::task_event`]), on the
/// writer's connection.
async fn raise(
    conn: &mut sqlx::PgConnection,
    rules: &[Rule],
    before: Option<State>,
    stored: &StoredRefusal,
) -> Result<(), StoreError> {
    if let Some((event, facts)) = treatment_refusal::task_event(before, &stored.refusal) {
        let source = Source {
            kind: "treatment-refusal".into(),
            id: stored.id.clone(),
            version: None,
        };
        tasks_store::raise(conn, rules, event, &facts, &stored.patient_id, &source).await?;
    }
    Ok(())
}

/// The current version of every refusal recorded on a patient record, those entered in error and withdrawn included,
/// oldest first.
///
/// # Errors
///
/// [`StoreError::Db`] on a database failure.
pub async fn list(db: &PgPool, patient_id: &str) -> Result<Vec<StoredRefusal>, StoreError> {
    let Some(patient_id) = ids::uuid(patient_id) else {
        return Ok(Vec::new());
    };
    let rows = sqlx::query(AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM treatment_refusal_current
                 WHERE patient_id = $1::uuid ORDER BY recorded_at, refusal_id"
    )))
    .bind(patient_id)
    .fetch_all(db)
    .await
    .map_err(db_err)?;
    rows.iter().map(row_to_stored).collect()
}

/// The refusals of a patient record and of every record merged into it. A refusal does not lapse because two records
/// were joined; its `patient_id` says whose record it is on.
///
/// # Errors
///
/// [`StoreError::Db`] on a database failure.
pub async fn list_with_merged(
    db: &PgPool,
    patient_id: &str,
) -> Result<Vec<StoredRefusal>, StoreError> {
    let Some(own) = ids::uuid(patient_id) else {
        return Ok(Vec::new());
    };
    let mut records = crate::links::sources_of(db, &own).await?;
    records.push(own.clone());
    // Two queries however long the chain of merges: the record's own first, then the others', each oldest first.
    let rows = sqlx::query(AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM treatment_refusal_current
                 WHERE patient_id = ANY($1::uuid[])
                 ORDER BY patient_id <> $2::uuid, recorded_at, refusal_id"
    )))
    .bind(records)
    .bind(own)
    .fetch_all(db)
    .await
    .map_err(db_err)?;
    rows.iter().map(row_to_stored).collect()
}

/// The current version of one refusal of one patient record.
///
/// # Errors
///
/// [`StoreError::NotFound`] when there is no such refusal on that record.
pub async fn current(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    refusal_id: &str,
) -> Result<StoredRefusal, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let (Some(patient_id), Some(refusal_id)) = (ids::uuid(patient_id), ids::uuid(refusal_id))
    else {
        return Err(StoreError::NotFound);
    };
    let row = sqlx::query(AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM treatment_refusal_current
                 WHERE refusal_id = $1::uuid AND patient_id = $2::uuid"
    )))
    .bind(refusal_id)
    .bind(patient_id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db_err)?
    .ok_or(StoreError::NotFound)?;
    row_to_stored(&row)
}

/// Append version `expected_version + 1`. Earlier versions stay. A doubt, a withdrawal that was heard and "entered in
/// error" are this call with another state.
///
/// # Errors
///
/// [`StoreError::NotFound`] for an unknown refusal or one on another record; [`StoreError::Conflict`] when
/// `expected_version` is not the current version, including a concurrent writer; [`StoreError::BadQuery`] for one
/// already entered in error, or a withdrawal no rule lets end it.
pub async fn update(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    refusal_id: &str,
    expected_version: i32,
    author: &str,
    reason: Option<&str>,
    record: &RefusalRecord,
) -> Result<StoredRefusal, StoreError> {
    let edit = Edit {
        patient_id,
        refusal_id,
        expected_version,
        author,
        reason,
        record,
    };
    update_with_rules(db, &edit, &tasks_store::rules()?).await
}

/// One change to a refusal, as [`update`] takes it.
#[derive(Debug, Clone, Copy)]
pub struct Edit<'a> {
    /// The patient record it is on.
    pub patient_id: &'a str,
    /// The refusal.
    pub refusal_id: &'a str,
    /// The version the change was made from.
    pub expected_version: i32,
    /// Who.
    pub author: &'a str,
    /// Why, for the version history.
    pub reason: Option<&'a str>,
    /// The whole new record.
    pub record: &'a RefusalRecord,
}

/// [`update`] with the task rules given, which a test sets: a change into doubt raises `treatment-refusal-doubted`, and
/// out of it `treatment-refusal-resolved`.
///
/// # Errors
///
/// As [`update`].
pub async fn update_with_rules(
    db: impl Into<Db<'_>>,
    edit: &Edit<'_>,
    rules: &[Rule],
) -> Result<StoredRefusal, StoreError> {
    let Edit {
        patient_id,
        refusal_id,
        expected_version,
        author,
        reason,
        record,
    } = *edit;
    check_withdrawal(record)?;
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let mut tx = conn.begin().await.map_err(db_err)?;
    crate::store::one_at_a_time(&mut tx, STORE, &patient_id.to_ascii_lowercase()).await?;
    let db = &mut *tx;
    let now = current(&mut *db, patient_id, refusal_id).await?;
    if now.version != expected_version {
        return Err(StoreError::Conflict {
            current: now.version,
        });
    }
    if now.refusal.state == State::EnteredInError {
        return Err(StoreError::BadQuery(ENDED.into()));
    }
    let new_version = crate::store::next_version(expected_version)?;
    let row = match crate::store::append_version(
        &mut *db,
        STORE,
        sqlx::query(AssertSqlSafe(format!(
            "INSERT INTO treatment_refusal_versions
                   (refusal_id, version, patient_id, author, reason, state, kind, identity, body)
                 VALUES ($1::uuid, $2, $3::uuid, $4, $5, $6, $7, $8, $9) RETURNING {COLUMNS}"
        )))
        .bind(now.id.clone())
        .bind(new_version)
        .bind(now.patient_id.clone())
        .bind(author)
        .bind(reason.map(str::to_string))
        .bind(record.state.as_str())
        .bind(record.kind.as_str())
        .bind(record.identity.as_str())
        .bind(serde_json::to_value(record).map_err(db_err)?),
    )
    .await?
    {
        crate::store::Appended::Row(row) => row,
        crate::store::Appended::Taken => {
            let current = current(&mut *db, patient_id, refusal_id).await?.version;
            return Err(StoreError::Conflict { current });
        }
    };
    let stored = row_to_stored(&row)?;
    raise(&mut *db, rules, Some(now.refusal.state), &stored).await?;
    tx.commit().await.map_err(db_err)?;
    Ok(stored)
}

/// The refusals recorded on `source`, not entered in error, that `seen` does not name: what a merge of `source` into
/// another record has not yet asked a person about. Withdrawn ones are asked about too, because whether a withdrawal ended
/// a refusal is the place's rule and not the software's.
///
/// # Errors
///
/// [`StoreError::Db`] on a database failure.
pub async fn unseen_on_merge(
    tx: &mut Transaction<'_, Postgres>,
    source: &str,
    seen: &[String],
) -> Result<Vec<String>, StoreError> {
    let Some(source) = ids::uuid(source) else {
        return Ok(Vec::new());
    };
    // Until the merge ends, no refusal can be recorded on the record merged away, so none arrives after this answer.
    crate::store::one_at_a_time(tx, STORE, &source).await?;
    let seen = ids::uuids(seen);
    let rows = sqlx::query(
        "SELECT refusal_id::text AS id FROM treatment_refusal_current
                 WHERE patient_id = $1::uuid AND state <> 'entered-in-error' AND NOT (refusal_id = ANY($2::uuid[]))
                 ORDER BY recorded_at, refusal_id",
    )
    .bind(source)
    .bind(seen)
    .fetch_all(&mut **tx)
    .await
    .map_err(db_err)?;
    rows.iter()
        .map(|r| r.try_get("id").map_err(db_err))
        .collect()
}

/// The refusals recorded on `target` **since** `source` was merged into it, not entered in error, that `seen` does not
/// name: any of them may be the other person's. Empty when `source` is not merged into `target` (the unmerge then fails
/// on its own).
///
/// # Errors
///
/// [`StoreError::Db`] on a database failure.
pub async fn unseen_on_unmerge(
    tx: &mut Transaction<'_, Postgres>,
    target: &str,
    source: &str,
    seen: &[String],
) -> Result<Vec<String>, StoreError> {
    let (Some(target), Some(source)) = (ids::uuid(target), ids::uuid(source)) else {
        return Ok(Vec::new());
    };
    // Until the unmerge ends, no refusal can be recorded on the survivor, so none arrives after this answer.
    crate::store::one_at_a_time(tx, STORE, &target).await?;
    let seen = ids::uuids(seen);
    let rows = sqlx::query(
        "SELECT c.refusal_id::text AS id FROM treatment_refusal_current c
                 WHERE c.patient_id = $1::uuid AND c.state <> 'entered-in-error' AND NOT (c.refusal_id = ANY($3::uuid[]))
                   AND (SELECT min(v.recorded_at) FROM treatment_refusal_versions v WHERE v.refusal_id = c.refusal_id)
                     > (SELECT min(l.recorded_at) FROM patient_link_versions l
                         WHERE l.link_id IN (SELECT link_id FROM patient_link_current
                                              WHERE source_patient_id = $2::uuid AND target_patient_id = $1::uuid
                                                AND status = 'active'))
                 ORDER BY c.recorded_at, c.refusal_id",
    )
    .bind(target)
    .bind(source)
    .bind(seen)
    .fetch_all(&mut **tx)
    .await
    .map_err(db_err)?;
    rows.iter()
        .map(|r| r.try_get("id").map_err(db_err))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::treatment_refusal::{
        Form, How, IdentityOf, Kind, Known, LifeAtRisk, Seen, Speaker, Withdrawal,
    };

    fn heard(state: State) -> RefusalRecord {
        RefusalRecord {
            kind: Kind::SpokenNow,
            treatment: "blood".into(),
            coding: None,
            accepts: None,
            circumstances: None,
            if_life_at_risk: LifeAtRisk::NotStated,
            form: Form {
                written: Known::No,
                signed: Known::No,
                witnessed: Known::No,
                witnesses: None,
                seen: Seen::NotSeen,
                kept: None,
            },
            made_on: None,
            confirmed_on: None,
            said_by: Speaker::Patient,
            power_seen: None,
            identity: IdentityOf::Certain,
            state,
            doubt: None,
            withdrawal: Some(Withdrawal {
                how: How::Spoken,
                noticed_by: "a nurse".into(),
                at: "2026-01-01T10:00:00Z".into(),
            }),
        }
    }

    #[test]
    fn with_no_rule_a_withdrawal_that_was_heard_is_doubted_and_never_ends_a_refusal() {
        assert!(matches!(
            check_withdrawal(&heard(State::Withdrawn)),
            Err(StoreError::BadQuery(m)) if m == WITHDRAWN_NEEDS_A_RULE
        ));
        for state in [State::Doubted, State::EnteredInError] {
            assert!(check_withdrawal(&heard(state)).is_ok(), "{state:?}");
        }
        // With no withdrawal there is nothing to check (`validate` refuses withdrawn without one).
        let mut none = heard(State::Withdrawn);
        none.withdrawal = None;
        assert!(check_withdrawal(&none).is_ok());
    }
}
