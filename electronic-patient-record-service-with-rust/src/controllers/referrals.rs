//! Referral routes (task CA-3, second slice): the closed loop of `spec/interoperability/closed-loop-referral.md`.
//!
//! - `GET  /api/patients/{id}/referrals`: the patient's referrals, current version of each.
//! - `POST /api/patients/{id}/referrals`: send one.
//! - `GET  /api/patients/{id}/referrals/{referral_id}`: the whole history of one loop, oldest first.
//! - `POST /api/patients/{id}/referrals/{referral_id}/events`: the next step, `{event, expected_version, note}`.
//!
//! A referral is clinical content, so the record-level decision applies in full: role, care relationship
//! (or a break-the-glass grant, for a read), purpose of use and consent. Every step is an append; the state
//! table (`referral.rs`) decides which step may follow which.
//!
//! **Who is the referrer and who the recipient (OQ-PC2, assumed).** The service has no model of an
//! organisation that receives referrals, so the role is taken from the history: the subject that sent the
//! referral acts as the referrer, anyone else with the update permission and a care relationship acts as the
//! recipient. The store then refuses an event sent by the wrong role, so the referrer can only *request* a
//! cancellation and only another subject can confirm it. This does not prove that the recipient is the team
//! the referral names. The audit row records ids, versions, states and the role, never the reason, the
//! question or a note.

use super::common::{Access, audit_resource, authorize, begin, commit, purpose_of};
use crate::api_error::ApiError;
use crate::audit::Action;
use crate::auth::Agent;
use crate::patients_store::StoreError;
use crate::policy::{self, Purpose, Resource, Subject};
use crate::referral::{Actor, Event};
use crate::referrals_store::{self as store, Referral, Step, StoredReferral};
use axum::{
    Extension,
    extract::{Json, Path},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};

/// Send a referral.
#[derive(Debug, Deserialize)]
pub struct CreateRequest {
    /// The referral.
    pub referral: Referral,
}

/// The next step of a referral.
#[derive(Debug, Deserialize)]
pub struct EventRequest {
    /// The event, as the state table spells it (`accept`, `request-cancel`, …).
    pub event: String,
    /// The version this step was made from; anything else is a 409.
    pub expected_version: i32,
    /// Why or what, for the events that need one.
    #[serde(default)]
    pub note: Option<String>,
}

/// A patient's referrals.
#[derive(Debug, Serialize)]
pub struct ReferralList {
    /// Current version of each, newest first.
    pub referrals: Vec<StoredReferral>,
}

/// One referral's history.
#[derive(Debug, Serialize)]
pub struct ReferralHistory {
    /// Every version, oldest first.
    pub versions: Vec<StoredReferral>,
}

fn failure(e: StoreError) -> ApiError {
    match e {
        StoreError::BadQuery(m) => ApiError::invalid(vec![m]),
        other => ApiError::from(&other),
    }
}

/// Audit a refusal of a write (a miss, a stale version, a refused step).
async fn refused(
    ctx: &AppContext,
    agent: &Agent,
    purpose: Purpose,
    action: Action,
    e: &StoreError,
) -> Option<ApiError> {
    if !matches!(
        e,
        StoreError::NotFound | StoreError::Conflict { .. } | StoreError::BadQuery(_)
    ) {
        return None;
    }
    audit_resource(
        &ctx.db,
        agent,
        Access {
            action,
            outcome: 4,
            purpose,
            resource: "Referral",
            patient_id: None,
        },
        // A store message such as "version conflict" or "a referral that is declined cannot take Accept".
        serde_json::json!({ "refused": e.to_string() }),
    )
    .await
    .err()
}

