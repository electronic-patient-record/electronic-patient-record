//! What every controller shares: the purpose of use, the record-level access decision (role,
//! relationship, consent, break-the-glass), the write transaction, the audit row and the error
//! answers (CQ-11). Error bodies come from [`crate::api_error::ApiError`], never from here.

use crate::api_error::ApiError;
use crate::audit::{self, Action, AuditError, Event};
use crate::auth::{self, Agent};
use crate::consent;
use crate::consents_store;
use crate::links;
use crate::patients_store::StoreError;
use crate::policy::{self, Consent, Purpose, Relationship, Resource, Subject};
use crate::pool::Db;
use crate::proxy::Scope;
use crate::relationships;
use axum::{
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use sqlx::{PgPool, Postgres};

/// The purpose of use the caller declared, `operations` by default. An unknown code is a 400.
pub(super) fn purpose_of(headers: &HeaderMap) -> Result<Purpose, ApiError> {
    match headers.get("x-purpose-of-use") {
        None => Ok(Purpose::Operations),
        Some(v) => v
            .to_str()
            .ok()
            .and_then(Purpose::parse)
            .ok_or_else(|| ApiError::bad_request("unknown X-Purpose-Of-Use")),
    }
}

/// The answer when an access could not be audited: the real error is logged and the caller gets a plain 500, because
/// the service never answers an access it could not record.
pub(crate) fn audit_failed(e: &AuditError) -> ApiError {
    if e.unavailable() {
        tracing::warn!(error = %e, "refusing to answer: the access could not be audited, the database is unavailable");
        return ApiError::from(&StoreError::Unavailable(e.to_string()));
    }
    tracing::error!(error = %e, "refusing to answer: the access could not be audited");
    ApiError::internal()
}

/// What the patients' consent says about this subject reading this kind of record, per patient.
/// A patient with no active consent has no entry. With login off (development only) there is no
/// subject and nothing is looked up. A failed lookup fails the request: an unchecked disclosure is
/// how a refusal gets ignored.
pub(super) async fn consent_verdicts(
    db: impl Into<Db<'_>>,
    subject: &Subject,
    purpose: Purpose,
    resource: Resource,
    patient_ids: &[String],
) -> Result<std::collections::HashMap<String, consent::Verdict>, ApiError> {
    let mut conn = db.into().acquire().await.map_err(acquire_failed)?;
    let db = &mut *conn;
    let mut verdicts = std::collections::HashMap::new();
    if !auth::authentication_enforced() || patient_ids.is_empty() {
        return Ok(verdicts);
    }
    // A refusal on a record that was merged into this one stands for this one too: the person's
    // choice does not lapse because two records were joined. Two queries for any number of ids.
    let active = consents_store::active_with_merged(&mut *db, patient_ids)
        .await
        .map_err(|e| ApiError::from(&e))?;
    let ctx = consent::Context {
        roles: subject.roles.clone(),
        purpose,
        resource,
        labels: std::collections::BTreeSet::new(),
        now: time_now(),
    };
    for (id, records) in &active {
        let mut verdict =
            consent::decide(records.iter().map(|c| (c.id.as_str(), &c.consent)), &ctx);
        // Which merged-in records the deciding consents belong to, so a refusal names its origin.
        let mut from_merged: Vec<String> = records
            .iter()
            .filter(|c| {
                verdict.decided_by.contains(&c.id) && !c.patient_id.eq_ignore_ascii_case(id)
            })
            .map(|c| c.patient_id.to_lowercase())
            .collect();
        from_merged.sort();
        from_merged.dedup();
        verdict.from_merged = from_merged;
        verdicts.insert(id.clone(), verdict);
    }
    Ok(verdicts)
}

/// The record-level decision for an access. With login off (development only) there is no subject
/// and nothing to decide. For a read of one patient's identity the patient's consent is looked up
/// and applied. On a refusal the refusal is audited and the 403 is returned.
pub(super) async fn authorize(
    db: &PgPool,
    agent: &Agent,
    subject: &Subject,
    resource: Resource,
    action: policy::Action,
    purpose: Purpose,
    patient_id: Option<&str>,
) -> Result<(), ApiError> {
    authorize_scoped(
        db, agent, subject, resource, action, purpose, patient_id, None,
    )
    .await
}

/// The proxy scope a resource needs when the route does not name one. A resource no scope covers
/// returns `None`, and a proxy then has no relationship at all (PA-5).
fn scope_of(resource: Resource) -> Option<Scope> {
    match resource {
        Resource::PatientIdentity => Some(Scope::Identity),
        Resource::AllergyIntolerance => Some(Scope::Allergies),
        Resource::Adjustment => Some(Scope::Adjustments),
        _ => None,
    }
}

/// [`authorize`], for a route that is a narrower part of a resource than the resource itself (the photo
/// and the access log are identity data to the policy, and their own scope to a proxy).
#[expect(
    clippy::too_many_arguments,
    reason = "RCA-EXC-6: a store call names each field of the record; one parameter struct for all is a wider change"
)]
pub(super) async fn authorize_scoped(
    db: &PgPool,
    agent: &Agent,
    subject: &Subject,
    resource: Resource,
    action: policy::Action,
    purpose: Purpose,
    patient_id: Option<&str>,
    scope: Option<Scope>,
) -> Result<(), ApiError> {
    if !auth::authentication_enforced() {
        return Ok(());
    }
    // Permission at the point of care (WL-4): where the setting requires it, a staff read of one patient's
    // clinical content needs a basis. The patient and a proxy act on their own behalf and need none.
    let on_their_own_behalf = !subject.roles.is_empty()
        && subject
            .roles
            .iter()
            .all(|r| matches!(r, policy::Role::Patient | policy::Role::Proxy));
    if crate::permission::required()
        && action.reads()
        && resource.clinical()
        && patient_id.is_some()
        && !on_their_own_behalf
        && audit::ORIGIN
            .try_with(|o| o.permission.is_none())
            .unwrap_or(true)
    {
        let mut event = Event::new(Action::Execute, agent.0.clone());
        event.outcome = 4;
        event.patient_ref = patient_id.map(|id| format!("Patient/{id}"));
        event.resource_ref = Some(format!("{}/{}", resource.name(), action.name()));
        event.purpose_of_use = Some(purpose.code().to_string());
        event.detail = Some(serde_json::json!({ "denied": "permission_basis_required" }));
        if let Err(e) = audit::append(db, &audit::chain(), &event).await {
            crate::metrics::UNAUDITED_REFUSALS.inc();
            tracing::error!(error = %e, "a refusal could not be audited");
        }
        return Err(ApiError::forbidden().with("permission_basis_required", true));
    }
    let mut verdict = consent::Verdict {
        consent: Consent::NotRecorded,
        decided_by: Vec::new(),
        from_merged: Vec::new(),
    };
    // The patient's consent is looked up for every read of one patient's identity or clinical data.
    if let (true, true, Some(id)) = (
        action.reads(),
        resource == Resource::PatientIdentity || resource.clinical(),
        patient_id,
    ) {
        let mut all = consent_verdicts(db, subject, purpose, resource, &[id.to_string()]).await?;
        if let Some(v) = all.remove(&id.to_lowercase()) {
            verdict = v;
        }
    }
    // A record merged into another is read-only: changes go to the surviving record. (Consent and
    // relationship changes are allowed, so a refusal on the old record can still be withdrawn.)
    if !action.reads()
        && (resource == Resource::PatientIdentity || resource.clinical())
        && let Some(id) = patient_id
        && let Some(into) = links::merged_into(db, id)
            .await
            .map_err(|e| ApiError::from(&e))?
    {
        return Err(
            ApiError::conflict("this record was merged into another: change that one")
                .with("replaced_by", into),
        );
    }
    // The subject's relationship to this patient, from the recorded relationships. A failed lookup
    // fails the request: assuming "none" would be safe, but would hide an outage as a refusal.
    let relationship = match patient_id {
        Some(id) => relationships::lookup(
            db,
            id,
            &agent.0,
            time_now(),
            scope.or_else(|| scope_of(resource)),
        )
        .await
        .map_err(|e| ApiError::from(&e))?,
        None => Relationship::None,
    };
    // A grant matters only to a treating role reading clinical content without a care relationship.
    let grant = match patient_id {
        Some(id)
            if action.reads()
                && resource.clinical()
                && !relationship.treating()
                && subject.roles.iter().any(|r| r.treats()) =>
        {
            crate::break_glass::active(db, id, &agent.0)
                .await
                .map_err(|e| ApiError::from(&e))?
        }
        _ => None,
    };
    let decision = policy::evaluate(&policy::Request {
        subject: subject.clone(),
        resource,
        action,
        labels: std::collections::BTreeSet::new(),
        relationship,
        purpose,
        consent: verdict.consent,
        break_glass: grant.as_ref().map(|g| policy::BreakGlass {
            reason: g.reason.clone(),
            expires_at: g.expires_unix,
        }),
        now: time_now(),
    });
    if let (Some(g), policy::Decision::Allow(obligations)) = (&grant, &decision)
        && obligations
            .iter()
            .any(|o| matches!(o, policy::Obligation::BreakTheGlass(_)))
    {
        // From here every audit row of this request is flagged and names the grant.
        audit::note_break_glass(&g.id);
    }
    let offer = matches!(
        decision,
        policy::Decision::Deny {
            btg_offer: true,
            ..
        }
    );
    if decision.allowed() {
        return Ok(());
    }
    let mut event = Event::new(Action::Execute, agent.0.clone());
    event.outcome = 4;
    event.patient_ref = patient_id.map(|id| format!("Patient/{id}"));
    event.resource_ref = Some(format!("{}/{}", resource.name(), action.name()));
    event.purpose_of_use = Some(purpose.code().to_string());
    // Consent ids only: what the consent says is not copied into the audit row. A refusal that came
    // from a record merged into this one names that record (WL-2).
    let mut detail =
        serde_json::json!({ "denied": format!("{decision:?}"), "consent": verdict.decided_by });
    if !verdict.from_merged.is_empty() {
        crate::json::set(
            &mut detail,
            "consent_from_merged",
            serde_json::json!(verdict.from_merged),
        );
    }
    event.detail = Some(detail);
    if let Err(e) = audit::append(db, &audit::chain(), &event).await {
        crate::metrics::UNAUDITED_REFUSALS.inc();
        tracing::error!(error = %e, "a refusal could not be audited");
    }
    // A refusal that break-the-glass could lift says so, so the screen can offer it.
    let refusal = if offer {
        ApiError::forbidden().with("break_glass_offer", true)
    } else {
        ApiError::forbidden()
    };
    Err(refusal)
}

