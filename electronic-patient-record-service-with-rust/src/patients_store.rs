//! Storage for the patient identity record: append-only versions in `patient_versions`.
//!
//! No function here updates or deletes a row. A change is a new version; two writers racing to
//! change the same version collide on the primary key and the loser gets [`StoreError::Conflict`].

use crate::ids;
use crate::pool::Db;
use electronic_patient_record_matcher::{
    NATIONAL_NUMBER_SYSTEM, Patient, is_placeholder_national_number, is_valid_national_number,
};
use sqlx::postgres::PgRow;
use sqlx::{AssertSqlSafe, PgPool, Row};

crate::store_errors!("patients");

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
    /// A drill or test record (EC-10): kept out of search and of the duplicate check for real
    /// patients, and never merged with a real record. Every version of a patient has the same value.
    #[serde(default)]
    pub drill: bool,
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
    /// The matcher says two records are not the same person (a merge is refused): the words, and the
    /// reasons as the matcher's stable codes (a JSON list) for a client that translates them.
    NotSamePerson {
        /// The English text, as `BadQuery` would carry it.
        message: String,
        /// `MatchResult.conflict_codes` as JSON.
        conflict_codes: serde_json::Value,
    },
    /// A version would drop `temporary_identity` with no reason given (TT-1). Every writer sends a
    /// whole record, so a writer that does not know the flag would otherwise clear it unnoticed.
    TemporaryIdentityCleared,
    /// The database failed.
    Db(String),
    /// The database cannot serve for now (no free connection, a lost connection, a server shutting
    /// down, read-only or full): the service's trouble, and it passes, so it is a 503 with `Retry-After`.
    Unavailable(String),
}

impl StoreError {
    /// The kind of a refusal, for its audit row (EH-16). The words are for the caller: they can quote what was sent, such
    /// as a code or an identifier, and the trail must not.
    #[must_use]
    pub fn refusal_kind(&self) -> &'static str {
        match self {
            StoreError::NotFound => "not_found",
            StoreError::Conflict { .. } => "version_conflict",
            StoreError::BadQuery(_) => "record_refused",
            StoreError::NotSamePerson { .. } => "not_same_person",
            StoreError::TemporaryIdentityCleared => "temporary_identity_cleared",
            StoreError::Db(_) => "database_error",
            StoreError::Unavailable(_) => "database_unavailable",
        }
    }
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::NotFound => write!(f, "patient not found"),
            StoreError::Conflict { current } => write!(f, "current version is {current}"),
            StoreError::BadQuery(e) => write!(f, "bad query: {e}"),
            StoreError::NotSamePerson { message, .. } => write!(f, "bad query: {message}"),
            StoreError::TemporaryIdentityCleared => write!(f, "{TEMPORARY_IDENTITY_CLEARED}"),
            StoreError::Db(e) => write!(f, "database error: {e}"),
            StoreError::Unavailable(e) => write!(f, "database unavailable: {e}"),
        }
    }
}

/// The English for [`StoreError::TemporaryIdentityCleared`].
pub const TEMPORARY_IDENTITY_CLEARED: &str =
    "this record is a temporary identity: giving a real identity needs a reason";

/// Check a patient before it is stored. Returns every problem found.
///
/// A record needs a family name, and its birth date, if given, must be `YYYY`, `YYYY-MM` or a real
/// `YYYY-MM-DD`. An national health number with a bad check digit is refused; an identifier in any other system
/// is kept as given (unverified).
#[must_use]
pub fn validate(patient: &Patient) -> Vec<String> {
    validate_coded(patient)
        .into_iter()
        .map(|(_, message)| message)
        .collect()
}

