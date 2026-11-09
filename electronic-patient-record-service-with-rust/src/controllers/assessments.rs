//! Assessment routes (WL-3): completed assessments, each under the form version it was made with.
//!
//! - `GET  /api/patients/{id}/assessments`: the form catalogue (every version, each saying whether it is
//!   approved) and the patient's assessments, newest first, each shown with its own form version's
//!   questions.
//! - `POST /api/patients/{id}/assessments`: record one against a named form version.
//!
//! Assessments are clinical content: the role, a care relationship, the purpose and the patient's consent
//! apply. The write and its audit row are one transaction; the row holds ids and the form version, never
//! an answer or a reason.

use super::common::{Access, audit_resource, authorize, begin, commit, purpose_of};
use crate::api_error::ApiError;
use crate::assessments::{self, AssessmentRecord, Form};
use crate::assessments_store as store;
use crate::audit::Action;
use crate::auth::Agent;
use crate::patients_store::StoreError;
use crate::policy::{self, Purpose, Resource, Subject};
use crate::pool::Pool;
use axa::extract::{Json, Path};
use axa::prelude::*;
use axum::{
    Extension,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

/// Record an assessment.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateRequest {
    /// The completed assessment.
    pub assessment: AssessmentRecord,
}

/// The catalogue and a patient's assessments.
#[derive(Debug, Serialize)]
pub struct AssessmentList {
    /// Every form version, by id then version.
    pub forms: &'static [Form],
    /// Newest first.
    pub assessments: Vec<store::StoredAssessment>,
}

const RESOURCE: &str = "QuestionnaireResponse";

fn failure(e: StoreError) -> ApiError {
    match e {
        StoreError::BadQuery(m) => ApiError::invalid(vec![m]),
        other => ApiError::from(&other),
    }
}

async fn refused(
    ctx: &AppContext,
    agent: &Agent,
    purpose: Purpose,
    action: Action,
    e: &StoreError,
) -> Option<ApiError> {
    if !matches!(e, StoreError::NotFound | StoreError::BadQuery(_)) {
        return None;
    }
    // The refusal is named by kind only: the store's words can quote a question id, never an answer.
    let kind = if matches!(e, StoreError::NotFound) {
        "not_found"
    } else {
        "answers_refused"
    };
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
        serde_json::json!({ "refused": kind }),
    )
    .await
    .err()
}

/// `GET /api/patients/{id}/assessments`. Audited with the count.
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
        Resource::QuestionnaireResponse,
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
        serde_json::json!({ "assessments": all.len() }),
    )
    .await
    {
        return r.into_response();
    }
    Json(AssessmentList {
        forms: assessments::catalogue(),
        assessments: all,
    })
    .into_response()
}

/// `POST /api/patients/{id}/assessments`: 201 with the stored assessment, shown under its form version.
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
        Resource::QuestionnaireResponse,
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
    match store::record(&mut *tx, &patient, &agent.0, &request.assessment).await {
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
                    "assessment": stored.id,
                    "form": stored.form.id,
                    "form_version": stored.form.version,
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
