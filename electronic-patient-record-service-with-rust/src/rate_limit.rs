//! Request limits per token subject (task OS-2, first slice; `spec/07` NFR-6).
//!
//! A fixed one-minute window per `(subject, class)` held in the process. The classes are `general`
//! and `compare` (`POST /api/patients/compare`). A request past its limit is answered 429 with
//! `Retry-After`; the first refusal of a subject in a window is an audit row and every refusal is
//! counted. A limit is a refusal a clinician could meet, so the defaults sit far above clinical use
//! and the limiter fails open: if its table is full of live entries a new subject is let through.
//! Requests without a token subject (login off, public paths) are not limited here.

use crate::api_error::ApiError;
use crate::audit::{self, Action, Event};
use crate::auth::{ANONYMOUS, Agent};
use axum::{
    extract::{Request, State},
    http::{HeaderValue, header},
    middleware::Next,
    response::Response,
};
use sea_orm::DatabaseConnection;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Seconds in a window.
pub const WINDOW_SECS: u64 = 60;
/// Default requests a subject may make per window, all routes.
pub const DEFAULT_GENERAL: u32 = 1200;
/// Default compare calls a subject may make per window.
pub const DEFAULT_COMPARE: u32 = 120;
/// The most `(subject, class)` entries kept. Past this, expired entries are dropped, and if none expired a
/// new subject is not limited.
pub const MAX_ENTRIES: usize = 100_000;
/// The path of the compare call.
const COMPARE_PATH: &str = "/api/patients/compare";

/// What a request is counted against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Class {
    /// Every route.
    General,
    /// `POST /api/patients/compare`.
    Compare,
}

impl Class {
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
    /// All routes.
    pub general: u32,
    /// The compare call.
    pub compare: u32,
}

impl Default for Limits {
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
pub fn parse_limit(name: &str, raw: Option<&str>, default: u32) -> Result<u32, String> {
    let Some(raw) = raw else {
        return Ok(default);
    };
    match raw.trim().parse::<u32>() {
        Ok(n) if (1..=1_000_000).contains(&n) => Ok(n),
        _ => Err(format!(
            "{name} must be a whole number from 1 to 1000000, not {raw:?}"
        )),
    }
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

    fn of(self, class: Class) -> u32 {
        match class {
            Class::General => self.general,
            Class::Compare => self.compare,
        }
    }
}

/// What the limiter says about one request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Within the limit.
    Allow,
    /// Past it.
    Refuse {
        /// Seconds until the window ends.
        retry_after: u64,
        /// The first refusal in this window, the one that is audited.
        first: bool,
    },
}

struct Slot {
    start: u64,
    count: u32,
    refused: bool,
}

/// The counters. Time is passed in, so tests need no clock.
pub struct Limiter {
    limits: Limits,
    slots: Mutex<HashMap<(String, Class), Slot>>,
}

impl Limiter {
    /// A limiter with `limits`.
    #[must_use]
    pub fn new(limits: Limits) -> Self {
        Limiter {
            limits,
            slots: Mutex::new(HashMap::new()),
        }
    }

    /// Count one request by `subject` in `class` at `now` (unix seconds).
    pub fn check(&self, subject: &str, class: Class, now: u64) -> Verdict {
        let mut slots = self.slots.lock().unwrap_or_else(|e| e.into_inner());
        let key = (subject.to_string(), class);
        if !slots.contains_key(&key) && slots.len() >= MAX_ENTRIES {
            slots.retain(|_, s| s.start + WINDOW_SECS > now);
            if slots.len() >= MAX_ENTRIES {
                return Verdict::Allow;
            }
        }
        let slot = slots.entry(key).or_insert(Slot {
            start: now,
            count: 0,
            refused: false,
        });
        if slot.start + WINDOW_SECS <= now {
            *slot = Slot {
                start: now,
                count: 0,
                refused: false,
            };
        }
        slot.count = slot.count.saturating_add(1);
        if slot.count <= self.limits.of(class) {
            return Verdict::Allow;
        }
        let first = !slot.refused;
        slot.refused = true;
        Verdict::Refuse {
            retry_after: (slot.start + WINDOW_SECS - now).max(1),
            first,
        }
    }
}

