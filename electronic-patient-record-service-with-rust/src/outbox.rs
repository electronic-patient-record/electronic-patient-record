//! The transactional outbox and its delivery runner (task OS-3, first slice; `spec/08` "Outbox").
//!
//! [`enqueue`] writes an event to `outbox_events` using the caller's transaction, so a rollback removes
//! the event and a commit keeps it. [`run_once`] later claims pending events one at a time, makes the
//! read path's own decision for the *recipient* ([`decide`]: `policy::evaluate` with the recipient's
//! roles and purpose, no relationship, and the patient's consent), and only on an allow calls the
//! recipient's [`Sink`]. Every outcome is an append-only `outbox_attempts` row, and a delivery or a
//! refusal is also an audit row naming the event, never its content.
//!
//! Delivery is at least once. A refusal is final; a failure is retried up to [`MAX_FAILURES`] times.
//! Nothing calls [`enqueue`] yet and no sink exists (IO-6): this module is the mechanism, not a feature.

use crate::audit::{self, Action as AuditAction, Chain};
use crate::consent;
use crate::consents_store;
use crate::ids;
use crate::patients_store::StoreError;
use crate::policy::{self, Consent, Decision, Purpose, Relationship, Resource, Role, Subject};
use crate::store::pg;
use async_trait::async_trait;
use sea_orm::{ConnectionTrait, DatabaseConnection, TransactionTrait};
use std::collections::{BTreeSet, HashMap};

crate::store_errors!("outbox");

/// How many `failed` attempts an event gets before it is left alone.
pub const MAX_FAILURES: i64 = 5;

/// An event to write.
#[derive(Debug, Clone)]
pub struct NewEvent<'a> {
    /// What happened, as the recipient knows it (`patient-registered`, say).
    pub kind: &'a str,
    /// The registered sink that receives it.
    pub recipient: &'a str,
    /// The roles the recipient acts under.
    pub roles: &'a [Role],
    /// The purpose of use the recipient acts under.
    pub purpose: Purpose,
    /// The kind of record the event discloses.
    pub resource: Resource,
    /// The patient concerned, if any.
    pub patient_id: Option<&'a str>,
    /// What is sent. Patient data when `patient_id` is set.
    pub payload: &'a serde_json::Value,
}

/// A stored event, as a [`Sink`] receives it.
#[derive(Debug, Clone)]
pub struct Event {
    /// Its id: a sink uses it to recognise a repeat.
    pub id: String,
    /// What happened.
    pub kind: String,
    /// The sink named.
    pub recipient: String,
    /// The role names stored with it.
    pub roles: Vec<String>,
    /// The purpose code stored with it.
    pub purpose: String,
    /// The resource name stored with it.
    pub resource: String,
    /// The patient concerned, if any.
    pub patient_id: Option<String>,
    /// What is sent.
    pub payload: serde_json::Value,
}

/// Where an event goes. A failure is a retry; the text must name no clinical content.
#[async_trait]
pub trait Sink: Send + Sync {
    /// Deliver `event`. A repeat of an event already delivered must be harmless.
    ///
    /// # Errors
    ///
    /// A reason the delivery failed.
    async fn deliver(&self, event: &Event) -> Result<(), String>;
}

/// The sinks by name.
pub type Sinks = HashMap<String, Box<dyn Sink>>;

/// What one [`run_once`] did.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// Events delivered.
    pub delivered: u32,
    /// Events refused by policy or consent.
    pub refused: u32,
    /// Attempts that failed.
    pub failed: u32,
}

/// Write an event, in the caller's transaction, and return its id.
///
/// # Errors
///
/// [`StoreError::BadQuery`] for a patient id that is not a UUID; [`StoreError::Db`] otherwise.
pub async fn enqueue(db: &impl ConnectionTrait, new: NewEvent<'_>) -> Result<String, StoreError> {
    let patient = match new.patient_id {
        Some(p) => Some(ids::uuid(p).ok_or_else(|| StoreError::BadQuery("patient id".into()))?),
        None => None,
    };
    let roles: Vec<String> = new.roles.iter().map(|r| r.as_str().to_string()).collect();
    let row = db
        .query_one_raw(pg(
            "INSERT INTO outbox_events
               (id, kind, recipient, recipient_roles, purpose, resource, patient_id, payload)
             VALUES (gen_random_uuid(), $1, $2, $3, $4, $5, $6::uuid, $7::jsonb)
             RETURNING id::text AS id",
            vec![
                new.kind.into(),
                new.recipient.into(),
                roles.into(),
                new.purpose.code().into(),
                new.resource.name().into(),
                patient.into(),
                new.payload.to_string().into(),
            ],
        ))
        .await
        .map_err(db_err)?
        .ok_or_else(|| StoreError::Db("insert returned no row".into()))?;
    row.try_get("", "id").map_err(db_err)
}

