//! What every controller shares: the purpose of use, the record-level access decision (role,
//! relationship, consent, break-the-glass), the write transaction, the audit row and the error
//! answers (CQ-11). Error bodies come from [`crate::api_error::ApiError`], never from here.

use crate::api_error::ApiError;
use crate::audit::{self, Action, AuditError, Event};
use crate::auth::{self, Agent};
use crate::consent;
use crate::consents_store;
use crate::links;
use crate::patients_store::StoreError;
use crate::policy::{self, Consent, Purpose, Relationship, Resource, Subject};
use crate::relationships;
use axum::{
    http::HeaderMap,
    response::{IntoResponse, Response},
};

/// The purpose of use the caller declared, `operations` by default. An unknown code is a 400.
pub(super) fn purpose_of(headers: &HeaderMap) -> Result<Purpose, Box<Response>> {
    match headers.get("x-purpose-of-use") {
        None => Ok(Purpose::Operations),
        Some(v) => v
            .to_str()
            .ok()
            .and_then(Purpose::parse)
            .ok_or_else(|| ApiError::bad_request("unknown X-Purpose-Of-Use").into()),
    }
}

pub(super) fn audit_failed(e: &AuditError) -> Response {
    tracing::error!(error = %e, "refusing to answer: the access could not be audited");
    ApiError::internal().into_response()
}

/// What the patients' consent says about this subject reading this kind of record, per patient.
/// A patient with no active consent has no entry. With login off (development only) there is no
/// subject and nothing is looked up. A failed lookup fails the request: an unchecked disclosure is
/// how a refusal gets ignored.
pub(super) async fn consent_verdicts(
    db: &impl sea_orm::ConnectionTrait,
    subject: &Subject,
    purpose: Purpose,
    resource: Resource,
    patient_ids: &[String],
) -> Result<std::collections::HashMap<String, consent::Verdict>, Box<Response>> {
    let mut verdicts = std::collections::HashMap::new();
    if !auth::authentication_enforced() || patient_ids.is_empty() {
        return Ok(verdicts);
    }
    // A refusal on a record that was merged into this one stands for this one too: the person's
    // choice does not lapse because two records were joined. Two queries for any number of ids.
    let active = consents_store::active_with_merged(db, patient_ids)
        .await
        .map_err(|e| Box::new(store_failure(&e)))?;
    let ctx = consent::Context {
        roles: subject.roles.clone(),
        purpose,
        resource,
        labels: std::collections::BTreeSet::new(),
        now: time_now(),
    };
    for (id, records) in &active {
        let verdict = consent::decide(records.iter().map(|c| (c.id.as_str(), &c.consent)), &ctx);
        verdicts.insert(id.clone(), verdict);
    }
    Ok(verdicts)
}

