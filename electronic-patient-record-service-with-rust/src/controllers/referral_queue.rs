//! Inbound referral messages and the queue of those that did not match (task PC-2, hazard H-044).
//!
//! - `POST /api/referral-messages`: a recipient system sends an outcome, a decline or an interim note,
//!   naming the referral by id. It moves the referral or is queued; it is never dropped.
//! - `GET  /api/referral-queue/count`, `GET /api/referral-queue`, `GET /api/referral-queue/{id}`: how many
//!   wait, the open ones (no text), one with the message as received.
//! - `POST /api/referral-queue/{id}/clear`: match it by hand to a named referral, or dismiss it, with a reason.
//!
//! **Assumed, awaiting the maintainer (OQ-PC2's neighbour):** the integrator role sends inbound messages and
//! the clinical administrator reads and clears the queue, as for the feed-refusal queue. The messages can hold
//! a result or a decision, so no other role reads them. No care relationship applies to either: the queue
//! belongs to no patient until it is matched. Every view, receipt and clearance is audited; the audit row names
//! ids, kinds and states, never the message, a note or a reason. A write and its audit row are one transaction.

use super::common::{Access, audit_resource, authorize, begin, commit, purpose_of, store_failure};
use crate::api_error::ApiError;
use crate::audit::Action;
use crate::auth::Agent;
use crate::patients_store::StoreError;
use crate::policy::{self, Resource, Subject};
use crate::referral_unmatched::{self as queue, Clearance, Filed, Inbound};
use axum::{
    Extension,
    extract::{Json, Path},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use loco_rs::prelude::*;
use serde::Deserialize;

/// An inbound message.
#[derive(Debug, Deserialize)]
pub struct MessageRequest {
    /// The referral it answers, as the sender wrote it. Absent or blank is queued as `no-id`.
    #[serde(default)]
    pub referral_ref: Option<String>,
    /// `outcome`, `decline`, `interim-note` and the other recipient events.
    #[serde(default)]
    pub event: Option<String>,
    /// The sender's note, for the events that need one.
    #[serde(default)]
    pub note: Option<String>,
    /// The message as received. Required.
    pub message: String,
}

/// Clear an entry.
#[derive(Debug, Deserialize)]
pub struct ClearRequest {
    /// `matched` (with `referral_id`) or `dismissed`.
    pub how: String,
    /// The referral it belongs to, for `matched`.
    #[serde(default)]
    pub referral_id: Option<String>,
    /// Why. Required.
    pub reason: String,
}

/// A store error as a response: 422 for something the caller can fix, 409 for a clearance that already happened,
/// anything else through the shared store failure.
fn failure(e: StoreError) -> Response {
    match e {
        StoreError::BadQuery(m) => ApiError::invalid(vec![m]).into_response(),
        StoreError::Conflict { .. } => {
            ApiError::new(StatusCode::CONFLICT, "conflict").into_response()
        }
        other => store_failure(&other),
    }
}

/// The shared start of every queue route: read the purpose of use and authorise `action` on the `ReferralQueue`
/// resource. No patient is named, so no care relationship applies.
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
) -> Result<crate::policy::Purpose, ApiError> {
    let purpose = purpose_of(headers)?;
    authorize(
        &ctx.db,
        agent,
        subject,
        Resource::ReferralQueue,
        action,
        purpose,
        None,
    )
    .await?;
    Ok(purpose)
}

