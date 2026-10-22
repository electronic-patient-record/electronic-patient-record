//! Merge and unmerge (task ID-5): two records that are the same person.
//!
//! A merge is a **link**, never a rewrite. The record merged away (the *source*) and everything
//! recorded against it stay exactly where they are, in the append-only tables; the link says that
//! the source now stands for the *target*. Reads follow the link: the source reads as "merged into
//! X", search and the duplicate check skip it, the target's allergies include the source's, and the
//! patient's consent refusals on either record apply to both. Unmerging is a new version of the same
//! link with status `reversed`; since nothing moved, the source and the target are as they were.
//!
//! Rules, each refusing rather than guessing:
//!
//! - the two records must exist and differ; the source must not already be merged away; the target
//!   must not itself be merged away (merge into the surviving record);
//! - the matcher must not call the pair a non-match, which includes two different verified NHS
//!   numbers; its score and decision are kept with the link;
//! - a reason is required (provenance), and merging is serialised so two merges cannot race.
//!
//! A care relationship on the source does **not** carry over: access to the surviving record needs a
//! relationship to it. Consent does carry over, and a refusal on either record stands.

use crate::ids;
use crate::patients_store::{self, StoreError};
use crate::store::pg;
use electronic_patient_record_matcher::{Decision, MatchingEngine, Patient};
use sea_orm::ConnectionTrait;
use serde::Serialize;

crate::store_errors!("links");

/// Serialises merges and unmerges (a different key from the audit chain's lock).
const MERGE_LOCK: i64 = 0x4550_524d_4552_4745; // "EPRMERGE"

/// How far up a chain of merges to look.
const MAX_DEPTH: i32 = 16;

/// A link as stored.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StoredLink {
    /// The link, the same for every version.
    pub id: String,
    /// 1 when merged; 2 when unmerged.
    pub version: i32,
    /// Who wrote this version.
    pub author: String,
    /// Why (always present).
    pub reason: String,
    /// The record merged away.
    pub source: String,
    /// The record that survives.
    pub target: String,
    /// `active` or `reversed`.
    pub status: String,
    /// The matcher's score at merge time, if it was computed.
    pub match_score: Option<f64>,
    /// The matcher's decision at merge time (`auto_link` or `review`).
    pub match_decision: Option<String>,
    /// When this version was written (UTC).
    pub recorded_at: String,
}

const TS: &str = "to_char(recorded_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"')";

fn columns() -> String {
    format!(
        "link_id::text AS id, version, author, reason, source_patient_id::text AS source,
         target_patient_id::text AS target, status, match_score, match_decision, {TS} AS at"
    )
}

fn row_to_link(r: &sea_orm::QueryResult) -> Result<StoredLink, StoreError> {
    Ok(StoredLink {
        id: r.try_get("", "id").map_err(db_err)?,
        version: r.try_get("", "version").map_err(db_err)?,
        author: r.try_get("", "author").map_err(db_err)?,
        reason: r.try_get("", "reason").map_err(db_err)?,
        source: r.try_get("", "source").map_err(db_err)?,
        target: r.try_get("", "target").map_err(db_err)?,
        status: r.try_get("", "status").map_err(db_err)?,
        match_score: r.try_get("", "match_score").map_err(db_err)?,
        match_decision: r.try_get("", "match_decision").map_err(db_err)?,
        recorded_at: r.try_get("", "at").map_err(db_err)?,
    })
}

/// The matcher's verdict on a pair that is to be merged, or why the merge is refused. Two different
/// verified NHS numbers, or anything else the matcher calls a non-match, refuse. Pure: the same pair
/// gives the same answer in either order.
///
/// # Errors
///
/// The reason, in words.
pub fn judge(source: &Patient, target: &Patient) -> Result<(f64, &'static str), String> {
    let result = MatchingEngine::default().compare(source, target);
    match result.decision {
        Decision::NonMatch => Err(match result.deterministic {
            Some(d) => format!("these do not look like the same person ({d})"),
            None => format!(
                "these do not look like the same person (match score {:.2}{})",
                result.score,
                if result.conflicts.is_empty() {
                    String::new()
                } else {
                    format!("; {}", result.conflicts.join("; "))
                }
            ),
        }),
        Decision::AutoLink => Ok((result.score, "auto_link")),
        Decision::Review => Ok((result.score, "review")),
    }
}

