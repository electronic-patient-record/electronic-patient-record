//! Chaos testing (idea taken from the patient-administration-system project): a seeded monkey hits
//! every route the service serves with hostile input. It reads `controllers::all_routes()`, so a
//! new route is covered the day it is added. For each request it picks a method the route accepts,
//! fills the path parameters with plausible and hostile values (random ids, `../`, NUL, long
//! strings, unicode), adds a junk query string and sends a generated body (none, malformed JSON,
//! or random JSON with the field names the handlers read). Whatever it sends, with login off:
//!
//! - no panic (a panic aborts the request task and the harness reports it);
//! - no 500 (a 5xx other than 502/503/504 is a bug);
//! - no stack trace, SQL text or file path in a response body.
//!
//! Seeded: a failure prints `EPR_CHAOS_SEED` and the request, and the same seed repeats it.
//! `EPR_CHAOS_ITERATIONS` (default 2000) sets the length. It writes whatever the API lets it
//! write, so it refuses any database whose name does not contain "test" unless `CI` is set.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test chaos_api -- --ignored --test-threads=1 --nocapture
//! ```

use axum::http::{HeaderName, HeaderValue, Method, StatusCode};
use electronic_patient_record::{app::App, controllers, routes};
use loco_rs::testing::prelude::request;
use serde_json::{Value, json};

/// xorshift64*: small, seedable, the same on every platform.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
    fn chance(&mut self, percent: u64) -> bool {
        self.next() % 100 < percent
    }
}

fn junk_string(r: &mut Rng) -> String {
    let uuid = format!(
        "{:08x}-{:04x}-4{:03x}-8{:03x}-{:012x}",
        r.next() as u32,
        r.next() as u16,
        r.next() as u16 & 0xfff,
        r.next() as u16 & 0xfff,
        r.next() & 0xffff_ffff_ffff
    );
    let long = "A".repeat(2000 + r.below(5000));
    r.pick(&[
        uuid.clone(),
        uuid,
        "not-a-uuid".into(),
        "../../etc/passwd".into(),
        "\u{0}".into(),
        "0".into(),
        "-1".into(),
        "9999999999999999999999".into(),
        "💥🔥ñ\u{202e}rtl".into(),
        "' OR '1'='1".into(),
        "\"; DROP TABLE patients; --".into(),
        "<script>alert(1)</script>".into(),
        "{{7*7}}".into(),
        "${jndi:ldap://x}".into(),
        long,
        "null".into(),
        "true".into(),
        " ".into(),
    ])
    .clone()
}

/// Field names the handlers read, so the monkey gets past the first validation line.
const KEYS: &[&str] = &[
    "patient",
    "id",
    "version",
    "expected_version",
    "reason",
    "override_reason",
    "purpose",
    "code",
    "value",
    "unit",
    "status",
    "kind",
    "ground",
    "target",
    "allergy",
    "substance",
    "category",
    "criticality",
    "observation",
    "effective_at",
    "range",
    "low",
    "high",
    "absent_reason",
    "names",
    "family",
    "given",
    "birth_date",
    "sex",
    "identifiers",
    "system",
    "telecom",
    "addresses",
    "request",
    "event",
    "statement",
    "disagreement",
    "no_known_allergies",
    "scope",
    "granted",
    "subject",
    "relationship",
    "until",
    "limit",
    "offset",
    "q",
    "from",
    "to",
];

fn junk_value(r: &mut Rng, depth: u32) -> Value {
    match r.below(if depth > 2 { 7 } else { 10 }) {
        0 => Value::Null,
        1 => json!(r.chance(50)),
        2 => json!(r.next() as i64),
        3 => json!(-(r.next() as i64 % 1000)),
        4 => json!(f64::from_bits(r.next()).clamp(-1e300, 1e300)),
        5 | 6 => Value::String(junk_string(r)),
        7 => Value::Array((0..r.below(4)).map(|_| junk_value(r, depth + 1)).collect()),
        _ => junk_object(r, depth + 1),
    }
}

fn junk_object(r: &mut Rng, depth: u32) -> Value {
    let mut m = serde_json::Map::new();
    for _ in 0..r.below(7) {
        let key = if r.chance(85) {
            (*r.pick(KEYS)).to_string()
        } else {
            junk_string(r)
        };
        m.insert(key, junk_value(r, depth));
    }
    Value::Object(m)
}

/// Percent-encode everything but unreserved characters, so a hostile value reaches the handler as
/// one path segment instead of breaking the URL.
fn encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn query(r: &mut Rng) -> String {
    if r.chance(50) {
        return String::new();
    }
    let n = 1 + r.below(3);
    let pairs: Vec<String> = (0..n)
        .map(|_| {
            let k = if r.chance(70) {
                (*r.pick(KEYS)).to_string()
            } else {
                junk_string(r)
            };
            format!("{}={}", encode(&k), encode(&junk_string(r)))
        })
        .collect();
    format!("?{}", pairs.join("&"))
}

