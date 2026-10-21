//! Consent endpoints (task SE-2): record, list and change a patient's consent.
//!
//! The handlers hang off `/api/patients/{id}/consents` and are registered by
//! `patients::routes`, because loco allows one `Routes` per prefix. Every change is a new version;
//! withdrawing a consent is an update to `status: inactive`. Audit rows carry ids, versions and
//! counts, never the provisions.

use super::patients::{
    Access, audit_resource, authorize, begin, commit, json_error, purpose_of, store_failure,
};
use crate::audit::Action;
use crate::auth::Agent;
use crate::consent::{self, ConsentRecord};
use crate::consents_store as store;
use crate::patients_store::{self, StoreError};
use crate::policy::{self, Resource, Subject};
use axum::{
    Extension,
    extract::{Json, Path},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use loco_rs::prelude::*;
use serde::Deserialize;

/// A new consent.
#[derive(Debug, Deserialize)]
pub struct CreateRequest {
    /// The consent record.
    pub consent: ConsentRecord,
    /// Why, for the version history.
    #[serde(default)]
    pub reason: Option<String>,
}

/// A change to a consent: a new version.
#[derive(Debug, Deserialize)]
pub struct UpdateRequest {
    /// The whole new record.
    pub consent: ConsentRecord,
    /// The version this change was made from; anything else is a 409.
    pub expected_version: i32,
    /// Why, for the version history.
    #[serde(default)]
    pub reason: Option<String>,
}

fn invalid(problems: &[String]) -> Response {
    json_error(
        StatusCode::UNPROCESSABLE_ENTITY,
        serde_json::json!({"errors": problems}),
    )
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
        Err(r) => return *r,
    };
    if let Err(r) = authorize(
        &ctx.db,
        &agent,
        &subject,
        Resource::Consent,
        policy::Action::Create,
        purpose,
        Some(&patient),
    )
    .await
    {
        return *r;
    }
    let problems = consent::validate(&request.consent);
    if !problems.is_empty() {
        return invalid(&problems);
    }
    let tx = match begin(&ctx.db).await {
        Ok(t) => t,
        Err(r) => return *r,
    };
    match store::create(
        &tx,
        &patient,
        &agent.0,
        request.reason.as_deref(),
        &request.consent,
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
                    resource: "Consent",
                    patient_id: Some(&stored.patient_id),
                },
                serde_json::json!({
                    "consent": stored.id,
                    "version": stored.version,
                    "status": stored.consent.status.as_str(),
                }),
            )
            .await
            {
                return *r;
            }
            if let Err(r) = commit(tx).await {
                return *r;
            }
            (StatusCode::CREATED, Json(stored)).into_response()
        }
        Err(e) => {
            // Nothing was written; end the transaction before the refusal is audited.
            let _ = sea_orm::TransactionSession::rollback(tx).await;
            refused(&ctx, &agent, purpose, Action::Create, &e).await
        }
    }
}

pub(super) async fn list(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path(patient): Path<String>,
) -> Response {
    let purpose = match purpose_of(&headers) {
        Ok(p) => p,
        Err(r) => return *r,
    };
    if let Err(r) = authorize(
        &ctx.db,
        &agent,
        &subject,
        Resource::Consent,
        policy::Action::Read,
        purpose,
        Some(&patient),
    )
    .await
    {
        return *r;
    }
    // A patient that does not exist is a 404, not an empty list.
    let known = match patients_store::current(&ctx.db, &patient).await {
        Ok(p) => p,
        Err(e) => return refused(&ctx, &agent, purpose, Action::Read, &e).await,
    };
    match store::list(&ctx.db, &known.id).await {
        Ok(consents) => {
            if let Err(r) = audit_resource(
                &ctx.db,
                &agent,
                Access {
                    action: Action::Read,
                    outcome: 0,
                    purpose,
                    resource: "Consent",
                    patient_id: Some(&known.id),
                },
                serde_json::json!({ "consents": consents.len() }),
            )
            .await
            {
                return *r;
            }
            Json(serde_json::json!({ "consents": consents })).into_response()
        }
        Err(e) => store_failure(&e),
    }
}

pub(super) async fn update(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path((patient, consent_id)): Path<(String, String)>,
    Json(request): Json<UpdateRequest>,
) -> Response {
    let purpose = match purpose_of(&headers) {
        Ok(p) => p,
        Err(r) => return *r,
    };
    if let Err(r) = authorize(
        &ctx.db,
        &agent,
        &subject,
        Resource::Consent,
        policy::Action::Update,
        purpose,
        Some(&patient),
    )
    .await
    {
        return *r;
    }
    let problems = consent::validate(&request.consent);
    if !problems.is_empty() {
        return invalid(&problems);
    }
    let tx = match begin(&ctx.db).await {
        Ok(t) => t,
        Err(r) => return *r,
    };
    match store::update(
        &tx,
        &patient,
        &consent_id,
        request.expected_version,
        &agent.0,
        request.reason.as_deref(),
        &request.consent,
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
                    resource: "Consent",
                    patient_id: Some(&stored.patient_id),
                },
                serde_json::json!({
                    "consent": stored.id,
                    "version": stored.version,
                    "status": stored.consent.status.as_str(),
                }),
            )
            .await
            {
                return *r;
            }
            if let Err(r) = commit(tx).await {
                return *r;
            }
            Json(stored).into_response()
        }
        Err(e) => {
            // Nothing was written; end the transaction before the refusal is audited.
            let _ = sea_orm::TransactionSession::rollback(tx).await;
            refused(&ctx, &agent, purpose, Action::Update, &e).await
        }
    }
}

/// A store failure as a response; a miss or a stale version is also an audited refusal.
async fn refused(
    ctx: &AppContext,
    agent: &Agent,
    purpose: policy::Purpose,
    action: Action,
    e: &StoreError,
) -> Response {
    if matches!(e, StoreError::NotFound | StoreError::Conflict { .. })
        && let Err(r) = audit_resource(
            &ctx.db,
            agent,
            Access {
                action,
                outcome: 4,
                purpose,
                resource: "Consent",
                patient_id: None,
            },
            serde_json::json!({ "refused": e.to_string() }),
        )
        .await
    {
        return *r;
    }
    store_failure(e)
}
