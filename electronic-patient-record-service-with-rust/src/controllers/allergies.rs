//! Allergy routes (task RC-5, in part): the first clinical content.
//!
//! - `GET  /api/patients/{id}/allergies`: every allergy (all statuses), the "no known allergies"
//!   statement if any, and the banner `summary`.
//! - `POST /api/patients/{id}/allergies`: record one.
//! - `PUT  /api/patients/{id}/allergies/{allergy_id}`: change it, end it, or mark it entered in error.
//! - `PUT  /api/patients/{id}/allergy-statement`: state, or withdraw, "no known allergies".
//!
//! Allergies are clinical content, so the record-level decision applies in full: the role, a care
//! relationship (or a break-the-glass grant, for a read), the purpose of use and the patient's consent. A
//! role that may not read them, such as a receptionist, is refused by the route gate. A write and its
//! audit row are one transaction. The audit row records ids, versions and statuses, never the
//! substance or the reaction.

use super::common::{Access, audit_resource, authorize, begin, commit, purpose_of};
use crate::allergies::{self as store, AllergyRecord, Status};
use crate::api_error::ApiError;
use crate::audit::Action;
use crate::auth::Agent;
use crate::patients_store::StoreError;
use crate::policy::{self, Purpose, Resource, Subject};
use axum::{
    Extension,
    extract::{Json, Path},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};

/// Record an allergy.
#[derive(Debug, Deserialize)]
pub struct CreateRequest {
    /// The allergy.
    pub allergy: AllergyRecord,
    /// Why, for the version history.
    #[serde(default)]
    pub reason: Option<String>,
}

/// Change an allergy: a new version.
#[derive(Debug, Deserialize)]
pub struct UpdateRequest {
    /// The whole new record.
    pub allergy: AllergyRecord,
    /// The version this change was made from; anything else is a 409.
    pub expected_version: i32,
    /// Why, for the version history.
    #[serde(default)]
    pub reason: Option<String>,
}

/// State, or withdraw, "no known allergies".
#[derive(Debug, Deserialize)]
pub struct StatementRequest {
    /// `true` states it; `false` withdraws an earlier statement.
    pub none_known: bool,
    /// The statement version last seen (0 for none).
    pub expected_version: i32,
    /// Why, for the version history.
    #[serde(default)]
    pub reason: Option<String>,
}

fn failure(e: StoreError) -> ApiError {
    match e {
        // A refused record or statement is the writer's to fix, like a refused identity record.
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
            resource: "AllergyIntolerance",
            patient_id: None,
        },
        // The text names no substance: it is a store message such as "version conflict".
        serde_json::json!({ "refused": e.to_string() }),
    )
    .await
    .err()
}

/// A patient's allergies, the statement and the banner summary.
#[derive(Debug, Serialize)]
pub struct AllergyList {
    /// Every allergy in every status.
    pub allergies: Vec<store::StoredAllergy>,
    /// "No known allergies", when stated.
    pub statement: Option<store::Assertion>,
    /// The banner state.
    pub summary: store::Summary,
    /// What a reader must be told about each allergy that has an amendment request (RC-4), by allergy
    /// id, newest first. An allergy with none has no key. The list fails rather than leave one out.
    pub amendment_notices: std::collections::BTreeMap<String, Vec<crate::amendment::Notice>>,
}

