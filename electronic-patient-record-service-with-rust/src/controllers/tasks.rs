//! Tasks that rules made (TL-3, hazard H-059): a team's list, closing a task, and the dry run of the rules.
//!
//! - `GET  /api/tasks?team=<role>`: the team's open tasks, due first, with `limit` and `more`; each with
//!   `reminded_on`, the day the reminder sender last reminded it, or null.
//! - `POST /api/tasks/{task_id}/close`: `{expected_version, status, reason}`, `done` or `cancelled`, by a person.
//! - `POST /api/tasks/dry-run`: `{event, facts, day?}`, what the service's rules would make for that event, drafts
//!   included and marked; nothing is written.
//!
//! **Assumed, awaiting the maintainer (OQ-TL3):** a team is a role, and only those who hold the role list or close
//! its tasks. Authorised as `Tasks` by role and purpose with no patient named, so no care relationship is checked
//! *here*: a task is a pointer (patient, the rule's own title, due day, the source's kind and id), and the record
//! opens through the patient's routes, which apply the full record-level decision. Every list, close and dry run
//! is audited with ids and counts, never a reason's text. A close and its audit rows are one transaction.

use super::common::{Access, audit_resource, authorize, begin, commit, purpose_of, store_failure};
use crate::api_error::ApiError;
use crate::audit::Action;
use crate::auth::Agent;
use crate::patients_store::StoreError;
use crate::policy::{self, Purpose, Resource, Role, Subject};
use crate::pool::Pool;
use crate::task_rules::{self, Event};
use crate::tasks_store::{self as store, Status};
use axa::extract::{Json, Path};
use axa::prelude::*;
use axum::{
    Extension,
    extract::Query,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use jiff::civil::Date;
use serde::Deserialize;
use std::collections::BTreeMap;

/// The audit row's resource name.
const RESOURCE: &str = "Tasks";

/// The most characters a fact's value may have in a dry run.
const MAX_FACT: usize = 200;

/// `?team=`.
#[derive(Debug, Deserialize)]
pub struct TeamQuery {
    /// The team: a role word, such as `clinician`.
    #[serde(default)]
    pub team: Option<String>,
}

/// Close a task.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CloseRequest {
    /// The version read; anything but the current one is a 409.
    pub expected_version: i32,
    /// `done` or `cancelled`.
    pub status: Status,
    /// Why, 1 to 500 characters.
    pub reason: String,
}

/// What would an event make?
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DryRunRequest {
    /// The event, as a rule names it (`result-filed`, ...).
    pub event: Event,
    /// The event's facts; each must be one the event carries.
    #[serde(default)]
    pub facts: BTreeMap<String, String>,
    /// The site day the event falls on, `YYYY-MM-DD`; today at the site when absent.
    #[serde(default)]
    pub day: Option<String>,
}

/// A store error as a response: 422 for something the caller can fix, 409 for a stale version, 404 for no task.
fn failure(e: StoreError) -> Response {
    match e {
        StoreError::BadQuery(m) => ApiError::invalid(vec![m]).into_response(),
        StoreError::Conflict { current } => ApiError::new(
            StatusCode::CONFLICT,
            format!("the task has changed: version {current} is current"),
        )
        .into_response(),
        StoreError::NotFound => ApiError::not_found().into_response(),
        other => store_failure(&other),
    }
}

/// The role `team` names, when the caller holds it. A team that is no role, or one the caller does not hold, is
/// `None`: the list is theirs alone who hold the role (OQ-TL3).
fn held(subject: &Subject, team: &str) -> Option<Role> {
    subject.roles.iter().copied().find(|r| r.as_str() == team)
}

/// Read the purpose and authorise `action` on `Tasks` with no patient named.
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
        Resource::Tasks,
        action,
        purpose,
        None,
    )
    .await?;
    Ok(purpose)
}