/// [`validate`] with a stable code beside each message, in the same order, so a client can say it in
/// the reader's language: `family_name_required`, `birth_date_invalid`, `national_number_placeholder`,
/// `national_number_check_digit`, `identifier_incomplete`, `death_date_not_accepted`.
#[must_use]
pub fn validate_coded(patient: &Patient) -> Vec<(&'static str, String)> {
    let mut problems = Vec::new();
    if !patient
        .names
        .iter()
        .any(|n| n.family.as_deref().is_some_and(|f| !f.trim().is_empty()))
    {
        problems.push((
            "family_name_required",
            "at least one name with a family name is required".to_string(),
        ));
    }
    if let Some(d) = &patient.birth_date
        && !valid_birth_date(d)
    {
        problems.push((
            "birth_date_invalid",
            format!("birth_date {d:?} is not YYYY, YYYY-MM or YYYY-MM-DD"),
        ));
    }
    // The field is the matcher's, and is filled only from a standing assertion (OQ-BV3). A death has an
    // author and a reason, so it is reported on its own route and never arrives with an identity.
    if patient.death_date.is_some() {
        problems.push((
            "death_date_not_accepted",
            "a date of death is not part of an identity: report it at /api/patients/{id}/death/assertions"
                .to_string(),
        ));
    }
    for id in &patient.identifiers {
        if id.system == NATIONAL_NUMBER_SYSTEM && is_placeholder_national_number(&id.value) {
            problems.push((
                "national_number_placeholder",
                "National health number is a placeholder (ten copies of one digit), not an issued number"
                    .to_string(),
            ));
        } else if id.system == NATIONAL_NUMBER_SYSTEM && !is_valid_national_number(&id.value) {
            problems.push((
                "national_number_check_digit",
                "National health number fails the modulus 11 check".to_string(),
            ));
        }
        if id.system.trim().is_empty() || id.value.trim().is_empty() {
            problems.push((
                "identifier_incomplete",
                "an identifier needs a system and a value".to_string(),
            ));
        }
    }
    problems
}

/// The 422 for a refused patient: `errors` as before, and `error_codes` (same order) beside it.
/// `None` when the patient is acceptable.
#[must_use]
pub fn refusal(patient: &Patient) -> Option<crate::api_error::ApiError> {
    let coded = validate_coded(patient);
    if coded.is_empty() {
        return None;
    }
    let codes: Vec<&str> = coded.iter().map(|(c, _)| *c).collect();
    let messages: Vec<String> = coded.iter().map(|(_, m)| m.clone()).collect();
    Some(crate::api_error::ApiError::invalid(messages).with("error_codes", codes))
}

pub(crate) use crate::dates::valid_birth_date;

/// Read one row into a [`Stored`] patient, putting the row's id into the record.
///
/// # Errors
///
/// [`StoreError::Db`] when a column is missing or the stored body is unreadable.
fn row_to_stored(row: &PgRow) -> Result<Stored, StoreError> {
    let id: String = row.try_get("id").map_err(db_err)?;
    let body: serde_json::Value = row.try_get("body").map_err(db_err)?;
    let mut patient: Patient = serde_json::from_value(body).map_err(db_err)?;
    patient.id = Some(id.clone());
    Ok(Stored {
        id,
        version: row.try_get("version").map_err(db_err)?,
        author: row.try_get("author").map_err(db_err)?,
        patient,
        drill: row.try_get("drill").map_err(db_err)?,
    })
}

/// A record merged into another is not a live patient: it is left out of search and of the
/// duplicate check (the surviving record is found instead). Its own rows and reads are unchanged.
const NOT_MERGED_AWAY: &str = "NOT EXISTS (
    SELECT 1 FROM patient_link_current l
    WHERE l.source_patient_id = patient_current.patient_id AND l.status = 'active')";

const COLUMNS: &str = "patient_id::text AS id, version, author, body, drill";

/// Create a real patient at version 1. The server assigns the id; any `id` in `patient` is ignored.
///
/// # Errors
///
/// [`StoreError::Db`] when the insert fails.
pub async fn create(
    db: impl Into<Db<'_>>,
    author: &str,
    reason: Option<&str>,
    patient: &Patient,
) -> Result<Stored, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    create_as(&mut *db, author, reason, patient, false).await
}

