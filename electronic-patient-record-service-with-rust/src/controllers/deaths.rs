//! Death assertion routes (BV-1). The rules are in [`crate::death`], the store in
//! [`crate::death_store`], and the spec is `spec/bereavement/death-assertion.md`.
//!
//! - `GET  /api/patients/{id}/death`: the one answer, and whether contact is held.
//! - `GET  /api/patients/{id}/death/assertions`: every assertion, withdrawn ones included.
//! - `POST /api/patients/{id}/death/assertions`: report a death.
//! - `PUT  /api/patients/{id}/death/assertions/{assertion_id}`: one step, `{expected_version, to, reason}`.
//! - `GET  /api/death-reports`: every report that waits for a person, oldest first, ids and times only.
//! - `GET  /api/death-reports/count`: how many.
//!
//! Who may do what was decided as OQ-BV2, unreviewed: any member of staff may report and read the
//! answer; a clinician or nurse may read the assertions and take a step; only a clinician may withdraw.
//! The policy table gives the first three and this route checks the last. A patient and a proxy have
//! none of these. **Who is acting is the token, never the body.** A write and its audit row are one
//! transaction; the audit row holds ids, versions and statuses, never a reason or a date.

use super::common::{Access, audit_resource, authorize, begin, commit, purpose_of};
use crate::api_error::ApiError;
use crate::audit::Action;
use crate::auth::{self, Agent};
use crate::death::{self, Actor, Occurred, Source, Status, Step};
use crate::death_store::{self as store, StoredAssertion};
use crate::patients_store::StoreError;
use crate::policy::{self, Purpose, Resource, Role, Subject};
use crate::pool::Pool;
use axa::prelude::*;
use axum::{
    Extension,
    extract::{Json, Path},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Report a death.
#[derive(Debug, Deserialize)]
pub struct ReportRequest {
    /// Where the report came from.
    pub source: Source,
    /// When the person died, or why that is not known.
    pub occurred: Occurred,
    /// What was said, and by whom. Required.
    pub reason: String,
}

/// One step on an assertion.
#[derive(Debug, Deserialize)]
pub struct StepRequest {
    /// The version of the assertion this step is made on.
    pub expected_version: i32,
    /// Where the assertion moves to.
    pub to: Status,
    /// Why. Required.
    pub reason: String,
}

/// Every assertion about a patient.
#[derive(Debug, Serialize)]
pub struct AssertionList {
    /// Newest first, withdrawn ones included.
    pub assertions: Vec<StoredAssertion>,
}

/// A store error as this route's answer: a refused step is a 422 saying what to fix; anything else
/// keeps its own status (a miss is 404, a stale version 409).
fn failure(e: StoreError) -> ApiError {
    match e {
        StoreError::BadQuery(m) => ApiError::invalid(vec![m]),
        other => ApiError::from(&other),
    }
}

/// Audit a refusal of a write or a read (a miss, a stale version, a refused step).
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
            resource: "DeathAssertion",
            patient_id: None,
        },
        serde_json::json!({ "refused": e.to_string() }),
    )
    .await
    .err()
}

/// Who a report is from: an integration's token is a feed, and a feed can never take a later step.
fn reporter(agent: &Agent, subject: &Subject) -> Actor {
    let only_integrator = subject.roles.len() == 1 && subject.roles.contains(&Role::Integrator);
    if only_integrator {
        Actor::Feed(agent.0.clone())
    } else {
        Actor::Person(agent.0.clone())
    }
}

/// Whether this subject may take a step to `to`. Withdrawal is a clinician's (OQ-BV2), and the on-duty
/// clinician is one.
fn may_step(subject: &Subject, to: Status) -> bool {
    !auth::authentication_enforced()
        || to != Status::WithdrawnInError
        || subject.roles.contains(&Role::Clinician)
        || subject.roles.contains(&Role::OnDutyClinician)
}

/// The role accountable for every report on the list (OQ-BV4).
const ACCOUNTABLE: Role = Role::OnDutyClinician;

/// Read the purpose and authorise the list, with no patient named: the list is the same for all who may
/// see it, and a refused consent does not hide a report from the people who must act on it.
async fn reports_guard(
    ctx: &AppContext,
    agent: &Agent,
    subject: &Subject,
    headers: &HeaderMap,
) -> Result<Purpose, ApiError> {
    let purpose = purpose_of(headers)?;
    authorize(
        &ctx.pool(),
        agent,
        subject,
        Resource::DeathReports,
        policy::Action::Search,
        purpose,
        None,
    )
    .await?;
    Ok(purpose)
}

/// Audit a look at the list, with the count and never an id of a patient.
async fn audit_reports(
    ctx: &AppContext,
    agent: &Agent,
    purpose: Purpose,
    waiting: i64,
) -> Result<(), ApiError> {
    audit_resource(
        &ctx.pool(),
        agent,
        Access {
            action: Action::Read,
            outcome: 0,
            purpose,
            resource: "DeathReports",
            patient_id: None,
        },
        serde_json::json!({ "waiting": waiting }),
    )
    .await
}

