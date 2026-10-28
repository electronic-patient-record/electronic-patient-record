//! Hazard register routes (task AS-17).
//!
//! - `GET  /api/hazards`: every hazard's current version, with its controls and, for a retired one, the
//!   evidence and who retired it.
//! - `POST /api/hazards`: raise a hazard (`{title, severity, likelihood}`).
//! - `PUT  /api/hazards/{hazard_id}`: one step, `{expected_version, event}`: `control`, `retire`, `reopen`.
//!
//! **Only the Clinical Safety Officer retires a hazard, and must give evidence.** The role comes from the
//! token, never from the body, and a retirement is refused with 403 unless the token holds
//! `clinical-safety-officer`. Unlike the other write routes this check does **not** switch off when
//! authentication is off: with no token there is no Clinical Safety Officer, so nobody retires. A
//! retirement with no evidence is a 422. Any writer may reopen. Every step is a version that is never
//! edited or removed, and a write and its audit row are one transaction; the audit row holds ids, versions,
//! states and the evidence hashes, never the free text.

use super::common::{Access, audit_resource, authorize, begin, commit, purpose_of};
use crate::api_error::ApiError;
use crate::audit::Action;
use crate::auth::Agent;
use crate::hazard::{Actor, Event};
use crate::hazard_store::{self as store, StoredHazard};
use crate::patients_store::StoreError;
use crate::policy::{self, Purpose, Resource, Role, Subject};
use axum::{
    Extension,
    extract::{Json, Path},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use chrono::Utc;
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};

/// Raise a hazard.
#[derive(Debug, Deserialize)]
pub struct CreateRequest {
    /// A short name.
    pub title: String,
    /// 1 to 5, before controls.
    pub severity: u8,
    /// 1 to 5, before controls.
    pub likelihood: u8,
}

/// A step on a hazard.
#[derive(Debug, Deserialize)]
pub struct UpdateRequest {
    /// The version of the hazard this step is made on.
    pub expected_version: i32,
    /// What to do.
    #[serde(flatten)]
    pub event: Event,
}

/// Every hazard.
#[derive(Debug, Serialize)]
pub struct HazardList {
    /// Newest first.
    pub hazards: Vec<StoredHazard>,
}

fn failure(e: StoreError) -> ApiError {
    match e {
        StoreError::BadQuery(m) => ApiError::invalid(vec![m]),
        other => ApiError::from(&other),
    }
}

/// Audit a refusal of a write (a miss, a stale version, a refused step).
async fn refused(
    ctx: &AppContext,
    agent: &Agent,
    purpose: Purpose,
    action: Action,
    why: &str,
) -> Option<ApiError> {
    audit_resource(
        &ctx.db,
        agent,
        Access {
            action,
            outcome: 4,
            purpose,
            resource: "Hazard",
            patient_id: None,
        },
        serde_json::json!({ "refused": why }),
    )
    .await
    .err()
}

/// What the audit row says about a step: the kind, and for a retirement the evidence hashes.
fn step_details(event: &Event) -> serde_json::Value {
    match event {
        Event::Control { .. } => serde_json::json!({ "event": "control" }),
        Event::Retire { evidence, .. } => serde_json::json!({
            "event": "retire",
            "evidence": evidence
                .iter()
                .map(|e| serde_json::json!({ "kind": e.kind, "sha256": e.sha256 }))
                .collect::<Vec<_>>(),
        }),
        Event::Reopen { .. } => serde_json::json!({ "event": "reopen" }),
    }
}

pub(super) async fn list(
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
        &ctx.db,
        &agent,
        &subject,
        Resource::Hazard,
        policy::Action::Read,
        purpose,
        None,
    )
    .await
    {
        return r.into_response();
    }
    let hazards = match store::list(&ctx.db).await {
        Ok(h) => h,
        Err(e) => return failure(e).into_response(),
    };
    // Audit before the content leaves.
    if let Err(r) = audit_resource(
        &ctx.db,
        &agent,
        Access {
            action: Action::Read,
            outcome: 0,
            purpose,
            resource: "Hazard",
            patient_id: None,
        },
        serde_json::json!({ "hazards": hazards.len() }),
    )
    .await
    {
        return r.into_response();
    }
    Json(HazardList { hazards }).into_response()
}

pub(super) async fn create(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
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
        Resource::Hazard,
        policy::Action::Create,
        purpose,
        None,
    )
    .await
    {
        return r.into_response();
    }
    let tx = match begin(&ctx.db).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    match store::raise(
        &tx,
        &agent.0,
        &request.title,
        request.severity,
        request.likelihood,
        Utc::now(),
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
                    resource: "Hazard",
                    patient_id: None,
                },
                serde_json::json!({ "hazard": stored.id, "version": stored.version }),
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
            if let Some(r) = refused(&ctx, &agent, purpose, Action::Create, &e.to_string()).await {
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
    Path(hazard_id): Path<String>,
    Json(request): Json<UpdateRequest>,
) -> Response {
    let purpose = match purpose_of(&headers) {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    if let Err(r) = authorize(
        &ctx.db,
        &agent,
        &subject,
        Resource::Hazard,
        policy::Action::Update,
        purpose,
        None,
    )
    .await
    {
        return r.into_response();
    }
    // The role is the token's. This check never switches off: no token, no Clinical Safety Officer.
    let is_cso = subject.roles.contains(&Role::ClinicalSafetyOfficer);
    if matches!(request.event, Event::Retire { .. }) && !is_cso {
        if let Some(r) = refused(
            &ctx,
            &agent,
            purpose,
            Action::Update,
            "retire needs the clinical-safety-officer role",
        )
        .await
        {
            return r.into_response();
        }
        return ApiError::forbidden().into_response();
    }
    let actor = Actor {
        id: &agent.0,
        is_cso,
    };
    let details = step_details(&request.event);
    let tx = match begin(&ctx.db).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    match store::advance(
        &tx,
        &hazard_id,
        request.expected_version,
        &actor,
        request.event,
        Utc::now(),
    )
    .await
    {
        Ok(stored) => {
            let mut audit = details;
            audit["hazard"] = stored.id.clone().into();
            audit["version"] = stored.version.into();
            audit["state"] = stored.hazard.state.word().into();
            if let Err(r) = audit_resource(
                &tx,
                &agent,
                Access {
                    action: Action::Update,
                    outcome: 0,
                    purpose,
                    resource: "Hazard",
                    patient_id: None,
                },
                audit,
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
            if matches!(
                e,
                StoreError::NotFound | StoreError::Conflict { .. } | StoreError::BadQuery(_)
            ) && let Some(r) =
                refused(&ctx, &agent, purpose, Action::Update, &e.to_string()).await
            {
                return r.into_response();
            }
            failure(e).into_response()
        }
    }
}

/// The hazard register's routes.
pub fn routes() -> Routes {
    Routes::new()
        .add("/api/hazards", get(list))
        .add("/api/hazards", post(create))
        .add("/api/hazards/{hazard_id}", put(update))
}