/// [`create`], as a drill or test record when `drill` is true (EC-10).
///
/// # Errors
///
/// [`StoreError::Db`] when the insert fails.
pub async fn create_as(
    db: impl Into<Db<'_>>,
    author: &str,
    reason: Option<&str>,
    patient: &Patient,
    drill: bool,
) -> Result<Stored, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let mut body = patient.clone();
    body.id = None;
    let row = sqlx::query(AssertSqlSafe(format!(
        "INSERT INTO patient_versions (patient_id, version, author, reason, body, drill)
                 VALUES (gen_random_uuid(), 1, $1, $2, $3, $4) RETURNING {COLUMNS}"
    )))
    .bind(author)
    .bind(reason.map(str::to_string))
    .bind(serde_json::to_value(&body).map_err(db_err)?)
    .bind(drill)
    .fetch_optional(&mut *db)
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
pub async fn lock_registration(db: impl Into<Db<'_>>) -> Result<(), StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(REGISTRATION_LOCK)
        .execute(&mut *db)
        .await
        .map(|_| ())
        .map_err(db_err)
}

/// The current version of a patient.
///
/// # Errors
///
/// [`StoreError::NotFound`] for an unknown or malformed id.
pub async fn current(db: impl Into<Db<'_>>, id: &str) -> Result<Stored, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let id = ids::uuid(id).ok_or(StoreError::NotFound)?;
    let row = sqlx::query(AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM patient_current WHERE patient_id = $1::uuid"
    )))
    .bind(id)
    .fetch_optional(&mut *db)
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
/// not the current version, including when a concurrent writer got there first;
/// [`StoreError::TemporaryIdentityCleared`] when the record stops being a temporary identity and
/// no reason says why.
pub async fn update(
    db: impl Into<Db<'_>>,
    id: &str,
    expected_version: i32,
    author: &str,
    reason: Option<&str>,
    patient: &Patient,
) -> Result<Stored, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let now = current(&mut *db, id).await?;
    if now.version != expected_version {
        return Err(StoreError::Conflict {
            current: now.version,
        });
    }
    if now.patient.temporary_identity
        && !patient.temporary_identity
        && reason.is_none_or(|r| r.trim().is_empty())
    {
        return Err(StoreError::TemporaryIdentityCleared);
    }
    let mut body = patient.clone();
    body.id = None;
    // A concurrent writer took this version: the primary key refuses the second (CQ-12).
    let new_version = crate::store::next_version(expected_version)?;
    let row = match crate::store::append_version(
        &mut *db,
        STORE,
        sqlx::query(AssertSqlSafe(format!(
            "INSERT INTO patient_versions (patient_id, version, author, reason, body, drill)
                 VALUES ($1::uuid, $2, $3, $4, $5, $6) RETURNING {COLUMNS}"
        )))
        .bind(now.id.clone())
        .bind(new_version)
        .bind(author)
        .bind(reason.map(str::to_string))
        .bind(serde_json::to_value(&body).map_err(db_err)?)
        .bind(now.drill),
    )
    .await?
    {
        crate::store::Appended::Row(row) => row,
        crate::store::Appended::Taken => {
            let current = current(&mut *db, id).await?.version;
            return Err(StoreError::Conflict { current });
        }
    };
    row_to_stored(&row)
}

/// The most candidates fetched for a duplicate check.
pub const CANDIDATE_LIMIT: i64 = 200;

/// The key an identifier is looked up by (AG-12): `system|value`, with the whitespace and hyphens taken out of a national
/// number, as the matcher reads one, and any other value as it was typed. Migration 45's `identifier_key` makes the same
/// key from what is stored.
#[must_use]
pub fn identifier_key(system: &str, value: &str) -> String {
    let national = [
        electronic_patient_record_matcher::NATIONAL_NUMBER_SYSTEM,
        electronic_patient_record_matcher::IHI_SYSTEM,
    ];
    if !national.contains(&system) {
        return format!("{system}|{value}");
    }
    let value: String = value
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .collect();
    format!("{system}|{value}")
}

/// A birth date with its day and month the other way round, when that is a date too and another one (AG-12).
fn transposed(birth: &str) -> Option<String> {
    let mut parts = birth.split('-');
    let (year, month, day) = (parts.next()?, parts.next()?, parts.next()?);
    let other = format!("{year}-{day}-{month}");
    (parts.next().is_none() && other != birth && valid_birth_date(&other)).then_some(other)
}

