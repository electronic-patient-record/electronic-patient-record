//! The append-only store behind the problem list (EH-1) and family history (EH-3), and the read **as at a time** (EH-2).
//! Both are the same shape (an entry with versions, each a whole record), so they share one store; what differs is an
//! [`Entry`]: its table, its rules, and what its list amounts to.
//!
//! Nothing is ever deleted: ending an entry, changing one's mind, or marking it entered in error is a new version, and
//! entered-in-error is final. A list follows merges: a person's entries do not disappear because two records were
//! joined. **As at** a time, it follows the merges that stood *then*, so the answer is what the record held at that
//! time and not today's record with the later lines left out.

#![deny(
    clippy::arithmetic_side_effects,
    clippy::default_numeric_fallback,
    clippy::integer_division,
    clippy::modulo_arithmetic,
    clippy::as_conversions,
    clippy::string_slice
)]

use crate::family_history::{self, FamilyHistoryRecord};
use crate::ids;
use crate::interop::terminology;
use crate::links;
use crate::patients_store::{self, StoreError};
use crate::pg_time::PgTimestamp;
use crate::policy::Resource;
use crate::pool::Db;
use crate::problem::{self, ProblemRecord};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sqlx::postgres::PgRow;
use sqlx::{AssertSqlSafe, PgConnection, Row};

/// The longest reason kept with a version.
pub const REASON_LIMIT: usize = 500;

/// A kind of entry: where it is kept, who may see it, and its rules.
pub trait Entry: Clone + Serialize + DeserializeOwned + Send + Sync + 'static {
    /// The store's name in its errors.
    const STORE: &'static str;
    /// The versions table.
    const TABLE: &'static str;
    /// The current view.
    const VIEW: &'static str;
    /// The id column.
    const ID: &'static str;
    /// The policy resource, whose name is also the audit row's.
    const RESOURCE: Resource;

    /// The stored word for the record's status.
    fn status(&self) -> &'static str;
    /// Set the status from its stored word; `false` for a word the service does not write.
    fn set_status(&mut self, word: &str) -> bool;
    /// Whether it is entered in error.
    fn in_error(&self) -> bool;
    /// What is wrong with the record on its own. The words name a field and never quote it.
    fn faults(&self) -> Vec<String>;
    /// What is wrong with the record beside the rest of the list. `own` is its id, or `None` for a new one.
    fn faults_in(&self, _own: Option<&str>, _all: &[Stored<Self>]) -> Vec<String> {
        Vec::new()
    }
    /// What a list amounts to, as JSON with a `state`.
    fn summary(all: &[Self]) -> serde_json::Value;
}

/// A coding whose SNOMED CT identifier, if it claims to be one, is not one. The words do not quote it.
fn coding_fault(c: Option<&crate::coding::Coding>) -> Option<String> {
    c.filter(|c| terminology::validate_coding(&c.system, &c.code).is_err())
        .map(|_| "the code is not a SNOMED CT concept identifier".to_string())
}

impl Entry for ProblemRecord {
    const STORE: &'static str = "problems";
    const TABLE: &'static str = "problem_versions";
    const VIEW: &'static str = "problem_current";
    const ID: &'static str = "problem_id";
    const RESOURCE: Resource = Resource::Condition;