/// The record decision for one event's recipient. `Ok` means the read path would allow it. `Err` is a
/// stable reason, final for this event. The recipient has no care relationship, no break-the-glass
/// and sees no masked content, so an allow that carries a masking or other obligation is refused:
/// the outbox cannot honour it.
///
/// # Errors
///
/// The reason the disclosure is refused.
pub fn decide(
    roles: &[String],
    resource: &str,
    purpose: &str,
    consent: Consent,
    now: i64,
) -> Result<(), &'static str> {
    let resource = Resource::parse(resource).ok_or("unknown-resource")?;
    let purpose = Purpose::parse(purpose).ok_or("unknown-purpose")?;
    let subject = Subject {
        roles: roles.iter().filter_map(|r| Role::parse(r)).collect(),
        organisations: BTreeSet::new(),
    };
    let request = policy::Request {
        subject,
        resource,
        action: policy::Action::Read,
        labels: BTreeSet::new(),
        relationship: Relationship::None,
        purpose,
        consent,
        break_glass: None,
        now,
    };
    match policy::evaluate(&request) {
        Decision::Allow(obligations) => {
            if obligations
                .iter()
                .all(|o| matches!(o, policy::Obligation::Audit))
            {
                Ok(())
            } else {
                Err("obligation-unsupported")
            }
        }
        Decision::Deny { reason, .. } => Err(match reason {
            policy::DenyReason::NotPermitted => "not-permitted",
            policy::DenyReason::PurposeNotAllowed => "purpose-not-allowed",
            policy::DenyReason::NoRelationship => "no-relationship",
            policy::DenyReason::ConsentDenied => "consent-denied",
        }),
    }
}

/// A sink that logs the event's id, kind and recipient and nothing else: no payload and no patient. It
/// proves the loop end to end and sends nothing anywhere.
pub struct LogSink;

#[async_trait]
impl Sink for LogSink {
    async fn deliver(&self, event: &Event) -> Result<(), String> {
        tracing::info!(id = %event.id, kind = %event.kind, recipient = %event.recipient, "outbox event delivered to the log");
        Ok(())
    }
}

/// How often the loop runs when `EPR_OUTBOX_INTERVAL_SECS` is set, in seconds. Unset means the loop is
/// off. A value that is not a whole number from 1 to 3600 stops the service starting.
///
/// # Errors
///
/// The text of the bad setting.
pub fn interval_from_env() -> Result<Option<std::time::Duration>, String> {
    parse_interval(std::env::var("EPR_OUTBOX_INTERVAL_SECS").ok().as_deref())
}

/// [`interval_from_env`] over a value already read.
///
/// # Errors
///
/// The text of the bad setting.
pub fn parse_interval(raw: Option<&str>) -> Result<Option<std::time::Duration>, String> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    match raw.trim().parse::<u64>() {
        Ok(n) if (1..=3600).contains(&n) => Ok(Some(std::time::Duration::from_secs(n))),
        _ => Err(format!(
            "EPR_OUTBOX_INTERVAL_SECS must be a whole number of seconds from 1 to 3600, not {raw:?}"
        )),
    }
}

/// Run [`run_once`] every `every`, for ever. A database failure is logged and the loop carries on, so a
/// restart of the database does not end delivery; a panic in a sink ends only that run's task.
pub async fn run_loop(
    db: DatabaseConnection,
    chain: &'static Chain,
    sinks: Sinks,
    every: std::time::Duration,
) {
    let mut tick = tokio::time::interval(every);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tick.tick().await;
        match run_once(&db, chain, &sinks, BATCH).await {
            Ok(r) if r != Report::default() => {
                tracing::info!(
                    delivered = r.delivered,
                    refused = r.refused,
                    failed = r.failed,
                    "outbox run"
                );
            }
            Ok(_) => {}
            Err(e) => tracing::error!(error = %e, "outbox run failed"),
        }
    }
}

/// Events one run of the loop may take.
const BATCH: u32 = 100;

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

/// Seconds a first failure waits before its retry; each further failure doubles it (capped at 2^10 times).
pub const BACKOFF_BASE_SECS: i64 = 30;

