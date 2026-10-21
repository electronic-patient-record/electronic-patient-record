//! Storage for the patient identity record: append-only versions in `patient_versions`.
//!
//! No function here updates or deletes a row. A change is a new version; two writers racing to
//! change the same version collide on the primary key and the loser gets [`StoreError::Conflict`].

use electronic_patient_record_matcher::{NHS_NUMBER_SYSTEM, Patient, is_valid_nhs_number};
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DbBackend, Statement, TransactionSession,
    TransactionTrait, Value,
};

/// A stored patient at one version.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Stored {
    /// The server-assigned id (a UUID).
    pub id: String,
    /// 1 for the first version.
    pub version: i32,
    /// Who wrote this version.
    pub author: String,
    /// The identity record, with `id` set to the server id.
    pub patient: Patient,
}

/// Why a store operation failed.
#[derive(Debug, PartialEq, Eq)]
pub enum StoreError {
    /// No such patient.
    NotFound,
    /// The expected version is not the current one.
    Conflict {
        /// The current version.
        current: i32,
    },
    /// The query is malformed.
    BadQuery(String),
    /// The database failed.
    Db(String),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::NotFound => write!(f, "patient not found"),
            StoreError::Conflict { current } => write!(f, "current version is {current}"),
            StoreError::BadQuery(e) => write!(f, "bad query: {e}"),
            StoreError::Db(e) => write!(f, "database error: {e}"),
        }
    }
}

fn db_err(e: impl std::fmt::Display) -> StoreError {
    StoreError::Db(e.to_string())
}

fn pg(sql: &str, values: Vec<Value>) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, sql, values)
}

/// Check a patient before it is stored. Returns every problem found.
///
/// A record needs a family name, and its birth date, if given, must be `YYYY`, `YYYY-MM` or a real
/// `YYYY-MM-DD`. An NHS number with a bad check digit is refused; an identifier in any other system
/// is kept as given (unverified).
#[must_use]
pub fn validate(patient: &Patient) -> Vec<String> {
    let mut problems = Vec::new();
    if !patient
        .names
        .iter()
        .any(|n| n.family.as_deref().is_some_and(|f| !f.trim().is_empty()))
    {
        problems.push("at least one name with a family name is required".to_string());
    }
    if let Some(d) = &patient.birth_date
        && !valid_birth_date(d)
    {
        problems.push(format!(
            "birth_date {d:?} is not YYYY, YYYY-MM or YYYY-MM-DD"
        ));
    }
    for id in &patient.identifiers {
        if id.system == NHS_NUMBER_SYSTEM && !is_valid_nhs_number(&id.value) {
            problems.push("NHS number fails the modulus 11 check".to_string());
        }
        if id.system.trim().is_empty() || id.value.trim().is_empty() {
            problems.push("an identifier needs a system and a value".to_string());
        }
    }
    problems
}

pub(crate) fn valid_birth_date(d: &str) -> bool {
    let parts: Vec<&str> = d.split('-').collect();
    let num = |s: &str, len: usize| {
        (s.len() == len && s.bytes().all(|b| b.is_ascii_digit()))
            .then(|| s.parse::<u32>().ok())
            .flatten()
    };
    match parts.as_slice() {
        [y] => num(y, 4).is_some_and(|y| y >= 1800),
        [y, m] => {
            num(y, 4).is_some_and(|y| y >= 1800) && num(m, 2).is_some_and(|m| (1..=12).contains(&m))
        }
        [y, m, day] => {
            let (Some(y), Some(m), Some(day)) = (num(y, 4), num(m, 2), num(day, 2)) else {
                return false;
            };
            let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
            let max = match m {
                1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
                4 | 6 | 9 | 11 => 30,
                2 => 28 + u32::from(leap),
                _ => return false,
            };
            y >= 1800 && (1..=max).contains(&day)
        }
        _ => false,
    }
}

