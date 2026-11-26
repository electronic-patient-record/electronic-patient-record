//! Refusal-of-treatment endpoints (RF-5): record, list and change a refusal of treatment on a patient record.
//!
//! Registered by `patients::routes` under `/api/patients/{id}/treatment-refusals`. This is **not** `consents`, which is
//! a refusal of disclosure. Every change is a new version; a doubt, a withdrawal that was heard and "entered in error" are
//! updates to the state. Audit rows carry ids, versions, kind and state, never what is refused. Nothing here answers
//! whether a person may be treated, and the list says "none recorded" and never "none".

use super::common::{Access, audit_resource, authorize, begin, commit, purpose_of, store_failure};
use crate::api_error::ApiError;
use crate::audit::Action;
use crate::auth::Agent;
use crate::patients_store::{self, StoreError};
use crate::policy::{self, Resource, Subject};
use crate::pool::Pool;
use crate::treatment_refusal::{self, RefusalRecord};
use crate::treatment_refusals_store as store;
use axa::extract::{Json, Path};
use axa::prelude::*;
use axum::{
    Extension,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

/// A new refusal of treatment.
#[derive(Debug, Deserialize)]
pub struct CreateRequest {
    /// The refusal.
    pub refusal: RefusalRecord,
    /// Why, for the version history.
    #[serde(default)]
    pub reason: Option<String>,
}

/// A change to a refusal: a new version.
#[derive(Debug, Deserialize)]
pub struct UpdateRequest {
    /// The whole new record.
    pub refusal: RefusalRecord,
    /// The version this change was made from; anything else is a 409.
    pub expected_version: i32,
    /// Why, for the version history.
    #[serde(default)]
    pub reason: Option<String>,
}

/// One refusal in a list: the stored version, and whether it is recorded on another record that was merged into this one.
#[derive(Debug, Serialize)]
pub struct Listed<'a> {
    /// The refusal.
    #[serde(flatten)]
    pub stored: &'a store::StoredRefusal,
    /// True when it is recorded on a record merged into this one: another record's, shown here.
    pub from_merged_record: bool,
}

fn invalid(problems: &[String]) -> Response {
    ApiError::invalid(problems.to_vec()).into_response()
}

/// What is wrong with a record, now.
fn problems_of(record: &RefusalRecord) -> Vec<String> {
    let now = jiff::Timestamp::now();
    treatment_refusal::validate(record, now, crate::clock::utc_date(now))
}

/// The audit detail of a stored refusal: ids and words from closed lists, never the treatment.
fn detail(stored: &store::StoredRefusal) -> serde_json::Value {
    serde_json::json!({
        "refusal": stored.id,
        "version": stored.version,
        "state": stored.refusal.state.as_str(),
        "kind": stored.refusal.kind.as_str(),
    })
}

/// `POST /api/patients/{id}/treatment-refusals`: record a refusal of treatment as version 1.
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
        Resource::TreatmentRefusal,
        policy::Action::Create,
        purpose,
        Some(&patient),
    )
    .await
    {
        return r.into_response();
    }
    let problems = problems_of(&request.refusal);
    if !problems.is_empty() {
        return invalid(&problems);
    }
    let mut tx = match begin(&ctx.pool()).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    match store::create(
        &mut *tx,
        &patient,
        &agent.0,
        request.reason.as_deref(),
        &request.refusal,
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
                    resource: "TreatmentRefusal",
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

/// `GET /api/patients/{id}/treatment-refusals`: every refusal on the record and on records merged into it, with a summary
/// that says "none recorded" when that is what is known. A refusal from a merged-in record says so. Audited with the count.
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
        Resource::TreatmentRefusal,
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
                    resource: "TreatmentRefusal",
                    patient_id: Some(&known.id),
                },
                serde_json::json!({ "refusals": found.len() }),
            )
            .await
            {
                return r.into_response();
            }
            let summary = treatment_refusal::summarise(found.iter().map(|s| &s.refusal));
            let refusals: Vec<Listed<'_>> = found
                .iter()
                .map(|stored| Listed {
                    stored,
                    from_merged_record: stored.patient_id != known.id,
                })
                .collect();
            Json(serde_json::json!({ "refusals": refusals, "summary": summary })).into_response()
        }
        Err(e) => store_failure(&e),
    }
}

/// `PUT /api/patients/{id}/treatment-refusals/{refusal_id}`: change a refusal as a new version. A stale
/// `expected_version` is a 409.
pub(super) async fn update(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path((patient, refusal_id)): Path<(String, String)>,
    Json(request): Json<UpdateRequest>,
) -> Response {
    let purpose = match purpose_of(&headers) {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    // Marking one entered in error is a clinician's act (the table gives that action to a clinician alone).
    let needed = if request.refusal.state == treatment_refusal::State::EnteredInError {
        policy::Action::MarkEnteredInError
    } else {
        policy::Action::Update
    };
    if let Err(r) = authorize(
        &ctx.pool(),
        &agent,
        &subject,
        Resource::TreatmentRefusal,
        needed,
        purpose,
        Some(&patient),
    )
    .await
    {
        return r.into_response();
    }
    let problems = problems_of(&request.refusal);
    if !problems.is_empty() {
        return invalid(&problems);
    }
    let mut tx = match begin(&ctx.pool()).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    match store::update(
        &mut *tx,
        &patient,
        &refusal_id,
        request.expected_version,
        &agent.0,
        request.reason.as_deref(),
        &request.refusal,
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
                    resource: "TreatmentRefusal",
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
            resource: "TreatmentRefusal",
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
