//! Process counters for the things an operator must see (task CQ-17), served at `GET /metrics` in the
//! Prometheus text format. They are plain atomics: no dependency, no labels, and no patient data.
//!
//! The counters start at zero on each start and are not stored. A counter that stays at zero says the
//! event has not happened since the process started, not that it never did.
//!
//! `/metrics` answers 404 unless `EPR_METRICS` is `1` or `true`. When on it needs no token (a scraper
//! seldom holds one), so a deployer exposes it on an internal network only.

use axum::{
    extract::{MatchedPath, Request},
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::{
    collections::BTreeMap,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};

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

/// A value that can go down as well as up. Set by the loop that knows it; zero until the first set.
#[derive(Debug, Default)]
pub struct Gauge(AtomicU64);

impl Gauge {
    /// A gauge at zero.
    #[must_use]
    pub const fn new() -> Self {
        Self(AtomicU64::new(0))
    }

    /// Replace the value.
    pub fn set(&self, value: u64) {
        self.0.store(value, Ordering::Relaxed);
    }

    /// The current value.
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

/// Outbox events delivered to their sink (OS-3).
pub static OUTBOX_DELIVERED: Counter = Counter::new();

/// Outbox events refused by the policy or the patient's consent. Each is also an audit row.
pub static OUTBOX_REFUSED: Counter = Counter::new();

/// Outbox delivery attempts that failed. A count that keeps rising while `delivered` does not is a
/// sink that is down or an event that cannot be sent.
pub static OUTBOX_FAILED: Counter = Counter::new();

/// Outbox events still to be tried, as of the last run of the loop (OS-3). Zero when the loop is off.
pub static OUTBOX_PENDING: Gauge = Gauge::new();

/// Outbox events that failed `MAX_FAILURES` times and are left: nothing retries them, so a value above
/// zero needs a person.
pub static OUTBOX_STUCK: Gauge = Gauge::new();

/// Requests refused because their subject was past a request limit (OS-2).
pub static RATE_LIMITED: Counter = Counter::new();

/// Upper bounds, in seconds, of the request-duration buckets.
pub const BUCKETS: [f64; 11] = [
    0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
];

/// Most distinct `(method, route, status)` series kept. Routes are templates, so the real count is
/// small; the cap only guards against a bug that puts an unbounded value in a label. Past it,
/// observations fold into the route `overflow`.
const MAX_SERIES: usize = 1000;

/// The route label for a request that matched no route.
const UNMATCHED: &str = "unmatched";

#[derive(Debug, Clone, Default)]
struct Histogram {
    /// Observations at or under each bound, not yet cumulative.
    buckets: [u64; BUCKETS.len()],
    count: u64,
    sum: f64,
}

type Series = BTreeMap<(String, String, u16), Histogram>;

static REQUESTS: Mutex<Series> = Mutex::new(BTreeMap::new());

fn observe_in(series: &mut Series, method: &str, route: &str, status: u16, seconds: f64) {
    let key = (method.to_string(), route.to_string(), status);
    let key = if series.contains_key(&key) || series.len() < MAX_SERIES {
        key
    } else {
        (method.to_string(), "overflow".to_string(), status)
    };
    let h = series.entry(key).or_default();
    h.count += 1;
    h.sum += seconds;
    if let Some(i) = BUCKETS.iter().position(|b| seconds <= *b) {
        h.buckets[i] += 1;
    }
}

/// Record one request. `route` is the matched route template, never the raw path.
pub fn observe(method: &str, route: &str, status: u16, seconds: f64) {
    let mut guard = REQUESTS.lock().unwrap_or_else(|e| e.into_inner());
    observe_in(&mut guard, method, route, status, seconds);
}

/// Middleware that times every request and labels it by method, matched route template and status.
/// It carries no patient identifier: a request that matched no route is labelled `unmatched`.
pub async fn duration_mw(request: Request, next: Next) -> Response {
    let started = Instant::now();
    let method = request.method().as_str().to_string();
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map_or_else(|| UNMATCHED.to_string(), |m| m.as_str().to_string());
    let response = next.run(request).await;
    observe(
        &method,
        &route,
        response.status().as_u16(),
        started.elapsed().as_secs_f64(),
    );
    response
}

fn escape(label: &str) -> String {
    label
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

fn render_histograms(series: &Series) -> String {
    if series.is_empty() {
        return String::new();
    }
    let name = "epr_http_request_duration_seconds";
    let mut out = format!(
        "# HELP {name} Request duration by method, matched route template and status.\n# TYPE {name} histogram\n"
    );
    for ((method, route, status), h) in series {
        let labels = format!(
            "method=\"{}\",route=\"{}\",status=\"{status}\"",
            escape(method),
            escape(route)
        );
        let mut cumulative = 0;
        for (bound, n) in BUCKETS.iter().zip(h.buckets) {
            cumulative += n;
            out.push_str(&format!(
                "{name}_bucket{{{labels},le=\"{bound}\"}} {cumulative}\n"
            ));
        }
        out.push_str(&format!(
            "{name}_bucket{{{labels},le=\"+Inf\"}} {}\n{name}_sum{{{labels}}} {}\n{name}_count{{{labels}}} {}\n",
            h.count, h.sum, h.count
        ));
    }
    out
}

/// `(name, help, counter)` for everything served.
fn all() -> [(&'static str, &'static str, &'static Counter); 7] {
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
        (
            "epr_rate_limited_total",
            "Requests refused for being past a request limit.",
            &RATE_LIMITED,
        ),
        (
            "epr_outbox_delivered_total",
            "Outbox events delivered.",
            &OUTBOX_DELIVERED,
        ),
        (
            "epr_outbox_refused_total",
            "Outbox events refused by policy or consent.",
            &OUTBOX_REFUSED,
        ),
        (
            "epr_outbox_failed_total",
            "Outbox delivery attempts that failed.",
            &OUTBOX_FAILED,
        ),
    ]
}

/// `(name, help, gauge)` for everything served.
fn gauges() -> [(&'static str, &'static str, &'static Gauge); 2] {
    [
        (
            "epr_outbox_pending",
            "Outbox events still to be tried, as of the last loop run.",
            &OUTBOX_PENDING,
        ),
        (
            "epr_outbox_stuck",
            "Outbox events that failed too often and are left.",
            &OUTBOX_STUCK,
        ),
    ]
}

/// The counters in the Prometheus text exposition format.
#[must_use]
pub fn render() -> String {
    let mut out = render_of(&all());
    for (name, help, gauge) in gauges() {
        out.push_str(&format!(
            "# HELP {name} {help}\n# TYPE {name} gauge\n{name} {}\n",
            gauge.get()
        ));
    }
    let series = REQUESTS.lock().unwrap_or_else(|e| e.into_inner());
    out.push_str(&render_histograms(&series));
    out
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
    fn a_histogram_is_cumulative_and_ends_at_the_count() {
        let mut series = Series::new();
        observe_in(&mut series, "GET", "/x/{id}", 200, 0.004);
        observe_in(&mut series, "GET", "/x/{id}", 200, 0.2);
        observe_in(&mut series, "GET", "/x/{id}", 200, 30.0);
        let text = render_histograms(&series);
        let l = "method=\"GET\",route=\"/x/{id}\",status=\"200\"";
        assert!(
            text.contains(&format!("_bucket{{{l},le=\"0.005\"}} 1\n")),
            "{text}"
        );
        assert!(
            text.contains(&format!("_bucket{{{l},le=\"0.25\"}} 2\n")),
            "{text}"
        );
        assert!(
            text.contains(&format!("_bucket{{{l},le=\"10\"}} 2\n")),
            "{text}"
        );
        assert!(
            text.contains(&format!("_bucket{{{l},le=\"+Inf\"}} 3\n")),
            "{text}"
        );
        assert!(text.contains(&format!("_count{{{l}}} 3\n")), "{text}");
    }

    #[test]
    fn series_are_capped_and_overflow_is_labelled() {
        let mut series = Series::new();
        for i in 0..MAX_SERIES + 50 {
            observe_in(&mut series, "GET", &format!("/r{i}"), 200, 0.001);
        }
        assert_eq!(
            series.len(),
            MAX_SERIES + 1,
            "one extra series for overflow"
        );
        assert_eq!(series[&("GET".into(), "overflow".into(), 200)].count, 50);
    }

    #[test]
    fn label_values_are_escaped() {
        assert_eq!(escape("a\"b\\c\nd"), "a\\\"b\\\\c\\nd");
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
