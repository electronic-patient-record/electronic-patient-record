//! Patient identity endpoints. The request and response bodies are the
//! matcher's own types: there is no adapter layer.

use super::common::{
    Access, audit_resource, authorize, begin, commit, consent_verdicts, purpose_of,
};
use crate::api_error::ApiError;
use crate::audit::{self, Action, AuditError, Chain, Event};
use crate::auth::Agent;
use crate::links;
use crate::patients_store::{self as store, Search, StoreError};
use crate::policy::{self, Consent, Purpose, Resource, Subject};
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
    /// Register a drill or test record (EC-10): it is compared only with other drill records, kept out
    /// of ordinary search and never merged with a real patient.
    #[serde(default)]
    pub drill: bool,
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

/// Write the audit row for a completed access to a patient. Never put demographics in `detail`.
async fn audit_access(
    db: &(impl sea_orm::ConnectionTrait + sea_orm::TransactionTrait),
    agent: &Agent,
    action: Action,
    outcome: i16,
    purpose: Purpose,
    patient_id: Option<&str>,
    detail: serde_json::Value,
) -> Result<(), ApiError> {
    let access = Access {
        action,
        outcome,
        purpose,
        resource: "Patient",
        patient_id,
    };
    audit_resource(db, agent, access, detail).await
}

/// A patient as read: the stored version, whether there is a photo to fetch, and merge status
/// (ID-5). A record merged away says where it went; a surviving record says what was merged into it.
#[derive(Debug, Serialize)]
pub struct PatientView {
    /// The current version.
    #[serde(flatten)]
    pub stored: store::Stored,
    /// The photo's version, `None` when there is none or it was withdrawn.
    pub photo_version: Option<i32>,
    /// The surviving record, when this one was merged away.
    pub replaced_by: Option<String>,
    /// The records merged into this one.
    pub replaces: Vec<String>,
    /// The site's time zone (`EPR_SITE_TIME_ZONE`), whose calendar day the banner's age is counted on.
    pub site_time_zone: String,
}

/// Register a patient: the duplicate check, the new version and its audit row are one transaction.
pub(super) async fn create_service(
    ctx: &AppContext,
    agent: &Agent,
    subject: &Subject,
    headers: &HeaderMap,
    request: &CreateRequest,
) -> Result<store::Stored, ApiError> {
    let purpose = match purpose_of(headers) {
        Ok(p) => p,
        Err(r) => return Err(r),
    };
    authorize(
        &ctx.db,
        agent,
        subject,
        Resource::PatientIdentity,
        policy::Action::Create,
        purpose,
        None,
    )
    .await?;
    let problems = store::validate(&request.patient);
    if !problems.is_empty() {
        return Err(ApiError::invalid(problems.clone()));
    }
    let override_reason = request
        .override_reason
        .as_deref()
        .map(str::trim)
        .filter(|r| !r.is_empty());
    // The duplicate check, the new version and its audit row are one transaction under the
    // registration lock (CQ-7): two registrations of one person can never both pass the check, and
    // if the audit row cannot be written the patient is not created.
    let tx = match begin(&ctx.db).await {
        Ok(t) => t,
        Err(r) => return Err(r),
    };
    if let Err(e) = store::lock_registration(&tx).await {
        return Err(ApiError::from(&e));
    }
    // More matches than the limit: only the strongest were compared, and the answer says so (CQ-36).
    let (duplicates, candidates_cut) =
        match store::candidates_in(&tx, &request.patient, request.drill).await {
            Ok((pool, cut)) => (likely_duplicates(&request.patient, pool), cut),
            // An unchecked registration is how duplicates happen: fail rather than skip the check.
            Err(e) => return Err(ApiError::from(&e)),
        };
    let ids: Vec<&str> = duplicates.iter().map(|c| c.existing.id.as_str()).collect();
    // A duplicate must still stop the registration, but a patient who refuses disclosure to this
    // subject is not shown to them: the candidate is reported as restricted, with no record,
    // score or breakdown.
    let existing_ids: Vec<String> = duplicates.iter().map(|c| c.existing.id.clone()).collect();
    // On the transaction: it holds a connection, and the development pool has only one.
    let verdicts = match consent_verdicts(
        &tx,
        subject,
        purpose,
        Resource::PatientIdentity,
        &existing_ids,
    )
    .await
    {
        Ok(v) => v,
        Err(r) => return Err(r),
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
        // Showing the candidates discloses them, so this is an audited access. The row is written on
        // the transaction (nothing else has been written to it) and committed before the refusal.
        audit_access(
            &tx,
            agent,
            Action::Create,
            4,
            purpose,
            None,
            serde_json::json!({
                "refused": "possible duplicate",
                "candidates": ids,
                "candidates_cut": candidates_cut,
            }),
        )
        .await?;
        commit(tx).await?;
        return Err(ApiError::conflict(
            "possible duplicate; resend with override_reason to register anyway",
        )
        .with("candidates", shown)
        .with("candidates_cut", candidates_cut));
    }
    let kept_reason = match (override_reason, request.reason.as_deref()) {
        (Some(o), Some(r)) => Some(format!("{r}; duplicate override: {o}")),
        (Some(o), None) => Some(format!("duplicate override: {o}")),
        (None, r) => r.map(str::to_string),
    };
    let stored = match store::create_as(
        &tx,
        &agent.0,
        kept_reason.as_deref(),
        &request.patient,
        request.drill,
    )
    .await
    {
        Ok(s) => s,
        Err(e) => return Err(ApiError::from(&e)),
    };
    audit_access(
        &tx,
        agent,
        Action::Create,
        0,
        purpose,
        Some(&stored.id),
        // The override reason is free text and stays in the version history, not here.
        serde_json::json!({
            "version": stored.version,
            "drill": stored.drill,
            "duplicate_override": !ids.is_empty(),
            "candidates": ids,
            "candidates_cut": candidates_cut,
        }),
    )
    .await?;
    commit(tx).await?;
    Ok(stored)
}

