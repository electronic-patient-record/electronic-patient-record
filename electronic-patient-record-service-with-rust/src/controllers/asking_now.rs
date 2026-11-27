//! Endpoints for asking a person now, and leaving against advice (RF-10, with EC-6): record what was offered, explained,
//! refused and accepted, list the records, and mark one entered in error. Registered by `patients::routes` under
//! `/api/patients/{id}/asking-now`.
//!
//! A record is never changed, so there is no `PUT`. **Nothing here decides whether a refusal stands**, and the place's pack
//! is not read yet (RF-3), so no gaps against it are given. Audit rows carry ids, the version and the state, never what was
//! offered, said or refused.

use super::common::{Access, audit_resource, authorize, begin, commit, purpose_of, store_failure};
use crate::api_error::ApiError;
use crate::asking_now::{self, Asked};
use crate::asking_now_store as store;
use crate::audit::Action;
use crate::auth::Agent;
use crate::decision_ability::State as RecordState;
use crate::patients_store::{self, StoreError};
use crate::policy::{self, Resource, Subject};
use crate::pool::Pool;
use axa::extract::{Json, Path};
use axa::prelude::*;
use axum::{
    Extension,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

/// A new record of asking.
#[derive(Debug, Deserialize)]
pub struct CreateRequest {
    /// The record.
    pub asked: Asked,
}

/// Marking a record entered in error.
#[derive(Debug, Deserialize)]
pub struct EnteredInErrorRequest {
    /// The version this was decided from; anything else is a 409.
    pub expected_version: i32,
    /// Why. Required.
    pub reason: String,
}

/// One record in a list: the stored version, and whether it is on another record merged into this one.
#[derive(Debug, Serialize)]
pub struct Listed<'a> {
    /// The record.
    #[serde(flatten)]
    pub stored: &'a store::StoredAsked,
    /// True when it is recorded on a record merged into this one.
    pub from_merged_record: bool,
}

fn invalid(problems: &[String]) -> Response {
    ApiError::invalid(problems.to_vec()).into_response()
}

fn detail(stored: &store::StoredAsked) -> serde_json::Value {
    serde_json::json!({ "record": stored.id, "version": stored.version, "state": stored.state.as_str() })
}

/// `POST /api/patients/{id}/asking-now`: record a time of asking, or of leaving against advice.
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
        Resource::AskingNow,
        policy::Action::Create,
        purpose,
        Some(&patient),
    )
    .await
    {
        return r.into_response();
    }
    let problems = asking_now::validate(&request.asked, jiff::Timestamp::now());
    if !problems.is_empty() {
        return invalid(&problems);
    }
    let mut tx = match begin(&ctx.pool()).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    match store::create(&mut *tx, &patient, &agent.0, &request.asked).await {
        Ok(stored) => {
            if let Err(r) = audit_resource(
                &mut *tx,
                &agent,
                Access {
                    action: Action::Create,
                    outcome: 0,
                    purpose,
                    resource: "AskingNow",
                    patient_id: Some(&stored.patient_id),
                },
                detail(&stored),
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
            refused(&ctx, &agent, purpose, Action::Create, &e).await
        }
    }
}

/// `GET /api/patients/{id}/asking-now`: every record on the patient record and on records merged into it, newest first,
/// and how many that stand say the person left against advice. Audited with the count.
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
        Resource::AskingNow,
        policy::Action::Read,
        purpose,
        Some(&patient),
    )
    .await
    {
        return r.into_response();
    }
    let known = match patients_store::current(&ctx.pool(), &patient).await {
        Ok(p) => p,
        Err(e) => return refused(&ctx, &agent, purpose, Action::Read, &e).await,
    };
    match store::list_with_merged(&ctx.pool(), &known.id).await {
        Ok(found) => {
            if let Err(r) = audit_resource(
                &ctx.pool(),
                &agent,
                Access {
                    action: Action::Read,
                    outcome: 0,
                    purpose,
                    resource: "AskingNow",
                    patient_id: Some(&known.id),
                },
                serde_json::json!({ "records": found.len() }),
            )
            .await
            {
                return r.into_response();
            }
            let records: Vec<Listed<'_>> = found
                .iter()
                .map(|stored| Listed {
                    stored,
                    from_merged_record: stored.patient_id != known.id,
                })
                .collect();
            let left = found
                .iter()
                .filter(|s| s.state == RecordState::Recorded && s.asked.leaving_against_advice)
                .count();
            Json(serde_json::json!({ "records": records, "left_against_advice": left }))
                .into_response()
        }
        Err(e) => store_failure(&e),
    }
}

/// `POST /api/patients/{id}/asking-now/{record_id}/entered-in-error`: mark a record made by mistake. A clinician's
/// act; the record stays as made.
pub(super) async fn entered_in_error(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path((patient, record_id)): Path<(String, String)>,
    Json(request): Json<EnteredInErrorRequest>,
) -> Response {
    let purpose = match purpose_of(&headers) {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    if let Err(r) = authorize(
        &ctx.pool(),
        &agent,
        &subject,
        Resource::AskingNow,
        policy::Action::MarkEnteredInError,
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
    match store::mark_entered_in_error(
        &mut *tx,
        &patient,
        &record_id,
        request.expected_version,
        &agent.0,
        &request.reason,
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
                    resource: "AskingNow",
                    patient_id: Some(&stored.patient_id),
                },
                detail(&stored),
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
            refused(&ctx, &agent, purpose, Action::Update, &e).await
        }
    }
}

/// A store failure as a response; a miss, a stale version or a refused rule is also an audited refusal.
async fn refused(
    ctx: &AppContext,
    agent: &Agent,
    purpose: policy::Purpose,
    action: Action,
    e: &StoreError,
) -> Response {
    if matches!(
        e,
        StoreError::NotFound | StoreError::Conflict { .. } | StoreError::BadQuery(_)
    ) && let Err(r) = audit_resource(
        &ctx.pool(),
        agent,
        Access {
            action,
            outcome: 4,
            purpose,
            resource: "AskingNow",
            patient_id: None,
        },
        serde_json::json!({ "refused": e.refusal_kind() }),
    )
    .await
    {
        return r.into_response();
    }
    match e {
        StoreError::BadQuery(m) => invalid(std::slice::from_ref(m)),
        other => store_failure(other),
    }
}
