//! The health of the HL7 feed, and its alarm (task IH-8, hazard H-023).
//!
//! A refused message that nobody opens is silent, and so is a feed that stops sending. This judges each configured feed
//! from what the database holds, by the window the site's local time falls in:
//!
//! - `unknown`: the feed has sent nothing since the record began. No data is never `healthy`.
//! - `critical`: no message for longer than the window allows (`EPR_FEED_SILENCE_MINUTES`), or at least
//!   `EPR_FEED_REFUSALS_CRITICAL` refusals in the last hour.
//! - `warning`: at least `EPR_FEED_REFUSALS_WARNING` refusals in the last hour, or any refusal still open.
//! - `healthy`: none of these.
//!
//! The windows are the site's (`EPR_SITE_TIME_ZONE`): `day` is Monday to Friday 08:00 to 17:59, `evening` the rest of a
//! weekday, `weekend` Saturday and Sunday. The default thresholds are placeholders until a deployer sets them (OQ-IH4).
//!
//! While the listener is on, a loop judges every minute and stores each change of state in `feed_alarm_states`
//! (append-only), logging it once: a restart reads the last stored state, so it does not raise an alarm again. The
//! state is the gauge `epr_feed_health{sender}` and the route `GET /api/feed/health`. Nobody is paged: there is no
//! notifier, so a deployer alerts on the log line or the gauge.

use crate::patients_store::StoreError;
use crate::pool::Db;
use jiff::civil::Weekday;
use jiff::tz::TimeZone;
use jiff::{SignedDuration, Timestamp};
use serde::Serialize;
use sqlx::{PgPool, Row};
use std::sync::OnceLock;

crate::store_errors!("feed health");

/// The state of one feed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    /// Nothing to judge by.
    Unknown,
    /// Working as expected.
    Healthy,
    /// A person should look soon.
    Warning,
    /// A person should look now.
    Critical,
}

impl State {
    /// The stored and served word.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            State::Unknown => "unknown",
            State::Healthy => "healthy",
            State::Warning => "warning",
            State::Critical => "critical",
        }
    }

    /// The gauge value: 0 unknown, 1 healthy, 2 warning, 3 critical.
    #[must_use]
    pub fn gauge(self) -> u64 {
        match self {
            State::Unknown => 0,
            State::Healthy => 1,
            State::Warning => 2,
            State::Critical => 3,
        }
    }

    fn from_word(w: &str) -> Option<State> {
        Some(match w {
            "unknown" => State::Unknown,
            "healthy" => State::Healthy,
            "warning" => State::Warning,
            "critical" => State::Critical,
            _ => return None,
        })
    }
}

/// A part of the week with its own silence threshold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Window {
    /// Monday to Friday, 08:00 to 17:59 local.
    Day,
    /// The rest of a weekday.
    Evening,
    /// Saturday and Sunday.
    Weekend,
}

impl Window {
    /// The window `now` falls in, in `zone`.
    #[must_use]
    pub fn at(zone: &TimeZone, now: Timestamp) -> Window {
        let local = now.to_zoned(zone.clone());
        match local.weekday() {
            Weekday::Saturday | Weekday::Sunday => Window::Weekend,
            _ if (8..18).contains(&local.hour()) => Window::Day,
            _ => Window::Evening,
        }
    }
}

/// When a feed's silence or refusals raise its state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Thresholds {
    /// Longest silence, in minutes, in the day window.
    pub silence_day: u32,
    /// In the evening window.
    pub silence_evening: u32,
    /// At the weekend.
    pub silence_weekend: u32,
    /// Refusals in the last hour that make a warning.
    pub refusals_warning: u32,
    /// Refusals in the last hour that make it critical.
    pub refusals_critical: u32,
}

impl Default for Thresholds {
    /// Placeholders, not advice (OQ-IH4): an hour by day, four in the evening, twelve at the weekend; a warning at one
    /// refusal in an hour, critical at ten.
    fn default() -> Self {
        Thresholds {
            silence_day: 60,
            silence_evening: 240,
            silence_weekend: 720,
            refusals_warning: 1,
            refusals_critical: 10,
        }
    }
}

impl Thresholds {
    /// The thresholds from the environment, or the defaults for what is not set.
    ///
    /// # Errors
    ///
    /// A message naming the setting when one is malformed: a bad setting stops the service starting.
    pub fn from_env() -> Result<Thresholds, String> {
        Self::from_lookup(|k| std::env::var(k).ok())
    }

