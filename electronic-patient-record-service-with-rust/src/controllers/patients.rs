//! Patient identity endpoints. The request and response bodies are the
//! matcher's own types: there is no adapter layer.

use crate::audit::{self, Action, AuditError, Chain, Event};
use crate::auth::{self, Agent};
use crate::patients_store::{self as store, Search, StoreError};
use crate::policy::{self, Consent, Purpose, Relationship, Subject};
use axum::{
    Extension,
    extract::{Json, Path, Query},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
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

/// A new patient.
#[derive(Debug, Deserialize)]
pub struct CreateRequest {
    /// The identity record. Any `id` is ignored: the server assigns one.
    pub patient: Patient,
    /// Why, for the version history.
    #[serde(default)]
    pub reason: Option<String>,
    /// Register anyway although the matcher found likely duplicates. A reason is required and is
    /// kept in the version history.
    #[serde(default)]
    pub override_reason: Option<String>,
}

/// A change to a patient: a new version.
#[derive(Debug, Deserialize)]
pub struct UpdateRequest {
    /// The whole new identity record.
    pub patient: Patient,
    /// The version this change was made from; anything else is a 409.
    pub expected_version: i32,
    /// Why, for the version history.
    #[serde(default)]
    pub reason: Option<String>,
}

/// An existing patient the new record may duplicate, with the matcher's explanation.
#[derive(Debug, Serialize)]
pub struct Candidate {
    /// The existing patient.
    pub existing: store::Stored,
    /// The matcher's result for (new, existing).
    #[serde(flatten)]
    pub result: MatchResult,
}

/// The existing patients the matcher would send to review, or would link automatically, ordered
/// by score (highest first). A `NonMatch` is not a candidate.
#[must_use]
pub fn likely_duplicates(new: &Patient, existing: Vec<store::Stored>) -> Vec<Candidate> {
    let engine = MatchingEngine::default();
    let mut found: Vec<Candidate> = existing
        .into_iter()
        .filter_map(|e| {
            let result = engine.compare(new, &e.patient);
            (result.decision != electronic_patient_record_matcher::Decision::NonMatch).then_some(
                Candidate {
                    existing: e,
                    result,
                },
            )
        })
        .collect();
    found.sort_by(|a, b| {
        b.result
            .score
            .total_cmp(&a.result.score)
            .then_with(|| a.existing.id.cmp(&b.existing.id))
    });
    found
}

fn json_error(status: StatusCode, body: serde_json::Value) -> Response {
    (status, Json(body)).into_response()
}

/// The purpose of use the caller declared, `operations` by default. An unknown code is a 400.
fn purpose_of(headers: &HeaderMap) -> Result<Purpose, Box<Response>> {
    match headers.get("x-purpose-of-use") {
        None => Ok(Purpose::Operations),
        Some(v) => v.to_str().ok().and_then(Purpose::parse).ok_or_else(|| {
            Box::new(json_error(
                StatusCode::BAD_REQUEST,
                serde_json::json!({"error": "unknown X-Purpose-Of-Use"}),
            ))
        }),
    }
}

fn audit_failed(e: &AuditError) -> Response {
    tracing::error!(error = %e, "refusing to answer: the access could not be audited");
    json_error(
        StatusCode::INTERNAL_SERVER_ERROR,
        serde_json::json!({"error": "internal error"}),
    )
}

/// The record-level decision for an identity access. With login off (development only) there is no
/// subject and nothing to decide. On a refusal the refusal is audited and the 403 is returned.
async fn authorize(
    db: &sea_orm::DatabaseConnection,
    agent: &Agent,
    subject: &Subject,
    action: policy::Action,
    purpose: Purpose,
    patient_ref: Option<String>,
) -> Result<(), Box<Response>> {
    if !auth::authentication_enforced() {
        return Ok(());
    }
    let decision = policy::evaluate(&policy::Request {
        subject: subject.clone(),
        resource: policy::Resource::PatientIdentity,
        action,
        labels: std::collections::BTreeSet::new(),
        // No care-relationship lookup exists yet, and identity is not clinical content, so none is
        // asserted. The patient and proxy roles need one, so they are refused here.
        relationship: Relationship::None,
        purpose,
        consent: Consent::NotRecorded,
        break_glass: None,
        now: time_now(),
    });
    if decision.allowed() {
        return Ok(());
    }
    let mut event = Event::new(Action::Execute, agent.0.clone());
    event.outcome = 4;
    event.patient_ref = patient_ref;
    event.resource_ref = Some(format!("PatientIdentity/{}", action.name()));
    event.purpose_of_use = Some(purpose.code().to_string());
    event.detail = Some(serde_json::json!({ "denied": format!("{decision:?}") }));
    if let Err(e) = audit::append(db, audit::chain(), &event).await {
        tracing::error!(error = %e, "a refusal could not be audited");
    }
    Err(Box::new(json_error(
        StatusCode::FORBIDDEN,
        serde_json::json!({"error": "forbidden"}),
    )))
}

fn time_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

/// Write the audit row for a completed access. Never put demographics in `detail`.
async fn audit_access(
    db: &sea_orm::DatabaseConnection,
    agent: &Agent,
    action: Action,
    outcome: i16,
    purpose: Purpose,
    patient_id: Option<&str>,
    detail: serde_json::Value,
) -> Result<(), Box<Response>> {
    let mut event = Event::new(action, agent.0.clone());
    event.outcome = outcome;
    event.patient_ref = patient_id.map(|id| format!("Patient/{id}"));
    event.resource_ref = Some("Patient".to_string());
    event.purpose_of_use = Some(purpose.code().to_string());
    event.detail = Some(detail);
    audit::append(db, audit::chain(), &event)
        .await
        .map(|_| ())
        .map_err(|e| Box::new(audit_failed(&e)))
}

fn store_failure(e: &StoreError) -> Response {
    match e {
        StoreError::NotFound => json_error(
            StatusCode::NOT_FOUND,
            serde_json::json!({"error": "not found"}),
        ),
        StoreError::Conflict { current } => json_error(
            StatusCode::CONFLICT,
            serde_json::json!({"error": "version conflict", "current_version": current}),
        ),
        StoreError::BadQuery(m) => {
            json_error(StatusCode::BAD_REQUEST, serde_json::json!({"error": m}))
        }
        StoreError::Db(m) => {
            tracing::error!(error = %m, "patient store failed");
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({"error": "internal error"}),
            )
        }
    }
}

