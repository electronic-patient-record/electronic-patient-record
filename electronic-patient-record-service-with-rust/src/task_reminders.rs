//! The reminder sender (TL-3, TL-4, hazard H-059): decides each task reminder once, on the site's calendar.
//!
//! Off unless `EPR_TASK_REMINDER_MINUTES` is set (1 to 1440): then every so many minutes it takes the reminders whose
//! day has come ([`tasks_store::reminders_due`]), decides each one (sent, or skipped with the reason when its day
//! passed while nothing ran and its rule says `skip`), and records that it ran today. A reminder is decided once: a
//! second replica, or a second run, finds it decided and leaves it. "Sent" puts the task on its team's list as
//! reminded (`reminded_on`); nothing goes to a patient, and no message leaves the service.
//!
//! Unset, no reminder is decided, so none shows on a list: the service logs a warning at start-up when it is off.

// RCA-ENF-3: this module is converted, so the arithmetic lints are denied here; the rest of the crate follows.
#![deny(
    clippy::arithmetic_side_effects,
    clippy::default_numeric_fallback,
    clippy::integer_division,
    clippy::modulo_arithmetic,
    clippy::as_conversions,
    clippy::string_slice
)]

use crate::patients_store::StoreError;
use crate::task_rules::ReminderNow;
use crate::tasks_store;
use jiff::civil::Date;
use sqlx::PgPool;
use std::time::Duration;

/// The setting.
pub const SETTING: &str = "EPR_TASK_REMINDER_MINUTES";

/// How often the sender runs, from `raw` (the setting's value): `None` when unset or blank (off).
///
/// # Errors
///
/// A message naming the setting when the value is not a whole number of minutes from 1 to 1440: a typo must not
/// switch reminders off in silence.
pub fn interval_from(raw: Option<&str>) -> Result<Option<Duration>, String> {
    let Some(raw) = raw.map(str::trim).filter(|v| !v.is_empty()) else {
        return Ok(None);
    };
    match raw.parse::<u64>() {
        Ok(m) if (1..=1440).contains(&m) => Ok(Some(Duration::from_mins(m))),
        _ => Err(format!(
            "{SETTING} must be a whole number of minutes from 1 to 1440, not {raw:?}"
        )),
    }
}

/// What one run decided.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tick {
    /// Reminders sent.
    pub sent: usize,
    /// Reminders skipped, with the reason recorded.
    pub skipped: usize,
    /// Reminders another run had already decided.
    pub already: usize,
}

/// Decide every reminder due on `today`, then record the run. Each decision is its own write, so a failure part way
/// leaves the decided ones decided and the rest for the next run.
///
/// # Errors
///
/// [`StoreError::Db`] when the reminders cannot be read, a decision cannot be written, or the run cannot be recorded.
pub async fn run_once(db: &PgPool, today: Date) -> Result<Tick, StoreError> {
    let mut tick = Tick::default();
    for r in tasks_store::reminders_due(db, today).await? {
        let skipped = match &r.now {
            ReminderNow::Wait => continue,
            ReminderNow::Send => None,
            ReminderNow::Skip { reason } => Some(reason.as_str()),
        };
        match tasks_store::decide_reminder(db, &r.task.id, r.on, skipped).await {
            Ok(()) if skipped.is_some() => count(&mut tick.skipped)?,
            Ok(()) => count(&mut tick.sent)?,
            Err(StoreError::Conflict { .. }) => count(&mut tick.already)?,
            Err(e) => return Err(e),
        }
    }
    tasks_store::record_run(db, today).await?;
    Ok(tick)
}

/// One more decision in a run's count. A count that no longer fits is an error, never a held or wrapped value.
fn count(n: &mut usize) -> Result<(), StoreError> {
    *n = n
        .checked_add(1)
        .ok_or_else(|| StoreError::Db("the reminder count overflows".into()))?;
    Ok(())
}

/// Run the sender every `every`, on the site's today, until the service stops. Counts only are logged: no patient,
/// task or title.
pub async fn run_loop(db: PgPool, every: Duration) {
    let mut ticks = tokio::time::interval(every);
    loop {
        ticks.tick().await;
        match run_once(&db, crate::site::today()).await {
            Ok(t) if t.sent > 0 || t.skipped > 0 => {
                tracing::info!(sent = t.sent, skipped = t.skipped, "task reminders decided");
            }
            Ok(_) => {}
            Err(e) => tracing::error!(error = %e, "task reminders could not be decided"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sender_is_off_unless_set_and_a_bad_value_stops_the_start() {
        assert_eq!(interval_from(None), Ok(None));
        assert_eq!(interval_from(Some("  ")), Ok(None));
        assert_eq!(
            interval_from(Some(" 15 ")),
            Ok(Some(Duration::from_secs(900)))
        );
        assert_eq!(
            interval_from(Some("1440")),
            Ok(Some(Duration::from_secs(86_400)))
        );
        for bad in ["0", "1441", "-5", "15m", "1.5", "fifteen"] {
            let e = interval_from(Some(bad)).unwrap_err();
            assert!(e.contains(SETTING) && e.contains(bad), "{e}");
        }
    }
}