    /// [`Thresholds::from_env`] over any lookup.
    ///
    /// # Errors
    ///
    /// As [`Thresholds::from_env`].
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Result<Thresholds, String> {
        let mut t = Thresholds::default();
        let set = |k: &str| {
            get(k)
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };
        if let Some(v) = set("EPR_FEED_SILENCE_MINUTES") {
            let bad = || {
                format!(
                    "EPR_FEED_SILENCE_MINUTES {v:?} must be day=N,evening=N,weekend=N in whole minutes from 1 to 10080"
                )
            };
            for part in v.split(',') {
                let (k, n) = part.split_once('=').ok_or_else(bad)?;
                let n: u32 = n
                    .trim()
                    .parse()
                    .ok()
                    .filter(|n| (1..=10_080).contains(n))
                    .ok_or_else(bad)?;
                match k.trim() {
                    "day" => t.silence_day = n,
                    "evening" => t.silence_evening = n,
                    "weekend" => t.silence_weekend = n,
                    _ => return Err(bad()),
                }
            }
        }
        let count = |k: &str, default: u32| -> Result<u32, String> {
            match set(k) {
                None => Ok(default),
                Some(v) => v
                    .parse::<u32>()
                    .ok()
                    .filter(|n| (1..=100_000).contains(n))
                    .ok_or_else(|| format!("{k} must be a whole number from 1 to 100000")),
            }
        };
        t.refusals_warning = count("EPR_FEED_REFUSALS_WARNING", t.refusals_warning)?;
        t.refusals_critical = count("EPR_FEED_REFUSALS_CRITICAL", t.refusals_critical)?;
        if t.refusals_critical < t.refusals_warning {
            return Err(
                "EPR_FEED_REFUSALS_CRITICAL must not be below EPR_FEED_REFUSALS_WARNING".into(),
            );
        }
        Ok(t)
    }

    fn silence(&self, w: Window) -> u32 {
        match w {
            Window::Day => self.silence_day,
            Window::Evening => self.silence_evening,
            Window::Weekend => self.silence_weekend,
        }
    }
}

/// What the database says about one feed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Facts {
    /// The last message the feed sent that was audited, applied or refused; `None` when it has sent none.
    #[serde(with = "crate::clock::utc_option")]
    pub last_message_at: Option<Timestamp>,
    /// Refusals received in the hour before now.
    pub refusals_last_hour: u64,
    /// Refusals nobody has closed.
    pub open_refusals: u64,
}

/// The judgement of one feed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Assessment {
    /// The state.
    pub state: State,
    /// The window now falls in.
    pub window: Window,
    /// Why, in words; none when healthy.
    pub reasons: Vec<&'static str>,
}

/// Why a feed is unknown.
pub const NEVER_SENT: &str = "this feed has sent no message since the record began";
/// Why a feed is critical for silence.
pub const SILENT: &str = "no message for longer than this window allows";
/// Why a feed is critical for refusals.
pub const MANY_REFUSED: &str = "many messages were refused in the last hour";
/// Why a feed is a warning for refusals.
pub const SOME_REFUSED: &str = "messages were refused in the last hour";
/// Why a feed is a warning for its queue.
pub const WAITING: &str = "refused messages are waiting for a person";

/// Judge a feed at `now` (pure).
#[must_use]
pub fn assess(now: Timestamp, zone: &TimeZone, facts: &Facts, t: &Thresholds) -> Assessment {
    let window = Window::at(zone, now);
    let Some(last) = facts.last_message_at else {
        return Assessment {
            state: State::Unknown,
            window,
            reasons: vec![NEVER_SENT],
        };
    };
    let mut state = State::Healthy;
    let mut reasons = Vec::new();
    let mut raise = |to: State, why: &'static str| {
        state = state.max(to);
        reasons.push(why);
    };
    if now.duration_since(last) > SignedDuration::from_mins(i64::from(t.silence(window))) {
        raise(State::Critical, SILENT);
    }
    if facts.refusals_last_hour >= u64::from(t.refusals_critical) {
        raise(State::Critical, MANY_REFUSED);
    } else if facts.refusals_last_hour >= u64::from(t.refusals_warning) {
        raise(State::Warning, SOME_REFUSED);
    }
    if facts.open_refusals > 0 {
        raise(State::Warning, WAITING);
    }
    Assessment {
        state,
        window,
        reasons,
    }
}

