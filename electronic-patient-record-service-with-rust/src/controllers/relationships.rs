//! Care-relationship endpoints: record, list and end who is on a patient's team, who is the
//! patient and who is their proxy. Clinical administrators only (see `relationships.rs`).
//! Registered by `patients::routes` under `/api/patients/{id}/relationships`.

use super::common::{Access, audit_resource, authorize, begin, commit, purpose_of, store_failure};
use crate::api_error::ApiError;
use crate::audit::Action;
use crate::auth::Agent;
use crate::patients_store::{self, StoreError};
use crate::policy::{self, Resource, Subject};
use crate::relationships::{self as store, RelationshipRecord};
use axa::prelude::*;
use axum::{
    Extension,
    extract::{Json, Path},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;

/// A new relationship.
#[derive(Debug, Deserialize)]
pub struct CreateRequest {
    /// The relationship.
    pub relationship: RelationshipRecord,
    /// Why, for the version history (for `self` or `proxy`, how identity was checked).
    #[serde(default)]
    pub reason: Option<String>,
}

/// A change to a relationship: a new version.
#[derive(Debug, Deserialize)]
pub struct UpdateRequest {
    /// The whole new record (same subject and kind).
    pub relationship: RelationshipRecord,
    /// The version this change was made from.
    pub expected_version: i32,
    /// Why.
    #[serde(default)]
    pub reason: Option<String>,
}

/// The audit detail for a relationship: its id, version, kind, subject and whether it ended, plus a proxy's terms
/// (ids and times, not clinical content), so the grant can be traced. It holds no demographics.
fn detail(s: &store::StoredRelationship) -> serde_json::Value {
    // The subject is an identifier, not demographics; it is in the row so the grant can be traced.
    serde_json::json!({
        "relationship": s.id,
        "version": s.version,
        "kind": s.relationship.kind.as_str(),
        "subject": s.relationship.subject,
        "ended": s.relationship.status == store::Status::Ended,
        // The proxy's terms are ids and times, not clinical content: the grant can be traced.
        "proxy": s.relationship.proxy,
    })
}

/// Audit a refusal of a relationship write (a miss, a stale version or a refused record) and return the answer.
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
        &ctx.db,
        agent,
        Access {
            action,
            outcome: 4,
            purpose,
            resource: "CareRelationship",
            patient_id: None,
        },
        serde_json::json!({ "refused": e.to_string() }),
    )
    .await
    {
        return r.into_response();
    }
    store_failure(e)
}

/// `POST /api/patients/{id}/relationships`: record who is on the care team, or who is the patient or their proxy.
/// Clinical administrators only, never the person who would gain access by it.
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
        Resource::CareRelationship,
        policy::Action::Create,
        purpose,
        Some(&patient),
    )
    .await
    {
        return r.into_response();
    }
    let problems = store::validate(&request.relationship, &agent.0, super::common::time_now());
    if !problems.is_empty() {
        return ApiError::invalid(problems.clone()).into_response();
    }
    let tx = match begin(&ctx.db).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    match store::create(
        &tx,
        &patient,
        &agent.0,
        request.reason.as_deref(),
        &request.relationship,
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
                    resource: "CareRelationship",
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
            // Nothing was written; end the transaction before the refusal is audited.
            let _ = sea_orm::TransactionSession::rollback(tx).await;
            refused(&ctx, &agent, purpose, Action::Create, &e).await
        }
    }
}

/// `GET /api/patients/{id}/relationships`: the relationships recorded for the patient, current version of each. Audited.
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
        Resource::CareRelationship,
        policy::Action::Read,
        purpose,
        Some(&patient),
    )
    .await
    {
        return r.into_response();
    }
    let known = match patients_store::current(&ctx.db, &patient).await {
        Ok(p) => p,
        Err(e) => return refused(&ctx, &agent, purpose, Action::Read, &e).await,
    };
    match store::list(&ctx.db, &known.id).await {
        Ok(found) => {
            if let Err(r) = audit_resource(
                &ctx.db,
                &agent,
                Access {
                    action: Action::Read,
                    outcome: 0,
                    purpose,
                    resource: "CareRelationship",
                    patient_id: Some(&known.id),
                },
                serde_json::json!({ "relationships": found.len() }),
            )
            .await
            {
                return r.into_response();
            }
            Json(serde_json::json!({ "relationships": found })).into_response()
        }
        Err(e) => store_failure(&e),
    }
}

/// `PUT /api/patients/{id}/relationships/{relationship_id}`: change or end a relationship as a new version.
/// A stale `expected_version` is a 409.
pub(super) async fn update(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path((patient, id)): Path<(String, String)>,
    Json(request): Json<UpdateRequest>,
) -> Response {
    let purpose = match purpose_of(&headers) {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    if let Err(r) = authorize(
        &ctx.db,
        &agent,
        &subject,
        Resource::CareRelationship,
        policy::Action::Update,
        purpose,
        Some(&patient),
    )
    .await
    {
        return r.into_response();
    }
    let problems = store::validate(&request.relationship, &agent.0, super::common::time_now());
    if !problems.is_empty() {
        return ApiError::invalid(problems.clone()).into_response();
    }
    let tx = match begin(&ctx.db).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    match store::update(
        &tx,
        &patient,
        &id,
        request.expected_version,
        &agent.0,
        request.reason.as_deref(),
        &request.relationship,
    )
    .await
    {
        Ok(stored) => {
            if let Err(r) = audit_resource(
                &tx,
                &agent,
                Access {
                    action: Action::Update,
                    outcome: 0,
                    purpose,
                    resource: "CareRelationship",
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
            // Nothing was written; end the transaction before the refusal is audited.
            let _ = sea_orm::TransactionSession::rollback(tx).await;
            refused(&ctx, &agent, purpose, Action::Update, &e).await
        }
    }
}