/// Where a record has been merged to: the surviving record at the top of the chain of active
/// merges, or `None` if the record is live.
///
/// # Errors
///
/// [`StoreError::Db`] on a database failure.
pub async fn merged_into(
    db: &impl ConnectionTrait,
    patient_id: &str,
) -> Result<Option<String>, StoreError> {
    let Some(patient_id) = ids::uuid(patient_id) else {
        return Ok(None);
    };
    let row = db
        .query_one_raw(pg(
            "WITH RECURSIVE up(id, depth) AS (
                 SELECT target_patient_id, 1 FROM patient_link_current
                 WHERE source_patient_id = $1::uuid AND status = 'active'
               UNION ALL
                 SELECT l.target_patient_id, up.depth + 1
                 FROM patient_link_current l JOIN up ON l.source_patient_id = up.id
                 WHERE l.status = 'active' AND up.depth < $2
             )
             SELECT id::text AS id FROM up ORDER BY depth DESC LIMIT 1",
            vec![patient_id.into(), MAX_DEPTH.into()],
        ))
        .await
        .map_err(db_err)?;
    row.map(|r| r.try_get("", "id").map_err(db_err)).transpose()
}

/// [`sources_of`] for many targets in one query: each target's merged-in records, keyed by the
/// target (lower case). A target with none has no entry; an id that is not a UUID is ignored.
///
/// # Errors
///
/// [`StoreError::Db`] on a database failure.
pub async fn sources_of_many(
    db: &impl ConnectionTrait,
    targets: &[String],
) -> Result<std::collections::HashMap<String, Vec<String>>, StoreError> {
    let mut by_target: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    let targets = ids::uuids(targets);
    if targets.is_empty() {
        return Ok(by_target);
    }
    let rows = db
        .query_all_raw(pg(
            "WITH RECURSIVE src(root, id) AS (
                 SELECT target_patient_id, source_patient_id FROM patient_link_current
                 WHERE target_patient_id = ANY($1::uuid[]) AND status = 'active'
               UNION
                 SELECT src.root, l.source_patient_id FROM patient_link_current l
                 JOIN src ON l.target_patient_id = src.id WHERE l.status = 'active'
             )
             SELECT root::text AS root, id::text AS id FROM src ORDER BY root, id",
            vec![targets.into()],
        ))
        .await
        .map_err(db_err)?;
    for r in &rows {
        let root: String = r.try_get("", "root").map_err(db_err)?;
        let id: String = r.try_get("", "id").map_err(db_err)?;
        by_target.entry(root).or_default().push(id);
    }
    Ok(by_target)
}

/// Every record merged, directly or through a chain, into `target` and still merged.
///
/// # Errors
///
/// [`StoreError::Db`] on a database failure.
pub async fn sources_of(
    db: &impl ConnectionTrait,
    target: &str,
) -> Result<Vec<String>, StoreError> {
    let Some(target) = ids::uuid(target) else {
        return Ok(Vec::new());
    };
    let rows = db
        .query_all_raw(pg(
            "WITH RECURSIVE src(id) AS (
                 SELECT source_patient_id FROM patient_link_current
                 WHERE target_patient_id = $1::uuid AND status = 'active'
               UNION
                 SELECT l.source_patient_id FROM patient_link_current l
                 JOIN src ON l.target_patient_id = src.id WHERE l.status = 'active'
             )
             SELECT id::text AS id FROM src ORDER BY id",
            vec![target.into()],
        ))
        .await
        .map_err(db_err)?;
    rows.iter()
        .map(|r| r.try_get("", "id").map_err(db_err))
        .collect()
}

