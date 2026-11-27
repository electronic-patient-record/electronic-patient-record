//! Who-speaks endpoints (RF-9): record who speaks for a person, list them with what they amount to, and change one as a
//! new version. Registered by `patients::routes` under `/api/patients/{id}/speakers`.
//!
//! **Nothing here decides who wins.** The list says which powers someone saw, which are only claimed (not a power), who has
//! no power, whether "nobody" sits beside a speaker, and on which decisions the powers that were seen disagree. Audit rows
//! carry ids, the version, the role and the state, never names, paper or views.

use super::common::{Access, audit_resource, authorize, begin, commit, purpose_of, store_failure};
use crate::api_error::ApiError;
use crate::audit::Action;
use crate::auth::Agent;
use crate::patients_store::{self, StoreError};
use crate::policy::{self, Resource, Subject};
use crate::pool::Pool;
use crate::speakers_store::{self as store, State as EntryState};
use crate::who_speaks::{self, Speaker};
use axa::extract::{Json, Path};
use axa::prelude::*;
use axum::{
    Extension,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

/// A new entry.
#[derive(Debug, Deserialize)]
pub struct CreateRequest {
    /// The entry.
    pub speaker: Speaker,
    /// Why, for the version history.
    #[serde(default)]
    pub reason: Option<String>,
}

/// A change to an entry: a new version.
#[derive(Debug, Deserialize)]
pub struct UpdateRequest {
    /// The whole new entry.
    pub speaker: Speaker,
    /// The version this change was made from; anything else is a 409.
    pub expected_version: i32,
    /// `stands`, or `entered-in-error` (a clinician's act, with a reason).
    #[serde(default = "stands")]
    pub state: EntryState,
    /// Why, for the version history; required to mark one entered in error.
    #[serde(default)]
    pub reason: Option<String>,
}

fn stands() -> EntryState {
    EntryState::Stands
}

/// One entry in a list.
#[derive(Debug, Serialize)]
pub struct Listed<'a> {
    /// The entry.
    #[serde(flatten)]
    pub stored: &'a store::StoredSpeaker,
    /// True when it is recorded on a record merged into this one.
    pub from_merged_record: bool,
}

/// What the entries that stand amount to, by id. Nothing in it says who decides.
#[derive(Debug, Serialize)]
pub struct Summary {
    /// Entries whose power someone saw.
    pub powers_seen: Vec<String>,
    /// Entries whose power is only claimed: not a power.
    pub claimed_not_seen: Vec<String>,
    /// Relatives and friends with no power: their word is a source.
    pub without_power: Vec<String>,
    /// Someone recorded that nobody speaks for the person.
    pub nobody_recorded: bool,
    /// "Nobody" beside someone who speaks: a person must look.
    pub contradicts: bool,
    /// The decisions on which powers that were seen disagree.
    pub powers_disagree: Vec<String>,
}

fn summary(found: &[store::StoredSpeaker]) -> Summary {
    let standing: Vec<&store::StoredSpeaker> = found
        .iter()
        .filter(|s| s.state == EntryState::Stands)
        .collect();
    let speakers: Vec<Speaker> = standing.iter().map(|s| s.speaker.clone()).collect();
    let s = who_speaks::summarise(&speakers);
    let ids_of = |picked: &[&Speaker]| -> Vec<String> {
        standing
            .iter()
            .zip(&speakers)
            .filter(|(_, sp)| picked.iter().any(|p| std::ptr::eq(*p, *sp)))
            .map(|(st, _)| st.id.clone())
            .collect()
    };
    let mut decisions: Vec<String> = Vec::new();
    for v in speakers.iter().flat_map(|sp| &sp.views) {
        if who_speaks::powers_disagree(&speakers, &v.about)
            && !decisions
                .iter()
                .any(|d| d.eq_ignore_ascii_case(v.about.trim()))
        {
            decisions.push(v.about.trim().to_string());
        }
    }
    Summary {
        powers_seen: ids_of(&s.powers_seen),
        claimed_not_seen: ids_of(&s.claimed_not_seen),
        without_power: ids_of(&s.without_power),
        nobody_recorded: s.nobody_recorded,
        contradicts: s.contradicts,
        powers_disagree: decisions,
    }
}

