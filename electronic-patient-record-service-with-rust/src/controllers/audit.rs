//! Audit trail endpoints (task AU-1 acceptance: a verify endpoint that detects a tampered row).
//!
//! `GET /api/audit/verify` recomputes the hash chain and reports whether it is intact. It is for
//! the `auditor` role. Verifying is itself an access to the audit log, so it writes its own audit
//! row (and answers 500 without a result if it cannot).

use super::patients::{authorize, json_error};
use crate::audit::{self, Action, Chain, Event, Head, Verdict};
use crate::auth::Agent;
use crate::policy::{self, Purpose, Resource, Subject};
use axum::{
    Extension,
    extract::Query,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use loco_rs::prelude::*;
use sea_orm::DatabaseConnection;

/// What the chain can and cannot show, by whether it is keyed and whether a checkpoint was given.
fn assurance(keyed: bool, checkpointed: bool) -> String {
    let key = if keyed {
        "keyed: a row cannot be edited or removed and the chain recomputed without the key."
    } else {
        "unkeyed (EPR_INTEGRITY_MAC_KEY is not set): an edited row is detected, but someone who \
         can write to the database can recompute a consistent chain."
    };
    let tail = if checkpointed {
        "Rows up to the checkpoint you supplied are confirmed present; rows removed after it are \
         not detected until you publish a newer one."
    } else {
        "Removing the newest rows is not detected unless you supply a checkpoint published earlier \
         (`head` in a previous answer, stored where the database's owner cannot write)."
    };
    format!("{key} {tail}")
}

/// A checkpoint the caller published from an earlier answer's `head`.
#[derive(Debug, serde::Deserialize)]
pub struct CheckpointQuery {
    /// The head row's id.
    pub checkpoint_id: Option<i64>,
    /// The head row's digest.
    pub checkpoint_digest: Option<String>,
}

impl CheckpointQuery {
    /// Both parts or neither; one alone is a malformed request.
    ///
    /// # Errors
    ///
    /// When only one of the two parts is given.
    pub fn head(self) -> Result<Option<Head>, &'static str> {
        match (self.checkpoint_id, self.checkpoint_digest) {
            (None, None) => Ok(None),
            (Some(id), Some(digest)) => Ok(Some(Head { id, digest })),
            _ => Err("checkpoint_id and checkpoint_digest go together"),
        }
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
    checkpoint: Option<&Head>,
) -> Result<(StatusCode, serde_json::Value), audit::AuditError> {
    let (verdict, head) = audit::verify_against(db, chain, audit::VERIFY_BATCH, checkpoint).await?;
    let assurance = assurance(chain.keyed(), checkpoint.is_some());
    let (status, body) = match &verdict {
        Verdict::Intact { rows } => (
            StatusCode::OK,
            serde_json::json!({
                "intact": true,
                "rows": rows,
                "keyed": chain.keyed(),
                "head": head,
                "checkpoint_confirmed": checkpoint.is_some(),
                "assurance": assurance,
            }),
        ),
        Verdict::Unchecked {
            rows,
            unchecked,
            first_id,
            causes,
        } => (
            // Not 409: nothing was found wrong, so a monitor must not read this as tampering. Not
            // 200 either: the chain was not fully checked.
            StatusCode::FAILED_DEPENDENCY,
            serde_json::json!({
                "intact": false,
                "tampering_found": false,
                "error": "key-not-held",
                "rows": rows,
                "unchecked": unchecked,
                "first_unchecked_id": first_id,
                "causes": causes,
                "keyed": chain.keyed(),
                "head": head,
                "assurance": "No row was found altered, but the rows listed could not be checked because this service does not hold the key that made their digest. This is a key problem, not evidence of tampering: configure the missing key (EPR_INTEGRITY_MAC_KEY, or EPR_INTEGRITY_MAC_KEYS_RETIRED for an old one) or upgrade the service, then verify again.",
            }),
        ),
        Verdict::Broken { id, reason } => (
            StatusCode::CONFLICT,
            serde_json::json!({
                "intact": false,
                "first_broken_id": id,
                "reason": reason,
                "keyed": chain.keyed(),
                "assurance": assurance,
            }),
        ),
    };
    let mut event = Event::new(Action::Execute, agent.0.clone());
    event.resource_ref = Some("AuditLog/$verify".to_string());
    event.purpose_of_use = Some(Purpose::Audit.code().to_string());
    event.detail = Some(match &verdict {
        Verdict::Intact { rows } => serde_json::json!({ "intact": true, "rows": rows }),
        Verdict::Broken { id, .. } => serde_json::json!({ "intact": false, "first_broken_id": id }),
        Verdict::Unchecked {
            unchecked,
            first_id,
            ..
        } => serde_json::json!({
            "intact": false,
            "tampering_found": false,
            "unchecked": unchecked,
            "first_unchecked_id": first_id,
        }),
    });
    audit::append(db, chain, &event).await?;
    Ok((status, body))
}

async fn verify(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    Query(query): Query<CheckpointQuery>,
) -> Response {
    let checkpoint = match query.head() {
        Ok(c) => c,
        Err(m) => return json_error(StatusCode::BAD_REQUEST, serde_json::json!({ "error": m })),
    };
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
    match verify_audited(&ctx.db, audit::chain(), &agent, checkpoint.as_ref()).await {
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