/// A database failure at the start of a request: 503 when the database cannot serve for now (the rule the stores
/// use, [`crate::store::unavailable`]), a plain 500 for anything else.
fn db_failed(e: &sqlx::Error, what: &str) -> ApiError {
    if crate::store::unavailable(e) {
        tracing::warn!(error = %e, "{what}: the database is unavailable");
        ApiError::from(&StoreError::Unavailable(format!("{what}: {e}")))
    } else {
        tracing::error!(error = %e, "{what}");
        ApiError::internal()
    }
}

/// A connection that could not be had: 503 if the database is unavailable, else the same 500 as any database failure.
pub(super) fn acquire_failed(e: sqlx::Error) -> ApiError {
    db_failed(&e, "could not get a database connection")
}

/// Start the transaction a write and its audit row share (see [`audit::append`]).
pub(super) async fn begin(db: &PgPool) -> Result<sqlx::Transaction<'static, Postgres>, ApiError> {
    db.begin()
        .await
        .map_err(|e| db_failed(&e, "could not start a transaction"))
}

/// Commit it. A failure means the write did not happen, so the request fails.
pub(super) async fn commit(tx: sqlx::Transaction<'static, Postgres>) -> Result<(), ApiError> {
    tx.commit().await.map_err(|e| {
        tracing::error!(error = %e, "could not commit a write and its audit row");
        ApiError::internal()
    })
}