/// `POST /api/referral-messages`: file an inbound message. It moves the referral it names, or is queued; the answer is
/// 200 either way and never repeats the message.
///
/// A blank message is a 422. A racing step is audited as a refusal and answered 409, so the sender may send again.
async fn receive(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Json(request): Json<MessageRequest>,
) -> Response {
    let purpose = match guard(&ctx, &agent, &subject, &headers, policy::Action::Create).await {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    if request.message.trim().is_empty() {
        return ApiError::invalid(vec!["the message must not be blank".into()]).into_response();
    }
    let tx = match begin(&ctx.db).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    let filed = queue::file(
        &tx,
        Inbound {
            agent: &agent.0,
            referral_ref: request.referral_ref.as_deref(),
            event: request.event.as_deref(),
            note: request.note.as_deref(),
            message: &request.message,
        },
        chrono::Utc::now(),
    )
    .await;
    match filed {
        Ok(filed) => {
            let (body, patient, detail) = match &filed {
                Filed::Moved { referral } => (
                    serde_json::json!({
                        "result": "moved", "referral": referral.id,
                        "version": referral.version, "state": referral.state.as_str(),
                    }),
                    Some(referral.patient_id.clone()),
                    serde_json::json!({
                        "result": "moved", "referral": referral.id, "version": referral.version,
                        "event": request.event, "state": referral.state.as_str(),
                    }),
                ),
                Filed::Queued { entry } => (
                    serde_json::json!({ "result": "queued", "queue_id": entry.id, "kind": entry.kind }),
                    None,
                    serde_json::json!({
                        "result": "queued", "queue_id": entry.id, "kind": entry.kind,
                    }),
                ),
            };
            if let Err(r) = audit_resource(
                &tx,
                &agent,
                Access {
                    action: Action::Create,
                    outcome: 0,
                    purpose,
                    resource: "ReferralMessage",
                    patient_id: patient.as_deref(),
                },
                detail,
            )
            .await
            {
                return r.into_response();
            }
            if let Err(r) = commit(tx).await {
                return r.into_response();
            }
            // 200 for both: the sender is told what happened, and the message is never repeated.
            Json(body).into_response()
        }
        Err(e) => {
            let _ = sea_orm::TransactionSession::rollback(tx).await;
            // A racing step: the sender may send again. Audited like any refused write.
            if matches!(e, StoreError::Conflict { .. })
                && let Err(r) = audit_resource(
                    &ctx.db,
                    &agent,
                    Access {
                        action: Action::Create,
                        outcome: 4,
                        purpose,
                        resource: "ReferralMessage",
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

/// `GET /api/referral-queue`: the open entries, oldest first, up to the list limit, without their text. Audited.
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
            resource: "ReferralQueue",
            patient_id: None,
        },
        serde_json::json!({ "open": open.len() }),
    )
    .await
    {
        return r.into_response();
    }
    Json(serde_json::json!({
        "entries": open, "limit": queue::LIST_LIMIT, "site_time_zone": crate::site::time_zone()
    }))
    .into_response()
}

/// `GET /api/referral-queue/count`: how many entries are open. Audited like any other search of the queue.
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
            resource: "ReferralQueue",
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

/// `GET /api/referral-queue/{id}`: one entry with the note and message as received. Audited before the text leaves.
async fn read(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let purpose = match guard(&ctx, &agent, &subject, &headers, policy::Action::Read).await {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
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
            resource: "ReferralQueue",
            patient_id: None,
        },
        serde_json::json!({ "entry": entry.id, "kind": entry.kind }),
    )
    .await
    {
        return r.into_response();
    }
    Json(entry).into_response()
}

/// `POST /api/referral-queue/{id}/clear`: match an entry by hand to a named referral, or dismiss it. A reason is
/// required and an entry clears once (409 after). The answer does not repeat the message.
async fn clear(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<ClearRequest>,
) -> Response {
    let purpose = match guard(&ctx, &agent, &subject, &headers, policy::Action::Update).await {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    let how = match (request.how.as_str(), request.referral_id.as_deref()) {
        ("matched", Some(referral)) => Clearance::Matched(referral),
        ("matched", None) => {
            return ApiError::invalid(vec!["a hand match names the referral".into()])
                .into_response();
        }
        ("dismissed", None) => Clearance::Dismissed,
        ("dismissed", Some(_)) => {
            return ApiError::invalid(vec!["a dismissal names no referral".into()]).into_response();
        }
        _ => {
            return ApiError::invalid(vec!["how is matched or dismissed".into()]).into_response();
        }
    };
    let tx = match begin(&ctx.db).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    match queue::clear(&tx, &id, &agent.0, how, &request.reason).await {
        Ok(entry) => {
            let cleared = entry.cleared.clone();
            if let Err(r) = audit_resource(
                &tx,
                &agent,
                Access {
                    action: Action::Update,
                    outcome: 0,
                    purpose,
                    resource: "ReferralQueue",
                    patient_id: None,
                },
                // The way and the two ids, never the free-text reason or the message.
                serde_json::json!({
                    "entry": entry.id,
                    "how": cleared.as_ref().map(|c| c.how.clone()),
                    "referral": cleared.and_then(|c| c.referral_id),
                }),
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
            entry.note = None;
            Json(entry).into_response()
        }
        Err(e) => {
            let _ = sea_orm::TransactionSession::rollback(tx).await;
            failure(e)
        }
    }
}

/// Routes for inbound referral messages and their queue.
#[must_use]
pub fn routes() -> Routes {
    Routes::new()
        .add("/api/referral-messages", post(receive))
        .add("/api/referral-queue", get(list))
        .add("/api/referral-queue/count", get(count))
        .add("/api/referral-queue/{id}", get(read))
        .add("/api/referral-queue/{id}/clear", post(clear))
}