/// The record itself followed by everything merged into it: the set of patient ids whose
/// allergies, and whose consents, count for this patient.
///
/// # Errors
///
/// [`StoreError::Db`] on a database failure.
pub async fn linked_ids(
    db: &impl ConnectionTrait,
    patient_id: &str,
) -> Result<Vec<String>, StoreError> {
    let mut ids = vec![patient_id.to_lowercase()];
    ids.extend(sources_of(db, patient_id).await?);
    Ok(ids)
}

/// Every link in which the patient is the source or the target, newest first, every status.
///
/// # Errors
///
/// [`StoreError::NotFound`] (no such patient), [`StoreError::Db`].
pub async fn list(
    db: &impl ConnectionTrait,
    patient_id: &str,
) -> Result<Vec<StoredLink>, StoreError> {
    let patient = patients_store::current(db, patient_id).await?;
    let rows = db
        .query_all_raw(pg(
            &format!(
                "SELECT {} FROM patient_link_current
                 WHERE source_patient_id = $1::uuid OR target_patient_id = $1::uuid
                 ORDER BY recorded_at DESC, link_id",
                columns()
            ),
            vec![patient.id.into()],
        ))
        .await
        .map_err(db_err)?;
    rows.iter().map(row_to_link).collect()
}

async fn lock(db: &impl ConnectionTrait) -> Result<(), StoreError> {
    db.execute_raw(pg(
        "SELECT pg_advisory_xact_lock($1)",
        vec![MERGE_LOCK.into()],
    ))
    .await
    .map(|_| ())
    .map_err(db_err)
}

/// Merge `source` into `target`. Call inside a transaction (the lock lasts until it ends).
///
/// # Errors
///
/// [`StoreError::NotFound`] (either record), [`StoreError::BadQuery`] (the reason is blank, the
/// records are the same, one is already merged away, or the matcher refuses the pair).
pub async fn merge(
    db: &impl ConnectionTrait,
    target_id: &str,
    source_id: &str,
    author: &str,
    reason: &str,
) -> Result<StoredLink, StoreError> {
    if reason.trim().is_empty() {
        return Err(StoreError::BadQuery(
            "a reason is required to merge records".into(),
        ));
    }
    lock(db).await?;
    let target = patients_store::current(db, target_id).await?;
    let source = patients_store::current(db, source_id).await?;
    if target.id == source.id {
        return Err(StoreError::BadQuery(
            "a record cannot be merged into itself".into(),
        ));
    }
    if let Some(into) = merged_into(db, &source.id).await? {
        return Err(StoreError::BadQuery(format!(
            "the record to merge away was already merged into {into}"
        )));
    }
    if let Some(into) = merged_into(db, &target.id).await? {
        return Err(StoreError::BadQuery(format!(
            "the surviving record was itself merged into {into}: merge into that one"
        )));
    }
    let (score, decision) =
        judge(&source.patient, &target.patient).map_err(StoreError::BadQuery)?;
    let row = db
        .query_one_raw(pg(
            &format!(
                "INSERT INTO patient_link_versions
                   (link_id, version, author, reason, source_patient_id, target_patient_id, status,
                    match_score, match_decision)
                 VALUES (gen_random_uuid(), 1, $1, $2, $3::uuid, $4::uuid, 'active', $5, $6)
                 RETURNING {}",
                columns()
            ),
            vec![
                author.into(),
                reason.trim().into(),
                source.id.into(),
                target.id.into(),
                score.into(),
                decision.into(),
            ],
        ))
        .await
        .map_err(db_err)?
        .ok_or_else(|| StoreError::Db("insert returned no row".into()))?;
    row_to_link(&row)
}

