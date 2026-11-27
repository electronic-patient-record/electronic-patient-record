//! Endpoints for a refusal of disclosure that the organisation declined (DR-10): record one, list them, and mark one
//! entered in error. Registered by `patients::routes` under `/api/patients/{id}/declined-refusals`.
//!
//! A decline is written only where the tenant's pack allows it (`EPR_DISCLOSURE_RULES_FILE`; never under `binds`, the
//! default, and never with no pack), and it names the pack's version. It is never changed. **It decides nothing yet**: the
//! refusal still applies. The patient reads their own, as they must be told. Audit rows carry ids, the version, the state
//! and the ground, never the words.

use super::common::{Access, audit_resource, authorize, begin, commit, purpose_of, store_failure};
use crate::api_error::ApiError;
use crate::audit::Action;
use crate::auth::Agent;
use crate::declined_refusal::{self, Declined};
use crate::declined_refusals_store as store;
use crate::patients_store::{self, StoreError};
use crate::policy::{self, Resource, Subject};
use crate::pool::Pool;
use axa::extract::{Json, Path};
use axa::prelude::*;
use axum::{
    Extension,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

/// A new decline.
#[derive(Debug, Deserialize)]
pub struct CreateRequest {
    /// The consent whose refusal is declined.
    pub consent_id: String,
    /// The decline.
    pub declined: Declined,
}

/// Marking a decline entered in error.
#[derive(Debug, Deserialize)]
pub struct EnteredInErrorRequest {
    /// The version this was decided from; anything else is a 409.
    pub expected_version: i32,
    /// Why. Required.
    pub reason: String,
}

/// One decline in a list: the stored version, and whether it is on another record merged into this one.
#[derive(Debug, Serialize)]
pub struct Listed<'a> {
    /// The decline.
    #[serde(flatten)]
    pub stored: &'a store::StoredDecline,
    /// True when it is recorded on a record merged into this one.
    pub from_merged_record: bool,
}

fn invalid(problems: &[String]) -> Response {
    ApiError::invalid(problems.to_vec()).into_response()
}

fn detail(stored: &store::StoredDecline) -> serde_json::Value {
    serde_json::json!({ "decline": stored.id, "consent": stored.consent_id, "version": stored.version, "state": stored.state.as_str(), "ground": stored.declined.ground.as_str() })
}

/// `POST /api/patients/{id}/declined-refusals`: record that a refusal of disclosure was declined, where the pack allows it.
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
        &ctx.pool(),
        &agent,
        &subject,
        Resource::DeclinedRefusal,
        policy::Action::Create,
        purpose,
        Some(&patient),
    )
    .await
    {
        return r.into_response();
    }
    // The pack in force decides whether a refusal may be declined at all; with none, it may not.
    let pack = match crate::disclosure_pack::load() {
        Ok(p) => p,
        Err(e) => {
            tracing::error!(error = %e, "the disclosure pack could not be read");
            return ApiError::internal().into_response();
        }
    };
    let problems =
        declined_refusal::validate(&request.declined, pack.as_ref(), jiff::Timestamp::now());
    if !problems.is_empty() {
        return invalid(&problems);
    }
    let Some(pack_version) = pack.as_ref().map(|p| p.version().to_string()) else {
        return invalid(&["a refusal cannot be declined here: there is no pack".to_string()]);
    };
    let mut tx = match begin(&ctx.pool()).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    match store::create(
        &mut *tx,
        &patient,
        &request.consent_id,
        &pack_version,
        &agent.0,
        &request.declined,
    )
    .await
    {
        Ok(stored) => {
            if let Err(r) = audit_resource(
                &mut *tx,
                &agent,
                Access {
                    action: Action::Create,
                    outcome: 0,
                    purpose,
                    resource: "DeclinedRefusal",
                    patient_id: Some(&stored.patient_id),
                },
                detail(&stored),
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
            let _ = tx.rollback().await;
            refused(&ctx, &agent, purpose, Action::Create, &e).await
        }
    }
}

/// `GET /api/patients/{id}/declined-refusals`: every decline on the patient record and on records merged into it, newest
/// first. The patient reads their own. Audited with the count.
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
        &ctx.pool(),
        &agent,
        &subject,
        Resource::DeclinedRefusal,
        policy::Action::Read,
        purpose,
        Some(&patient),
    )
    .await
    {
        return r.into_response();
    }
    let known = match patients_store::current(&ctx.pool(), &patient).await {
        Ok(p) => p,
        Err(e) => return refused(&ctx, &agent, purpose, Action::Read, &e).await,
    };
    match store::list_with_merged(&ctx.pool(), &known.id).await {
        Ok(found) => {
            if let Err(r) = audit_resource(
                &ctx.pool(),
                &agent,
                Access {
                    action: Action::Read,
                    outcome: 0,
                    purpose,
                    resource: "DeclinedRefusal",
                    patient_id: Some(&known.id),
                },
                serde_json::json!({ "declines": found.len() }),
            )
            .await
            {
                return r.into_response();
            }
            let declines: Vec<Listed<'_>> = found
                .iter()
                .map(|stored| Listed {
                    stored,
                    from_merged_record: stored.patient_id != known.id,
                })
                .collect();
            Json(serde_json::json!({ "declines": declines })).into_response()
        }
        Err(e) => store_failure(&e),
    }
}

/// `POST /api/patients/{id}/declined-refusals/{decline_id}/entered-in-error`: mark a decline recorded by mistake. The
/// clinical administrator's act; the decline stays as made.
pub(super) async fn entered_in_error(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path((patient, decline_id)): Path<(String, String)>,
    Json(request): Json<EnteredInErrorRequest>,
) -> Response {
    let purpose = match purpose_of(&headers) {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    if let Err(r) = authorize(
        &ctx.pool(),
        &agent,
        &subject,
        Resource::DeclinedRefusal,
        policy::Action::MarkEnteredInError,
        purpose,
        Some(&patient),
    )
    .await
    {
        return r.into_response();
    }
    let mut tx = match begin(&ctx.pool()).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    match store::mark_entered_in_error(
        &mut *tx,
        &patient,
        &decline_id,
        request.expected_version,
        &agent.0,
        &request.reason,
    )
    .await
    {
        Ok(stored) => {
            if let Err(r) = audit_resource(
                &mut *tx,
                &agent,
                Access {
                    action: Action::Update,
                    outcome: 0,
                    purpose,
                    resource: "DeclinedRefusal",
                    patient_id: Some(&stored.patient_id),
                },
                detail(&stored),
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
            let _ = tx.rollback().await;
            refused(&ctx, &agent, purpose, Action::Update, &e).await
        }
    }
}

/// A store failure as a response; a miss, a stale version or a refused rule is also an audited refusal.
async fn refused(
    ctx: &AppContext,
    agent: &Agent,
    purpose: policy::Purpose,
    action: Action,
    e: &StoreError,
) -> Response {
    if matches!(
        e,
        StoreError::NotFound | StoreError::Conflict { .. } | StoreError::BadQuery(_)
    ) && let Err(r) = audit_resource(
        &ctx.pool(),
        agent,
        Access {
            action,
            outcome: 4,
            purpose,
            resource: "DeclinedRefusal",
            patient_id: None,
        },
        serde_json::json!({ "refused": e.refusal_kind() }),
    )
    .await
    {
        return r.into_response();
    }
    match e {
        StoreError::BadQuery(m) => invalid(std::slice::from_ref(m)),
        other => store_failure(other),
    }
}