pub(super) async fn create(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Json(request): Json<CreateRequest>,
) -> Response {
    match create_service(&ctx, &agent, &subject, &headers, &request).await {
        Ok(stored) => (StatusCode::CREATED, Json(stored)).into_response(),
        Err(e) => e.into_response(),
    }
}

pub(super) async fn read_service(
    ctx: &AppContext,
    agent: &Agent,
    subject: &Subject,
    headers: &HeaderMap,
    id: &str,
) -> Result<PatientView, ApiError> {
    let purpose = match purpose_of(headers) {
        Ok(p) => p,
        Err(r) => return Err(r),
    };
    authorize(
        &ctx.db,
        agent,
        subject,
        Resource::PatientIdentity,
        policy::Action::Read,
        purpose,
        Some(id),
    )
    .await?;
    match store::current(&ctx.db, id).await {
        Ok(stored) => {
            // Audit before the data leaves.
            audit_access(
                &ctx.db,
                agent,
                Action::Read,
                0,
                purpose,
                Some(&stored.id),
                serde_json::json!({"version": stored.version}),
            )
            .await?;
            // Whether there is a photo to fetch, so the banner does not ask for one that is not
            // there. A failed lookup fails the read: a banner must not quietly lose its photo.
            let photo_version = match crate::photos_store::current(&ctx.db, &stored.id).await {
                Ok(p) => p.filter(|p| p.bytes.is_some()).map(|p| p.version),
                Err(e) => return Err(ApiError::from(&e)),
            };
            // Merge status (ID-5): a record merged away says where it went; a surviving record
            // says what was merged into it. A failed lookup fails the read.
            let replaced_by = match links::merged_into(&ctx.db, &stored.id).await {
                Ok(r) => r,
                Err(e) => return Err(ApiError::from(&e)),
            };
            let replaces = match links::sources_of(&ctx.db, &stored.id).await {
                Ok(r) => r,
                Err(e) => return Err(ApiError::from(&e)),
            };
            Ok(PatientView {
                stored,
                photo_version,
                replaced_by,
                replaces,
                site_time_zone: crate::site::time_zone(),
            })
        }
        Err(StoreError::NotFound) => {
            // A miss is an access too. Only a well-formed id is recorded as the patient reference.
            audit_access(
                &ctx.db,
                agent,
                Action::Read,
                4,
                purpose,
                None,
                serde_json::json!({"found": false}),
            )
            .await?;
            Err(ApiError::from(&StoreError::NotFound))
        }
        Err(e) => Err(ApiError::from(&e)),
    }
}

