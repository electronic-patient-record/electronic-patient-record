//! Reasonable adjustment routes (PA-6). The rules are in [`crate::adjustments`], the store in
//! [`crate::adjustments_store`], and the spec is `spec/reasonable-adjustments/index.md`.
//!
//! - `GET  /api/patients/{id}/adjustments`: every adjustment, ended ones included.
//! - `POST /api/patients/{id}/adjustments`: record one, `{code, note?, review_by?, reason}`.
//! - `PUT  /api/patients/{id}/adjustments/{adjustment_id}`: review or end one,
//!   `{expected_version, action, review_by?, reason}`.
//!
//! Any member of staff who meets patients may read and write these, with no care relationship, because
//! reception needs to know before the person arrives. A patient reads and corrects their own, and a
//! proxy with the `adjustments` scope reads, each only through their own relationship. **Who is
//! acting is the token, never the body.** A write and its audit row are one transaction; the audit row
//! holds ids, versions and statuses, never a code, a note or a reason.

use super::common::{Access, audit_resource, authorize, begin, commit, purpose_of};
use crate::adjustments::{self, Change};
use crate::adjustments_store::{self as store, StoredAdjustment};
use crate::api_error::ApiError;
use crate::audit::Action;
use crate::auth::Agent;
use crate::patients_store::StoreError;
use crate::policy::{self, Purpose, Resource, Subject};
use axa::prelude::*;
use axum::{
    Extension,
    extract::{Json, Path},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Record an adjustment.
#[derive(Debug, Deserialize)]
pub struct RecordRequest {
    /// A code from the catalogue.
    pub code: String,
    /// A short note. A spoken interpreter needs the language here.
    pub note: Option<String>,
    /// The day by which to check it with the person again.
    pub review_by: Option<String>,
    /// Why it is being recorded, such as who said so. Required.
    pub reason: String,
}

/// What a change does.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChangeAction {
    /// It was checked with the person and still applies.
    Review,
    /// It no longer applies.
    End,
}

/// One change to an adjustment.
#[derive(Debug, Deserialize)]
pub struct ChangeRequest {
    /// The version this change is made on.
    pub expected_version: i32,
    /// What it does.
    pub action: ChangeAction,
    /// For a review, the next date, if one is set.
    pub review_by: Option<String>,
    /// Why. Required.
    pub reason: String,
}

/// Every adjustment for a patient.
#[derive(Debug, Serialize)]
pub struct AdjustmentList {
    /// Newest first, ended ones included.
    pub adjustments: Vec<StoredAdjustment>,
}

/// A store error as this route's answer: a refusal is a 422 saying what to fix; anything else keeps its
/// own status (a miss is 404, a stale version 409).
fn failure(e: StoreError) -> ApiError {
    match e {
        StoreError::BadQuery(m) => ApiError::invalid(vec![m]),
        other => ApiError::from(&other),
    }
}

/// Audit a refusal of a write or a read (a miss, a stale version, a refused change).
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
        &ctx.db,
        agent,
        Access {
            action,
            outcome: 4,
            purpose,
            resource: "Adjustment",
            patient_id: None,
        },
        serde_json::json!({ "refused": e.to_string() }),
    )
    .await
    .err()
}

/// The list, as a value, for the JSON route and the FHIR search: one decision, one audit row.
pub(super) async fn list_service(
    ctx: &AppContext,
    agent: &Agent,
    subject: &Subject,
    headers: &HeaderMap,
    patient: &str,
) -> Result<AdjustmentList, ApiError> {
    let purpose = purpose_of(headers)?;
    authorize(
        &ctx.db,
        agent,
        subject,
        Resource::Adjustment,
        policy::Action::Search,
        purpose,
        Some(patient),
    )
    .await?;
    let today = Utc::now().date_naive();
    let adjustments = match store::list(&ctx.db, patient, today).await {
        Ok(a) => a,
        Err(e) => {
            if let Some(r) = refused(ctx, agent, purpose, Action::Read, &e).await {
                return Err(r);
            }
            return Err(failure(e));
        }
    };
    audit_resource(
        &ctx.db,
        agent,
        Access {
            action: Action::Read,
            outcome: 0,
            purpose,
            resource: "Adjustment",
            patient_id: Some(patient),
        },
        serde_json::json!({ "adjustments": adjustments.len() }),
    )
    .await?;
    Ok(AdjustmentList { adjustments })
}

