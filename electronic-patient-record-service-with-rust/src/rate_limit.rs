//! Request limits per token subject (task OS-2, first slice; `spec/07` NFR-6), counted by `axa-ratelimit`.
//!
//! A fixed one-minute window per `(subject, class)` held in the process. The classes are `general`
//! and `compare` (`POST /api/patients/compare`), each its own [`axa::ratelimit::Limiter`] counting by
//! the verified token subject, so a compare call counts only against the compare limit. A request
//! without a token subject (login off, public paths) is not counted. A request past its limit is
//! answered 429 with `Retry-After` and the `RateLimit-*` headers; the first refusal of a subject in a
//! window is an audit row and every refusal is counted. A limit is a refusal a clinician could meet
//! (H-039), so the defaults sit far above clinical use and both limiters fail open
//! ([`WhenFull::Allow`]): a new subject that finds the table full is let through uncounted, and counted
//! in `epr_rate_limit_overflowed_total`. The limiter holds a subject only as a keyed hash.

use crate::api_error::ApiError;
use crate::audit::{self, Action, Event};
use crate::auth::{ANONYMOUS, Agent};
use axa::ratelimit::{Keyed, Limiter, Verdict, WhenFull, by_key};
use axum::{
    Router,
    extract::{Request, State},
    http::{Method, StatusCode, header},
    middleware::{Next, from_fn_with_state},
    response::Response,
};
use sqlx::PgPool;
use std::num::NonZeroU32;

/// Seconds in a window.
pub const WINDOW_SECS: u32 = 60;
/// Default requests a subject may make per window, all routes but compare.
pub const DEFAULT_GENERAL: NonZeroU32 = NonZeroU32::new(1200).unwrap();
/// Default compare calls a subject may make per window.
pub const DEFAULT_COMPARE: NonZeroU32 = NonZeroU32::new(120).unwrap();
/// The path of the compare call.
const COMPARE_PATH: &str = "/api/patients/compare";
/// The largest limit a setting may give.
const MAX_SETTING: u32 = 1_000_000;

/// What a request is counted against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Class {
    /// Every route but compare.
    General,
    /// `POST /api/patients/compare`.
    Compare,
}

impl Class {
    /// The class of `req`.
    fn of(req: &Request) -> Class {
        if req.method() == Method::POST && req.uri().path() == COMPARE_PATH {
            Class::Compare
        } else {
            Class::General
        }
    }

    /// The name recorded in the audit row.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Class::General => "general",
            Class::Compare => "compare",
        }
    }
}

/// The limits per window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// All routes but compare.
    pub general: NonZeroU32,
    /// The compare call.
    pub compare: NonZeroU32,
}

impl Default for Limits {
    /// The built-in limits: `DEFAULT_GENERAL` and `DEFAULT_COMPARE` requests per `WINDOW_SECS` window.
    fn default() -> Self {
        Limits {
            general: DEFAULT_GENERAL,
            compare: DEFAULT_COMPARE,
        }
    }
}

/// A limit from its setting: `None` is the default, anything but a whole number from 1 to 1,000,000 is
/// an error that stops the service starting.
///
/// # Errors
///
/// The text of the bad setting.
pub fn parse_limit(
    name: &str,
    raw: Option<&str>,
    default: NonZeroU32,
) -> Result<NonZeroU32, String> {
    let Some(raw) = raw else {
        return Ok(default);
    };
    raw.trim()
        .parse::<NonZeroU32>()
        .ok()
        .filter(|n| n.get() <= MAX_SETTING)
        .ok_or_else(|| {
            format!("{name} must be a whole number from 1 to {MAX_SETTING}, not {raw:?}")
        })
}

impl Limits {
    /// The limits from `EPR_RATE_LIMIT_PER_MINUTE` and `EPR_RATE_LIMIT_COMPARE_PER_MINUTE`.
    ///
    /// # Errors
    ///
    /// The text of a bad setting.
    pub fn from_env() -> Result<Limits, String> {
        let get = |n: &str| std::env::var(n).ok();
        Ok(Limits {
            general: parse_limit(
                "EPR_RATE_LIMIT_PER_MINUTE",
                get("EPR_RATE_LIMIT_PER_MINUTE").as_deref(),
                DEFAULT_GENERAL,
            )?,
            compare: parse_limit(
                "EPR_RATE_LIMIT_COMPARE_PER_MINUTE",
                get("EPR_RATE_LIMIT_COMPARE_PER_MINUTE").as_deref(),
                DEFAULT_COMPARE,
            )?,
        })
    }
}

/// The verified token subject of `req`, if it has one.
fn subject(req: &Request) -> Option<String> {
    req.extensions()
        .get::<Agent>()
        .map(|a| a.0.clone())
        .filter(|a| a != ANONYMOUS)
}

/// The two limiters. Clones share their counts.
#[derive(Debug, Clone)]
pub struct Limiters {
    /// Every route but compare, by subject.
    pub general: Keyed<String>,
    /// The compare call, by subject.
    pub compare: Keyed<String>,
}

