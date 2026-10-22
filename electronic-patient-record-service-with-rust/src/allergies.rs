//! Allergies and intolerances: the first clinical content (task RC-5, in part).
//!
//! A FHIR `AllergyIntolerance`-shaped record per allergy (append-only versions) and, per patient, a
//! separate recorded statement "no known allergies" by a named author. The patient banner needs to
//! say one of four true things, and never leave a gap that reads as "none":
//!
//! - **Active allergies**: the list, high criticality first.
//! - **No known allergies**: a clinician said so, at a time, and nothing has been recorded since.
//! - **None active**: allergy records exist but none is active, and nobody has said "none known".
//! - **Not recorded**: nothing at all. This is *unknown*, and the banner says it does not mean none.
//!
//! A statement of "no known allergies" cannot be made while an active allergy is recorded, and an
//! allergy recorded after the statement voids it (the summary falls back to what the records show).
//! Nothing is ever deleted: ending an allergy or marking it entered-in-error is a new version, and
//! entered-in-error is final.

use crate::ids;
use crate::links;
use crate::patients_store::{self, StoreError};
use crate::store::pg;
use sea_orm::{ConnectionTrait, TransactionTrait};
use serde::{Deserialize, Serialize};

crate::store_errors!("allergies");

/// The kind of substance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Category {
    /// A food.
    Food,
    /// A medicine.
    Medication,
    /// Something in the environment (latex, pollen).
    Environment,
    /// A biological product.
    Biologic,
}

/// How serious a reaction could be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Criticality {
    /// Unlikely to be life-threatening.
    Low,
    /// Potentially life-threatening.
    High,
    /// Not assessed. Shown as such, never as low.
    UnableToAssess,
}

/// Whether the allergy is current.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    /// Current: it counts.
    Active,
    /// No longer applies (resolved). Kept in the history.
    Inactive,
    /// Recorded by mistake. Final: it cannot be reopened.
    EnteredInError,
}

impl Status {
    /// The stored word.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Active => "active",
            Status::Inactive => "inactive",
            Status::EnteredInError => "entered-in-error",
        }
    }

    fn parse(s: &str) -> Option<Status> {
        match s {
            "active" => Some(Status::Active),
            "inactive" => Some(Status::Inactive),
            "entered-in-error" => Some(Status::EnteredInError),
            _ => None,
        }
    }
}

/// A coded substance, optional.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Coding {
    /// The code system URI. A SNOMED CT code is checked (check digit, names a concept).
    pub system: String,
    /// The code.
    pub code: String,
}

/// One allergy as recorded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllergyRecord {
    /// What the patient reacts to, in words. Required: a code alone is not readable at the bedside.
    pub substance: String,
    /// The same, coded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coding: Option<Coding>,
    /// The kind of substance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<Category>,
    /// How serious. Absent means not stated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub criticality: Option<Criticality>,
    /// The reaction, in words.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reaction: Option<String>,
    /// When it started: `YYYY`, `YYYY-MM` or `YYYY-MM-DD`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub onset: Option<String>,
    /// Current or not.
    pub status: Status,
}

/// What is wrong with a record, in words; empty if nothing.
#[must_use]
pub fn validate(r: &AllergyRecord) -> Vec<String> {
    let mut problems = Vec::new();
    let text = r.substance.trim();
    if text.is_empty() {
        problems.push("the substance is required".to_string());
    } else if text.chars().count() > 200 || text.chars().any(char::is_control) {
        problems
            .push("the substance must be up to 200 characters, without control characters".into());
    }
    if let Some(c) = &r.coding {
        let system_ok = c.system.starts_with("http://")
            || c.system.starts_with("https://")
            || c.system.starts_with("urn:");
        if !system_ok || c.system.len() > 200 {
            problems.push("the code system must be a URI (http, https or urn)".into());
        } else if c.code.trim().is_empty()
            || c.code.len() > 64
            || c.code.chars().any(char::is_control)
        {
            problems.push("the code must be 1 to 64 characters".into());
        } else if let Err(m) = crate::interop::terminology::validate_coding(&c.system, &c.code) {
            problems.push(m);
        }
    }
    if let Some(reaction) = &r.reaction
        && (reaction.chars().count() > 500 || reaction.chars().any(|c| c.is_control() && c != '\n'))
    {
        problems.push("the reaction must be up to 500 characters".into());
    }
    if let Some(onset) = &r.onset
        && !patients_store::valid_birth_date(onset)
    {
        problems.push("onset must be YYYY, YYYY-MM or YYYY-MM-DD".into());
    }
    problems
}

