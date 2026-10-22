//! "My record": the patients the signed-in caller is, or is a proxy for.
//!
//! `GET /api/me/patients` lists the records linked to the caller's token `sub` by a `self` or
//! `proxy` relationship (see `relationships.rs`). The purpose of use is fixed at `patient-access`:
//! this endpoint exists for that. Each record still goes through the full record-level decision
//! (role, relationship, consent), so a proxy whose access the patient has refused sees nothing.
//!
//! `GET /api/me/patients/{id}/access-log` is the patient-visible access log (task SE-6): who
//! touched this record and when. See [`access_log`].

use super::common::{Access, audit_failed, audit_resource, authorize, store_failure, time_now};
use crate::api_error::ApiError;
use crate::audit::{self, Action, PatientLogRow};
use crate::auth::Agent;
use crate::patients_store as patients;
use crate::policy::{self, Purpose, Relationship, Resource, Subject};
use crate::relationships;
use axum::{
    Extension,
    extract::{Path, Query},
    response::{IntoResponse, Response},
};
use loco_rs::prelude::*;

async fn patients_of_caller(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
) -> Response {
    let purpose = Purpose::PatientAccess;
    let linked = match relationships::own_patients(&ctx.db, &agent.0, time_now()).await {
        Ok(l) => l,
        Err(e) => return store_failure(&e),
    };
    let mut records = Vec::new();
    for (id, kind) in &linked {
        // A refusal here is audited by `authorize`; the patient is simply left out.
        if authorize(
            &ctx.db,
            &agent,
            &subject,
            Resource::PatientIdentity,
            policy::Action::Read,
            purpose,
            Some(id),
        )
        .await
        .is_err()
        {
            continue;
        }
        match patients::current(&ctx.db, id).await {
            Ok(stored) => {
                let photo_version = match crate::photos_store::current(&ctx.db, &stored.id).await {
                    Ok(p) => p.filter(|p| p.bytes.is_some()).map(|p| p.version),
                    Err(e) => return store_failure(&e),
                };
                records.push(serde_json::json!({
                    "relationship": kind.as_str(),
                    "patient": stored,
                    "photo_version": photo_version,
                }));
            }
            Err(e) => return store_failure(&e),
        }
    }
    if let Err(r) = audit_resource(
        &ctx.db,
        &agent,
        Access {
            action: Action::Read,
            outcome: 0,
            purpose,
            resource: "Patient",
            patient_id: None,
        },
        serde_json::json!({ "own_record": true, "linked": linked.len(), "shown": records.len() }),
    )
    .await
    {
        return *r;
    }
    axum::Json(serde_json::json!({ "records": records })).into_response()
}

/// Entries per page of the access log.
const LOG_PAGE: i64 = 50;

/// Page control for the access log.
#[derive(Debug, serde::Deserialize)]
struct LogQuery {
    /// Return entries older than this `id` (the `next` of the previous page).
    before: Option<i64>,
}

/// One access, in words a patient can read. Who did it is never a staff identifier: `you` is the
/// caller's own activity, everything else is `staff` with the roles and organisations the token
/// carried when the row was written (none for older rows).
fn entry(row: &PatientLogRow, caller: &str) -> serde_json::Value {
    let refused = row.outcome != 0;
    let what = match row.resource.as_deref().unwrap_or("") {
        r if r.starts_with("Consent") => "consent choices",
        r if r.starts_with("CareRelationship") => "care relationships",
        r if r.starts_with("Patient") => "identity record",
        _ => "record",
    };
    let did = match (refused, row.action.as_str()) {
        (true, _) => "was refused access to",
        (_, "C") => "created",
        (_, "R") => "viewed",
        (_, "U") => "changed",
        (_, "D") => "removed",
        _ => "used",
    };
    serde_json::json!({
        "id": row.id,
        "at": row.at,
        "by": if row.agent == caller { "you" } else { "staff" },
        // What the token said at the time. Empty for older rows: the page then says "staff".
        "roles": if row.agent == caller { Vec::new() } else { row.roles.clone() },
        "organisations": if row.agent == caller { Vec::new() } else { row.organisations.clone() },
        "did": did,
        "what": what,
        "refused": refused,
        "purpose": row.purpose,
        "break_glass": row.break_glass,
    })
}

