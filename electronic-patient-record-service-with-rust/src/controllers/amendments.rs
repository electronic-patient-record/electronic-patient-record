//! Amendment request routes (task RC-4): a patient asks for a record to be corrected, a clinician
//! decides, and the correction is recorded as a new version of the record itself.
//!
//! - `GET  /api/patients/{id}/amendments`: every request about this patient's records, newest first,
//!   each with the notice a reader of the record must be told and the time a decision is due.
//! - `POST /api/patients/{id}/amendments`: the patient asks for a record to be amended.
//! - `PUT  /api/patients/{id}/amendments/{amendment_id}`: one step, `{expected_version, step}`.
//!
//! The record-level decision applies in full (role, care relationship, purpose, consent); a proxy has no
//! scope for amendments yet, so is refused. **Who is acting is the token, never the body**: the requester
//! is the agent who opened it, and `by` on a decision is the agent who sends it. Deciding steps
//! (`accept`, `deny`, `apply`, `extend`) need the clinician role; the requester's own steps (`withdraw`,
//! `disagree`) need the patient role. A write and its audit row are one transaction; the audit row holds
//! ids, kinds, versions and states, never the request text, a reason or a statement.

use super::common::{Access, audit_resource, authorize, begin, commit, purpose_of};
use crate::amendment::{Event, Ground};
use crate::amendments_store::{self as store, StoredAmendment};
use crate::api_error::ApiError;
use crate::audit::Action;
use crate::auth::{self, Agent};
use crate::patients_store::StoreError;
use crate::policy::{self, Purpose, Resource, Role, Subject};
use crate::pool::Pool;
use axa::extract::{Json, Path};
use axa::prelude::*;
use axum::{
    Extension,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Ask for a record to be amended.
#[derive(Debug, Deserialize)]
pub struct CreateRequest {
    /// `Observation` or `AllergyIntolerance`.
    pub kind: String,
    /// The record's id.
    pub target_id: String,
    /// The version of it the patient read.
    pub version: i32,
    /// What should change, in the patient's words.
    pub text: String,
}

/// One step on a request.
#[derive(Debug, Deserialize)]
#[serde(tag = "step", rename_all = "kebab-case")]
pub enum Step {
    /// Agree.
    Accept {
        /// Why.
        reason: String,
    },
    /// Refuse.
    Deny {
        /// The ground.
        ground: Ground,
        /// Why.
        reason: String,
    },
    /// The correction is recorded as this version of the record.
    Apply {
        /// The version.
        version: i32,
    },
    /// Take the one extension.
    Extend {
        /// Why.
        reason: String,
    },
    /// The patient withdraws.
    Withdraw,
    /// The patient states that they disagree with a refusal.
    Disagree {
        /// The statement.
        statement: String,
    },
}

/// A step on a request.
#[derive(Debug, Deserialize)]
pub struct UpdateRequest {
    /// The version of the request this step is made on.
    pub expected_version: i32,
    /// What to do.
    #[serde(flatten)]
    pub step: Step,
}

/// Every request of a patient.
#[derive(Debug, Serialize)]
pub struct AmendmentList {
    /// Newest first.
    pub amendments: Vec<serde_json::Value>,
}

impl Step {
    /// The role that may take this step.
    fn role(&self) -> Role {
        match self {
            Step::Accept { .. } | Step::Deny { .. } | Step::Apply { .. } | Step::Extend { .. } => {
                Role::Clinician
            }
            Step::Withdraw | Step::Disagree { .. } => Role::Patient,
        }
    }

    /// The audit-store [`Event`] this step writes, with `by` (the token subject, never the body) where the event records who.
    fn event(self, by: &str) -> Event {
        let by = by.to_string();
        match self {
            Step::Accept { reason } => Event::Accept { by, reason },
            Step::Deny { ground, reason } => Event::Deny { by, ground, reason },
            Step::Apply { version } => Event::Apply { version },
            Step::Extend { reason } => Event::Extend { reason },
            Step::Withdraw => Event::Withdraw { by },
            Step::Disagree { statement } => Event::Disagree { by, statement },
        }
    }
}

/// A store error as this route's answer: a refused record is a 422 listing what to fix; anything else keeps its own
/// status through `ApiError::from` (a miss is 404, a stale version 409).
fn failure(e: StoreError) -> ApiError {
    match e {
        StoreError::BadQuery(m) => ApiError::invalid(vec![m]),
        other => ApiError::from(&other),
    }
}

/// The request with what a reader needs beside it.
fn view(a: &StoredAmendment, now: chrono::DateTime<Utc>) -> serde_json::Value {
    let mut v = serde_json::to_value(a).unwrap_or_default();
    v["notice"] = serde_json::to_value(a.request.notice()).unwrap_or_default();
    v["due"] = a
        .request
        .due()
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
        .into();
    v["overdue"] = a.request.overdue(now).into();
    v
}

/// Audit a refusal of a write (a miss, a stale version, a refused step).
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
            resource: "Amendment",
            patient_id: None,
        },
        serde_json::json!({ "refused": e.to_string() }),
    )
    .await
    .err()
}