/// An allergy as stored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredAllergy {
    /// Server id, the same for every version.
    pub id: String,
    /// The patient.
    pub patient_id: String,
    /// 1 for the first version.
    pub version: i32,
    /// Who wrote this version.
    pub author: String,
    /// When this version was written (UTC).
    pub recorded_at: String,
    /// The record.
    pub allergy: AllergyRecord,
}

/// The patient-level statement "no known allergies".
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Assertion {
    /// 1 for the first statement.
    pub version: i32,
    /// Who made it.
    pub author: String,
    /// When (UTC).
    pub recorded_at: String,
    /// `true` for "no known allergies"; `false` when that statement was withdrawn.
    pub no_known_allergies: bool,
}

/// What the banner should say.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub enum Summary {
    /// At least one active allergy.
    Active {
        /// Substance and criticality, high criticality first.
        items: Vec<SummaryItem>,
    },
    /// A clinician stated there are none, and nothing has been recorded since.
    NoKnown {
        /// When it was stated.
        asserted_at: String,
        /// Who stated it.
        asserted_by: String,
    },
    /// Records exist but none is active, and "none known" has not been stated since.
    NoneActive {
        /// How many inactive records there are.
        inactive: usize,
    },
    /// Nothing is recorded. Unknown, not none.
    NotRecorded,
}

/// One line of the active list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SummaryItem {
    /// The substance in words.
    pub substance: String,
    /// How serious, if stated.
    pub criticality: Option<Criticality>,
}

/// The banner summary from the current allergy versions and the current statement.
///
/// `allergies` are the current version of each allergy. Entered-in-error ones are ignored: they
/// never happened. A statement is valid only if no allergy version was written after it.
#[must_use]
pub fn summarise(allergies: &[StoredAllergy], statement: Option<&Assertion>) -> Summary {
    let real: Vec<&StoredAllergy> = allergies
        .iter()
        .filter(|a| a.allergy.status != Status::EnteredInError)
        .collect();
    let mut active: Vec<&StoredAllergy> = real
        .iter()
        .copied()
        .filter(|a| a.allergy.status == Status::Active)
        .collect();
    if !active.is_empty() {
        // High criticality first, then not-assessed, then low or unstated; then by name.
        let rank = |c: Option<Criticality>| match c {
            Some(Criticality::High) => 0,
            Some(Criticality::UnableToAssess) => 1,
            _ => 2,
        };
        active.sort_by(|a, b| {
            rank(a.allergy.criticality)
                .cmp(&rank(b.allergy.criticality))
                .then_with(|| {
                    a.allergy
                        .substance
                        .to_lowercase()
                        .cmp(&b.allergy.substance.to_lowercase())
                })
        });
        return Summary::Active {
            items: active
                .into_iter()
                .map(|a| SummaryItem {
                    substance: a.allergy.substance.clone(),
                    criticality: a.allergy.criticality,
                })
                .collect(),
        };
    }
    if let Some(s) = statement.filter(|s| s.no_known_allergies) {
        // Timestamps are `YYYY-MM-DDTHH:MM:SS.ffffffZ`: the text sorts as time.
        let touched_since = allergies.iter().any(|a| a.recorded_at > s.recorded_at);
        if !touched_since {
            return Summary::NoKnown {
                asserted_at: s.recorded_at.clone(),
                asserted_by: s.author.clone(),
            };
        }
    }
    if real.is_empty() {
        Summary::NotRecorded
    } else {
        Summary::NoneActive {
            inactive: real.len(),
        }
    }
}