pub(super) async fn read(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    match read_service(&ctx, &agent, &subject, &headers, &id).await {
        Ok(view) => Json(view).into_response(),
        Err(e) => e.into_response(),
    }
}

pub(super) async fn update_service(
    ctx: &AppContext,
    agent: &Agent,
    subject: &Subject,
    headers: &HeaderMap,
    id: &str,
    request: &UpdateRequest,
) -> Result<store::Stored, ApiError> {
    let purpose = match purpose_of(headers) {
        Ok(p) => p,
        Err(r) => return Err(r),
    };
    authorize(
        &ctx.db,
        agent,
        subject,
        Resource::PatientIdentity,
        policy::Action::Update,
        purpose,
        Some(id),
    )
    .await?;
    let problems = store::validate(&request.patient);
    if !problems.is_empty() {
        return Err(ApiError::invalid(problems.clone()));
    }
    let tx = match begin(&ctx.db).await {
        Ok(t) => t,
        Err(r) => return Err(r),
    };
    match store::update(
        &tx,
        id,
        request.expected_version,
        &agent.0,
        request.reason.as_deref(),
        &request.patient,
    )
    .await
    {
        Ok(stored) => {
            audit_access(
                &tx,
                agent,
                Action::Update,
                0,
                purpose,
                Some(&stored.id),
                serde_json::json!({"version": stored.version}),
            )
            .await?;
            commit(tx).await?;
            Ok(stored)
        }
        Err(e @ (StoreError::Conflict { .. } | StoreError::NotFound)) => {
            // Nothing was written; end the transaction before the refusal is audited.
            let _ = sea_orm::TransactionSession::rollback(tx).await;
            audit_access(
                &ctx.db,
                agent,
                Action::Update,
                4,
                purpose,
                None,
                serde_json::json!({"refused": e.to_string()}),
            )
            .await?;
            Err(ApiError::from(&e))
        }
        Err(e) => Err(ApiError::from(&e)),
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
    match update_service(&ctx, &agent, &subject, &headers, &id, &request).await {
        Ok(stored) => Json(stored).into_response(),
        Err(e) => e.into_response(),
    }
}

pub(super) async fn search_service(
    ctx: &AppContext,
    agent: &Agent,
    subject: &Subject,
    headers: &HeaderMap,
    query: &Search,
) -> Result<Vec<store::Stored>, ApiError> {
    let purpose = match purpose_of(headers) {
        Ok(p) => p,
        Err(r) => return Err(r),
    };
    authorize(
        &ctx.db,
        agent,
        subject,
        Resource::PatientIdentity,
        policy::Action::Search,
        purpose,
        None,
    )
    .await?;
    if !query.any() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "give at least one of family, birth_date, identifier",
        ));
    }
    match store::search(&ctx.db, query).await {
        Ok(mut found) => {
            // Drop what the patient's consent refuses this subject. A hidden result is not
            // counted in the response; the audit row says how many there were.
            let ids: Vec<String> = found.iter().map(|s| s.id.clone()).collect();
            let verdicts =
                match consent_verdicts(&ctx.db, subject, purpose, Resource::PatientIdentity, &ids)
                    .await
                {
                    Ok(v) => v,
                    Err(r) => return Err(r),
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
            audit_access(
                &ctx.db,
                agent,
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
            .await?;
            Ok(found)
        }
        Err(e) => Err(ApiError::from(&e)),
    }
}

pub(super) async fn search(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Query(query): Query<Search>,
) -> Response {
    match search_service(&ctx, &agent, &subject, &headers, &query).await {
        Ok(found) => Json(serde_json::json!({"results": found, "limit": store::SEARCH_LIMIT}))
            .into_response(),
        Err(e) => e.into_response(),
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
        .add("/{id}/merge", post(super::links::merge))
        .add("/{id}/unmerge", post(super::links::unmerge))
        .add("/{id}/links", get(super::links::list))
        .add("/{id}/allergies", get(super::allergies::list))
        .add("/{id}/allergies", post(super::allergies::create))
        .add(
            "/{id}/allergies/{allergy_id}",
            put(super::allergies::update),
        )
        .add("/{id}/allergy-statement", put(super::allergies::statement))
        .add("/{id}/photo", get(super::photos::read))
        .add("/{id}/photo", put(super::photos::put))
        .add("/{id}/photo/withdraw", post(super::photos::withdraw))
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
            drill: false,
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
