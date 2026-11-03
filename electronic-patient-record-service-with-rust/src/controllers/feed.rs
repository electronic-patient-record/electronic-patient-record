//! The work queue for refused HL7 messages (task OR-3): `GET /api/feed/refusals/count` (how many wait),
//! `GET /api/feed/refusals` (the open ones),
//! `GET /api/feed/refusals/{id}` (one, with the message as received) and
//! `POST /api/feed/refusals/{id}/resolve` (close it, with an outcome and a reason).
//!
//! The messages are patient data that belong to no patient on record, so the policy table gives
//! the `FeedQueue` resource to the clinical administrator only and no care relationship applies.
//! Every view and every resolution is audited; the audit row names the refusal, never its text.
//! A listing carries no message text. A resolution and its audit row are one transaction.

use super::common::{Access, audit_resource, authorize, begin, commit, purpose_of, store_failure};
use crate::api_error::ApiError;
use crate::audit::Action;
use crate::auth::Agent;
use crate::feed_queue::{self as queue, Outcome};
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

/// Close a refusal.
#[derive(Debug, Deserialize)]
pub struct ResolveRequest {
    /// `handled` or `dismissed`.
    pub outcome: Outcome,
    /// Why. Required.
    pub reason: String,
}

/// A store error as a response: 422 for something the caller can fix, 409 for a refusal already resolved, anything else
/// through the shared store failure.
fn failure(e: StoreError) -> Response {
    match e {
        StoreError::BadQuery(m) => ApiError::invalid(vec![m.to_string()]).into_response(),
        StoreError::Conflict { .. } => {
            ApiError::new(StatusCode::CONFLICT, "already resolved").into_response()
        }
        other => store_failure(&other),
    }
}

/// `GET /api/feed/refusals`: the open refusals, oldest first, up to the list limit, without their message text.
/// The list does not say when it was cut short (PR-1). Audited.
async fn list(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
) -> Response {
    let purpose = match purpose_of(&headers) {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    if let Err(r) = authorize(
        &ctx.db,
        &agent,
        &subject,
        Resource::FeedQueue,
        policy::Action::Search,
        purpose,
        None,
    )
    .await
    {
        return r.into_response();
    }
    let open = match queue::list_open(&ctx.db).await {
        Ok(o) => o,
        Err(e) => return failure(e),
    };
    if let Err(r) = audit_resource(
        &ctx.db,
        &agent,
        Access {
            action: Action::Read,
            outcome: 0,
            purpose,
            resource: "FeedQueue",
            patient_id: None,
        },
        serde_json::json!({ "open": open.len() }),
    )
    .await
    {
        return r.into_response();
    }
    Json(serde_json::json!({ "refusals": open, "limit": queue::LIST_LIMIT, "site_time_zone": crate::site::time_zone() })).into_response()
}

/// How many are waiting, for the admin screens' navigation. No message text, no ids. Audited like
/// any other search of the queue.
async fn count(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
) -> Response {
    let purpose = match purpose_of(&headers) {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    if let Err(r) = authorize(
        &ctx.db,
        &agent,
        &subject,
        Resource::FeedQueue,
        policy::Action::Search,
        purpose,
        None,
    )
    .await
    {
        return r.into_response();
    }
    let open = match queue::count_open(&ctx.db).await {
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
            resource: "FeedQueue",
            patient_id: None,
        },
        serde_json::json!({ "count": open }),
    )
    .await
    {
        return r.into_response();
    }
    Json(serde_json::json!({ "open": open })).into_response()
}

/// `GET /api/feed/refusals/{id}`: one refusal with the message as received. Audited before the text leaves; the audit
/// row names the refusal, its HL7 event and kind, never the message.
async fn read(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let purpose = match purpose_of(&headers) {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    if let Err(r) = authorize(
        &ctx.db,
        &agent,
        &subject,
        Resource::FeedQueue,
        policy::Action::Read,
        purpose,
        None,
    )
    .await
    {
        return r.into_response();
    }
    let entry = match queue::get(&ctx.db, &id).await {
        Ok(e) => e,
        Err(e) => return failure(e),
    };
    // Audit before the message text leaves.
    if let Err(r) = audit_resource(
        &ctx.db,
        &agent,
        Access {
            action: Action::Read,
            outcome: 0,
            purpose,
            resource: "FeedQueue",
            patient_id: None,
        },
        serde_json::json!({ "refusal": entry.id, "event": entry.event, "kind": entry.kind }),
    )
    .await
    {
        return r.into_response();
    }
    Json(entry).into_response()
}

/// `POST /api/feed/refusals/{id}/resolve`: close a refusal with an outcome and a reason (required). A refusal closes
/// once (409 after). The answer does not repeat the message text.
async fn resolve(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<ResolveRequest>,
) -> Response {
    let purpose = match purpose_of(&headers) {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    if let Err(r) = authorize(
        &ctx.db,
        &agent,
        &subject,
        Resource::FeedQueue,
        policy::Action::Update,
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
    match queue::resolve(&tx, &id, &agent.0, request.outcome, &request.reason).await {
        Ok(entry) => {
            if let Err(r) = audit_resource(
                &tx,
                &agent,
                Access {
                    action: Action::Update,
                    outcome: 0,
                    purpose,
                    resource: "FeedQueue",
                    patient_id: None,
                },
                // The outcome, never the free-text reason or the message.
                serde_json::json!({ "refusal": entry.id, "outcome": request.outcome }),
            )
            .await
            {
                return r.into_response();
            }
            if let Err(r) = commit(tx).await {
                return r.into_response();
            }
            // The message text is not repeated in the answer to a write.
            let mut entry = entry;
            entry.message = None;
            Json(entry).into_response()
        }
        Err(e) => {
            let _ = sea_orm::TransactionSession::rollback(tx).await;
            failure(e)
        }
    }
}

/// Routes under `/api/feed`.
#[must_use]
pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/feed")
        .add("/refusals", get(list))
        .add("/refusals/count", get(count))
        .add("/refusals/{id}", get(read))
        .add("/refusals/{id}/resolve", post(resolve))
}