/// Who accessed this record, newest first, 50 at a time (`?before=<next>` for older).
///
/// Only the patient and a proxy may read it: the caller needs a `self` or `proxy` relationship in
/// force to the patient (a clinician's treating relationship is not enough), and goes through the
/// same record-level decision as reading the record, with purpose `patient-access`. Refusals show
/// as entries too, since an attempt is something a patient may want to know. Reading the log is
/// itself audited, so it appears in the next reading as `you`.
async fn access_log(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    Path(id): Path<String>,
    Query(query): Query<LogQuery>,
) -> Response {
    let purpose = Purpose::PatientAccess;
    if let Err(r) = authorize(
        &ctx.db,
        &agent,
        &subject,
        Resource::PatientIdentity,
        policy::Action::Read,
        purpose,
        Some(&id),
    )
    .await
    {
        return *r;
    }
    // `authorize` allows a treating relationship to read the identity record; the log is for the
    // patient and proxy only.
    match relationships::lookup(&ctx.db, &id, &agent.0, time_now()).await {
        Ok(Relationship::SelfPatient | Relationship::Proxy) => {}
        Ok(_) => {
            let _ = audit_resource(
                &ctx.db,
                &agent,
                Access {
                    action: Action::Execute,
                    outcome: 4,
                    purpose,
                    resource: "AccessLog",
                    patient_id: Some(&id),
                },
                serde_json::json!({ "denied": "not-patient-or-proxy" }),
            )
            .await;
            return ApiError::forbidden().into_response();
        }
        Err(e) => return store_failure(&e),
    }
    // One more than a page, to know whether there is another.
    let mut rows = match audit::patient_log(&ctx.db, &id, query.before, LOG_PAGE + 1).await {
        Ok(r) => r,
        Err(e) => return audit_failed(&e),
    };
    let more = rows.len() > usize::try_from(LOG_PAGE).unwrap_or(usize::MAX);
    rows.truncate(usize::try_from(LOG_PAGE).unwrap_or(usize::MAX));
    let entries: Vec<_> = rows.iter().map(|r| entry(r, &agent.0)).collect();
    let next = if more {
        rows.last().map(|r| r.id)
    } else {
        None
    };
    if let Err(r) = audit_resource(
        &ctx.db,
        &agent,
        Access {
            action: Action::Read,
            outcome: 0,
            purpose,
            resource: "AccessLog",
            patient_id: Some(&id),
        },
        serde_json::json!({ "entries": entries.len() }),
    )
    .await
    {
        return *r;
    }
    axum::Json(serde_json::json!({ "entries": entries, "next": next })).into_response()
}

/// Routes under `/api/me`.
#[must_use]
pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/me")
        .add("/patients", get(patients_of_caller))
        .add("/patients/{id}/access-log", get(access_log))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(agent: &str, action: &str, outcome: i16, resource: &str) -> PatientLogRow {
        PatientLogRow {
            id: 7,
            at: "2026-10-21T09:00:00.000000Z".into(),
            action: action.into(),
            outcome,
            agent: agent.into(),
            resource: Some(resource.into()),
            purpose: Some("treatment".into()),
            break_glass: false,
            roles: vec!["clinician".into()],
            organisations: vec!["riverside".into()],
        }
    }

    #[test]
    fn a_clinicians_view_reads_as_staff_viewed_and_never_names_them() {
        let e = entry(&row("clinician-sub-123", "R", 0, "Patient"), "patient-sub");
        assert_eq!(e["by"], "staff");
        assert_eq!(e["roles"], serde_json::json!(["clinician"]));
        assert_eq!(e["organisations"], serde_json::json!(["riverside"]));
        assert_eq!(e["did"], "viewed");
        assert_eq!(e["what"], "identity record");
        assert_eq!(e["refused"], false);
        assert_eq!(e["purpose"], "treatment");
        assert!(!e.to_string().contains("clinician-sub-123"));
    }

    #[test]
    fn the_callers_own_activity_is_you_and_refusals_are_marked() {
        let own = entry(&row("patient-sub", "R", 0, "AccessLog"), "patient-sub");
        assert_eq!(own["by"], "you");
        assert_eq!(own["roles"], serde_json::json!([]));
        let refused = entry(&row("x", "E", 4, "PatientIdentity/Read"), "patient-sub");
        assert_eq!(refused["refused"], true);
        assert_eq!(refused["did"], "was refused access to");
        let consent = entry(&row("x", "U", 0, "Consent"), "patient-sub");
        assert_eq!(
            (consent["did"].clone(), consent["what"].clone()),
            ("changed".into(), "consent choices".into())
        );
    }
}
