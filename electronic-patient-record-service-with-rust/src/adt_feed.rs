//! Applying an HL7 v2 ADT message to the patient record (task OR-3): the part of the MLLP listener
//! that decides what a message *does*.
//!
//! `interop::hl7_adt` reads the message into an identity record; this module matches it to a
//! patient, applies it, audits it and answers. The rules lean toward doing nothing and saying why,
//! because the feed has no human watching it:
//!
//! - The sender is not trusted to name itself: the audit agent is configured, and the peer address
//!   is recorded. The integrator role's permission to create and update identity is checked through
//!   the policy engine like any other caller's.
//! - A message is matched on the identifiers it carries (the sender's own number, `urn:hl7v2:…`, then
//!   a valid national health number). One match is an update; none is a new patient; more than one is refused.
//! - A new patient that the matcher thinks may already exist is **refused**, not created: a duplicate
//!   made by a machine at 3 a.m. is worse than a message that waits.
//! - An update never blanks a field the message does not carry, and it never changes a birth date or
//!   an national health number: those corrections are made by a person, in the UI, with a reason.
//! - A death on a message (BV-2) is a report: it is stored as an assertion from this feed, it holds
//!   contact at once, and the message is answered `AA`. It never touches the identity record, and only
//!   a person can confirm or withdraw it. A repeat of what the feed already said adds nothing, because
//!   a source that sets PID-29 sends it on every later message. A repeat of a report a person withdrew
//!   as an error is refused, so that neither the error nor a real death passes unseen. A message about
//!   nobody on record is refused rather than creating a patient in order to report them dead.
//! - The change and its audit row are one transaction; the audit row holds the event, the control id
//!   and what happened, never the message.
//!
//! - A refusal kept in the work queue (`feed_queue`) is answered `AR` by default (IH-1): a person will deal with it, so
//!   the sender should not send it again. `AE` is kept for what a resend can fix (the database unavailable, the
//!   refusal not stored). A resend of a message already waiting adds no row and gets the same answer.
//!   `EPR_MLLP_KEPT_ANSWER=AE` answers kept refusals `AE` instead, for a sender that needs it (OQ-IH1).
//!
//! Not built: `A40` merges, `PV1`, `ORU`/`SIU`. (TLS is the listener's: [`crate::mllp`].)

use crate::audit::{self, Action, Event};
use crate::controllers::patients::likely_duplicates;
use crate::death::{self, Actor, Assertion, Occurred, Source, Status};
use crate::death_store;
use crate::feed_queue;
use crate::interop::hl7_adt::{self, Inbound, Rejection};
use crate::patients_store::{self as store, StoreError};
use crate::policy::{self, Consent, Purpose, Relationship, Resource, Role, Subject};
use electronic_patient_record_matcher::{Identifier, NATIONAL_NUMBER_SYSTEM, Patient};
use sqlx::PgPool;

/// Who the feed is, for the audit trail and the policy check. The sender's own claim (MSH-3, MSH-4)
/// is never used for either.
#[derive(Debug, Clone)]
pub struct FeedIdentity {
    /// The audit agent, e.g. `mllp:adt-feed`.
    pub agent: String,
    /// The peer's address, recorded on every row.
    pub peer: Option<String>,
    /// The name the sender's TLS client certificate was checked for (`EPR_MLLP_TLS_CLIENT_NAMES`), recorded on every
    /// audit row as `peer_certificate`; `None` without TLS client names.
    pub peer_certificate: Option<String>,
    /// How a refusal kept in the work queue is answered.
    pub kept_answer: KeptAnswer,
    /// The processing id (MSH-11) this deployment takes, `P`, `T` or `D`; any other is refused and kept (IH-2). `None`
    /// takes any, as before.
    pub processing_id: Option<char>,
    /// What this feed's sender may send (IH-6); empty lists take anything.
    pub rules: SenderRules,
}

impl FeedIdentity {
    /// An audit row's detail with `peer_certificate` added when the sender's certificate was checked for a name.
    fn detail(&self, mut detail: serde_json::Value) -> serde_json::Value {
        if let (Some(name), Some(map)) = (&self.peer_certificate, detail.as_object_mut()) {
            map.insert("peer_certificate".into(), name.clone().into());
        }
        detail
    }
}

/// What a feed's sender may send (IH-6), from the listener's settings. An empty list takes anything, as before.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SenderRules {
    /// Sending applications, each `APP` (MSH-3) or `APP^FACILITY` (MSH-3 and MSH-4), first components only.
    pub applications: Vec<String>,
    /// HL7 versions (MSH-12), such as `2.5` or `2.5.1`.
    pub versions: Vec<String>,
    /// Trigger events (MSH-9.2), a subset of [`hl7_adt::HANDLED_EVENTS`].
    pub events: Vec<String>,
}

/// Why a message from an application this feed does not take is refused (IH-6). Like every reason, it quotes nothing.
pub const WRONG_APPLICATION: &str = "MSH-3 and MSH-4 (sending application and facility) are not ones this feed takes: a person must check where this message came from";
/// Why a message of another HL7 version is refused (IH-6).
pub const WRONG_VERSION: &str = "MSH-12 (HL7 version) is not one this feed takes";
/// Why a trigger event this feed does not take is refused (IH-6).
pub const WRONG_EVENT: &str = "this trigger event is not one this feed takes";

/// Field `n` of the MSH segment (MSH-1 is the separator), first component only, read without a full parse.
fn msh_first_component(text: &str, n: usize) -> Option<String> {
    let msh = text.split(['\r', '\n']).find(|l| l.starts_with("MSH"))?;
    let sep = msh.chars().nth(3)?;
    let component = msh.chars().nth(4).unwrap_or('^');
    let field = msh.split(sep).nth(n - 1)?;
    Some(
        field
            .split(component)
            .next()
            .unwrap_or_default()
            .to_string(),
    )
}

impl SenderRules {
    /// The first rule `text` breaks, if any.
    fn broken_by(&self, text: &str) -> Option<&'static str> {
        let app = msh_first_component(text, 3).unwrap_or_default();
        let facility = msh_first_component(text, 4).unwrap_or_default();
        if !self.applications.is_empty()
            && !self.applications.iter().any(|a| match a.split_once('^') {
                Some((x, f)) => x == app && f == facility,
                None => *a == app,
            })
        {
            return Some(WRONG_APPLICATION);
        }
        let version = msh_first_component(text, 12).unwrap_or_default();
        if !self.versions.is_empty() && !self.versions.contains(&version) {
            return Some(WRONG_VERSION);
        }
        if !self.events.is_empty() {
            let msh9 = text
                .split(['\r', '\n'])
                .find(|l| l.starts_with("MSH"))
                .and_then(|m| {
                    m.chars()
                        .nth(3)
                        .and_then(|sep| m.split(sep).nth(8).map(str::to_string))
                })
                .unwrap_or_default();
            let event = msh9.split('^').nth(1).unwrap_or_default();
            if !self.events.iter().any(|e| e == event) {
                return Some(WRONG_EVENT);
            }
        }
        None
    }
}