async fn create(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Json(request): Json<CreateRequest>,
) -> Response {
    let purpose = match purpose_of(&headers) {
        Ok(p) => p,
        Err(r) => return *r,
    };
    if let Err(r) = authorize(
        &ctx.db,
        &agent,
        &subject,
        policy::Action::Create,
        purpose,
        None,
    )
    .await
    {
        return *r;
    }
    let problems = store::validate(&request.patient);
    if !problems.is_empty() {
        return json_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            serde_json::json!({"errors": problems}),
        );
    }
    let override_reason = request
        .override_reason
        .as_deref()
        .map(str::trim)
        .filter(|r| !r.is_empty());
    let duplicates = match store::candidates(&ctx.db, &request.patient).await {
        Ok(pool) => likely_duplicates(&request.patient, pool),
        // An unchecked registration is how duplicates happen: fail rather than skip the check.
        Err(e) => return store_failure(&e),
    };
    let ids: Vec<&str> = duplicates.iter().map(|c| c.existing.id.as_str()).collect();
    if !duplicates.is_empty() && override_reason.is_none() {
        // Showing the candidates discloses them, so this is an audited access.
        if let Err(r) = audit_access(
            &ctx.db,
            &agent,
            Action::Create,
            4,
            purpose,
            None,
            serde_json::json!({"refused": "possible duplicate", "candidates": ids}),
        )
        .await
        {
            return *r;
        }
        return json_error(
            StatusCode::CONFLICT,
            serde_json::json!({
                "error": "possible duplicate; resend with override_reason to register anyway",
                "candidates": duplicates,
            }),
        );
    }
    let kept_reason = match (override_reason, request.reason.as_deref()) {
        (Some(o), Some(r)) => Some(format!("{r}; duplicate override: {o}")),
        (Some(o), None) => Some(format!("duplicate override: {o}")),
        (None, r) => r.map(str::to_string),
    };
    let stored =
        match store::create(&ctx.db, &agent.0, kept_reason.as_deref(), &request.patient).await {
            Ok(s) => s,
            Err(e) => return store_failure(&e),
        };
    if let Err(r) = audit_access(
        &ctx.db,
        &agent,
        Action::Create,
        0,
        purpose,
        Some(&stored.id),
        // The override reason is free text and stays in the version history, not here.
        serde_json::json!({
            "version": stored.version,
            "duplicate_override": !ids.is_empty(),
            "candidates": ids,
        }),
    )
    .await
    {
        return *r;
    }
    (StatusCode::CREATED, Json(stored)).into_response()
}

async fn read(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let purpose = match purpose_of(&headers) {
        Ok(p) => p,
        Err(r) => return *r,
    };
    if let Err(r) = authorize(
        &ctx.db,
        &agent,
        &subject,
        policy::Action::Read,
        purpose,
        Some(format!("Patient/{id}")),
    )
    .await
    {
        return *r;
    }
    match store::current(&ctx.db, &id).await {
        Ok(stored) => {
            // Audit before the data leaves.
            if let Err(r) = audit_access(
                &ctx.db,
                &agent,
                Action::Read,
                0,
                purpose,
                Some(&stored.id),
                serde_json::json!({"version": stored.version}),
            )
            .await
            {
                return *r;
            }
            Json(stored).into_response()
        }
        Err(StoreError::NotFound) => {
            // A miss is an access too. Only a well-formed id is recorded as the patient reference.
            if let Err(r) = audit_access(
                &ctx.db,
                &agent,
                Action::Read,
                4,
                purpose,
                None,
                serde_json::json!({"found": false}),
            )
            .await
            {
                return *r;
            }
            store_failure(&StoreError::NotFound)
        }
        Err(e) => store_failure(&e),
    }
}

