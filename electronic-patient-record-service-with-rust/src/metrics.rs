//! Process counters for the things an operator must see (task CQ-17), served at `GET /metrics` in the
//! Prometheus text format. They are plain atomics: no dependency, no labels, and no patient data.
//!
//! The counters start at zero on each start and are not stored. A counter that stays at zero says the
//! event has not happened since the process started, not that it never did.
//!
//! `/metrics` answers 404 unless `EPR_METRICS` is `1` or `true`. When on it needs no token (a scraper
//! seldom holds one), so a deployer exposes it on an internal network only.

use axum::{
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use std::sync::atomic::{AtomicU64, Ordering};

/// A monotonically increasing count.
#[derive(Debug, Default)]
pub struct Counter(AtomicU64);

impl Counter {
    /// A counter at zero.
    #[must_use]
    pub const fn new() -> Self {
        Self(AtomicU64::new(0))
    }

    /// Add one.
    pub fn inc(&self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }

    /// The current count.
    #[must_use]
    pub fn get(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

/// Audit appends that failed. Every one failed the request it belonged to (an access that cannot be
/// audited does not happen), so a rising count is an outage of the audit trail.
pub static AUDIT_APPEND_FAILURES: Counter = Counter::new();

/// Refusals that left no audit row: a 401 or 503 from the authentication middleware (nothing is
/// written before a caller is known), and a 403 whose audit row could not be written.
pub static UNAUDITED_REFUSALS: Counter = Counter::new();

/// HL7 messages refused, whether or not the refusal was stored.
pub static FEED_REFUSALS: Counter = Counter::new();

/// `(name, help, counter)` for everything served.
fn all() -> [(&'static str, &'static str, &'static Counter); 3] {
    [
        (
            "epr_audit_append_failures_total",
            "Audit appends that failed.",
            &AUDIT_APPEND_FAILURES,
        ),
        (
            "epr_unaudited_refusals_total",
            "Refusals that left no audit row.",
            &UNAUDITED_REFUSALS,
        ),
        (
            "epr_feed_refused_messages_total",
            "HL7 messages refused.",
            &FEED_REFUSALS,
        ),
    ]
}

/// The counters in the Prometheus text exposition format.
#[must_use]
pub fn render() -> String {
    render_of(&all())
}

fn render_of(counters: &[(&str, &str, &Counter)]) -> String {
    let mut out = String::new();
    for (name, help, counter) in counters {
        out.push_str(&format!(
            "# HELP {name} {help}\n# TYPE {name} counter\n{name} {}\n",
            counter.get()
        ));
    }
    out
}

/// True when `EPR_METRICS` is `1` or `true`.
#[must_use]
pub fn enabled(value: Option<&str>) -> bool {
    matches!(
        value.map(|v| v.trim().to_ascii_lowercase()).as_deref(),
        Some("1" | "true")
    )
}

/// The `/metrics` answer: the counters, or 404 when metrics are off.
#[must_use]
pub fn response(enabled: bool) -> Response {
    if !enabled {
        return StatusCode::NOT_FOUND.into_response();
    }
    (
        [(
            header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        render(),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_text_format_has_help_type_and_value() {
        let c = Counter::new();
        c.inc();
        c.inc();
        assert_eq!(
            render_of(&[("x_total", "An x.", &c)]),
            "# HELP x_total An x.\n# TYPE x_total counter\nx_total 2\n"
        );
    }

    #[test]
    fn only_one_and_true_turn_metrics_on() {
        for on in ["1", "true", " TRUE "] {
            assert!(enabled(Some(on)), "{on}");
        }
        for off in [None, Some(""), Some("0"), Some("yes"), Some("false")] {
            assert!(!enabled(off), "{off:?}");
        }
    }

    #[test]
    fn every_served_counter_has_a_distinct_total_name() {
        let names: Vec<_> = all().iter().map(|(n, _, _)| *n).collect();
        assert!(
            names
                .iter()
                .all(|n| n.starts_with("epr_") && n.ends_with("_total"))
        );
        let mut sorted = names.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), names.len());
    }
}
