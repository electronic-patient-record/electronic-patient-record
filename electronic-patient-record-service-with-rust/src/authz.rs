//! Authorization middleware: the route gate in front of every handler.
//!
//! Runs after authentication ([`crate::auth::require_auth_mw`]). For each request it finds the
//! route's policy entry ([`crate::routes`]) and asks the engine ([`crate::policy::gate`]) whether
//! the caller's roles may ever do that to that kind of resource. No entry, or no permission, is a
//! `403`, and the refusal is written to the audit trail. When login is off (development only)
//! there is no caller to authorize and the gate stands aside, as the authentication guard does.
//!
//! This is the coarse layer. Record-level decisions (care relationship, labels, consent,
//! break-the-glass) are [`crate::policy::evaluate`], called by a handler once it holds a record.

use crate::audit::{self, Action as AuditAction, Event};
use crate::auth::{self, Agent};
use crate::policy::{Subject, gate};
use crate::routes;
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use sea_orm::DatabaseConnection;

/// The state the middleware needs to audit a refusal.
#[derive(Clone)]
pub struct Authz {
    /// Where audit rows go.
    pub db: DatabaseConnection,
}

/// The reasons a request is refused at the gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// The route has no policy entry.
    NoPolicy,
    /// The caller's roles do not permit the action.
    NotPermitted,
    /// Login is on but the request carries no subject (a wiring error; refused).
    NoSubject,
}

impl Refusal {
    fn code(self) -> &'static str {
        match self {
            Refusal::NoPolicy => "no-policy-for-route",
            Refusal::NotPermitted => "role-not-permitted",
            Refusal::NoSubject => "no-subject",
        }
    }
}

/// The gate decision for a request, pure so it can be tested without a server.
///
/// # Errors
///
/// A [`Refusal`] when the request must be refused.
pub fn check(
    enforced: bool,
    method: &axum::http::Method,
    path: &str,
    subject: Option<&Subject>,
) -> Result<(), (Refusal, Option<&'static str>)> {
    if !enforced || auth::is_public_path(path) {
        return Ok(());
    }
    let Some((template, need)) = routes::lookup(method, path) else {
        return Err((Refusal::NoPolicy, None));
    };
    let Some(subject) = subject else {
        return Err((Refusal::NoSubject, Some(template)));
    };
    if gate(subject, need.resource, need.action) {
        Ok(())
    } else {
        Err((Refusal::NotPermitted, Some(template)))
    }
}

/// Axum middleware over the route table and the engine.
pub async fn authorize_mw(State(authz): State<Authz>, req: Request, next: Next) -> Response {
    let actor = req
        .extensions()
        .get::<Subject>()
        .map(|s| audit::Actor {
            roles: s.roles.iter().map(|r| r.as_str().to_string()).collect(),
            organisations: s.organisations.iter().cloned().collect(),
        })
        .unwrap_or_default();
    audit::ACTOR
        .scope(actor, authorize_inner(authz, req, next))
        .await
}

async fn authorize_inner(authz: Authz, req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let subject = req.extensions().get::<Subject>().cloned();
    let agent = req
        .extensions()
        .get::<Agent>()
        .map_or_else(|| auth::ANONYMOUS.to_string(), |a| a.0.clone());
    match check(
        auth::authentication_enforced(),
        &method,
        &path,
        subject.as_ref(),
    ) {
        Ok(()) => next.run(req).await,
        Err((refusal, template)) => {
            audit_refusal(
                &authz.db,
                &agent,
                &method,
                template,
                refusal,
                subject.as_ref(),
            )
            .await;
            (StatusCode::FORBIDDEN, "forbidden").into_response()
        }
    }
}

/// Record the refusal. If the row cannot be written the refusal still stands (nothing was
/// disclosed), and the failure is logged.
async fn audit_refusal(
    db: &DatabaseConnection,
    agent: &str,
    method: &axum::http::Method,
    template: Option<&'static str>,
    refusal: Refusal,
    subject: Option<&Subject>,
) {
    let mut event = Event::new(AuditAction::Execute, agent);
    event.outcome = 4;
    // The route template, never the raw path: a path can carry an identifier.
    event.resource_ref = Some(template.unwrap_or("unmatched-route").to_string());
    event.detail = Some(serde_json::json!({
        "denied": refusal.code(),
        "method": method.as_str(),
        "roles": subject.map(|s| s.roles.iter().map(|r| r.as_str()).collect::<Vec<_>>()),
    }));
    if let Err(e) = audit::append(db, audit::chain(), &event).await {
        tracing::error!(error = %e, "a refusal could not be audited");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::Role;
    use axum::http::Method;

    fn with(roles: &[Role]) -> Subject {
        Subject {
            roles: roles.iter().copied().collect(),
            ..Subject::default()
        }
    }

    #[test]
    fn login_off_or_a_public_path_skips_the_gate() {
        assert!(check(false, &Method::POST, "/api/patients/compare", None).is_ok());
        assert!(check(true, &Method::GET, "/_health", None).is_ok());
    }

    #[test]
    fn a_route_without_a_policy_entry_is_refused_even_for_every_role() {
        let all = with(&Role::ALL);
        assert_eq!(
            check(true, &Method::GET, "/api/anything", Some(&all))
                .unwrap_err()
                .0,
            Refusal::NoPolicy
        );
        assert_eq!(
            check(true, &Method::DELETE, "/api/patients/42", Some(&all))
                .unwrap_err()
                .0,
            Refusal::NoPolicy,
            "the table is by method too"
        );
    }

    #[test]
    fn roles_decide_the_compare_route() {
        for role in Role::ALL {
            let r = check(
                true,
                &Method::POST,
                "/api/patients/compare",
                Some(&with(&[role])),
            );
            let expected = matches!(
                role,
                Role::Clinician
                    | Role::Nurse
                    | Role::Pharmacist
                    | Role::AlliedHealth
                    | Role::Receptionist
                    | Role::ClinicalAdmin
                    | Role::Integrator
            );
            assert_eq!(r.is_ok(), expected, "{role:?}");
        }
        // No roles at all (a valid token that carries none) is refused.
        assert_eq!(
            check(
                true,
                &Method::POST,
                "/api/patients/compare",
                Some(&Subject::default())
            )
            .unwrap_err()
            .0,
            Refusal::NotPermitted
        );
        assert_eq!(
            check(true, &Method::POST, "/api/patients/compare", None)
                .unwrap_err()
                .0,
            Refusal::NoSubject
        );
    }
}