/// The middleware's state.
#[derive(Clone)]
pub struct Gate {
    /// The counters.
    pub limiter: Arc<Limiter>,
    /// For the audit row.
    pub db: DatabaseConnection,
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Count every authenticated request and refuse one past its limit. Mounted after authentication, so
/// the subject is the verified token's.
pub async fn rate_limit_mw(State(gate): State<Gate>, req: Request, next: Next) -> Response {
    let Some(agent) = req.extensions().get::<Agent>().map(|a| a.0.clone()) else {
        return next.run(req).await;
    };
    if agent == ANONYMOUS {
        return next.run(req).await;
    }
    let class = if req.method() == axum::http::Method::POST && req.uri().path() == COMPARE_PATH {
        Class::Compare
    } else {
        Class::General
    };
    match gate.limiter.check(&agent, class, now()) {
        Verdict::Allow => next.run(req).await,
        Verdict::Refuse { retry_after, first } => {
            crate::metrics::RATE_LIMITED.inc();
            if first {
                let mut event = Event::new(Action::Execute, agent);
                event.outcome = 4;
                event.resource_ref = Some("rate-limit".to_string());
                event.detail = Some(serde_json::json!({
                    "denied": "rate-limit",
                    "class": class.name(),
                    "method": req.method().as_str(),
                }));
                if let Err(e) = audit::append(&gate.db, audit::chain(), &event).await {
                    crate::metrics::UNAUDITED_REFUSALS.inc();
                    tracing::error!(error = %e, "a rate-limit refusal could not be audited");
                }
            }
            let mut res = ApiError::new(
                axum::http::StatusCode::TOO_MANY_REQUESTS,
                "too many requests; slow down",
            )
            .with("retry_after_seconds", retry_after)
            .for_path(req.uri().path());
            if let Ok(v) = HeaderValue::from_str(&retry_after.to_string()) {
                res.headers_mut().insert(header::RETRY_AFTER, v);
            }
            res
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limiter(general: u32, compare: u32) -> Limiter {
        Limiter::new(Limits { general, compare })
    }

    #[test]
    fn the_limit_trips_and_the_first_refusal_is_flagged_once() {
        let l = limiter(3, 3);
        for _ in 0..3 {
            assert_eq!(l.check("a", Class::General, 100), Verdict::Allow);
        }
        assert_eq!(
            l.check("a", Class::General, 110),
            Verdict::Refuse {
                retry_after: 50,
                first: true
            }
        );
        assert_eq!(
            l.check("a", Class::General, 111),
            Verdict::Refuse {
                retry_after: 49,
                first: false
            }
        );
    }

    #[test]
    fn a_new_window_starts_clean_and_subjects_and_classes_are_separate() {
        let l = limiter(1, 1);
        assert_eq!(l.check("a", Class::General, 0), Verdict::Allow);
        assert!(matches!(
            l.check("a", Class::General, 1),
            Verdict::Refuse { .. }
        ));
        assert_eq!(l.check("b", Class::General, 1), Verdict::Allow);
        assert_eq!(l.check("a", Class::Compare, 1), Verdict::Allow);
        assert_eq!(l.check("a", Class::General, WINDOW_SECS), Verdict::Allow);
        // The refusal flag is per window too.
        assert!(matches!(
            l.check("a", Class::General, WINDOW_SECS + 1),
            Verdict::Refuse { first: true, .. }
        ));
    }

    #[test]
    fn retry_after_is_never_zero() {
        let l = limiter(1, 1);
        l.check("a", Class::General, 0);
        assert_eq!(
            l.check("a", Class::General, WINDOW_SECS - 1),
            Verdict::Refuse {
                retry_after: 1,
                first: true
            }
        );
    }

    #[test]
    fn a_clinician_at_the_default_limits_never_trips() {
        let l = Limiter::new(Limits::default());
        // Five requests a second for ten minutes, a compare every two seconds.
        for t in 0..600u64 {
            for _ in 0..5 {
                assert_eq!(l.check("dr", Class::General, t), Verdict::Allow, "t={t}");
            }
            if t % 2 == 0 {
                assert_eq!(l.check("dr", Class::Compare, t), Verdict::Allow, "t={t}");
            }
        }
    }

    #[test]
    fn a_full_table_lets_a_new_subject_through() {
        let l = limiter(1, 1);
        {
            let mut slots = l.slots.lock().unwrap();
            for i in 0..MAX_ENTRIES {
                slots.insert(
                    (format!("s{i}"), Class::General),
                    Slot {
                        start: 1000,
                        count: 1,
                        refused: false,
                    },
                );
            }
        }
        // Nothing has expired, so the new subject is not counted.
        assert_eq!(l.check("new", Class::General, 1001), Verdict::Allow);
        assert_eq!(l.check("new", Class::General, 1001), Verdict::Allow);
        // Once the entries have expired the table is cleared and counting resumes.
        assert_eq!(l.check("new", Class::General, 5000), Verdict::Allow);
        assert!(matches!(
            l.check("new", Class::General, 5000),
            Verdict::Refuse { .. }
        ));
    }

    #[test]
    fn a_setting_is_a_whole_number_in_range_or_the_service_does_not_start() {
        assert_eq!(parse_limit("X", None, 7), Ok(7));
        assert_eq!(parse_limit("X", Some(" 50 "), 7), Ok(50));
        for bad in ["0", "1000001", "-1", "many", "", "1.5"] {
            assert!(parse_limit("X", Some(bad), 7).is_err(), "{bad:?}");
        }
    }
}