/// Claim and process up to `limit` pending events, each in its own transaction, and say what happened.
///
/// # Errors
///
/// [`StoreError::Db`] when the database fails; the event being processed stays pending.
pub async fn run_once(
    db: &DatabaseConnection,
    chain: &Chain,
    sinks: &Sinks,
    limit: u32,
) -> Result<Report, StoreError> {
    run_once_with(db, chain, sinks, limit, BACKOFF_BASE_SECS).await
}

/// [`run_once`] with the first retry's wait in seconds; 0 retries at the next run (for tests).
///
/// # Errors
///
/// [`StoreError::Db`] when the database fails; the event being processed stays pending.
pub async fn run_once_with(
    db: &DatabaseConnection,
    chain: &Chain,
    sinks: &Sinks,
    limit: u32,
    backoff_secs: i64,
) -> Result<Report, StoreError> {
    let mut report = Report::default();
    // An event gets at most one attempt per run, so a failure waits for the next run instead of
    // using up every retry at once.
    let mark: i64 = db
        .query_one_raw(pg(
            "SELECT coalesce(max(id), 0) AS m FROM outbox_attempts",
            vec![],
        ))
        .await
        .map_err(db_err)?
        .ok_or_else(|| StoreError::Db("no row".into()))?
        .try_get("", "m")
        .map_err(db_err)?;
    for _ in 0..limit {
        let tx = db.begin().await.map_err(db_err)?;
        let Some(event) = claim(&tx, mark, backoff_secs).await? else {
            tx.rollback().await.map_err(db_err)?;
            break;
        };
        let (outcome, reason) = process(&tx, &event, sinks).await;
        tx.execute_raw(pg(
            "INSERT INTO outbox_attempts (event_id, outcome, reason) VALUES ($1::uuid, $2, $3)",
            vec![
                event.id.clone().into(),
                outcome.into(),
                reason.clone().into(),
            ],
        ))
        .await
        .map_err(db_err)?;
        match outcome {
            "delivered" => report.delivered += 1,
            "refused" => report.refused += 1,
            _ => report.failed += 1,
        }
        if outcome != "failed" {
            // The audit row goes last: it holds the chain's lock until the commit.
            let mut row = audit::Event::new(AuditAction::Read, event.recipient.clone());
            row.outcome = if outcome == "delivered" { 0 } else { 4 };
            row.patient_ref.clone_from(&event.patient_id);
            row.resource_ref = Some(format!("outbox/{}", event.id));
            row.purpose_of_use = Some(event.purpose.clone());
            row.detail = Some(serde_json::json!({
                "outbox": outcome, "kind": event.kind, "reason": reason,
            }));
            audit::append(&tx, chain, &row)
                .await
                .map_err(|e| db_err(e.to_string()))?;
        }
        tx.commit().await.map_err(db_err)?;
    }
    Ok(report)
}

/// The oldest pending event not attempted since `mark` (the newest attempt when the run began), locked so
/// no other runner takes it.
async fn claim(
    tx: &impl ConnectionTrait,
    mark: i64,
    backoff_secs: i64,
) -> Result<Option<Event>, StoreError> {
    let row = tx
        .query_one_raw(pg(
            &format!(
                "SELECT e.id::text AS id, e.kind, e.recipient, e.recipient_roles, e.purpose, e.resource,
                        e.patient_id::text AS patient_id, e.payload
                 FROM outbox_events e
                 WHERE NOT EXISTS (SELECT 1 FROM outbox_attempts a
                                   WHERE a.event_id = e.id AND a.outcome IN ('delivered', 'refused'))
                   AND NOT EXISTS (SELECT 1 FROM outbox_attempts a WHERE a.event_id = e.id AND a.id > $1)
                   AND (SELECT count(*) FROM outbox_attempts a
                        WHERE a.event_id = e.id AND a.outcome = 'failed') < {MAX_FAILURES}
                   AND NOT EXISTS (
                       SELECT 1 FROM outbox_attempts a
                       WHERE a.event_id = e.id AND a.outcome = 'failed'
                       HAVING max(a.attempted_at)
                              + make_interval(secs => $2::double precision
                                  * power(2, least(count(*) - 1, 10))) > now())
                 ORDER BY e.created_at, e.id
                 LIMIT 1
                 FOR UPDATE OF e SKIP LOCKED"
            ),
            vec![mark.into(), backoff_secs.into()],
        ))
        .await
        .map_err(db_err)?;
    row.map(|r| {
        Ok(Event {
            id: r.try_get("", "id").map_err(db_err)?,
            kind: r.try_get("", "kind").map_err(db_err)?,
            recipient: r.try_get("", "recipient").map_err(db_err)?,
            roles: r.try_get("", "recipient_roles").map_err(db_err)?,
            purpose: r.try_get("", "purpose").map_err(db_err)?,
            resource: r.try_get("", "resource").map_err(db_err)?,
            patient_id: r.try_get("", "patient_id").map_err(db_err)?,
            payload: r.try_get("", "payload").map_err(db_err)?,
        })
    })
    .transpose()
}

