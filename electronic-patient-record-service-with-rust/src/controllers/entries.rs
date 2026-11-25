//! Problem list routes (EH-1), relative history routes (EH-3), and the reads **as at a time** (EH-2). One set of handlers
//! serves both kinds of entry ([`Entry`]); the route decides which.
//!
//! - `GET  /api/patients/{id}/problems`: every problem in every status, oldest first, with what the list amounts to.
//! - `GET  /api/patients/{id}/problems/as-at?time=`: the list as it stood at a past time.
//! - `POST /api/patients/{id}/problems` `{record, reason?}`: record one.
//! - `PUT  /api/patients/{id}/problems/{problem_id}` `{record, expected_version, reason}`: change it, end it, or mark
//!   it entered in error. A reason is required.
//! - `GET  /api/patients/{id}/problems/{problem_id}/versions`: every version, oldest first.
//! - `GET  /api/patients/{id}/problems/{problem_id}/labels`: what happened to its security labels, oldest first.
//! - `POST /api/patients/{id}/problems/{problem_id}/labels` `{act: add, label, basis}` or
//!   `{act: lift, label, basis, reason}` (OQ-PI7).
//! - the same seven under `/api/patients/{id}/relative-history` and `/api/patients/{id}/compatibility-typing` (DM-1).
//!
//! This is clinical content, so the record-level decision applies in full: the role, a care relationship (or a
//! break-the-glass grant, for a read), the purpose of use and the patient's consent. **It applies as it stands today**:
//! a reader who may not see the list now may not see it as at a time either. **A patient or proxy reads none** (purpose
//! `patient-access` is refused here) until it is decided what a patient sees of a diagnosis they have not been told
//! (OQ-EH1), as for a suspicion. A write and its audit row are one transaction. The audit row records ids, versions,
//! statuses, counts and the time asked for, never a title, a condition, a code, a relative, a reason or a label.
//!
//! **An entry whose labels the reader may not see is withheld** (EH-8, `spec/access-control/masking.md`): the store answers
//! as if it were not there, and the list says how many are withheld.

use super::common::{
    Access, AsAtQuery, Decider, LEFT_OUT, RECONSTRUCTION, audit_resource, authorize, begin, commit,
    purpose_of, with_records,
};
use crate::api_error::ApiError;
use crate::audit::Action;
use crate::auth::Agent;
use crate::entries_store::{
    self as store, Change, Entry, LabelChange, LabelError, Stored, StoredAt,
};
use crate::entry_labels::StoredAct;
use crate::labelling::Refusal;
use crate::masking::Completeness;
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
    /// Whether that is all.
    #[serde(flatten)]
    pub completeness: Completeness,
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
    /// Whether that is all.
    #[serde(flatten)]
    pub completeness: Completeness,
    /// What the list amounted to then.
    pub summary: serde_json::Value,
}

/// One entry's versions.
#[derive(Debug, Serialize)]
pub struct VersionList<R> {
    /// Oldest first.
    pub versions: Vec<Stored<R>>,
}

