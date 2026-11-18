//! The site's time zone (RC-9): the zone whose calendar day a patient's age is counted on; and the site's holidays
//! (TL-3), the days that are not working days when a task's due day is counted.
//!
//! Set by `EPR_SITE_TIME_ZONE`, an IANA name such as `Europe/London`; unset or blank means `UTC`. A value
//! that is not an IANA name stops the service starting ([`init`]): a typo that quietly fell back to UTC
//! would show some patients' ages a day out near midnight, and nobody would know why. The zone is the
//! site's, not the reader's: every clinician at the site sees the same age on the same day.

use jiff::tz::TimeZone;
use jiff::{Timestamp, civil::Date};
use std::collections::BTreeSet;
use std::sync::OnceLock;

/// The zone when none is set.
pub const DEFAULT_ZONE: &str = "UTC";

static ZONE: OnceLock<String> = OnceLock::new();
static HOLIDAYS: OnceLock<BTreeSet<Date>> = OnceLock::new();

/// The most holidays the setting may list: about ten years of a site's calendar. A longer list is a mistake.
pub const MAX_HOLIDAYS: usize = 200;

/// The zone named by `value` (blank or absent is [`DEFAULT_ZONE`]).
///
/// # Errors
///
/// A message naming the setting when `value` is not an IANA time zone name.
pub fn zone_from(value: Option<&str>) -> Result<String, String> {
    let name = value
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or(DEFAULT_ZONE);
    // jiff finds a zone whatever its case; the name must be the database's own spelling, as it had to be before
    // DEP-1, so a setting reads the same in every log and screen.
    let known = TimeZone::get(name)
        .ok()
        .is_some_and(|tz| tz.iana_name() == Some(name));
    known.then(|| name.to_string()).ok_or_else(|| {
        format!(
            "EPR_SITE_TIME_ZONE `{name}` is not an IANA time zone name (for example Europe/London)"
        )
    })
}

/// The holidays named by `value`: calendar days `YYYY-MM-DD`, separated by commas, spaces or both, in any order, a
/// repeat counted once. Blank or absent is none: weekends are still not working days.
///
/// # Errors
///
/// A message naming the setting and the first value that is not a calendar day, or saying there are more than
/// [`MAX_HOLIDAYS`]. A day that does not parse is refused rather than skipped: a due day counted through a forgotten
/// holiday is a day early, and nobody would know why.
pub fn holidays_from(value: Option<&str>) -> Result<BTreeSet<Date>, String> {
    let mut days = BTreeSet::new();
    for word in value
        .unwrap_or("")
        .split([',', ' ', '\t', '\n'])
        .filter(|w| !w.is_empty())
    {
        let ok = word.len() == 10 && word.as_bytes()[4] == b'-' && word.as_bytes()[7] == b'-';
        let day: Date = ok.then(|| word.parse().ok()).flatten().ok_or_else(|| {
            format!("EPR_SITE_HOLIDAYS `{word}` is not a calendar day (YYYY-MM-DD)")
        })?;
        days.insert(day);
    }
    if days.len() > MAX_HOLIDAYS {
        return Err(format!(
            "EPR_SITE_HOLIDAYS lists {} days; at most {MAX_HOLIDAYS}",
            days.len()
        ));
    }
    Ok(days)
}

/// Read and check `EPR_SITE_TIME_ZONE` and `EPR_SITE_HOLIDAYS` once, at start-up.
///
/// # Errors
///
/// As [`zone_from`] and [`holidays_from`].
pub fn init() -> Result<(), String> {
    let zone = zone_from(std::env::var("EPR_SITE_TIME_ZONE").ok().as_deref())?;
    let holidays = holidays_from(std::env::var("EPR_SITE_HOLIDAYS").ok().as_deref())?;
    let _ = ZONE.set(zone);
    let _ = HOLIDAYS.set(holidays);
    Ok(())
}

