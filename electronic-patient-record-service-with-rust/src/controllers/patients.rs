//! Patient identity endpoints. The request and response bodies are the
//! matcher's own types: there is no adapter layer.

use crate::audit::{self, Action, AuditError, Chain, Event};
use crate::auth::{self, Agent};
use crate::consent;
use crate::consents_store;
use crate::patients_store::{self as store, Search, StoreError};
use crate::policy::{self, Consent, Purpose, Relationship, Resource, Subject};
use crate::relationships;
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

pub(super) fn json_error(status: StatusCode, body: serde_json::Value) -> Response {
    (status, Json(body)).into_response()
}

/// The purpose of use the caller declared, `operations` by default. An unknown code is a 400.
pub(super) fn purpose_of(headers: &HeaderMap) -> Result<Purpose, Box<Response>> {
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

pub(super) fn audit_failed(e: &AuditError) -> Response {
    tracing::error!(error = %e, "refusing to answer: the access could not be audited");
    json_error(
        StatusCode::INTERNAL_SERVER_ERROR,
        serde_json::json!({"error": "internal error"}),
    )
}

/// What the patients' consent says about this subject reading this kind of record, per patient.
/// A patient with no active consent has no entry. With login off (development only) there is no
/// subject and nothing is looked up. A failed lookup fails the request: an unchecked disclosure is
/// how a refusal gets ignored.
pub(super) async fn consent_verdicts(
    db: &sea_orm::DatabaseConnection,
    subject: &Subject,
    purpose: Purpose,
    resource: Resource,
    patient_ids: &[String],
) -> Result<std::collections::HashMap<String, consent::Verdict>, Box<Response>> {
    let mut verdicts = std::collections::HashMap::new();
    if !auth::authentication_enforced() || patient_ids.is_empty() {
        return Ok(verdicts);
    }
    let ids: Vec<String> = patient_ids.iter().map(|i| i.to_lowercase()).collect();
    let active = consents_store::active_for(db, &ids)
        .await
        .map_err(|e| Box::new(store_failure(&e)))?;
    let ctx = consent::Context {
        roles: subject.roles.clone(),
        purpose,
        resource,
        labels: std::collections::BTreeSet::new(),
        now: time_now(),
    };
    for (id, records) in &active {
        let verdict = consent::decide(records.iter().map(|c| (c.id.as_str(), &c.consent)), &ctx);
        verdicts.insert(id.clone(), verdict);
    }
    Ok(verdicts)
}

/// The record-level decision for an access. With login off (development only) there is no subject
/// and nothing to decide. For a read of one patient's identity the patient's consent is looked up
/// and applied. On a refusal the refusal is audited and the 403 is returned.
pub(super) async fn authorize(
    db: &sea_orm::DatabaseConnection,
    agent: &Agent,
    subject: &Subject,
    resource: Resource,
    action: policy::Action,
    purpose: Purpose,
    patient_id: Option<&str>,
) -> Result<(), Box<Response>> {
    if !auth::authentication_enforced() {
        return Ok(());
    }
    let mut verdict = consent::Verdict {
        consent: Consent::NotRecorded,
        decided_by: Vec::new(),
    };
    if let (true, Resource::PatientIdentity, Some(id)) = (action.reads(), resource, patient_id) {
        let mut all = consent_verdicts(db, subject, purpose, resource, &[id.to_string()]).await?;
        if let Some(v) = all.remove(&id.to_lowercase()) {
            verdict = v;
        }
    }
    // The subject's relationship to this patient, from the recorded relationships. A failed lookup
    // fails the request: assuming "none" would be safe, but would hide an outage as a refusal.
    let relationship = match patient_id {
        Some(id) => relationships::lookup(db, id, &agent.0, time_now())
            .await
            .map_err(|e| Box::new(store_failure(&e)))?,
        None => Relationship::None,
    };
    let decision = policy::evaluate(&policy::Request {
        subject: subject.clone(),
        resource,
        action,
        labels: std::collections::BTreeSet::new(),
        relationship,
        purpose,
        consent: verdict.consent,
        break_glass: None,
        now: time_now(),
    });
    if decision.allowed() {
        return Ok(());
    }
    let mut event = Event::new(Action::Execute, agent.0.clone());
    event.outcome = 4;
    event.patient_ref = patient_id.map(|id| format!("Patient/{id}"));
    event.resource_ref = Some(format!("{}/{}", resource.name(), action.name()));
    event.purpose_of_use = Some(purpose.code().to_string());
    // Consent ids only: what the consent says is not copied into the audit row.
    event.detail = Some(
        serde_json::json!({ "denied": format!("{decision:?}"), "consent": verdict.decided_by }),
    );
    if let Err(e) = audit::append(db, audit::chain(), &event).await {
        tracing::error!(error = %e, "a refusal could not be audited");
    }
    Err(Box::new(json_error(
        StatusCode::FORBIDDEN,
        serde_json::json!({"error": "forbidden"}),
    )))
}

pub(super) fn time_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

/// Write the audit row for a completed access to a patient. Never put demographics in `detail`.
async fn audit_access(
    db: &sea_orm::DatabaseConnection,
    agent: &Agent,
    action: Action,
    outcome: i16,
    purpose: Purpose,
    patient_id: Option<&str>,
    detail: serde_json::Value,
) -> Result<(), Box<Response>> {
    let access = Access {
        action,
        outcome,
        purpose,
        resource: "Patient",
        patient_id,
    };
    audit_resource(db, agent, access, detail).await
}

/// What was accessed, for an audit row.
pub(super) struct Access<'a> {
    pub action: Action,
    pub outcome: i16,
    pub purpose: Purpose,
    pub resource: &'a str,
    pub patient_id: Option<&'a str>,
}