impl Limiters {
    /// Limiters at `limits`, failing open when their table is full.
    #[must_use]
    pub fn new(limits: Limits) -> Self {
        let window = NonZeroU32::new(WINDOW_SECS).unwrap_or(NonZeroU32::MIN);
        let limiter = |limit| Limiter::new(limit, window).when_full(WhenFull::Allow);
        Limiters {
            general: limiter(limits.general).key(|req: &Request| {
                (Class::of(req) == Class::General)
                    .then(|| subject(req))
                    .flatten()
            }),
            compare: limiter(limits.compare).key(|req: &Request| {
                (Class::of(req) == Class::Compare)
                    .then(|| subject(req))
                    .flatten()
            }),
        }
    }
}

/// The answering layer's state.
#[derive(Clone)]
struct Gate {
    /// For the audit row.
    db: PgPool,
}

/// Count every authenticated request on `router` and refuse one past its limit. Add it after
/// authentication, so that it runs after it and the subject is the verified token's.
pub fn layer(router: Router, limiters: Limiters, db: PgPool) -> Router {
    router
        .layer(from_fn_with_state(limiters.general, by_key::<String>))
        .layer(from_fn_with_state(limiters.compare, by_key::<String>))
        .layer(from_fn_with_state(Gate { db }, answer_mw))
}

/// Outside the limiters: count an overflow, and for a refusal count it, audit the first in the window,
/// and answer in the route's own shape (an `OperationOutcome` under `/fhir/`) with the limiter's headers.
async fn answer_mw(State(gate): State<Gate>, req: Request, next: Next) -> Response {
    let class = Class::of(&req);
    let method = req.method().clone();
    let path = req.uri().path().to_owned();
    let agent = subject(&req);
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
    crate::metrics::RATE_LIMITED.inc();
    if verdict.first_refusal
        && let Some(agent) = agent
    {
        let mut event = Event::new(Action::Execute, agent);
        event.outcome = 4;
        event.resource_ref = Some("rate-limit".to_string());
        event.detail = Some(serde_json::json!({
            "denied": "rate-limit",
            "class": class.name(),
            "method": method.as_str(),
        }));
        if let Err(e) = audit::append(&gate.db, audit::chain(), &event).await {
            crate::metrics::UNAUDITED_REFUSALS.inc();
            tracing::error!(error = %e, "a rate-limit refusal could not be audited");
        }
    }
    let mut answer = ApiError::new(
        StatusCode::TOO_MANY_REQUESTS,
        "too many requests; slow down",
    )
    .with("retry_after_seconds", verdict.reset_seconds)
    .for_path(&path);
    verdict.stamp(&mut answer);
    if let Some(wait) = res.headers().get(header::RETRY_AFTER) {
        answer
            .headers_mut()
            .insert(header::RETRY_AFTER, wait.clone());
    }
    answer
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nz(n: u32) -> NonZeroU32 {
        NonZeroU32::new(n).unwrap()
    }

    #[test]
    fn a_clinician_at_the_default_limits_never_trips() {
        let l = Limiters::new(Limits::default());
        // One window of five requests a second and a compare every two seconds. The window is fixed, so
        // every minute of a longer shift is counted the same way.
        for _ in 0..300 {
            assert!(l.general.limiter().check("dr".to_string()).allowed);
        }
        for _ in 0..30 {
            assert!(l.compare.limiter().check("dr".to_string()).allowed);
        }
        // The default itself is the edge.
        let fresh = Limiters::new(Limits::default());
        for _ in 0..DEFAULT_GENERAL.get() {
            assert!(fresh.general.limiter().check("dr".to_string()).allowed);
        }
        let past = fresh.general.limiter().check("dr".to_string());
        assert!(!past.allowed);
        assert!(past.first_refusal);
        assert!(
            !fresh
                .general
                .limiter()
                .check("dr".to_string())
                .first_refusal
        );
    }

    #[test]
    fn a_full_table_lets_a_new_subject_through_and_says_so() {
        let l = Limiters::new(Limits {
            general: nz(1),
            compare: nz(1),
        });
        let general = l.general.limiter();
        for i in 0..axa::ratelimit::MAX_KEYS {
            general.check(format!("s{i}"));
        }
        for _ in 0..3 {
            let v = general.check("new".to_string());
            assert!(v.allowed, "a new subject is let through, not refused");
            assert!(v.overflowed);
        }
        // A subject already held is still counted.
        assert!(!general.check("s0".to_string()).allowed);
    }

    #[test]
    fn a_setting_is_a_whole_number_in_range_or_the_service_does_not_start() {
        assert_eq!(parse_limit("X", None, nz(7)), Ok(nz(7)));
        assert_eq!(parse_limit("X", Some(" 50 "), nz(7)), Ok(nz(50)));
        assert_eq!(parse_limit("X", Some("1000000"), nz(7)), Ok(nz(1_000_000)));
        for bad in ["0", "1000001", "-1", "many", "", "1.5"] {
            assert!(parse_limit("X", Some(bad), nz(7)).is_err(), "{bad:?}");
        }
    }
}
