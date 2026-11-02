//! Idempotent creates (task OF-1).
//!
//! A `POST` under `/api` that carries an `Idempotency-Key` header is run once. A retry (a double
//! click, a client that timed out) with the same key and the same request gets the stored answer,
//! with `Idempotent-Replayed: true`, and writes nothing. The same key with a different method, path
//! or body is a 422; a key whose first request is still running, or ended without an answer being
//! kept, is a 409. A request without the header behaves as before.
//!
//! Only a successful answer is kept. A refusal or failure releases the key, so the client can fix the
//! request and send it again with the same key. Keys are per token subject and expire after a day; the
//! stored answer can hold patient data, so [`run_loop`] removes expired rows. A replay is audited (it
//! discloses the answer again), and if that row cannot be written the replay is refused.
//!
//! Not covered: `/fhir/` creates, and a request whose first run wrote the record and then failed with
//! a non-success answer (the key is released, so a retry can write again).

use crate::api_error::ApiError;
use crate::audit::{self, Action, Event};
use crate::auth::Agent;
use crate::store::pg;
use axum::{
    body::{Body, to_bytes},
    extract::{Request, State},
    http::{HeaderName, HeaderValue, Method, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr};
use sha2::{Digest, Sha256};

/// The request header.
pub const HEADER: &str = "idempotency-key";
/// Seconds a key is kept: one day.
pub const TTL_SECS: i64 = 86_400;
/// The most request body bytes read for a keyed request.
const MAX_REQUEST: usize = 1 << 20;
/// The most response body bytes kept for a replay.
const MAX_RESPONSE: usize = 4 << 20;

/// The state the middleware needs.
#[derive(Clone)]
pub struct Gate {
    /// Where the keys live.
    pub db: DatabaseConnection,
}

/// Whether `key` is 1 to 128 letters, digits or `. _ : -`.
#[must_use]
pub fn valid_key(key: &str) -> bool {
    (1..=128).contains(&key.len())
        && key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
}

/// An answer kept for replay.
#[derive(Debug, PartialEq, Eq)]
pub struct Stored {
    /// The HTTP status.
    pub status: u16,
    /// The `Content-Type`, if there was one.
    pub content_type: Option<String>,
    /// The body.
    pub body: Vec<u8>,
}

/// What reserving a key found.
#[derive(Debug, PartialEq, Eq)]
pub enum Reservation {
    /// A new key: run the request.
    Fresh,
    /// The same request was answered before.
    Replay(Stored),
    /// The first request has not finished, or finished without an answer being kept.
    Busy,
    /// The key was used for a different request.
    Mismatch,
}

fn digest_of(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::from("sha256:"), |mut s, b| {
            s.push_str(&format!("{b:02x}"));
            s
        })
}

/// Reserve `(subject, key)` for this request, or say what is already there. An expired key is
/// removed first, so it can be used again.
///
/// # Errors
///
/// A database failure.
pub async fn reserve(
    db: &impl ConnectionTrait,
    subject: &str,
    key: &str,
    method: &str,
    path: &str,
    digest: &str,
) -> Result<Reservation, DbErr> {
    db.execute_raw(pg(
        "DELETE FROM idempotency_keys WHERE subject = $1 AND key = $2
         AND created_at < now() - make_interval(secs => $3)",
        vec![subject.into(), key.into(), (TTL_SECS as f64).into()],
    ))
    .await?;
    let inserted = db
        .execute_raw(pg(
            "INSERT INTO idempotency_keys (subject, key, method, path, body_digest, state)
             VALUES ($1, $2, $3, $4, $5, 'pending') ON CONFLICT (subject, key) DO NOTHING",
            vec![
                subject.into(),
                key.into(),
                method.into(),
                path.into(),
                digest.into(),
            ],
        ))
        .await?;
    if inserted.rows_affected() == 1 {
        return Ok(Reservation::Fresh);
    }
    let Some(row) = db
        .query_one_raw(pg(
            "SELECT method, path, body_digest, state, status::int AS status, content_type, response
             FROM idempotency_keys WHERE subject = $1 AND key = $2",
            vec![subject.into(), key.into()],
        ))
        .await?
    else {
        // Removed between the insert and the read (expired and purged): treat as busy, not as new.
        return Ok(Reservation::Busy);
    };
    let same = row.try_get::<String>("", "method")? == method
        && row.try_get::<String>("", "path")? == path
        && row.try_get::<String>("", "body_digest")? == digest;
    if !same {
        return Ok(Reservation::Mismatch);
    }
    if row.try_get::<String>("", "state")? != "done" {
        return Ok(Reservation::Busy);
    }
    let status: i32 = row.try_get("", "status")?;
    Ok(Reservation::Replay(Stored {
        status: u16::try_from(status).unwrap_or(500),
        content_type: row.try_get("", "content_type")?,
        body: row.try_get("", "response")?,
    }))
}