const TS: &str = "to_char(recorded_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"')";

fn columns() -> String {
    format!(
        "allergy_id::text AS id, patient_id::text AS patient_id, version, author, {TS} AS at, status, body"
    )
}

fn row_to_allergy(row: &sea_orm::QueryResult) -> Result<StoredAllergy, StoreError> {
    let body: serde_json::Value = row.try_get("", "body").map_err(db_err)?;
    let status: String = row.try_get("", "status").map_err(db_err)?;
    let mut allergy: AllergyRecord = serde_json::from_value(body).map_err(db_err)?;
    // The column is the source of truth for the status.
    allergy.status = Status::parse(&status).ok_or_else(|| db_err("unknown stored status"))?;
    Ok(StoredAllergy {
        id: row.try_get("", "id").map_err(db_err)?,
        patient_id: row.try_get("", "patient_id").map_err(db_err)?,
        version: row.try_get("", "version").map_err(db_err)?,
        author: row.try_get("", "author").map_err(db_err)?,
        recorded_at: row.try_get("", "at").map_err(db_err)?,
        allergy,
    })
}

/// Record a new allergy at version 1.
///
/// # Errors
///
/// [`StoreError::NotFound`] (no such patient), [`StoreError::BadQuery`] (the record is refused).
pub async fn create(
    db: &impl ConnectionTrait,
    patient_id: &str,
    author: &str,
    reason: Option<&str>,
    record: &AllergyRecord,
) -> Result<StoredAllergy, StoreError> {
    let problems = validate(record);
    if !problems.is_empty() {
        return Err(StoreError::BadQuery(problems.join("; ")));
    }
    if record.status == Status::EnteredInError {
        return Err(StoreError::BadQuery(
            "a new allergy cannot start as entered in error".into(),
        ));
    }
    let patient = patients_store::current(db, patient_id).await?;
    let row = db
        .query_one_raw(pg(
            &format!(
                "INSERT INTO allergy_versions (allergy_id, version, patient_id, author, reason, status, body)
                 VALUES (gen_random_uuid(), 1, $1::uuid, $2, $3, $4, $5) RETURNING {}",
                columns()
            ),
            vec![
                patient.id.into(),
                author.into(),
                reason.map(str::to_string).into(),
                record.status.as_str().into(),
                serde_json::to_value(record).map_err(db_err)?.into(),
            ],
        ))
        .await
        .map_err(db_err)?
        .ok_or_else(|| StoreError::Db("insert returned no row".into()))?;
    row_to_allergy(&row)
}

/// The current version of every allergy of a patient, every status, oldest first. It includes the
/// allergies recorded against any record **merged into** this one (their `patient_id` still names
/// the record they were made on): a person's allergies must not disappear because two records were
/// joined. After an unmerge they are the other record's again, untouched.
///
/// # Errors
///
/// [`StoreError::NotFound`] (no such patient).
pub async fn list(
    db: &impl ConnectionTrait,
    patient_id: &str,
) -> Result<Vec<StoredAllergy>, StoreError> {
    let patient = patients_store::current(db, patient_id).await?;
    let linked = ids::uuids(&links::linked_ids(db, &patient.id).await?);
    let rows = db
        .query_all_raw(pg(
            &format!(
                "SELECT {} FROM allergy_current WHERE patient_id = ANY($1::uuid[])
                 ORDER BY recorded_at, allergy_id",
                columns()
            ),
            vec![linked.into()],
        ))
        .await
        .map_err(db_err)?;
    rows.iter().map(row_to_allergy).collect()
}

