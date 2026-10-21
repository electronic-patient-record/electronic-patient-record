//! "My record": the patients the signed-in caller is, or is a proxy for.
//!
//! `GET /api/me/patients` lists the records linked to the caller's token `sub` by a `self` or
//! `proxy` relationship (see `relationships.rs`). The purpose of use is fixed at `patient-access`:
//! this endpoint exists for that. Each record still goes through the full record-level decision
//! (role, relationship, consent), so a proxy whose access the patient has refused sees nothing.

use super::patients::{Access, audit_resource, authorize, store_failure, time_now};
use crate::audit::Action;
use crate::auth::Agent;
use crate::patients_store as patients;
use crate::policy::{self, Purpose, Resource, Subject};
use crate::relationships;
use axum::{
    Extension,
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
            Ok(stored) => records.push(serde_json::json!({
                "relationship": kind.as_str(),
                "patient": stored,
            })),
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

/// Routes under `/api/me`.
#[must_use]
pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/me")
        .add("/patients", get(patients_of_caller))
}