/// A response body must not show how the service is built.
fn leaks(body: &str) -> Option<&'static str> {
    [
        ("panicked at", "a panic message"),
        ("stack backtrace", "a backtrace"),
        ("RUST_BACKTRACE", "a backtrace hint"),
        ("SELECT ", "SQL text"),
        ("INSERT INTO", "SQL text"),
        ("sqlx", "a database driver name"),
        ("SeaORM", "a database driver name"),
        ("/src/", "a source path"),
        ("/Users/", "a file path"),
        ("/home/", "a file path"),
    ]
    .iter()
    .find(|(needle, _)| body.contains(needle))
    .map(|(_, what)| *what)
}

fn database_is_throwaway() {
    let url = std::env::var("DATABASE_URL").unwrap_or_default();
    let name = url.rsplit('/').next().unwrap_or_default();
    assert!(
        name.contains("test") || std::env::var("CI").is_ok(),
        "the monkey writes whatever the API allows: run it on a database named like *_test (got {name:?})"
    );
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL (a throwaway database)"]
async fn no_route_panics_answers_500_or_leaks_internals_under_hostile_input() {
    database_is_throwaway();
    // SAFETY: set before any thread of this test reads it, and the suite runs single-threaded.
    unsafe { std::env::set_var("EPR_REQUIRE_AUTH", "false") };
    let seed: u64 = std::env::var("EPR_CHAOS_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64
                | 1
        });
    let iterations: usize = std::env::var("EPR_CHAOS_ITERATIONS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2000);
    println!("EPR_CHAOS_SEED={seed} EPR_CHAOS_ITERATIONS={iterations}");

    let table: Vec<(Vec<Method>, String)> = controllers::all_routes()
        .collect()
        .into_iter()
        .map(|route| (route.actions, routes::normalise(&route.uri)))
        .collect();
    assert!(
        table.len() >= 20,
        "the monkey saw only {} routes",
        table.len()
    );

    request::<App, _, _>(move |server, _ctx| async move {
        let mut r = Rng(seed);
        let mut failures: Vec<String> = Vec::new();
        let mut seen: std::collections::BTreeMap<u16, usize> = std::collections::BTreeMap::new();
        for _ in 0..iterations {
            let (methods, template) = r.pick(&table).clone();
            let method = r.pick(&methods).clone();
            let path: String = template
                .split('/')
                .map(|seg| {
                    if seg.starts_with('{') && seg.ends_with('}') {
                        encode(&junk_string(&mut r))
                    } else {
                        seg.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("/");
            let url = format!("{path}{}", query(&mut r));
            let mut req = server.method(method.clone(), &url);
            let kind = r.below(5);
            let sent = match kind {
                0 => String::from("(no body)"),
                1 => {
                    req = req
                        .add_header(
                            HeaderName::from_static("content-type"),
                            HeaderValue::from_static("application/json"),
                        )
                        .bytes(b"{\"patient\": {".to_vec().into());
                    String::from("(malformed JSON)")
                }
                2 => {
                    req = req
                        .add_header(
                            HeaderName::from_static("content-type"),
                            HeaderValue::from_static("text/plain"),
                        )
                        .text(junk_string(&mut r));
                    String::from("(wrong content type)")
                }
                _ => {
                    let body = junk_object(&mut r, 0);
                    let text = body.to_string();
                    req = req.json(&body);
                    text.chars().take(300).collect()
                }
            };
            let res = req.await;
            let status = res.status_code();
            let text = res.text();
            *seen.entry(status.as_u16()).or_default() += 1;
            let bad_status = status.is_server_error()
                && !matches!(
                    status,
                    StatusCode::BAD_GATEWAY
                        | StatusCode::SERVICE_UNAVAILABLE
                        | StatusCode::GATEWAY_TIMEOUT
                );
            let leaked = leaks(&text);
            if bad_status || leaked.is_some() {
                let short: String = text.chars().take(200).collect();
                failures.push(format!(
                    "{method} {template} -> {status}{} | url {} | body {sent} | answer {short}",
                    leaked.map(|w| format!(" (leaks {w})")).unwrap_or_default(),
                    url.chars().take(200).collect::<String>()
                ));
            }
        }
        println!("answers by status: {seen:?}");
        // A control: if every request were refused alike, the monkey would be testing the gate, not
        // the handlers. Some requests must get past it and be answered or refused by a handler.
        assert!(
            seen.len() >= 4,
            "the monkey saw only these answers: {seen:?}"
        );
        assert!(
            failures.is_empty(),
            "EPR_CHAOS_SEED={seed}: {} bad answers, first ones:\n{}",
            failures.len(),
            failures
                .iter()
                .take(10)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    })
    .await;
}
