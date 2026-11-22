//! Process counters for the things an operator must see (task CQ-17), served at `GET /metrics` in the
//! Prometheus text format. They are plain atomics: no dependency, no labels, and no patient data.
//!
//! The counters start at zero on each start and are not stored. A counter that stays at zero says the
//! event has not happened since the process started, not that it never did.
//!
//! `/metrics` answers 404 unless `EPR_METRICS` is `1` or `true`. When on it needs no token (a scraper
//! seldom holds one), so a deployer exposes it on an internal network only.

// RCA-ENF-3: this module is converted, so the arithmetic lints are denied here; the rest of the crate follows.
#![deny(
    clippy::arithmetic_side_effects,
    clippy::default_numeric_fallback,
    clippy::integer_division,
    clippy::modulo_arithmetic,
    clippy::as_conversions,
    clippy::string_slice
)]

/// A count that no longer fits `u64`. It is an error, never a held or wrapped value: a Prometheus counter that
/// wrapped would read as a reset, and one held at the top would read as a count that stopped. The figure is named
/// and no value is shown (RCA-P-4, RCA-ERR-2). Not reachable in practice: it takes `2^64` events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Overflow {
    /// The metric whose count no longer fits.
    pub figure: &'static str,
}

impl std::fmt::Display for Overflow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: the count no longer fits", self.figure)
    }
}

/// The first metric that overflowed in this process. From then on `/metrics` answers 500 naming it, until the process
/// restarts: the exposed counts would otherwise be wrong without saying so (RCA-NONE-4).
static OVERFLOWED: OnceLock<&'static str> = OnceLock::new();

/// Remember an overflow, and say so once in the log.
fn record(overflow: Overflow) {
    if OVERFLOWED.set(overflow.figure).is_ok() {
        tracing::error!(
            figure = overflow.figure,
            "a metric count no longer fits: /metrics now answers 500"
        );
    }
}

/// One more in a count of `figure`.
fn inc(counter: &mut u64, figure: &'static str) -> Result<(), Overflow> {
    *counter = counter.checked_add(1).ok_or(Overflow { figure })?;
    Ok(())
}

