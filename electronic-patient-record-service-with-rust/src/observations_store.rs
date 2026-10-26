//! Observations (RC-6): the append-only store behind `/api/patients/{id}/observations`. The rules for
//! one observation are in [`crate::observations`]; this adds the time it was measured, a status, the
//! versions, and the patient. Like allergies, a correction is a new version and a mistake is
//! `entered-in-error`, which is final; reads include observations made on a record merged into this one.
//!
//! The flag is **computed when read**, from the stored value and the sender's range, and is never
//! stored: a stored flag could disagree with its own inputs, and an old flag would outlive a corrected
//! rule. It is `null` when it cannot be said (see [`crate::observations::flag`]).

use crate::ids;
use crate::links;
use crate::observations::{self, Flag, Observation};
use crate::patients_store::{self, StoreError};
use crate::store::pg;
use chrono::{DateTime, Datelike, Duration, SecondsFormat, Utc};
use sea_orm::{ConnectionTrait, TransactionTrait};
use serde::{Deserialize, Serialize};

crate::store_errors!("observations");

/// How far ahead of the server's clock a measured time may be: a clock that is a little fast is
/// normal, a time an hour ahead is a mistake.
pub const FUTURE_SKEW_MINUTES: i64 = 5;

/// Whether a version stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    /// A recorded result.
    Final,
    /// Recorded by mistake. Final: it cannot be reopened.
    EnteredInError,
}

impl Status {
    /// The stored word.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Final => "final",
            Status::EnteredInError => "entered-in-error",
        }
    }

    fn parse(s: &str) -> Option<Status> {
        match s {
            "final" => Some(Status::Final),
            "entered-in-error" => Some(Status::EnteredInError),
            _ => None,
        }
    }
}

/// One observation as written. The measurement's fields sit beside `effective_at` and `status` rather
/// than under `serde(flatten)`: with serde_json's `arbitrary_precision` (which loco turns on) a
/// flattened struct cannot read a number back.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObservationRecord {
    /// LOINC code.
    pub code: String,
    /// The measured value, or `None` with an `absent_reason`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    /// UCUM unit; required with a value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    /// Why there is no value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub absent_reason: Option<observations::AbsentReason>,
    /// The reference range the sender gave.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<observations::Range>,
    /// The critical range the sender gave.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub critical: Option<observations::Range>,
    /// When it was measured, an RFC 3339 instant. Stored in UTC.
    pub effective_at: String,
    /// Whether it stands.
    pub status: Status,
}

impl ObservationRecord {
    /// The measurement, for the rules in [`crate::observations`].
    #[must_use]
    pub fn observation(&self) -> Observation {
        Observation {
            code: self.code.clone(),
            value: self.value,
            unit: self.unit.clone(),
            absent_reason: self.absent_reason,
            range: self.range,
            critical: self.critical,
        }
    }
}

/// An observation as stored.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StoredObservation {
    /// Server id, the same for every version.
    pub id: String,
    /// The patient the record was made on.
    pub patient_id: String,
    /// 1 for the first version.
    pub version: i32,
    /// Who wrote this version.
    pub author: String,
    /// When this version was written (UTC).
    pub recorded_at: String,
    /// The site's IANA zone when it was written.
    pub site_zone: String,
    /// The record, with `effective_at` in UTC.
    #[serde(flatten)]
    pub record: ObservationRecord,
    /// Where the value sits against its range, computed now; `null` when it cannot be said.
    pub flag: Option<Flag>,
}

fn parse_effective(
    text: &str,
    now: DateTime<Utc>,
) -> Result<DateTime<Utc>, (&'static str, String)> {
    let at = DateTime::parse_from_rfc3339(text)
        .map_err(|_| {
            (
                "effective_at_invalid",
                "effective_at must be an RFC 3339 time, such as 2026-10-26T09:30:00Z".to_string(),
            )
        })?
        .with_timezone(&Utc);
    if at.year() < 1900 {
        return Err((
            "effective_at_invalid",
            "effective_at is before 1900".to_string(),
        ));
    }
    if at > now + Duration::minutes(FUTURE_SKEW_MINUTES) {
        return Err((
            "effective_at_future",
            format!("effective_at is more than {FUTURE_SKEW_MINUTES} minutes in the future"),
        ));
    }
    Ok(at)
}

