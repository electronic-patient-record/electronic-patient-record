//! Integration tokens (TL-1): issue, list, describe and revoke.
//!
//! - `POST /api/tokens`: `{name, description?, scopes, expires_in_days}`; 201 with the token, **shown once**, and its
//!   metadata. `Cache-Control: no-store`. 409 when the name was ever used.
//! - `GET  /api/tokens`: every token's metadata, newest first: never the token or its hash.
//! - `POST /api/tokens/{token_id}/description`: `{expected_version, description}`.
//! - `POST /api/tokens/{token_id}/revoke`: `{expected_version, reason}`. Final.
//!
//! **Assumed, awaiting the maintainer (OQ-TL4):** the clinical administrator manages tokens (resource `ApiToken`),
//! and any administrator may revoke any token. Every issue, change and revocation is audited with the token's id,
//! name and scopes, never the token; a list is audited with the count. A write and its audit row are one
//! transaction. A token signs in through `auth::authenticate` (`Bearer epr_...`), as `token:<name>` with its scopes.

use super::common::{Access, audit_resource, authorize, begin, commit, purpose_of, store_failure};
use crate::api_error::ApiError;
use crate::api_tokens_store::{self as store, NewToken};
use crate::audit::Action;
use crate::auth::Agent;
use crate::patients_store::StoreError;
use crate::policy::{self, Purpose, Resource, Subject};
use crate::pool::Pool;
use axa::extract::{Json, Path};
use axa::prelude::*;
use axum::{
    Extension,
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::Deserialize;

/// The audit row's resource name.
const RESOURCE: &str = "ApiToken";

/// Issue a token.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IssueRequest {
    /// Its name, which is its audit agent.
    pub name: String,
    /// What it is for.
    #[serde(default)]
    pub description: String,
    /// What it may do.
    pub scopes: Vec<String>,
    /// How many days it lives, 1 to 365.
    pub expires_in_days: i64,
}

/// Change a token's description.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DescribeRequest {
    /// The version read.
    pub expected_version: i32,
    /// The new description.
    pub description: String,
}

/// Revoke a token.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevokeRequest {
    /// The version read.
    pub expected_version: i32,
    /// Why.
    pub reason: String,
}

fn failure(e: StoreError) -> Response {
    match e {
        StoreError::BadQuery(m) => ApiError::invalid(vec![m]).into_response(),
        StoreError::Conflict { current: 0 } => ApiError::new(
            StatusCode::CONFLICT,
            "that name has been used: a token's name is never reused",
        )
        .into_response(),
        StoreError::Conflict { current } => ApiError::new(
            StatusCode::CONFLICT,
            format!("the token has changed: version {current} is current"),
        )
        .into_response(),
        StoreError::NotFound => ApiError::not_found().into_response(),
        other => store_failure(&other),
    }
}

async fn guard(
    ctx: &AppContext,
    agent: &Agent,
    subject: &Subject,
    headers: &HeaderMap,
    action: policy::Action,
) -> Result<Purpose, ApiError> {
    let purpose = purpose_of(headers)?;
    authorize(
        &ctx.pool(),
        agent,
        subject,
        Resource::ApiToken,
        action,
        purpose,
        None,
    )
    .await?;
    Ok(purpose)
}

/// The audit detail of a token: never the token or its hash.
fn detail(view: &store::TokenView, what: &str) -> serde_json::Value {
    serde_json::json!({
        "token": what, "id": view.id, "name": view.name, "version": view.version,
        "scopes": view.scopes, "expires_at": view.expires_at, "revoked": view.revoked,
    })
}