pub(super) async fn list(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path(patient): Path<String>,
) -> Response {
    let purpose = match purpose_of(&headers) {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    if let Err(r) = authorize(
        &ctx.db,
        &agent,
        &subject,
        Resource::Referral,
        policy::Action::Read,
        purpose,
        Some(&patient),
    )
    .await
    {
        return r.into_response();
    }
    let referrals = match store::for_patient(&ctx.db, &patient).await {
        Ok(r) => r,
        Err(e) => return failure(e).into_response(),
    };
    // Audit before the clinical content leaves.
    if let Err(r) = audit_resource(
        &ctx.db,
        &agent,
        Access {
            action: Action::Read,
            outcome: 0,
            purpose,
            resource: "Referral",
            patient_id: Some(&patient),
        },
        serde_json::json!({ "referrals": referrals.len() }),
    )
    .await
    {
        return r.into_response();
    }
    Json(ReferralList { referrals }).into_response()
}

pub(super) async fn history(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path((patient, referral_id)): Path<(String, String)>,
) -> Response {
    let purpose = match purpose_of(&headers) {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    if let Err(r) = authorize(
        &ctx.db,
        &agent,
        &subject,
        Resource::Referral,
        policy::Action::Read,
        purpose,
        Some(&patient),
    )
    .await
    {
        return r.into_response();
    }
    let versions = match store::history(&ctx.db, &referral_id).await {
        // A referral id from another patient's record is a miss, never a disclosure.
        Ok(v) if v.first().is_some_and(|f| f.patient_id == patient) => v,
        Ok(_) => return failure(StoreError::NotFound).into_response(),
        Err(e) => {
            if let Some(r) = refused(&ctx, &agent, purpose, Action::Read, &e).await {
                return r.into_response();
            }
            return failure(e).into_response();
        }
    };
    if let Err(r) = audit_resource(
        &ctx.db,
        &agent,
        Access {
            action: Action::Read,
            outcome: 0,
            purpose,
            resource: "Referral",
            patient_id: Some(&patient),
        },
        serde_json::json!({ "referral": referral_id, "versions": versions.len() }),
    )
    .await
    {
        return r.into_response();
    }
    Json(ReferralHistory { versions }).into_response()
}

pub(super) async fn create(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path(patient): Path<String>,
    Json(request): Json<CreateRequest>,
) -> Response {
    let purpose = match purpose_of(&headers) {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    if let Err(r) = authorize(
        &ctx.db,
        &agent,
        &subject,
        Resource::Referral,
        policy::Action::Create,
        purpose,
        Some(&patient),
    )
    .await
    {
        return r.into_response();
    }
    let tx = match begin(&ctx.db).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    match store::request(
        &tx,
        &agent.0,
        &patient,
        &request.referral,
        chrono::Utc::now(),
    )
    .await
    {
        Ok(stored) => {
            if let Err(r) = audit_resource(
                &tx,
                &agent,
                Access {
                    action: Action::Create,
                    outcome: 0,
                    purpose,
                    resource: "Referral",
                    patient_id: Some(&stored.patient_id),
                },
                serde_json::json!({
                    "referral": stored.id,
                    "version": stored.version,
                    "state": stored.state.as_str(),
                    "priority": request.referral.priority,
                }),
            )
            .await
            {
                return r.into_response();
            }
            if let Err(r) = commit(tx).await {
                return r.into_response();
            }
            (StatusCode::CREATED, Json(stored)).into_response()
        }
        Err(e) => {
            let _ = sea_orm::TransactionSession::rollback(tx).await;
            if let Some(r) = refused(&ctx, &agent, purpose, Action::Create, &e).await {
                return r.into_response();
            }
            failure(e).into_response()
        }
    }
}

pub(super) async fn advance(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path((patient, referral_id)): Path<(String, String)>,
    Json(request): Json<EventRequest>,
) -> Response {
    let purpose = match purpose_of(&headers) {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    if let Err(r) = authorize(
        &ctx.db,
        &agent,
        &subject,
        Resource::Referral,
        policy::Action::Update,
        purpose,
        Some(&patient),
    )
    .await
    {
        return r.into_response();
    }
    let Some(event) = Event::parse(&request.event) else {
        return ApiError::invalid(vec![format!("unknown event {:?}", request.event)])
            .into_response();
    };
    let tx = match begin(&ctx.db).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    match step(&tx, &patient, &referral_id, &agent.0, event, &request).await {
        Ok((stored, actor)) => {
            if let Err(r) = audit_resource(
                &tx,
                &agent,
                Access {
                    action: Action::Update,
                    outcome: 0,
                    purpose,
                    resource: "Referral",
                    patient_id: Some(&stored.patient_id),
                },
                serde_json::json!({
                    "referral": stored.id,
                    "version": stored.version,
                    "event": event.as_str(),
                    "role": match actor { Actor::Referrer => "referrer", Actor::Recipient => "recipient" },
                    "state": stored.state.as_str(),
                }),
            )
            .await
            {
                return r.into_response();
            }
            if let Err(r) = commit(tx).await {
                return r.into_response();
            }
            Json(stored).into_response()
        }
        Err(e) => {
            let _ = sea_orm::TransactionSession::rollback(tx).await;
            if let Some(r) = refused(&ctx, &agent, purpose, Action::Update, &e).await {
                return r.into_response();
            }
            failure(e).into_response()
        }
    }
}

/// Work out who is acting from the history, then take the step.
async fn step(
    tx: &sea_orm::DatabaseTransaction,
    patient: &str,
    referral_id: &str,
    by: &str,
    event: Event,
    request: &EventRequest,
) -> Result<(StoredReferral, Actor), StoreError> {
    let versions = store::history(tx, referral_id).await?;
    let first = versions.first().ok_or(StoreError::NotFound)?;
    // A referral id taken from another patient's record is a miss.
    if first.patient_id != patient {
        return Err(StoreError::NotFound);
    }
    let actor = if first.author == by {
        Actor::Referrer
    } else {
        Actor::Recipient
    };
    let stored = store::advance(
        tx,
        referral_id,
        request.expected_version,
        Step {
            actor,
            by,
            event,
            note: request.note.as_deref(),
        },
        chrono::Utc::now(),
    )
    .await?;
    Ok((stored, actor))
}
