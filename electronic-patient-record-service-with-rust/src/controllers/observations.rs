//! Observation routes (task RC-6, first store slice): vital signs.
//!
//! - `GET  /api/patients/{id}/observations`: every observation (all statuses), newest measurement
//!   first, each with its computed flag.
//! - `POST /api/patients/{id}/observations`: record one.
//! - `GET  /api/patients/{id}/observations/{observation_id}/versions`: every version, oldest first.
//! - `PUT  /api/patients/{id}/observations/{observation_id}`: correct it, or mark it entered in error.
//!
//! Observations are clinical content, so the record-level decision applies in full: the role, a care
//! relationship (or a break-the-glass grant, for a read), the purpose of use and the patient's consent. A
//! proxy has no scope for them yet, so a proxy is refused (PA-5). A write and its audit row are one
//! transaction. The audit row records ids, versions, statuses and counts, never a value, a unit or a
//! range.
//!
//! A refused record is 422 `{errors, error_codes}` and the refusal is audited like any other.

use super::common::{Access, audit_resource, authorize, begin, commit, purpose_of};
use crate::api_error::ApiError;
use crate::audit::Action;
use crate::auth::Agent;
use crate::observations_store::{self as store, ObservationRecord, Status};
use crate::patients_store::StoreError;
use crate::policy::{self, Purpose, Resource, Subject};
use axum::{
    Extension,
    extract::{Json, Path},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use chrono::Utc;
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};

/// Record an observation.
#[derive(Debug, Deserialize)]
pub struct CreateRequest {
    /// The observation.
    pub observation: ObservationRecord,
    /// Why, for the version history.
    #[serde(default)]
    pub reason: Option<String>,
}

/// Change an observation: a new version.
#[derive(Debug, Deserialize)]
pub struct UpdateRequest {
    /// The whole new record.
    pub observation: ObservationRecord,
    /// The version this change was made from; anything else is a 409.
    pub expected_version: i32,
    /// Why, for the version history.
    #[serde(default)]
    pub reason: Option<String>,
}

/// A patient's observations.
#[derive(Debug, Serialize)]
pub struct ObservationList {
    /// Every observation in every status, newest measurement first.
    pub observations: Vec<store::StoredObservation>,
    /// What a reader must be told about each observation that has an amendment request (RC-4), by
    /// observation id, newest request first. An observation with none has no key. Never left out
    /// when it cannot be read: the list fails instead.
    pub amendment_notices: std::collections::BTreeMap<String, Vec<crate::amendment::Notice>>,
}

/// One observation's versions.
#[derive(Debug, Serialize)]
pub struct VersionList {
    /// Oldest first.
    pub versions: Vec<store::ObservationVersion>,
}

fn failure(e: StoreError) -> ApiError {
    match e {
        // A refused record is the writer's to fix, like a refused identity record.
        StoreError::BadQuery(m) => ApiError::invalid(vec![m.to_string()]),
        other => ApiError::from(&other),
    }
}

/// Audit a refusal of a write (a miss, a stale version, a refused record).
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
            resource: "Observation",
            patient_id: None,
        },
        // A store message such as "version conflict"; the validation text names a code, a unit or a
        // field, never a measured value, so it is safe to keep.
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
) -> Result<ObservationList, ApiError> {
    let purpose = purpose_of(headers)?;
    authorize(
        &ctx.db,
        agent,
        subject,
        Resource::Observation,
        policy::Action::Read,
        purpose,
        Some(patient),
    )
    .await?;
    let all = match store::list(&ctx.db, patient).await {
        Ok(a) => a,
        Err(e) => {
            if let Some(r) = refused(ctx, agent, purpose, Action::Read, &e).await {
                return Err(r);
            }
            return Err(failure(e));
        }
    };
    // Audit before the clinical content leaves.
    let patient_id = all.first().map(|o| o.patient_id.clone());
    audit_resource(
        &ctx.db,
        agent,
        Access {
            action: Action::Read,
            outcome: 0,
            purpose,
            resource: "Observation",
            patient_id: patient_id.as_deref().or(Some(patient)),
        },
        serde_json::json!({ "observations": all.len() }),
    )
    .await?;
    let amendment_notices =
        crate::amendments_store::notices_by_target(&ctx.db, patient, "Observation")
            .await
            .map_err(failure)?;
    Ok(ObservationList {
        observations: all,
        amendment_notices,
    })
}

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