/// One allergy's current version, found under the patient or under a record merged into it.
///
/// # Errors
///
/// [`StoreError::NotFound`] when there is no such allergy for that patient.
pub async fn current(
    db: &impl ConnectionTrait,
    patient_id: &str,
    allergy_id: &str,
) -> Result<StoredAllergy, StoreError> {
    let allergy_id = ids::uuid(allergy_id).ok_or(StoreError::NotFound)?;
    let linked = ids::uuids(&links::linked_ids(db, patient_id).await?);
    let row = db
        .query_one_raw(pg(
            &format!(
                "SELECT {} FROM allergy_current
                 WHERE allergy_id = $1::uuid AND patient_id = ANY($2::uuid[])",
                columns()
            ),
            vec![allergy_id.into(), linked.into()],
        ))
        .await
        .map_err(db_err)?
        .ok_or(StoreError::NotFound)?;
    row_to_allergy(&row)
}

/// Append version `expected_version + 1`. Ending an allergy is `status: inactive`; a mistake is
/// `status: entered-in-error`, which is final.
///
/// # Errors
///
/// [`StoreError::NotFound`], [`StoreError::Conflict`] (stale or racing version),
/// [`StoreError::BadQuery`] (refused record, or reopening an entered-in-error allergy).
pub async fn update(
    db: &(impl ConnectionTrait + TransactionTrait),
    patient_id: &str,
    allergy_id: &str,
    expected_version: i32,
    author: &str,
    reason: Option<&str>,
    record: &AllergyRecord,
) -> Result<StoredAllergy, StoreError> {
    let problems = validate(record);
    if !problems.is_empty() {
        return Err(StoreError::BadQuery(problems.join("; ")));
    }
    let now = current(db, patient_id, allergy_id).await?;
    if now.version != expected_version {
        return Err(StoreError::Conflict {
            current: now.version,
        });
    }
    if now.allergy.status == Status::EnteredInError && record.status != Status::EnteredInError {
        return Err(StoreError::BadQuery(
            "an allergy entered in error cannot be reopened: record a new one".into(),
        ));
    }
    // A concurrent writer took this version: the primary key refuses the second (CQ-12).
    let row = match crate::store::append_version(db, STORE, pg(
            &format!(
                "INSERT INTO allergy_versions (allergy_id, version, patient_id, author, reason, status, body)
                 VALUES ($1::uuid, $2, $3::uuid, $4, $5, $6, $7) RETURNING {}",
                columns()
            ),
            vec![
                now.id.clone().into(),
                (expected_version + 1).into(),
                now.patient_id.clone().into(),
                author.into(),
                reason.map(str::to_string).into(),
                record.status.as_str().into(),
                serde_json::to_value(record).map_err(db_err)?.into(),
            ],
        )).await? {
        crate::store::Appended::Row(row) => row,
        crate::store::Appended::Taken => {
            let current = current(db, patient_id, allergy_id).await?.version;
            return Err(StoreError::Conflict { current });
        }
    };
    row_to_allergy(&row)
}

/// The patient's current "no known allergies" statement, if one was ever made.
///
/// # Errors
///
/// [`StoreError::NotFound`] (no such patient).
pub async fn statement(
    db: &impl ConnectionTrait,
    patient_id: &str,
) -> Result<Option<Assertion>, StoreError> {
    let patient = patients_store::current(db, patient_id).await?;
    let row = db
        .query_one_raw(pg(
            &format!(
                "SELECT version, author, {TS} AS at, kind FROM allergy_assertion_current
                 WHERE patient_id = $1::uuid"
            ),
            vec![patient.id.into()],
        ))
        .await
        .map_err(db_err)?;
    row.map(|r| {
        Ok(Assertion {
            version: r.try_get("", "version").map_err(db_err)?,
            author: r.try_get("", "author").map_err(db_err)?,
            recorded_at: r.try_get("", "at").map_err(db_err)?,
            no_known_allergies: r.try_get::<String>("", "kind").map_err(db_err)?
                == "no-known-allergies",
        })
    })
    .transpose()
}

