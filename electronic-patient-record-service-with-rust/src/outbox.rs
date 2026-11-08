//! The transactional outbox (task OS-3; `spec/08` "Outbox"), kept and relayed by `axa-outbox` 0.2.
//!
//! [`enqueue`] writes an event in the caller's transaction, so a rollback removes it and a commit keeps
//! it. [`run`] relays: for each event it makes the read path's own decision for the *recipient*
//! ([`decide`]: `policy::evaluate` with the recipient's roles and purpose, no relationship, and the
//! patient's consent), and only a delivery reaches the recipient's [`Sink`]. Everything that happens
//! to an event is an attempt in `axa_outbox_attempts`, and a delivery, a refusal or a changed event is
//! also an audit row naming the event, never its content, written in the outcome's own transaction
//! ([`record`]), so the two commit together or not at all.
//!
//! What the decision needs (the recipient's roles, the purpose, the kind of record and the patient) is
//! kept in the payload beside what is sent, so the fingerprint `axa-outbox` stores covers it: an event
//! whose decision inputs were changed is never published.
//!
//! Any standing report that the patient has died holds an event about them (BV-3, OQ-BV1): it is not
//! delivered, writes no attempt and stays owed, because a refusal is final and a withdrawn report must
//! release it. A lookup that fails (the death store, the consents) decides nothing: the event waits and
//! is decided again, and is never delivered unchecked.
//!
//! Delivery is at least once. A refusal is final. A sink's failure is tried [`MAX_ATTEMPTS`] times in
//! all, and is then `failed` until a person releases it. Nothing calls [`enqueue`] yet and no sink
//! exists but [`LogSink`] (IO-6): this module is the mechanism, not a feature.

use crate::audit::{self, Action as AuditAction};
use crate::consent;
use crate::consents_store;
use crate::death_store;
use crate::ids;
use crate::patients_store::StoreError;
use crate::policy::{
    self, Consent, Decision as PolicyDecision, Purpose, Relationship, Resource, Role, Subject,
};
use async_trait::async_trait;
use axa::outbox::{Attempt, Claimed, Decision, Failure, Outbox, OutboxOptions, Outcome};
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, Postgres, Row, Transaction};
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use uuid::Uuid;

crate::store_errors!("outbox");

/// Tries an event gets, the first included, before it is `failed` and left for a person.
pub const MAX_ATTEMPTS: i32 = 5;

/// The wait before the first retry of an event; `axa-outbox` doubles it after each.
pub const RETRY_BACKOFF: Duration = Duration::from_secs(30);

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

/// What the decision reads, stored in the payload beside what is sent.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Route {
    /// The role names the recipient acts under.
    roles: Vec<String>,
    /// The purpose code.
    purpose: String,
    /// The resource name.
    resource: String,
    /// The patient concerned, if any.
    patient_id: Option<String>,
}

/// The stored payload: the route, and what is sent.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Envelope {
    /// What the decision reads.
    route: Route,
    /// What is sent.
    body: serde_json::Value,
}

/// An event as a [`Sink`] receives it.
#[derive(Debug, Clone)]
pub struct Delivery {
    /// Its id: a sink uses it to recognise a repeat.
    pub id: Uuid,
    /// What happened.
    pub kind: String,
    /// The sink named.
    pub recipient: String,
    /// The patient concerned, if any.
    pub patient_id: Option<String>,
    /// What is sent.
    pub body: serde_json::Value,
}

/// Where an event goes.
#[async_trait]
pub trait Sink: Send + Sync {
    /// Deliver `event`. A repeat of an event already delivered must be harmless.
    ///
    /// # Errors
    ///
    /// Why it failed, as a short code that names no clinical content: [`Failure::unavailable`] when the
    /// recipient can take nothing now, [`Failure::rejected`] when it refused this event.
    async fn deliver(&self, event: &Delivery) -> Result<(), Failure>;
}

/// The sinks by name.
pub type Sinks = HashMap<String, Box<dyn Sink>>;

