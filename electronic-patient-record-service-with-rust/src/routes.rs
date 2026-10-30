//! The route policy table: every route the service serves, with the resource and action it
//! needs. The authorization middleware looks each request up here; a route with no entry is
//! refused (403), so a new endpoint cannot skip the policy engine by being forgotten. The route
//! census test (`tests/route_census.rs`) fails the build when a route and this table disagree.

use crate::policy::{Action, Resource};
use axum::http::Method;

/// What a route needs from the policy engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Requirement {
    /// The kind of resource the route touches.
    pub resource: Resource,
    /// What it does to it.
    pub action: Action,
}

/// `(method, path template, requirement)`. A `{name}` segment matches one non-empty segment.
/// Health probes are not here: they are public (`auth::is_public_path`).
pub const TABLE: &[(&str, &str, Resource, Action)] = &[
    (
        "POST",
        "/api/patients/compare",
        Resource::PatientIdentity,
        Action::Search,
    ),
    (
        "POST",
        "/api/patients",
        Resource::PatientIdentity,
        Action::Create,
    ),
    (
        "GET",
        "/api/patients",
        Resource::PatientIdentity,
        Action::Search,
    ),
    (
        "GET",
        "/api/patients/{id}",
        Resource::PatientIdentity,
        Action::Read,
    ),
    (
        "PUT",
        "/api/patients/{id}",
        Resource::PatientIdentity,
        Action::Update,
    ),
    ("GET", "/api/audit", Resource::AuditLog, Action::Search),
    (
        "GET",
        "/api/audit/export",
        Resource::AuditLog,
        Action::Export,
    ),
    (
        "GET",
        "/fhir/R4/AuditEvent",
        Resource::AuditLog,
        Action::Search,
    ),
    ("GET", "/api/audit/verify", Resource::AuditLog, Action::Read),
    (
        "POST",
        "/api/patients/{id}/consents",
        Resource::Consent,
        Action::Create,
    ),
    (
        "GET",
        "/api/patients/{id}/consents",
        Resource::Consent,
        Action::Read,
    ),
    (
        "PUT",
        "/api/patients/{id}/consents/{consent_id}",
        Resource::Consent,
        Action::Update,
    ),
    (
        "POST",
        "/api/patients/{id}/relationships",
        Resource::CareRelationship,
        Action::Create,
    ),
    (
        "GET",
        "/api/patients/{id}/relationships",
        Resource::CareRelationship,
        Action::Read,
    ),
    (
        "PUT",
        "/api/patients/{id}/relationships/{relationship_id}",
        Resource::CareRelationship,
        Action::Update,
    ),
    (
        "GET",
        "/api/openapi.json",
        Resource::PatientIdentity,
        Action::Read,
    ),
    (
        "GET",
        "/fhir/R4/metadata",
        Resource::PatientIdentity,
        Action::Read,
    ),
    (
        "GET",
        "/fhir/R4/Patient",
        Resource::PatientIdentity,
        Action::Search,
    ),
    (
        "POST",
        "/fhir/R4/Patient",
        Resource::PatientIdentity,
        Action::Create,
    ),
    (
        "GET",
        "/fhir/R4/Patient/{id}",
        Resource::PatientIdentity,
        Action::Read,
    ),
    (
        "PUT",
        "/fhir/R4/Patient/{id}",
        Resource::PatientIdentity,
        Action::Update,
    ),
    (
        "GET",
        "/fhir/R5/metadata",
        Resource::PatientIdentity,
        Action::Read,
    ),
    (
        "GET",
        "/fhir/R5/Patient",
        Resource::PatientIdentity,
        Action::Search,
    ),
    (
        "GET",
        "/fhir/R5/Patient/{id}",
        Resource::PatientIdentity,
        Action::Read,
    ),
    (
        "GET",
        "/fhir/R5/AllergyIntolerance",
        Resource::AllergyIntolerance,
        Action::Read,
    ),
    (
        "GET",
        "/fhir/R5/Observation",
        Resource::Observation,
        Action::Read,
    ),
    (
        "GET",
        "/fhir/R4/AllergyIntolerance",
        Resource::AllergyIntolerance,
        Action::Read,
    ),
    (
        "GET",
        "/fhir/R4/Observation",
        Resource::Observation,
        Action::Read,
    ),
    (
        "GET",
        "/api/feed/refusals",
        Resource::FeedQueue,
        Action::Search,
    ),
    (
        "GET",
        "/api/feed/refusals/count",
        Resource::FeedQueue,
        Action::Search,
    ),
    (
        "GET",
        "/api/feed/refusals/{id}",
        Resource::FeedQueue,
        Action::Read,
    ),
    (
        "POST",
        "/api/feed/refusals/{id}/resolve",
        Resource::FeedQueue,
        Action::Update,
    ),
    (
        "POST",
        "/api/patients/{id}/break-glass",
        Resource::BreakGlass,
        Action::Create,
    ),
    (
        "GET",
        "/api/break-glass/reviews",
        Resource::BreakGlass,
        Action::Search,
    ),
    (
        "GET",
        "/api/break-glass/reviews/count",
        Resource::BreakGlass,
        Action::Search,
    ),
    (
        "POST",
        "/api/break-glass/reviews/{id}",
        Resource::BreakGlass,
        Action::Update,
    ),
    (
        "POST",
        "/api/patients/{id}/merge",
        Resource::PatientLink,
        Action::Create,
    ),
    (
        "POST",
        "/api/patients/{id}/unmerge",
        Resource::PatientLink,
        Action::Update,
    ),
    (
        "GET",
        "/api/patients/{id}/links",
        Resource::PatientLink,
        Action::Read,
    ),
    (
        "GET",
        "/api/patients/{id}/allergies",
        Resource::AllergyIntolerance,
        Action::Read,
    ),
    (
        "POST",
        "/api/patients/{id}/allergies",
        Resource::AllergyIntolerance,
        Action::Create,
    ),
    (
        "PUT",
        "/api/patients/{id}/allergies/{allergy_id}",
        Resource::AllergyIntolerance,
        Action::Update,
    ),
    (
        "PUT",
        "/api/patients/{id}/allergy-statement",
        Resource::AllergyIntolerance,
        Action::Create,
    ),
    (
        "GET",
        "/api/patients/{id}/amendments",
        Resource::Amendment,
        Action::Read,
    ),
    (
        "POST",
        "/api/patients/{id}/amendments",
        Resource::Amendment,
        Action::Create,
    ),
    (
        "PUT",
        "/api/patients/{id}/amendments/{amendment_id}",
        Resource::Amendment,
        Action::Update,
    ),
    ("GET", "/api/hazards", Resource::Hazard, Action::Read),
    ("POST", "/api/hazards", Resource::Hazard, Action::Create),
    (
        "PUT",
        "/api/hazards/{hazard_id}",
        Resource::Hazard,
        Action::Update,
    ),
    (
        "GET",
        "/api/patients/{id}/observations",
        Resource::Observation,
        Action::Read,
    ),
    (
        "POST",
        "/api/patients/{id}/observations",
        Resource::Observation,
        Action::Create,
    ),
    (
        "PUT",
        "/api/patients/{id}/observations/{observation_id}",
        Resource::Observation,
        Action::Update,
    ),
    (
        "GET",
        "/api/patients/{id}/observations/{observation_id}/versions",
        Resource::Observation,
        Action::Read,
    ),
    (
        "GET",
        "/api/patients/{id}/photo",
        Resource::PatientIdentity,
        Action::Read,
    ),
    (
        "PUT",
        "/api/patients/{id}/photo",
        Resource::PatientIdentity,
        Action::Update,
    ),
    (
        "POST",
        "/api/patients/{id}/photo/withdraw",
        Resource::PatientIdentity,
        Action::Update,
    ),
    (
        "GET",
        "/api/me/patients",
        Resource::PatientIdentity,
        Action::Read,
    ),
    (
        "GET",
        "/api/me/patients/{id}/access-log",
        Resource::PatientIdentity,
        Action::Read,
    ),
];