/// `GET /api/patients/{id}/adjustments`: every adjustment, newest first. Audited.
pub(super) async fn list(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path(patient): Path<String>,
) -> Response {
    match list_service(&ctx, &agent, &subject, &headers, &patient).await {
        Ok(list) => Json(list).into_response(),
        Err(e) => e.into_response(),
    }
}

/// `POST /api/patients/{id}/adjustments`: record one. The author is the token.
pub(super) async fn create(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path(patient): Path<String>,
    Json(request): Json<RecordRequest>,
) -> Response {
    let purpose = match purpose_of(&headers) {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    if let Err(r) = authorize(
        &ctx.db,
        &agent,
        &subject,
        Resource::Adjustment,
        policy::Action::Create,
        purpose,
        Some(&patient),
    )
    .await
    {
        return r.into_response();
    }
    let today = Utc::now().date_naive();
    let made = adjustments::record(
        &request.code,
        request.note.as_deref(),
        request.review_by.as_deref(),
        &request.reason,
        today,
    )
    .map_err(store::refused);
    let tx = match begin(&ctx.db).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    let written = match made {
        Ok(adjustment) => store::record(&tx, &patient, &agent.0, &adjustment, today).await,
        Err(e) => Err(e),
    };
    match written {
        Ok(stored) => {
            if let Err(r) = audit_resource(
                &tx,
                &agent,
                Access {
                    action: Action::Create,
                    outcome: 0,
                    purpose,
                    resource: "Adjustment",
                    patient_id: Some(&stored.patient_id),
                },
                serde_json::json!({
                    "adjustment": stored.id,
                    "version": stored.version,
                    "status": stored.adjustment.status.word(),
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
            let _ = sea_orm::TransactionSession::rollback(tx).await;
            if let Some(r) = refused(&ctx, &agent, purpose, Action::Create, &e).await {
                return r.into_response();
            }
            failure(e).into_response()
        }
    }
}

/// `PUT /api/patients/{id}/adjustments/{adjustment_id}`: review or end one. A stale `expected_version`
/// is a 409.
pub(super) async fn update(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path((patient, adjustment_id)): Path<(String, String)>,
    Json(request): Json<ChangeRequest>,
) -> Response {
    let purpose = match purpose_of(&headers) {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    if let Err(r) = authorize(
        &ctx.db,
        &agent,
        &subject,
        Resource::Adjustment,
        policy::Action::Update,
        purpose,
        Some(&patient),
    )
    .await
    {
        return r.into_response();
    }
    let what = match request.action {
        ChangeAction::Review => Change::Reviewed {
            review_by: request.review_by,
        },
        ChangeAction::End => Change::End,
    };
    let today = Utc::now().date_naive();
    let tx = match begin(&ctx.db).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    match store::change(
        &tx,
        &patient,
        &adjustment_id,
        request.expected_version,
        &agent.0,
        &what,
        &request.reason,
        today,
    )
    .await
    {
        Ok(stored) => {
            if let Err(r) = audit_resource(
                &tx,
                &agent,
                Access {
                    action: Action::Update,
                    outcome: 0,
                    purpose,
                    resource: "Adjustment",
                    patient_id: Some(&stored.patient_id),
                },
                serde_json::json!({
                    "adjustment": stored.id,
                    "version": stored.version,
                    "status": stored.adjustment.status.word(),
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
            let _ = sea_orm::TransactionSession::rollback(tx).await;
            if let Some(r) = refused(&ctx, &agent, purpose, Action::Update, &e).await {
                return r.into_response();
            }
            failure(e).into_response()
        }
    }
}
