//! `/_ping`, `/_health` and `/_readiness`. `axa`'s readiness route checks nothing without its `sea-orm`
//! feature, and a load balancer must not send traffic to a service that cannot reach its database, so this
//! service has its own.

use crate::pool::Pool;
use axa::prelude::*;
use axum::http::StatusCode;
use serde_json::json;

async fn ok() -> Response {
    Json(json!({ "ok": true })).into_response()
}

/// `200` when the database answers, else `503`. The error goes to the log, not the caller: the route is public.
async fn readiness(State(ctx): State<AppContext>) -> Response {
    match sqlx::query("SELECT 1").execute(&ctx.pool()).await {
        Ok(_) => ok().await,
        Err(error) => {
            tracing::error!(%error, "readiness_db_ping_error");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "ok": false })),
            )
                .into_response()
        }
    }
}

/// The three probe routes, all public.
#[must_use]
pub fn routes() -> Routes {
    Routes::new()
        .add("/_ping", get(ok))
        .add("/_health", get(ok))
        .add("/_readiness", get(readiness))
}