/// Why a message for another kind of deployment is refused (IH-2): a test message is never applied to a live record, nor
/// a live one to a test system.
pub const WRONG_PROCESSING_ID: &str = "MSH-11 (processing id) is not the one this deployment takes: a person must check where this message was meant to go";

/// MSH-11's first character, read without a full parse, as [`message_ids`] reads MSH-9 and MSH-10.
fn processing_id_of(text: &str) -> Option<char> {
    let msh = text.split(['\r', '\n']).find(|l| l.starts_with("MSH"))?;
    let sep = msh.chars().nth(3)?;
    msh.split(sep).nth(10)?.chars().next()
}

/// How a refusal that was kept in the work queue is answered (IH-1, `EPR_MLLP_KEPT_ANSWER`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KeptAnswer {
    /// `AR`: kept for a person, do not send it again (the default).
    #[default]
    Reject,
    /// `AE`: for a sender that must be told `AE` for a refused message.
    Error,
}

/// The words `MSA-3` starts with when a refusal was kept for a person.
pub const KEPT_NOTE: &str = "kept for review by a person; do not resend";

/// The fingerprint domain for a sender's control id ([`control_digest`]).
pub const CONTROL_ID_DOMAIN: &str = "feed-control-id";

/// A sender's control id (MSH-10) as it is kept anywhere but the stored message text: a keyed fingerprint of the
/// agent and the id (IH-3). Some senders build MSH-10 from a patient's number, so the plain value goes in no audit
/// row, queue column, version reason or log line. The same sender and id give the same fingerprint, so a resend still
/// matches. An id that could not be read (`?`) stays `?`.
#[must_use]
pub fn control_digest(agent: &str, control_id: &str) -> String {
    if control_id == "?" {
        return control_id.to_string();
    }
    audit::chain().fingerprint(CONTROL_ID_DOMAIN, &format!("{agent}\0{control_id}"))
}

/// What a message did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Applied {
    /// A new patient was created.
    Created {
        /// Its id.
        id: String,
    },
    /// An existing patient was changed (a new version).
    Updated {
        /// Its id.
        id: String,
        /// The new version.
        version: i32,
    },
    /// The message matched a patient and held nothing new.
    Unchanged {
        /// Its id.
        id: String,
    },
}

/// Why a message was not applied. Always acknowledged `AE` (the sender fixes it or a person does),
/// except [`Refusal::Unsupported`], which is `AR`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// The message type is not handled, or the text is not HL7.
    Unsupported(String),
    /// The message is unacceptable or conflicts with the record; the reasons.
    Conflict(Vec<String>),
    /// A transient failure; the sender should retry.
    Temporary(String),
}

/// The identifier systems that carry a sender's own numbers.
fn is_local(i: &Identifier) -> bool {
    i.system.starts_with("urn:hl7v2:")
}

/// Combine what the record holds with what a message carries, for an update. Returns `Ok(None)` when
/// the message adds nothing, so no version is written for a repeat.
///
/// Fields the message lacks are kept. Names: the message's first name replaces the record's first,
/// and its other names are added if new. A different birth date or national health number is a conflict.
///
/// # Errors
///
/// The reasons the message cannot be applied.
pub fn merge_update(
    existing: &Patient,
    incoming: &Patient,
) -> Result<Option<Patient>, Vec<String>> {
    let mut conflicts = Vec::new();
    if let (Some(a), Some(b)) = (&existing.birth_date, &incoming.birth_date)
        && a != b
    {
        conflicts.push(
            "the birth date differs from the record: a person must correct it, with a reason"
                .to_string(),
        );
    }
    let national = |p: &Patient| {
        p.identifiers
            .iter()
            .find(|i| i.system == NATIONAL_NUMBER_SYSTEM)
            .map(|i| i.value.clone())
    };
    if let (Some(a), Some(b)) = (national(existing), national(incoming))
        && a != b
    {
        conflicts.push(
            "the national health number differs from the record: a person must correct it, with a reason"
                .to_string(),
        );
    }
    if !conflicts.is_empty() {
        return Err(conflicts);
    }
    let mut out = existing.clone();
    if out.birth_date.is_none() {
        out.birth_date.clone_from(&incoming.birth_date);
    }
    if incoming.sex.is_some() {
        out.sex = incoming.sex;
    }
    for (n, name) in incoming.names.iter().enumerate() {
        if n == 0 && !out.names.is_empty() {
            out.names[0] = name.clone();
        } else if !out.names.contains(name) {
            out.names.push(name.clone());
        }
    }
    for id in &incoming.identifiers {
        match out.identifiers.iter_mut().find(|i| i.system == id.system) {
            Some(have) => have.value.clone_from(&id.value),
            None => out.identifiers.push(id.clone()),
        }
    }
    for a in &incoming.addresses {
        if !out.addresses.contains(a) {
            out.addresses.push(a.clone());
        }
    }
    for t in &incoming.telecom {
        if !out.telecom.contains(t) {
            out.telecom.push(t.clone());
        }
    }
    Ok((out != *existing).then_some(out))
}

/// The subject the feed acts as: the integrator role only, so it can do exactly what the policy table gives that role.
fn feed_subject() -> Subject {
    Subject {
        roles: std::collections::BTreeSet::from([Role::Integrator]),
        ..Subject::default()
    }
}

/// Whether the policy engine lets the feed's role do `action` to `resource`.
fn permitted(resource: Resource, action: policy::Action) -> bool {
    policy::evaluate(&policy::Request {
        subject: feed_subject(),
        resource,
        action,
        labels: std::collections::BTreeSet::new(),
        relationship: Relationship::None,
        purpose: Purpose::Operations,
        consent: Consent::NotRecorded,
        break_glass: None,
        now: 0,
    })
    .allowed()
}

/// A store failure as a refusal the sender should retry (`Temporary`); the real error is logged and never sent.
fn temporary(e: &StoreError) -> Refusal {
    tracing::error!(error = %e, "an ADT message could not be applied");
    Refusal::Temporary("temporary failure, please retry".into())
}

/// Why an update older than one already applied is refused (IH-5). Time-free, like every reason: it reaches `MSA-3`.
pub const OUT_OF_ORDER: &str = "this update is older than one this feed has already applied for this patient: a person must check which is right";

/// Whether this feed has applied a message for `patient_id` that is newer than `at`, read under a transaction-scoped
/// lock on the feed and patient, so two connections cannot both pass.
async fn newer_applied(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    agent: &str,
    patient_id: &str,
    at: jiff::Timestamp,
) -> Result<bool, Refusal> {
    let db_err = |e: sqlx::Error| temporary(&StoreError::Db(e.to_string()));
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended('feed-order:' || $1 || ':' || $2, 0))",
    )
    .bind(agent)
    .bind(patient_id)
    .execute(&mut **tx)
    .await
    .map_err(db_err)?;
    let newer: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM feed_message_times
                         WHERE agent = $1 AND patient_id = $2::uuid AND message_time > $3)",
    )
    .bind(agent)
    .bind(patient_id)
    .bind(crate::pg_time::PgTimestamp(at))
    .fetch_one(&mut **tx)
    .await
    .map_err(db_err)?;
    Ok(newer)
}