/// Current patients that share an identifier ([`identifier_key`]), a birth date or a family name (without case,
/// accents, spaces or hyphens; or one part of a double name) with `patient`, or have their names or the day and month of
/// their birth date the other way round: the pool the matcher then scores. A pair sharing none of these is not compared, so
/// a duplicate with a misspelt name, a birth date that is otherwise different and no common identifier is **not** found
/// (`spec/patient-identity/candidate-recall.md`). At most [`CANDIDATE_LIMIT`], strongest first; see [`candidates_ranked`].
///
/// # Errors
///
/// [`StoreError::Db`] on a database failure.
pub async fn candidates(
    db: impl Into<Db<'_>>,
    patient: &Patient,
) -> Result<Vec<Stored>, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    Ok(candidates_ranked(&mut *db, patient).await?.0)
}

/// [`candidates`], and whether more matched than [`CANDIDATE_LIMIT`] (CQ-36). When the list is cut,
/// the strongest evidence is kept: a shared identifier first, then the same family name and birth
/// date together, then a birth date alone, then a family name with the day and month swapped, then the names swapped, then
/// a family name alone, then a swapped date alone. Within a rank, patient id.
///
/// # Errors
///
/// [`StoreError::Db`] on a database failure.
pub async fn candidates_ranked(
    db: impl Into<Db<'_>>,
    patient: &Patient,
) -> Result<(Vec<Stored>, bool), StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    candidates_in(&mut *db, patient, false).await
}

/// [`candidates_ranked`] within one kind of record: a drill registration is compared only with drill
/// records and a real one only with real records, so the two can never be offered for a link (EC-10).
///
/// # Errors
///
/// [`StoreError::Db`] on a database failure.
pub async fn candidates_in(
    db: impl Into<Db<'_>>,
    patient: &Patient,
    drill: bool,
) -> Result<(Vec<Stored>, bool), StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let identifiers: Vec<String> = patient
        .identifiers
        .iter()
        .map(|i| identifier_key(&i.system, &i.value))
        .collect();
    let asked = serde_json::to_value(patient).map_err(db_err)?;
    let named = patient.names.iter().any(|n| {
        n.family.as_deref().is_some_and(|f| !f.trim().is_empty())
            || n.given.iter().any(|g| !g.trim().is_empty())
    });
    if identifiers.is_empty() && !named && patient.birth_date.is_none() {
        return Ok((Vec::new(), false));
    }
    let swapped_date = patient.birth_date.as_deref().and_then(transposed);
    // The names are folded by the database on both sides (migration 46): here for what is asked, once, and in the stored
    // columns for what is there. Each of these may only widen the list: the
    // matcher judges, and the strongest evidence comes first in case the list is cut.
    let keys = sqlx::query("SELECT family_keys($1) AS family, given_keys($1) AS given")
        .bind(asked)
        .fetch_one(&mut *db)
        .await
        .map_err(db_err)?;
    let family: Vec<String> = keys.try_get("family").map_err(db_err)?;
    let given: Vec<String> = keys.try_get("given").map_err(db_err)?;
    let by_identifier = "(id_keys && $1::text[])";
    let by_birth = "($2::text IS NOT NULL AND body->>'birth_date' = $2)";
    let by_swapped_date = "($3::text IS NOT NULL AND body->>'birth_date' = $3)";
    let by_family = "(family_name_keys && $4::text[])";
    let by_swapped_names = "(family_name_keys && $5::text[] AND given_name_keys && $4::text[])";
    let rows = sqlx::query(AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM patient_current
                 WHERE ({by_identifier} OR {by_birth} OR {by_family} OR {by_swapped_date} OR {by_swapped_names})
                   AND drill = $6
                   AND {NOT_MERGED_AWAY}
                 ORDER BY {by_identifier} DESC, ({by_birth} AND {by_family}) DESC, {by_birth} DESC,
                          ({by_swapped_date} AND {by_family}) DESC, {by_swapped_names} DESC,
                          {by_family} DESC, patient_id
                 LIMIT {}",
        CANDIDATE_LIMIT + 1
    )))
    .bind(identifiers)
    .bind(patient.birth_date.clone())
    .bind(swapped_date)
    .bind(family)
    .bind(given)
    .bind(drill)
    .fetch_all(&mut *db)
    .await
    .map_err(db_err)?;
    let mut found = rows
        .iter()
        .map(row_to_stored)
        .collect::<Result<Vec<_>, _>>()?;
    let cut = found.len() > usize::try_from(CANDIDATE_LIMIT).unwrap_or(usize::MAX);
    found.truncate(usize::try_from(CANDIDATE_LIMIT).unwrap_or(usize::MAX));
    Ok((found, cut))
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
    /// Search drill and test records instead of real ones (`?drill=true`, EC-10). Never both.
    #[serde(default)]
    pub drill: bool,
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

