//! Audit retention (task OS-13): monthly partitions of `audit_events` and an archive step that moves, never
//! deletes (migration 18 holds the SQL). The maintenance loop makes the partitions ahead of time, because
//! an insert with no partition fails and an access that cannot be audited does not happen. Archiving is
//! off unless `EPR_AUDIT_ARCHIVE_AFTER_MONTHS` is set: archived rows leave `audit_events`, and a patient's
//! access log and the audit search still list them through the `audit_events_all` view (hazard H-042).

use crate::store::pg;
use sea_orm::{ConnectionTrait, DatabaseConnection, TransactionTrait};

/// Months of partitions made ahead.
pub const MONTHS_AHEAD: i32 = 24;

/// How often the maintenance loop runs.
const EVERY: std::time::Duration = std::time::Duration::from_secs(6 * 60 * 60);

/// The most months of audit rows `EPR_AUDIT_ARCHIVE_AFTER_MONTHS` may keep online.
const MAX_MONTHS: u32 = 1200;

/// A displayable error as the text the callers return.
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// Make the partitions for this month and the next `months_ahead`; how many were made.
///
/// # Errors
///
/// The database's message.
pub async fn ensure_partitions(
    db: &impl ConnectionTrait,
    months_ahead: i32,
) -> Result<i32, String> {
    db.query_one_raw(pg(
        "SELECT audit_ensure_partitions($1) AS made",
        vec![months_ahead.into()],
    ))
    .await
    .map_err(err)?
    .ok_or_else(|| "no answer".to_string())?
    .try_get("", "made")
    .map_err(err)
}

/// Move the partitions that end at or before `months` whole months ago into `audit_archive`, oldest
/// first, recording an anchor for each; the names moved.
///
/// # Errors
///
/// The database's message; nothing is moved then (the function is one statement).
pub async fn archive_older_than(
    db: &impl ConnectionTrait,
    months: u32,
) -> Result<Vec<String>, String> {
    let months = i32::try_from(months).map_err(err)?;
    let row = db
        .query_one_raw(pg(
            "SELECT audit_archive_before(
                 ((date_trunc('month', now() AT TIME ZONE 'UTC') - make_interval(months => $1))
                   AT TIME ZONE 'UTC')) AS moved",
            vec![months.into()],
        ))
        .await
        .map_err(err)?
        .ok_or_else(|| "no answer".to_string())?;
    row.try_get("", "moved").map_err(err)
}

/// `EPR_AUDIT_ARCHIVE_AFTER_MONTHS`: unset means never archive; otherwise 1 to 1200.
///
/// # Errors
///
/// A value that is not a whole number from 1 to 1200.
pub fn parse_archive_months(value: Option<&str>) -> Result<Option<u32>, String> {
    match value.map(str::trim) {
        None => Ok(None),
        Some(v) => match v.parse::<u32>() {
            Ok(n) if (1..=MAX_MONTHS).contains(&n) => Ok(Some(n)),
            _ => Err(format!(
                "EPR_AUDIT_ARCHIVE_AFTER_MONTHS must be a whole number from 1 to {MAX_MONTHS}, not {v:?}"
            )),
        },
    }
}

/// The setting from the environment.
///
/// # Errors
///
/// See [`parse_archive_months`].
pub fn archive_months_from_env() -> Result<Option<u32>, String> {
    parse_archive_months(
        std::env::var("EPR_AUDIT_ARCHIVE_AFTER_MONTHS")
            .ok()
            .as_deref(),
    )
}

/// The advisory-lock key of the maintenance job: one replica at a time runs it (PR-4).
pub const LOCK_KEY: i64 = 0x4550_5241_5544_4954;

/// What one run of the job did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ran {
    /// Another replica held the lock; this one did nothing.
    Skipped,
    /// This replica ran the job.
    Done {
        /// Partitions made.
        made: i32,
        /// Partitions archived.
        moved: Vec<String>,
    },
}

/// Run the job once: take the job's transaction-scoped advisory lock without waiting, make the partitions
/// and archive when asked. A replica that finds the lock taken does nothing (two replicas used to race:
/// the second archive failed with "relation does not exist" and was logged as an error).
///
/// # Errors
///
/// The database's message; the transaction is rolled back then.
pub async fn run_once(
    db: &DatabaseConnection,
    months_ahead: i32,
    archive_after_months: Option<u32>,
) -> Result<Ran, String> {
    let tx = db.begin().await.map_err(err)?;
    let got: bool = tx
        .query_one_raw(pg(
            "SELECT pg_try_advisory_xact_lock($1) AS got",
            vec![LOCK_KEY.into()],
        ))
        .await
        .map_err(err)?
        .ok_or_else(|| "no answer".to_string())?
        .try_get("", "got")
        .map_err(err)?;
    if !got {
        return Ok(Ran::Skipped);
    }
    let made = ensure_partitions(&tx, months_ahead).await?;
    let moved = match archive_after_months {
        Some(months) => archive_older_than(&tx, months).await?,
        None => Vec::new(),
    };
    tx.commit().await.map_err(err)?;
    Ok(Ran::Done { made, moved })
}

/// Make the partitions, and archive when asked, now and every few hours. A failure is logged and the loop
/// carries on. Only one replica runs each round ([`run_once`]).
pub async fn run_loop(db: DatabaseConnection, archive_after_months: Option<u32>) {
    let mut tick = tokio::time::interval(EVERY);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tick.tick().await;
        match run_once(&db, MONTHS_AHEAD, archive_after_months).await {
            Ok(Ran::Skipped) => {
                tracing::debug!("audit maintenance skipped: another replica is running it")
            }
            Ok(Ran::Done { made, moved }) => {
                if made > 0 {
                    tracing::info!(made, "audit partitions made");
                }
                if !moved.is_empty() {
                    tracing::warn!(partitions = ?moved, "audit partitions archived");
                }
            }
            Err(e) => tracing::error!(error = %e, "audit maintenance failed"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archiving_is_off_unless_set_and_refuses_nonsense() {
        assert_eq!(parse_archive_months(None), Ok(None));
        assert_eq!(parse_archive_months(Some(" 12 ")), Ok(Some(12)));
        for bad in ["0", "1201", "-1", "a year", ""] {
            assert!(parse_archive_months(Some(bad)).is_err(), "{bad:?}");
        }
    }
}