/// `GET /api/patients/{id}/amendments`: every amendment request about the patient's records, newest first, each
/// with its notice and the time a decision is due. Audited.
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
        Resource::Amendment,
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
    // Audit before the content leaves.
    let patient_id = all.first().map(|a| a.patient_id.clone());
    if let Err(r) = audit_resource(
        &ctx.pool(),
        &agent,
        Access {
            action: Action::Read,
            outcome: 0,
            purpose,
            resource: "Amendment",
            patient_id: patient_id.as_deref().or(Some(&patient)),
        },
        serde_json::json!({ "amendments": all.len() }),
    )
    .await
    {
        return r.into_response();
    }
    let now = Utc::now();
    Json(AmendmentList {
        amendments: all.iter().map(|a| view(a, now)).collect(),
    })
    .into_response()
}

/// `POST /api/patients/{id}/amendments`: the patient asks for a record to be amended. The requester is the token
/// subject. The request text is stored but never copied into the audit row.
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
        Resource::Amendment,
        policy::Action::Create,
        purpose,
        Some(&patient),
    )
    .await
    {
        return r.into_response();
    }
    let now = Utc::now();
    let mut tx = match begin(&ctx.pool()).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    match store::open(
        &mut *tx,
        &patient,
        &agent.0,
        &request.kind,
        &request.target_id,
        request.version,
        &request.text,
        now,
    )
    .await
    {
        Ok(stored) => {
            if let Err(r) = audit_resource(
                &mut *tx,
                &agent,
                Access {
                    action: Action::Create,
                    outcome: 0,
                    purpose,
                    resource: "Amendment",
                    patient_id: Some(&stored.patient_id),
                },
                serde_json::json!({
                    "amendment": stored.id,
                    "kind": stored.request.target.kind,
                    "target": stored.request.target.id,
                    "version": stored.version,
                }),
            )
            .await
            {
                return r.into_response();
            }
            if let Err(r) = commit(tx).await {
                return r.into_response();
            }
            (StatusCode::CREATED, Json(view(&stored, now))).into_response()
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

/// `PUT /api/patients/{id}/amendments/{amendment_id}`: one step of a request. Deciding steps need the clinician role
/// and the requester's own steps (`withdraw`, `disagree`) the patient role; a stale `expected_version` is a 409.
pub(super) async fn update(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path((patient, amendment_id)): Path<(String, String)>,
    Json(request): Json<UpdateRequest>,
) -> Response {
    let purpose = match purpose_of(&headers) {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    if let Err(r) = authorize(
        &ctx.pool(),
        &agent,
        &subject,
        Resource::Amendment,
        policy::Action::Update,
        purpose,
        Some(&patient),
    )
    .await
    {
        return r.into_response();
    }
    // A step belongs to one role: a patient does not decide, a clinician does not speak for the patient.
    if auth::authentication_enforced() && !subject.roles.contains(&request.step.role()) {
        return ApiError::forbidden().into_response();
    }
    let now = Utc::now();
    let event = request.step.event(&agent.0);
    let mut tx = match begin(&ctx.pool()).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    match store::advance(
        &mut *tx,
        &patient,
        &amendment_id,
        request.expected_version,
        &agent.0,
        event,
        now,
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
                    resource: "Amendment",
                    patient_id: Some(&stored.patient_id),
                },
                serde_json::json!({
                    "amendment": stored.id,
                    "version": stored.version,
                    "state": stored.request.state.word(),
                }),
            )
            .await
            {
                return r.into_response();
            }
            if let Err(r) = commit(tx).await {
                return r.into_response();
            }
            Json(view(&stored, now)).into_response()
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