/// What happened to an entry's labels.
#[derive(Debug, Serialize)]
pub struct LabelActs {
    /// Oldest first.
    pub acts: Vec<StoredAct>,
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
    decider: Decider,
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
        let purpose = purpose_of(headers)?;
        if purpose == Purpose::PatientAccess {
            refused::<R>(ctx, agent, purpose, audit, "patient_access").await?;
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
            purpose,
            Some(patient),
        )
        .await?;
        Ok(Caller {
            ctx,
            agent,
            purpose,
            patient,
            decider: Decider::load(&ctx.pool(), agent, subject, R::RESOURCE, purpose, patient)
                .await?,
        })
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
        refused::<R>(self.ctx, self.agent, self.purpose, action, kind).await
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

/// Audit a refusal by kind only.
async fn refused<R: Entry>(
    ctx: &AppContext,
    agent: &Agent,
    purpose: Purpose,
    action: Action,
    kind: &str,
) -> Result<(), ApiError> {
    let access = Access {
        action,
        outcome: 4,
        purpose,
        resource: R::RESOURCE.name(),
        patient_id: None,
    };
    audit_resource(
        &ctx.pool(),
        agent,
        access,
        serde_json::json!({ "refused": kind }),
    )
    .await
}

/// Add to an audit row how many entries were withheld, when any were.
fn with_withheld(mut detail: serde_json::Value, completeness: Completeness) -> serde_json::Value {
    if let Completeness::Partial { withheld } = completeness {
        crate::json::set(&mut detail, "withheld", serde_json::json!(withheld));
    }
    detail
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
        let viewer = caller.decider.viewer();
        let view = match store::list::<R>(&ctx.pool(), &patient, &viewer).await {
            Ok(view) => view,
            Err(e) => return Err(caller.fail::<R>(Action::Read, e).await),
        };
        caller.decider.note(&viewer);
        let summary = view.summary();
        let detail =
            serde_json::json!({ "entries": view.entries.len(), "summary": summary.get("state") });
        caller
            .read::<R>(with_records(
                with_withheld(detail, view.completeness),
                &patient,
                view.entries.iter().map(|e| e.patient_id.as_str()),
            ))
            .await?;
        Ok(Json(EntryList {
            entries: view.entries,
            completeness: view.completeness,
            summary,
        })
        .into_response())
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
        let viewer = caller.decider.viewer();
        let view = match store::list_at::<R>(&ctx.pool(), &patient, at, &viewer).await {
            Ok(view) => view,
            Err(e) => return Err(caller.fail::<R>(Action::Read, e).await),
        };
        caller.decider.note(&viewer);
        let summary = view.summary();
        let as_at = crate::clock::rfc3339_micros(at);
        caller
            .read::<R>(with_records(
                with_withheld(
                    serde_json::json!({ "entries": view.entries.len(), "as_at": as_at }),
                    view.completeness,
                ),
                &patient,
                view.entries.iter().map(|e| e.stored.patient_id.as_str()),
            ))
            .await?;
        Ok(Json(EntriesAsAt {
            basis: RECONSTRUCTION,
            left_out: LEFT_OUT,
            as_at,
            read_at: crate::clock::rfc3339_micros(now),
            summary,
            completeness: view.completeness,
            entries: view.entries,
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
        let viewer = caller.decider.viewer();
        let versions = match store::history::<R>(&ctx.pool(), &patient, &entry, &viewer).await {
            Ok(all) => all,
            Err(e) => return Err(caller.fail::<R>(Action::Read, e).await),
        };
        caller.decider.note(&viewer);
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
        let viewer = caller.decider.viewer();
        let mut tx = begin(&ctx.pool()).await?;
        let result = store::create(
            &mut *tx,
            &patient,
            &agent.0,
            request.reason.as_deref(),
            &request.record,
            &viewer,
        )
        .await;
        caller.decider.note(&viewer);
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
        let viewer = caller.decider.viewer();
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
            &viewer,
        )
        .await;
        caller.decider.note(&viewer);
        let stored = written(&caller, tx, Action::Update, result).await?;
        Ok(Json(stored).into_response())
    };
    answer(write.await)
}

/// What happened to one entry's labels, oldest first. Audited with the count.
pub(super) async fn labels<R: Entry>(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path((patient, entry)): Path<(String, String)>,
) -> Response {
    let read = async {
        let action = (policy::Action::Read, Action::Read);
        let caller = Caller::gate::<R>(&ctx, &agent, &subject, &headers, &patient, action).await?;
        let viewer = caller.decider.viewer();
        let acts = match store::label_history::<R>(&ctx.pool(), &patient, &entry, &viewer).await {
            Ok(all) => all,
            Err(e) => return Err(caller.fail::<R>(Action::Read, e).await),
        };
        caller.decider.note(&viewer);
        caller
            .read::<R>(
                serde_json::json!({ "entry": entry.to_lowercase(), "label_acts": acts.len() }),
            )
            .await?;
        Ok(Json(LabelActs { acts }).into_response())
    };
    answer(read.await)
}

/// Add or lift a label. Audited with the entry and that its labels changed, and not with which: a label says what it hides.
pub(super) async fn relabel<R: Entry>(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path((patient, entry)): Path<(String, String)>,
    Json(request): Json<LabelChange>,
) -> Response {
    let write = async {
        let action = (policy::Action::Update, Action::Update);
        let caller = Caller::gate::<R>(&ctx, &agent, &subject, &headers, &patient, action).await?;
        let viewer = caller.decider.viewer();
        let mut tx = begin(&ctx.pool()).await?;
        let actor = caller.decider.actor();
        let result =
            store::relabel::<R>(&mut *tx, &patient, &entry, &actor, &request, &viewer).await;
        caller.decider.note(&viewer);
        let (id, owner) = match result {
            Ok(done) => done,
            Err(e) => {
                let _ = tx.rollback().await;
                return Err(label_failure::<R>(&caller, e).await);
            }
        };
        audit_resource(
            &mut *tx,
            caller.agent,
            caller.access::<R>(Action::Update, 0, Some(caller.patient)),
            with_records(
                serde_json::json!({ "entry": id, "labels": "changed" }),
                caller.patient,
                [owner.as_str()],
            ),
        )
        .await?;
        commit(tx).await?;
        Ok(Json(request).into_response())
    };
    answer(write.await)
}

/// A label change that was refused, as the answer, with its audit row first.
async fn label_failure<R: Entry>(caller: &Caller<'_>, e: LabelError) -> ApiError {
    let refusal = match e {
        LabelError::Store(e) => return caller.fail::<R>(Action::Update, e).await,
        LabelError::Unknown => None,
        LabelError::Refused(r) => Some(r),
    };
    if let Err(unaudited) = caller.refused::<R>(Action::Update, "label").await {
        return unaudited;
    }
    let said = refusal.map_or("the label is not one this service knows", Refusal::message);
    match refusal {
        Some(Refusal::NotTreating | Refusal::Own) => ApiError::forbidden().with("reason", said),
        Some(Refusal::AlreadyThere | Refusal::NotThere) => ApiError::conflict(said),
        Some(Refusal::NotTheirs | Refusal::Custody | Refusal::Reason) | None => {
            ApiError::invalid(vec![said.into()])
        }
    }
}