/// Record the time of a message the feed applied, in the same transaction as the change (IH-5).
async fn record_time(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    agent: &str,
    patient_id: &str,
    at: jiff::Timestamp,
    control_id: &str,
) -> Result<(), Refusal> {
    sqlx::query(
        "INSERT INTO feed_message_times (agent, patient_id, message_time, control_id) VALUES ($1, $2::uuid, $3, $4)",
    )
    .bind(agent)
    .bind(patient_id)
    .bind(crate::pg_time::PgTimestamp(at))
    .bind(control_id)
    .execute(&mut **tx)
    .await
    .map(|_| ())
    .map_err(|e| temporary(&StoreError::Db(e.to_string())))
}

/// Find the one patient a message is about, by the identifiers it carries.
async fn find(db: &PgPool, patient: &Patient) -> Result<Option<store::Stored>, Refusal> {
    // The sender's own numbers first, then a national number.
    let locals = patient.identifiers.iter().filter(|i| is_local(i));
    let national = patient
        .identifiers
        .iter()
        .filter(|i| i.system == NATIONAL_NUMBER_SYSTEM);
    for id in locals.chain(national) {
        let found = store::search(
            db,
            &store::Search {
                identifier: Some(format!("{}|{}", id.system, id.value)),
                ..store::Search::default()
            },
        )
        .await
        .map_err(|e| temporary(&e))?;
        match found.len() {
            0 => {}
            1 => return Ok(found.into_iter().next()),
            n => {
                return Err(Refusal::Conflict(vec![format!(
                    "{n} patients carry the identifier {}: a person must sort this out",
                    id.system
                )]));
            }
        }
    }
    Ok(None)
}

/// What a message will do, decided before the transaction starts.
enum Plan {
    Nothing {
        id: String,
    },
    Update {
        id: String,
        version: i32,
        next: Patient,
    },
    Create,
}

/// Both give a date, the same one, or neither does.
fn same_date(a: &Occurred, b: &Occurred) -> bool {
    match (a, b) {
        (Occurred::Known { at: x }, Occurred::Known { at: y }) => x == y,
        (Occurred::NotKnown { .. }, Occurred::NotKnown { .. }) => true,
        _ => false,
    }
}

/// The report a message's death should store: `None` when this feed has already said the same thing
/// and it stands. A date that cannot be stored becomes a date not known, because the report still holds
/// contact.
///
/// # Errors
///
/// A [`Refusal`] when a person withdrew this same report as an error, or the store failed.
async fn death_report(
    db: &PgPool,
    patient_id: &str,
    occurred: &Occurred,
    reason: &str,
    who: &FeedIdentity,
) -> Result<Option<Assertion>, Refusal> {
    if !permitted(Resource::DeathAssertion, policy::Action::Create) {
        return Err(Refusal::Conflict(vec![
            "the feed's role may not do this".into(),
        ]));
    }
    let feed = Actor::Feed(who.agent.clone());
    let now = jiff::Timestamp::now();
    let report = |occurred: Occurred| {
        death::report(Source::OtherSystem, feed.clone(), occurred, reason, now)
    };
    let assertion = report(occurred.clone())
        .or_else(|_| {
            report(Occurred::NotKnown {
                reason: hl7_adt::UNREADABLE_DEATH_DATE.into(),
            })
        })
        .map_err(|_| Refusal::Temporary("temporary failure, please retry".into()))?;
    let earlier: Vec<_> = death_store::list(db, patient_id)
        .await
        .map_err(|e| temporary(&e))?
        .into_iter()
        .map(|s| s.assertion)
        .filter(|a| a.reported_by == feed && same_date(&a.occurred, &assertion.occurred))
        .collect();
    if earlier.iter().any(|a| a.status != Status::WithdrawnInError) {
        return Ok(None);
    }
    if earlier.is_empty() {
        Ok(Some(assertion))
    } else {
        Err(Refusal::Conflict(vec![
            "this message repeats a report of death that a person withdrew as an error: a person must look at it".into(),
        ]))
    }
}

/// Apply a read message and record any death it reports. A message that cannot be applied says that
/// its death was not recorded, so that a person reading the queue knows.
///
/// # Errors
///
/// A [`Refusal`]; nothing was written (a refusal is audited separately by the caller).
pub async fn apply(db: &PgPool, inbound: &Inbound, who: &FeedIdentity) -> Result<Applied, Refusal> {
    apply_in(db, inbound, who, Mode::Apply).await
}

/// Whether a message's changes are kept (IH-11).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// The feed: the change and its audit rows commit together.
    Apply,
    /// A dry run: everything runs in the transaction, which is then rolled back.
    DryRun,
}

async fn apply_in(
    db: &PgPool,
    inbound: &Inbound,
    who: &FeedIdentity,
    mode: Mode,
) -> Result<Applied, Refusal> {
    let mut result = apply_message(db, inbound, who, mode).await;
    if inbound.death.is_some()
        && let Err(Refusal::Conflict(reasons)) = &mut result
    {
        reasons.push(hl7_adt::DEATH_NOT_APPLIED.to_string());
    }
    result
}

