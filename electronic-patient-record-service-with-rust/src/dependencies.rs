//! `GET /_dependencies` says what is down (PX-5), and only says.
//!
//! `/_health` stays a plain "the process is up", because the container restarts on it, and `/_readiness`
//! stays the database alone, because a load balancer acts on it: a stopped key service must not take
//! the API out of service. So this route always answers 200, and a client reads `degraded`. `checked`
//! lists what was asked, so a dependency that is not configured never reads as one that is up.
//!
//! The route is public, so one answer is reused for 5 seconds and one round runs at a time: however
//! often it is called, a dependency is asked once per round. A key service is asked by opening a TCP
//! connection, which shows that it is listening and no more. Names are labels: an address or an error's
//! text goes to the log, never to the caller.

use crate::pool::Pool;
use axa::AppContext;
use axa::routes::Routes;
use axum::{Json, extract::State, routing::get};
use serde::Serialize;
use sqlx::PgPool;
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, LazyLock},
    time::Duration,
};
use tokio::{
    net::TcpStream,
    sync::Mutex,
    task::JoinSet,
    time::{Instant, timeout},
};

/// How long one answer is reused.
const REUSE_FOR: Duration = Duration::from_secs(5);

/// How long one dependency may take to answer.
const TIME_LIMIT: Duration = Duration::from_secs(2);

type Outcome = std::result::Result<(), String>;
type Pending = Pin<Box<dyn Future<Output = Outcome> + Send>>;

/// One dependency to ask: its label, and the question.
pub struct Check {
    name: &'static str,
    run: Pending,
}

impl Check {
    /// A dependency called `name`, answered by `run`.
    pub fn new(name: &'static str, run: impl Future<Output = Outcome> + Send + 'static) -> Self {
        Self {
            name,
            run: Box::pin(run),
        }
    }
}

/// What `/_dependencies` says.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Report {
    /// Whether nothing asked is down.
    pub ok: bool,
    /// The labels of what was asked, in order.
    pub checked: Vec<String>,
    /// The labels of what did not answer, in order.
    pub degraded: Vec<String>,
}

/// The last answer. Clones share it.
#[derive(Clone)]
pub struct Reused {
    last: Arc<Mutex<Option<(Instant, Report)>>>,
    reuse_for: Duration,
    time_limit: Duration,
}

impl Reused {
    /// Nothing asked yet, with these limits.
    #[must_use]
    pub fn new(reuse_for: Duration, time_limit: Duration) -> Self {
        Self {
            last: Arc::default(),
            reuse_for,
            time_limit,
        }
    }

    /// The reused answer, or a new round when it has gone stale. `checks` builds the round and is called
    /// only then. The round runs in its own task, so a caller that hangs up cannot cancel it and leave
    /// nothing to reuse, and callers that arrive during a round wait for its answer.
    pub async fn answer<F>(&self, checks: F) -> Report
    where
        F: FnOnce() -> Vec<Check> + Send + 'static,
    {
        let reused = self.clone();
        let round = tokio::spawn(async move {
            let mut last = reused.last.lock().await;
            if let Some((at, report)) = last.as_ref()
                && at.elapsed() < reused.reuse_for
            {
                return report.clone();
            }
            let report = round(checks(), reused.time_limit).await;
            *last = Some((Instant::now(), report.clone()));
            report
        })
        .await;
        round.unwrap_or_else(|error| {
            tracing::error!(%error, "dependency_round_failed");
            Report {
                ok: false,
                checked: Vec::new(),
                degraded: vec!["dependencies".to_string()],
            }
        })
    }
}