/// The current time as seconds since the Unix epoch, for the policy engine's expiry checks. A clock before 1970 reads
/// as 0 and one beyond the range saturates at `i64::MAX`.
pub(super) fn time_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

/// The query of a read as at a time (EH-2).
#[derive(Debug, serde::Deserialize)]
pub(super) struct AsAtQuery {
    /// The time, RFC 3339 with an offset or `Z`. Required.
    pub time: Option<String>,
}

impl AsAtQuery {
    /// The time asked for. Missing, unreadable or in the future is a 400: nobody can say what the record will hold, and
    /// a default of now would pass for an answer about the past.
    pub(super) fn instant(&self, now: jiff::Timestamp) -> Result<jiff::Timestamp, ApiError> {
        let at = self
            .time
            .as_deref()
            .and_then(crate::clock::parse_rfc3339)
            .ok_or_else(|| {
                ApiError::bad_request("time must be an RFC 3339 time with an offset or Z")
            })?;
        if at > now {
            return Err(ApiError::bad_request("time is in the future"));
        }
        Ok(at)
    }
}

/// What was accessed, for an audit row.
pub(super) struct Access<'a> {
    pub action: Action,
    pub outcome: i16,
    pub purpose: Purpose,
    pub resource: &'a str,
    pub patient_id: Option<&'a str>,
}

