//! Route census: every route the service serves is either public or has a policy entry, and every
//! policy entry names a route that exists. A new endpoint therefore cannot reach production
//! without being given a resource and an action, and the UI cannot reach one that skips the
//! policy engine, because the authorization middleware refuses any route without an entry.

use electronic_patient_record::{auth, controllers, routes};

fn served() -> Vec<(String, String)> {
    controllers::all_routes()
        .collect()
        .into_iter()
        .flat_map(|route| {
            let uri = routes::normalise(&route.uri);
            route
                .actions
                .into_iter()
                .map(move |m| (m.to_string(), uri.clone()))
        })
        .collect()
}

#[test]
fn every_served_route_is_public_or_has_a_policy_entry() {
    let served = served();
    assert!(!served.is_empty(), "the census saw no routes");
    for (method, uri) in &served {
        let allowed = auth::is_public_path(uri)
            || routes::TABLE
                .iter()
                .any(|(m, t, _, _)| m == method && routes::template_matches(t, uri));
        assert!(
            allowed,
            "{method} {uri} has no policy entry in src/routes.rs"
        );
    }
}

#[test]
fn every_policy_entry_names_a_served_route() {
    let served = served();
    for (method, template, _, _) in routes::TABLE {
        assert!(
            served
                .iter()
                .any(|(m, uri)| m == method && routes::normalise(uri) == *template),
            "{method} {template} is in the policy table but is not served"
        );
    }
}

#[test]
fn only_health_probes_are_public() {
    for (method, uri) in served() {
        if auth::is_public_path(&uri) {
            assert!(
                ["/_health", "/_ping", "/_readiness"].contains(&uri.as_str()),
                "{method} {uri} is public"
            );
        }
    }
}