fn row_to_stored(row: &sea_orm::QueryResult) -> Result<Stored, StoreError> {
    let id: String = row.try_get("", "id").map_err(db_err)?;
    let body: serde_json::Value = row.try_get("", "body").map_err(db_err)?;
    let mut patient: Patient = serde_json::from_value(body).map_err(db_err)?;
    patient.id = Some(id.clone());
    Ok(Stored {
        id,
        version: row.try_get("", "version").map_err(db_err)?,
        author: row.try_get("", "author").map_err(db_err)?,
        patient,
    })
}

/// A record merged into another is not a live patient: it is left out of search and of the
/// duplicate check (the surviving record is found instead). Its own rows and reads are unchanged.
const NOT_MERGED_AWAY: &str = "NOT EXISTS (
    SELECT 1 FROM patient_link_current l
    WHERE l.source_patient_id = patient_current.patient_id AND l.status = 'active')";

const COLUMNS: &str = "patient_id::text AS id, version, author, body";

/// Create a patient at version 1. The server assigns the id; any `id` in `patient` is ignored.
///
/// # Errors
///
/// [`StoreError::Db`] when the insert fails.
pub async fn create(
    db: &impl ConnectionTrait,
    author: &str,
    reason: Option<&str>,
    patient: &Patient,
) -> Result<Stored, StoreError> {
    let mut body = patient.clone();
    body.id = None;
    let row = db
        .query_one_raw(pg(
            &format!(
                "INSERT INTO patient_versions (patient_id, version, author, reason, body)
                 VALUES (gen_random_uuid(), 1, $1, $2, $3) RETURNING {COLUMNS}"
            ),
            vec![
                author.into(),
                reason.map(str::to_string).into(),
                serde_json::to_value(&body).map_err(db_err)?.into(),
            ],
        ))
        .await
        .map_err(db_err)?
        .ok_or_else(|| StoreError::Db("insert returned no row".into()))?;
    row_to_stored(&row)
}

/// Postgres advisory-lock key serialising patient registration ("EPRR" in ASCII).
const REGISTRATION_LOCK: i64 = 0x4550_5252;

/// Serialise registration: take this inside the creating transaction, then look for duplicates
/// with [`candidates`] on the same transaction, then [`create`]. Two registrations of one person
/// can then never both find no candidate. The lock lasts until the transaction ends; take it before
/// any audit append so the lock order is always registration, then audit.
///
/// # Errors
///
/// [`StoreError::Db`] when the lock cannot be taken.
pub async fn lock_registration(db: &impl ConnectionTrait) -> Result<(), StoreError> {
    db.execute_raw(pg(
        "SELECT pg_advisory_xact_lock($1)",
        vec![REGISTRATION_LOCK.into()],
    ))
    .await
    .map(|_| ())
    .map_err(db_err)
}

/// The current version of a patient.
///
/// # Errors
///
/// [`StoreError::NotFound`] for an unknown or malformed id.
pub async fn current(db: &impl ConnectionTrait, id: &str) -> Result<Stored, StoreError> {
    let row = db
        .query_one_raw(pg(
            &format!("SELECT {COLUMNS} FROM patient_current WHERE patient_id::text = lower($1)"),
            vec![id.into()],
        ))
        .await
        .map_err(db_err)?
        .ok_or(StoreError::NotFound)?;
    row_to_stored(&row)
}

