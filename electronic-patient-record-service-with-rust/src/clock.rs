//! The time the service acts on (task PR-5). Code that decides by the clock takes a [`Clock`] instead of
//! asking the system, so a test moves time instead of rewriting rows. [`SystemClock`] is the real one;
//! [`FixedClock`] stands still until told to move.
//!
//! First slice: idempotency expiry takes the time from here. Break-glass expiry and the referral
//! timeouts still ask the database's `now()` in SQL and are not yet on this clock.

use std::sync::Mutex;

use chrono::{DateTime, Duration, Utc};

/// A source of the current time.
pub trait Clock: Send + Sync {
    /// The time now.
    fn now(&self) -> DateTime<Utc>;
}

/// The real time.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// A clock that shows one time until it is moved. For tests.
#[derive(Debug)]
pub struct FixedClock(Mutex<DateTime<Utc>>);

impl FixedClock {
    /// A clock showing `at`.
    #[must_use]
    pub fn new(at: DateTime<Utc>) -> Self {
        FixedClock(Mutex::new(at))
    }

    /// Move the clock forward (or back, with a negative duration).
    pub fn advance(&self, by: Duration) {
        let mut at = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *at += by;
    }
}

impl Clock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        *self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fixed_clock_stands_still_until_moved() {
        let start = "2026-11-04T09:00:00Z".parse().unwrap();
        let c = FixedClock::new(start);
        assert_eq!(c.now(), start);
        assert_eq!(c.now(), start);
        c.advance(Duration::hours(25));
        assert_eq!(c.now(), start + Duration::hours(25));
        c.advance(Duration::hours(-25));
        assert_eq!(c.now(), start);
    }
}
