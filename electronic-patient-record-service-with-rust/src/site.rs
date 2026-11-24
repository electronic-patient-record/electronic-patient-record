//! The site's time zone and holidays, as one running service holds them (RC-9, TL-3, QA-16): read from that service's
//! settings (`EPR_SITE_TIME_ZONE`, `EPR_SITE_HOLIDAYS`) once at start, and kept in its [`Runtime`]. The parsing and
//! the day arithmetic are the domain crate's, re-exported here unchanged.

pub use electronic_patient_record_domain::site::*;

use crate::runtime::Runtime;
use jiff::tz::TimeZone;
use jiff::{Timestamp, civil::Date};
use std::collections::BTreeSet;
use std::sync::Arc;

/// Read and check `EPR_SITE_TIME_ZONE` and `EPR_SITE_HOLIDAYS` from this service's settings, once, at start-up.
///
/// # Errors
///
/// As [`zone_from`] and [`holidays_from`].
pub fn init() -> Result<(), String> {
    let zone = zone_from(crate::runtime::setting("EPR_SITE_TIME_ZONE").as_deref())?;
    let holidays = holidays_from(crate::runtime::setting("EPR_SITE_HOLIDAYS").as_deref())?;
    let runtime = Runtime::current();
    let _ = runtime.site_zone.set(zone);
    let _ = runtime.site_holidays.set(Arc::new(holidays));
    Ok(())
}

/// The site's holidays: the ones [`init`] checked, or none when it has not run (a test).
#[must_use]
pub fn holidays() -> Arc<BTreeSet<Date>> {
    Runtime::current()
        .site_holidays
        .get_or_init(|| Arc::new(BTreeSet::new()))
        .clone()
}

/// The site's zone: the one [`init`] checked, or [`DEFAULT_ZONE`] when it has not run (a test).
#[must_use]
pub fn time_zone() -> String {
    Runtime::current()
        .site_zone
        .get()
        .cloned()
        .unwrap_or_else(|| DEFAULT_ZONE.to_string())
}

/// The site's zone as a [`TimeZone`]: the one [`init`] checked, or UTC when it has not run (a test).
#[must_use]
pub fn zone() -> TimeZone {
    TimeZone::get(&time_zone()).unwrap_or(TimeZone::UTC)
}

/// Today at the site: the day a "last day" or a due day is judged on, the same for every reader there.
#[must_use]
pub fn today() -> Date {
    day_in(&zone(), Timestamp::now())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unset_in_a_test_the_zone_is_utc_and_there_are_no_holidays() {
        assert_eq!(zone(), TimeZone::UTC);
        assert_eq!(time_zone(), DEFAULT_ZONE);
        assert!(holidays().is_empty());
    }

    #[tokio::test]
    async fn each_app_holds_its_own_zone_and_holidays() {
        use crate::runtime::{Runtime, Settings};
        let london = Arc::new(Runtime::with_settings(Settings::from_pairs(&[
            ("EPR_SITE_TIME_ZONE", "Europe/London"),
            ("EPR_SITE_HOLIDAYS", "2026-12-25"),
        ])));
        let plain = Arc::new(Runtime::with_settings(Settings::from_pairs(&[])));
        let in_london = Runtime::scope(london, async {
            init().unwrap();
            (time_zone(), holidays().len())
        })
        .await;
        let in_plain = Runtime::scope(plain, async {
            init().unwrap();
            (time_zone(), holidays().len())
        })
        .await;
        assert_eq!(in_london, ("Europe/London".to_string(), 1));
        assert_eq!(in_plain, (DEFAULT_ZONE.to_string(), 0));
    }
}