/// Undo the merge of `source` into `target`: a new version of the link, status `reversed`. Nothing
/// else changes, because nothing was moved.
///
/// # Errors
///
/// [`StoreError::NotFound`], [`StoreError::BadQuery`] (blank reason, or no active merge of that
/// record into this one).
pub async fn unmerge(
    db: &impl ConnectionTrait,
    target_id: &str,
    source_id: &str,
    author: &str,
    reason: &str,
) -> Result<StoredLink, StoreError> {
    if reason.trim().is_empty() {
        return Err(StoreError::BadQuery(
            "a reason is required to unmerge records".into(),
        ));
    }
    lock(db).await?;
    let target = patients_store::current(db, target_id).await?;
    let source = patients_store::current(db, source_id).await?;
    let row = db
        .query_one_raw(pg(
            &format!(
                "SELECT {} FROM patient_link_current
                 WHERE source_patient_id = $1::uuid AND target_patient_id = $2::uuid
                   AND status = 'active'",
                columns()
            ),
            vec![source.id.clone().into(), target.id.clone().into()],
        ))
        .await
        .map_err(db_err)?
        .ok_or_else(|| {
            StoreError::BadQuery("that record is not currently merged into this one".into())
        })?;
    let now = row_to_link(&row)?;
    let row = db
        .query_one_raw(pg(
            &format!(
                "INSERT INTO patient_link_versions
                   (link_id, version, author, reason, source_patient_id, target_patient_id, status,
                    match_score, match_decision)
                 VALUES ($1::uuid, $2, $3, $4, $5::uuid, $6::uuid, 'reversed', $7, $8)
                 RETURNING {}",
                columns()
            ),
            vec![
                now.id.clone().into(),
                (now.version + 1).into(),
                author.into(),
                reason.trim().into(),
                source.id.into(),
                target.id.into(),
                now.match_score.into(),
                now.match_decision.clone().into(),
            ],
        ))
        .await
        .map_err(db_err)?
        .ok_or_else(|| StoreError::Db("insert returned no row".into()))?;
    row_to_link(&row)
}

#[cfg(test)]
mod tests {
    use super::*;
    use electronic_patient_record_matcher::{HumanName, Identifier, NHS_NUMBER_SYSTEM};

    fn person(family: &str, given: &str, dob: &str, nhs: Option<&str>) -> Patient {
        Patient {
            names: vec![HumanName {
                family: Some(family.into()),
                given: vec![given.into()],
            }],
            birth_date: Some(dob.into()),
            identifiers: nhs
                .map(|n| {
                    vec![Identifier {
                        system: NHS_NUMBER_SYSTEM.into(),
                        value: n.into(),
                    }]
                })
                .unwrap_or_default(),
            ..Default::default()
        }
    }

    #[test]
    fn the_same_person_may_be_merged_and_the_verdict_is_kept() {
        let a = person("Smith", "John", "1980-03-14", Some("9434765919"));
        let b = person("Smith", "John", "1980-03-14", Some("9434765919"));
        let (score, decision) = judge(&a, &b).unwrap();
        assert!(score > 0.9, "{score}");
        assert_eq!(decision, "auto_link");
        // A typo in the name, same birth date, one has no NHS number: for a person to decide.
        let c = person("Smyth", "Jon", "1980-03-14", None);
        assert!(judge(&c, &a).is_ok());
    }

    #[test]
    fn two_different_verified_nhs_numbers_refuse_the_merge_whatever_else_matches() {
        let a = person("Smith", "John", "1980-03-14", Some("9434765919"));
        // 9999999999 is a valid-looking number that differs; the matcher treats two different valid
        // numbers as a non-match.
        let b = person("Smith", "John", "1980-03-14", Some("4010232137"));
        let e = judge(&a, &b).unwrap_err();
        assert!(e.contains("do not look like the same person"), "{e}");
    }

    #[test]
    fn different_people_are_refused_and_the_order_does_not_matter() {
        let a = person("Smith", "John", "1980-03-14", None);
        let b = person("Okafor", "Chiamaka", "1992-11-30", None);
        assert!(judge(&a, &b).is_err());
        assert!(judge(&b, &a).is_err());
        let c = person("Smith", "John", "1980-03-14", None);
        assert_eq!(judge(&a, &c).is_ok(), judge(&c, &a).is_ok());
    }
}
