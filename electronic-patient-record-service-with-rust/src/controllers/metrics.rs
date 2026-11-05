//! `GET /metrics`: the process counters (task CQ-17). Public like the health probes, so a scraper
//! needs no token, and 404 unless `EPR_METRICS` is on. See [`crate::metrics`].

use crate::metrics;
use axa::prelude::*;
use axum::response::Response;

/// `GET /metrics`: the Prometheus text exposition, or 404 when metrics are not enabled.
async fn serve() -> Response {
    metrics::response(metrics::enabled(
        std::env::var("EPR_METRICS").ok().as_deref(),
    ))
}

/// The route.
#[must_use]
pub fn routes() -> Routes {
    Routes::new().add("/metrics", get(serve))
}