/// `GET /api/death-reports`: the reports that wait for a person, oldest first. Audited with the count.
pub(super) async fn reports(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
) -> Response {
    let purpose = match reports_guard(&ctx, &agent, &subject, &headers).await {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    let page = match store::waiting(&ctx.pool()).await {
        Ok(p) => p,
        Err(e) => return failure(e).into_response(),
    };
    let seen = i64::try_from(page.len()).unwrap_or(i64::MAX);
    if let Err(r) = audit_reports(&ctx, &agent, purpose, seen).await {
        return r.into_response();
    }
    Json(serde_json::json!({
        "items": page.rows, "limit": store::WAITING_LIMIT, "more": page.more,
        "accountable_role": ACCOUNTABLE.as_str(),
        "site_time_zone": crate::site::time_zone()
    }))
    .into_response()
}

/// `GET /api/death-reports/count`: how many wait. Audited like the list.
pub(super) async fn reports_count(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
) -> Response {
    let purpose = match reports_guard(&ctx, &agent, &subject, &headers).await {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    let open = match store::waiting_count(&ctx.pool()).await {
        Ok(n) => n,
        Err(e) => return failure(e).into_response(),
    };
    if let Err(r) = audit_reports(&ctx, &agent, purpose, open).await {
        return r.into_response();
    }
    Json(serde_json::json!({ "open": open })).into_response()
}

/// `GET /api/patients/{id}/death`: the one answer. Audited.
pub(super) async fn answer(
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
        Resource::DeathAssertion,
        policy::Action::Read,
        purpose,
        Some(&patient),
    )
    .await
    {
        return r.into_response();
    }
    let told = match store::answer(&ctx.pool(), &patient).await {
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
            resource: "DeathAssertion",
            patient_id: Some(&patient),
        },
        serde_json::json!({ "holds_contact": told.holds_contact() }),
    )
    .await
    {
        return r.into_response();
    }
    let mut body = serde_json::to_value(&told).unwrap_or_default();
    body["holds_contact"] = told.holds_contact().into();
    Json(body).into_response()
}

/// `GET /api/patients/{id}/death/assertions`: every assertion, newest first. Audited.
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
        Resource::DeathAssertion,
        policy::Action::Search,
        purpose,
        Some(&patient),
    )
    .await
    {
        return r.into_response();
    }
    let assertions = match store::list(&ctx.pool(), &patient).await {
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
            resource: "DeathAssertion",
            patient_id: Some(&patient),
        },
        serde_json::json!({ "assertions": assertions.len() }),
    )
    .await
    {
        return r.into_response();
    }
    Json(AssertionList { assertions }).into_response()
}

/// `POST /api/patients/{id}/death/assertions`: report a death. The reporter is the token.
pub(super) async fn create(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path(patient): Path<String>,
    Json(request): Json<ReportRequest>,
) -> Response {
    let purpose = match purpose_of(&headers) {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    if let Err(r) = authorize(
        &ctx.pool(),
        &agent,
        &subject,
        Resource::DeathAssertion,
        policy::Action::Create,
        purpose,
        Some(&patient),
    )
    .await
    {
        return r.into_response();
    }
    let made = death::report(
        request.source,
        reporter(&agent, &subject),
        request.occurred,
        &request.reason,
        Utc::now(),
    )
    .map_err(store::refused);
    let mut tx = match begin(&ctx.pool()).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    let written = match made {
        Ok(assertion) => store::report(&mut *tx, &patient, &assertion).await,
        Err(e) => Err(e),
    };
    match written {
        Ok(stored) => {
            if let Err(r) = audit_resource(
                &mut *tx,
                &agent,
                Access {
                    action: Action::Create,
                    outcome: 0,
                    purpose,
                    resource: "DeathAssertion",
                    patient_id: Some(&stored.patient_id),
                },
                serde_json::json!({
                    "assertion": stored.id,
                    "version": stored.version,
                    "status": stored.assertion.status.word(),
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

/// `PUT /api/patients/{id}/death/assertions/{assertion_id}`: one step. A stale `expected_version` is a 409.
pub(super) async fn update(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path((patient, assertion_id)): Path<(String, String)>,
    Json(request): Json<StepRequest>,
) -> Response {
    let purpose = match purpose_of(&headers) {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    if let Err(r) = authorize(
        &ctx.pool(),
        &agent,
        &subject,
        Resource::DeathAssertion,
        policy::Action::Update,
        purpose,
        Some(&patient),
    )
    .await
    {
        return r.into_response();
    }
    if !may_step(&subject, request.to) {
        return ApiError::forbidden().into_response();
    }
    let step = Step {
        to: request.to,
        by: Actor::Person(agent.0.clone()),
        reason: request.reason,
    };
    let mut tx = match begin(&ctx.pool()).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    match store::advance(
        &mut *tx,
        &patient,
        &assertion_id,
        request.expected_version,
        &step,
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
                    resource: "DeathAssertion",
                    patient_id: Some(&stored.patient_id),
                },
                serde_json::json!({
                    "assertion": stored.id,
                    "version": stored.version,
                    "status": stored.assertion.status.word(),
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
            let _ = tx.rollback().await;
            if let Some(r) = refused(&ctx, &agent, purpose, Action::Update, &e).await {
                return r.into_response();
            }
            failure(e).into_response()
        }
    }
}

/// The routes that name no patient. The rest are mounted with the patient's routes.
#[must_use]
pub fn routes() -> Routes {
    Routes::new()
        .add("/api/death-reports", get(reports))
        .add("/api/death-reports/count", get(reports_count))
}
