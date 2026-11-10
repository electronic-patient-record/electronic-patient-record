//! Watch list routes (WL-6).
//!
//! - `GET  /api/patients/{id}/watch-list`: who is watching this patient and why: every entry, active
//!   first, ended and lapsed ones kept.
//! - `POST /api/patients/{id}/watch-list` `{entry: {reason, until?}}`: put the patient on one's own list;
//!   409 if already on it.
//! - `PUT  /api/patients/{id}/watch-list/{entry_id}` `{expected_version, end_reason}`: end one's own entry.
//! - `GET  /api/watch-list`: one's own active entries across patients, as pointers with no reason.
//!
//! The patient routes are clinical (`WatchList`): role, care relationship, purpose and consent apply. The
//! own list (`OwnWatchList`) needs no relationship because it holds only ids and days; a reason is read
//! through the patient's route, under the full decision. Audit rows hold ids, versions, states and
//! counts, never a reason.

use super::common::{Access, audit_resource, authorize, begin, commit, purpose_of};
use crate::api_error::ApiError;
use crate::audit::Action;
use crate::auth::Agent;
use crate::patients_store::StoreError;
use crate::policy::{self, Purpose, Resource, Subject};
use crate::pool::Pool;
use crate::watch_list::NewEntry;
use crate::watch_list_store::{self as store, Entry, Pointer};
use axa::extract::{Json, Path};
use axa::prelude::*;
use axum::{
    Extension,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

/// Add a patient to one's list.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AddRequest {
    /// The entry.
    pub entry: NewEntry,
}

/// End one's own entry.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EndRequest {
    /// The version this was made from; anything else is a 409.
    pub expected_version: i32,
    /// Why it ends.
    #[serde(default)]
    pub end_reason: Option<String>,
}

/// A patient's entries.
#[derive(Debug, Serialize)]
pub struct EntryList {
    /// Active first, then newest first.
    pub entries: Vec<Entry>,
}

/// One's own list.
#[derive(Debug, Serialize)]
pub struct OwnList {
    /// Oldest first.
    pub items: Vec<Pointer>,
    /// The most returned.
    pub limit: i64,
    /// Whether any were left out.
    pub more: bool,
    /// Entries whose last day has passed and that the owner has not ended, oldest last day first.
    pub lapsed: Vec<Pointer>,
    /// Whether any lapsed ones were left out.
    pub lapsed_more: bool,
}

const RESOURCE: &str = "WatchList";

/// The site's day (`EPR_SITE_TIME_ZONE`): a last day lapses when it has passed there.
fn today() -> chrono::NaiveDate {
    crate::site::today()
}

fn failure(e: StoreError) -> ApiError {
    match e {
        StoreError::BadQuery(m) => ApiError::invalid(vec![m]),
        other => ApiError::from(&other),
    }
}

async fn refused(
    ctx: &AppContext,
    agent: &Agent,
    purpose: Purpose,
    action: Action,
    e: &StoreError,
) -> Option<ApiError> {
    let kind = match e {
        StoreError::NotFound => "not_found",
        StoreError::Conflict { .. } => "version_conflict",
        StoreError::BadQuery(_) => "entry_refused",
        _ => return None,
    };
    audit_resource(
        &ctx.pool(),
        agent,
        Access {
            action,
            outcome: 4,
            purpose,
            resource: RESOURCE,
            patient_id: None,
        },
        serde_json::json!({ "refused": kind }),
    )
    .await
    .err()
}

async fn gate(
    ctx: &AppContext,
    agent: &Agent,
    subject: &Subject,
    headers: &HeaderMap,
    patient: &str,
    action: policy::Action,
) -> Result<Purpose, ApiError> {
    let purpose = purpose_of(headers)?;
    authorize(
        &ctx.pool(),
        agent,
        subject,
        Resource::WatchList,
        action,
        purpose,
        Some(patient),
    )
    .await?;
    Ok(purpose)
}