/// What the database holds about `agent`'s feed at `now`.
///
/// # Errors
///
/// [`StoreError::Db`] when a query fails.
pub async fn facts(
    db: impl Into<Db<'_>>,
    agent: &str,
    now: Timestamp,
) -> Result<Facts, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let db = &mut *conn;
    let row = sqlx::query(
        "SELECT
           to_char((SELECT max(recorded_at) FROM audit_events WHERE agent = $1 AND resource_ref = 'Patient/hl7')
             AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"') AS last_message_at,
           (SELECT count(*) FROM feed_refusals WHERE agent = $1 AND received_at > $2::timestamptz) AS refusals_last_hour,
           (SELECT count(*) FROM feed_refusals f LEFT JOIN feed_refusal_resolutions r USING (refusal_id)
             WHERE f.agent = $1 AND r.refusal_id IS NULL) AS open_refusals",
    )
    .bind(agent)
    .bind(crate::clock::rfc3339_micros(now - SignedDuration::from_hours(1)))
    .fetch_one(&mut *db)
    .await
    .map_err(db_err)?;
    let n = |c: &str| -> Result<u64, StoreError> {
        row.try_get::<i64, _>(c)
            .map(|v| u64::try_from(v).unwrap_or(0))
            .map_err(db_err)
    };
    Ok(Facts {
        last_message_at: row
            .try_get::<Option<String>, _>("last_message_at")
            .map_err(db_err)?
            .as_deref()
            .and_then(crate::clock::parse_rfc3339),
        refusals_last_hour: n("refusals_last_hour")?,
        open_refusals: n("open_refusals")?,
    })
}

/// The last stored state of `agent`'s feed and when it began, if any was stored.
///
/// # Errors
///
/// [`StoreError::Db`] when the query fails.
pub async fn last_state(
    db: impl Into<Db<'_>>,
    agent: &str,
) -> Result<Option<(State, Timestamp)>, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let row = sqlx::query(
        "SELECT state, to_char(recorded_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"') AS recorded_at
           FROM feed_alarm_states WHERE agent = $1 ORDER BY feed_alarm_states.recorded_at DESC, seq DESC LIMIT 1",
    )
    .bind(agent)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db_err)?;
    let Some(row) = row else { return Ok(None) };
    let word: String = row.try_get("state").map_err(db_err)?;
    let at: String = row.try_get("recorded_at").map_err(db_err)?;
    let at = crate::clock::parse_rfc3339(&at)
        .ok_or_else(|| StoreError::Db(format!("a feed alarm time that is not a time: {at}")))?;
    Ok(State::from_word(&word).map(|s| (s, at)))
}

/// Judge `agent`'s feed and store the state when it changed. Returns the assessment and whether it changed. A change is
/// logged once: an error for critical, a warning for warning, information otherwise.
///
/// # Errors
///
/// [`StoreError::Db`] when a query fails.
pub async fn check(
    db: &PgPool,
    agent: &str,
    now: Timestamp,
    zone: &TimeZone,
    t: &Thresholds,
) -> Result<(Assessment, bool), StoreError> {
    let found = facts(db, agent, now).await?;
    let judged = assess(now, zone, &found, t);
    crate::metrics::feed_health(agent, judged.state.gauge());
    let before = last_state(db, agent).await?.map(|(s, _)| s);
    if before == Some(judged.state) {
        return Ok((judged, false));
    }
    sqlx::query("INSERT INTO feed_alarm_states (agent, state, reasons) VALUES ($1, $2, $3::jsonb)")
        .bind(agent)
        .bind(judged.state.word())
        .bind(serde_json::to_string(&judged.reasons).map_err(db_err)?)
        .execute(db)
        .await
        .map_err(db_err)?;
    let from = before.map_or("none", State::word);
    let to = judged.state.word();
    let reasons = judged.reasons.join("; ");
    match judged.state {
        State::Critical => tracing::error!(feed = %agent, from, to, %reasons, "HL7 feed alarm"),
        State::Warning => tracing::warn!(feed = %agent, from, to, %reasons, "HL7 feed alarm"),
        _ => tracing::info!(feed = %agent, from, to, %reasons, "HL7 feed state"),
    }
    Ok((judged, true))
}

/// The feed the listener serves, and its thresholds, set once at start-up when the listener is on.
static FEED: OnceLock<(String, Thresholds)> = OnceLock::new();

/// The configured feed, if the listener is on.
#[must_use]
pub fn configured() -> Option<&'static (String, Thresholds)> {
    FEED.get()
}

