//! Audit retention (task OS-13): monthly partitions of `audit_events` and an archive step that moves, never
//! deletes (migration 18 holds the SQL). The maintenance loop makes the partitions ahead of time, because
//! an insert with no partition fails and an access that cannot be audited does not happen. Archiving is
//! off unless `EPR_AUDIT_ARCHIVE_AFTER_MONTHS` is set: archived rows leave `audit_events`, so a patient's
//! access log no longer lists them (hazard H-042).

use crate::store::pg;
use sea_orm::{ConnectionTrait, DatabaseConnection};

/// Months of partitions made ahead.
pub const MONTHS_AHEAD: i32 = 24;

/// How often the maintenance loop runs.
const EVERY: std::time::Duration = std::time::Duration::from_secs(6 * 60 * 60);

/// The most months of audit rows `EPR_AUDIT_ARCHIVE_AFTER_MONTHS` may keep online.
const MAX_MONTHS: u32 = 1200;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// Make the partitions for this month and the next `months_ahead`; how many were made.
///
/// # Errors
///
/// The database's message.
pub async fn ensure_partitions(db: &DatabaseConnection, months_ahead: i32) -> Result<i32, String> {
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
    db: &DatabaseConnection,
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

/// Make the partitions, and archive when asked, now and every few hours. A failure is logged and the loop
/// carries on.
pub async fn run_loop(db: DatabaseConnection, archive_after_months: Option<u32>) {
    let mut tick = tokio::time::interval(EVERY);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tick.tick().await;
        match ensure_partitions(&db, MONTHS_AHEAD).await {
            Ok(0) => {}
            Ok(n) => tracing::info!(made = n, "audit partitions made"),
            Err(e) => tracing::error!(error = %e, "audit partitions could not be made"),
        }
        if let Some(months) = archive_after_months {
            match archive_older_than(&db, months).await {
                Ok(moved) if moved.is_empty() => {}
                Ok(moved) => tracing::warn!(partitions = ?moved, "audit partitions archived"),
                Err(e) => tracing::error!(error = %e, "audit archive failed"),
            }
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