/// Write an event, in the caller's transaction, and return its id. A payload over 64 KiB is refused,
/// and the caller's transaction should then not commit.
///
/// # Errors
///
/// [`StoreError::BadQuery`] for a patient id that is not a UUID; [`StoreError::Db`] otherwise.
pub async fn enqueue(
    tx: &mut Transaction<'_, Postgres>,
    new: NewEvent<'_>,
) -> Result<String, StoreError> {
    let patient = match new.patient_id {
        Some(p) => Some(
            ids::uuid(p)
                .ok_or_else(|| StoreError::BadQuery("patient id".into()))?
                .to_string(),
        ),
        None => None,
    };
    let envelope = Envelope {
        route: Route {
            roles: new.roles.iter().map(|r| r.as_str().to_string()).collect(),
            purpose: new.purpose.code().to_string(),
            resource: new.resource.name().to_string(),
            patient_id: patient.clone(),
        },
        body: new.payload.clone(),
    };
    let payload = serde_json::to_value(&envelope).map_err(db_err)?;
    let event_id = Uuid::new_v4();
    // The key keeps one patient's events in order, so a hold on a patient holds back only theirs.
    let key = patient.unwrap_or_else(|| event_id.to_string());
    let event = axa::outbox::Event {
        event_id,
        topic: new.recipient.to_string(),
        kind: new.kind.to_string(),
        key,
        payload,
    };
    axa::outbox::write(tx, &event).await.map_err(db_err)?;
    Ok(event_id.to_string())
}

/// The record decision for one event's recipient. `Ok` means the read path would allow it. `Err` is a
/// stable reason, final for this event. The recipient has no care relationship, no break-the-glass
/// and sees no masked content, so an allow that carries a masking or other obligation is refused:
/// the outbox cannot honour it.
///
/// # Errors
///
/// The reason the disclosure is refused.
pub fn permits(
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
        PolicyDecision::Allow(obligations) => {
            if obligations
                .iter()
                .all(|o| matches!(o, policy::Obligation::Audit))
            {
                Ok(())
            } else {
                Err("obligation-unsupported")
            }
        }
        PolicyDecision::Deny { reason, .. } => Err(match reason {
            policy::DenyReason::NotPermitted => "not-permitted",
            policy::DenyReason::PurposeNotAllowed => "purpose-not-allowed",
            policy::DenyReason::NoRelationship => "no-relationship",
            policy::DenyReason::ConsentDenied => "consent-denied",
        }),
    }
}

/// A lookup the decision needs failed: the event is decided again later, never delivered unchecked.
fn lookup_failed(what: &str) -> sqlx::Error {
    sqlx::Error::Protocol(format!("{what} lookup failed"))
}

/// Whether one claimed event may go, in the claim's transaction: held while any report of the
/// patient's death stands, refused for good unless the read path would allow it, and otherwise
/// delivered.
///
/// # Errors
///
/// A failed lookup, which leaves the event as it was, to be decided again.
pub async fn decide(event: &Claimed, conn: &mut PgConnection) -> Result<Decision, sqlx::Error> {
    let Ok(envelope) = serde_json::from_value::<Envelope>(event.payload.clone()) else {
        // Not written by `enqueue`, so nothing says who may have it.
        return Ok(Decision::refuse("not-an-epr-event"));
    };
    let route = envelope.route;
    // First, so that nothing about a held event is decided or sent.
    if let Some(patient) = &route.patient_id {
        let answer = death_store::answer(&mut *conn, patient)
            .await
            .map_err(|_| lookup_failed("death"))?;
        if answer.holds_contact() {
            return Ok(Decision::Hold);
        }
    }
    // An unchecked disclosure is how a refusal gets ignored: no consent answer, no decision.
    let verdict = consent_for(&mut *conn, &route)
        .await
        .map_err(|_| lookup_failed("consent"))?;
    Ok(
        match permits(
            &route.roles,
            &route.resource,
            &route.purpose,
            verdict,
            now(),
        ) {
            Ok(()) => Decision::Deliver,
            Err(reason) => Decision::refuse(reason),
        },
    )
}

