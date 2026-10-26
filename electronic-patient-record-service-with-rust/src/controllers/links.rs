//! Merge and unmerge routes (task ID-5): `POST /api/patients/{id}/merge`,
//! `POST /api/patients/{id}/unmerge` and `GET /api/patients/{id}/links`.
//!
//! `{id}` is always the record that **survives**; the body names the record merged away. Merging is
//! an administrator's act (`PatientLink` in the policy table); a clinician may read the links. Each
//! change and its audit row are one transaction. See `links.rs` for the rules and what a merge does
//! and does not carry over.

use super::common::{Access, audit_resource, authorize, begin, commit, purpose_of, store_failure};
use crate::api_error::ApiError;
use crate::audit::Action;
use crate::auth::Agent;
use crate::links as store;
use crate::patients_store::StoreError;
use crate::policy::{self, Resource, Subject};
use axum::{
    Extension,
    extract::{Json, Path},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use loco_rs::prelude::*;
use serde::Deserialize;

/// Merge or unmerge another record with this one.
#[derive(Debug, Deserialize)]
pub struct LinkRequest {
    /// The record merged away (or to be separated again).
    pub source: String,
    /// Why. Required: it is the provenance of the change.
    pub reason: String,
}

fn failure(e: StoreError) -> Response {
    match e {
        StoreError::BadQuery(m) => {
            let mut e = ApiError::invalid(vec![m.to_string()]);
            if let Some((code, into)) = store::refusal(&m) {
                e = e.with("refusal", code);
                if let Some(into) = into {
                    e = e.with("refusal_into", into);
                }
            }
            e.into_response()
        }
        other => store_failure(&other),
    }
}

async fn change(
    ctx: &AppContext,
    agent: &Agent,
    subject: &Subject,
    headers: &HeaderMap,
    target: &str,
    request: &LinkRequest,
    merging: bool,
) -> Response {
    let purpose = match purpose_of(headers) {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    let (needed, action) = if merging {
        (policy::Action::Create, Action::Create)
    } else {
        (policy::Action::Update, Action::Update)
    };
    if let Err(r) = authorize(
        &ctx.db,
        agent,
        subject,
        Resource::PatientLink,
        needed,
        purpose,
        Some(target),
    )
    .await
    {
        return r.into_response();
    }
    let tx = match begin(&ctx.db).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    let done = if merging {
        store::merge(&tx, target, &request.source, &agent.0, &request.reason).await
    } else {
        store::unmerge(&tx, target, &request.source, &agent.0, &request.reason).await
    };
    match done {
        Ok(link) => {
            if let Err(r) = audit_resource(
                &tx,
                agent,
                Access {
                    action,
                    outcome: 0,
                    purpose,
                    resource: "PatientLink",
                    patient_id: Some(&link.target),
                },
                // Identifiers and the matcher's verdict, never demographics or the free-text reason.
                serde_json::json!({
                    "link": link.id,
                    "version": link.version,
                    "status": link.status,
                    "source": link.source,
                    "target": link.target,
                    "match_decision": link.match_decision,
                    "match_score": link.match_score,
                }),
            )
            .await
            {
                return r.into_response();
            }
            if let Err(r) = commit(tx).await {
                return r.into_response();
            }
            (
                if merging {
                    StatusCode::CREATED
                } else {
                    StatusCode::OK
                },
                Json(link),
            )
                .into_response()
        }
        Err(e) => {
            let _ = sea_orm::TransactionSession::rollback(tx).await;
            // A refused merge is an access worth recording: who tried to join which records.
            if matches!(
                e,
                StoreError::NotFound | StoreError::BadQuery(_) | StoreError::NotSamePerson { .. }
            ) && let Err(r) = audit_resource(
                &ctx.db,
                agent,
                Access {
                    action,
                    outcome: 4,
                    purpose,
                    resource: "PatientLink",
                    patient_id: None,
                },
                serde_json::json!({ "refused": e.to_string() }),
            )
            .await
            {
                return r.into_response();
            }
            failure(e)
        }
    }
}

pub(super) async fn merge(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path(target): Path<String>,
    Json(request): Json<LinkRequest>,
) -> Response {
    change(&ctx, &agent, &subject, &headers, &target, &request, true).await
}

pub(super) async fn unmerge(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path(target): Path<String>,
    Json(request): Json<LinkRequest>,
) -> Response {
    change(&ctx, &agent, &subject, &headers, &target, &request, false).await
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
        Err(r) => return r.into_response(),
    };
    if let Err(r) = authorize(
        &ctx.db,
        &agent,
        &subject,
        Resource::PatientLink,
        policy::Action::Read,
        purpose,
        Some(&patient),
    )
    .await
    {
        return r.into_response();
    }
    match store::list(&ctx.db, &patient).await {
        Ok(links) => {
            if let Err(r) = audit_resource(
                &ctx.db,
                &agent,
                Access {
                    action: Action::Read,
                    outcome: 0,
                    purpose,
                    resource: "PatientLink",
                    patient_id: links.first().map(|l| l.target.as_str()).or(Some(&patient)),
                },
                serde_json::json!({ "links": links.len() }),
            )
            .await
            {
                return r.into_response();
            }
            Json(serde_json::json!({ "links": links })).into_response()
        }
        Err(e) => failure(e),
    }
}