async fn update(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<UpdateRequest>,
) -> Response {
    let purpose = match purpose_of(&headers) {
        Ok(p) => p,
        Err(r) => return *r,
    };
    if let Err(r) = authorize(
        &ctx.db,
        &agent,
        &subject,
        policy::Action::Update,
        purpose,
        Some(format!("Patient/{id}")),
    )
    .await
    {
        return *r;
    }
    let problems = store::validate(&request.patient);
    if !problems.is_empty() {
        return json_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            serde_json::json!({"errors": problems}),
        );
    }
    match store::update(
        &ctx.db,
        &id,
        request.expected_version,
        &agent.0,
        request.reason.as_deref(),
        &request.patient,
    )
    .await
    {
        Ok(stored) => {
            if let Err(r) = audit_access(
                &ctx.db,
                &agent,
                Action::Update,
                0,
                purpose,
                Some(&stored.id),
                serde_json::json!({"version": stored.version}),
            )
            .await
            {
                return *r;
            }
            Json(stored).into_response()
        }
        Err(e @ (StoreError::Conflict { .. } | StoreError::NotFound)) => {
            if let Err(r) = audit_access(
                &ctx.db,
                &agent,
                Action::Update,
                4,
                purpose,
                None,
                serde_json::json!({"refused": e.to_string()}),
            )
            .await
            {
                return *r;
            }
            store_failure(&e)
        }
        Err(e) => store_failure(&e),
    }
}

async fn search(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Query(query): Query<Search>,
) -> Response {
    let purpose = match purpose_of(&headers) {
        Ok(p) => p,
        Err(r) => return *r,
    };
    if let Err(r) = authorize(
        &ctx.db,
        &agent,
        &subject,
        policy::Action::Search,
        purpose,
        None,
    )
    .await
    {
        return *r;
    }
    if !query.any() {
        return json_error(
            StatusCode::BAD_REQUEST,
            serde_json::json!({"error": "give at least one of family, birth_date, identifier"}),
        );
    }
    match store::search(&ctx.db, &query).await {
        Ok(found) => {
            // The criteria are demographics, so only which kinds were used and how many matched.
            let used: Vec<&str> = [
                ("family", &query.family),
                ("birth_date", &query.birth_date),
                ("identifier", &query.identifier),
            ]
            .into_iter()
            .filter(|(_, v)| v.as_deref().is_some_and(|s| !s.trim().is_empty()))
            .map(|(k, _)| k)
            .collect();
            if let Err(r) = audit_access(
                &ctx.db,
                &agent,
                Action::Read,
                0,
                purpose,
                None,
                serde_json::json!({"search": used, "results": found.len()}),
            )
            .await
            {
                return *r;
            }
            Json(serde_json::json!({"results": found, "limit": store::SEARCH_LIMIT}))
                .into_response()
        }
        Err(e) => store_failure(&e),
    }
}

/// Routes under `/api/patients`.
#[must_use]
pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/patients")
        .add("/compare", post(compare))
        .add("/", post(create))
        .add("/", get(search))
        .add("/{id}", get(read))
        .add("/{id}", put(update))
}

#[cfg(test)]
mod tests {
    use super::*;
    use electronic_patient_record_matcher::{Decision, HumanName, Identifier, NHS_NUMBER_SYSTEM};

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

    fn stored(id: &str, patient: Patient) -> store::Stored {
        store::Stored {
            id: id.into(),
            version: 1,
            author: "a".into(),
            patient,
        }
    }

    #[test]
    fn the_same_person_is_a_candidate_and_a_different_person_is_not() {
        let new = p("Smith", "John", "1980-03-14");
        let found = likely_duplicates(
            &new,
            vec![
                stored("b", p("Okafor", "Chiamaka", "1992-11-30")),
                stored("a", p("Smith", "John", "1980-03-14")),
                stored("c", p("Smith", "Jon", "1980-03-14")),
            ],
        );
        let ids: Vec<&str> = found.iter().map(|c| c.existing.id.as_str()).collect();
        assert_eq!(ids, ["a", "c"], "best match first, the stranger dropped");
        assert!(found[0].result.score >= found[1].result.score);
        assert!(
            !found[1].result.fields.is_empty(),
            "the breakdown is returned"
        );
    }

    #[test]
    fn a_shared_nhs_number_with_a_different_birth_date_is_still_a_candidate() {
        let mut new = p("Smith", "John", "1980-03-14");
        new.identifiers = vec![Identifier {
            system: NHS_NUMBER_SYSTEM.into(),
            value: "9434765919".into(),
        }];
        let mut other = p("Smith", "John", "1981-03-14");
        other.identifiers = new.identifiers.clone();
        let found = likely_duplicates(&new, vec![stored("a", other)]);
        assert_eq!(
            found.len(),
            1,
            "a data-entry error must not hide a duplicate"
        );
    }

    #[test]
    fn an_identical_record_is_the_strongest_candidate() {
        let new = p("Smith", "John", "1980-03-14");
        let found = likely_duplicates(&new, vec![stored("a", new.clone())]);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].result.decision, Decision::AutoLink);
    }
}