/// Send one event that [`decide`] let go to its recipient's sink.
async fn publish(sinks: &Sinks, event: Claimed) -> Result<(), Failure> {
    let Some(sink) = sinks.get(&event.topic) else {
        // Not lost: tried again, then failed until a person releases it once the sink exists.
        return Err(Failure::rejected("no-sink"));
    };
    let Ok(envelope) = serde_json::from_value::<Envelope>(event.payload) else {
        return Err(Failure::rejected("not-an-epr-event"));
    };
    let delivery = Delivery {
        id: event.event_id,
        kind: event.kind,
        recipient: event.topic,
        patient_id: envelope.route.patient_id,
        body: envelope.body,
    };
    sink.deliver(&delivery).await
}

/// Count each outcome, and audit a delivery, a refusal or a changed event on the process's audit chain,
/// in the outcome's own transaction. A count is made before that transaction commits, so an outcome that then fails to
/// commit is still counted.
///
/// # Errors
///
/// The audit row could not be written, so the outcome is not written either.
pub async fn record(attempt: &Attempt, conn: &mut PgConnection) -> Result<(), sqlx::Error> {
    match attempt.outcome {
        Outcome::Delivered => crate::metrics::OUTBOX_DELIVERED.inc(),
        Outcome::Refused => crate::metrics::OUTBOX_REFUSED.inc(),
        Outcome::Rejected | Outcome::Failed | Outcome::Lost | Outcome::Abandoned => {
            crate::metrics::OUTBOX_FAILED.inc();
        }
    }
    let tampered =
        attempt.outcome == Outcome::Failed && attempt.reason.as_deref() == Some("tampered");
    if !matches!(attempt.outcome, Outcome::Delivered | Outcome::Refused) && !tampered {
        return Ok(());
    }
    // The route, for the patient and the purpose. A changed payload may not have one.
    let route: Option<Route> = sqlx::query(
        "SELECT payload -> 'route' AS route FROM axa_outbox_payloads WHERE event_id = $1",
    )
    .bind(attempt.event_id)
    .fetch_optional(&mut *conn)
    .await?
    .and_then(|r| {
        r.try_get::<Option<serde_json::Value>, _>("route")
            .ok()
            .flatten()
    })
    .and_then(|v| serde_json::from_value(v).ok());
    let mut row = audit::Event::new(AuditAction::Read, attempt.topic.clone());
    row.outcome = match attempt.outcome {
        Outcome::Delivered => 0,
        Outcome::Refused => 4,
        _ => 8,
    };
    row.resource_ref = Some(format!("outbox/{}", attempt.event_id));
    if let Some(route) = route.filter(|_| !tampered) {
        row.patient_ref = route.patient_id;
        row.purpose_of_use = Some(route.purpose);
    }
    row.detail = Some(serde_json::json!({
        "outbox": attempt.outcome.as_str(), "kind": attempt.kind, "reason": attempt.reason,
    }));
    audit::append(&mut *conn, audit::chain(), &row)
        .await
        .map_err(|e| sqlx::Error::Protocol(format!("outbox audit row: {e}")))?;
    Ok(())
}

/// A sink that logs the event's id, kind and recipient and nothing else: no payload and no patient. It
/// proves the loop end to end and sends nothing anywhere.
pub struct LogSink;

#[async_trait]
impl Sink for LogSink {
    /// Delivers by writing a log line and nothing else; no message leaves the service.
    async fn deliver(&self, event: &Delivery) -> Result<(), Failure> {
        tracing::info!(id = %event.id, kind = %event.kind, recipient = %event.recipient, "outbox event delivered to the log");
        Ok(())
    }
}