/// Keep the answer for `(subject, key)`.
///
/// # Errors
///
/// A database failure.
pub async fn complete(
    db: &impl ConnectionTrait,
    subject: &str,
    key: &str,
    stored: &Stored,
) -> Result<(), DbErr> {
    db.execute_raw(pg(
        "UPDATE idempotency_keys SET state = 'done', status = $3, content_type = $4, response = $5
         WHERE subject = $1 AND key = $2 AND state = 'pending'",
        vec![
            subject.into(),
            key.into(),
            i16::try_from(stored.status).unwrap_or(500).into(),
            stored.content_type.clone().into(),
            stored.body.clone().into(),
        ],
    ))
    .await?;
    Ok(())
}

/// Let go of a key whose request did not succeed, so it can be sent again.
///
/// # Errors
///
/// A database failure.
pub async fn release(db: &impl ConnectionTrait, subject: &str, key: &str) -> Result<(), DbErr> {
    db.execute_raw(pg(
        "DELETE FROM idempotency_keys WHERE subject = $1 AND key = $2 AND state = 'pending'",
        vec![subject.into(), key.into()],
    ))
    .await?;
    Ok(())
}

/// Remove keys older than [`TTL_SECS`]; returns how many.
///
/// # Errors
///
/// A database failure.
pub async fn purge_expired(db: &impl ConnectionTrait) -> Result<u64, DbErr> {
    Ok(db
        .execute_raw(pg(
            "DELETE FROM idempotency_keys WHERE created_at < now() - make_interval(secs => $1)",
            vec![(TTL_SECS as f64).into()],
        ))
        .await?
        .rows_affected())
}

/// Purge expired keys every hour, for the life of the process.
pub async fn run_loop(db: DatabaseConnection) {
    let mut tick = tokio::time::interval(std::time::Duration::from_secs(3600));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tick.tick().await;
        match purge_expired(&db).await {
            Ok(0) => {}
            Ok(n) => tracing::info!(removed = n, "idempotency keys expired"),
            Err(e) => tracing::error!(error = %e, "idempotency keys could not be purged"),
        }
    }
}

fn replayed(stored: Stored) -> Response {
    let mut response = (
        StatusCode::from_u16(stored.status).unwrap_or(StatusCode::OK),
        stored.body,
    )
        .into_response();
    let h = response.headers_mut();
    if let Some(ct) = stored
        .content_type
        .and_then(|c| HeaderValue::from_str(&c).ok())
    {
        h.insert(header::CONTENT_TYPE, ct);
    }
    h.insert(
        HeaderName::from_static("idempotent-replayed"),
        HeaderValue::from_static("true"),
    );
    response
}

