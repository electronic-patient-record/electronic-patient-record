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
use axum::{
    Extension,
    extract::{Json, Path},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use loco_rs::prelude::*;
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

fn detail(s: &store::StoredRelationship) -> serde_json::Value {
    // The subject is an identifier, not demographics; it is in the row so the grant can be traced.
    serde_json::json!({
        "relationship": s.id,
        "version": s.version,
        "kind": s.relationship.kind.as_str(),
        "subject": s.relationship.subject,
        "ended": s.relationship.status == store::Status::Ended,
    })
}

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
        return *r;
    }
    store_failure(e)
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
        Err(r) => return *r,
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
        return *r;
    }
    let problems = store::validate(&request.relationship);
    if !problems.is_empty() {
        return ApiError::invalid(problems.clone()).into_response();
    }
    let tx = match begin(&ctx.db).await {
        Ok(t) => t,
        Err(r) => return *r,
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
                return *r;
            }
            if let Err(r) = commit(tx).await {
                return *r;
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

pub(super) async fn list(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path(patient): Path<String>,
) -> Response {
    let purpose = match purpose_of(&headers) {
        Ok(p) => p,
        Err(r) => return *r,
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
        return *r;
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
                return *r;
            }
            Json(serde_json::json!({ "relationships": found })).into_response()
        }
        Err(e) => store_failure(&e),
    }
}

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
        Err(r) => return *r,
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
        return *r;
    }
    let problems = store::validate(&request.relationship);
    if !problems.is_empty() {
        return ApiError::invalid(problems.clone()).into_response();
    }
    let tx = match begin(&ctx.db).await {
        Ok(t) => t,
        Err(r) => return *r,
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
                return *r;
            }
            if let Err(r) = commit(tx).await {
                return *r;
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