/// Every version of one observation, oldest first, each with the reason it was written. A read of
/// clinical content: the same access decision as the list, and an audit row (ids and a count, never a
/// value) before anything leaves.
pub(super) async fn versions(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path((patient, observation_id)): Path<(String, String)>,
) -> Response {
    let purpose = match purpose_of(&headers) {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    if let Err(r) = authorize(
        &ctx.db,
        &agent,
        &subject,
        Resource::Observation,
        policy::Action::Read,
        purpose,
        Some(&patient),
    )
    .await
    {
        return r.into_response();
    }
    let all = match store::history(&ctx.db, &patient, &observation_id).await {
        Ok(a) => a,
        Err(e) => {
            if let Some(r) = refused(&ctx, &agent, purpose, Action::Read, &e).await {
                return r.into_response();
            }
            return failure(e).into_response();
        }
    };
    let patient_id = all.first().map(|v| v.stored.patient_id.clone());
    if let Err(r) = audit_resource(
        &ctx.db,
        &agent,
        Access {
            action: Action::Read,
            outcome: 0,
            purpose,
            resource: "Observation",
            patient_id: patient_id.as_deref().or(Some(&patient)),
        },
        serde_json::json!({ "observation": observation_id, "versions": all.len() }),
    )
    .await
    {
        return r.into_response();
    }
    Json(VersionList { versions: all }).into_response()
}

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
        &ctx.db,
        &agent,
        &subject,
        Resource::Observation,
        policy::Action::Create,
        purpose,
        Some(&patient),
    )
    .await
    {
        return r.into_response();
    }
    let now = Utc::now();
    // Check the record first so the 422 carries codes; the store checks again.
    if let Some(invalid) = store::refusal(&request.observation, now) {
        let as_store = StoreError::BadQuery("the observation was refused".into());
        if let Some(r) = refused(&ctx, &agent, purpose, Action::Create, &as_store).await {
            return r.into_response();
        }
        return invalid.into_response();
    }
    let tx = match begin(&ctx.db).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    match store::create(
        &tx,
        &patient,
        &agent.0,
        request.reason.as_deref(),
        &request.observation,
        &crate::site::time_zone(),
        now,
    )
    .await
    {
        Ok(stored) => {
            if let Err(r) = audit_resource(
                &tx,
                &agent,
                Access {
                    action: Action::Create,
                    outcome: 0,
                    purpose,
                    resource: "Observation",
                    patient_id: Some(&stored.patient_id),
                },
                serde_json::json!({
                    "observation": stored.id,
                    "version": stored.version,
                    "status": stored.record.status.as_str(),
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

pub(super) async fn update(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path((patient, observation_id)): Path<(String, String)>,
    Json(request): Json<UpdateRequest>,
) -> Response {
    let purpose = match purpose_of(&headers) {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    // Marking a mistake is its own permission: a nurse may correct an observation but not retract one.
    let needed = if request.observation.status == Status::EnteredInError {
        policy::Action::MarkEnteredInError
    } else {
        policy::Action::Update
    };
    if let Err(r) = authorize(
        &ctx.db,
        &agent,
        &subject,
        Resource::Observation,
        needed,
        purpose,
        Some(&patient),
    )
    .await
    {
        return r.into_response();
    }
    let now = Utc::now();
    if let Some(invalid) = store::refusal(&request.observation, now) {
        let as_store = StoreError::BadQuery("the observation was refused".into());
        if let Some(r) = refused(&ctx, &agent, purpose, Action::Update, &as_store).await {
            return r.into_response();
        }
        return invalid.into_response();
    }
    let tx = match begin(&ctx.db).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    match store::update(
        &tx,
        &patient,
        &observation_id,
        request.expected_version,
        &agent.0,
        request.reason.as_deref(),
        &request.observation,
        &crate::site::time_zone(),
        now,
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
                    resource: "Observation",
                    patient_id: Some(&stored.patient_id),
                },
                serde_json::json!({
                    "observation": stored.id,
                    "version": stored.version,
                    "status": stored.record.status.as_str(),
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