    fn status(&self) -> &'static str {
        self.status.as_str()
    }

    fn set_status(&mut self, word: &str) -> bool {
        let parsed = problem::Status::parse(word);
        if let Some(s) = parsed {
            self.status = s;
        }
        parsed.is_some()
    }

    fn in_error(&self) -> bool {
        self.status == problem::Status::EnteredInError
    }

    fn faults(&self) -> Vec<String> {
        let mut out = problem::validate(self, crate::site::today());
        out.extend(coding_fault(self.coding.as_ref()));
        out
    }

    fn faults_in(&self, own: Option<&str>, all: &[Stored<Self>]) -> Vec<String> {
        let links: Vec<problem::Link> = all
            .iter()
            .map(|s| problem::Link {
                id: s.id.clone(),
                standing: !s.record.in_error(),
                due_to: s.record.due_to.clone(),
            })
            .collect();
        let mut out = Vec::new();
        if self.in_error() {
            // Its own cause no longer matters; what names it as a cause does.
            let named = own.map_or(0_usize, |id| problem::named_as_cause(id, &links));
            if named > 0 {
                out.push(format!(
                    "{named} problem(s) name this as their cause: change them first"
                ));
            }
        } else if let Some(cause) = &self.due_to {
            out.extend(problem::cause_problem(own, cause, &links).map(String::from));
        }
        out
    }

    fn summary(all: &[Self]) -> serde_json::Value {
        serde_json::json!(problem::summarise(all))
    }
}

impl Entry for FamilyHistoryRecord {
    const STORE: &'static str = "family history";
    const TABLE: &'static str = "family_history_versions";
    const VIEW: &'static str = "family_history_current";
    const ID: &'static str = "entry_id";
    const RESOURCE: Resource = Resource::FamilyMemberHistory;

    fn status(&self) -> &'static str {
        self.status.as_str()
    }

    fn set_status(&mut self, word: &str) -> bool {
        let parsed = family_history::Status::parse(word);
        if let Some(s) = parsed {
            self.status = s;
        }
        parsed.is_some()
    }

    fn in_error(&self) -> bool {
        self.status == family_history::Status::EnteredInError
    }

    fn faults(&self) -> Vec<String> {
        let mut out = family_history::validate(self);
        out.extend(coding_fault(self.coding.as_ref()));
        out
    }

    /// `recorded` with a count, or `not-recorded`: an empty family history is unknown, never "nothing in the family".
    fn summary(all: &[Self]) -> serde_json::Value {
        let standing = all.iter().filter(|r| !r.in_error()).count();
        if standing == 0 {
            serde_json::json!({ "state": "not-recorded" })
        } else {
            serde_json::json!({ "state": "recorded", "recorded": standing })
        }
    }
}

/// An entry as stored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stored<R> {
    /// Server id, the same for every version.
    pub id: String,
    /// The patient record it was written on.
    pub patient_id: String,
    /// 1 for the first version.
    pub version: i32,
    /// Who wrote this version.
    pub author: String,
    /// When this version was written (UTC).
    pub recorded_at: String,
    /// Why this version was written. Required from version 2.
    pub reason: Option<String>,
    /// The record.
    pub record: R,
}

/// An entry as it stood at a past time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StoredAt<R> {
    /// The version that was current then.
    #[serde(flatten)]
    pub stored: Stored<R>,
    /// Whether a later version has been written since, so that this is no longer what the record says.
    pub changed_since: bool,
}

fn failed<R: Entry, E: std::fmt::Display + 'static>(e: E) -> StoreError {
    crate::store::db_err(R::STORE, e)
}

/// The columns every read selects, aliased to the names [`row_to`] reads.
fn columns<R: Entry>() -> String {
    format!(
        "{}::text AS id, patient_id::text AS patient_id, version, author, reason, status, body,
         to_char(recorded_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"') AS at",
        R::ID
    )
}

/// Read one row. The `status` column is the source of truth and replaces the status in the stored body.
fn row_to<R: Entry>(row: &PgRow) -> Result<Stored<R>, StoreError> {
    let body: serde_json::Value = row.try_get("body").map_err(failed::<R, _>)?;
    let status: String = row.try_get("status").map_err(failed::<R, _>)?;
    let mut record: R = serde_json::from_value(body).map_err(failed::<R, _>)?;
    if !record.set_status(&status) {
        return Err(failed::<R, _>("unknown stored status"));
    }
    Ok(Stored {
        id: row.try_get("id").map_err(failed::<R, _>)?,
        patient_id: row.try_get("patient_id").map_err(failed::<R, _>)?,
        version: row.try_get("version").map_err(failed::<R, _>)?,
        author: row.try_get("author").map_err(failed::<R, _>)?,
        recorded_at: row.try_get("at").map_err(failed::<R, _>)?,
        reason: row.try_get("reason").map_err(failed::<R, _>)?,
        record,
    })
}

/// One writer at a time on one patient's list, until the caller's transaction ends: the rules that look at the rest of
/// the list would otherwise pass for two writers at once and fail together. With no transaction it holds nothing.
async fn lock<R: Entry>(db: &mut PgConnection, patient_id: &str) -> Result<(), StoreError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(format!("{}:{}", R::TABLE, patient_id.to_lowercase()))
        .execute(db)
        .await
        .map(|_| ())
        .map_err(failed::<R, _>)
}

