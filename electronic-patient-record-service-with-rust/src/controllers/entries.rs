//! Problem list routes (EH-1), relative history routes (EH-3), and the reads **as at a time** (EH-2). One set of handlers
//! serves both kinds of entry ([`Entry`]); the route decides which.
//!
//! - `GET  /api/patients/{id}/problems`: every problem in every status, oldest first, with what the list amounts to.
//! - `GET  /api/patients/{id}/problems/as-at?time=`: the list as it stood at a past time.
//! - `POST /api/patients/{id}/problems` `{record, reason?}`: record one.
//! - `PUT  /api/patients/{id}/problems/{problem_id}` `{record, expected_version, reason}`: change it, end it, or mark
//!   it entered in error. A reason is required.
//! - `GET  /api/patients/{id}/problems/{problem_id}/versions`: every version, oldest first.
//! - the same five under `/api/patients/{id}/relative-history` and `/api/patients/{id}/compatibility-typing` (DM-1).
//!
//! This is clinical content, so the record-level decision applies in full: the role, a care relationship (or a
//! break-the-glass grant, for a read), the purpose of use and the patient's consent. **It applies as it stands today**:
//! a reader who may not see the list now may not see it as at a time either. **A patient or proxy reads none** (purpose
//! `patient-access` is refused here) until it is decided what a patient sees of a diagnosis they have not been told
//! (OQ-EH1), as for a suspicion. A write and its audit row are one transaction. The audit row records ids, versions,
//! statuses, counts and the time asked for, never a title, a condition, a code, a relative or a reason.

use super::common::{
    Access, AsAtQuery, LEFT_OUT, RECONSTRUCTION, audit_resource, authorize, begin, commit,
    purpose_of, with_records,
};
use crate::api_error::ApiError;
use crate::audit::Action;
use crate::auth::Agent;
use crate::entries_store::{self as store, Change, Entry, Stored, StoredAt};
use crate::patients_store::StoreError;
use crate::policy::{self, Purpose, Subject};
use crate::pool::Pool;
use axa::extract::{Json, Path, Query};
use axa::prelude::*;
use axum::{
    Extension,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

/// Record an entry.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateRequest<R> {
    /// The record.
    pub record: R,
    /// Why, for the version history.
    #[serde(default = "Option::default")]
    pub reason: Option<String>,
}

/// Change an entry: a new version.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateRequest<R> {
    /// The whole new record.
    pub record: R,
    /// The version this change was made from; anything else is a 409.
    pub expected_version: i32,
    /// Why. Required, and refused with 422 when it is missing.
    #[serde(default = "Option::default")]
    pub reason: Option<String>,
}

/// A patient's entries.
#[derive(Debug, Serialize)]
pub struct EntryList<R> {
    /// Every entry in every status, oldest first.
    pub entries: Vec<Stored<R>>,
    /// What the list amounts to. An empty list is `not-recorded`, never none.
    pub summary: serde_json::Value,
}

/// A patient's entries as they stood at a past time.
#[derive(Debug, Serialize)]
pub struct EntriesAsAt<R> {
    /// What this is: rebuilt from stored versions, never what was shown (AG-7).
    pub basis: &'static str,
    /// What it leaves out.
    pub left_out: [&'static str; 5],
    /// The time asked for (UTC).
    pub as_at: String,
    /// When this was read (UTC). It is not the record as it stands now.
    pub read_at: String,
    /// Every entry that had been written by then, at its version of then.
    pub entries: Vec<StoredAt<R>>,
    /// What the list amounted to then.
    pub summary: serde_json::Value,
}

/// One entry's versions.
#[derive(Debug, Serialize)]
pub struct VersionList<R> {
    /// Oldest first.
    pub versions: Vec<Stored<R>>,
}

/// A store error as this route's answer: a refused record is a 422 listing what to fix; anything else keeps its own
/// status through `ApiError::from` (a miss is 404, a stale version 409).
fn failure(e: StoreError) -> ApiError {
    match e {
        StoreError::BadQuery(m) => ApiError::invalid(vec![m]),
        other => ApiError::from(&other),
    }
}

/// Who is asking, for what purpose, about which patient.
struct Caller<'a> {
    ctx: &'a AppContext,
    agent: &'a Agent,
    purpose: Purpose,
    patient: &'a str,
}

