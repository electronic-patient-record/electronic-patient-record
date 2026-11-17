//! The work queue for refused HL7 messages (task OR-3): `GET /api/feed/refusals/count` (how many wait),
//! `GET /api/feed/refusals` (the open ones),
//! `GET /api/feed/refusals/{id}` (one, with the message as received) and
//! `POST /api/feed/refusals/{id}/resolve` (close it, with an outcome and a reason), and
//! `GET /api/feed/health` (each configured feed's state, IH-8), and
//! `POST /api/feed/dry-run` (what a message would do, keeping nothing, IH-11).
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
use crate::pool::Pool;
use axa::extract::{Json, Path};
use axa::prelude::*;
use axum::{
    Extension,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
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

/// `GET /api/feed/refusals`: the open refusals, oldest first, up to the list limit, without their message text; `more`
/// says whether any were left behind (PR-1).
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
        &ctx.pool(),
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
    let open = match queue::list_open(&ctx.pool()).await {
        Ok(o) => o,
        Err(e) => return failure(e),
    };
    if let Err(r) = audit_resource(
        &ctx.pool(),
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
    Json(serde_json::json!({ "refusals": open.rows, "limit": queue::LIST_LIMIT, "more": open.more, "site_time_zone": crate::site::time_zone() })).into_response()
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
        &ctx.pool(),
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
    let open = match queue::count_open(&ctx.pool()).await {
        Ok(n) => n,
        Err(e) => return failure(e),
    };
    if let Err(r) = audit_resource(
        &ctx.pool(),
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

/// A message to try.
#[derive(Debug, Deserialize)]
pub struct DryRunRequest {
    /// The message as the sender would send it, segments separated by `\r` (a `\n` is read as one).
    pub message: String,
}

/// The largest message a dry run takes: the listener's own frame limit.
const DRY_RUN_MAX: usize = crate::mllp::MAX_FRAME;

/// `POST /api/feed/dry-run`: run a message through the feed's real path with the listening feed's rules, in a
/// transaction that is rolled back (IH-11), and say what it would do and what the feed would answer. Nothing is kept:
/// no change, no queue entry, no count. For a clinical administrator, as for the queue; the audit row records that a
/// dry run was made and its outcome word, never the message.
async fn dry_run(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Json(request): Json<DryRunRequest>,
) -> Response {
    let purpose = match purpose_of(&headers) {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    if let Err(r) = authorize(
        &ctx.pool(),
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
    if request.message.len() > DRY_RUN_MAX {
        return ApiError::invalid(vec![format!("the message is over {DRY_RUN_MAX} bytes")])
            .into_response();
    }
    let text = request.message.replace("\r\n", "\r").replace('\n', "\r");
    let default_feed = crate::adt_feed::FeedIdentity {
        agent: "mllp:adt-feed".into(),
        peer: None,
        peer_certificate: None,
        kept_answer: crate::adt_feed::KeptAnswer::default(),
        processing_id: None,
        rules: crate::adt_feed::SenderRules::default(),
    };
    let configured = crate::adt_feed::configured();
    let who = configured.unwrap_or(&default_feed);
    let result = crate::adt_feed::dry_run(&ctx.pool(), &text, who).await;
    if let Err(r) = audit_resource(
        &ctx.pool(),
        &agent,
        Access {
            action: Action::Execute,
            outcome: 0,
            purpose,
            resource: "FeedQueue",
            patient_id: None,
        },
        serde_json::json!({ "dry_run": result.would }),
    )
    .await
    {
        return r.into_response();
    }
    Json(serde_json::json!({
        "would": result.would,
        "reasons": result.reasons,
        "warnings": result.warnings,
        "acknowledgement": result.acknowledgement,
        "feed": who.agent,
        "feed_configured": configured.is_some(),
    }))
    .into_response()
}

/// `GET /api/feed/health`: each configured feed judged now (IH-8): its state, the window, why, since when the state has
/// held (from the last stored change, if it matches), the facts and the thresholds. No message text, no patient.
/// `feeds` is empty when the listener is off. Audited like any read of the queue.
async fn health(
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
        &ctx.pool(),
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
    let now = jiff::Timestamp::now();
    let zone = crate::site::zone();
    let mut feeds = Vec::new();
    if let Some((sender, thresholds)) = crate::feed_health::configured() {
        let facts = match crate::feed_health::facts(&ctx.pool(), sender, now).await {
            Ok(f) => f,
            Err(e) => return failure(e),
        };
        let judged = crate::feed_health::assess(now, &zone, &facts, thresholds);
        let since = match crate::feed_health::last_state(&ctx.pool(), sender).await {
            Ok(Some((state, at))) if state == judged.state => Some(at),
            Ok(_) => None,
            Err(e) => return failure(e),
        };
        feeds.push(serde_json::json!({
            "sender": sender,
            "state": judged.state,
            "window": judged.window,
            "reasons": judged.reasons,
            "since": since.map(crate::clock::rfc3339_z),
            "last_message_at": facts.last_message_at.map(crate::clock::rfc3339_z),
            "refusals_last_hour": facts.refusals_last_hour,
            "open_refusals": facts.open_refusals,
            "thresholds": thresholds,
        }));
    }
    if let Err(r) = audit_resource(
        &ctx.pool(),
        &agent,
        Access {
            action: Action::Read,
            outcome: 0,
            purpose,
            resource: "FeedQueue",
            patient_id: None,
        },
        serde_json::json!({ "health": feeds.len() }),
    )
    .await
    {
        return r.into_response();
    }
    Json(serde_json::json!({ "feeds": feeds, "site_time_zone": crate::site::time_zone() }))
        .into_response()
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
        &ctx.pool(),
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
    let entry = match queue::get(&ctx.pool(), &id).await {
        Ok(e) => e,
        Err(e) => return failure(e),
    };
    // Audit before the message text leaves.
    if let Err(r) = audit_resource(
        &ctx.pool(),
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
        &ctx.pool(),
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
    let mut tx = match begin(&ctx.pool()).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    match queue::resolve(&mut *tx, &id, &agent.0, request.outcome, &request.reason).await {
        Ok(entry) => {
            if let Err(r) = audit_resource(
                &mut *tx,
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
            let _ = tx.rollback().await;
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
        .add("/health", get(health))
        .add("/dry-run", post(dry_run))
}