/// Append version `expected_version + 1`. The earlier versions stay.
///
/// # Errors
///
/// [`StoreError::NotFound`] for an unknown id; [`StoreError::Conflict`] when `expected_version` is
/// not the current version, including when a concurrent writer got there first.
pub async fn update(
    db: &(impl ConnectionTrait + TransactionTrait),
    id: &str,
    expected_version: i32,
    author: &str,
    reason: Option<&str>,
    patient: &Patient,
) -> Result<Stored, StoreError> {
    let now = current(db, id).await?;
    if now.version != expected_version {
        return Err(StoreError::Conflict {
            current: now.version,
        });
    }
    let mut body = patient.clone();
    body.id = None;
    let sp = db.begin().await.map_err(db_err)?;
    let inserted = sp
        .query_one_raw(pg(
            &format!(
                "INSERT INTO patient_versions (patient_id, version, author, reason, body)
                 VALUES ($1::uuid, $2, $3, $4, $5) RETURNING {COLUMNS}"
            ),
            vec![
                now.id.clone().into(),
                (expected_version + 1).into(),
                author.into(),
                reason.map(str::to_string).into(),
                serde_json::to_value(&body).map_err(db_err)?.into(),
            ],
        ))
        .await;
    let inserted = match inserted {
        Ok(v) => {
            sp.commit().await.map_err(db_err)?;
            Ok(v)
        }
        Err(e) => {
            // A failed statement aborts a PostgreSQL transaction; only the savepoint is lost, so the
            // caller's transaction (if any) can still read the current version below.
            let _ = sp.rollback().await;
            Err(e)
        }
    };
    match inserted {
        Ok(Some(row)) => row_to_stored(&row),
        Ok(None) => Err(StoreError::Db("insert returned no row".into())),
        // A concurrent writer took this version: the primary key refuses the second.
        Err(e) if e.to_string().contains("patient_versions_pkey") => {
            let current = current(db, id).await?.version;
            Err(StoreError::Conflict { current })
        }
        Err(e) => Err(db_err(e)),
    }
}

/// The most candidates fetched for a duplicate check.
pub const CANDIDATE_LIMIT: i64 = 200;

/// Current patients that share an identifier, a birth date or a family name (case-insensitive,
/// exact) with `patient`: the pool the matcher then scores. A pair sharing none of these is not
/// compared, so a duplicate with a misspelt name, a different birth date and no common identifier
/// is **not** found.
///
/// # Errors
///
/// [`StoreError::Db`] on a database failure.
pub async fn candidates(
    db: &impl ConnectionTrait,
    patient: &Patient,
) -> Result<Vec<Stored>, StoreError> {
    let identifiers: Vec<serde_json::Value> = patient
        .identifiers
        .iter()
        .map(|i| serde_json::json!({ "system": i.system, "value": i.value }))
        .collect();
    let families: Vec<String> = patient
        .names
        .iter()
        .filter_map(|n| n.family.as_deref())
        .map(|f| f.trim().to_lowercase())
        .filter(|f| !f.is_empty())
        .collect();
    if identifiers.is_empty() && families.is_empty() && patient.birth_date.is_none() {
        return Ok(Vec::new());
    }
    let rows = db
        .query_all_raw(pg(
            &format!(
                "SELECT {COLUMNS} FROM patient_current
                 WHERE (EXISTS (SELECT 1 FROM jsonb_array_elements($1::jsonb) i
                                WHERE body->'identifiers' @> jsonb_build_array(i))
                    OR ($2::text IS NOT NULL AND body->>'birth_date' = $2)
                    OR EXISTS (SELECT 1 FROM jsonb_array_elements(body->'names') n
                               WHERE lower(trim(n->>'family')) = ANY($3::text[])))
                   AND {NOT_MERGED_AWAY}
                 ORDER BY patient_id
                 LIMIT {CANDIDATE_LIMIT}"
            ),
            vec![
                serde_json::Value::Array(identifiers).into(),
                patient.birth_date.clone().into(),
                families.into(),
            ],
        ))
        .await
        .map_err(db_err)?;
    rows.iter().map(row_to_stored).collect()
}

/// Search criteria. At least one must be set: there is no "list everyone".
#[derive(Debug, Default, Clone, serde::Deserialize)]
pub struct Search {
    /// Family name prefix, case-insensitive.
    pub family: Option<String>,
    /// Exact birth date as stored.
    pub birth_date: Option<String>,
    /// `system|value`.
    pub identifier: Option<String>,
}

/// The most results a search returns.
pub const SEARCH_LIMIT: i64 = 50;

impl Search {
    /// Whether any criterion is set.
    #[must_use]
    pub fn any(&self) -> bool {
        [&self.family, &self.birth_date, &self.identifier]
            .iter()
            .any(|c| c.as_deref().is_some_and(|s| !s.trim().is_empty()))
    }
}