/// What is wrong with a record, as a stable code and a sentence, in a fixed order (the observation's
/// own problems first); empty if nothing.
#[must_use]
pub fn validate(record: &ObservationRecord, now: DateTime<Utc>) -> Vec<(&'static str, String)> {
    let mut problems = observations::validate(&record.observation());
    if let Err(p) = parse_effective(&record.effective_at, now) {
        problems.push(p);
    }
    problems
}

/// The 422 for a refused record: `errors` and `error_codes` in the same order. `None` when acceptable.
#[must_use]
pub fn refusal(
    record: &ObservationRecord,
    now: DateTime<Utc>,
) -> Option<crate::api_error::ApiError> {
    let coded = validate(record, now);
    if coded.is_empty() {
        return None;
    }
    let codes: Vec<&str> = coded.iter().map(|(c, _)| *c).collect();
    let messages: Vec<String> = coded.iter().map(|(_, m)| m.clone()).collect();
    Some(crate::api_error::ApiError::invalid(messages).with("error_codes", codes))
}

const TS: &str = "to_char(recorded_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"')";
const EFF: &str = "to_char(effective_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"')";

fn columns() -> String {
    format!(
        "observation_id::text AS id, patient_id::text AS patient_id, version, author, {TS} AS at, \
         {EFF} AS effective, site_zone, status, body"
    )
}

fn row_to_observation(row: &sea_orm::QueryResult) -> Result<StoredObservation, StoreError> {
    let body: serde_json::Value = row.try_get("", "body").map_err(db_err)?;
    let status: String = row.try_get("", "status").map_err(db_err)?;
    let mut record: ObservationRecord = serde_json::from_value(body).map_err(db_err)?;
    // The columns are the source of truth for the status and the time.
    record.status = Status::parse(&status).ok_or_else(|| db_err("unknown stored status"))?;
    record.effective_at = row.try_get("", "effective").map_err(db_err)?;
    let flag = observations::flag(&record.observation());
    Ok(StoredObservation {
        id: row.try_get("", "id").map_err(db_err)?,
        patient_id: row.try_get("", "patient_id").map_err(db_err)?,
        version: row.try_get("", "version").map_err(db_err)?,
        author: row.try_get("", "author").map_err(db_err)?,
        recorded_at: row.try_get("", "at").map_err(db_err)?,
        site_zone: row.try_get("", "site_zone").map_err(db_err)?,
        record,
        flag,
    })
}

fn bad(record: &ObservationRecord, now: DateTime<Utc>) -> Option<StoreError> {
    let problems = validate(record, now);
    (!problems.is_empty()).then(|| {
        StoreError::BadQuery(
            problems
                .into_iter()
                .map(|(_, m)| m)
                .collect::<Vec<_>>()
                .join("; "),
        )
    })
}

fn utc_text(record: &ObservationRecord, now: DateTime<Utc>) -> Result<String, StoreError> {
    parse_effective(&record.effective_at, now)
        .map(|t| t.to_rfc3339_opts(SecondsFormat::Micros, true))
        .map_err(|(_, m)| StoreError::BadQuery(m))
}

/// Record a new observation at version 1. `site_zone` is the site's zone now.
///
/// # Errors
///
/// [`StoreError::NotFound`] (no such patient), [`StoreError::BadQuery`] (the record is refused, or
/// starts as entered in error).
pub async fn create(
    db: &impl ConnectionTrait,
    patient_id: &str,
    author: &str,
    reason: Option<&str>,
    record: &ObservationRecord,
    site_zone: &str,
    now: DateTime<Utc>,
) -> Result<StoredObservation, StoreError> {
    if let Some(e) = bad(record, now) {
        return Err(e);
    }
    if record.status == Status::EnteredInError {
        return Err(StoreError::BadQuery(
            "a new observation cannot start as entered in error".into(),
        ));
    }
    let effective = utc_text(record, now)?;
    let patient = patients_store::current(db, patient_id).await?;
    let row = db
        .query_one_raw(pg(
            &format!(
                "INSERT INTO observation_versions
                   (observation_id, version, patient_id, author, reason, status, effective_at, site_zone, body)
                 VALUES (gen_random_uuid(), 1, $1::uuid, $2, $3, $4, $5::timestamptz, $6, $7)
                 RETURNING {}",
                columns()
            ),
            vec![
                patient.id.into(),
                author.into(),
                reason.map(str::to_string).into(),
                record.status.as_str().into(),
                effective.into(),
                site_zone.into(),
                serde_json::to_value(record).map_err(db_err)?.into(),
            ],
        ))
        .await
        .map_err(db_err)?
        .ok_or_else(|| StoreError::Db("insert returned no row".into()))?;
    row_to_observation(&row)
}

