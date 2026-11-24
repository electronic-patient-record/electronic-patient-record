//! Chaos testing as every role (the login-on counterpart of `chaos_api`): the same seeded monkey, the same hostile
//! input on every route, but with login **on** and each request carrying one of: a token for each of the service's
//! roles, no token, or a garbage token. Whatever it sends:
//!
//! - no panic;
//! - no 500 (a 5xx other than 502/503/504 is a bug);
//! - no stack trace, SQL text or file path in a response body.
//!
//! The gate, the role checks, consent and the relationship rules all run here, which they do not with login off.
//! It boots with its own settings, so it runs inside `main.rs`. It writes whatever the API allows, so it
//! refuses a database whose name has no "test" in it. `EPR_CHAOS_SEED` repeats a run; `EPR_CHAOS_ITERATIONS`
//! (default 2000) sets its length.
//!
//! ```text
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test main chaos_roles:: -- --ignored --test-threads=1 --nocapture
//! ```

use crate::chaos_support;
use authentication_verifier::test_idp::{AUDIENCE, REALM, TestIdp};
use axa_test::prelude::request_with;
use axum::http::{HeaderName, HeaderValue};
use electronic_patient_record::app::App;
use electronic_patient_record_domain::policy::Role;
use serde_json::{Value, json};

fn bearer(token: &str) -> Option<(HeaderName, HeaderValue)> {
    Some((
        HeaderName::from_static("authorization"),
        HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
    ))
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL (a throwaway database)"]
async fn no_route_panics_answers_500_or_leaks_internals_for_any_role() {
    chaos_support::database_is_throwaway();
    let idp = TestIdp::start().await;
    let map: Value = Role::ALL
        .iter()
        .map(|r| (r.as_str().to_string(), json!({ "role": [r.as_str()] })))
        .collect();
    // This app's own settings: no process environment is written.
    let db_url = std::env::var("DB_URL").expect("DB_URL must be set for --ignored tests");
    let role_map = map.to_string();
    let settings: Vec<(&str, &str)> = vec![
        ("DB_URL", &db_url),
        ("EPR_REQUIRE_AUTH", "true"),
        ("EPR_KEYCLOAK_URL", &idp.base_url),
        ("EPR_KEYCLOAK_REALM", REALM),
        ("EPR_KEYCLOAK_AUDIENCES", AUDIENCE),
        ("EPR_KEYCLOAK_ROLE_MAP", &role_map),
    ];
    let mut callers: Vec<Option<(HeaderName, HeaderValue)>> = vec![None, bearer("not-a-token")];
    for (n, role) in Role::ALL.iter().enumerate() {
        let token = idp.token(|c| {
            c["realm_access"] = json!({ "roles": [role.as_str()] });
            c["sub"] = json!(format!("0c4f1e2a-0000-4000-8000-{:012x}", 0x1000 + n));
        });
        callers.push(bearer(&token));
    }
    let (seed, iterations) = chaos_support::seed_and_iterations();
    request_with::<App, _, _>(&settings, move |server, _ctx| async move {
        let (seen, failures) = chaos_support::monkey(&server, seed, iterations, &callers).await;
        println!("answers by status: {seen:?}");
        // Some requests must pass the gate and be answered by a handler, and some must be refused by it.
        assert!(
            seen.contains_key(&401) && seen.len() >= 5,
            "the monkey saw only these answers: {seen:?}"
        );
        assert!(
            failures.is_empty(),
            "EPR_CHAOS_SEED={seed}: {} bad answers, first ones:\n{}",
            failures.len(),
            failures
                .iter()
                .take(10)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    })
    .await;
}