/// Decide and deliver one event: the outcome and its reason.
async fn process(
    tx: &impl ConnectionTrait,
    event: &Event,
    sinks: &Sinks,
) -> (&'static str, Option<String>) {
    let failed = |why: &str| ("failed", Some(why.to_string()));
    let Some(sink) = sinks.get(&event.recipient) else {
        return failed("no-sink");
    };
    let verdict = match consent_for(tx, event).await {
        Ok(v) => v,
        // An unchecked disclosure is how a refusal gets ignored: no consent answer, no delivery.
        Err(_) => return failed("consent-lookup"),
    };
    if let Err(reason) = decide(
        &event.roles,
        &event.resource,
        &event.purpose,
        verdict,
        now(),
    ) {
        return ("refused", Some(reason.to_string()));
    }
    match sink.deliver(event).await {
        Ok(()) => ("delivered", None),
        Err(e) => ("failed", Some(e)),
    }
}

/// The patient's consent for this recipient reading this kind of record.
async fn consent_for(tx: &impl ConnectionTrait, event: &Event) -> Result<Consent, StoreError> {
    let Some(patient) = &event.patient_id else {
        return Ok(Consent::NotRecorded);
    };
    let (Some(resource), Some(purpose)) = (
        Resource::parse(&event.resource),
        Purpose::parse(&event.purpose),
    ) else {
        // `decide` refuses these; there is nothing to look up.
        return Ok(Consent::NotRecorded);
    };
    let active = consents_store::active_with_merged(tx, std::slice::from_ref(patient)).await?;
    let ctx = consent::Context {
        roles: event.roles.iter().filter_map(|r| Role::parse(r)).collect(),
        purpose,
        resource,
        labels: BTreeSet::new(),
        now: now(),
    };
    Ok(active
        .get(&patient.to_lowercase())
        .map_or(Consent::NotRecorded, |records| {
            consent::decide(records.iter().map(|c| (c.id.as_str(), &c.consent)), &ctx).consent
        }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roles(r: &[&str]) -> Vec<String> {
        r.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn the_loop_interval_is_off_unless_set_and_refuses_nonsense() {
        assert_eq!(parse_interval(None), Ok(None));
        for bad in ["0", "3601", "-1", "soon", ""] {
            assert!(parse_interval(Some(bad)).is_err(), "{bad:?}");
        }
        assert_eq!(
            parse_interval(Some(" 30 ")),
            Ok(Some(std::time::Duration::from_secs(30)))
        );
    }

    #[test]
    fn an_unknown_resource_or_purpose_is_refused() {
        assert_eq!(
            decide(
                &roles(&["clinician"]),
                "nope",
                "treatment",
                Consent::NotRecorded,
                0
            ),
            Err("unknown-resource")
        );
        assert_eq!(
            decide(
                &roles(&["clinician"]),
                "Observation",
                "nope",
                Consent::NotRecorded,
                0
            ),
            Err("unknown-purpose")
        );
    }

    #[test]
    fn no_roles_means_no_disclosure() {
        let r = Resource::PatientIdentity.name();
        assert!(decide(&[], r, "operations", Consent::NotRecorded, 0).is_err());
        // A role the engine does not know grants nothing.
        assert!(
            decide(
                &roles(&["wizard"]),
                r,
                "operations",
                Consent::NotRecorded,
                0
            )
            .is_err()
        );
    }

    #[test]
    fn a_clinician_recipient_without_a_relationship_is_refused() {
        let r = Resource::Observation.name();
        assert_eq!(
            decide(
                &roles(&["clinician"]),
                r,
                "treatment",
                Consent::NotRecorded,
                0
            ),
            Err("no-relationship")
        );
    }

    #[test]
    fn an_integrator_never_reads_clinical_content() {
        let r = Resource::Observation.name();
        assert_eq!(
            decide(
                &roles(&["integrator"]),
                r,
                "operations",
                Consent::NotRecorded,
                0
            ),
            Err("not-permitted")
        );
    }
}
