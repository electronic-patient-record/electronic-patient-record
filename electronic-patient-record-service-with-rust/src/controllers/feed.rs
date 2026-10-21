//! The work queue for refused HL7 messages (task OR-3): `GET /api/feed/refusals/count` (how many wait),
//! `GET /api/feed/refusals` (the open ones),
//! `GET /api/feed/refusals/{id}` (one, with the message as received) and
//! `POST /api/feed/refusals/{id}/resolve` (close it, with an outcome and a reason).
//!
//! The messages are patient data that belong to no patient on record, so the policy table gives
//! the `FeedQueue` resource to the clinical administrator only and no care relationship applies.
//! Every view and every resolution is audited; the audit row names the refusal, never its text.
//! A listing carries no message text. A resolution and its audit row are one transaction.

use super::patients::{
    Access, audit_resource, authorize, begin, commit, json_error, purpose_of, store_failure,
};
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

fn failure(e: StoreError) -> Response {
    match e {
        StoreError::BadQuery(m) => json_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            serde_json::json!({ "errors": [m] }),
        ),
        StoreError::Conflict { .. } => json_error(
            StatusCode::CONFLICT,
            serde_json::json!({ "error": "already resolved" }),
        ),
        other => store_failure(&other),
    }
}

async fn list(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
) -> Response {
    let purpose = match purpose_of(&headers) {
        Ok(p) => p,
        Err(r) => return *r,
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
        return *r;
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
        return *r;
    }
    Json(serde_json::json!({ "refusals": open, "limit": queue::LIST_LIMIT })).into_response()
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
        Err(r) => return *r,
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
        return *r;
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
        return *r;
    }
    Json(serde_json::json!({ "open": open })).into_response()
}

async fn read(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let purpose = match purpose_of(&headers) {
        Ok(p) => p,
        Err(r) => return *r,
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
        return *r;
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
        return *r;
    }
    Json(entry).into_response()
}

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
        Err(r) => return *r,
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
        return *r;
    }
    let tx = match begin(&ctx.db).await {
        Ok(t) => t,
        Err(r) => return *r,
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
                return *r;
            }
            if let Err(r) = commit(tx).await {
                return *r;
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