/// Apply a read message: create, update or leave alone, and store a report of death if it holds a
/// new one. Everything and its audit rows commit together.
async fn apply_message(
    db: &PgPool,
    inbound: &Inbound,
    who: &FeedIdentity,
    mode: Mode,
) -> Result<Applied, Refusal> {
    let existing = find(db, &inbound.patient).await?;
    let action = if existing.is_some() {
        policy::Action::Update
    } else {
        policy::Action::Create
    };
    if !permitted(Resource::PatientIdentity, action) {
        return Err(Refusal::Conflict(vec![
            "the feed's role may not do this".into(),
        ]));
    }
    // Everything that reads through the pool is done before the transaction starts: the
    // transaction holds a connection, and asking the pool for a second one while holding it waits
    // forever on a pool of one (the development default).
    let plan = match existing {
        Some(current) => {
            match merge_update(&current.patient, &inbound.patient).map_err(Refusal::Conflict)? {
                None => Plan::Nothing { id: current.id },
                Some(next) => {
                    let problems = store::validate(&next);
                    if !problems.is_empty() {
                        return Err(Refusal::Conflict(problems));
                    }
                    Plan::Update {
                        id: current.id,
                        version: current.version,
                        next,
                    }
                }
            }
        }
        // A new patient the matcher thinks may already exist waits for a person; that check runs in
        // the transaction, under the registration lock.
        None => Plan::Create,
    };
    let reason = format!("HL7 ADT {} {}", inbound.event, inbound.control_id);
    // A death is a report about somebody on record: a message about nobody waits for a person.
    let death = match (&inbound.death, &plan) {
        (None, _) => None,
        (Some(_), Plan::Create) => {
            return Err(Refusal::Conflict(vec![
                "no patient on record matches this message".into(),
            ]));
        }
        (Some(occurred), Plan::Nothing { id } | Plan::Update { id, .. }) => {
            death_report(db, id, occurred, &reason, who).await?
        }
    };
    let mut tx = db
        .begin()
        .await
        .map_err(|e| temporary(&StoreError::Db(e.to_string())))?;
    // A late message never undoes a newer one (IH-5): under a lock per feed and patient, an update older than the
    // newest one this feed applied for the patient waits for a person.
    if let (Plan::Update { id, .. }, Some(at)) = (&plan, inbound.message_time)
        && newer_applied(&mut tx, &who.agent, id, at).await?
    {
        return Err(Refusal::Conflict(vec![OUT_OF_ORDER.to_string()]));
    }
    // Whether the duplicate check saw only the strongest CANDIDATE_LIMIT of more matches (CQ-36).
    let mut candidates_cut = false;
    let (applied, audit_action) = match plan {
        Plan::Nothing { id } => (Applied::Unchanged { id }, Action::Read),
        Plan::Update { id, version, next } => {
            match store::update(&mut *tx, &id, version, &who.agent, Some(&reason), &next).await {
                Ok(s) => (
                    Applied::Updated {
                        id: s.id,
                        version: s.version,
                    },
                    Action::Update,
                ),
                Err(StoreError::Conflict { .. }) => {
                    return Err(Refusal::Temporary(
                        "the record changed meanwhile, please retry".into(),
                    ));
                }
                Err(e) => return Err(temporary(&e)),
            }
        }
        Plan::Create => {
            // Under the registration lock (CQ-7), so another registration of this person, by a
            // person or a second feed connection, cannot pass the same check at the same time.
            store::lock_registration(&mut *tx)
                .await
                .map_err(|e| temporary(&e))?;
            let (pool, cut) = store::candidates_ranked(&mut *tx, &inbound.patient)
                .await
                .map_err(|e| temporary(&e))?;
            candidates_cut = cut;
            let likely = likely_duplicates(&inbound.patient, pool);
            if !likely.is_empty() {
                return Err(Refusal::Conflict(vec![format!(
                    "{} existing patient(s) may be the same person: a person must decide whether to link or register",
                    likely.len()
                )]));
            }
            match store::create(&mut *tx, &who.agent, Some(&reason), &inbound.patient).await {
                Ok(s) => (Applied::Created { id: s.id }, Action::Create),
                Err(e) => return Err(temporary(&e)),
            }
        }
    };
    let id = match &applied {
        Applied::Created { id } | Applied::Updated { id, .. } | Applied::Unchanged { id } => {
            id.clone()
        }
    };
    if let Some(assertion) = &death {
        let stored = death_store::report(&mut *tx, &id, assertion)
            .await
            .map_err(|e| temporary(&e))?;
        let mut row = Event::new(Action::Create, who.agent.clone());
        row.patient_ref = Some(format!("Patient/{id}"));
        row.resource_ref = Some("DeathAssertion".into());
        row.purpose_of_use = Some(Purpose::Operations.code().into());
        row.network_address.clone_from(&who.peer);
        row.detail = Some(who.detail(serde_json::json!({
            "assertion": stored.id,
            "version": stored.version,
            "status": stored.assertion.status.word(),
            "control_id": inbound.control_id,
        })));
        audit::append(&mut *tx, audit::chain(), &row)
            .await
            .map_err(|e| temporary(&StoreError::Db(e.to_string())))?;
    }
    // The message's time, for the next message's order check (IH-5). A message with no usable time is applied, as
    // before, and its audit row says so.
    if let Some(at) = inbound.message_time {
        record_time(&mut tx, &who.agent, &id, at, &inbound.control_id).await?;
    }
    let mut event = Event::new(audit_action, who.agent.clone());
    event.patient_ref = Some(format!("Patient/{id}"));
    event.resource_ref = Some("Patient/hl7".into());
    event.purpose_of_use = Some(Purpose::Operations.code().into());
    event.network_address.clone_from(&who.peer);
    event.detail = Some(who.detail(serde_json::json!({
        "hl7_event": inbound.event,
        "control_id": inbound.control_id,
        "result": match &applied {
            Applied::Created { .. } => "created",
            Applied::Updated { .. } => "updated",
            Applied::Unchanged { .. } => "unchanged",
        },
        "warnings": inbound.warnings.len(),
        "candidates_cut": candidates_cut,
        "message_time_known": inbound.message_time.is_some(),
    })));
    audit::append(&mut *tx, audit::chain(), &event)
        .await
        .map_err(|e| temporary(&StoreError::Db(e.to_string())))?;
    match mode {
        Mode::Apply => tx.commit().await,
        // A dry run keeps nothing: the change, the death report, the message time and the audit rows all go (IH-11).
        Mode::DryRun => tx.rollback().await,
    }
    .map_err(|e| temporary(&StoreError::Db(e.to_string())))?;
    Ok(applied)
}

/// What a message would do, from [`dry_run`] (IH-11).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct DryRun {
    /// `created`, `updated`, `unchanged`, `refused` (it would be kept for a person), `retry` (a temporary failure:
    /// the feed would answer `AE`) or `not-hl7`.
    pub would: &'static str,
    /// Why, when refused; never a message value.
    pub reasons: Vec<String>,
    /// How many warnings the reader had (their words are not shown: they name fields of the message).
    pub warnings: usize,
    /// The acknowledgement the feed would send, or `None` for text with no sender to answer.
    pub acknowledgement: Option<String>,
}