/// The record-level decision for an access. With login off (development only) there is no subject
/// and nothing to decide. For a read of one patient's identity the patient's consent is looked up
/// and applied. On a refusal the refusal is audited and the 403 is returned.
pub(super) async fn authorize(
    db: &sea_orm::DatabaseConnection,
    agent: &Agent,
    subject: &Subject,
    resource: Resource,
    action: policy::Action,
    purpose: Purpose,
    patient_id: Option<&str>,
) -> Result<(), Box<Response>> {
    if !auth::authentication_enforced() {
        return Ok(());
    }
    let mut verdict = consent::Verdict {
        consent: Consent::NotRecorded,
        decided_by: Vec::new(),
    };
    // The patient's consent is looked up for every read of one patient's identity or clinical data.
    if let (true, true, Some(id)) = (
        action.reads(),
        resource == Resource::PatientIdentity || resource.clinical(),
        patient_id,
    ) {
        let mut all = consent_verdicts(db, subject, purpose, resource, &[id.to_string()]).await?;
        if let Some(v) = all.remove(&id.to_lowercase()) {
            verdict = v;
        }
    }
    // A record merged into another is read-only: changes go to the surviving record. (Consent and
    // relationship changes are allowed, so a refusal on the old record can still be withdrawn.)
    if !action.reads()
        && (resource == Resource::PatientIdentity || resource.clinical())
        && let Some(id) = patient_id
        && let Some(into) = links::merged_into(db, id)
            .await
            .map_err(|e| Box::new(store_failure(&e)))?
    {
        return Err(
            ApiError::conflict("this record was merged into another: change that one")
                .with("replaced_by", into)
                .into(),
        );
    }
    // The subject's relationship to this patient, from the recorded relationships. A failed lookup
    // fails the request: assuming "none" would be safe, but would hide an outage as a refusal.
    let relationship = match patient_id {
        Some(id) => relationships::lookup(db, id, &agent.0, time_now())
            .await
            .map_err(|e| Box::new(store_failure(&e)))?,
        None => Relationship::None,
    };
    // A grant matters only to a treating role reading clinical content without a care relationship.
    let grant = match patient_id {
        Some(id)
            if action.reads()
                && resource.clinical()
                && !relationship.treating()
                && subject.roles.iter().any(|r| r.treats()) =>
        {
            crate::break_glass::active(db, id, &agent.0)
                .await
                .map_err(|e| Box::new(store_failure(&e)))?
        }
        _ => None,
    };
    let decision = policy::evaluate(&policy::Request {
        subject: subject.clone(),
        resource,
        action,
        labels: std::collections::BTreeSet::new(),
        relationship,
        purpose,
        consent: verdict.consent,
        break_glass: grant.as_ref().map(|g| policy::BreakGlass {
            reason: g.reason.clone(),
            expires_at: g.expires_unix,
        }),
        now: time_now(),
    });
    if let (Some(g), policy::Decision::Allow(obligations)) = (&grant, &decision)
        && obligations
            .iter()
            .any(|o| matches!(o, policy::Obligation::BreakTheGlass(_)))
    {
        // From here every audit row of this request is flagged and names the grant.
        audit::note_break_glass(&g.id);
    }
    let offer = matches!(
        decision,
        policy::Decision::Deny {
            btg_offer: true,
            ..
        }
    );
    if decision.allowed() {
        return Ok(());
    }
    let mut event = Event::new(Action::Execute, agent.0.clone());
    event.outcome = 4;
    event.patient_ref = patient_id.map(|id| format!("Patient/{id}"));
    event.resource_ref = Some(format!("{}/{}", resource.name(), action.name()));
    event.purpose_of_use = Some(purpose.code().to_string());
    // Consent ids only: what the consent says is not copied into the audit row.
    event.detail = Some(
        serde_json::json!({ "denied": format!("{decision:?}"), "consent": verdict.decided_by }),
    );
    if let Err(e) = audit::append(db, audit::chain(), &event).await {
        tracing::error!(error = %e, "a refusal could not be audited");
    }
    // A refusal that break-the-glass could lift says so, so the screen can offer it.
    let refusal = if offer {
        ApiError::forbidden().with("break_glass_offer", true)
    } else {
        ApiError::forbidden()
    };
    Err(refusal.into())
}

/// Start the transaction a write and its audit row share (see [`audit::append`]).
pub(super) async fn begin(
    db: &sea_orm::DatabaseConnection,
) -> Result<sea_orm::DatabaseTransaction, Box<Response>> {
    sea_orm::TransactionTrait::begin(db).await.map_err(|e| {
        tracing::error!(error = %e, "could not start a transaction");
        ApiError::internal().into()
    })
}

/// Commit it. A failure means the write did not happen, so the request fails.
pub(super) async fn commit(tx: sea_orm::DatabaseTransaction) -> Result<(), Box<Response>> {
    sea_orm::TransactionSession::commit(tx).await.map_err(|e| {
        tracing::error!(error = %e, "could not commit a write and its audit row");
        ApiError::internal().into()
    })
}

pub(super) fn time_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

/// What was accessed, for an audit row.
pub(super) struct Access<'a> {
    pub action: Action,
    pub outcome: i16,
    pub purpose: Purpose,
    pub resource: &'a str,
    pub patient_id: Option<&'a str>,
}

/// Write the audit row for an access. Never put demographics or consent content in `detail`.
pub(super) async fn audit_resource(
    db: &(impl sea_orm::ConnectionTrait + sea_orm::TransactionTrait),
    agent: &Agent,
    access: Access<'_>,
    detail: serde_json::Value,
) -> Result<(), Box<Response>> {
    let mut event = Event::new(access.action, agent.0.clone());
    event.outcome = access.outcome;
    event.patient_ref = access.patient_id.map(|id| format!("Patient/{id}"));
    event.resource_ref = Some(access.resource.to_string());
    event.purpose_of_use = Some(access.purpose.code().to_string());
    event.detail = Some(detail);
    audit::append(db, audit::chain(), &event)
        .await
        .map(|_| ())
        .map_err(|e| Box::new(audit_failed(&e)))
}

/// A store error as its response ([`ApiError`]'s `From<&StoreError>`).
pub(super) fn store_failure(e: &StoreError) -> Response {
    ApiError::from(e).into_response()
}
