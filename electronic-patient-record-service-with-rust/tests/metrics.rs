//! The process counters (task CQ-17) and `/metrics`. No database needed: a disconnected connection
//! fails every query, which is exactly what the failure counters watch for. The counters are
//! process-wide and tests run side by side, so each test checks it rose, never an exact value.

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::routing::get;
use electronic_patient_record::audit::{self, Action, Event};
use electronic_patient_record::feed_queue::{self, Kind, New};
use electronic_patient_record::{auth, metrics};
use sea_orm::DatabaseConnection;
use tower::ServiceExt;

async fn body(res: axum::response::Response) -> String {
    let bytes = axum::body::to_bytes(res.into_body(), 1 << 20)
        .await
        .unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[tokio::test]
async fn metrics_is_404_when_off_and_lists_every_counter_when_on() {
    assert_eq!(metrics::response(false).status(), StatusCode::NOT_FOUND);
    let on = metrics::response(true);
    assert_eq!(on.status(), StatusCode::OK);
    assert!(
        on.headers()["content-type"]
            .to_str()
            .unwrap()
            .starts_with("text/plain")
    );
    let text = body(on).await;
    for name in [
        "epr_audit_append_failures_total",
        "epr_unaudited_refusals_total",
        "epr_feed_refused_messages_total",
    ] {
        assert!(text.contains(&format!("# TYPE {name} counter")), "{name}");
        assert!(text.contains(&format!("\n{name} ")), "{name} has a value");
    }
}

#[tokio::test]
async fn a_failed_audit_append_is_counted() {
    let before = metrics::AUDIT_APPEND_FAILURES.get();
    let db = DatabaseConnection::default();
    let event = Event::new(Action::Read, "tester");
    assert!(audit::append(&db, audit::chain(), &event).await.is_err());
    assert!(metrics::AUDIT_APPEND_FAILURES.get() > before);
}

#[tokio::test]
async fn a_refused_feed_message_is_counted_even_when_it_cannot_be_stored() {
    let before = metrics::FEED_REFUSALS.get();
    let db = DatabaseConnection::default();
    let reasons = vec!["no PID segment".to_string()];
    let stored = feed_queue::record(
        &db,
        New {
            agent: "feed",
            peer: None,
            event: "ADT^A04",
            control_id: "1",
            kind: Kind::Unsupported,
            reasons: &reasons,
            message: "MSH|^~\\&|",
        },
    )
    .await;
    assert!(stored.is_err());
    assert!(metrics::FEED_REFUSALS.get() > before);
}

#[tokio::test]
async fn a_request_turned_away_before_the_audit_is_counted_and_shown() {
    // `auth::init` has not run in this process, so every non-public route is refused (503).
    let before = metrics::UNAUDITED_REFUSALS.get();
    let app = Router::new()
        .route("/api/anything", get(|| async { "ok" }))
        .layer(axum::middleware::from_fn(auth::require_auth_mw));
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/anything")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert!(metrics::UNAUDITED_REFUSALS.get() > before);
    let text = body(metrics::response(true)).await;
    let line = text
        .lines()
        .find(|l| l.starts_with("epr_unaudited_refusals_total "))
        .unwrap();
    assert_ne!(line, "epr_unaudited_refusals_total 0");
}

#[test]
fn metrics_is_a_public_route_and_is_served() {
    assert!(auth::is_public_path("/metrics"));
    let served = electronic_patient_record::controllers::all_routes()
        .collect()
        .into_iter()
        .any(|r| r.uri == "/metrics");
    assert!(served, "/metrics is not in all_routes()");
}

#[tokio::test]
async fn the_duration_histogram_is_labelled_by_route_template_never_by_path() {
    let app = Router::new()
        .route("/things/{id}", get(|| async { "ok" }))
        .route("/refuses", get(|| async { StatusCode::FORBIDDEN }))
        .layer(axum::middleware::from_fn(metrics::duration_mw));
    for uri in [
        "/things/patient-zq7x-secret",
        "/things/another-id",
        "/refuses",
        "/no/such/route/with-an-id-zq7y",
    ] {
        app.clone()
            .oneshot(Request::get(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
    }
    let text = body(metrics::response(true)).await;
    assert!(
        text.contains(r#"route="/things/{id}",status="200""#),
        "{text}"
    );
    assert!(text.contains(r#"route="/refuses",status="403""#), "{text}");
    assert!(text.contains(r#"route="unmatched",status="404""#), "{text}");
    for leaked in ["zq7x", "secret", "another-id", "zq7y", "with-an-id"] {
        assert!(!text.contains(leaked), "{leaked} leaked into a label");
    }
    assert!(text.contains("# TYPE epr_http_request_duration_seconds histogram"));
}
