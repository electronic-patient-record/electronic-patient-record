//! Result routes (OR-4 first slice, WL-1): a result's versions and their sign-off.
//!
//! - `GET  /api/patients/{id}/results`: every result at its current version, newest filed first, each
//!   with its sign-off state; a result changed since sign-off carries the version that was signed.
//! - `POST /api/patients/{id}/results`: file one.
//! - `PUT  /api/patients/{id}/results/{result_id}`: a new version (final after preliminary, amended,
//!   corrected, or entered in error), with a reason.
//! - `POST /api/patients/{id}/results/{result_id}/sign-off`: sign off the version named in the body.
//!   Naming a version that is no longer current is a 409 with the current version: the signer has
//!   not seen the change.
//!
//! Results are clinical content: the role, a care relationship (or break-the-glass for a read), the
//! purpose and the patient's consent all apply. A write and its audit row are one transaction; the row
//! holds ids, versions, statuses and counts, never a test name, value, unit or range.

use super::common::{Access, audit_resource, authorize, begin, commit, purpose_of};
use crate::api_error::ApiError;
use crate::audit::Action;
use crate::auth::Agent;
use crate::patients_store::StoreError;
use crate::policy::{self, Purpose, Resource, Subject};
use crate::pool::Pool;
use crate::results::{ResultRecord, Status};
use crate::results_store as store;
use axa::extract::{Json, Path};
use axa::prelude::*;
use axum::{
    Extension,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

/// File a result.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateRequest {
    /// The result.
    pub result: ResultRecord,
}

/// A new version of a result.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateRequest {
    /// The whole new version.
    pub result: ResultRecord,
    /// The version this change was made from; anything else is a 409.
    pub expected_version: i32,
    /// Why it changed (required).
    #[serde(default)]
    pub reason: Option<String>,
}

/// Sign off a version.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignOffRequest {
    /// The version the signer read.
    pub version: i32,
}

/// A patient's results.
#[derive(Debug, Serialize)]
pub struct ResultList {
    /// Newest filed first.
    pub results: Vec<store::ResultView>,
}

const RESOURCE: &str = "DiagnosticReport";

/// A store error as this route's answer: a refused record is a 422; anything else keeps its own status
/// (a miss is 404, a stale version 409).
fn failure(e: StoreError) -> ApiError {
    match e {
        StoreError::BadQuery(m) => ApiError::invalid(vec![m]),
        other => ApiError::from(&other),
    }
}

/// Audit a refused write (a miss, a stale version, a refused record).
async fn refused(
    ctx: &AppContext,
    agent: &Agent,
    purpose: Purpose,
    action: Action,
    e: &StoreError,
) -> Option<ApiError> {
    if !matches!(
        e,
        StoreError::NotFound | StoreError::Conflict { .. } | StoreError::BadQuery(_)
    ) {
        return None;
    }
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
        // The store's words name a rule or a version, never a value.
        serde_json::json!({ "refused": e.to_string() }),
    )
    .await
    .err()
}

/// `GET /api/patients/{id}/results`. Audited with the count, and how many are unsigned or changed.
pub(super) async fn list(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path(patient): Path<String>,
) -> Response {
    let purpose = match purpose_of(&headers) {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    if let Err(r) = authorize(
        &ctx.pool(),
        &agent,
        &subject,
        Resource::DiagnosticReport,
        policy::Action::Read,
        purpose,
        Some(&patient),
    )
    .await
    {
        return r.into_response();
    }
    let all = match store::list(&ctx.pool(), &patient).await {
        Ok(a) => a,
        Err(e) => {
            if let Some(r) = refused(&ctx, &agent, purpose, Action::Read, &e).await {
                return r.into_response();
            }
            return failure(e).into_response();
        }
    };
    let open = all
        .iter()
        .filter(|r| {
            matches!(
                r.sign_off,
                crate::results::SignOff::Unsigned | crate::results::SignOff::ChangedSinceSignOff
            )
        })
        .count();
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
        serde_json::json!({ "results": all.len(), "to_sign": open }),
    )
    .await
    {
        return r.into_response();
    }
    Json(ResultList { results: all }).into_response()
}

