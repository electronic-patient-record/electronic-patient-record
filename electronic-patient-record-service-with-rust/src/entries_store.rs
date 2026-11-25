//! The append-only store behind the problem list (EH-1), relative history (EH-3) and compatibility typing (DM-1), and the read **as at a time** (EH-2).
//! Both are the same shape (an entry with versions, each a whole record), so they share one store; what differs is an
//! [`Entry`]: its table, its rules, and what its list amounts to.
//!
//! Nothing is ever deleted: ending an entry, changing one's mind, or marking it entered in error is a new version, and
//! entered-in-error is final. A list follows merges: a person's entries do not disappear because two records were
//! joined. **As at** a time, it follows the merges that stood *then*, so the answer is what the record held at that
//! time and not today's record with the later lines left out.
//!
//! **Every read is for a [`Viewer`]** (EH-8): an entry whose security labels they may not see is not there for them, in a
//! list, a history, a rule's refusal or a version number, and what a list amounts to says that it is not all. Labels are
//! not versions: an entry's newest labels apply to it at every version and as at any time.

#![deny(
    clippy::arithmetic_side_effects,
    clippy::default_numeric_fallback,
    clippy::integer_division,
    clippy::modulo_arithmetic,
    clippy::as_conversions,
    clippy::string_slice
)]

use crate::entry_labels::{self, StoredAct};
use crate::ids;
use crate::interop::terminology;
use crate::labelling::{self, Actor, Basis};
use crate::links;
use crate::masking::{self, Completeness, Labels};
use crate::patients_store::{self, StoreError};
use crate::pg_time::PgTimestamp;
use crate::policy::{Decision, Label, Resource};
use crate::pool::Db;
use crate::problem::{self, ProblemRecord};
use crate::relative_history::{self, RelativeHistoryRecord};
use crate::typing::{self, TypingRecord};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sqlx::postgres::PgRow;
use sqlx::{AssertSqlSafe, PgConnection, Row};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, Ordering};

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
    /// The word its labels are kept under.
    const KIND: &'static str;

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
    fn summary(all: &[&Stored<Self>]) -> serde_json::Value;
    /// This record beside withheld entries: it does not point at one.
    #[must_use]
    fn shown(self, _withheld: &BTreeSet<String>) -> Self {
        self
    }
    /// What to store when a writer who was shown `stored` through [`Entry::shown`] sends this.
    #[must_use]
    fn keeping(self, _stored: &Self, _withheld: &BTreeSet<String>) -> Self {
        self
    }
}

/// A coding whose SNOMED CT identifier, if it claims to be one, is not one. The words do not quote it.
fn coding_fault(c: Option<&crate::coding::Coding>) -> Option<String> {
    c.filter(|c| terminology::validate_coding(&c.system, &c.code).is_err())
        .map(|_| "the code is not a SNOMED CT concept identifier".to_string())
}

/// What the cause rules need of a stored problem.
fn problem_link(s: &Stored<ProblemRecord>) -> problem::Link {
    problem::Link {
        id: s.id.clone(),
        standing: !s.record.in_error(),
        refuted: s.record.certainty == problem::Certainty::Refuted,
        due_to: s.record.due_to.clone(),
    }
}

impl Entry for ProblemRecord {
    const STORE: &'static str = "problems";
    const TABLE: &'static str = "problem_versions";
    const VIEW: &'static str = "problem_current";
    const ID: &'static str = "problem_id";
    const RESOURCE: Resource = Resource::Condition;
    const KIND: &'static str = "problem";

    fn shown(self, withheld: &BTreeSet<String>) -> Self {
        masking::show_problem(self, withheld)
    }

    fn keeping(self, stored: &Self, withheld: &BTreeSet<String>) -> Self {
        masking::keep_withheld_cause(self, stored, withheld)
    }

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
        let links: Vec<problem::Link> = all.iter().map(problem_link).collect();
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

    fn summary(all: &[&Stored<Self>]) -> serde_json::Value {
        let links: Vec<problem::Link> = all.iter().map(|s| problem_link(s)).collect();
        let mut out = serde_json::json!(problem::summarise(all.iter().map(|s| &s.record)));
        crate::json::set(
            &mut out,
            "causes",
            serde_json::json!(problem::causes(&links)),
        );
        out
    }
}

