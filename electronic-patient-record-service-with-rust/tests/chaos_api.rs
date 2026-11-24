//! Chaos testing (idea taken from the patient-administration-system project): a seeded monkey hits
//! every route the service serves with hostile input. It reads `controllers::all_routes()`, so a
//! new route is covered the day it is added. For each request it picks a method the route accepts,
//! fills the path parameters with plausible and hostile values (random ids, `../`, NUL, long
//! strings, unicode), adds a junk query string and sends a generated body (none, malformed JSON,
//! or random JSON with the field names the handlers read). Whatever it sends, with login off:
//!
//! - no panic (a panic aborts the request task and the harness reports it);
//! - no 500 (a 5xx other than 502/503/504 is a bug);
//! - no stack trace, SQL text or file path in a response body.
//!
//! Seeded: a failure prints `EPR_CHAOS_SEED` and the request, and the same seed repeats it.
//! `EPR_CHAOS_ITERATIONS` (default 2000) sets the length. It writes whatever the API lets it
//! write, so it refuses any database whose name does not contain "test" unless `CI` is set.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test main chaos_api:: -- --ignored --test-threads=1 --nocapture
//! ```

use crate::chaos_support;
use axa_test::prelude::request_with;
use electronic_patient_record::app::App;

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL (a throwaway database)"]
async fn no_route_panics_answers_500_or_leaks_internals_under_hostile_input() {
    chaos_support::database_is_throwaway();
    let db_url = std::env::var("DB_URL").expect("DB_URL must be set for --ignored tests");
    let settings = [("DB_URL", db_url.as_str()), ("EPR_REQUIRE_AUTH", "false")];
    let (seed, iterations) = chaos_support::seed_and_iterations();
    request_with::<App, _, _>(&settings, move |server, _ctx| async move {
        let (seen, failures) = chaos_support::monkey(&server, seed, iterations, &[None]).await;
        println!("answers by status: {seen:?}");
        // A control: if every request were refused alike, the monkey would be testing the gate, not
        // the handlers. Some requests must get past it and be answered or refused by a handler.
        assert!(
            seen.len() >= 4,
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