/// The current version of every observation of a patient, every status, newest measurement first. It
/// includes observations made on any record **merged into** this one (their `patient_id` still names
/// the record they were made on); after an unmerge they are the other record's again.
///
/// # Errors
///
/// [`StoreError::NotFound`] (no such patient).
pub async fn list(
    db: &impl ConnectionTrait,
    patient_id: &str,
) -> Result<Vec<StoredObservation>, StoreError> {
    let patient = patients_store::current(db, patient_id).await?;
    let linked = ids::uuids(&links::linked_ids(db, &patient.id).await?);
    let rows = db
        .query_all_raw(pg(
            &format!(
                "SELECT {} FROM observation_current WHERE patient_id = ANY($1::uuid[])
                 ORDER BY effective_at DESC, observation_id",
                columns()
            ),
            vec![linked.into()],
        ))
        .await
        .map_err(db_err)?;
    rows.iter().map(row_to_observation).collect()
}

/// One observation's current version, found under the patient or under a record merged into it.
///
/// # Errors
///
/// [`StoreError::NotFound`] when there is no such observation for that patient.
pub async fn current(
    db: &impl ConnectionTrait,
    patient_id: &str,
    observation_id: &str,
) -> Result<StoredObservation, StoreError> {
    let observation_id = ids::uuid(observation_id).ok_or(StoreError::NotFound)?;
    let linked = ids::uuids(&links::linked_ids(db, patient_id).await?);
    let row = db
        .query_one_raw(pg(
            &format!(
                "SELECT {} FROM observation_current
                 WHERE observation_id = $1::uuid AND patient_id = ANY($2::uuid[])",
                columns()
            ),
            vec![observation_id.into(), linked.into()],
        ))
        .await
        .map_err(db_err)?
        .ok_or(StoreError::NotFound)?;
    row_to_observation(&row)
}