/// Whether `path` matches `template`.
#[must_use]
pub fn template_matches(template: &str, path: &str) -> bool {
    let mut t = template.split('/');
    let mut p = path.split('/');
    loop {
        match (t.next(), p.next()) {
            (None, None) => return true,
            (Some(t), Some(p)) => {
                let wildcard = t.starts_with('{') && t.ends_with('}');
                if wildcard {
                    if p.is_empty() {
                        return false;
                    }
                } else if t != p {
                    return false;
                }
            }
            _ => return false,
        }
    }
}

/// The entry for a request, with the template that matched.
#[must_use]
pub fn lookup(method: &Method, path: &str) -> Option<(&'static str, Requirement)> {
    TABLE
        .iter()
        .find(|(m, template, _, _)| *m == method.as_str() && template_matches(template, path))
        .map(|(_, template, resource, action)| {
            (
                *template,
                Requirement {
                    resource: *resource,
                    action: *action,
                },
            )
        })
}

/// Normalise a framework route (`/api/patients/:id` or `{id}`) to the table's `{id}` style.
#[must_use]
pub fn normalise(uri: &str) -> String {
    uri.split('/')
        .map(|s| match s.strip_prefix(':') {
            Some(name) => format!("{{{name}}}"),
            None => s.to_string(),
        })
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn templates_match_whole_segments() {
        assert!(template_matches(
            "/api/patients/compare",
            "/api/patients/compare"
        ));
        assert!(!template_matches(
            "/api/patients/compare",
            "/api/patients/compare/x"
        ));
        assert!(!template_matches("/api/patients/compare", "/api/patients"));
        assert!(template_matches("/api/patients/{id}", "/api/patients/42"));
        assert!(!template_matches("/api/patients/{id}", "/api/patients/"));
        assert!(!template_matches(
            "/api/patients/{id}",
            "/api/patients/42/x"
        ));
    }

    #[test]
    fn lookup_is_by_method_and_path() {
        assert!(lookup(&Method::POST, "/api/patients/compare").is_some());
        assert!(lookup(&Method::DELETE, "/api/patients/compare").is_none());
        assert!(
            lookup(&Method::DELETE, "/api/patients/42").is_none(),
            "there is no delete"
        );
        assert!(lookup(&Method::POST, "/api/unknown").is_none());
        // Traversal-ish paths do not match a table entry by accident.
        assert!(lookup(&Method::POST, "/api/patients/compare/../x").is_none());
    }

    #[test]
    fn framework_params_normalise() {
        assert_eq!(normalise("/api/patients/:id/x"), "/api/patients/{id}/x");
        assert_eq!(normalise("/api/patients/{id}"), "/api/patients/{id}");
    }
}