/// Judge the feed every minute, for as long as the service runs. Set once, when the listener starts.
pub async fn run_loop(db: PgPool, agent: String, thresholds: Thresholds) {
    let _ = FEED.set((agent.clone(), thresholds));
    let mut every = tokio::time::interval(std::time::Duration::from_secs(60));
    loop {
        every.tick().await;
        if let Err(e) = check(
            &db,
            &agent,
            Timestamp::now(),
            &crate::site::zone(),
            &thresholds,
        )
        .await
        {
            tracing::error!(feed = %agent, error = %e, "the HL7 feed's health could not be judged");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn at(s: &str) -> Timestamp {
        s.parse().unwrap()
    }

    #[test]
    fn the_window_is_the_sites_local_day_evening_or_weekend() {
        let london = TimeZone::get("Europe/London").unwrap();
        // Tuesday 2026-11-17: 08:00 and 17:59 are day, 18:00 and 07:59 evening; Saturday is the weekend.
        assert_eq!(Window::at(&london, at("2026-11-17T08:00:00Z")), Window::Day);
        assert_eq!(Window::at(&london, at("2026-11-17T17:59:00Z")), Window::Day);
        assert_eq!(
            Window::at(&london, at("2026-11-17T18:00:00Z")),
            Window::Evening
        );
        assert_eq!(
            Window::at(&london, at("2026-11-17T07:59:00Z")),
            Window::Evening
        );
        assert_eq!(
            Window::at(&london, at("2026-11-21T12:00:00Z")),
            Window::Weekend
        );
        // The zone decides: 08:30 UTC is 19:30 in Sydney, an evening.
        let sydney = TimeZone::get("Australia/Sydney").unwrap();
        assert_eq!(
            Window::at(&sydney, at("2026-11-17T08:30:00Z")),
            Window::Evening
        );
        // Summer time: 07:30 UTC on a July weekday is 08:30 in London, the day.
        assert_eq!(Window::at(&london, at("2026-07-14T07:30:00Z")), Window::Day);
    }

    #[test]
    fn no_data_is_unknown_never_healthy() {
        let a = assess(
            at("2026-11-17T10:00:00Z"),
            &TimeZone::UTC,
            &Facts {
                last_message_at: None,
                refusals_last_hour: 0,
                open_refusals: 0,
            },
            &Thresholds::default(),
        );
        assert_eq!(a.state, State::Unknown);
        assert_eq!(a.reasons, vec![NEVER_SENT]);
    }

    #[test]
    fn silence_beyond_the_window_is_critical_and_refusals_raise_it() {
        let t = Thresholds::default();
        let now = at("2026-11-17T10:00:00Z"); // a Tuesday morning: the day window, 60 minutes
        let facts = |mins: i64, last_hour: u64, open: u64| Facts {
            last_message_at: Some(now - SignedDuration::from_mins(mins)),
            refusals_last_hour: last_hour,
            open_refusals: open,
        };
        let judge = |f: Facts| assess(now, &TimeZone::UTC, &f, &t);
        assert_eq!(judge(facts(5, 0, 0)).state, State::Healthy);
        assert!(judge(facts(5, 0, 0)).reasons.is_empty());
        assert_eq!(
            judge(facts(60, 0, 0)).state,
            State::Healthy,
            "exactly the limit is not beyond it"
        );
        let silent = judge(facts(61, 0, 0));
        assert_eq!(
            (silent.state, silent.reasons),
            (State::Critical, vec![SILENT])
        );
        assert_eq!(judge(facts(5, 1, 1)).state, State::Warning);
        assert_eq!(judge(facts(5, 0, 1)).reasons, vec![WAITING]);
        let many = judge(facts(5, 10, 10));
        assert_eq!(
            (many.state, many.reasons),
            (State::Critical, vec![MANY_REFUSED, WAITING])
        );
        // The same silence at the weekend is within its twelve hours.
        let saturday = at("2026-11-21T10:00:00Z");
        let f = Facts {
            last_message_at: Some(saturday - SignedDuration::from_mins(300)),
            refusals_last_hour: 0,
            open_refusals: 0,
        };
        assert_eq!(
            assess(saturday, &TimeZone::UTC, &f, &t).state,
            State::Healthy
        );
    }

    #[test]
    fn thresholds_are_read_checked_and_default_to_placeholders() {
        let env = |pairs: &'static [(&'static str, &'static str)]| {
            move |k: &str| {
                pairs
                    .iter()
                    .find(|(n, _)| *n == k)
                    .map(|(_, v)| (*v).to_string())
            }
        };
        assert_eq!(
            Thresholds::from_lookup(env(&[])).unwrap(),
            Thresholds::default()
        );
        let t = Thresholds::from_lookup(env(&[
            ("EPR_FEED_SILENCE_MINUTES", "day=15, evening=90,weekend=600"),
            ("EPR_FEED_REFUSALS_WARNING", "2"),
            ("EPR_FEED_REFUSALS_CRITICAL", "5"),
        ]))
        .unwrap();
        assert_eq!(
            (
                t.silence_day,
                t.silence_evening,
                t.silence_weekend,
                t.refusals_warning,
                t.refusals_critical
            ),
            (15, 90, 600, 2, 5)
        );
        for bad in [
            &[("EPR_FEED_SILENCE_MINUTES", "day=0")][..],
            &[("EPR_FEED_SILENCE_MINUTES", "night=30")][..],
            &[("EPR_FEED_SILENCE_MINUTES", "day")][..],
            &[("EPR_FEED_REFUSALS_WARNING", "0")][..],
            &[
                ("EPR_FEED_REFUSALS_WARNING", "3"),
                ("EPR_FEED_REFUSALS_CRITICAL", "2"),
            ][..],
        ] {
            assert!(Thresholds::from_lookup(env(bad)).is_err(), "{bad:?}");
        }
    }
}