/// `GET /api/patients/{id}/watch-list`. Audited with the counts.
pub(super) async fn for_patient(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path(patient): Path<String>,
) -> Response {
    let purpose = match gate(
        &ctx,
        &agent,
        &subject,
        &headers,
        &patient,
        policy::Action::Search,
    )
    .await
    {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    let entries = match store::for_patient(&ctx.pool(), &patient, today()).await {
        Ok(e) => e,
        Err(e) => {
            if let Some(r) = refused(&ctx, &agent, purpose, Action::Read, &e).await {
                return r.into_response();
            }
            return failure(e).into_response();
        }
    };
    let active = entries
        .iter()
        .filter(|e| e.state == crate::watch_list::State::Active)
        .count();
    if let Err(r) = audit_resource(
        &ctx.pool(),
        &agent,
        Access {
            action: Action::Read,
            outcome: 0,
            purpose,
            resource: RESOURCE,
            patient_id: Some(&patient),
        },
        serde_json::json!({ "entries": entries.len(), "active": active }),
    )
    .await
    {
        return r.into_response();
    }
    Json(EntryList { entries }).into_response()
}

/// `POST /api/patients/{id}/watch-list`: 201 with the entry, on the caller's own list.
pub(super) async fn add(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path(patient): Path<String>,
    Json(request): Json<AddRequest>,
) -> Response {
    let purpose = match gate(
        &ctx,
        &agent,
        &subject,
        &headers,
        &patient,
        policy::Action::Create,
    )
    .await
    {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    let mut tx = match begin(&ctx.pool()).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    let result = store::add(&mut *tx, &patient, &agent.0, &request.entry, today()).await;
    finish(
        &ctx,
        &agent,
        purpose,
        Action::Create,
        tx,
        result,
        StatusCode::CREATED,
    )
    .await
}

/// `PUT /api/patients/{id}/watch-list/{entry_id}`: end one's own entry; 200 with the ended entry.
pub(super) async fn end(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path((patient, entry_id)): Path<(String, String)>,
    Json(request): Json<EndRequest>,
) -> Response {
    let purpose = match gate(
        &ctx,
        &agent,
        &subject,
        &headers,
        &patient,
        policy::Action::Update,
    )
    .await
    {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    let mut tx = match begin(&ctx.pool()).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    let result = store::end(
        &mut *tx,
        &patient,
        &entry_id,
        &agent.0,
        request.expected_version,
        request.end_reason.as_deref(),
        today(),
    )
    .await;
    finish(
        &ctx,
        &agent,
        purpose,
        Action::Update,
        tx,
        result,
        StatusCode::OK,
    )
    .await
}

async fn finish(
    ctx: &AppContext,
    agent: &Agent,
    purpose: Purpose,
    action: Action,
    mut tx: sqlx::Transaction<'static, sqlx::Postgres>,
    result: Result<Entry, StoreError>,
    status: StatusCode,
) -> Response {
    match result {
        Ok(entry) => {
            if let Err(r) = audit_resource(
                &mut *tx,
                agent,
                Access {
                    action,
                    outcome: 0,
                    purpose,
                    resource: RESOURCE,
                    patient_id: Some(&entry.patient_id),
                },
                serde_json::json!({
                    "entry": entry.id,
                    "version": entry.version,
                    "state": entry.state.word(),
                }),
            )
            .await
            {
                return r.into_response();
            }
            if let Err(r) = commit(tx).await {
                return r.into_response();
            }
            (status, Json(entry)).into_response()
        }
        Err(e) => {
            let _ = tx.rollback().await;
            if let Some(r) = refused(ctx, agent, purpose, action, &e).await {
                return r.into_response();
            }
            failure(e).into_response()
        }
    }
}

/// `GET /api/watch-list`: the caller's own active entries, as pointers. Audited with the count.
pub(super) async fn own(
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
        &ctx.pool(),
        &agent,
        &subject,
        Resource::OwnWatchList,
        policy::Action::Search,
        purpose,
        None,
    )
    .await
    {
        return r.into_response();
    }
    let (items, more) = match store::own(&ctx.pool(), &agent.0, today()).await {
        Ok(p) => p,
        Err(e) => return failure(e).into_response(),
    };
    let (lapsed, lapsed_more) = match store::lapsed(&ctx.pool(), &agent.0, today()).await {
        Ok(p) => p,
        Err(e) => return failure(e).into_response(),
    };
    if let Err(r) = audit_resource(
        &ctx.pool(),
        &agent,
        Access {
            action: Action::Read,
            outcome: 0,
            purpose,
            resource: "OwnWatchList",
            patient_id: None,
        },
        serde_json::json!({ "entries": items.len(), "more": more, "lapsed": lapsed.len() }),
    )
    .await
    {
        return r.into_response();
    }
    Json(OwnList {
        items,
        limit: store::OWN_LIMIT,
        more,
        lapsed,
        lapsed_more,
    })
    .into_response()
}

/// The own-list route; the patient routes are added in `patients.rs`.
pub fn routes() -> Routes {
    Routes::new().add("/api/watch-list", get(own))
}
