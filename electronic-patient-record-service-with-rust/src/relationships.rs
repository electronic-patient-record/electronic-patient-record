//! Care relationships: who is on a patient's team, who is the patient, who is their proxy.
//!
//! A relationship ties a token subject (the `sub` the audit trail records) to one patient. The
//! policy engine's patient and proxy roles work *only* through such a relationship, and clinical
//! content needs a treating one. Records are append-only versions; ending one is a new version.
//!
//! Only a `clinical-admin` creates or ends them (the table gives the subject no way to grant
//! themselves access). **Nothing here proves** the person holding a `sub` is the patient: that is
//! the administrator's identity check, outside this service.

use crate::consent::{Period, in_period};
use crate::patients_store::{self, StoreError};
use crate::policy::Relationship;
use sea_orm::{DatabaseConnection, DbBackend, Statement, Value};
use serde::{Deserialize, Serialize};

/// What the subject is to the patient.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    /// On the care team.
    CareTeam,
    /// In an active encounter.
    ActiveEncounter,
    /// The subject is the patient.
    #[serde(rename = "self")]
    SelfPatient,
    /// A registered proxy.
    Proxy,
}

impl Kind {
    /// The text stored in the `kind` column.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::CareTeam => "care-team",
            Kind::ActiveEncounter => "active-encounter",
            Kind::SelfPatient => "self",
            Kind::Proxy => "proxy",
        }
    }

    fn relationship(self) -> Relationship {
        match self {
            Kind::CareTeam => Relationship::CareTeam,
            Kind::ActiveEncounter => Relationship::ActiveEncounter,
            Kind::SelfPatient => Relationship::SelfPatient,
            Kind::Proxy => Relationship::Proxy,
        }
    }
}

/// In force, or ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// In force (within its period).
    Active,
    /// Ended. Kept for the history.
    Ended,
}

impl Status {
    fn as_str(self) -> &'static str {
        match self {
            Status::Active => "active",
            Status::Ended => "ended",
        }
    }
}

/// The body of one version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationshipRecord {
    /// The token `sub` of the person.
    pub subject: String,
    /// What they are to the patient.
    pub kind: Kind,
    /// In force or ended.
    pub status: Status,
    /// Optional dates (`YYYY-MM-DD`, inclusive).
    #[serde(default)]
    pub period: Option<Period>,
}

/// Every problem with a record.
#[must_use]
pub fn validate(r: &RelationshipRecord) -> Vec<String> {
    let mut problems = Vec::new();
    if r.subject.trim().is_empty() || r.subject.len() > 200 {
        problems.push("subject must be the person's token sub (1 to 200 characters)".to_string());
    }
    if let Some(p) = &r.period {
        for d in [&p.start, &p.end].into_iter().flatten() {
            if crate::consent::day_start(d).is_none() {
                problems.push(format!("period: {d:?} is not a real YYYY-MM-DD date"));
            }
        }
        if let (Some(a), Some(b)) = (
            p.start.as_deref().and_then(crate::consent::day_start),
            p.end.as_deref().and_then(crate::consent::day_start),
        ) && a > b
        {
            problems.push("period: the start is after the end".to_string());
        }
    }
    problems
}

/// The strongest relationship in force at `now`. Order: self, proxy, active encounter, care team,
/// none. An unreadable period counts as not in force, so corrupt data can only withhold access.
#[must_use]
pub fn resolve<'a>(
    records: impl IntoIterator<Item = &'a RelationshipRecord>,
    now: i64,
) -> Relationship {
    let rank = |r: Relationship| match r {
        Relationship::SelfPatient => 4,
        Relationship::Proxy => 3,
        Relationship::ActiveEncounter => 2,
        Relationship::CareTeam => 1,
        Relationship::None => 0,
    };
    records
        .into_iter()
        .filter(|r| r.status == Status::Active && in_period(r.period.as_ref(), now) == Some(true))
        .map(|r| r.kind.relationship())
        .max_by_key(|r| rank(*r))
        .unwrap_or(Relationship::None)
}

/// A relationship as stored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StoredRelationship {
    /// The id, the same for every version.
    pub id: String,
    /// The patient.
    pub patient_id: String,
    /// 1 for the first version.
    pub version: i32,
    /// Who wrote this version.
    pub author: String,
    /// The record.
    pub relationship: RelationshipRecord,
}

fn db_err(e: impl std::fmt::Display) -> StoreError {
    StoreError::Db(e.to_string())
}

fn pg(sql: &str, values: Vec<Value>) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, sql, values)
}

const COLUMNS: &str =
    "relationship_id::text AS id, patient_id::text AS patient_id, version, author, body";

fn row_to_stored(row: &sea_orm::QueryResult) -> Result<StoredRelationship, StoreError> {
    let body: serde_json::Value = row.try_get("", "body").map_err(db_err)?;
    Ok(StoredRelationship {
        id: row.try_get("", "id").map_err(db_err)?,
        patient_id: row.try_get("", "patient_id").map_err(db_err)?,
        version: row.try_get("", "version").map_err(db_err)?,
        author: row.try_get("", "author").map_err(db_err)?,
        relationship: serde_json::from_value(body).map_err(db_err)?,
    })
}