/// Append version `expected_version + 1`. A correction is a new `final` record; a mistake is
/// `status: entered-in-error`, which is final.
///
/// # Errors
///
/// [`StoreError::NotFound`], [`StoreError::Conflict`] (stale or racing version),
/// [`StoreError::BadQuery`] (refused record, or reopening an entered-in-error observation).
#[allow(clippy::too_many_arguments)]
pub async fn update(
    db: &(impl ConnectionTrait + TransactionTrait),
    patient_id: &str,
    observation_id: &str,
    expected_version: i32,
    author: &str,
    reason: Option<&str>,
    record: &ObservationRecord,
    site_zone: &str,
    now: DateTime<Utc>,
) -> Result<StoredObservation, StoreError> {
    if let Some(e) = bad(record, now) {
        return Err(e);
    }
    let effective = utc_text(record, now)?;
    let was = current(db, patient_id, observation_id).await?;
    if was.version != expected_version {
        return Err(StoreError::Conflict {
            current: was.version,
        });
    }
    if was.record.status == Status::EnteredInError && record.status != Status::EnteredInError {
        return Err(StoreError::BadQuery(
            "an observation entered in error cannot be reopened: record a new one".into(),
        ));
    }
    // A concurrent writer took this version: the primary key refuses the second (CQ-12).
    let appended = crate::store::append_version(
        db,
        STORE,
        pg(
            &format!(
                "INSERT INTO observation_versions
                   (observation_id, version, patient_id, author, reason, status, effective_at, site_zone, body)
                 VALUES ($1::uuid, $2, $3::uuid, $4, $5, $6, $7::timestamptz, $8, $9)
                 RETURNING {}",
                columns()
            ),
            vec![
                was.id.clone().into(),
                (expected_version + 1).into(),
                was.patient_id.clone().into(),
                author.into(),
                reason.map(str::to_string).into(),
                record.status.as_str().into(),
                effective.into(),
                site_zone.into(),
                serde_json::to_value(record).map_err(db_err)?.into(),
            ],
        ),
    )
    .await?;
    match appended {
        crate::store::Appended::Row(row) => row_to_observation(&row),
        crate::store::Appended::Taken => {
            let current = current(db, patient_id, observation_id).await?.version;
            Err(StoreError::Conflict { current })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observations::AbsentReason;

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-10-26T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }
    fn record(at: &str) -> ObservationRecord {
        ObservationRecord {
            code: "8867-4".into(),
            value: Some(72.0),
            unit: Some("/min".into()),
            absent_reason: None,
            range: None,
            critical: None,
            effective_at: at.into(),
            status: Status::Final,
        }
    }
    fn codes(r: &ObservationRecord) -> Vec<&'static str> {
        validate(r, now()).into_iter().map(|(c, _)| c).collect()
    }

    #[test]
    fn a_time_is_accepted_in_any_offset_and_up_to_the_skew_ahead() {
        assert!(codes(&record("2026-10-26T09:30:00Z")).is_empty());
        assert!(codes(&record("2026-10-26T05:30:00-04:00")).is_empty());
        assert!(
            codes(&record("2026-10-26T12:05:00Z")).is_empty(),
            "exactly the skew ahead"
        );
        assert!(codes(&record("1900-01-01T00:00:00Z")).is_empty());
    }

    #[test]
    fn a_time_past_the_skew_ahead_is_refused_even_when_the_offset_hides_it() {
        assert_eq!(
            codes(&record("2026-10-26T12:05:01Z")),
            ["effective_at_future"]
        );
        // 09:00 in New York is 13:00 UTC, an hour ahead of the server.
        assert_eq!(
            codes(&record("2026-10-26T09:00:00-04:00")),
            ["effective_at_future"]
        );
    }

    #[test]
    fn a_time_that_is_not_a_real_instant_is_refused() {
        for bad in [
            "",
            "yesterday",
            "2026-10-26",
            "2026-10-26 09:30",
            "2026-02-30T09:30:00Z",
            "1899-12-31T23:59:59Z",
        ] {
            assert_eq!(codes(&record(bad)), ["effective_at_invalid"], "{bad:?}");
        }
    }

    #[test]
    fn the_observation_problems_come_before_the_time_problem() {
        let mut r = record("nope");
        r.unit = Some("[degF]".into());
        assert_eq!(codes(&r), ["unit_mismatch", "effective_at_invalid"]);
    }

    #[test]
    fn a_refusal_carries_the_codes_in_the_same_order_as_the_sentences() {
        assert!(refusal(&record("2026-10-26T09:30:00Z"), now()).is_none());
        let mut r = record("nope");
        r.value = None;
        r.unit = None;
        let body = refusal(&r, now()).unwrap().body();
        assert_eq!(
            body["error_codes"],
            serde_json::json!(["value_missing", "effective_at_invalid"])
        );
        assert_eq!(body["errors"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn the_status_words_round_trip_and_an_unknown_one_is_refused() {
        for s in [Status::Final, Status::EnteredInError] {
            assert_eq!(Status::parse(s.as_str()), Some(s));
        }
        assert_eq!(Status::parse("amended"), None);
    }

    #[test]
    fn a_record_reads_flat_and_an_absent_value_keeps_its_reason() {
        let r: ObservationRecord = serde_json::from_str(
            r#"{"code":"8867-4","absent_reason":"failed","effective_at":"2026-10-26T09:30:00Z","status":"final"}"#,
        )
        .unwrap();
        assert_eq!(r.value, None);
        assert_eq!(r.absent_reason, Some(AbsentReason::Failed));
        assert!(validate(&r, now()).is_empty());
        let back = serde_json::to_value(&r).unwrap();
        assert_eq!(back["code"], "8867-4");
        assert_eq!(back["status"], "final");
    }
}