impl<'a> Caller<'a> {
    /// The purpose, the patient-access refusal, and the policy decision, in that order.
    async fn gate<R: Entry>(
        ctx: &'a AppContext,
        agent: &'a Agent,
        subject: &Subject,
        headers: &HeaderMap,
        patient: &'a str,
        (action, audit): (policy::Action, Action),
    ) -> Result<Caller<'a>, ApiError> {
        let caller = Caller {
            ctx,
            agent,
            purpose: purpose_of(headers)?,
            patient,
        };
        if caller.purpose == Purpose::PatientAccess {
            caller.refused::<R>(audit, "patient_access").await?;
            return Err(ApiError::forbidden().with(
                "reason",
                "a patient's view of this list is not built (OQ-EH1)",
            ));
        }
        authorize(
            &ctx.pool(),
            agent,
            subject,
            R::RESOURCE,
            action,
            caller.purpose,
            Some(patient),
        )
        .await?;
        Ok(caller)
    }

    /// An audit row for this access.
    fn access<R: Entry>(
        &self,
        action: Action,
        outcome: i16,
        patient_id: Option<&'a str>,
    ) -> Access<'a> {
        Access {
            action,
            outcome,
            purpose: self.purpose,
            resource: R::RESOURCE.name(),
            patient_id,
        }
    }

    /// Audit a refusal by kind only.
    async fn refused<R: Entry>(&self, action: Action, kind: &str) -> Result<(), ApiError> {
        audit_resource(
            &self.ctx.pool(),
            self.agent,
            self.access::<R>(action, 4, None),
            serde_json::json!({ "refused": kind }),
        )
        .await
    }

    /// A store error as the answer, with its audit row first.
    async fn fail<R: Entry>(&self, action: Action, e: StoreError) -> ApiError {
        if matches!(
            e,
            StoreError::NotFound | StoreError::Conflict { .. } | StoreError::BadQuery(_)
        ) && let Err(unaudited) = self.refused::<R>(action, e.refusal_kind()).await
        {
            return unaudited;
        }
        failure(e)
    }

    /// The audit row of a read, before the clinical content leaves.
    async fn read<R: Entry>(&self, detail: serde_json::Value) -> Result<(), ApiError> {
        audit_resource(
            &self.ctx.pool(),
            self.agent,
            self.access::<R>(Action::Read, 0, Some(self.patient)),
            detail,
        )
        .await
    }
}

/// Any answer, or the error's.
fn answer(result: Result<Response, ApiError>) -> Response {
    result.unwrap_or_else(IntoResponse::into_response)
}

/// Every entry in every status. Audited with the count and the state.
pub(super) async fn list<R: Entry>(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path(patient): Path<String>,
) -> Response {
    let read = async {
        let action = (policy::Action::Read, Action::Read);
        let caller = Caller::gate::<R>(&ctx, &agent, &subject, &headers, &patient, action).await?;
        let entries = match store::list::<R>(&ctx.pool(), &patient).await {
            Ok(all) => all,
            Err(e) => return Err(caller.fail::<R>(Action::Read, e).await),
        };
        let summary = R::summary(&entries.iter().collect::<Vec<_>>());
        caller
            .read::<R>(with_records(
                serde_json::json!({ "entries": entries.len(), "summary": summary.get("state") }),
                &patient,
                entries.iter().map(|e| e.patient_id.as_str()),
            ))
            .await?;
        Ok(Json(EntryList { entries, summary }).into_response())
    };
    answer(read.await)
}

/// The entries as they stood at `time`. The access decision is today's. Audited with the time asked for.
pub(super) async fn as_at<R: Entry>(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path(patient): Path<String>,
    Query(query): Query<AsAtQuery>,
) -> Response {
    let read = async {
        let action = (policy::Action::Read, Action::Read);
        let caller = Caller::gate::<R>(&ctx, &agent, &subject, &headers, &patient, action).await?;
        let now = jiff::Timestamp::now();
        let at = query.instant(now)?;
        let entries = match store::list_at::<R>(&ctx.pool(), &patient, at).await {
            Ok(all) => all,
            Err(e) => return Err(caller.fail::<R>(Action::Read, e).await),
        };
        let summary = R::summary(&entries.iter().map(|s| &s.stored).collect::<Vec<_>>());
        let as_at = crate::clock::rfc3339_micros(at);
        caller
            .read::<R>(with_records(
                serde_json::json!({ "entries": entries.len(), "as_at": as_at }),
                &patient,
                entries.iter().map(|e| e.stored.patient_id.as_str()),
            ))
            .await?;
        Ok(Json(EntriesAsAt {
            basis: RECONSTRUCTION,
            left_out: LEFT_OUT,
            as_at,
            read_at: crate::clock::rfc3339_micros(now),
            summary,
            entries,
        })
        .into_response())
    };
    answer(read.await)
}