/// State "no known allergies" (`true`) or withdraw that statement (`false`). `expected_version` is
/// the statement version the caller last saw (0 for none). Refused while an active allergy is
/// recorded: it would contradict the record.
///
/// # Errors
///
/// [`StoreError::NotFound`], [`StoreError::Conflict`], [`StoreError::BadQuery`].
pub async fn assert_none_known(
    db: &(impl ConnectionTrait + TransactionTrait),
    patient_id: &str,
    expected_version: i32,
    author: &str,
    reason: Option<&str>,
    none_known: bool,
) -> Result<Assertion, StoreError> {
    let have = statement(db, patient_id).await?;
    let current_version = have.as_ref().map_or(0, |a| a.version);
    if current_version != expected_version {
        return Err(StoreError::Conflict {
            current: current_version,
        });
    }
    if none_known {
        let active = list(db, patient_id)
            .await?
            .iter()
            .filter(|a| a.allergy.status == Status::Active)
            .count();
        if active > 0 {
            return Err(StoreError::BadQuery(format!(
                "{active} active allergy record(s) exist: end them first, or do not state that there are none"
            )));
        }
    } else if !have.as_ref().is_some_and(|a| a.no_known_allergies) {
        return Err(StoreError::BadQuery(
            "there is no statement to withdraw".into(),
        ));
    }
    let id = patients_store::current(db, patient_id).await?.id;
    // A concurrent writer took this version: the primary key refuses the second (CQ-12).
    let r = match crate::store::append_version(
        db,
        STORE,
        pg(
            &format!(
                "INSERT INTO allergy_assertion_versions (patient_id, version, author, reason, kind)
                 VALUES ($1::uuid, $2, $3, $4, $5) RETURNING version, author, {TS} AS at, kind"
            ),
            vec![
                id.into(),
                (expected_version + 1).into(),
                author.into(),
                reason.map(str::to_string).into(),
                (if none_known {
                    "no-known-allergies"
                } else {
                    "withdrawn"
                })
                .into(),
            ],
        ),
    )
    .await?
    {
        crate::store::Appended::Row(row) => row,
        crate::store::Appended::Taken => {
            let current = statement(db, patient_id).await?.map_or(0, |a| a.version);
            return Err(StoreError::Conflict { current });
        }
    };
    Ok(Assertion {
        version: r.try_get("", "version").map_err(db_err)?,
        author: r.try_get("", "author").map_err(db_err)?,
        recorded_at: r.try_get("", "at").map_err(db_err)?,
        no_known_allergies: none_known,
    })
}

