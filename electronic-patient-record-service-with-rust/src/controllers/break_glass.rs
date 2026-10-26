//! Break-the-glass routes (task SE-3).
//!
//! - `POST /api/patients/{id}/break-glass`: a treating clinician with no care relationship declares an
//!   emergency and gives a reason; the grant lasts four hours and is not extended by asking again.
//! - `GET /api/break-glass/reviews`, `GET /api/break-glass/reviews/count`: the grants nobody has
//!   reviewed (auditor).
//! - `POST /api/break-glass/reviews/{id}`: record whether a grant was appropriate, with a note. Not by the
//!   person who opened it.
//!
//! A grant and its audit row are one transaction, and the audit row of the grant is itself flagged
//! break-glass. What a grant allows is the policy engine's decision (see `break_glass.rs` at the crate root).
//! There is no real-time notification: the review queue and its count are what exist.

use super::common::{Access, audit_resource, authorize, begin, commit, purpose_of, store_failure};
use crate::api_error::ApiError;
use crate::audit::{self, Action};
use crate::auth::Agent;
use crate::break_glass::{self as store, Outcome};
use crate::patients_store::StoreError;
use crate::policy::{self, Resource, Subject};
use axum::{
    Extension,
    extract::{Json, Path},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use loco_rs::prelude::*;
use serde::Deserialize;

/// Declare an emergency.
#[derive(Debug, Deserialize)]
pub struct OpenRequest {
    /// Why. Required.
    pub reason: String,
}

/// Review a grant.
#[derive(Debug, Deserialize)]
pub struct ReviewRequest {
    /// `appropriate` or `inappropriate`.
    pub outcome: Outcome,
    /// Why. Required.
    pub note: String,
}

fn failure(e: StoreError) -> Response {
    match e {
        StoreError::BadQuery(m) => ApiError::invalid(vec![m.to_string()]).into_response(),
        StoreError::Conflict { .. } => {
            ApiError::new(StatusCode::CONFLICT, "already reviewed").into_response()
        }
        other => store_failure(&other),
    }
}

async fn open(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path(patient): Path<String>,
    Json(request): Json<OpenRequest>,
) -> Response {
    let purpose = match purpose_of(&headers) {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    if let Err(r) = authorize(
        &ctx.db,
        &agent,
        &subject,
        Resource::BreakGlass,
        policy::Action::Create,
        purpose,
        None,
    )
    .await
    {
        return r.into_response();
    }
    let tx = match begin(&ctx.db).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    match store::open(&tx, &patient, &agent.0, &request.reason).await {
        Ok((grant, created)) => {
            // The opening of a grant is itself break-glass access: flag its row.
            audit::note_break_glass(&grant.id);
            if let Err(r) = audit_resource(
                &tx,
                &agent,
                Access {
                    action: Action::Create,
                    outcome: 0,
                    purpose,
                    resource: "BreakGlass",
                    patient_id: Some(&grant.patient_id),
                },
                // Ids and times, never the free-text reason (it is in the grant, for the review).
                serde_json::json!({ "grant": grant.id, "expires_at": grant.expires_at, "new": created }),
            )
            .await
            {
                return r.into_response();
            }
            if let Err(r) = commit(tx).await {
                return r.into_response();
            }
            (
                if created {
                    StatusCode::CREATED
                } else {
                    StatusCode::OK
                },
                Json(grant),
            )
                .into_response()
        }
        Err(e) => {
            let _ = sea_orm::TransactionSession::rollback(tx).await;
            failure(e)
        }
    }
}

async fn guard(
    ctx: &AppContext,
    agent: &Agent,
    subject: &Subject,
    headers: &HeaderMap,
    action: policy::Action,
) -> Result<policy::Purpose, ApiError> {
    let purpose = purpose_of(headers)?;
    authorize(
        &ctx.db,
        agent,
        subject,
        Resource::BreakGlass,
        action,
        purpose,
        None,
    )
    .await?;
    Ok(purpose)
}

async fn list(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
) -> Response {
    let purpose = match guard(&ctx, &agent, &subject, &headers, policy::Action::Search).await {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    let grants = match store::unreviewed(&ctx.db).await {
        Ok(g) => g,
        Err(e) => return failure(e),
    };
    if let Err(r) = audit_resource(
        &ctx.db,
        &agent,
        Access {
            action: Action::Read,
            outcome: 0,
            purpose,
            resource: "BreakGlass",
            patient_id: None,
        },
        serde_json::json!({ "unreviewed": grants.len() }),
    )
    .await
    {
        return r.into_response();
    }
    Json(serde_json::json!({ "grants": grants, "limit": store::LIST_LIMIT, "site_time_zone": crate::site::time_zone() })).into_response()
}

async fn count(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
) -> Response {
    let purpose = match guard(&ctx, &agent, &subject, &headers, policy::Action::Search).await {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    let n = match store::count_unreviewed(&ctx.db).await {
        Ok(n) => n,
        Err(e) => return failure(e),
    };
    let overdue = match store::count_overdue(&ctx.db).await {
        Ok(n) => n,
        Err(e) => return failure(e),
    };
    if let Err(r) = audit_resource(
        &ctx.db,
        &agent,
        Access {
            action: Action::Read,
            outcome: 0,
            purpose,
            resource: "BreakGlass",
            patient_id: None,
        },
        serde_json::json!({ "count": n, "overdue": overdue }),
    )
    .await
    {
        return r.into_response();
    }
    Json(serde_json::json!({ "unreviewed": n, "overdue": overdue })).into_response()
}

async fn review(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<ReviewRequest>,
) -> Response {
    let purpose = match guard(&ctx, &agent, &subject, &headers, policy::Action::Update).await {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    let tx = match begin(&ctx.db).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    match store::review(&tx, &id, &agent.0, request.outcome, &request.note).await {
        Ok(grant) => {
            if let Err(r) = audit_resource(
                &tx,
                &agent,
                Access {
                    action: Action::Update,
                    outcome: 0,
                    purpose,
                    resource: "BreakGlass",
                    patient_id: Some(&grant.patient_id),
                },
                serde_json::json!({ "grant": grant.id, "outcome": request.outcome }),
            )
            .await
            {
                return r.into_response();
            }
            if let Err(r) = commit(tx).await {
                return r.into_response();
            }
            Json(grant).into_response()
        }
        Err(e) => {
            let _ = sea_orm::TransactionSession::rollback(tx).await;
            failure(e)
        }
    }
}

/// Routes: the grant under `/api/patients`, the reviews under `/api/break-glass`.
#[must_use]
pub fn routes() -> Routes {
    Routes::new()
        .add("/api/patients/{id}/break-glass", post(open))
        .add("/api/break-glass/reviews", get(list))
        .add("/api/break-glass/reviews/count", get(count))
        .add("/api/break-glass/reviews/{id}", post(review))
}