/// Run a message through the feed's real path, with `who`'s rules, and keep nothing (IH-11): the reading, the rules,
/// matching, the duplicate check, the order check and the writes all run, in a transaction that is rolled back. A
/// refusal is not stored in the work queue, nothing is counted, and the only trace is the caller's audit row.
pub async fn dry_run(db: &PgPool, text: &str, who: &FeedIdentity) -> DryRun {
    let read = hl7_adt::patient_from_adt(text).map(|mut inbound| {
        inbound.control_id = control_digest(&who.agent, &inbound.control_id);
        inbound
    });
    let not_for_here = if who
        .processing_id
        .is_some_and(|want| processing_id_of(text) != Some(want))
    {
        Some(WRONG_PROCESSING_ID)
    } else {
        who.rules.broken_by(text)
    };
    let warnings = read.as_ref().map_or(0, |i| i.warnings.len());
    let (would, reasons, outcome): (&'static str, Vec<String>, Result<Inbound, Rejection>) =
        match &read {
            Err(Rejection::NotHl7(m)) => (
                "not-hl7",
                vec![m.clone()],
                Err(Rejection::NotHl7(m.clone())),
            ),
            Err(r) => {
                let reasons = match r {
                    Rejection::Unsupported(m) => vec![m.clone()],
                    Rejection::Invalid(m) | Rejection::Kept(m) => m.clone(),
                    Rejection::NotHl7(_) => Vec::new(),
                };
                ("refused", reasons, Err(answer_for(r.clone(), true, who)))
            }
            Ok(_) if not_for_here.is_some() => {
                let why = not_for_here.unwrap_or(WRONG_PROCESSING_ID).to_string();
                (
                    "refused",
                    vec![why.clone()],
                    Err(answer_for(Rejection::Unsupported(why), true, who)),
                )
            }
            Ok(inbound) => match apply_in(db, inbound, who, Mode::DryRun).await {
                Ok(Applied::Created { .. }) => ("created", Vec::new(), Ok(inbound.clone())),
                Ok(Applied::Updated { .. }) => ("updated", Vec::new(), Ok(inbound.clone())),
                Ok(Applied::Unchanged { .. }) => ("unchanged", Vec::new(), Ok(inbound.clone())),
                Err(Refusal::Temporary(m)) => {
                    ("retry", vec![m.clone()], Err(Rejection::Invalid(vec![m])))
                }
                Err(Refusal::Unsupported(m)) => (
                    "refused",
                    vec![m.clone()],
                    Err(answer_for(Rejection::Unsupported(m), true, who)),
                ),
                Err(Refusal::Conflict(m)) => (
                    "refused",
                    m.clone(),
                    Err(answer_for(Rejection::Invalid(m), true, who)),
                ),
            },
        };
    DryRun {
        would,
        reasons,
        warnings,
        acknowledgement: hl7_adt::acknowledgement(text, &outcome, "DRYRUN", "00000000000000"),
    }
}

/// The identity of the feed the service listens for, set when the listener starts; a dry run uses it, so it applies
/// the same rules (IH-11).
static CONFIGURED: std::sync::OnceLock<FeedIdentity> = std::sync::OnceLock::new();

/// Record the listening feed's identity (no peer). Set once.
pub fn set_configured(who: FeedIdentity) {
    let _ = CONFIGURED.set(who);
}

/// The listening feed's identity, if the listener is on.
#[must_use]
pub fn configured() -> Option<&'static FeedIdentity> {
    CONFIGURED.get()
}

/// The trigger event (MSH-9.2) and control id (MSH-10) of a message, cleaned for the audit row, even
/// when the message was refused before it was fully read. `"?"` where there is nothing usable.
fn message_ids(text: &str) -> (String, String) {
    let clean = |s: &str| {
        let s: String = s.chars().filter(|c| !c.is_control()).take(64).collect();
        if s.is_empty() { "?".to_string() } else { s }
    };
    let Some(msh) = text.split('\r').find(|l| l.starts_with("MSH")) else {
        return ("?".into(), "?".into());
    };
    let Some(sep) = msh.chars().nth(3) else {
        return ("?".into(), "?".into());
    };
    let fields: Vec<&str> = msh.split(sep).collect();
    let event = fields
        .get(8)
        .and_then(|t| t.split('^').nth(1))
        .map_or_else(|| "?".to_string(), clean);
    let control = fields.get(9).map_or_else(|| "?".to_string(), |c| clean(c));
    (event, control)
}

/// Write the audit row for a message that was refused (nothing else was written), and keep the message in the work
/// queue unless the failure was temporary. Returns whether the message is in the queue (stored now, or already
/// waiting from an earlier send). Failure is logged.
async fn audit_refusal(
    db: &PgPool,
    who: &FeedIdentity,
    event: &str,
    control_id: &str,
    why: &Refusal,
    text: &str,
) -> bool {
    let mut row = Event::new(Action::Execute, who.agent.clone());
    row.outcome = 4;
    row.resource_ref = Some("Patient/hl7".into());
    row.purpose_of_use = Some(Purpose::Operations.code().into());
    row.network_address.clone_from(&who.peer);
    row.detail = Some(who.detail(serde_json::json!({
        "hl7_event": event,
        "control_id": control_id,
        "refused": match why {
            Refusal::Unsupported(_) => "unsupported",
            Refusal::Conflict(_) => "conflict",
            Refusal::Temporary(_) => "temporary",
        },
    })));
    if let Err(e) = audit::append(db, audit::chain(), &row).await {
        tracing::error!(error = %e, "an ADT refusal could not be audited");
    }
    // Keep the message for a person (the work queue). A transient failure is not queued: the
    // sender retries and nothing is wrong with the message. A failure here is logged; the
    // acknowledgement still says the message was refused.
    let (kind, reasons) = match why {
        Refusal::Unsupported(m) => (feed_queue::Kind::Unsupported, vec![m.clone()]),
        Refusal::Conflict(m) => (feed_queue::Kind::Conflict, m.clone()),
        Refusal::Temporary(_) => return false,
    };
    match feed_queue::record(
        db,
        feed_queue::New {
            agent: &who.agent,
            peer: who.peer.as_deref(),
            event,
            control_id,
            kind,
            reasons: &reasons,
            message: text,
        },
    )
    .await
    {
        Ok(_) => true,
        Err(e) => {
            tracing::error!(error = %e, "a refused ADT message could not be queued");
            false
        }
    }
}

/// The answer to a refusal: a kept one is `AR` (or `AE` by setting) with [`KEPT_NOTE`] first; one that was not kept
/// stays as the reader said (`AE`, so the sender tries again, or `AR` for a type not handled).
fn answer_for(rejection: Rejection, kept: bool, who: &FeedIdentity) -> Rejection {
    if !kept {
        return rejection;
    }
    // A type not handled is `AR` whatever the setting: no resend can change it.
    let (reasons, always_reject) = match rejection {
        Rejection::Unsupported(m) => (vec![m], true),
        Rejection::Invalid(m) | Rejection::Kept(m) => (m, false),
        other @ Rejection::NotHl7(_) => return other,
    };
    let note = std::iter::once(KEPT_NOTE.to_string())
        .chain(reasons)
        .collect::<Vec<_>>();
    if always_reject || who.kept_answer == KeptAnswer::Reject {
        Rejection::Kept(note)
    } else {
        Rejection::Invalid(note)
    }
}

