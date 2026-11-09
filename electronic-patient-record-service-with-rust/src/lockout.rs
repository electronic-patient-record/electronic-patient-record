//! Lockout by client address (task OS-2; `spec/07` NFR-6), counted by `axa-ratelimit`.
//!
//! Off unless [`SETTING`] is set. When it is, an address that has been answered that many `401`s in a
//! fixed [`WINDOW_SECS`] window is refused every request until the window ends, before authentication
//! runs: 429 with `Retry-After`, in the route's own shape. Only a `401` counts, so a client with a valid
//! token is never slowed. The first refusal of an address in a window is an audit row naming the
//! address; every refusal is counted in `epr_lockouts_total`.
//!
//! It is off by default because the UI's server does not forward the browser's address: behind it every
//! user is one address, and a few expired tokens would lock out every clinician at once (H-049).

use crate::api_error::ApiError;
use crate::audit::{self, Action, Event};
use crate::auth::ANONYMOUS;
use axa::ratelimit::{Limiter, Verdict, WhenFull, lock_out_failures};
use axum::{
    Router,
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::{Next, from_fn_with_state},
    response::Response,
};
use sqlx::PgPool;
use std::net::IpAddr;
use std::num::NonZeroU32;

/// The setting: how many `401`s an address may be answered in a window before it is locked out.
pub const SETTING: &str = "LOCKOUT_CLIENT_ADDRESS_AFTER_COUNT";
/// Seconds in a window, and so the longest a lockout lasts.
pub const WINDOW_SECS: u32 = 900;
/// The largest count the setting may give.
const MAX_SETTING: u32 = 1_000_000;

/// The count from its setting: `None` (unset) is off; anything but a whole number from 1 to 1,000,000 is
/// an error that stops the service starting.
///
/// # Errors
///
/// The text of the bad setting.
pub fn parse_count(raw: Option<&str>) -> Result<Option<NonZeroU32>, String> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    raw.trim()
        .parse::<NonZeroU32>()
        .ok()
        .filter(|n| n.get() <= MAX_SETTING)
        .map(Some)
        .ok_or_else(|| {
            format!("{SETTING} must be a whole number from 1 to {MAX_SETTING}, not {raw:?}")
        })
}

/// The count from [`SETTING`], or `None` when the lockout is off.
///
/// # Errors
///
/// The text of a bad setting.
pub fn count_from_env() -> Result<Option<NonZeroU32>, String> {
    parse_count(std::env::var(SETTING).ok().as_deref())
}

/// A limiter that locks an address out after `count` `401`s in a window. It fails open: a new address
/// that finds the table full is let through uncounted, never refused.
#[must_use]
pub fn limiter(count: NonZeroU32) -> Limiter {
    let window = NonZeroU32::new(WINDOW_SECS).unwrap_or(NonZeroU32::MIN);
    Limiter::new(count, window).when_full(WhenFull::Allow)
}

/// The answering layer's state.
#[derive(Clone)]
struct Gate {
    /// For the audit row.
    db: PgPool,
}

/// Lock out an address that earns too many `401`s on `router`, when `lockout` is `Some`. Add it outside
/// authentication, so that it sees authentication's `401`s and refuses before a token is checked.
pub fn layer(router: Router, lockout: Option<Limiter>, db: PgPool) -> Router {
    let Some(limiter) = lockout else {
        return router;
    };
    router
        .layer(from_fn_with_state(limiter, lock_out_failures))
        .layer(from_fn_with_state(Gate { db }, answer_mw))
}

/// Outside the lockout: count an overflow, and for a refusal count it, audit the first in the window,
/// and answer in the route's own shape with the limiter's headers.
async fn answer_mw(State(gate): State<Gate>, req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let path = req.uri().path().to_owned();
    let address = req.extensions().get::<IpAddr>().map(ToString::to_string);
    let res = next.run(req).await;
    let Some(verdict) = res.extensions().get::<Verdict>().copied() else {
        return res;
    };
    if verdict.overflowed {
        crate::metrics::RATE_LIMIT_OVERFLOWED.inc();
    }
    if verdict.allowed {
        return res;
    }
    crate::metrics::LOCKOUTS.inc();
    if verdict.first_refusal {
        let mut event = Event::new(Action::Execute, ANONYMOUS.to_string());
        event.outcome = 4;
        event.resource_ref = Some("lockout".to_string());
        event.detail = Some(serde_json::json!({
            "denied": "lockout",
            "method": method.as_str(),
        }));
        event.network_address = address;
        if let Err(e) = audit::append(&gate.db, audit::chain(), &event).await {
            crate::metrics::UNAUDITED_REFUSALS.inc();
            tracing::error!(error = %e, "a lockout could not be audited");
        }
    }
    let mut answer = ApiError::new(
        StatusCode::TOO_MANY_REQUESTS,
        "too many failed sign-ins from this address; wait and sign in again",
    )
    .with("retry_after_seconds", verdict.reset_seconds)
    .for_path(&path);
    verdict.stamp(&mut answer);
    answer.headers_mut().insert(
        header::RETRY_AFTER,
        axum::http::HeaderValue::from(verdict.reset_seconds),
    );
    answer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unset_is_off_and_a_bad_setting_stops_the_service() {
        assert_eq!(parse_count(None), Ok(None));
        assert_eq!(parse_count(Some(" 10 ")), Ok(NonZeroU32::new(10)));
        assert_eq!(parse_count(Some("1000000")), Ok(NonZeroU32::new(1_000_000)));
        for bad in ["0", "1000001", "-1", "off", "", "1.5"] {
            let e = parse_count(Some(bad)).unwrap_err();
            assert!(e.contains(SETTING), "{bad:?}: {e}");
        }
    }

    #[test]
    fn a_full_table_lets_a_new_address_through() {
        let l = limiter(NonZeroU32::MIN);
        assert!(l.check("a").allowed);
        assert!(!l.check("a").allowed, "the count is the edge");
        for i in 0..axa::ratelimit::MAX_KEYS {
            l.check(i);
        }
        let v = l.check("new");
        assert!(v.allowed && v.overflowed);
    }
}