fn like_prefix(s: &str) -> String {
    let mut out = String::new();
    for c in s.trim().chars() {
        if matches!(c, '%' | '_' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('%');
    out
}

/// Search current versions, at most [`SEARCH_LIMIT`], ordered by family name then id.
///
/// # Errors
///
/// [`StoreError::Db`] on a database failure, or when `identifier` is not `system|value`.
pub async fn search(db: &DatabaseConnection, q: &Search) -> Result<Vec<Stored>, StoreError> {
    let family = q.family.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let birth = q
        .birth_date
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let identifier = match q
        .identifier
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        None => None,
        Some(s) => {
            let (system, value) = s
                .split_once('|')
                .filter(|(a, b)| !a.is_empty() && !b.is_empty())
                .ok_or_else(|| StoreError::BadQuery("identifier must be system|value".into()))?;
            Some(serde_json::json!([{ "system": system, "value": value }]))
        }
    };
    let rows = db
        .query_all_raw(pg(
            &format!(
                "SELECT {COLUMNS} FROM patient_current
                 WHERE ($1::text IS NULL OR EXISTS (
                         SELECT 1 FROM jsonb_array_elements(body->'names') n
                         WHERE lower(n->>'family') LIKE lower($1) ESCAPE '\\'))
                   AND ($2::text IS NULL OR body->>'birth_date' = $2)
                   AND ($3::jsonb IS NULL OR body->'identifiers' @> $3::jsonb)
                   AND {NOT_MERGED_AWAY}
                 ORDER BY lower(body->'names'->0->>'family'), patient_id
                 LIMIT {SEARCH_LIMIT}"
            ),
            vec![
                family.map(like_prefix).into(),
                birth.map(str::to_string).into(),
                identifier.into(),
            ],
        ))
        .await
        .map_err(db_err)?;
    rows.iter().map(row_to_stored).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use electronic_patient_record_matcher::{HumanName, Identifier};

    fn patient(family: &str, dob: Option<&str>) -> Patient {
        Patient {
            names: vec![HumanName {
                family: Some(family.into()),
                given: vec![],
            }],
            birth_date: dob.map(Into::into),
            ..Default::default()
        }
    }

    #[test]
    fn a_name_is_required_and_dates_must_be_real() {
        assert!(validate(&patient("Smith", Some("1980-03-14"))).is_empty());
        assert!(validate(&patient("Smith", Some("1980"))).is_empty());
        assert!(validate(&patient("Smith", Some("1980-03"))).is_empty());
        assert!(validate(&patient("Smith", Some("2024-02-29"))).is_empty());
        for bad in [
            "1980-13-01",
            "1980-02-30",
            "2023-02-29",
            "80-01-01",
            "1980/01/01",
            "abcd",
            "1700-01-01",
            "",
        ] {
            assert!(!validate(&patient("Smith", Some(bad))).is_empty(), "{bad}");
        }
        assert!(!validate(&patient("  ", None)).is_empty());
        assert!(!validate(&Patient::default()).is_empty());
    }

    #[test]
    fn nhs_numbers_are_checked_and_other_systems_are_kept() {
        let mut p = patient("Smith", None);
        p.identifiers = vec![Identifier {
            system: NHS_NUMBER_SYSTEM.into(),
            value: "9434765919".into(),
        }];
        assert!(validate(&p).is_empty());
        p.identifiers[0].value = "9434765910".into();
        assert!(validate(&p).iter().any(|e| e.contains("modulus 11")));
        p.identifiers = vec![Identifier {
            system: "urn:example:mrn".into(),
            value: "anything".into(),
        }];
        assert!(validate(&p).is_empty());
        p.identifiers[0].value = " ".into();
        assert!(!validate(&p).is_empty());
    }

    #[test]
    fn like_wildcards_in_a_search_are_escaped() {
        assert_eq!(like_prefix("O'Neil"), "O'Neil%");
        assert_eq!(like_prefix("50%_\\"), "50\\%\\_\\\\%");
    }

    #[test]
    fn an_empty_search_is_not_a_search() {
        assert!(!Search::default().any());
        assert!(
            !Search {
                family: Some("  ".into()),
                ..Default::default()
            }
            .any()
        );
        assert!(
            Search {
                birth_date: Some("1980".into()),
                ..Default::default()
            }
            .any()
        );
    }
}