/// Run a keyed `POST` once. Mounted inside authorization, so a refused request never reserves a key.
pub async fn idempotency_mw(State(gate): State<Gate>, req: Request, next: Next) -> Response {
    let path = req.uri().path().to_string();
    let Some(raw) = req.headers().get(HEADER).cloned() else {
        return next.run(req).await;
    };
    if req.method() != Method::POST || !path.starts_with("/api/") {
        return next.run(req).await;
    }
    let Some(agent) = req.extensions().get::<Agent>().map(|a| a.0.clone()) else {
        return next.run(req).await;
    };
    let key = match raw.to_str() {
        Ok(k) if valid_key(k) => k.to_string(),
        _ => {
            return ApiError::bad_request(
                "Idempotency-Key must be 1 to 128 letters, digits or . _ : -",
            )
            .into_response();
        }
    };
    let (parts, body) = req.into_parts();
    let Ok(bytes) = to_bytes(body, MAX_REQUEST).await else {
        return ApiError::bad_request(
            "the request body is too large to send with an Idempotency-Key",
        )
        .into_response();
    };
    let digest = digest_of(&bytes);
    let method = parts.method.as_str().to_string();
    match reserve(&gate.db, &agent, &key, &method, &path, &digest).await {
        Err(e) => {
            // Without the guard the request could write twice, so it is not run.
            tracing::error!(error = %e, "the Idempotency-Key could not be checked");
            ApiError::internal().into_response()
        }
        Ok(Reservation::Mismatch) => ApiError::invalid(vec![
            "this Idempotency-Key was used for a different request".to_string(),
        ])
        .into_response(),
        Ok(Reservation::Busy) => ApiError::conflict(
            "a request with this Idempotency-Key is still running, or ended without an answer being kept; use a new key",
        )
        .into_response(),
        Ok(Reservation::Replay(stored)) => {
            let mut event = Event::new(Action::Read, agent);
            event.resource_ref = Some("IdempotentReplay".to_string());
            event.detail = Some(serde_json::json!({ "method": method, "path": path }));
            if let Err(e) = audit::append(&gate.db, audit::chain(), &event).await {
                tracing::error!(error = %e, "refusing to replay: it could not be audited");
                return ApiError::internal().into_response();
            }
            replayed(stored)
        }
        Ok(Reservation::Fresh) => {
            let response = next
                .run(Request::from_parts(parts, Body::from(bytes)))
                .await;
            let (head, body) = response.into_parts();
            let kept = if head.status.is_success() {
                to_bytes(body, MAX_RESPONSE).await.ok()
            } else {
                None
            };
            let Some(kept) = kept else {
                if let Err(e) = release(&gate.db, &agent, &key).await {
                    tracing::error!(error = %e, "an Idempotency-Key could not be released");
                }
                // The body of a refusal is small; of a success we could not keep, say so.
                return if head.status.is_success() {
                    ApiError::internal().into_response()
                } else {
                    Response::from_parts(head, Body::empty())
                };
            };
            let stored = Stored {
                status: head.status.as_u16(),
                content_type: head
                    .headers
                    .get(header::CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok())
                    .map(str::to_string),
                body: kept.to_vec(),
            };
            if let Err(e) = complete(&gate.db, &agent, &key, &stored).await {
                // The record is written; a retry is told the key is busy, never allowed to write again.
                tracing::error!(error = %e, "an answer could not be kept for its Idempotency-Key");
            }
            Response::from_parts(head, Body::from(kept))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_is_one_to_128_safe_characters() {
        for good in ["a", "Abc-123_x.y:z", &"k".repeat(128)] {
            assert!(valid_key(good), "{good}");
        }
        for bad in [
            "",
            " ",
            "a b",
            "a/b",
            "ключ",
            &"k".repeat(129),
            "a\nb",
            "a;b",
        ] {
            assert!(!valid_key(bad), "{bad:?}");
        }
    }

    #[test]
    fn the_digest_names_its_algorithm_and_differs_by_body() {
        assert!(digest_of(b"x").starts_with("sha256:"));
        assert_ne!(digest_of(b"x"), digest_of(b"y"));
        assert_eq!(digest_of(b"x"), digest_of(b"x"));
    }
}
