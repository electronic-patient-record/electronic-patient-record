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
//!   a valid NHS number). One match is an update; none is a new patient; more than one is refused.
//! - A new patient that the matcher thinks may already exist is **refused**, not created: a duplicate
//!   made by a machine at 3 a.m. is worse than a message that waits.
//! - An update never blanks a field the message does not carry, and it never changes a birth date or
//!   an NHS number: those corrections are made by a person, in the UI, with a reason.
//! - A death on a message (BV-2) is a report: it is stored as an assertion from this feed, it holds
//!   contact at once, and the message is answered `AA`. It never touches the identity record, and only
//!   a person can confirm or withdraw it. A repeat of what the feed already said adds nothing, because
//!   a source that sets PID-29 sends it on every later message. A repeat of a report a person withdrew
//!   as an error is refused, so that neither the error nor a real death passes unseen. A message about
//!   nobody on record is refused rather than creating a patient in order to report them dead.
//! - The change and its audit row are one transaction; the audit row holds the event, the control id
//!   and what happened, never the message.
//!
//! Not built: `A40` merges, `PV1`, `ORU`/`SIU`, a work queue for the messages that are refused
//! (the sender is told `AE` and is expected to hold and retry), and TLS.

use crate::audit::{self, Action, Event};
use crate::controllers::patients::likely_duplicates;
use crate::death::{self, Actor, Assertion, Occurred, Source, Status};
use crate::death_store;
use crate::feed_queue;
use crate::interop::hl7_adt::{self, Inbound, Rejection};
use crate::patients_store::{self as store, StoreError};
use crate::policy::{self, Consent, Purpose, Relationship, Resource, Role, Subject};
use electronic_patient_record_matcher::{Identifier, NHS_NUMBER_SYSTEM, Patient};
use sqlx::PgPool;