/// A reason that is there, not blank and not too long, or why it will not do. `required` from version 2.
fn reason_fault(reason: Option<&str>, required: bool) -> Option<String> {
    match reason {
        None if required => Some("a reason is required".into()),
        Some(r) if r.trim().is_empty() => Some("the reason must not be blank".into()),
        Some(r) if r.chars().count() > REASON_LIMIT => Some(format!(
            "the reason must be up to {REASON_LIMIT} characters"
        )),
        _ => None,
    }
}

fn refuse(problems: Vec<String>) -> Result<(), StoreError> {
    if problems.is_empty() {
        Ok(())
    } else {
        Err(StoreError::BadQuery(problems.join("; ")))
    }
}

/// Record a new entry at version 1. Call it inside a transaction: the rules that look at the rest of the list hold a
/// lock until it ends.
///
/// # Errors
///
/// [`StoreError::NotFound`] (no such patient), [`StoreError::BadQuery`] (the record is refused).
pub async fn create<R: Entry>(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    author: &str,
    reason: Option<&str>,
    record: &R,
) -> Result<Stored<R>, StoreError> {
    let mut conn = db.into().acquire().await.map_err(failed::<R, _>)?;
    let db = &mut *conn;
    let mut problems = record.faults();
    problems.extend(reason_fault(reason, false));
    if record.in_error() {
        problems.push("a new entry cannot start as entered in error".into());
    }
    refuse(problems)?;
    let patient = patients_store::current(&mut *db, patient_id).await?;
    lock::<R>(&mut *db, &patient.id).await?;
    let all = list::<R>(&mut *db, &patient.id).await?;
    refuse(record.faults_in(None, &all))?;
    let row = sqlx::query(AssertSqlSafe(format!(
        "INSERT INTO {} ({}, version, patient_id, author, reason, status, body)
             VALUES (gen_random_uuid(), 1, $1::uuid, $2, $3, $4, $5) RETURNING {}",
        R::TABLE,
        R::ID,
        columns::<R>()
    )))
    .bind(patient.id)
    .bind(author)
    .bind(reason.map(str::to_string))
    .bind(record.status())
    .bind(serde_json::to_value(record).map_err(failed::<R, _>)?)
    .fetch_optional(&mut *db)
    .await
    .map_err(failed::<R, _>)?
    .ok_or_else(|| failed::<R, _>("insert returned no row"))?;
    row_to(&row)
}

/// The current version of every entry of a patient, every status, oldest first, with those of any record merged into
/// this one.
///
/// # Errors
///
/// [`StoreError::NotFound`] (no such patient).
pub async fn list<R: Entry>(
    db: impl Into<Db<'_>>,
    patient_id: &str,
) -> Result<Vec<Stored<R>>, StoreError> {
    let mut conn = db.into().acquire().await.map_err(failed::<R, _>)?;
    let db = &mut *conn;
    let patient = patients_store::current(&mut *db, patient_id).await?;
    let linked = ids::uuids(&links::linked_ids(&mut *db, &patient.id).await?);
    let rows = sqlx::query(AssertSqlSafe(format!(
        "SELECT {} FROM {} WHERE patient_id = ANY($1::uuid[]) ORDER BY recorded_at, {}",
        columns::<R>(),
        R::VIEW,
        R::ID
    )))
    .bind(linked)
    .fetch_all(&mut *db)
    .await
    .map_err(failed::<R, _>)?;
    rows.iter().map(row_to).collect()
}