impl Entry for RelativeHistoryRecord {
    const STORE: &'static str = "relative history";
    const TABLE: &'static str = "relative_history_versions";
    const VIEW: &'static str = "relative_history_current";
    const ID: &'static str = "entry_id";
    const RESOURCE: Resource = Resource::RelativeHistory;
    const KIND: &'static str = "relative-history";

    fn status(&self) -> &'static str {
        self.status.as_str()
    }

    fn set_status(&mut self, word: &str) -> bool {
        let parsed = relative_history::Status::parse(word);
        if let Some(s) = parsed {
            self.status = s;
        }
        parsed.is_some()
    }

    fn in_error(&self) -> bool {
        self.status == relative_history::Status::EnteredInError
    }

    fn faults(&self) -> Vec<String> {
        let mut out = relative_history::validate(self);
        out.extend(coding_fault(self.coding.as_ref()));
        out
    }

    fn summary(all: &[&Stored<Self>]) -> serde_json::Value {
        serde_json::json!(relative_history::summarise(all.iter().map(|s| &s.record)))
    }
}

impl Entry for TypingRecord {
    const STORE: &'static str = "compatibility typing";
    const TABLE: &'static str = "typing_versions";
    const VIEW: &'static str = "typing_current";
    const ID: &'static str = "typing_id";
    const RESOURCE: Resource = Resource::CompatibilityTyping;
    const KIND: &'static str = "compatibility-typing";

    fn status(&self) -> &'static str {
        self.status.as_str()
    }

    fn set_status(&mut self, word: &str) -> bool {
        let parsed = typing::Status::parse(word);
        if let Some(s) = parsed {
            self.status = s;
        }
        parsed.is_some()
    }

    fn in_error(&self) -> bool {
        self.status == typing::Status::EnteredInError
    }

    fn faults(&self) -> Vec<String> {
        typing::validate(self, jiff::Timestamp::now())
    }

    fn summary(all: &[&Stored<Self>]) -> serde_json::Value {
        serde_json::json!(typing::summarise(all.iter().map(|s| &s.record)))
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
    /// The entry's security labels now, whichever version this is.
    pub labels: Vec<String>,
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

/// Who a read is for.
pub struct Viewer<'a> {
    decide: Box<dyn Fn(BTreeSet<Label>) -> Decision + Send + Sync + 'a>,
    told: bool,
    emergency: AtomicBool,
}

impl std::fmt::Debug for Viewer<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Viewer")
            .field("told", &self.told)
            .finish_non_exhaustive()
    }
}

impl<'a> Viewer<'a> {
    /// A viewer for whom `decide` is the record decision for an entry with these labels, and who is `told` how many
    /// entries are withheld or not ([`masking::told`]).
    pub fn new(
        decide: impl Fn(BTreeSet<Label>) -> Decision + Send + Sync + 'a,
        told: bool,
    ) -> Viewer<'a> {
        Viewer {
            decide: Box::new(decide),
            told,
            emergency: AtomicBool::new(false),
        }
    }

    /// One from whom nothing with a readable label is withheld. For tests: no route uses it.
    #[must_use]
    pub fn everything() -> Viewer<'static> {
        Viewer::new(|_| Decision::Allow(Vec::new()), true)
    }

    /// Whether anything was shown to them only because of break-the-glass.
    #[must_use]
    pub fn by_emergency(&self) -> bool {
        self.emergency.load(Ordering::SeqCst)
    }

    fn mask<T>(
        &self,
        entries: Vec<T>,
        id: impl Fn(&T) -> &str,
        labels: impl Fn(&T) -> &[String],
    ) -> masking::Masked<T> {
        let masked = masking::mask(
            entries,
            id,
            |e| Labels::read(labels(e).iter().map(String::as_str)),
            |set| (self.decide)(set),
            self.told,
        );
        self.emergency
            .fetch_or(masked.by_emergency, Ordering::SeqCst);
        masked
    }
}

/// What a list amounts to for someone who may not have been given all of it. It claims nothing that an entry they were not
/// given could change: not that nothing is recorded or active, and not that reports agree.
fn summary_of<R: Entry>(entries: &[&Stored<R>], completeness: Completeness) -> serde_json::Value {
    let mut out = R::summary(entries);
    if completeness == Completeness::Whole {
        return out;
    }
    crate::json::set(&mut out, "partial", serde_json::json!(true));
    for (key, kept) in [
        ("state", &["active", "recorded"][..]),
        ("abo", &["disagree"]),
        ("rhd", &["disagree"]),
    ] {
        let claims = out
            .get(key)
            .and_then(serde_json::Value::as_str)
            .is_some_and(|v| !kept.contains(&v));
        if claims {
            crate::json::set(&mut out, key, serde_json::json!("not-stated"));
        }
    }
    out
}