/// The whole path for one message text: read, apply, audit, and build the acknowledgement. Returns
/// `None` when the text is not HL7 at all (nothing to answer). `ack_id` and `timestamp` are the
/// acknowledgement's own control id and `YYYYMMDDHHMMSS` time.
pub async fn process(
    db: &PgPool,
    text: &str,
    who: &FeedIdentity,
    ack_id: &str,
    timestamp: &str,
) -> Option<String> {
    // The control id is kept only as a fingerprint from here on (IH-3); the acknowledgement still echoes the sender's
    // own MSH-10, read from the text.
    let read = hl7_adt::patient_from_adt(text).map(|mut inbound| {
        inbound.control_id = control_digest(&who.agent, &inbound.control_id);
        inbound
    });
    let not_for_here = if who
        .processing_id
        .is_some_and(|want| processing_id_of(text) != Some(want))
    {
        Some(WRONG_PROCESSING_ID)
    } else {
        who.rules.broken_by(text)
    };
    let outcome: Result<Inbound, Rejection> = match &read {
        Ok(inbound) if not_for_here.is_some() => {
            let why = not_for_here.unwrap_or(WRONG_PROCESSING_ID).to_string();
            let refusal = Refusal::Unsupported(why.clone());
            let kept =
                audit_refusal(db, who, &inbound.event, &inbound.control_id, &refusal, text).await;
            Err(answer_for(Rejection::Unsupported(why), kept, who))
        }
        Ok(inbound) => match apply(db, inbound, who).await {
            Ok(applied) => {
                tracing::info!(event = %inbound.event, control_id = %inbound.control_id, ?applied, "ADT message applied");
                Ok(inbound.clone())
            }
            Err(refusal) => {
                let kept =
                    audit_refusal(db, who, &inbound.event, &inbound.control_id, &refusal, text)
                        .await;
                let rejection = match refusal {
                    Refusal::Unsupported(m) => Rejection::Unsupported(m),
                    Refusal::Conflict(m) => Rejection::Invalid(m),
                    Refusal::Temporary(m) => Rejection::Invalid(vec![m]),
                };
                Err(answer_for(rejection, kept, who))
            }
        },
        Err(rejection) => {
            if !matches!(rejection, Rejection::NotHl7(_)) {
                let why = match rejection {
                    Rejection::Unsupported(m) => Refusal::Unsupported(m.clone()),
                    Rejection::Invalid(m) | Rejection::Kept(m) => Refusal::Conflict(m.clone()),
                    Rejection::NotHl7(m) => Refusal::Conflict(vec![m.clone()]),
                };
                let (event, control_id) = message_ids(text);
                let control_id = control_digest(&who.agent, &control_id);
                let kept = audit_refusal(db, who, &event, &control_id, &why, text).await;
                let ack = hl7_adt::acknowledgement(
                    text,
                    &Err(answer_for(rejection.clone(), kept, who)),
                    ack_id,
                    timestamp,
                );
                count(who, text, ack.as_deref());
                return ack;
            }
            Err(rejection.clone())
        }
    };
    let ack = hl7_adt::acknowledgement(text, &outcome, ack_id, timestamp);
    count(who, text, ack.as_deref());
    ack
}