/// Write the audit row for an access. Never put demographics or consent content in `detail`.
pub(super) async fn audit_resource(
    db: &sea_orm::DatabaseConnection,
    agent: &Agent,
    access: Access<'_>,
    detail: serde_json::Value,
) -> Result<(), Box<Response>> {
    let mut event = Event::new(access.action, agent.0.clone());
    event.outcome = access.outcome;
    event.patient_ref = access.patient_id.map(|id| format!("Patient/{id}"));
    event.resource_ref = Some(access.resource.to_string());
    event.purpose_of_use = Some(access.purpose.code().to_string());
    event.detail = Some(detail);
    audit::append(db, audit::chain(), &event)
        .await
        .map(|_| ())
        .map_err(|e| Box::new(audit_failed(&e)))
}

pub(super) fn store_failure(e: &StoreError) -> Response {
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

pub(super) async fn create(
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
        Resource::PatientIdentity,
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
    // A duplicate must still stop the registration, but a patient who refuses disclosure to this
    // subject is not shown to them: the candidate is reported as restricted, with no record,
    // score or breakdown.
    let existing_ids: Vec<String> = duplicates.iter().map(|c| c.existing.id.clone()).collect();
    let verdicts = match consent_verdicts(
        &ctx.db,
        &subject,
        purpose,
        Resource::PatientIdentity,
        &existing_ids,
    )
    .await
    {
        Ok(v) => v,
        Err(r) => return *r,
    };
    let restricted = |id: &str| {
        verdicts
            .get(&id.to_lowercase())
            .is_some_and(|v| v.consent == Consent::Deny)
    };
    let shown: Vec<serde_json::Value> = duplicates
        .iter()
        .map(|c| {
            if restricted(&c.existing.id) {
                serde_json::json!({ "restricted": true })
            } else {
                serde_json::to_value(c).unwrap_or(serde_json::Value::Null)
            }
        })
        .collect();
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
                "candidates": shown,
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

pub(super) async fn read(
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
        Resource::PatientIdentity,
        policy::Action::Read,
        purpose,
        Some(&id),
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

pub(super) async fn update(
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
        Resource::PatientIdentity,
        policy::Action::Update,
        purpose,
        Some(&id),
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

pub(super) async fn search(
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
        Resource::PatientIdentity,
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
        Ok(mut found) => {
            // Drop what the patient's consent refuses this subject. A hidden result is not
            // counted in the response; the audit row says how many there were.
            let ids: Vec<String> = found.iter().map(|s| s.id.clone()).collect();
            let verdicts =
                match consent_verdicts(&ctx.db, &subject, purpose, Resource::PatientIdentity, &ids)
                    .await
                {
                    Ok(v) => v,
                    Err(r) => return *r,
                };
            let before = found.len();
            found.retain(|s| {
                verdicts
                    .get(&s.id.to_lowercase())
                    .is_none_or(|v| v.consent != Consent::Deny)
            });
            let hidden_by_consent = before - found.len();
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
                serde_json::json!({
                    "search": used,
                    "results": found.len(),
                    "hidden_by_consent": hidden_by_consent,
                }),
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
        .add("/{id}/consents", post(super::consents::create))
        .add("/{id}/consents", get(super::consents::list))
        .add("/{id}/consents/{consent_id}", put(super::consents::update))
        .add("/{id}/relationships", post(super::relationships::create))
        .add("/{id}/relationships", get(super::relationships::list))
        .add(
            "/{id}/relationships/{relationship_id}",
            put(super::relationships::update),
        )
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
