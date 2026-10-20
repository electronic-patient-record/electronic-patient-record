//! Patient identity endpoints. The request and response bodies are the
//! matcher's own types: there is no adapter layer.

use crate::audit::{self, Action, AuditError, Chain, Event};
use crate::auth::Agent;
use axum::{Extension, extract::Json, response::Response};
use electronic_patient_record_matcher::{MatchResult, MatchingEngine, Patient};
use loco_rs::prelude::*;
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};

/// Two patients to compare.
#[derive(Debug, Serialize, Deserialize)]
pub struct CompareRequest {
    /// First patient.
    pub a: Patient,
    /// Second patient.
    pub b: Patient,
}

/// Compare two supplied patients with the default configuration. Pure: it
/// reads nothing from the database, so there is nothing to audit yet.
#[must_use]
pub fn compare_patients(request: &CompareRequest) -> MatchResult {
    MatchingEngine::default().compare(&request.a, &request.b)
}

/// Compare, and audit the access **before** returning the result. The audit detail records the
/// decision and score, never the demographics supplied. If the audit cannot be written, no
/// result is returned: an access that cannot be audited does not happen.
///
/// # Errors
///
/// When the audit row cannot be appended.
pub async fn compare_audited(
    db: &DatabaseConnection,
    chain: &Chain,
    agent: &Agent,
    request: &CompareRequest,
) -> Result<MatchResult, AuditError> {
    let result = compare_patients(request);
    let mut event = Event::new(Action::Execute, agent.0.clone());
    event.resource_ref = Some("Patient/$compare".to_string());
    event.detail = Some(serde_json::json!({
        "decision": result.decision,
        "score": result.score,
    }));
    audit::append(db, chain, &event).await?;
    Ok(result)
}

async fn compare(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Json(request): Json<CompareRequest>,
) -> Result<Response> {
    match compare_audited(&ctx.db, audit::chain(), &agent, &request).await {
        Ok(result) => format::json(result),
        Err(e) => {
            tracing::error!(error = %e, "refusing to answer: the access could not be audited");
            Err(Error::InternalServerError)
        }
    }
}

/// Routes under `/api/patients`.
#[must_use]
pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/patients")
        .add("/compare", post(compare))
}

#[cfg(test)]
mod tests {
    use super::*;
    use electronic_patient_record_matcher::{Decision, HumanName};

    fn p(family: &str, given: &str, dob: &str) -> Patient {
        Patient {
            names: vec![HumanName {
                family: Some(family.into()),
                given: vec![given.into()],
            }],
            birth_date: Some(dob.into()),
            ..Default::default()
        }
    }

    #[test]
    fn different_people_do_not_match() {
        let r = compare_patients(&CompareRequest {
            a: p("Smith", "John", "1980-03-14"),
            b: p("Okafor", "Chiamaka", "1992-11-30"),
        });
        assert_eq!(r.decision, Decision::NonMatch);
    }

    #[test]
    fn request_deserialises_from_json() {
        let json = r#"{"a":{"names":[{"family":"Smith","given":["John"]}]},"b":{}}"#;
        let request: CompareRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.a.names.len(), 1);
    }
}