/// Who the feed is, for the audit trail and the policy check. The sender's own claim (MSH-3, MSH-4)
/// is never used for either.
#[derive(Debug, Clone)]
pub struct FeedIdentity {
    /// The audit agent, e.g. `mllp:adt-feed`.
    pub agent: String,
    /// The peer's address, recorded on every row.
    pub peer: Option<String>,
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
/// and its other names are added if new. A different birth date or NHS number is a conflict.
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
    let nhs = |p: &Patient| {
        p.identifiers
            .iter()
            .find(|i| i.system == NHS_NUMBER_SYSTEM)
            .map(|i| i.value.clone())
    };
    if let (Some(a), Some(b)) = (nhs(existing), nhs(incoming))
        && a != b
    {
        conflicts.push(
            "the NHS number differs from the record: a person must correct it, with a reason"
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

/// Find the one patient a message is about, by the identifiers it carries.
async fn find(db: &PgPool, patient: &Patient) -> Result<Option<store::Stored>, Refusal> {
    // The sender's own numbers first, then a national number.
    let locals = patient.identifiers.iter().filter(|i| is_local(i));
    let nhs = patient
        .identifiers
        .iter()
        .filter(|i| i.system == NHS_NUMBER_SYSTEM);
    for id in locals.chain(nhs) {
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
    let now = chrono::Utc::now();
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
    let mut result = apply_message(db, inbound, who).await;
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
        row.detail = Some(serde_json::json!({
            "assertion": stored.id,
            "version": stored.version,
            "status": stored.assertion.status.word(),
            "control_id": inbound.control_id,
        }));
        audit::append(&mut *tx, audit::chain(), &row)
            .await
            .map_err(|e| temporary(&StoreError::Db(e.to_string())))?;
    }
    let mut event = Event::new(audit_action, who.agent.clone());
    event.patient_ref = Some(format!("Patient/{id}"));
    event.resource_ref = Some("Patient/hl7".into());
    event.purpose_of_use = Some(Purpose::Operations.code().into());
    event.network_address.clone_from(&who.peer);
    event.detail = Some(serde_json::json!({
        "hl7_event": inbound.event,
        "control_id": inbound.control_id,
        "result": match &applied {
            Applied::Created { .. } => "created",
            Applied::Updated { .. } => "updated",
            Applied::Unchanged { .. } => "unchanged",
        },
        "warnings": inbound.warnings.len(),
        "candidates_cut": candidates_cut,
    }));
    audit::append(&mut *tx, audit::chain(), &event)
        .await
        .map_err(|e| temporary(&StoreError::Db(e.to_string())))?;
    tx.commit()
        .await
        .map_err(|e| temporary(&StoreError::Db(e.to_string())))?;
    Ok(applied)
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

/// Write the audit row for a message that was refused (nothing else was written). Failure is logged.
async fn audit_refusal(
    db: &PgPool,
    who: &FeedIdentity,
    event: &str,
    control_id: &str,
    why: &Refusal,
    text: &str,
) {
    let mut row = Event::new(Action::Execute, who.agent.clone());
    row.outcome = 4;
    row.resource_ref = Some("Patient/hl7".into());
    row.purpose_of_use = Some(Purpose::Operations.code().into());
    row.network_address.clone_from(&who.peer);
    row.detail = Some(serde_json::json!({
        "hl7_event": event,
        "control_id": control_id,
        "refused": match why {
            Refusal::Unsupported(_) => "unsupported",
            Refusal::Conflict(_) => "conflict",
            Refusal::Temporary(_) => "temporary",
        },
    }));
    if let Err(e) = audit::append(db, audit::chain(), &row).await {
        tracing::error!(error = %e, "an ADT refusal could not be audited");
    }
    // Keep the message for a person (the work queue). A transient failure is not queued: the
    // sender retries and nothing is wrong with the message. A failure here is logged; the
    // acknowledgement still says the message was refused.
    let (kind, reasons) = match why {
        Refusal::Unsupported(m) => (feed_queue::Kind::Unsupported, vec![m.clone()]),
        Refusal::Conflict(m) => (feed_queue::Kind::Conflict, m.clone()),
        Refusal::Temporary(_) => return,
    };
    if let Err(e) = feed_queue::record(
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
        tracing::error!(error = %e, "a refused ADT message could not be queued");
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
    let read = hl7_adt::patient_from_adt(text);
    let outcome: Result<Inbound, Rejection> = match &read {
        Ok(inbound) => match apply(db, inbound, who).await {
            Ok(applied) => {
                tracing::info!(event = %inbound.event, control_id = %inbound.control_id, ?applied, "ADT message applied");
                Ok(inbound.clone())
            }
            Err(refusal) => {
                audit_refusal(db, who, &inbound.event, &inbound.control_id, &refusal, text).await;
                Err(match refusal {
                    Refusal::Unsupported(m) => Rejection::Unsupported(m),
                    Refusal::Conflict(m) => Rejection::Invalid(m),
                    Refusal::Temporary(m) => Rejection::Invalid(vec![m]),
                })
            }
        },
        Err(rejection) => {
            if !matches!(rejection, Rejection::NotHl7(_)) {
                let why = match rejection {
                    Rejection::Unsupported(m) => Refusal::Unsupported(m.clone()),
                    Rejection::Invalid(m) => Refusal::Conflict(m.clone()),
                    Rejection::NotHl7(m) => Refusal::Conflict(vec![m.clone()]),
                };
                let (event, control_id) = message_ids(text);
                audit_refusal(db, who, &event, &control_id, &why, text).await;
            }
            Err(rejection.clone())
        }
    };
    hl7_adt::acknowledgement(text, &outcome, ack_id, timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;
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
                    system: NHS_NUMBER_SYSTEM.into(),
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
    fn a_different_birth_date_or_nhs_number_is_never_applied() {
        let dob = Patient {
            birth_date: Some("1981-03-14".into()),
            ..Patient::default()
        };
        let e = merge_update(&person(), &dob).unwrap_err();
        assert!(e[0].contains("birth date"), "{e:?}");
        let nhs = Patient {
            identifiers: vec![Identifier {
                system: NHS_NUMBER_SYSTEM.into(),
                value: "4010232137".into(),
            }],
            ..Patient::default()
        };
        let e = merge_update(&person(), &nhs).unwrap_err();
        assert!(e[0].contains("NHS number"), "{e:?}");
        // Both at once: both are reported.
        let both = Patient {
            birth_date: Some("1999-01-01".into()),
            ..nhs
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