/// The site's holidays: the ones [`init`] checked, or none when it has not run (a test).
#[must_use]
pub fn holidays() -> &'static BTreeSet<Date> {
    HOLIDAYS.get_or_init(BTreeSet::new)
}

/// The site's zone: the one [`init`] checked, or [`DEFAULT_ZONE`] when it has not run (a test).
#[must_use]
pub fn time_zone() -> String {
    ZONE.get()
        .cloned()
        .unwrap_or_else(|| DEFAULT_ZONE.to_string())
}

/// The site's zone as a [`TimeZone`]: the one [`init`] checked, or UTC when it has not run (a test).
#[must_use]
pub fn zone() -> TimeZone {
    TimeZone::get(&time_zone()).unwrap_or(TimeZone::UTC)
}

/// The calendar day `now` falls on in `zone`.
#[must_use]
pub fn day_in(zone: &TimeZone, now: Timestamp) -> Date {
    now.to_zoned_datetime(zone.clone()).date()
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
    fn the_sites_day_can_differ_from_the_utc_day() {
        let late: Timestamp = "2026-06-30T23:30:00Z".parse().unwrap();
        assert_eq!(day_in(&TimeZone::UTC, late).to_string(), "2026-06-30");
        let london = TimeZone::get("Europe/London").unwrap();
        assert_eq!(day_in(&london, late).to_string(), "2026-07-01");
        assert_eq!(zone(), TimeZone::UTC, "unset in a test: UTC");
    }

    #[test]
    fn unset_or_blank_is_utc_and_an_iana_name_is_kept() {
        assert_eq!(zone_from(None).unwrap(), "UTC");
        assert_eq!(zone_from(Some("  ")).unwrap(), "UTC");
        assert_eq!(zone_from(Some("Europe/London")).unwrap(), "Europe/London");
        assert_eq!(
            zone_from(Some(" Pacific/Auckland ")).unwrap(),
            "Pacific/Auckland"
        );
    }

    #[test]
    fn a_name_that_is_not_a_zone_is_refused_naming_the_setting() {
        for bad in [
            "BST",
            "London",
            "Mars/Olympus",
            "+01:00",
            "europe/london-ish",
            // The right zone in the wrong case: refused before DEP-1, and still.
            "europe/london",
        ] {
            let e = zone_from(Some(bad)).unwrap_err();
            assert!(e.contains("EPR_SITE_TIME_ZONE") && e.contains(bad), "{e}");
        }
    }

    #[test]
    fn holidays_are_calendar_days_and_anything_else_stops_the_service() {
        let d = |s: &str| -> Date { s.parse().unwrap() };
        assert!(holidays_from(None).unwrap().is_empty());
        assert!(holidays_from(Some("  ")).unwrap().is_empty());
        assert_eq!(
            holidays_from(Some("2026-12-28, 2026-12-25,2026-12-25\n2027-01-01")).unwrap(),
            BTreeSet::from([d("2026-12-25"), d("2026-12-28"), d("2027-01-01")]),
            "any separator, any order, a repeat once"
        );
        for bad in [
            "2026-13-01",
            "2026-02-30",
            "25/12/2026",
            "20261225",
            "2026-1-1",
            "christmas",
        ] {
            let e = holidays_from(Some(&format!("2026-12-25,{bad}"))).unwrap_err();
            assert!(e.contains("EPR_SITE_HOLIDAYS") && e.contains(bad), "{e}");
        }
        let many: Vec<String> = (0..=MAX_HOLIDAYS)
            .map(|i| {
                (d("2026-01-01") + jiff::Span::new().days(i64::try_from(i).unwrap())).to_string()
            })
            .collect();
        assert!(holidays_from(Some(&many[..MAX_HOLIDAYS].join(","))).is_ok());
        assert!(
            holidays_from(Some(&many.join(",")))
                .unwrap_err()
                .contains("at most")
        );
    }
}