/// Audit a refusal the route makes after the policy allowed it (a team the caller does not hold), then answer 403.
async fn refuse(
    ctx: &AppContext,
    agent: &Agent,
    purpose: Purpose,
    action: Action,
    detail: serde_json::Value,
) -> Response {
    if let Err(r) = audit_resource(
        &ctx.pool(),
        agent,
        Access {
            action,
            outcome: 4,
            purpose,
            resource: RESOURCE,
            patient_id: None,
        },
        detail,
    )
    .await
    {
        return r.into_response();
    }
    ApiError::forbidden().into_response()
}

/// `GET /api/tasks?team=`: the team's open tasks. Audited with the team and the count.
async fn list(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Query(query): Query<TeamQuery>,
) -> Response {
    let purpose = match guard(&ctx, &agent, &subject, &headers, policy::Action::Search).await {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    let Some(team) = query
        .team
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
    else {
        return ApiError::invalid(vec!["team is required: a role, such as clinician".into()])
            .into_response();
    };
    if held(&subject, team).is_none() {
        return refuse(
            &ctx,
            &agent,
            purpose,
            Action::Read,
            serde_json::json!({ "team": team, "refused": "not the caller's role" }),
        )
        .await;
    }
    let (tasks, more) = match store::for_team(&ctx.pool(), team).await {
        Ok(p) => p,
        Err(e) => return failure(e),
    };
    let ids: Vec<String> = tasks.iter().map(|t| t.id.clone()).collect();
    let reminded = match store::reminded_on(&ctx.pool(), &ids).await {
        Ok(r) => r,
        Err(e) => return failure(e),
    };
    // Each task with the day it was last reminded, or null: what the reminder sender has put on the list.
    let items: Vec<serde_json::Value> = tasks
        .iter()
        .map(|t| {
            let mut v = serde_json::to_value(t).unwrap_or_default();
            crate::json::set(
                &mut v,
                "reminded_on",
                reminded
                    .get(&t.id)
                    .map_or(serde_json::Value::Null, |d| d.to_string().into()),
            );
            v
        })
        .collect();
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
        serde_json::json!({ "team": team, "open": items.len(), "more": more }),
    )
    .await
    {
        return r.into_response();
    }
    Json(serde_json::json!({
        "team": team, "items": items, "limit": store::TEAM_LIMIT, "more": more,
        "site_time_zone": crate::site::time_zone()
    }))
    .into_response()
}