/// Every entry as it stood at `at`: for each, the latest version written by then, with the entries of the records that
/// were merged into this one **then**. An entry first written later is not there at all.
///
/// # Errors
///
/// [`StoreError::NotFound`] (no such patient now; whether it was registered at `at` is not asked).
pub async fn list_at<R: Entry>(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    at: jiff::Timestamp,
) -> Result<Vec<StoredAt<R>>, StoreError> {
    let mut conn = db.into().acquire().await.map_err(failed::<R, _>)?;
    let db = &mut *conn;
    let patient = patients_store::current(&mut *db, patient_id).await?;
    let linked = ids::uuids(&links::linked_ids_at(&mut *db, &patient.id, at).await?);
    let rows = sqlx::query(AssertSqlSafe(format!(
        "SELECT {}, EXISTS (
                 SELECT 1 FROM {table} n WHERE n.{id} = v.{id} AND n.version > v.version
             ) AS changed_since
             FROM (SELECT DISTINCT ON ({id}) * FROM {table}
                   WHERE patient_id = ANY($1::uuid[]) AND recorded_at <= $2
                   ORDER BY {id}, version DESC) v
             ORDER BY recorded_at, {id}",
        columns::<R>(),
        table = R::TABLE,
        id = R::ID
    )))
    .bind(linked)
    .bind(PgTimestamp(at))
    .fetch_all(&mut *db)
    .await
    .map_err(failed::<R, _>)?;
    rows.iter()
        .map(|row| {
            Ok(StoredAt {
                changed_since: row.try_get("changed_since").map_err(failed::<R, _>)?,
                stored: row_to(row)?,
            })
        })
        .collect()
}

/// Every version of one entry, **oldest first**, found under the patient or a record merged into it.
///
/// # Errors
///
/// [`StoreError::NotFound`] when there is no such entry for that patient.
pub async fn history<R: Entry>(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    entry_id: &str,
) -> Result<Vec<Stored<R>>, StoreError> {
    let mut conn = db.into().acquire().await.map_err(failed::<R, _>)?;
    let db = &mut *conn;
    let entry_id = ids::uuid(entry_id).ok_or(StoreError::NotFound)?;
    let linked = ids::uuids(&links::linked_ids(&mut *db, patient_id).await?);
    let rows = sqlx::query(AssertSqlSafe(format!(
        "SELECT {} FROM {} WHERE {} = $1::uuid AND patient_id = ANY($2::uuid[]) ORDER BY version",
        columns::<R>(),
        R::TABLE,
        R::ID
    )))
    .bind(entry_id)
    .bind(linked)
    .fetch_all(&mut *db)
    .await
    .map_err(failed::<R, _>)?;
    if rows.is_empty() {
        return Err(StoreError::NotFound);
    }
    rows.iter().map(row_to).collect()
}

/// A change to one entry.
#[derive(Debug, Clone, Copy)]
pub struct Change<'a, R> {
    /// The version it was made from.
    pub expected_version: i32,
    /// Who makes it.
    pub author: &'a str,
    /// Why. Required.
    pub reason: Option<&'a str>,
    /// The whole new record.
    pub record: &'a R,
}