/// `detail` with `records`: the records merged into the opened chart that contributed to the access, when any did, in
/// order and once each (AG-6). The audit row is about the chart that was **opened**; these say whose rows were in it, for
/// an authorised investigation, and are not in the patient-facing log.
pub(super) fn with_records<'a>(
    mut detail: serde_json::Value,
    opened: &str,
    contributing: impl IntoIterator<Item = &'a str>,
) -> serde_json::Value {
    let others: std::collections::BTreeSet<String> = contributing
        .into_iter()
        .filter(|id| !id.eq_ignore_ascii_case(opened))
        .map(str::to_lowercase)
        .collect();
    if !others.is_empty() {
        crate::json::set(&mut detail, "records", serde_json::json!(others));
    }
    detail
}

/// Write the audit row for an access. Never put demographics or consent content in `detail`.
pub(super) async fn audit_resource(
    db: impl Into<Db<'_>>,
    agent: &Agent,
    access: Access<'_>,
    detail: serde_json::Value,
) -> Result<(), ApiError> {
    let mut conn = db.into().acquire().await.map_err(acquire_failed)?;
    let db = &mut *conn;
    let mut event = Event::new(access.action, agent.0.clone());
    event.outcome = access.outcome;
    event.patient_ref = access.patient_id.map(|id| format!("Patient/{id}"));
    event.resource_ref = Some(access.resource.to_string());
    event.purpose_of_use = Some(access.purpose.code().to_string());
    event.detail = Some(detail);
    audit::append(&mut *db, &audit::chain(), &event)
        .await
        .map(|_| ())
        .map_err(|e| audit_failed(&e))
}

/// A store error as its response ([`ApiError`]'s `From<&StoreError>`).
pub(super) fn store_failure(e: &StoreError) -> Response {
    ApiError::from(e).into_response()
}

#[cfg(test)]
mod tests {
    use super::{AsAtQuery, with_records};
    use axum::http::StatusCode;

    #[test]
    fn the_records_behind_a_chart_are_named_once_and_never_the_chart_itself() {
        let detail = || serde_json::json!({ "n": 3 });
        assert_eq!(with_records(detail(), "A", ["a", "A"]), detail());
        assert_eq!(with_records(detail(), "a", []), detail());
        assert_eq!(
            with_records(detail(), "a", ["C", "a", "b", "c"]),
            serde_json::json!({ "n": 3, "records": ["b", "c"] })
        );
    }

    #[test]
    fn a_read_as_at_a_time_needs_a_real_past_time() {
        let now: jiff::Timestamp = "2026-11-24T12:00:00Z".parse().unwrap();
        let ask = |t: Option<&str>| {
            AsAtQuery {
                time: t.map(String::from),
            }
            .instant(now)
        };
        assert_eq!(ask(Some("2026-11-24T12:00:00Z")).unwrap(), now);
        assert_eq!(
            ask(Some("2026-11-24T13:00:00+01:00")).unwrap(),
            now,
            "an offset is read"
        );
        for bad in [
            None,
            Some(""),
            Some("yesterday"),
            Some("2026-11-24"),
            Some("2026-11-24T12:00:00"),
            Some("2026-11-24T12:00:01Z"),
        ] {
            assert_eq!(
                ask(bad).unwrap_err().status(),
                StatusCode::BAD_REQUEST,
                "{bad:?}"
            );
        }
    }
}