/// `POST /api/tokens`: issue a token. The answer is the only time the token is shown.
async fn issue(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Json(request): Json<IssueRequest>,
) -> Response {
    let purpose = match guard(&ctx, &agent, &subject, &headers, policy::Action::Create).await {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    let new = NewToken {
        name: request.name.trim(),
        description: &request.description,
        scopes: &request.scopes,
        days: request.expires_in_days,
    };
    let problems = store::problems(&new);
    if !problems.is_empty() {
        return ApiError::invalid(problems).into_response();
    }
    let mut tx = match begin(&ctx.pool()).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    match store::issue(&mut tx, &new, &agent.0).await {
        Ok(issued) => {
            if let Err(r) = audit_resource(
                &mut *tx,
                &agent,
                Access {
                    action: Action::Create,
                    outcome: 0,
                    purpose,
                    resource: RESOURCE,
                    patient_id: None,
                },
                detail(&issued.view, "issued"),
            )
            .await
            {
                return r.into_response();
            }
            if let Err(r) = commit(tx).await {
                return r.into_response();
            }
            let mut response = (StatusCode::CREATED, Json(issued)).into_response();
            // The token is in this body and nowhere else: no cache may keep it.
            response
                .headers_mut()
                .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
            response
        }
        Err(e) => {
            let _ = tx.rollback().await;
            failure(e)
        }
    }
}

/// `GET /api/tokens`: every token's metadata. Audited with the count.
async fn list(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
) -> Response {
    let purpose = match guard(&ctx, &agent, &subject, &headers, policy::Action::Search).await {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    let tokens = match store::list(&ctx.pool()).await {
        Ok(t) => t,
        Err(e) => return failure(e),
    };
    if let Err(r) = audit_resource(
        &ctx.pool(),
        &agent,
        Access {
            action: Action::Read,
            outcome: 0,
            purpose,
            resource: RESOURCE,
            patient_id: None,
        },
        serde_json::json!({ "tokens": tokens.len() }),
    )
    .await
    {
        return r.into_response();
    }
    Json(serde_json::json!({ "tokens": tokens, "scopes": store::scope_words() })).into_response()
}

/// A description change or a revocation, audited as an update.
async fn change(
    ctx: &AppContext,
    agent: &Agent,
    subject: &Subject,
    headers: &HeaderMap,
    token_id: &str,
    what: &str,
    run: impl AsyncFnOnce(&mut sqlx::PgConnection) -> Result<store::TokenView, StoreError>,
) -> Response {
    let purpose = match guard(ctx, agent, subject, headers, policy::Action::Update).await {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    let mut tx = match begin(&ctx.pool()).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    match run(&mut tx).await {
        Ok(view) => {
            if let Err(r) = audit_resource(
                &mut *tx,
                agent,
                Access {
                    action: Action::Update,
                    outcome: 0,
                    purpose,
                    resource: RESOURCE,
                    patient_id: None,
                },
                detail(&view, what),
            )
            .await
            {
                return r.into_response();
            }
            if let Err(r) = commit(tx).await {
                return r.into_response();
            }
            Json(view).into_response()
        }
        Err(e) => {
            let _ = tx.rollback().await;
            tracing::debug!(token = %token_id, "token {what} refused");
            failure(e)
        }
    }
}

/// `POST /api/tokens/{token_id}/description`.
async fn describe(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path(token_id): Path<String>,
    Json(request): Json<DescribeRequest>,
) -> Response {
    let by = agent.0.clone();
    change(
        &ctx,
        &agent,
        &subject,
        &headers,
        &token_id,
        "described",
        async |conn| {
            store::describe(
                conn,
                &token_id,
                request.expected_version,
                &request.description,
                &by,
            )
            .await
        },
    )
    .await
}

/// `POST /api/tokens/{token_id}/revoke`.
async fn revoke(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path(token_id): Path<String>,
    Json(request): Json<RevokeRequest>,
) -> Response {
    let by = agent.0.clone();
    change(
        &ctx,
        &agent,
        &subject,
        &headers,
        &token_id,
        "revoked",
        async |conn| {
            store::revoke(
                conn,
                &token_id,
                request.expected_version,
                &request.reason,
                &by,
            )
            .await
        },
    )
    .await
}

/// The token routes.
pub fn routes() -> Routes {
    Routes::new()
        .add("/api/tokens", get(list).post(issue))
        .add("/api/tokens/{token_id}/description", post(describe))
        .add("/api/tokens/{token_id}/revoke", post(revoke))
}