/// Append version `expected_version + 1`. A mistake is entered in error, which is final. Call it inside a transaction,
/// as [`create`].
///
/// # Errors
///
/// [`StoreError::NotFound`], [`StoreError::Conflict`] (stale or racing version), [`StoreError::BadQuery`] (refused
/// record, no reason, or reopening an entry in error).
pub async fn update<R: Entry>(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    entry_id: &str,
    change: Change<'_, R>,
) -> Result<Stored<R>, StoreError> {
    let mut conn = db.into().acquire().await.map_err(failed::<R, _>)?;
    let db = &mut *conn;
    let mut problems = change.record.faults();
    problems.extend(reason_fault(change.reason, true));
    refuse(problems)?;
    let entry_id = ids::uuid(entry_id).ok_or(StoreError::NotFound)?;
    let patient = patients_store::current(&mut *db, patient_id).await?;
    lock::<R>(&mut *db, &patient.id).await?;
    let all = list::<R>(&mut *db, &patient.id).await?;
    let now = all
        .iter()
        .find(|s| s.id == entry_id)
        .ok_or(StoreError::NotFound)?;
    if now.version != change.expected_version {
        return Err(StoreError::Conflict {
            current: now.version,
        });
    }
    if now.record.in_error() {
        return Err(StoreError::BadQuery(
            "an entry in error is final: record a new one".into(),
        ));
    }
    refuse(change.record.faults_in(Some(&entry_id), &all))?;
    let next = crate::store::next_version(change.expected_version)?;
    // A concurrent writer took this version: the primary key refuses the second (CQ-12).
    let insert = format!(
        "INSERT INTO {} ({}, version, patient_id, author, reason, status, body)
             VALUES ($1::uuid, $2, $3::uuid, $4, $5, $6, $7) RETURNING {}",
        R::TABLE,
        R::ID,
        columns::<R>()
    );
    let query = sqlx::query(AssertSqlSafe(insert))
        .bind(entry_id.clone())
        .bind(next)
        .bind(now.patient_id.clone())
        .bind(change.author)
        .bind(change.reason.map(str::to_string))
        .bind(change.record.status())
        .bind(serde_json::to_value(change.record).map_err(failed::<R, _>)?);
    match crate::store::append_version(&mut *db, R::STORE, query).await? {
        crate::store::Appended::Row(row) => row_to(&row),
        crate::store::Appended::Taken => {
            let current = history::<R>(&mut *db, &patient.id, &entry_id)
                .await?
                .last()
                .map(|s| s.version)
                .ok_or(StoreError::NotFound)?;
            Err(StoreError::Conflict { current })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coding::Coding;

    #[test]
    fn a_reason_is_required_to_change_and_optional_to_record() {
        assert_eq!(reason_fault(None, false), None);
        assert!(reason_fault(None, true).is_some());
        for required in [true, false] {
            assert!(reason_fault(Some("  "), required).is_some());
            assert!(reason_fault(Some(&"x".repeat(REASON_LIMIT + 1)), required).is_some());
            assert_eq!(reason_fault(Some("seen in clinic"), required), None);
        }
    }

    #[test]
    fn a_snomed_code_is_checked_and_not_quoted() {
        let coding = |code: &str| Coding {
            system: "http://snomed.info/sct".into(),
            code: code.into(),
        };
        assert_eq!(coding_fault(None), None);
        assert_eq!(coding_fault(Some(&coding("22298006"))), None);
        let said = coding_fault(Some(&coding("22298007"))).unwrap();
        assert!(!said.contains("22298007"), "{said}");
    }

    #[test]
    fn an_empty_family_history_is_unknown() {
        assert_eq!(FamilyHistoryRecord::summary(&[])["state"], "not-recorded");
        let mut r: FamilyHistoryRecord = serde_json::from_value(serde_json::json!({
            "relationship": "mother", "condition": "c", "presence": "absent",
            "source": "patient", "status": "recorded"
        }))
        .unwrap();
        assert_eq!(
            FamilyHistoryRecord::summary(&[r.clone()])["recorded"],
            1_i32
        );
        assert!(r.set_status("entered-in-error"));
        assert!(!r.set_status("active"));
        assert_eq!(FamilyHistoryRecord::summary(&[r])["state"], "not-recorded");
    }
}