/// Ask every dependency at once. One that fails, outlasts `limit` or panics is degraded.
async fn round(checks: Vec<Check>, limit: Duration) -> Report {
    let names: Vec<&'static str> = checks.iter().map(|c| c.name).collect();
    let mut running = JoinSet::new();
    for (index, check) in checks.into_iter().enumerate() {
        running.spawn(async move { (index, timeout(limit, check.run).await) });
    }
    let mut answered = vec![false; names.len()];
    while let Some(finished) = running.join_next().await {
        match finished {
            Ok((index, Ok(Ok(())))) => answered[index] = true,
            Ok((index, Ok(Err(error)))) => {
                tracing::warn!(dependency = names[index], %error, "dependency_down");
            }
            Ok((index, Err(_))) => {
                tracing::warn!(dependency = names[index], "dependency_timed_out");
            }
            Err(error) => tracing::error!(%error, "dependency_check_panicked"),
        }
    }
    let degraded: Vec<String> = names
        .iter()
        .zip(&answered)
        .filter(|(_, answered)| !**answered)
        .map(|(name, _)| (*name).to_string())
        .collect();
    Report {
        ok: degraded.is_empty(),
        checked: names.iter().map(ToString::to_string).collect(),
        degraded,
    }
}

/// The host and port of a service's URL. A URL that cannot be read is a failed check, not a skipped one.
fn address(url: &str) -> std::result::Result<(String, u16), String> {
    let parsed = url::Url::parse(url).map_err(|e| e.to_string())?;
    let host = parsed.host_str().ok_or("the URL has no host")?;
    let port = parsed
        .port_or_known_default()
        .ok_or("the URL has no port")?;
    Ok((host.trim_matches(['[', ']']).to_string(), port))
}

/// Whether anything is listening at `url`.
async fn listening(url: String) -> Outcome {
    TcpStream::connect(address(&url)?)
        .await
        .map(drop)
        .map_err(|e| e.to_string())
}

/// The value of a setting, when it says anything.
fn setting(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty())
}

/// The database, then each key service this deployment configures.
fn configured(pool: PgPool, keycloak: Option<String>, paseto_keys: Option<String>) -> Vec<Check> {
    let mut checks = vec![Check::new("database", async move {
        sqlx::query("SELECT 1")
            .execute(&pool)
            .await
            .map(drop)
            .map_err(|e| e.to_string())
    })];
    if let Some(url) = keycloak {
        checks.push(Check::new("keycloak", listening(url)));
    }
    if let Some(url) = paseto_keys {
        checks.push(Check::new("paseto-keys", listening(url)));
    }
    checks
}

static LAST: LazyLock<Reused> = LazyLock::new(|| Reused::new(REUSE_FOR, TIME_LIMIT));

async fn report(State(ctx): State<AppContext>) -> Json<Report> {
    let pool = ctx.pool();
    let answer = LAST.answer(move || {
        configured(
            pool,
            setting("EPR_KEYCLOAK_URL"),
            setting("EPR_PASETO_KEYS_URL"),
        )
    });
    Json(answer.await)
}