/// How often the relay looks for work when `EPR_OUTBOX_INTERVAL_SECS` is set, in seconds. Unset means
/// the relay is off. A value that is not a whole number from 1 to 3600 stops the service starting.
///
/// # Errors
///
/// The text of the bad setting.
pub fn interval_from_env() -> Result<Option<Duration>, String> {
    parse_interval(std::env::var("EPR_OUTBOX_INTERVAL_SECS").ok().as_deref())
}

/// [`interval_from_env`] over a value already read.
///
/// # Errors
///
/// The text of the bad setting.
pub fn parse_interval(raw: Option<&str>) -> Result<Option<Duration>, String> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    match raw.trim().parse::<u64>() {
        Ok(n) if (1..=3600).contains(&n) => Ok(Some(Duration::from_secs(n))),
        _ => Err(format!(
            "EPR_OUTBOX_INTERVAL_SECS must be a whole number of seconds from 1 to 3600, not {raw:?}"
        )),
    }
}

/// The relay's settings: look for work every `poll`, try an event [`MAX_ATTEMPTS`] times, waiting
/// [`RETRY_BACKOFF`] and then twice as long each time.
#[must_use]
pub fn options(poll: Duration) -> OutboxOptions {
    OutboxOptions {
        poll_interval: poll,
        max_attempts: MAX_ATTEMPTS,
        retry_backoff: RETRY_BACKOFF,
        ..OutboxOptions::default()
    }
}

/// Relay events until `shutdown` turns true: decide, publish and record each, as above.
pub async fn run(outbox: Outbox, sinks: Sinks, shutdown: watch::Receiver<bool>) {
    let sinks = Arc::new(sinks);
    outbox
        .run(
            decide,
            move |event: Claimed| {
                let sinks = Arc::clone(&sinks);
                async move { publish(&sinks, event).await }
            },
            record,
            shutdown,
        )
        .await;
}

/// How long [`Running::stop`] waits: `axa-outbox`'s publish timeout, which a publish in progress is given
/// to finish, and five seconds to hand the rest of the batch back.
pub const STOP_WAIT: Duration = Duration::from_secs(35);

/// A relay and its depth gauges, running until [`Running::stop`].
pub struct Running {
    stop: watch::Sender<bool>,
    relay: tokio::task::JoinHandle<()>,
    depth: tokio::task::JoinHandle<()>,
}

impl Running {
    /// Start [`run`] and [`watch_depth`] (every `every`) on `outbox`.
    #[must_use]
    pub fn start(outbox: Outbox, sinks: Sinks, every: Duration) -> Self {
        let (stop, stopped) = watch::channel(false);
        let depth = tokio::spawn(watch_depth(outbox.clone(), every));
        let relay = tokio::spawn(run(outbox, sinks, stopped));
        Self { stop, relay, depth }
    }

    /// Tell the relay to stop and wait up to `wait` for it: a publish in progress finishes and the rest
    /// of the batch goes back. Answers whether it stopped; one that did not is aborted and logged, and its
    /// claim is recovered by the next relay as after a crash.
    pub async fn stop(self, wait: Duration) -> bool {
        self.depth.abort();
        // A relay that has already ended has dropped its receiver; there is nothing to tell.
        let _ = self.stop.send(true);
        let abort = self.relay.abort_handle();
        match tokio::time::timeout(wait, self.relay).await {
            Ok(Ok(())) => true,
            Ok(Err(e)) => {
                tracing::error!(error = %e, "outbox relay ended in a panic");
                false
            }
            Err(_) => {
                abort.abort();
                tracing::error!(
                    seconds = wait.as_secs(),
                    "outbox relay did not stop in time; its claim is recovered as after a crash"
                );
                false
            }
        }
    }
}

/// The relay the service started, kept for [`stop_kept`].
static KEPT: std::sync::Mutex<Option<Running>> = std::sync::Mutex::new(None);

/// Keep `running` until [`stop_kept`]; one kept before is stopped first, so none is left unowned.
pub async fn keep(running: Running) {
    let before = KEPT
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .replace(running);
    if let Some(before) = before {
        before.stop(STOP_WAIT).await;
    }
}