/// `s` as a `LIKE` prefix pattern: trimmed, with `%`, `_` and `\` escaped by `axa`'s `escape_like` so the
/// text is matched literally. The query says `ESCAPE '\'`, which is the escape `escape_like` writes.
fn like_prefix(s: &str) -> String {
    format!("{}%", axa::cursor_pagination::escape_like(s.trim()))
}

/// Search current versions, at most [`SEARCH_LIMIT`], ordered by family name then id.
///
/// # Errors
///
/// [`StoreError::Db`] on a database failure, or when `identifier` is not `system|value`.
pub async fn search(db: &PgPool, q: &Search) -> Result<Vec<Stored>, StoreError> {
    search_page(db, q, SEARCH_LIMIT, 0).await
}

/// [`search`], `limit` rows from `offset` in the same order (family name, then id, which is unique, so the order is
/// stable), for [`crate::cursor`]'s pages.
///
/// # Errors
///
/// As [`search`].
pub async fn search_page(
    db: &PgPool,
    q: &Search,
    limit: i64,
    offset: i64,
) -> Result<Vec<Stored>, StoreError> {
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
            Some(vec![identifier_key(system, value)])
        }
    };
    let rows = sqlx::query(AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM patient_current
                 WHERE ($1::text IS NULL OR EXISTS (
                         SELECT 1 FROM jsonb_array_elements(body->'names') n
                         WHERE lower(n->>'family') LIKE lower($1) ESCAPE '\\'))
                   AND ($2::text IS NULL OR body->>'birth_date' = $2)
                   AND ($3::text[] IS NULL OR id_keys @> $3::text[])
                   AND drill = $4
                   AND {NOT_MERGED_AWAY}
                 ORDER BY lower(body->'names'->0->>'family'), patient_id
                 LIMIT $5 OFFSET $6"
    )))
    .bind(family.map(like_prefix))
    .bind(birth.map(str::to_string))
    .bind(identifier)
    .bind(q.drill)
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await
    .map_err(db_err)?;
    rows.iter().map(row_to_stored).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use electronic_patient_record_matcher::{HumanName, Identifier};

    #[test]
    fn a_birth_date_is_transposed_only_to_another_real_date() {
        assert_eq!(transposed("1984-03-07").as_deref(), Some("1984-07-03"));
        for none in [
            "1984-03-13",
            "1984-03-03",
            "1984",
            "1984-03",
            "",
            "1984-03-07-1",
            "soon",
        ] {
            assert_eq!(transposed(none), None, "{none}");
        }
    }

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
        let coded = validate_coded(&patient("Smith", Some("1980-02-30")));
        assert_eq!(coded.len(), 1);
        assert_eq!(coded[0].0, "birth_date_invalid");
        assert_eq!(
            validate_coded(&Patient::default())
                .iter()
                .map(|(c, _)| *c)
                .collect::<Vec<_>>(),
            ["family_name_required"]
        );
        assert!(refusal(&patient("Smith", None)).is_none());
    }

    #[test]
    fn national_numbers_are_checked_and_other_systems_are_kept() {
        let mut p = patient("Smith", None);
        p.identifiers = vec![Identifier {
            system: NATIONAL_NUMBER_SYSTEM.into(),
            value: "9434765919".into(),
        }];
        assert!(validate(&p).is_empty());
        p.identifiers[0].value = "9434765910".into();
        assert!(validate(&p).iter().any(|e| e.contains("modulus 11")));
        // CQ-2: a placeholder passes modulus 11 but is refused, with its own reason.
        p.identifiers[0].value = "000 000 0000".into();
        let problems = validate(&p);
        assert!(
            problems.iter().any(|e| e.contains("placeholder")),
            "{problems:?}"
        );
        assert!(
            !problems.iter().any(|e| e.contains("modulus 11")),
            "{problems:?}"
        );
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