/// Count a message by its feed, its event and the answer it got (IH-9).
fn count(who: &FeedIdentity, text: &str, ack: Option<&str>) {
    let (event, _) = message_ids(text);
    let answer = ack
        .and_then(|a| a.split('\r').find(|l| l.starts_with("MSA|")))
        .and_then(|msa| msa.split('|').nth(1))
        .unwrap_or("none");
    crate::metrics::feed_message(&who.agent, &event, hl7_adt::HANDLED_EVENTS, answer);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed(kept_answer: KeptAnswer) -> FeedIdentity {
        FeedIdentity {
            agent: "mllp:test".into(),
            peer: None,
            peer_certificate: None,
            kept_answer,
            processing_id: None,
            rules: SenderRules::default(),
        }
    }

    #[test]
    fn sender_rules_take_only_what_is_listed_and_an_empty_list_takes_anything() {
        // IH-6. The sample is from REG at CLINIC, HL7 2.5, an A08.
        assert_eq!(SenderRules::default().broken_by(SAMPLE), None);
        let rules = |a: &[&str], v: &[&str], e: &[&str]| SenderRules {
            applications: a.iter().map(|x| (*x).to_string()).collect(),
            versions: v.iter().map(|x| (*x).to_string()).collect(),
            events: e.iter().map(|x| (*x).to_string()).collect(),
        };
        assert_eq!(rules(&["REG"], &["2.5"], &["A08"]).broken_by(SAMPLE), None);
        assert_eq!(rules(&["REG^CLINIC"], &[], &[]).broken_by(SAMPLE), None);
        assert_eq!(
            rules(&["REG^ELSEWHERE"], &[], &[]).broken_by(SAMPLE),
            Some(WRONG_APPLICATION)
        );
        assert_eq!(
            rules(&["OTHER"], &[], &[]).broken_by(SAMPLE),
            Some(WRONG_APPLICATION)
        );
        assert_eq!(
            rules(&[], &["2.5.1"], &[]).broken_by(SAMPLE),
            Some(WRONG_VERSION)
        );
        assert_eq!(
            rules(&[], &[], &["A28", "A31"]).broken_by(SAMPLE),
            Some(WRONG_EVENT)
        );
        // A component after the first does not matter: `REG^1.2.3^ISO` is still REG.
        let with_oid = SAMPLE.replace("MSH|^~\\&|REG|", "MSH|^~\\&|REG^1.2.3^ISO|");
        assert_eq!(rules(&["REG"], &[], &[]).broken_by(&with_oid), None);
        for reason in [WRONG_APPLICATION, WRONG_VERSION, WRONG_EVENT] {
            assert!(
                !reason.contains("REG") && !reason.contains("2.5"),
                "{reason}"
            );
        }
    }

    #[test]
    fn a_control_id_is_kept_as_a_fingerprint_bound_to_its_sender() {
        // IH-3: never the plain value; the same sender and id match; another sender's same id does not; `?` stays.
        let a = control_digest("mllp:one", "20261117-9434765919");
        assert_eq!(a, control_digest("mllp:one", "20261117-9434765919"));
        assert!(!a.contains("9434765919"), "{a}");
        assert_ne!(a, control_digest("mllp:two", "20261117-9434765919"));
        assert_eq!(control_digest("mllp:one", "?"), "?");
    }

    #[test]
    fn a_kept_refusal_is_ar_with_the_note_first_and_one_not_kept_is_unchanged() {
        // IH-1. Kept: AR by default, the note first, then the reasons.
        let invalid = Rejection::Invalid(vec!["a different birth date".into()]);
        assert_eq!(
            answer_for(invalid.clone(), true, &feed(KeptAnswer::Reject)),
            Rejection::Kept(vec![KEPT_NOTE.into(), "a different birth date".into()])
        );
        // AE by setting, with the same words.
        assert_eq!(
            answer_for(invalid.clone(), true, &feed(KeptAnswer::Error)),
            Rejection::Invalid(vec![KEPT_NOTE.into(), "a different birth date".into()])
        );
        // A type not handled stays AR whatever the setting.
        let unsupported = Rejection::Unsupported("ORU^R01 is not handled".into());
        assert!(matches!(
            answer_for(unsupported.clone(), true, &feed(KeptAnswer::Error)),
            Rejection::Kept(_)
        ));
        // Not kept (the database unavailable, the queue write failed): as the reader said, so AE invites a resend.
        assert_eq!(
            answer_for(invalid.clone(), false, &feed(KeptAnswer::Reject)),
            invalid
        );
        assert_eq!(
            answer_for(unsupported.clone(), false, &feed(KeptAnswer::Reject)),
            unsupported
        );
    }
    use electronic_patient_record_matcher::{AdministrativeSex, HumanName, Telecom};

    fn person() -> Patient {
        Patient {
            death_date: None,
            temporary_identity: false,
            id: Some("x".into()),
            names: vec![
                HumanName {
                    family: Some("Smith".into()),
                    given: vec!["Ann".into()],
                },
                HumanName {
                    family: Some("Jones".into()),
                    given: vec![],
                },
            ],
            birth_date: Some("1980-03-14".into()),
            sex: Some(AdministrativeSex::Female),
            identifiers: vec![
                Identifier {
                    system: NATIONAL_NUMBER_SYSTEM.into(),
                    value: "9434765919".into(),
                },
                Identifier {
                    system: "urn:hl7v2:LOCAL".into(),
                    value: "M-1".into(),
                },
            ],
            addresses: vec![],
            telecom: vec![Telecom {
                system: "phone".into(),
                value: "01632 960001".into(),
            }],
        }
    }

    #[test]
    fn a_message_never_ends_a_temporary_identity() {
        let existing = Patient {
            temporary_identity: true,
            ..person()
        };
        let incoming = Patient {
            names: vec![HumanName {
                family: Some("Smyth".into()),
                given: vec!["Ann".into()],
            }],
            ..person()
        };
        let next = merge_update(&existing, &incoming).unwrap().unwrap();
        assert!(next.temporary_identity);
    }

    #[test]
    fn a_repeat_of_what_is_held_changes_nothing() {
        let p = person();
        assert_eq!(merge_update(&p, &p).unwrap(), None);
    }

    #[test]
    fn fields_the_message_lacks_are_kept_and_new_ones_added() {
        let existing = person();
        let incoming = Patient {
            names: vec![HumanName {
                family: Some("Smyth".into()),
                given: vec!["Ann".into()],
            }],
            telecom: vec![Telecom {
                system: "email".into(),
                value: "ann@example.org".into(),
            }],
            identifiers: vec![Identifier {
                system: "urn:hl7v2:OTHER".into(),
                value: "77".into(),
            }],
            ..Patient::default()
        };
        let out = merge_update(&existing, &incoming).unwrap().unwrap();
        assert_eq!(
            out.names[0].family.as_deref(),
            Some("Smyth"),
            "the first name is corrected"
        );
        assert_eq!(
            out.names[1].family.as_deref(),
            Some("Jones"),
            "the other names stay"
        );
        assert_eq!(out.birth_date, existing.birth_date);
        assert_eq!(out.sex, existing.sex);
        assert_eq!(out.telecom.len(), 2, "the phone stays, the email is added");
        assert_eq!(out.identifiers.len(), 3);
        assert_eq!(out.id, existing.id);
    }

    #[test]
    fn an_identifier_of_a_known_system_takes_the_new_value() {
        let incoming = Patient {
            identifiers: vec![Identifier {
                system: "urn:hl7v2:LOCAL".into(),
                value: "M-2".into(),
            }],
            ..Patient::default()
        };
        let out = merge_update(&person(), &incoming).unwrap().unwrap();
        let local: Vec<_> = out
            .identifiers
            .iter()
            .filter(|i| i.system == "urn:hl7v2:LOCAL")
            .collect();
        assert_eq!(local.len(), 1);
        assert_eq!(local[0].value, "M-2");
    }

    #[test]
    fn a_different_birth_date_or_national_number_is_never_applied() {
        let dob = Patient {
            birth_date: Some("1981-03-14".into()),
            ..Patient::default()
        };
        let e = merge_update(&person(), &dob).unwrap_err();
        assert!(e[0].contains("birth date"), "{e:?}");
        let with_number = Patient {
            identifiers: vec![Identifier {
                system: NATIONAL_NUMBER_SYSTEM.into(),
                value: "4010232137".into(),
            }],
            ..Patient::default()
        };
        let e = merge_update(&person(), &with_number).unwrap_err();
        assert!(e[0].contains("national health number"), "{e:?}");
        // Both at once: both are reported.
        let both = Patient {
            birth_date: Some("1999-01-01".into()),
            ..with_number
        };
        assert_eq!(merge_update(&person(), &both).unwrap_err().len(), 2);
    }

    #[test]
    fn a_missing_birth_date_on_the_record_is_filled_in_but_not_changed_later() {
        let existing = Patient {
            birth_date: None,
            ..person()
        };
        let incoming = Patient {
            birth_date: Some("1980-03-14".into()),
            ..Patient::default()
        };
        assert_eq!(
            merge_update(&existing, &incoming)
                .unwrap()
                .unwrap()
                .birth_date
                .as_deref(),
            Some("1980-03-14")
        );
    }

    #[test]
    fn the_feeds_role_may_create_and_update_identity_but_nothing_else_is_asked_of_it() {
        let identity = Resource::PatientIdentity;
        assert!(permitted(identity, policy::Action::Create));
        assert!(permitted(identity, policy::Action::Update));
        assert!(!permitted(identity, policy::Action::Read));
        assert!(!permitted(identity, policy::Action::Export));
        // A feed can report a death and do nothing else with one.
        let death = Resource::DeathAssertion;
        assert!(permitted(death, policy::Action::Create));
        for action in [
            policy::Action::Read,
            policy::Action::Search,
            policy::Action::Update,
        ] {
            assert!(!permitted(death, action));
        }
    }

    #[test]
    fn the_event_and_control_id_are_pulled_from_msh_and_cleaned() {
        let m = "MSH|^~\\&|A|B|C|D|20261021||ADT^A28^ADT_A05|MSG 7|P|2.5\rPID|1";
        assert_eq!(message_ids(m), ("A28".into(), "MSG 7".into()));
        assert_eq!(message_ids("hello"), ("?".into(), "?".into()));
        assert_eq!(message_ids("MSH"), ("?".into(), "?".into()));
        let long = format!("MSH|^~\\&|A|B|C|D|T||ADT^A01|{}|P", "x".repeat(200));
        assert_eq!(message_ids(&long).1.len(), 64);
        let ctl = "MSH|^~\\&|A|B|C|D|T||ADT^A01|a\u{7}b|P";
        assert_eq!(message_ids(ctl).1, "ab");
    }

    /// A small deterministic generator, so a failure can be replayed.
    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            self.0 >> 33
        }
        fn below(&mut self, n: usize) -> usize {
            usize::try_from(self.next()).unwrap_or(0) % n.max(1)
        }
    }

    const SAMPLE: &str = "MSH|^~\\&|REG|CLINIC|EPR|HOSP|20261021090000||ADT^A08^ADT_A05|C1|P|2.5\rEVN|A08|20261021090000\rPID|1||M-1^^^LOCAL~9434765919^^^NHS^NH||Smith^Ann^Q||19800314|F|||1 High St^^Leeds^^LS1 4AB||01632960001\r";

    /// Whatever the bytes, reading a message, finding its ids and answering it must not panic, and
    /// an acknowledgement, when there is one, must itself be readable.
    fn exercise(bytes: &[u8]) {
        let text = String::from_utf8_lossy(bytes);
        let read = hl7_adt::patient_from_adt(&text);
        let _ = message_ids(&text);
        if let Some(ack) = hl7_adt::acknowledgement(&text, &read, "ACK1", "20261021090000") {
            assert!(ack.starts_with("MSH"), "{ack:?}");
        }
    }

    /// The sample with field `n` of the first `seg` segment (`MSH` counted as HL7 counts it) set to `value`.
    fn with_field(seg: &str, n: usize, value: &str) -> String {
        SAMPLE
            .split('\r')
            .map(|line| {
                if !line.starts_with(seg) {
                    return line.to_string();
                }
                let mut f: Vec<String> = line.split('|').map(str::to_string).collect();
                // MSH-1 is the separator itself, so MSH-n is at index n-1; other segments' field n is at index n.
                let at = if seg == "MSH" { n - 1 } else { n };
                while f.len() <= at {
                    f.push(String::new());
                }
                f[at] = value.to_string();
                f.join("|")
            })
            .collect::<Vec<_>>()
            .join("\r")
    }

    const MARKER: &str = "QzMarkerQz";

    /// Every text the reader and the feed's answer produce for `text`, other than the stored message itself.
    fn said(text: &str) -> Vec<String> {
        let read = hl7_adt::patient_from_adt(text);
        let mut out = Vec::new();
        match &read {
            Ok(i) => out.extend(i.warnings.iter().cloned()),
            Err(Rejection::Unsupported(m) | Rejection::NotHl7(m)) => out.push(m.clone()),
            Err(Rejection::Invalid(m) | Rejection::Kept(m)) => out.extend(m.iter().cloned()),
        }
        let who = FeedIdentity {
            agent: "mllp:test".into(),
            peer: None,
            peer_certificate: None,
            kept_answer: KeptAnswer::Reject,
            processing_id: None,
            rules: SenderRules::default(),
        };
        for kept in [false, true] {
            let answer = read.clone().map_err(|r| answer_for(r, kept, &who));
            if let Some(ack) = hl7_adt::acknowledgement(text, &answer, "ACK1", "20261021090000") {
                // The acknowledgement's MSH copies the sender's own MSH-3 to MSH-6 and MSA-2 echoes MSH-10, which the
                // marker never goes in; everything else must not carry it.
                out.extend(
                    ack.split('\r')
                        .filter(|l| l.starts_with("MSA"))
                        .map(str::to_string),
                );
            }
        }
        let (event, _) = message_ids(text);
        out.push(event);
        out
    }

    #[test]
    fn no_message_value_appears_in_a_reason_a_warning_or_an_acknowledgement() {
        // IH-4: the marker in every PID field (as a whole value, and in each of five components and a repetition),
        // in MSH-7, MSH-8 and EVN; never in what the feed says about the message. The structural fields (MSH-3 to
        // 6, MSH-9 to 12) are left out: the acknowledgement and the audit row carry them on purpose.
        let mut cases: Vec<(String, String)> = Vec::new();
        for n in 1..=30 {
            for value in [
                MARKER.to_string(),
                format!("{MARKER}^{MARKER}^{MARKER}^{MARKER}^{MARKER}"),
                format!("x~{MARKER}"),
            ] {
                cases.push((format!("PID-{n} = {value}"), with_field("PID", n, &value)));
            }
        }
        for n in [7, 8] {
            cases.push((format!("MSH-{n}"), with_field("MSH", n, MARKER)));
        }
        for n in 1..=6 {
            cases.push((format!("EVN-{n}"), with_field("EVN", n, MARKER)));
        }
        // And text that does not read as HL7 at all, where the reader's own error is the reason kept.
        for text in [
            MARKER.to_string(),
            format!("MSH{MARKER}"),
            format!("MSH|^~\\&|{MARKER}"),
            format!("{MARKER}\rPID|1||{MARKER}"),
            format!("MSH|^~\\&|A|B|C|D|T||ADT^A28|C9|P|2.5\rPID|1||{MARKER}\u{0}{MARKER}"),
        ] {
            cases.push((format!("unreadable {text:?}"), text));
        }
        let mut leaks = Vec::new();
        for (what, text) in &cases {
            for line in said(text) {
                if line.contains(MARKER) {
                    leaks.push(format!("{what}: {line}"));
                }
            }
        }
        assert!(
            leaks.is_empty(),
            "a message value leaked:\n{}",
            leaks.join("\n")
        );
    }

    #[test]
    fn the_sample_is_readable_so_the_fuzzing_starts_from_something_real() {
        assert!(hl7_adt::patient_from_adt(SAMPLE).is_ok());
    }

    #[test]
    fn a_message_cut_anywhere_or_with_any_one_byte_removed_never_panics() {
        let bytes = SAMPLE.as_bytes();
        for n in 0..=bytes.len() {
            exercise(&bytes[..n]);
            exercise(&bytes[n..]);
            let mut without = bytes.to_vec();
            if n < without.len() {
                without.remove(n);
                exercise(&without);
            }
        }
    }

    #[test]
    fn random_damage_never_panics() {
        let mut rng = Lcg(0x00EC_0DE5);
        let specials: &[u8] = b"|^~\\&\r\n\x0b\x1c\0 ";
        for _ in 0..3_000 {
            let mut m = SAMPLE.as_bytes().to_vec();
            for _ in 0..=rng.below(6) {
                let at = rng.below(m.len() + 1);
                match rng.below(5) {
                    0 if at < m.len() => {
                        m.remove(at);
                    }
                    1 => m.insert(at, specials[rng.below(specials.len())]),
                    2 => m.insert(at, u8::try_from(rng.below(256)).unwrap_or(0)),
                    3 if at < m.len() => m[at] = specials[rng.below(specials.len())],
                    _ => {
                        let end = (at + rng.below(40)).min(m.len());
                        let chunk = m[at..end].to_vec();
                        let to = rng.below(m.len() + 1);
                        for (i, b) in chunk.into_iter().enumerate() {
                            m.insert((to + i).min(m.len()), b);
                        }
                    }
                }
            }
            exercise(&m);
        }
    }

    #[test]
    fn arbitrary_bytes_and_huge_fields_never_panic() {
        let mut rng = Lcg(7);
        for _ in 0..500 {
            let len = rng.below(400);
            let bytes: Vec<u8> = (0..len)
                .map(|_| u8::try_from(rng.below(256)).unwrap_or(0))
                .collect();
            exercise(&bytes);
        }
        let long = "A".repeat(200_000);
        exercise(
            format!("MSH|^~\\&|{long}|X|||||ADT^A08|{long}|P|2.5\rPID|1||{long}\r").as_bytes(),
        );
        exercise(format!("MSH|^~\\&{}\r", "|".repeat(50_000)).as_bytes());
    }

    #[test]
    fn random_frames_are_either_a_frame_an_error_or_more_to_read() {
        use crate::mllp::take_frame;
        let mut rng = Lcg(99);
        for _ in 0..2_000 {
            let len = rng.below(64);
            let mut buf: Vec<u8> = (0..len)
                .map(|_| match rng.below(4) {
                    0 => crate::mllp::VT,
                    1 => crate::mllp::FS,
                    2 => crate::mllp::CR,
                    _ => u8::try_from(rng.below(256)).unwrap_or(0),
                })
                .collect();
            let before = buf.len();
            if let Ok(Some(payload)) = take_frame(&mut buf, 32) {
                assert!(payload.len() <= 32);
                assert!(buf.len() < before, "a frame consumes its bytes");
            }
        }
    }
}