/// The banner summary for a patient.
///
/// # Errors
///
/// [`StoreError::NotFound`] (no such patient).
pub async fn summary(db: &impl ConnectionTrait, patient_id: &str) -> Result<Summary, StoreError> {
    let all = list(db, patient_id).await?;
    let statement = statement(db, patient_id).await?;
    Ok(summarise(&all, statement.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(substance: &str, status: Status, criticality: Option<Criticality>) -> AllergyRecord {
        AllergyRecord {
            substance: substance.into(),
            coding: None,
            category: None,
            criticality,
            reaction: None,
            onset: None,
            status,
        }
    }

    fn stored(
        substance: &str,
        status: Status,
        crit: Option<Criticality>,
        at: &str,
    ) -> StoredAllergy {
        StoredAllergy {
            id: substance.into(),
            patient_id: "p".into(),
            version: 1,
            author: "a".into(),
            recorded_at: at.into(),
            allergy: rec(substance, status, crit),
        }
    }

    fn none_known(at: &str) -> Assertion {
        Assertion {
            version: 1,
            author: "dr".into(),
            recorded_at: at.into(),
            no_known_allergies: true,
        }
    }

    const T1: &str = "2026-10-21T09:00:00.000000Z";
    const T2: &str = "2026-10-21T10:00:00.000000Z";

    #[test]
    fn nothing_recorded_is_unknown_never_none() {
        assert_eq!(summarise(&[], None), Summary::NotRecorded);
        // A withdrawn statement says nothing either.
        let mut withdrawn = none_known(T1);
        withdrawn.no_known_allergies = false;
        assert_eq!(summarise(&[], Some(&withdrawn)), Summary::NotRecorded);
    }

    #[test]
    fn active_allergies_come_first_high_criticality_leading() {
        let all = [
            stored("Latex", Status::Active, Some(Criticality::Low), T1),
            stored("Penicillin", Status::Active, Some(Criticality::High), T1),
            stored("Pollen", Status::Active, None, T1),
            stored(
                "Iodine",
                Status::Active,
                Some(Criticality::UnableToAssess),
                T1,
            ),
        ];
        let Summary::Active { items } = summarise(&all, None) else {
            panic!()
        };
        let names: Vec<&str> = items.iter().map(|i| i.substance.as_str()).collect();
        assert_eq!(names, ["Penicillin", "Iodine", "Latex", "Pollen"]);
        // An active allergy beats a "none known" statement, whatever its date.
        assert!(matches!(
            summarise(&all, Some(&none_known(T2))),
            Summary::Active { .. }
        ));
    }

    #[test]
    fn none_known_stands_until_something_is_recorded_after_it() {
        assert!(matches!(
            summarise(&[], Some(&none_known(T1))),
            Summary::NoKnown { .. }
        ));
        // An allergy recorded, then ended, after the statement voids it.
        let later = [stored("Nuts", Status::Inactive, None, T2)];
        assert_eq!(
            summarise(&later, Some(&none_known(T1))),
            Summary::NoneActive { inactive: 1 }
        );
        // The same record made before the statement does not.
        let earlier = [stored("Nuts", Status::Inactive, None, T1)];
        assert!(matches!(
            summarise(&earlier, Some(&none_known(T2))),
            Summary::NoKnown { .. }
        ));
    }

    #[test]
    fn entered_in_error_never_counts() {
        let mistaken = [stored(
            "Penicillin",
            Status::EnteredInError,
            Some(Criticality::High),
            T1,
        )];
        assert_eq!(summarise(&mistaken, None), Summary::NotRecorded);
        assert!(matches!(
            summarise(&mistaken, Some(&none_known(T2))),
            Summary::NoKnown { .. }
        ));
    }

    #[test]
    fn only_inactive_records_say_so_and_do_not_claim_none() {
        let all = [
            stored("Nuts", Status::Inactive, None, T1),
            stored("Dust", Status::Inactive, None, T1),
        ];
        assert_eq!(summarise(&all, None), Summary::NoneActive { inactive: 2 });
    }

    #[test]
    fn validation_catches_missing_substance_bad_codes_and_dates() {
        assert!(validate(&rec("Penicillin", Status::Active, None)).is_empty());
        assert!(!validate(&rec("  ", Status::Active, None)).is_empty());
        assert!(!validate(&rec(&"x".repeat(201), Status::Active, None)).is_empty());
        let mut r = rec("Penicillin", Status::Active, None);
        r.coding = Some(Coding {
            system: "http://snomed.info/sct".into(),
            code: "764146007".into(),
        });
        assert!(validate(&r).is_empty(), "{:?}", validate(&r));
        r.coding = Some(Coding {
            system: "http://snomed.info/sct".into(),
            code: "764146008".into(),
        });
        assert!(validate(&r).iter().any(|p| p.contains("SNOMED")));
        r.coding = Some(Coding {
            system: "javascript:alert(1)".into(),
            code: "1".into(),
        });
        assert!(validate(&r).iter().any(|p| p.contains("URI")));
        r.coding = None;
        for onset in ["2020", "2020-05", "2020-05-17"] {
            r.onset = Some(onset.into());
            assert!(validate(&r).is_empty(), "{onset}");
        }
        for onset in ["last year", "2020-13", "2020-02-30"] {
            r.onset = Some(onset.into());
            assert!(!validate(&r).is_empty(), "{onset}");
        }
    }
}
