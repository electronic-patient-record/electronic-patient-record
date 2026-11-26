//! Judgement endpoints (RF-8): record whether a person can make one decision now, list the judgements, and mark one
//! entered in error. Registered by `patients::routes` under `/api/patients/{id}/decision-ability`.
//!
//! A judgement is never changed, so there is no `PUT`. The list gives each judgement with its age and what it does not
//! record, and the decisions on which the latest views of two people differ. **Nothing here answers whether a person "has
//! capacity".** Audit rows carry ids and the version, never the decision, the reasons or the conclusion.

use super::common::{Access, audit_resource, authorize, begin, commit, purpose_of, store_failure};
use crate::api_error::ApiError;
use crate::audit::Action;
use crate::auth::Agent;
use crate::decision_ability::{self, Judgement, State as JudgementState};
use crate::decision_ability_store as store;
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

/// A new judgement.
#[derive(Debug, Deserialize)]
pub struct CreateRequest {
    /// The judgement.
    pub judgement: Judgement,
}

/// Marking a judgement entered in error.
#[derive(Debug, Deserialize)]
pub struct EnteredInErrorRequest {
    /// The version this was decided from; anything else is a 409.
    pub expected_version: i32,
    /// Why. Required.
    pub reason: String,
}

/// One judgement in a list: the stored version, how old it is, what it does not record, and whether it is on another
/// record merged into this one.
#[derive(Debug, Serialize)]
pub struct Listed<'a> {
    /// The judgement.
    #[serde(flatten)]
    pub stored: &'a store::StoredJudgement,
    /// Whole minutes since it was made, for the screen. A judgement goes out of date by its age, and is never renewed.
    pub age_minutes: Option<i64>,
    /// What it does not record. Not a verdict.
    pub gaps: Vec<&'static str>,
    /// True when it is recorded on a record merged into this one.
    pub from_merged_record: bool,
}

fn invalid(problems: &[String]) -> Response {
    ApiError::invalid(problems.to_vec()).into_response()
}

fn detail(stored: &store::StoredJudgement) -> serde_json::Value {
    serde_json::json!({ "judgement": stored.id, "version": stored.version, "state": stored.state.as_str() })
}

/// `POST /api/patients/{id}/decision-ability`: record a judgement.
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
        Resource::DecisionAbility,
        policy::Action::Create,
        purpose,
        Some(&patient),
    )
    .await
    {
        return r.into_response();
    }
    let problems = decision_ability::validate(&request.judgement, jiff::Timestamp::now());
    if !problems.is_empty() {
        return invalid(&problems);
    }
    let mut tx = match begin(&ctx.pool()).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    match store::create(&mut *tx, &patient, &agent.0, &request.judgement).await {
        Ok(stored) => {
            if let Err(r) = audit_resource(
                &mut *tx,
                &agent,
                Access {
                    action: Action::Create,
                    outcome: 0,
                    purpose,
                    resource: "DecisionAbility",
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

/// `GET /api/patients/{id}/decision-ability`: every judgement on the record and on records merged into it, newest first,
/// each with its age and gaps, and the decisions on which two people's latest views differ. Audited with the count.
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
        Resource::DecisionAbility,
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
                    resource: "DecisionAbility",
                    patient_id: Some(&known.id),
                },
                serde_json::json!({ "judgements": found.len() }),
            )
            .await
            {
                return r.into_response();
            }
            let now = jiff::Timestamp::now();
            let judgements: Vec<Listed<'_>> = found
                .iter()
                .map(|stored| Listed {
                    stored,
                    age_minutes: decision_ability::age_minutes(&stored.judgement, now),
                    gaps: decision_ability::gaps(&stored.judgement),
                    from_merged_record: stored.patient_id != known.id,
                })
                .collect();
            Json(serde_json::json!({ "judgements": judgements, "disputed": disputed(&found) }))
                .into_response()
        }
        Err(e) => store_failure(&e),
    }
}

/// The decisions, in the words of their newest judgement, on which the latest views of two people differ. One entered in
/// error counts for nothing.
fn disputed(found: &[store::StoredJudgement]) -> Vec<String> {
    let standing: Vec<Judgement> = found
        .iter()
        .filter(|s| s.state == JudgementState::Recorded)
        .map(|s| s.judgement.clone())
        .collect();
    let mut out: Vec<String> = Vec::new();
    for j in &standing {
        let about = decision_ability::about_this(&standing, &j.about);
        if decision_ability::disputed(&about)
            && let Some(newest) = about.first()
            && !out.contains(&newest.about)
        {
            out.push(newest.about.clone());
        }
    }
    out
}

/// `POST /api/patients/{id}/decision-ability/{judgement_id}/entered-in-error`: mark a judgement recorded by mistake. A
/// clinician's act; the judgement stays as made.
pub(super) async fn entered_in_error(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path((patient, judgement_id)): Path<(String, String)>,
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
        Resource::DecisionAbility,
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
        &judgement_id,
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
                    resource: "DecisionAbility",
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
            resource: "DecisionAbility",
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