pub(super) async fn list_service(
    ctx: &AppContext,
    agent: &Agent,
    subject: &Subject,
    headers: &HeaderMap,
    patient: &str,
) -> Result<AllergyList, ApiError> {
    let purpose = match purpose_of(headers) {
        Ok(p) => p,
        Err(r) => return Err(r),
    };
    authorize(
        &ctx.db,
        agent,
        subject,
        Resource::AllergyIntolerance,
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
    let statement = match store::statement(&ctx.db, patient).await {
        Ok(s) => s,
        Err(e) => return Err(failure(e)),
    };
    let summary = store::summarise(&all, statement.as_ref());
    let state = serde_json::to_value(&summary)
        .ok()
        .and_then(|v| v.get("state").cloned());
    // Audit before the clinical content leaves.
    let patient_id = all.first().map(|a| a.patient_id.clone());
    audit_resource(
        &ctx.db,
        agent,
        Access {
            action: Action::Read,
            outcome: 0,
            purpose,
            resource: "AllergyIntolerance",
            patient_id: patient_id.as_deref().or(Some(patient)),
        },
        serde_json::json!({ "allergies": all.len(), "summary": state }),
    )
    .await?;
    let amendment_notices =
        crate::amendments_store::notices_by_target(&ctx.db, patient, "AllergyIntolerance")
            .await
            .map_err(|e| match e {
                crate::patients_store::StoreError::BadQuery(m) => ApiError::invalid(vec![m]),
                other => ApiError::from(&other),
            })?;
    Ok(AllergyList {
        allergies: all,
        statement,
        summary,
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
        Resource::AllergyIntolerance,
        policy::Action::Create,
        purpose,
        Some(&patient),
    )
    .await
    {
        return r.into_response();
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
        &request.allergy,
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
                    resource: "AllergyIntolerance",
                    patient_id: Some(&stored.patient_id),
                },
                serde_json::json!({
                    "allergy": stored.id,
                    "version": stored.version,
                    "status": stored.allergy.status.as_str(),
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
    Path((patient, allergy_id)): Path<(String, String)>,
    Json(request): Json<UpdateRequest>,
) -> Response {
    let purpose = match purpose_of(&headers) {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    // Marking a mistake is its own permission: a nurse may update an allergy but not retract one.
    let needed = if request.allergy.status == Status::EnteredInError {
        policy::Action::MarkEnteredInError
    } else {
        policy::Action::Update
    };
    if let Err(r) = authorize(
        &ctx.db,
        &agent,
        &subject,
        Resource::AllergyIntolerance,
        needed,
        purpose,
        Some(&patient),
    )
    .await
    {
        return r.into_response();
    }
    let tx = match begin(&ctx.db).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    match store::update(
        &tx,
        &patient,
        &allergy_id,
        request.expected_version,
        &agent.0,
        request.reason.as_deref(),
        &request.allergy,
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
                    resource: "AllergyIntolerance",
                    patient_id: Some(&stored.patient_id),
                },
                serde_json::json!({
                    "allergy": stored.id,
                    "version": stored.version,
                    "status": stored.allergy.status.as_str(),
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

pub(super) async fn statement(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path(patient): Path<String>,
    Json(request): Json<StatementRequest>,
) -> Response {
    let purpose = match purpose_of(&headers) {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    let (needed, action) = if request.none_known {
        (policy::Action::Create, Action::Create)
    } else {
        (policy::Action::Update, Action::Update)
    };
    if let Err(r) = authorize(
        &ctx.db,
        &agent,
        &subject,
        Resource::AllergyIntolerance,
        needed,
        purpose,
        Some(&patient),
    )
    .await
    {
        return r.into_response();
    }
    let tx = match begin(&ctx.db).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    match store::assert_none_known(
        &tx,
        &patient,
        request.expected_version,
        &agent.0,
        request.reason.as_deref(),
        request.none_known,
    )
    .await
    {
        Ok(statement) => {
            if let Err(r) = audit_resource(
                &tx,
                &agent,
                Access {
                    action,
                    outcome: 0,
                    purpose,
                    resource: "AllergyIntolerance/statement",
                    patient_id: Some(&patient),
                },
                serde_json::json!({
                    "version": statement.version,
                    "no_known_allergies": statement.no_known_allergies,
                }),
            )
            .await
            {
                return r.into_response();
            }
            if let Err(r) = commit(tx).await {
                return r.into_response();
            }
            Json(statement).into_response()
        }
        Err(e) => {
            let _ = sea_orm::TransactionSession::rollback(tx).await;
            if let Some(r) = refused(&ctx, &agent, purpose, action, &e).await {
                return r.into_response();
            }
            failure(e).into_response()
        }
    }
}