/// `POST /api/tasks/{task_id}/close`: a person closes a task of a team whose role they hold, at the version read,
/// with a reason. Another team's task is a 403; a closed task or a blank reason a 422; a stale version a 409.
async fn close(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path(task_id): Path<String>,
    Json(request): Json<CloseRequest>,
) -> Response {
    let purpose = match guard(&ctx, &agent, &subject, &headers, policy::Action::Update).await {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    let task = match store::one(&ctx.pool(), &task_id).await {
        Ok(t) => t,
        Err(e) => return failure(e),
    };
    if held(&subject, &task.team).is_none() {
        return refuse(
            &ctx,
            &agent,
            purpose,
            Action::Update,
            serde_json::json!({ "task": task.id, "team": task.team, "refused": "not the caller's role" }),
        )
        .await;
    }
    let mut tx = match begin(&ctx.pool()).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    match store::close(
        &mut *tx,
        &task.id,
        request.expected_version,
        request.status,
        &agent.0,
        &request.reason,
    )
    .await
    {
        Ok(closed) => {
            if let Err(r) = audit_resource(
                &mut *tx,
                &agent,
                Access {
                    action: Action::Update,
                    outcome: 0,
                    purpose,
                    resource: RESOURCE,
                    patient_id: Some(&closed.patient_id),
                },
                serde_json::json!({
                    "task": closed.id, "version": closed.version, "status": closed.status.word(),
                    "team": closed.team,
                }),
            )
            .await
            {
                return r.into_response();
            }
            if let Err(r) = commit(tx).await {
                return r.into_response();
            }
            Json(closed).into_response()
        }
        Err(e) => {
            let _ = tx.rollback().await;
            failure(e)
        }
    }
}

/// What is wrong with a dry run's facts, if anything: a fact the event does not carry (a typo would match nothing in
/// silence), or a value too long.
fn fact_problems(event: Event, facts: &BTreeMap<String, String>) -> Vec<String> {
    let mut problems = Vec::new();
    for (fact, value) in facts {
        if !event.facts().contains(&fact.as_str()) {
            problems.push(format!(
                "{} carries no fact {fact} (it carries {})",
                event.word(),
                event.facts().join(", ")
            ));
        } else if value.chars().count() > MAX_FACT {
            problems.push(format!(
                "the fact {fact} is longer than {MAX_FACT} characters"
            ));
        }
    }
    problems
}

/// `POST /api/tasks/dry-run`: what the service's rules would make for an event, on the site's calendar, drafts
/// included and marked `approved: false`. Nothing is written but the audit row.
async fn dry_run(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Json(request): Json<DryRunRequest>,
) -> Response {
    let purpose = match guard(&ctx, &agent, &subject, &headers, policy::Action::Read).await {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    let mut problems = fact_problems(request.event, &request.facts);
    let day = match request.day.as_deref() {
        None => Some(crate::site::today()),
        Some(d) => {
            let parsed = (d.len() == 10).then(|| d.parse::<Date>().ok()).flatten();
            if parsed.is_none() {
                problems.push(format!("day `{d}` is not a calendar day (YYYY-MM-DD)"));
            }
            parsed
        }
    };
    let (Some(day), true) = (day, problems.is_empty()) else {
        return ApiError::invalid(problems).into_response();
    };
    let rules = match store::rules() {
        Ok(r) => r,
        Err(e) => return failure(e),
    };
    let Some(planned) = task_rules::plan(
        &rules,
        request.event,
        &request.facts,
        day,
        &crate::site::holidays(),
    ) else {
        return ApiError::invalid(vec![
            "a due or reminder date would be outside the calendar for this day".to_string(),
        ])
        .into_response();
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
        serde_json::json!({ "dry_run": request.event.word(), "planned": planned.len() }),
    )
    .await
    {
        return r.into_response();
    }
    Json(serde_json::json!({
        "event": request.event.word(), "day": day.to_string(), "planned": planned,
        "site_time_zone": crate::site::time_zone(), "site_holidays": crate::site::holidays().len()
    }))
    .into_response()
}

/// The task routes.
pub fn routes() -> Routes {
    Routes::new()
        .add("/api/tasks", get(list))
        .add("/api/tasks/dry-run", post(dry_run))
        .add("/api/tasks/{task_id}/close", post(close))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dry_run_refuses_a_fact_the_event_does_not_carry_and_an_overlong_value() {
        let ok = BTreeMap::from([("abnormal".to_string(), "true".to_string())]);
        assert!(fact_problems(Event::ResultFiled, &ok).is_empty());
        let typo = BTreeMap::from([("abnromal".to_string(), "true".to_string())]);
        let p = fact_problems(Event::ResultFiled, &typo);
        assert!(
            p[0].contains("abnromal") && p[0].contains("abnormal, test"),
            "{p:?}"
        );
        let long = BTreeMap::from([("test".to_string(), "x".repeat(MAX_FACT + 1))]);
        assert!(fact_problems(Event::ResultFiled, &long)[0].contains("longer"));
    }

    #[test]
    fn a_team_is_the_callers_only_when_they_hold_its_role() {
        let subject = Subject {
            roles: [Role::Nurse].into(),
            ..Subject::default()
        };
        assert_eq!(held(&subject, "nurse"), Some(Role::Nurse));
        assert_eq!(held(&subject, "clinician"), None);
        assert_eq!(held(&subject, "team-of-anything"), None);
    }
}