/// `GET /_dependencies`.
#[must_use]
pub fn routes() -> Routes {
    Routes::new().add("/_dependencies", get(report))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::net::TcpListener;

    fn reused(reuse_for: Duration) -> Reused {
        Reused::new(reuse_for, Duration::from_millis(50))
    }

    fn pool() -> PgPool {
        sqlx::postgres::PgPoolOptions::new()
            .acquire_timeout(Duration::from_millis(20))
            .connect_lazy("postgres://secret-user:secret-pass@127.0.0.1:1/nothing")
            .unwrap()
    }

    #[test]
    fn an_address_is_read_from_the_url_with_the_scheme_giving_the_port() {
        let at = |url: &str| address(url).unwrap();
        assert_eq!(
            at("http://keycloak:8080/realms/x"),
            ("keycloak".into(), 8080)
        );
        assert_eq!(
            at("https://id.example.org/keys"),
            ("id.example.org".into(), 443)
        );
        assert_eq!(at("http://id.example.org"), ("id.example.org".into(), 80));
        assert_eq!(at("http://[::1]:8080/"), ("::1".into(), 8080));
        assert!(address("not a url").is_err());
    }

    #[tokio::test]
    async fn a_stopped_key_service_is_degraded_and_a_listening_one_is_not() {
        let up = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let down = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let up_url = format!("http://{}/", up.local_addr().unwrap());
        let down_url = format!("http://{}/", down.local_addr().unwrap());
        drop(down);
        let checks = vec![
            Check::new("keycloak", listening(up_url)),
            Check::new("paseto-keys", listening(down_url)),
        ];
        let report = round(checks, Duration::from_secs(1)).await;
        assert!(!report.ok);
        assert_eq!(report.checked, ["keycloak", "paseto-keys"]);
        assert_eq!(report.degraded, ["paseto-keys"]);
    }

    #[tokio::test]
    async fn a_url_that_cannot_be_read_is_degraded_not_skipped() {
        let checks = vec![Check::new("keycloak", listening("nonsense".into()))];
        let report = round(checks, Duration::from_secs(1)).await;
        assert_eq!(report.degraded, ["keycloak"]);
    }

    #[tokio::test]
    async fn only_what_is_configured_is_asked() {
        let names = |checks: Vec<Check>| checks.iter().map(|c| c.name).collect::<Vec<_>>();
        assert_eq!(names(configured(pool(), None, None)), ["database"]);
        let url = || Some("http://127.0.0.1:1/".to_string());
        assert_eq!(
            names(configured(pool(), url(), url())),
            ["database", "keycloak", "paseto-keys"]
        );
    }

    #[tokio::test]
    async fn the_answer_names_a_label_and_never_an_address_or_an_error() {
        let report = round(
            configured(pool(), Some("http://secret-host:1/".into()), None),
            Duration::from_secs(1),
        )
        .await;
        assert_eq!(report.degraded, ["database", "keycloak"]);
        let text = serde_json::to_string(&report).unwrap();
        assert!(!text.contains("secret"), "{text}");
    }

    #[tokio::test]
    async fn one_that_hangs_or_panics_is_degraded_and_the_others_still_answer() {
        let hangs = Check::new("hangs", std::future::pending());
        let panics = Check::new("panics", async { panic!("no") });
        let fine = Check::new("fine", async { Ok(()) });
        let report = round(vec![hangs, panics, fine], Duration::from_millis(20)).await;
        assert_eq!(report.degraded, ["hangs", "panics"]);
    }

    #[tokio::test]
    async fn many_calls_make_one_check() {
        let asked = Arc::new(AtomicUsize::new(0));
        let reused = reused(Duration::from_secs(60));
        let mut calls = JoinSet::new();
        for _ in 0..50 {
            let (reused, asked) = (reused.clone(), asked.clone());
            calls.spawn(async move {
                reused
                    .answer(move || {
                        asked.fetch_add(1, Ordering::SeqCst);
                        vec![Check::new("slow", async {
                            tokio::time::sleep(Duration::from_millis(10)).await;
                            Ok(())
                        })]
                    })
                    .await
            });
        }
        while let Some(call) = calls.join_next().await {
            assert!(call.unwrap().ok);
        }
        assert_eq!(asked.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn a_stale_answer_is_asked_again_and_a_failure_is_reused_too() {
        let asked = Arc::new(AtomicUsize::new(0));
        let reused = reused(Duration::ZERO);
        let call = || {
            let asked = asked.clone();
            reused.answer(move || {
                asked.fetch_add(1, Ordering::SeqCst);
                vec![Check::new("down", async { Err("no".to_string()) })]
            })
        };
        assert!(!call().await.ok);
        assert!(!call().await.ok);
        assert_eq!(asked.load(Ordering::SeqCst), 2);

        let kept = self::reused(Duration::from_secs(60));
        let down = || vec![Check::new("down", async { Err("no".to_string()) })];
        assert!(!kept.answer(down).await.ok);
        assert_eq!(kept.answer(Vec::new).await.degraded, ["down"]);
    }
}
