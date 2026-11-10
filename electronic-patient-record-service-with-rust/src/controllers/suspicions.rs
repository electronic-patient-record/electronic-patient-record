//! Suspicion routes (RS-1): what the referrer suspects, on one referral, under a pathway (RS-6).
//!
//! - `GET  /api/patients/{id}/referrals/{referral_id}/suspicion`: every version, oldest first, and the
//!   current one, each with its pathway's plan (priority, target due days, overdue).
//! - `POST …/suspicion` `{suspicion}`: raise it (version 1); 409 if the referral already carries one.
//! - `PUT  …/suspicion` `{suspicion, expected_version, reason}`: amend it; a reason is required, an
//!   amendment that changes nothing is refused, a stale version is 409; after an outcome, 422.
//! - `POST …/suspicion/outcome` `{outcome, expected_version}`: end it (RS-3): confirmed with the
//!   diagnosis, ruled out with who and on what, or withdrawn with a reason. Once only. Nothing else ends a
//!   suspicion: not its referral closing, a cancellation, a redirect, or a missed test or appointment.
//!
//! A suspicion is a provisional diagnosis, so it is authorised as `Condition`: the role, a care
//! relationship, the purpose and the patient's consent apply. **A patient or proxy reads none** (purpose
//! `patient-access` is refused here) until RS-4 decides what a patient sees of a suspicion they have not
//! been told. The write and its audit row are one transaction; the row holds ids, versions and the names of
//! the fields changed, never the pathway (its id names the condition), the condition, grounds, factors or
//! reason.

use super::common::{Access, audit_resource, authorize, begin, commit, purpose_of};
use crate::api_error::ApiError;
use crate::audit::Action;
use crate::auth::Agent;
use crate::patients_store::StoreError;
use crate::policy::{self, Purpose, Resource, Subject};
use crate::pool::Pool;
use crate::suspicion::{Outcome, Suspicion};
use crate::suspicions_store::{self as store, StoredSuspicion};
use axa::extract::{Json, Path};
use axa::prelude::*;
use axum::{
    Extension,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;

/// Raise a suspicion.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RaiseRequest {
    /// The suspicion.
    pub suspicion: Suspicion,
}

/// Amend a suspicion.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AmendRequest {
    /// The whole suspicion, as it should now read.
    pub suspicion: Suspicion,
    /// The version this was made from; anything else is a 409.
    pub expected_version: i32,
    /// Why it changes.
    #[serde(default)]
    pub reason: Option<String>,
}

/// End a suspicion.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EndRequest {
    /// The outcome.
    pub outcome: Outcome,
    /// The suspicion version this ends; anything else is a 409.
    pub expected_version: i32,
}

/// Mark a suspicion's outcome entered in error (RS-7).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarkInErrorRequest {
    /// The outcome's `seq`; anything but the current outcome is a 409.
    pub seq: i32,
    /// Why it was entered in error.
    pub reason: String,
}

const RESOURCE: &str = "Condition";

fn failure(e: StoreError) -> ApiError {
    match e {
        StoreError::BadQuery(m) => ApiError::invalid(vec![m]),
        other => ApiError::from(&other),
    }
}

/// Audit a refusal by kind only: the store's words can name a field or pathway, never its value, but the
/// kind is all the trail needs.
async fn refused(
    ctx: &AppContext,
    agent: &Agent,
    purpose: Purpose,
    action: Action,
    kind: &str,
) -> Option<ApiError> {
    audit_resource(
        &ctx.pool(),
        agent,
        Access {
            action,
            outcome: 4,
            purpose,
            resource: RESOURCE,
            patient_id: None,
        },
        serde_json::json!({ "refused": kind, "suspicion": true }),
    )
    .await
    .err()
}

fn kind_of(e: &StoreError) -> Option<&'static str> {
    match e {
        StoreError::NotFound => Some("not_found"),
        StoreError::Conflict { .. } => Some("version_conflict"),
        StoreError::BadQuery(_) => Some("suspicion_refused"),
        _ => None,
    }
}