/// `POST /api/patients/{id}/results`: file a result. 201 with the stored first version.
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
        Err(r) => return r.into_response(),
    };
    if let Err(r) = authorize(
        &ctx.pool(),
        &agent,
        &subject,
        Resource::DiagnosticReport,
        policy::Action::Create,
        purpose,
        Some(&patient),
    )
    .await
    {
        return r.into_response();
    }
    let mut tx = match begin(&ctx.pool()).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    match store::create(&mut *tx, &patient, &agent.0, &request.result).await {
        Ok(stored) => {
            if let Err(r) = audit_resource(
                &mut *tx,
                &agent,
                Access {
                    action: Action::Create,
                    outcome: 0,
                    purpose,
                    resource: RESOURCE,
                    patient_id: Some(&stored.patient_id),
                },
                serde_json::json!({
                    "result": stored.id,
                    "version": stored.version,
                    "status": stored.result.status.word(),
                }),
            )
            .await
            {
                return r.into_response();
            }
            if let Err(r) = commit(tx).await {
                return r.into_response();
            }
            (StatusCode::CREATED, Json(stored)).into_response()
        }
        Err(e) => {
            let _ = tx.rollback().await;
            if let Some(r) = refused(&ctx, &agent, purpose, Action::Create, &e).await {
                return r.into_response();
            }
            failure(e).into_response()
        }
    }
}

/// `PUT /api/patients/{id}/results/{result_id}`: a new version. Entered in error is its own permission.
pub(super) async fn update(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path((patient, result_id)): Path<(String, String)>,
    Json(request): Json<UpdateRequest>,
) -> Response {
    let purpose = match purpose_of(&headers) {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    let needed = if request.result.status == Status::EnteredInError {
        policy::Action::MarkEnteredInError
    } else {
        policy::Action::Update
    };
    if let Err(r) = authorize(
        &ctx.pool(),
        &agent,
        &subject,
        Resource::DiagnosticReport,
        needed,
        purpose,
        Some(&patient),
    )
    .await
    {
        return r.into_response();
    }
    let mut tx = match begin(&ctx.pool()).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    match store::update(
        &mut *tx,
        &patient,
        &result_id,
        request.expected_version,
        &agent.0,
        request.reason.as_deref(),
        &request.result,
    )
    .await
    {
        Ok(stored) => {
            if let Err(r) = audit_resource(
                &mut *tx,
                &agent,
                Access {
                    action: Action::Update,
                    outcome: 0,
                    purpose,
                    resource: RESOURCE,
                    patient_id: Some(&stored.patient_id),
                },
                serde_json::json!({
                    "result": stored.id,
                    "version": stored.version,
                    "status": stored.result.status.word(),
                }),
            )
            .await
            {
                return r.into_response();
            }
            if let Err(r) = commit(tx).await {
                return r.into_response();
            }
            Json(stored).into_response()
        }
        Err(e) => {
            let _ = tx.rollback().await;
            if let Some(r) = refused(&ctx, &agent, purpose, Action::Update, &e).await {
                return r.into_response();
            }
            failure(e).into_response()
        }
    }
}

/// `POST /api/patients/{id}/results/{result_id}/sign-off`: sign off the named version. 201 with the
/// sign-off, or 200 with the existing one if that version was already signed.
pub(super) async fn sign_off(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path((patient, result_id)): Path<(String, String)>,
    Json(request): Json<SignOffRequest>,
) -> Response {
    let purpose = match purpose_of(&headers) {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    if let Err(r) = authorize(
        &ctx.pool(),
        &agent,
        &subject,
        Resource::DiagnosticReport,
        policy::Action::Update,
        purpose,
        Some(&patient),
    )
    .await
    {
        return r.into_response();
    }
    let mut tx = match begin(&ctx.pool()).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    match store::sign_off(&mut *tx, &patient, &result_id, request.version, &agent.0).await {
        Ok((signed, created)) => {
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
                    "result": signed.result_id,
                    "signed_version": signed.version,
                    "sign_off": signed.id,
                    "new": created,
                }),
            )
            .await
            {
                return r.into_response();
            }
            if let Err(r) = commit(tx).await {
                return r.into_response();
            }
            let status = if created {
                StatusCode::CREATED
            } else {
                StatusCode::OK
            };
            (status, Json(signed)).into_response()
        }
        Err(e) => {
            let _ = tx.rollback().await;
            if let Some(r) = refused(&ctx, &agent, purpose, Action::Update, &e).await {
                return r.into_response();
            }
            failure(e).into_response()
        }
    }
}
