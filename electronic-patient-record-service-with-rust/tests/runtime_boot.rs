//! QA-16: two apps booted in one program from settings they are given, with no process environment between
//! them. `EPR_METRICS` is read on each request (`controllers/metrics.rs`), so one app serves `/metrics` and the
//! other answers `404`, while both run at once.
//!
//! ```text
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test runtime_boot -- --ignored --test-threads=1
//! ```

use axa_test::prelude::request_with;
use axum::http::StatusCode;
use electronic_patient_record::app::App;

/// The settings every app here needs: the database from the process, which names where a test database is, and
/// login off. The process's environment is read, never written.
fn base() -> Vec<(&'static str, String)> {
    vec![
        (
            "DB_URL",
            std::env::var("DB_URL").expect("DB_URL names the test database"),
        ),
        ("EPR_REQUIRE_AUTH", "false".to_string()),
    ]
}

fn with(extra: &[(&'static str, &str)]) -> Vec<(&'static str, String)> {
    let mut all = base();
    all.extend(extra.iter().map(|(k, v)| (*k, (*v).to_string())));
    all
}

fn borrowed<'a>(pairs: &'a [(&'static str, String)]) -> Vec<(&'static str, &'a str)> {
    pairs.iter().map(|(k, v)| (*k, v.as_str())).collect()
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn each_app_reads_the_settings_it_was_given() {
    let on = with(&[("EPR_METRICS", "1")]);
    let off = base();
    let (on_pairs, off_pairs) = (borrowed(&on), borrowed(&off));
    tokio::join!(
        request_with::<App, _, _>(&on_pairs, |server, _| async move {
            let status = server.get("/metrics").await.status_code();
            assert_eq!(
                status,
                StatusCode::OK,
                "the app given EPR_METRICS=1 serves them"
            );
        }),
        request_with::<App, _, _>(&off_pairs, |server, _| async move {
            let status = server.get("/metrics").await.status_code();
            assert_eq!(
                status,
                StatusCode::NOT_FOUND,
                "the app not given it does not"
            );
        }),
    );
}