/// A list as one viewer may have it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct View<R> {
    /// What they may have, oldest first.
    pub entries: Vec<Stored<R>>,
    /// Whether that is all.
    pub completeness: Completeness,
    withheld: BTreeSet<String>,
}

impl<R: Entry> View<R> {
    fn of(raw: Vec<Stored<R>>, viewer: &Viewer<'_>) -> View<R> {
        let masked = viewer.mask(raw, |s| s.id.as_str(), |s| &s.labels);
        View {
            entries: masked
                .shown
                .into_iter()
                .map(|s| Stored {
                    record: s.record.shown(&masked.withheld_ids),
                    ..s
                })
                .collect(),
            completeness: masked.completeness,
            withheld: masked.withheld_ids,
        }
    }

    /// What it amounts to.
    #[must_use]
    pub fn summary(&self) -> serde_json::Value {
        summary_of(&self.entries.iter().collect::<Vec<_>>(), self.completeness)
    }
}

/// A list as at a past time, as one viewer may have it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewAt<R> {
    /// What they may have, oldest first.
    pub entries: Vec<StoredAt<R>>,
    /// Whether that is all.
    pub completeness: Completeness,
}

impl<R: Entry> ViewAt<R> {
    /// What it amounted to.
    #[must_use]
    pub fn summary(&self) -> serde_json::Value {
        let entries: Vec<&Stored<R>> = self.entries.iter().map(|s| &s.stored).collect();
        summary_of(&entries, self.completeness)
    }
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
        labels: Vec::new(),
        record,
    })
}

/// What happened to the labels of the entries on these records, by entry id.
async fn acts<R: Entry>(
    db: &mut PgConnection,
    linked: &[String],
) -> Result<BTreeMap<String, Vec<StoredAct>>, StoreError> {
    entry_labels::of_patients(db, R::KIND, linked).await
}

