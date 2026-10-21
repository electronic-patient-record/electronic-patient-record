//! Audit trail endpoints (task AU-1 acceptance: a verify endpoint that detects a tampered row).
//!
//! `GET /api/audit/verify` recomputes the hash chain and reports whether it is intact. It is for
//! the `auditor` role. Verifying is itself an access to the audit log, so it writes its own audit
//! row (and answers 500 without a result if it cannot).

use super::patients::{authorize, json_error};
use crate::audit::{self, Action, Chain, Event, Verdict};
use crate::auth::Agent;
use crate::policy::{self, Purpose, Resource, Subject};
use axum::{
    Extension,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use loco_rs::prelude::*;
use sea_orm::DatabaseConnection;

/// What the chain can and cannot show, by whether it is keyed.
fn assurance(keyed: bool) -> &'static str {
    if keyed {
        "keyed: a row cannot be edited or removed and the chain recomputed without the key. \
         Removing the newest rows is not detected."
    } else {
        "unkeyed (EPR_INTEGRITY_MAC_KEY is not set): an edited row is detected, but someone who \
         can write to the database can recompute a consistent chain. Removing the newest rows is \
         not detected."
    }
}

/// Verify the chain and audit the verification, returning the HTTP status and body. A broken chain
/// is `409`, not `200`, so a monitor that only looks at the status still notices.
///
/// # Errors
///
/// When the rows cannot be read or the audit row cannot be written.
pub async fn verify_audited(
    db: &DatabaseConnection,
    chain: &Chain,
    agent: &Agent,
) -> Result<(StatusCode, serde_json::Value), audit::AuditError> {
    let verdict = audit::verify(db, chain).await?;
    let (status, body) = match &verdict {
        Verdict::Intact { rows } => (
            StatusCode::OK,
            serde_json::json!({
                "intact": true,
                "rows": rows,
                "keyed": chain.keyed(),
                "assurance": assurance(chain.keyed()),
            }),
        ),
        Verdict::Broken { id, reason } => (
            StatusCode::CONFLICT,
            serde_json::json!({
                "intact": false,
                "first_broken_id": id,
                "reason": reason,
                "keyed": chain.keyed(),
                "assurance": assurance(chain.keyed()),
            }),
        ),
    };
    let mut event = Event::new(Action::Execute, agent.0.clone());
    event.resource_ref = Some("AuditLog/$verify".to_string());
    event.purpose_of_use = Some(Purpose::Audit.code().to_string());
    event.detail = Some(match &verdict {
        Verdict::Intact { rows } => serde_json::json!({ "intact": true, "rows": rows }),
        Verdict::Broken { id, .. } => serde_json::json!({ "intact": false, "first_broken_id": id }),
    });
    audit::append(db, chain, &event).await?;
    Ok((status, body))
}

async fn verify(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
) -> Response {
    // Verifying the audit trail is, by its nature, for the purpose `audit`.
    if let Err(r) = authorize(
        &ctx.db,
        &agent,
        &subject,
        Resource::AuditLog,
        policy::Action::Read,
        Purpose::Audit,
        None,
    )
    .await
    {
        return *r;
    }
    match verify_audited(&ctx.db, audit::chain(), &agent).await {
        Ok((status, body)) => (status, axum::Json(body)).into_response(),
        Err(e) => {
            tracing::error!(error = %e, "audit verification failed");
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({"error": "internal error"}),
            )
        }
    }
}

/// Routes under `/api/audit`.
#[must_use]
pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/audit")
        .add("/verify", get(verify))
}