/// Record a relationship at version 1.
///
/// # Errors
///
/// [`StoreError::NotFound`] when the patient does not exist.
pub async fn create(
    db: &DatabaseConnection,
    patient_id: &str,
    author: &str,
    reason: Option<&str>,
    record: &RelationshipRecord,
) -> Result<StoredRelationship, StoreError> {
    let patient = patients_store::current(db, patient_id).await?;
    let row = sea_orm::ConnectionTrait::query_one_raw(
        db,
        pg(
            &format!(
                "INSERT INTO relationship_versions
                   (relationship_id, version, patient_id, subject, kind, status, author, reason, body)
                 VALUES (gen_random_uuid(), 1, $1::uuid, $2, $3, $4, $5, $6, $7) RETURNING {COLUMNS}"
            ),
            vec![
                patient.id.into(),
                record.subject.clone().into(),
                record.kind.as_str().into(),
                record.status.as_str().into(),
                author.into(),
                reason.map(str::to_string).into(),
                serde_json::to_value(record).map_err(db_err)?.into(),
            ],
        ),
    )
    .await
    .map_err(db_err)?
    .ok_or_else(|| StoreError::Db("insert returned no row".into()))?;
    row_to_stored(&row)
}

/// The current version of each relationship of a patient, ended ones included.
///
/// # Errors
///
/// [`StoreError::Db`] on a database failure.
pub async fn list(
    db: &DatabaseConnection,
    patient_id: &str,
) -> Result<Vec<StoredRelationship>, StoreError> {
    let rows = sea_orm::ConnectionTrait::query_all_raw(
        db,
        pg(
            &format!(
                "SELECT {COLUMNS} FROM relationship_current
                 WHERE patient_id::text = lower($1) ORDER BY recorded_at, relationship_id"
            ),
            vec![patient_id.into()],
        ),
    )
    .await
    .map_err(db_err)?;
    rows.iter().map(row_to_stored).collect()
}

/// The current version of one relationship of one patient.
///
/// # Errors
///
/// [`StoreError::NotFound`] when there is none.
pub async fn current(
    db: &DatabaseConnection,
    patient_id: &str,
    id: &str,
) -> Result<StoredRelationship, StoreError> {
    let row = sea_orm::ConnectionTrait::query_one_raw(
        db,
        pg(
            &format!(
                "SELECT {COLUMNS} FROM relationship_current
                 WHERE relationship_id::text = lower($1) AND patient_id::text = lower($2)"
            ),
            vec![id.into(), patient_id.into()],
        ),
    )
    .await
    .map_err(db_err)?
    .ok_or(StoreError::NotFound)?;
    row_to_stored(&row)
}

/// Append version `expected_version + 1` (ending a relationship is this with `status: ended`). The
/// subject and kind cannot be changed: end this one and record another.
///
/// # Errors
///
/// [`StoreError::NotFound`], [`StoreError::Conflict`] (stale or racing version), or
/// [`StoreError::BadQuery`] when the subject or kind differs from the stored one.
pub async fn update(
    db: &DatabaseConnection,
    patient_id: &str,
    id: &str,
    expected_version: i32,
    author: &str,
    reason: Option<&str>,
    record: &RelationshipRecord,
) -> Result<StoredRelationship, StoreError> {
    let now = current(db, patient_id, id).await?;
    if now.version != expected_version {
        return Err(StoreError::Conflict {
            current: now.version,
        });
    }
    if now.relationship.subject != record.subject || now.relationship.kind != record.kind {
        return Err(StoreError::BadQuery(
            "the subject and kind cannot change: end this relationship and record another".into(),
        ));
    }
    let inserted = sea_orm::ConnectionTrait::query_one_raw(
        db,
        pg(
            &format!(
                "INSERT INTO relationship_versions
                   (relationship_id, version, patient_id, subject, kind, status, author, reason, body)
                 VALUES ($1::uuid, $2, $3::uuid, $4, $5, $6, $7, $8, $9) RETURNING {COLUMNS}"
            ),
            vec![
                now.id.clone().into(),
                (expected_version + 1).into(),
                now.patient_id.clone().into(),
                record.subject.clone().into(),
                record.kind.as_str().into(),
                record.status.as_str().into(),
                author.into(),
                reason.map(str::to_string).into(),
                serde_json::to_value(record).map_err(db_err)?.into(),
            ],
        ),
    )
    .await;
    match inserted {
        Ok(Some(row)) => row_to_stored(&row),
        Ok(None) => Err(StoreError::Db("insert returned no row".into())),
        Err(e) if e.to_string().contains("relationship_versions_pkey") => {
            let current = current(db, patient_id, id).await?.version;
            Err(StoreError::Conflict { current })
        }
        Err(e) => Err(db_err(e)),
    }
}