fn invalid(problems: &[String]) -> Response {
    ApiError::invalid(problems.to_vec()).into_response()
}

fn detail(stored: &store::StoredSpeaker) -> serde_json::Value {
    serde_json::json!({
        "speaker": stored.id, "version": stored.version,
        "role": stored.speaker.role.as_str(), "state": stored.state.as_str(),
    })
}

async fn guard(
    ctx: &AppContext,
    agent: &Agent,
    subject: &Subject,
    headers: &HeaderMap,
    action: policy::Action,
    patient: &str,
) -> Result<policy::Purpose, Box<Response>> {
    let purpose = purpose_of(headers).map_err(|e| Box::new(e.into_response()))?;
    authorize(
        &ctx.pool(),
        agent,
        subject,
        Resource::Speakers,
        action,
        purpose,
        Some(patient),
    )
    .await
    .map_err(|e| Box::new(e.into_response()))?;
    Ok(purpose)
}

/// `POST /api/patients/{id}/speakers`: record who speaks for the person, or that nobody does.
pub(super) async fn create(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path(patient): Path<String>,
    Json(request): Json<CreateRequest>,
) -> Response {
    let purpose = match guard(
        &ctx,
        &agent,
        &subject,
        &headers,
        policy::Action::Create,
        &patient,
    )
    .await
    {
        Ok(p) => p,
        Err(r) => return *r,
    };
    let problems = who_speaks::validate(&request.speaker);
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
        &request.speaker,
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
                    resource: "Speakers",
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

/// `GET /api/patients/{id}/speakers`: every entry on the record and on records merged into it, with what those that stand
/// amount to. Audited with the count.
pub(super) async fn list(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path(patient): Path<String>,
) -> Response {
    let purpose = match guard(
        &ctx,
        &agent,
        &subject,
        &headers,
        policy::Action::Read,
        &patient,
    )
    .await
    {
        Ok(p) => p,
        Err(r) => return *r,
    };
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
                    resource: "Speakers",
                    patient_id: Some(&known.id),
                },
                serde_json::json!({ "speakers": found.len() }),
            )
            .await
            {
                return r.into_response();
            }
            let speakers: Vec<Listed<'_>> = found
                .iter()
                .map(|stored| Listed {
                    stored,
                    from_merged_record: stored.patient_id != known.id,
                })
                .collect();
            Json(serde_json::json!({ "speakers": speakers, "summary": summary(&found) }))
                .into_response()
        }
        Err(e) => store_failure(&e),
    }
}

/// `PUT /api/patients/{id}/speakers/{speaker_id}`: change an entry as a new version, or mark it entered in error (a
/// clinician's act, with a reason). A stale `expected_version` is a 409.
pub(super) async fn update(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path((patient, speaker_id)): Path<(String, String)>,
    Json(request): Json<UpdateRequest>,
) -> Response {
    let needed = if request.state == EntryState::EnteredInError {
        policy::Action::MarkEnteredInError
    } else {
        policy::Action::Update
    };
    let purpose = match guard(&ctx, &agent, &subject, &headers, needed, &patient).await {
        Ok(p) => p,
        Err(r) => return *r,
    };
    let problems = who_speaks::validate(&request.speaker);
    if !problems.is_empty() {
        return invalid(&problems);
    }
    let mut tx = match begin(&ctx.pool()).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    let change = store::Change {
        expected_version: request.expected_version,
        author: &agent.0,
        reason: request.reason.as_deref(),
        speaker: &request.speaker,
        state: request.state,
    };
    match store::update(&mut *tx, &patient, &speaker_id, &change).await {
        Ok(stored) => {
            if let Err(r) = audit_resource(
                &mut *tx,
                &agent,
                Access {
                    action: Action::Update,
                    outcome: 0,
                    purpose,
                    resource: "Speakers",
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
            resource: "Speakers",
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