/// Stop the kept relay, if any, waiting up to [`STOP_WAIT`]. Answers whether there was none or it stopped.
pub async fn stop_kept() -> bool {
    let kept = KEPT
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .take();
    match kept {
        Some(running) => running.stop(STOP_WAIT).await,
        None => true,
    }
}

/// Events still owed (waiting, claimed or held) and events left `failed` for a person, as
/// `(pending, stuck)`.
///
/// # Errors
///
/// [`StoreError::Db`] when the database fails.
pub async fn depth(outbox: &Outbox) -> Result<(i64, i64), StoreError> {
    let stats = outbox.stats().await.map_err(db_err)?;
    Ok((stats.pending + stats.claimed + stats.held, stats.failed))
}

/// Set the depth gauges every `every`, for ever.
pub async fn watch_depth(outbox: Outbox, every: Duration) {
    let mut tick = tokio::time::interval(every);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tick.tick().await;
        match depth(&outbox).await {
            Ok((pending, stuck)) => {
                crate::metrics::OUTBOX_PENDING.set(u64::try_from(pending).unwrap_or(0));
                crate::metrics::OUTBOX_STUCK.set(u64::try_from(stuck).unwrap_or(0));
            }
            Err(e) => tracing::error!(error = %e, "outbox depth failed"),
        }
    }
}

/// The current time in whole seconds since the Unix epoch; a clock before 1970 reads as 0 and one beyond range saturates.
fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

/// The patient's consent for this recipient reading this kind of record.
async fn consent_for(conn: &mut PgConnection, route: &Route) -> Result<Consent, StoreError> {
    let Some(patient) = &route.patient_id else {
        return Ok(Consent::NotRecorded);
    };
    let (Some(resource), Some(purpose)) = (
        Resource::parse(&route.resource),
        Purpose::parse(&route.purpose),
    ) else {
        // `permits` refuses these; there is nothing to look up.
        return Ok(Consent::NotRecorded);
    };
    let active =
        consents_store::active_with_merged(&mut *conn, std::slice::from_ref(patient)).await?;
    let ctx = consent::Context {
        roles: route.roles.iter().filter_map(|r| Role::parse(r)).collect(),
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
    fn the_migration_makes_the_tables_axa_outbox_expects() {
        let sql = include_str!("../migrations/20261108100028_axa_outbox.sql");
        let squash = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
        let migration = squash(sql);
        for statement in axa::outbox::SETUP_STATEMENTS {
            assert!(
                migration.contains(&squash(statement)),
                "the migration has drifted from axa-outbox: {statement}"
            );
        }
    }

    #[test]
    fn the_relay_settings_are_ones_axa_outbox_accepts() {
        let o = options(Duration::from_secs(5));
        assert_eq!(o.max_attempts, MAX_ATTEMPTS);
        assert_eq!(o.retry_backoff, RETRY_BACKOFF);
        assert_eq!(o.poll_interval, Duration::from_secs(5));
    }

    #[test]
    fn the_loop_interval_is_off_unless_set_and_refuses_nonsense() {
        assert_eq!(parse_interval(None), Ok(None));
        for bad in ["0", "3601", "-1", "soon", ""] {
            assert!(parse_interval(Some(bad)).is_err(), "{bad:?}");
        }
        assert_eq!(
            parse_interval(Some(" 30 ")),
            Ok(Some(Duration::from_secs(30)))
        );
    }

    #[test]
    fn an_unknown_resource_or_purpose_is_refused() {
        assert_eq!(
            permits(
                &roles(&["clinician"]),
                "nope",
                "treatment",
                Consent::NotRecorded,
                0
            ),
            Err("unknown-resource")
        );
        assert_eq!(
            permits(
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
        assert!(permits(&[], r, "operations", Consent::NotRecorded, 0).is_err());
        // A role the engine does not know grants nothing.
        assert!(
            permits(
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
            permits(
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
            permits(
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