/// The relationship `subject` has to a patient now: the strongest one in force.
///
/// # Errors
///
/// [`StoreError::Db`] on a database failure; the caller must fail the request, not assume none.
pub async fn lookup(
    db: &DatabaseConnection,
    patient_id: &str,
    subject: &str,
    now: i64,
) -> Result<Relationship, StoreError> {
    let rows = sea_orm::ConnectionTrait::query_all_raw(
        db,
        pg(
            "SELECT body FROM relationship_current
             WHERE patient_id::text = lower($1) AND subject = $2 AND status = 'active'",
            vec![patient_id.into(), subject.into()],
        ),
    )
    .await
    .map_err(db_err)?;
    let records = rows
        .iter()
        .map(|r| {
            let body: serde_json::Value = r.try_get("", "body").map_err(db_err)?;
            serde_json::from_value::<RelationshipRecord>(body).map_err(db_err)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(resolve(&records, now))
}

/// The patients `subject` is, or is a proxy for, now: `(patient id, kind)` for each active
/// `self` or `proxy` relationship in force, ordered by patient id. Care-team relationships are
/// not listed: this is "my record", not "my caseload".
///
/// # Errors
///
/// [`StoreError::Db`] on a database failure.
pub async fn own_patients(
    db: &DatabaseConnection,
    subject: &str,
    now: i64,
) -> Result<Vec<(String, Kind)>, StoreError> {
    let rows = sea_orm::ConnectionTrait::query_all_raw(
        db,
        pg(
            "SELECT patient_id::text AS patient_id, body FROM relationship_current
             WHERE subject = $1 AND status = 'active' AND kind IN ('self', 'proxy')
             ORDER BY patient_id",
            vec![subject.into()],
        ),
    )
    .await
    .map_err(db_err)?;
    let mut out: Vec<(String, Kind)> = Vec::new();
    for r in &rows {
        let id: String = r.try_get("", "patient_id").map_err(db_err)?;
        let body: serde_json::Value = r.try_get("", "body").map_err(db_err)?;
        let rec: RelationshipRecord = serde_json::from_value(body).map_err(db_err)?;
        if in_period(rec.period.as_ref(), now) != Some(true) {
            continue;
        }
        // One entry per patient; `self` outranks `proxy`.
        match out.iter_mut().find(|(p, _)| *p == id) {
            Some(existing) => {
                if rec.kind == Kind::SelfPatient {
                    existing.1 = Kind::SelfPatient;
                }
            }
            None => out.push((id, rec.kind)),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_790_000_000;

    fn rec(kind: Kind, status: Status) -> RelationshipRecord {
        RelationshipRecord {
            subject: "u".into(),
            kind,
            status,
            period: None,
        }
    }

    #[test]
    fn the_strongest_active_relationship_wins() {
        let all = [
            rec(Kind::CareTeam, Status::Active),
            rec(Kind::Proxy, Status::Active),
            rec(Kind::ActiveEncounter, Status::Active),
        ];
        assert_eq!(resolve(&all, NOW), Relationship::Proxy);
        assert_eq!(resolve(&all[..1], NOW), Relationship::CareTeam);
        assert_eq!(resolve(&[], NOW), Relationship::None);
        let mut with_self = all.to_vec();
        with_self.push(rec(Kind::SelfPatient, Status::Active));
        assert_eq!(resolve(&with_self, NOW), Relationship::SelfPatient);
    }

    #[test]
    fn ended_relationships_and_those_out_of_period_grant_nothing() {
        assert_eq!(
            resolve(&[rec(Kind::SelfPatient, Status::Ended)], NOW),
            Relationship::None
        );
        let day = |d: &str| crate::consent::day_start(d).unwrap();
        let mut r = rec(Kind::CareTeam, Status::Active);
        r.period = Some(Period {
            start: Some("2026-09-21".into()),
            end: Some("2026-09-22".into()),
        });
        assert_eq!(resolve(&[r.clone()], day("2026-09-20")), Relationship::None);
        assert_eq!(
            resolve(&[r.clone()], day("2026-09-22") + 86_399),
            Relationship::CareTeam
        );
        assert_eq!(resolve(&[r.clone()], day("2026-09-23")), Relationship::None);
        r.period = Some(Period {
            start: Some("garbage".into()),
            end: None,
        });
        assert_eq!(
            resolve(&[r], NOW),
            Relationship::None,
            "unreadable withholds"
        );
    }

    #[test]
    fn validation_and_json_shape() {
        assert!(validate(&rec(Kind::SelfPatient, Status::Active)).is_empty());
        let mut r = rec(Kind::Proxy, Status::Active);
        r.subject = "  ".into();
        r.period = Some(Period {
            start: Some("2026-10-02".into()),
            end: Some("2026-10-01".into()),
        });
        assert_eq!(validate(&r).len(), 2);
        let parsed: RelationshipRecord =
            serde_json::from_str(r#"{"subject":"s","kind":"self","status":"active"}"#).unwrap();
        assert_eq!(parsed.kind, Kind::SelfPatient);
        assert!(
            serde_json::from_str::<RelationshipRecord>(
                r#"{"subject":"s","kind":"boss","status":"active"}"#
            )
            .is_err()
        );
        for k in [
            Kind::CareTeam,
            Kind::ActiveEncounter,
            Kind::SelfPatient,
            Kind::Proxy,
        ] {
            assert_eq!(serde_json::to_value(k).unwrap(), k.as_str());
        }
    }
}
