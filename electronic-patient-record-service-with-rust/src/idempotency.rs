//! Idempotent creates (task OF-1), run by `axa-idempotency`.
//!
//! A `POST` under `/api` that carries an `Idempotency-Key` header is run once. A retry (a double
//! click, a client that timed out) with the same key and the same request gets the kept answer,
//! with `Idempotent-Replayed: true`, and writes nothing. The same key with a different method, path,
//! query or body is a 422; a key whose first request is still running, or ended without its answer
//! being kept, is a 409. A request without the header behaves as before.
//!
//! Only a `2xx` or `3xx` answer is kept. A refusal or failure releases the key, so the client can fix
//! the request and send it again with the same key. Keys are per token subject and expire after a day;
//! the kept answer can hold patient data, so [`run_loop`] sweeps expired rows. A replay is audited (it
//! discloses the answer again), and if that row cannot be written the replay is refused.
//!
//! **The lease is the key's whole life** ([`LEASE`]). `axa-idempotency` lets the same request take over a
//! key whose run stopped reporting in, so that a crash does not block a retry. Its default of a minute
//! would also let a retry write a second record when the first run wrote the record and then could not
//! keep its answer, or crashed after its commit. Here that retry is a 409 until the key expires, as it
//! was before (H-043).
//!
//! Not covered: `/fhir/` creates, other methods, and a request whose first run wrote the record and then
//! failed with a non-success answer (the key is released, so a retry can write again).

use crate::api_error::ApiError;
use crate::audit::{self, Action, Event};
use crate::auth::Agent;
use axa::idempotency::{Idempotency, REPLAYED, idempotent};
use axum::{
    Router,
    extract::{Request, State},
    http::Method,
    middleware::{Next, from_fn_with_state},
    response::{IntoResponse, Response},
};
use sqlx::PgPool;
use std::time::Duration;

/// The request header.
pub const HEADER: &str = "idempotency-key";
/// How long a key and its answer are kept: one day.
pub const TTL: Duration = Duration::from_hours(24);
/// How long a run may go without reporting in before the same request may take its key over: as long
/// as the key lives, so that a run that may have written is never run again under its key.
pub const LEASE: Duration = TTL;
/// The most request body bytes read for a keyed request.
const MAX_REQUEST: usize = 1 << 20;
/// The most response body bytes kept for a replay.
const MAX_RESPONSE: usize = 4 << 20;

/// Whether a request is one this layer runs once: a `POST` under `/api/`.
fn covered(method: &Method, path: &str) -> bool {
    method == Method::POST && path.starts_with("/api/")
}

/// The keys, in `db`, per token subject.
#[must_use]
pub fn keys(db: PgPool) -> Idempotency {
    Idempotency::new(db, |req: &Request| {
        req.extensions().get::<Agent>().map(|a| a.0.clone())
    })
    .only(covered)
    .ttl(TTL)
    .lease(LEASE)
    .max_request_bytes(MAX_REQUEST)
    .max_response_bytes(MAX_RESPONSE)
}

/// The audit layer's state.
#[derive(Clone)]
struct Gate {
    /// For the audit row.
    db: PgPool,
}

/// Run each keyed `POST` on `router` once. Add it inside authorization, so a refused request never
/// takes a key and a replay reaches only a caller the gate let through.
pub fn layer(router: Router, keys: Idempotency, db: PgPool) -> Router {
    router
        .layer(from_fn_with_state(keys, idempotent))
        .layer(from_fn_with_state(Gate { db }, audit_replay_mw))
}

/// Outside the keys: a replay discloses its answer again, so it is an audit row, and a replay that
/// cannot be audited is refused.
async fn audit_replay_mw(State(gate): State<Gate>, req: Request, next: Next) -> Response {
    if !covered(req.method(), req.uri().path()) || !req.headers().contains_key(HEADER) {
        return next.run(req).await;
    }
    let method = req.method().as_str().to_owned();
    let path = req.uri().path().to_owned();
    let agent = req.extensions().get::<Agent>().map(|a| a.0.clone());
    let res = next.run(req).await;
    let (true, Some(agent)) = (res.headers().contains_key(REPLAYED), agent) else {
        return res;
    };
    let mut event = Event::new(Action::Read, agent);
    event.resource_ref = Some("IdempotentReplay".to_string());
    event.detail = Some(serde_json::json!({ "method": method, "path": path }));
    if let Err(e) = audit::append(&gate.db, audit::chain(), &event).await {
        tracing::error!(error = %e, "refusing to replay: it could not be audited");
        return ApiError::internal().into_response();
    }
    res
}

/// Sweep expired keys every hour, for the life of the process.
pub async fn run_loop(keys: Idempotency) {
    let mut tick = tokio::time::interval(Duration::from_secs(3600));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tick.tick().await;
        match keys.sweep().await {
            Ok(0) => {}
            Ok(n) => tracing::info!(removed = n, "idempotency keys expired"),
            Err(e) => tracing::error!(error = %e, "idempotency keys could not be swept"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_post_under_api_is_covered() {
        assert!(covered(&Method::POST, "/api/patients"));
        for (m, p) in [
            (Method::POST, "/fhir/R4/Patient"),
            (Method::PUT, "/api/patients/x"),
            (Method::PATCH, "/api/patients/x"),
            (Method::GET, "/api/patients"),
        ] {
            assert!(!covered(&m, p), "{m} {p}");
        }
    }

    #[test]
    fn the_migration_makes_the_table_axa_idempotency_expects() {
        let sql = include_str!("../migrations/20261107000027_axa_idempotency.sql");
        let squash = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
        let migration = squash(sql);
        for statement in axa::idempotency::SETUP_STATEMENTS {
            assert!(
                migration.contains(&squash(statement)),
                "the migration has drifted from axa-idempotency: {statement}"
            );
        }
    }

    #[test]
    fn a_run_that_may_have_written_is_never_taken_over_while_its_key_lives() {
        assert!(LEASE >= TTL);
    }
}