/// The purpose, the patient-access refusal, and the policy decision, in that order.
async fn gate(
    ctx: &AppContext,
    agent: &Agent,
    subject: &Subject,
    headers: &HeaderMap,
    patient: &str,
    action: policy::Action,
    audit: Action,
) -> Result<Purpose, ApiError> {
    let purpose = purpose_of(headers)?;
    if purpose == Purpose::PatientAccess {
        if let Some(r) = refused(ctx, agent, purpose, audit, "patient_access").await {
            return Err(r);
        }
        return Err(ApiError::forbidden().with(
            "reason",
            "a patient's view of a suspicion is not built (RS-4)",
        ));
    }
    authorize(
        &ctx.pool(),
        agent,
        subject,
        Resource::Condition,
        action,
        purpose,
        Some(patient),
    )
    .await?;
    Ok(purpose)
}

fn written(stored: &StoredSuspicion, referral_id: &str) -> serde_json::Value {
    serde_json::json!({
        "suspicion": true,
        "referral": referral_id,
        "version": stored.version,
        "changed": stored.changed,
    })
}

/// `GET …/referrals/{referral_id}/suspicion`. Audited with the count of versions.
pub(super) async fn history(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path((patient, referral_id)): Path<(String, String)>,
) -> Response {
    let purpose = match gate(
        &ctx,
        &agent,
        &subject,
        &headers,
        &patient,
        policy::Action::Read,
        Action::Read,
    )
    .await
    {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    let history =
        match store::history(&ctx.pool(), &patient, &referral_id, chrono::Utc::now()).await {
            Ok(h) => h,
            Err(e) => {
                if let Some(kind) = kind_of(&e)
                    && let Some(r) = refused(&ctx, &agent, purpose, Action::Read, kind).await
                {
                    return r.into_response();
                }
                return failure(e).into_response();
            }
        };
    if let Err(r) = audit_resource(
        &ctx.pool(),
        &agent,
        Access {
            action: Action::Read,
            outcome: 0,
            purpose,
            resource: RESOURCE,
            patient_id: Some(&patient),
        },
        serde_json::json!({ "suspicion": true, "referral": referral_id, "versions": history.versions.len() }),
    )
    .await
    {
        return r.into_response();
    }
    // The newest pathway versions ride along, as an assessment list carries its forms, so a screen can
    // offer the bands, factors, subtypes and agreement a new suspicion needs. They hold no patient data.
    let mut body = serde_json::to_value(&history).unwrap_or_default();
    body["pathways"] = serde_json::to_value(crate::pathways::newest()).unwrap_or_default();
    Json(body).into_response()
}

/// `POST …/referrals/{referral_id}/suspicion`: 201 with version 1 and its plan.
pub(super) async fn raise(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path((patient, referral_id)): Path<(String, String)>,
    Json(request): Json<RaiseRequest>,
) -> Response {
    let purpose = match gate(
        &ctx,
        &agent,
        &subject,
        &headers,
        &patient,
        policy::Action::Create,
        Action::Create,
    )
    .await
    {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    let mut tx = match begin(&ctx.pool()).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    let result = store::raise(
        &mut *tx,
        &patient,
        &referral_id,
        &agent.0,
        &request.suspicion,
        chrono::Utc::now(),
    )
    .await;
    finish(
        &ctx,
        &agent,
        purpose,
        Action::Create,
        tx,
        result,
        &referral_id,
        StatusCode::CREATED,
    )
    .await
}

/// `PUT …/referrals/{referral_id}/suspicion`: 200 with the new version and its plan.
pub(super) async fn amend(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path((patient, referral_id)): Path<(String, String)>,
    Json(request): Json<AmendRequest>,
) -> Response {
    let purpose = match gate(
        &ctx,
        &agent,
        &subject,
        &headers,
        &patient,
        policy::Action::Update,
        Action::Update,
    )
    .await
    {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    let mut tx = match begin(&ctx.pool()).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    let result = store::amend(
        &mut *tx,
        &patient,
        &referral_id,
        &agent.0,
        request.expected_version,
        request.reason.as_deref(),
        &request.suspicion,
        chrono::Utc::now(),
    )
    .await;
    finish(
        &ctx,
        &agent,
        purpose,
        Action::Update,
        tx,
        result,
        &referral_id,
        StatusCode::OK,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn finish(
    ctx: &AppContext,
    agent: &Agent,
    purpose: Purpose,
    action: Action,
    mut tx: sqlx::Transaction<'static, sqlx::Postgres>,
    result: Result<StoredSuspicion, StoreError>,
    referral_id: &str,
    status: StatusCode,
) -> Response {
    match result {
        Ok(stored) => {
            if let Err(r) = audit_resource(
                &mut *tx,
                agent,
                Access {
                    action,
                    outcome: 0,
                    purpose,
                    resource: RESOURCE,
                    patient_id: Some(&stored.patient_id),
                },
                written(&stored, referral_id),
            )
            .await
            {
                return r.into_response();
            }
            if let Err(r) = commit(tx).await {
                return r.into_response();
            }
            (status, Json(stored)).into_response()
        }
        Err(e) => {
            let _ = tx.rollback().await;
            if let Some(kind) = kind_of(&e)
                && let Some(r) = refused(ctx, agent, purpose, action, kind).await
            {
                return r.into_response();
            }
            failure(e).into_response()
        }
    }
}

/// `POST …/referrals/{referral_id}/suspicion/outcome`: 201 with the whole history, now ended. The audit
/// row names the outcome's kind and the version it ended, never the diagnosis or its grounds.
pub(super) async fn end(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path((patient, referral_id)): Path<(String, String)>,
    Json(request): Json<EndRequest>,
) -> Response {
    let purpose = match gate(
        &ctx,
        &agent,
        &subject,
        &headers,
        &patient,
        policy::Action::Update,
        Action::Update,
    )
    .await
    {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    let mut tx = match begin(&ctx.pool()).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    match store::end(
        &mut *tx,
        &patient,
        &referral_id,
        &agent.0,
        request.expected_version,
        &request.outcome,
        chrono::Utc::now(),
    )
    .await
    {
        Ok(history) => {
            if let Err(r) = audit_resource(
                &mut *tx,
                &agent,
                Access {
                    action: Action::Update,
                    outcome: 0,
                    purpose,
                    resource: RESOURCE,
                    patient_id: Some(&patient),
                },
                serde_json::json!({
                    "suspicion": true,
                    "referral": referral_id,
                    "version": request.expected_version,
                    "ended": request.outcome.kind.word(),
                }),
            )
            .await
            {
                return r.into_response();
            }
            if let Err(r) = commit(tx).await {
                return r.into_response();
            }
            (StatusCode::CREATED, Json(history)).into_response()
        }
        Err(e) => {
            let _ = tx.rollback().await;
            if let Some(kind) = kind_of(&e)
                && let Some(r) = refused(&ctx, &agent, purpose, Action::Update, kind).await
            {
                return r.into_response();
            }
            failure(e).into_response()
        }
    }
}

/// `POST …/referrals/{referral_id}/suspicion/outcome/entered-in-error`: 200 with the whole history, the
/// suspicion open again (RS-7). The audit row names the outcome's number, never its kind, the diagnosis
/// or the reason.
pub(super) async fn mark_outcome_in_error(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path((patient, referral_id)): Path<(String, String)>,
    Json(request): Json<MarkInErrorRequest>,
) -> Response {
    let purpose = match gate(
        &ctx,
        &agent,
        &subject,
        &headers,
        &patient,
        policy::Action::Update,
        Action::Update,
    )
    .await
    {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    let mut tx = match begin(&ctx.pool()).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    match store::mark_outcome_in_error(
        &mut *tx,
        &patient,
        &referral_id,
        &agent.0,
        request.seq,
        &request.reason,
        chrono::Utc::now(),
    )
    .await
    {
        Ok(history) => {
            if let Err(r) = audit_resource(
                &mut *tx,
                &agent,
                Access {
                    action: Action::Update,
                    outcome: 0,
                    purpose,
                    resource: RESOURCE,
                    patient_id: Some(&patient),
                },
                serde_json::json!({
                    "suspicion": true,
                    "referral": referral_id,
                    "outcome_entered_in_error": request.seq,
                }),
            )
            .await
            {
                return r.into_response();
            }
            if let Err(r) = commit(tx).await {
                return r.into_response();
            }
            (StatusCode::OK, Json(history)).into_response()
        }
        Err(e) => {
            let _ = tx.rollback().await;
            if let Some(kind) = kind_of(&e)
                && let Some(r) = refused(&ctx, &agent, purpose, Action::Update, kind).await
            {
                return r.into_response();
            }
            failure(e).into_response()
        }
    }
}