use axum::{
    extract::{MatchedPath, Request},
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::{
    collections::BTreeMap,
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Instant,
};

/// A monotonically increasing count.
#[derive(Debug, Default)]
pub struct Counter(AtomicU64, AtomicBool);

impl Counter {
    /// A counter at zero.
    #[must_use]
    pub const fn new() -> Self {
        Self(AtomicU64::new(0), AtomicBool::new(false))
    }

    /// Add one. A count that no longer fits is not changed; the counter is marked, and [`render`] refuses, naming it.
    pub fn inc(&self) {
        let added = self
            .0
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1));
        if added.is_err() {
            self.1.store(true, Ordering::Relaxed);
        }
    }

    /// Whether a count was lost to overflow.
    #[must_use]
    pub fn overflowed(&self) -> bool {
        self.1.load(Ordering::Relaxed)
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

/// Outbox attempts that failed: a sink's refusal, an event out of attempts or changed, or a lost relay.
/// A count that keeps rising while `delivered` does not is a sink that is down or an event that cannot
/// be sent.
pub static OUTBOX_FAILED: Counter = Counter::new();

/// Outbox events still owed (waiting, claimed or held), as last read by the relay's depth watch (OS-3).
/// Zero when the relay is off.
pub static OUTBOX_PENDING: Gauge = Gauge::new();

/// Outbox events left `failed` (out of attempts, or changed): nothing retries them until a person
/// releases one, so a value above zero needs a person.
pub static OUTBOX_STUCK: Gauge = Gauge::new();

/// Requests refused because their subject was past a request limit (OS-2).
pub static RATE_LIMITED: Counter = Counter::new();

/// Requests refused because their client address was locked out after too many `401`s (OS-2, H-049).
pub static LOCKOUTS: Counter = Counter::new();

/// Requests let through uncounted because the limiter's table was full (OS-2, H-039). The limiter fails
/// open so that no clinician is refused for it, so a rising count is either a burst of new subjects or
/// an attack that has made the limit useless, and needs a person.
pub static RATE_LIMIT_OVERFLOWED: Counter = Counter::new();

/// HL7 feed messages by sender, event and answer (IH-9). The sender is the feed's configured agent, never anything the
/// message says; the event is a handled trigger or `other`; the answer is `AA`, `AE`, `AR` or `none` (no
/// acknowledgement, because the frame was not HL7). No control id, patient or text is a label. Past
/// [`MAX_FEED_SERIES`] distinct series, new ones count under the sender `overflow`.
static FEED_MESSAGES: Mutex<BTreeMap<(String, String, String), u64>> = Mutex::new(BTreeMap::new());

/// Most distinct feed series kept: a few configured feeds times a few events and answers.
const MAX_FEED_SERIES: usize = 200;

/// Count one feed message. `event` outside `handled` is counted as `other`, so a sender cannot make new series.
pub fn feed_message(sender: &str, event: &str, handled: &[&str], answer: &str) {
    let mut series = FEED_MESSAGES.lock().unwrap_or_else(|e| e.into_inner());
    if let Err(overflow) = feed_in(&mut series, sender, event, handled, answer) {
        record(overflow);
    }
}

/// [`feed_message`] on a given table.
fn feed_in(
    series: &mut BTreeMap<(String, String, String), u64>,
    sender: &str,
    event: &str,
    handled: &[&str],
    answer: &str,
) -> Result<(), Overflow> {
    let event = if handled.contains(&event) {
        event
    } else {
        "other"
    };
    let answer = if matches!(answer, "AA" | "AE" | "AR") {
        answer
    } else {
        "none"
    };
    let key = (sender.to_string(), event.to_string(), answer.to_string());
    let key = if series.contains_key(&key) || series.len() < MAX_FEED_SERIES {
        key
    } else {
        (
            "overflow".to_string(),
            event.to_string(),
            answer.to_string(),
        )
    };
    inc(series.entry(key).or_default(), FEED_FIGURE)
}

/// The feed counter's name.
const FEED_FIGURE: &str = "epr_feed_messages_total";

/// The request histogram's name.
const HISTOGRAM_FIGURE: &str = "epr_http_request_duration_seconds";

/// Each configured feed's health (IH-8): 0 unknown, 1 healthy, 2 warning, 3 critical, as last judged.
static FEED_HEALTH: Mutex<BTreeMap<String, u64>> = Mutex::new(BTreeMap::new());

/// Set a configured feed's health gauge. `sender` is the configured agent.
pub fn feed_health(sender: &str, value: u64) {
    let mut g = FEED_HEALTH.lock().unwrap_or_else(|e| e.into_inner());
    if g.contains_key(sender) || g.len() < MAX_FEED_SERIES {
        g.insert(sender.to_string(), value);
    }
}

fn render_feed_health() -> String {
    let g = FEED_HEALTH.lock().unwrap_or_else(|e| e.into_inner());
    if g.is_empty() {
        return String::new();
    }
    let name = "epr_feed_health";
    let mut out = format!(
        "# HELP {name} HL7 feed health as last judged: 0 unknown, 1 healthy, 2 warning, 3 critical.\n# TYPE {name} gauge\n"
    );
    for (sender, v) in g.iter() {
        out.push_str(&format!("{name}{{sender=\"{}\"}} {v}\n", escape(sender)));
    }
    out
}

/// The feed counter's lines, one per sender, event and answer.
fn render_feed() -> String {
    let series = FEED_MESSAGES.lock().unwrap_or_else(|e| e.into_inner());
    if series.is_empty() {
        return String::new();
    }
    let name = FEED_FIGURE;
    let mut out = format!(
        "# HELP {name} HL7 feed messages by configured sender, event and answer.\n# TYPE {name} counter\n"
    );
    for ((sender, event, answer), n) in series.iter() {
        out.push_str(&format!(
            "{name}{{sender=\"{}\",event=\"{}\",answer=\"{}\"}} {n}\n",
            escape(sender),
            escape(event),
            escape(answer)
        ));
    }
    out
}

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

/// Record one request in `series`, keyed by method, route template and status. Once there are `MAX_SERIES` series, a new
/// key is counted under the route `overflow` instead, so a flood of distinct routes cannot grow the table without limit.
fn observe_in(
    series: &mut Series,
    method: &str,
    route: &str,
    status: u16,
    seconds: f64,
) -> Result<(), Overflow> {
    let key = (method.to_string(), route.to_string(), status);
    let key = if series.contains_key(&key) || series.len() < MAX_SERIES {
        key
    } else {
        (method.to_string(), "overflow".to_string(), status)
    };
    let h = series.entry(key).or_default();
    inc(&mut h.count, HISTOGRAM_FIGURE)?;
    h.sum += seconds;
    if let Some(slot) = BUCKETS
        .iter()
        .position(|b| seconds <= *b)
        .and_then(|i| h.buckets.get_mut(i))
    {
        inc(slot, HISTOGRAM_FIGURE)?;
    }
    Ok(())
}

/// Record one request. `route` is the matched route template, never the raw path.
pub fn observe(method: &str, route: &str, status: u16, seconds: f64) {
    let mut guard = REQUESTS.lock().unwrap_or_else(|e| e.into_inner());
    if let Err(overflow) = observe_in(&mut guard, method, route, status, seconds) {
        record(overflow);
    }
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

/// A Prometheus label value with backslash, double quote and newline escaped.
fn escape(label: &str) -> String {
    label
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

/// The request-duration histogram, one set of lines per method, route template and status.
fn render_histograms(series: &Series) -> Result<String, Overflow> {
    if series.is_empty() {
        return Ok(String::new());
    }
    let name = HISTOGRAM_FIGURE;
    let mut out = format!(
        "# HELP {name} Request duration by method, matched route template and status.\n# TYPE {name} histogram\n"
    );
    for ((method, route, status), h) in series {
        let labels = format!(
            "method=\"{}\",route=\"{}\",status=\"{status}\"",
            escape(method),
            escape(route)
        );
        let mut cumulative = 0_u64;
        for (bound, n) in BUCKETS.iter().zip(h.buckets) {
            cumulative = cumulative.checked_add(n).ok_or(Overflow { figure: name })?;
            out.push_str(&format!(
                "{name}_bucket{{{labels},le=\"{bound}\"}} {cumulative}\n"
            ));
        }
        out.push_str(&format!(
            "{name}_bucket{{{labels},le=\"+Inf\"}} {}\n{name}_sum{{{labels}}} {}\n{name}_count{{{labels}}} {}\n",
            h.count, h.sum, h.count
        ));
    }
    Ok(out)
}

/// `(name, help, counter)` for everything served.
fn all() -> [(&'static str, &'static str, &'static Counter); 9] {
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
            "epr_lockouts_total",
            "Requests refused because their client address was locked out.",
            &LOCKOUTS,
        ),
        (
            "epr_rate_limit_overflowed_total",
            "Requests let through uncounted because the limiter's table was full.",
            &RATE_LIMIT_OVERFLOWED,
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
///
/// # Errors
///
/// [`Overflow`] naming the first metric whose count no longer fits: nothing is shown, as the text would be wrong.
pub fn render() -> Result<String, Overflow> {
    if let Some(figure) = OVERFLOWED.get() {
        return Err(Overflow { figure });
    }
    let mut out = render_of(&all())?;
    for (name, help, gauge) in gauges() {
        out.push_str(&format!(
            "# HELP {name} {help}\n# TYPE {name} gauge\n{name} {}\n",
            gauge.get()
        ));
    }
    out.push_str(&render_feed());
    out.push_str(&render_feed_health());
    let series = REQUESTS.lock().unwrap_or_else(|e| e.into_inner());
    out.push_str(&render_histograms(&series)?);
    Ok(out)
}

/// Counters in the text format, each with its `HELP` and `TYPE` lines.
fn render_of(counters: &[(&'static str, &str, &Counter)]) -> Result<String, Overflow> {
    let mut out = String::new();
    for (name, help, counter) in counters {
        if counter.overflowed() {
            return Err(Overflow { figure: name });
        }
        out.push_str(&format!(
            "# HELP {name} {help}\n# TYPE {name} counter\n{name} {}\n",
            counter.get()
        ));
    }
    Ok(out)
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
    match render() {
        Ok(text) => (
            [(
                header::CONTENT_TYPE,
                "text/plain; version=0.0.4; charset=utf-8",
            )],
            text,
        )
            .into_response(),
        // RCA-ERR-4: the answer says a count could not be computed and shows no number.
        Err(overflow) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("metrics: {overflow}\n"),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_count_that_no_longer_fits_is_an_error_naming_the_figure_never_held_or_wrapped() {
        let mut n = 0_u64;
        inc(&mut n, "x").unwrap();
        assert_eq!(n, 1);
        let mut full = u64::MAX;
        assert_eq!(inc(&mut full, "x"), Err(Overflow { figure: "x" }));
        assert_eq!(full, u64::MAX, "not changed by the failed update");

        // The atomic counter marks itself, keeps its value, and the text refuses to show it.
        let c = Counter::new();
        c.0.store(u64::MAX, Ordering::Relaxed);
        assert!(!c.overflowed());
        c.inc();
        assert!(c.overflowed());
        assert_eq!(c.get(), u64::MAX);
        assert_eq!(
            render_of(&[("x_total", "An x.", &c)]),
            Err(Overflow { figure: "x_total" })
        );
        assert_eq!(
            Overflow { figure: "x_total" }.to_string(),
            "x_total: the count no longer fits"
        );
    }

    #[test]
    fn a_histogram_or_feed_count_at_the_top_is_an_error_not_a_new_value() {
        let mut series = Series::new();
        observe_in(&mut series, "GET", "/x", 200, 0.004).unwrap();
        for h in series.values_mut() {
            h.count = u64::MAX;
        }
        assert_eq!(
            observe_in(&mut series, "GET", "/x", 200, 0.004),
            Err(Overflow {
                figure: HISTOGRAM_FIGURE
            })
        );
        let mut feed = BTreeMap::new();
        feed_in(&mut feed, "s", "A08", &["A08"], "AA").unwrap();
        for n in feed.values_mut() {
            *n = u64::MAX;
        }
        assert_eq!(
            feed_in(&mut feed, "s", "A08", &["A08"], "AA"),
            Err(Overflow {
                figure: FEED_FIGURE
            })
        );
    }

    use super::*;

    #[test]
    fn feed_messages_are_counted_by_sender_event_and_answer_with_no_value_from_the_message() {
        let handled = ["A08", "A28"];
        feed_message("mllp:ih9-test", "A08", &handled, "AA");
        feed_message("mllp:ih9-test", "A08", &handled, "AA");
        feed_message("mllp:ih9-test", "ZQ9-9434765919", &handled, "AR");
        feed_message("mllp:ih9-test", "A28", &handled, "something");
        let text = render().unwrap();
        assert!(
            text.contains("# TYPE epr_feed_messages_total counter"),
            "{text}"
        );
        assert!(
            text.contains(
                r#"epr_feed_messages_total{sender="mllp:ih9-test",event="A08",answer="AA"} 2"#
            ),
            "{text}"
        );
        assert!(
            text.contains(
                r#"epr_feed_messages_total{sender="mllp:ih9-test",event="other",answer="AR"} 1"#
            ),
            "{text}"
        );
        assert!(
            text.contains(
                r#"epr_feed_messages_total{sender="mllp:ih9-test",event="A28",answer="none"} 1"#
            ),
            "{text}"
        );
        assert!(!text.contains("9434765919"), "{text}");
        // The older, unlabelled count keeps its name: alerts may depend on it.
        assert!(text.contains("epr_feed_refused_messages_total"), "{text}");
    }

    #[test]
    fn the_text_format_has_help_type_and_value() {
        let c = Counter::new();
        c.inc();
        c.inc();
        assert_eq!(
            render_of(&[("x_total", "An x.", &c)]).unwrap(),
            "# HELP x_total An x.\n# TYPE x_total counter\nx_total 2\n"
        );
    }

    #[test]
    fn a_histogram_is_cumulative_and_ends_at_the_count() {
        let mut series = Series::new();
        observe_in(&mut series, "GET", "/x/{id}", 200, 0.004).unwrap();
        observe_in(&mut series, "GET", "/x/{id}", 200, 0.2).unwrap();
        observe_in(&mut series, "GET", "/x/{id}", 200, 30.0).unwrap();
        let text = render_histograms(&series).unwrap();
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
            observe_in(&mut series, "GET", &format!("/r{i}"), 200, 0.001).unwrap();
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