/// Every version of one entry, oldest first, each with its author and reason.
pub(super) async fn versions<R: Entry>(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path((patient, entry)): Path<(String, String)>,
) -> Response {
    let read = async {
        let action = (policy::Action::Read, Action::Read);
        let caller = Caller::gate::<R>(&ctx, &agent, &subject, &headers, &patient, action).await?;
        let versions = match store::history::<R>(&ctx.pool(), &patient, &entry).await {
            Ok(all) => all,
            Err(e) => return Err(caller.fail::<R>(Action::Read, e).await),
        };
        let id = versions.first().map(|v| v.id.clone());
        caller
            .read::<R>(with_records(
                serde_json::json!({ "entry": id, "versions": versions.len() }),
                &patient,
                versions.iter().map(|v| v.patient_id.as_str()),
            ))
            .await?;
        Ok(Json(VersionList { versions }).into_response())
    };
    answer(read.await)
}

/// Commit a write with its audit row, or roll it back and audit the refusal.
async fn written<R: Entry>(
    caller: &Caller<'_>,
    mut tx: sqlx::Transaction<'static, sqlx::Postgres>,
    action: Action,
    result: Result<Stored<R>, StoreError>,
) -> Result<Stored<R>, ApiError> {
    let stored = match result {
        Ok(stored) => stored,
        Err(e) => {
            let _ = tx.rollback().await;
            return Err(caller.fail::<R>(action, e).await);
        }
    };
    audit_resource(
        &mut *tx,
        caller.agent,
        Access {
            action,
            outcome: 0,
            purpose: caller.purpose,
            resource: R::RESOURCE.name(),
            patient_id: Some(caller.patient),
        },
        with_records(
            serde_json::json!({
                "entry": stored.id,
                "version": stored.version,
                "status": stored.record.status(),
            }),
            caller.patient,
            [stored.patient_id.as_str()],
        ),
    )
    .await?;
    commit(tx).await?;
    Ok(stored)
}

/// Record an entry. 201 with the stored first version.
pub(super) async fn create<R: Entry>(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path(patient): Path<String>,
    Json(request): Json<CreateRequest<R>>,
) -> Response {
    let write = async {
        let action = (policy::Action::Create, Action::Create);
        let caller = Caller::gate::<R>(&ctx, &agent, &subject, &headers, &patient, action).await?;
        let mut tx = begin(&ctx.pool()).await?;
        let result = store::create(
            &mut *tx,
            &patient,
            &agent.0,
            request.reason.as_deref(),
            &request.record,
        )
        .await;
        let stored = written(&caller, tx, Action::Create, result).await?;
        Ok((StatusCode::CREATED, Json(stored)).into_response())
    };
    answer(write.await)
}

/// Change, end or retract an entry as a new version. Marking it entered in error is its own permission, so a nurse may
/// change an entry and not retract one. A stale `expected_version` is a 409.
pub(super) async fn update<R: Entry>(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path((patient, entry)): Path<(String, String)>,
    Json(request): Json<UpdateRequest<R>>,
) -> Response {
    let write = async {
        let needed = if request.record.in_error() {
            policy::Action::MarkEnteredInError
        } else {
            policy::Action::Update
        };
        let action = (needed, Action::Update);
        let caller = Caller::gate::<R>(&ctx, &agent, &subject, &headers, &patient, action).await?;
        let mut tx = begin(&ctx.pool()).await?;
        let result = store::update(
            &mut *tx,
            &patient,
            &entry,
            Change {
                expected_version: request.expected_version,
                author: &agent.0,
                reason: request.reason.as_deref(),
                record: &request.record,
            },
        )
        .await;
        let stored = written(&caller, tx, Action::Update, result).await?;
        Ok(Json(stored).into_response())
    };
    answer(write.await)
}
