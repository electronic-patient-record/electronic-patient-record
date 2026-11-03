//! The referrer's worklist (hazard H-044): what the recipient has said back that the referrer has not yet seen.
//!
//! - `GET  /api/referrals/worklist`: the caller's own referrals waiting for them, oldest first, ids and states
//!   only, with `limit` and `more` (PR-1).
//! - `GET  /api/referrals/worklist/count`: how many.
//! - `POST /api/referrals/{referral_id}/acknowledge`: `{version}`, "I have seen this version". Once only.
//!
//! **Assumed, awaiting the maintainer (OQ-PC5):** the referrer is whoever sent version 1, and the list is theirs
//! alone. Authorised as the `Referral` resource by role and purpose; no patient is named, so no care relationship
//! is checked *here*: the list holds only referrals the caller sent, carries no clinical text, and the referral
//! itself opens through `/api/patients/{id}/referrals`, which applies the full record-level decision. Every
//! view and acknowledgement is audited with ids, counts and states, never text. A write and its audit row are
//! one transaction.

use super::common::{Access, audit_resource, authorize, begin, commit, purpose_of, store_failure};
use crate::api_error::ApiError;
use crate::audit::Action;
use crate::auth::Agent;
use crate::patients_store::StoreError;
use crate::policy::{self, Purpose, Resource, Subject};
use crate::referral_worklist as worklist;
use axum::{
    Extension,
    extract::{Json, Path},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use loco_rs::prelude::*;
use serde::Deserialize;

/// "I have seen this version."
#[derive(Debug, Deserialize)]
pub struct AcknowledgeRequest {
    /// The version being acknowledged; anything but the current one is a 409.
    pub version: i32,
}

/// A store error as a response: 422 for something the caller can fix, 409 for a stale or repeated acknowledgement.
fn failure(e: StoreError) -> Response {
    match e {
        StoreError::BadQuery(m) => ApiError::invalid(vec![m]).into_response(),
        StoreError::Conflict { .. } => {
            ApiError::new(StatusCode::CONFLICT, "conflict").into_response()
        }
        other => store_failure(&other),
    }
}

/// Read the purpose of use and authorise `action` on `Referral` with no patient named.
///
/// # Errors
///
/// The purpose header is unknown (400), or the role, purpose or policy refuses (403, audited by `authorize`).
async fn guard(
    ctx: &AppContext,
    agent: &Agent,
    subject: &Subject,
    headers: &HeaderMap,
    action: policy::Action,
) -> Result<Purpose, ApiError> {
    let purpose = purpose_of(headers)?;
    authorize(
        &ctx.db,
        agent,
        subject,
        Resource::ReferralWorklist,
        action,
        purpose,
        None,
    )
    .await?;
    Ok(purpose)
}

/// `GET /api/referrals/worklist`: the caller's referrals that wait for them. Audited with the count.
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
    let page = match worklist::list(&ctx.db, &agent.0).await {
        Ok(p) => p,
        Err(e) => return failure(e),
    };
    if let Err(r) = audit_resource(
        &ctx.db,
        &agent,
        Access {
            action: Action::Read,
            outcome: 0,
            purpose,
            resource: "ReferralWorklist",
            patient_id: None,
        },
        serde_json::json!({ "waiting": page.len() }),
    )
    .await
    {
        return r.into_response();
    }
    Json(serde_json::json!({
        "items": page.rows, "limit": worklist::LIST_LIMIT, "more": page.more,
        "site_time_zone": crate::site::time_zone()
    }))
    .into_response()
}

/// `GET /api/referrals/worklist/count`: how many wait. Audited like the list.
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
    let waiting = match worklist::count(&ctx.db, &agent.0).await {
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
            resource: "ReferralWorklist",
            patient_id: None,
        },
        serde_json::json!({ "count": waiting }),
    )
    .await
    {
        return r.into_response();
    }
    Json(serde_json::json!({ "open": waiting })).into_response()
}

/// `POST /api/referrals/{referral_id}/acknowledge`: the referrer has seen `version`. A referral the caller did not
/// send is a 404; a version that is no longer current, or was already acknowledged, is a 409.
async fn acknowledge(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path(referral_id): Path<String>,
    Json(request): Json<AcknowledgeRequest>,
) -> Response {
    let purpose = match guard(&ctx, &agent, &subject, &headers, policy::Action::Update).await {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    let tx = match begin(&ctx.db).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    match worklist::acknowledge(&tx, &referral_id, request.version, &agent.0).await {
        Ok(()) => {
            if let Err(r) = audit_resource(
                &tx,
                &agent,
                Access {
                    action: Action::Update,
                    outcome: 0,
                    purpose,
                    resource: "ReferralWorklist",
                    patient_id: None,
                },
                serde_json::json!({ "referral": referral_id, "version": request.version }),
            )
            .await
            {
                return r.into_response();
            }
            if let Err(r) = commit(tx).await {
                return r.into_response();
            }
            Json(serde_json::json!({ "referral": referral_id, "version": request.version, "acknowledged": true }))
                .into_response()
        }
        Err(e) => {
            let _ = sea_orm::TransactionSession::rollback(tx).await;
            if matches!(
                e,
                StoreError::NotFound | StoreError::Conflict { .. } | StoreError::BadQuery(_)
            ) && let Err(r) = audit_resource(
                &ctx.db,
                &agent,
                Access {
                    action: Action::Update,
                    outcome: 4,
                    purpose,
                    resource: "ReferralWorklist",
                    patient_id: None,
                },
                serde_json::json!({ "refused": e.to_string() }),
            )
            .await
            {
                return r.into_response();
            }
            failure(e)
        }
    }
}

/// The worklist routes.
pub fn routes() -> Routes {
    Routes::new()
        .add("/api/referrals/worklist", get(list))
        .add("/api/referrals/worklist/count", get(count))
        .add(
            "/api/referrals/{referral_id}/acknowledge",
            post(acknowledge),
        )
}