/// The labels of one entry now.
fn labels_of(acts: &BTreeMap<String, Vec<StoredAct>>, id: &str) -> Vec<String> {
    let Some(all) = acts.get(id) else {
        return Vec::new();
    };
    let all: Vec<labelling::Act> = all.iter().map(|a| a.act.clone()).collect();
    labelling::current(&all).into_iter().collect()
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
    viewer: &Viewer<'_>,
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
    let view = View::of(every::<R>(&mut *db, &patient.id).await?, viewer);
    refuse(record.faults_in(None, &view.entries))?;
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

/// The current version of every entry of a patient that `viewer` may have, every status, oldest first, with those of any
/// record merged into this one.
///
/// # Errors
///
/// [`StoreError::NotFound`] (no such patient).
pub async fn list<R: Entry>(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    viewer: &Viewer<'_>,
) -> Result<View<R>, StoreError> {
    let mut conn = db.into().acquire().await.map_err(failed::<R, _>)?;
    Ok(View::of(every::<R>(&mut conn, patient_id).await?, viewer))
}

/// [`list`], with nothing withheld. Not for an answer.
async fn every<R: Entry>(
    db: &mut PgConnection,
    patient_id: &str,
) -> Result<Vec<Stored<R>>, StoreError> {
    let patient = patients_store::current(&mut *db, patient_id).await?;
    let linked = links::linked_ids(&mut *db, &patient.id).await?;
    let labels = acts::<R>(&mut *db, &linked).await?;
    let linked = ids::uuids(&linked);
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
    rows.iter()
        .map(|row| {
            let stored: Stored<R> = row_to(row)?;
            Ok(Stored {
                labels: labels_of(&labels, &stored.id),
                ..stored
            })
        })
        .collect()
}

/// Every entry that `viewer` may have today, as it stood at `at`: for each, the latest version written by then, with the entries of the records that
/// were merged into this one **then**. An entry first written later is not there at all.
///
/// # Errors
///
/// [`StoreError::NotFound`] (no such patient now; whether it was registered at `at` is not asked).
pub async fn list_at<R: Entry>(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    at: jiff::Timestamp,
    viewer: &Viewer<'_>,
) -> Result<ViewAt<R>, StoreError> {
    let mut conn = db.into().acquire().await.map_err(failed::<R, _>)?;
    let db = &mut *conn;
    let patient = patients_store::current(&mut *db, patient_id).await?;
    let linked = links::linked_ids_at(&mut *db, &patient.id, at).await?;
    let labels = acts::<R>(&mut *db, &linked).await?;
    let linked = ids::uuids(&linked);
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
    let all = rows
        .iter()
        .map(|row| {
            let stored: Stored<R> = row_to(row)?;
            Ok(StoredAt {
                changed_since: row.try_get("changed_since").map_err(failed::<R, _>)?,
                stored: Stored {
                    labels: labels_of(&labels, &stored.id),
                    ..stored
                },
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    let masked = viewer.mask(all, |s| s.stored.id.as_str(), |s| &s.stored.labels);
    Ok(ViewAt {
        entries: masked
            .shown
            .into_iter()
            .map(|s| StoredAt {
                changed_since: s.changed_since,
                stored: Stored {
                    record: s.stored.record.shown(&masked.withheld_ids),
                    ..s.stored
                },
            })
            .collect(),
        completeness: masked.completeness,
    })
}

/// Every version of one entry that `viewer` may have, **oldest first**, found under the patient or a record merged into it.
///
/// # Errors
///
/// [`StoreError::NotFound`] when there is no such entry for that patient, or none that they may have.
pub async fn history<R: Entry>(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    entry_id: &str,
    viewer: &Viewer<'_>,
) -> Result<Vec<Stored<R>>, StoreError> {
    let mut conn = db.into().acquire().await.map_err(failed::<R, _>)?;
    let db = &mut *conn;
    let view = View::of(every::<R>(&mut *db, patient_id).await?, viewer);
    let entry_id = ids::uuid(entry_id).ok_or(StoreError::NotFound)?;
    let shown = view
        .entries
        .iter()
        .find(|s| s.id == entry_id)
        .ok_or(StoreError::NotFound)?;
    let versions = every_version::<R>(&mut *db, patient_id, &entry_id).await?;
    Ok(versions
        .into_iter()
        .map(|s| Stored {
            labels: shown.labels.clone(),
            record: s.record.shown(&view.withheld),
            ..s
        })
        .collect())
}

/// [`history`], with nothing withheld. Not for an answer.
///
/// # Errors
///
/// [`StoreError::NotFound`] when there is no such entry for that patient.
async fn every_version<R: Entry>(
    db: &mut PgConnection,
    patient_id: &str,
    entry_id: &str,
) -> Result<Vec<Stored<R>>, StoreError> {
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
    viewer: &Viewer<'_>,
) -> Result<Stored<R>, StoreError> {
    let mut conn = db.into().acquire().await.map_err(failed::<R, _>)?;
    let db = &mut *conn;
    let mut problems = change.record.faults();
    problems.extend(reason_fault(change.reason, true));
    refuse(problems)?;
    let entry_id = ids::uuid(entry_id).ok_or(StoreError::NotFound)?;
    let patient = patients_store::current(&mut *db, patient_id).await?;
    lock::<R>(&mut *db, &patient.id).await?;
    let all = every::<R>(&mut *db, &patient.id).await?;
    let view = View::of(all.clone(), viewer);
    let now = view
        .entries
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
    // The rules see what the writer sees, so that a refusal does not tell of an entry withheld from them.
    refuse(change.record.faults_in(Some(&entry_id), &view.entries))?;
    let stored = all
        .iter()
        .find(|s| s.id == entry_id)
        .ok_or(StoreError::NotFound)?;
    let record = change
        .record
        .clone()
        .keeping(&stored.record, &view.withheld);
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
        .bind(record.status())
        .bind(serde_json::to_value(&record).map_err(failed::<R, _>)?);
    match crate::store::append_version(&mut *db, R::STORE, query).await? {
        crate::store::Appended::Row(row) => {
            let written: Stored<R> = row_to(&row)?;
            Ok(Stored {
                labels: now.labels.clone(),
                record: written.record.shown(&view.withheld),
                ..written
            })
        }
        crate::store::Appended::Taken => {
            let current = every_version::<R>(&mut *db, &patient.id, &entry_id)
                .await?
                .last()
                .map(|s| s.version)
                .ok_or(StoreError::NotFound)?;
            Err(StoreError::Conflict { current })
        }
    }
}

/// A change to one entry's labels.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(
    tag = "act",
    rename_all = "kebab-case",
    rename_all_fields = "snake_case",
    deny_unknown_fields
)]
pub enum LabelChange {
    /// Add one.
    Add {
        /// Which, by its code.
        label: String,
        /// On whose decision.
        basis: Basis,
    },
    /// Lift one.
    Lift {
        /// Which, by its code.
        label: String,
        /// On whose decision.
        basis: Basis,
        /// Why.
        reason: String,
    },
}

/// Why a label was not added or lifted.
#[derive(Debug)]
pub enum LabelError {
    /// The store: no such entry for this viewer, or a database error.
    Store(StoreError),
    /// Not a label this version knows.
    Unknown,
    /// The rule.
    Refused(labelling::Refusal),
}

impl From<StoreError> for LabelError {
    fn from(e: StoreError) -> LabelError {
        LabelError::Store(e)
    }
}

/// Add or lift a label of an entry that `viewer` may have. Call it inside a transaction, as [`create`]: it holds the lock
/// on the patient's list. Returns the entry's id and the patient record it is on.
///
/// # Errors
///
/// [`LabelError`].
pub async fn relabel<R: Entry>(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    entry_id: &str,
    actor: &Actor,
    change: &LabelChange,
    viewer: &Viewer<'_>,
) -> Result<(String, String), LabelError> {
    let mut conn = db.into().acquire().await.map_err(failed::<R, _>)?;
    let db = &mut *conn;
    let entry_id = ids::uuid(entry_id).ok_or(StoreError::NotFound)?;
    let patient = patients_store::current(&mut *db, patient_id).await?;
    lock::<R>(&mut *db, &patient.id).await?;
    let view = View::of(every::<R>(&mut *db, &patient.id).await?, viewer);
    let entry = view
        .entries
        .iter()
        .find(|s| s.id == entry_id)
        .ok_or(StoreError::NotFound)?;
    let linked = links::linked_ids(&mut *db, &patient.id).await?;
    let history: Vec<labelling::Act> = acts::<R>(&mut *db, &linked)
        .await?
        .remove(&entry_id)
        .into_iter()
        .flatten()
        .map(|a| a.act)
        .collect();
    let known = |code: &str| Label::parse(code).ok_or(LabelError::Unknown);
    let act = match change {
        LabelChange::Add { label, basis } => labelling::add(&history, actor, known(label)?, *basis),
        LabelChange::Lift {
            label,
            basis,
            reason,
        } => labelling::lift(&history, actor, known(label)?, *basis, reason),
    }
    .map_err(LabelError::Refused)?;
    entry_labels::append(&mut *db, R::KIND, &entry_id, &entry.patient_id, &act).await?;
    Ok((entry_id, entry.patient_id.clone()))
}

/// What happened to the labels of one entry that `viewer` may have, oldest first.
///
/// # Errors
///
/// [`StoreError::NotFound`] as [`history`].
pub async fn label_history<R: Entry>(
    db: impl Into<Db<'_>>,
    patient_id: &str,
    entry_id: &str,
    viewer: &Viewer<'_>,
) -> Result<Vec<StoredAct>, StoreError> {
    let mut conn = db.into().acquire().await.map_err(failed::<R, _>)?;
    let db = &mut *conn;
    let entry_id = ids::uuid(entry_id).ok_or(StoreError::NotFound)?;
    let view = View::of(every::<R>(&mut *db, patient_id).await?, viewer);
    if !view.entries.iter().any(|s| s.id == entry_id) {
        return Err(StoreError::NotFound);
    }
    let linked = links::linked_ids(&mut *db, patient_id).await?;
    Ok(acts::<R>(&mut *db, &linked)
        .await?
        .remove(&entry_id)
        .into_iter()
        .flatten()
        .collect())
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
    fn the_stored_status_word_decides() {
        let mut r: RelativeHistoryRecord = serde_json::from_value(serde_json::json!({
            "genetic": "full-sibling", "environment": "childhood", "condition": "c",
            "presence": "absent", "source": "patient", "status": "recorded"
        }))
        .unwrap();
        let stored = |record: &RelativeHistoryRecord| Stored {
            id: "e".into(),
            patient_id: "p".into(),
            version: 1,
            author: "a".into(),
            recorded_at: "t".into(),
            reason: None,
            labels: Vec::new(),
            record: record.clone(),
        };
        assert_eq!(
            RelativeHistoryRecord::summary(&[&stored(&r)])["inherited"],
            1_i32
        );
        assert!(r.set_status("entered-in-error"));
        assert!(!r.set_status("active"));
        assert_eq!(
            RelativeHistoryRecord::summary(&[&stored(&r)])["state"],
            "not-recorded"
        );
    }
}
