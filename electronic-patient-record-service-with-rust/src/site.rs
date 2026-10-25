//! The site's time zone (RC-9): the zone whose calendar day a patient's age is counted on.
//!
//! Set by `EPR_SITE_TIME_ZONE`, an IANA name such as `Europe/London`; unset or blank means `UTC`. A value
//! that is not an IANA name stops the service starting ([`init`]): a typo that quietly fell back to UTC
//! would show some patients' ages a day out near midnight, and nobody would know why. The zone is the
//! site's, not the reader's: every clinician at the site sees the same age on the same day.

use chrono_tz::Tz;
use std::sync::OnceLock;

/// The zone when none is set.
pub const DEFAULT_ZONE: &str = "UTC";

static ZONE: OnceLock<String> = OnceLock::new();

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
    name.parse::<Tz>().map(|_| name.to_string()).map_err(|_| {
        format!(
            "EPR_SITE_TIME_ZONE `{name}` is not an IANA time zone name (for example Europe/London)"
        )
    })
}

/// Read and check `EPR_SITE_TIME_ZONE` once, at start-up.
///
/// # Errors
///
/// As [`zone_from`].
pub fn init() -> Result<(), String> {
    let zone = zone_from(std::env::var("EPR_SITE_TIME_ZONE").ok().as_deref())?;
    let _ = ZONE.set(zone);
    Ok(())
}

/// The site's zone: the one [`init`] checked, or [`DEFAULT_ZONE`] when it has not run (a test).
#[must_use]
pub fn time_zone() -> String {
    ZONE.get()
        .cloned()
        .unwrap_or_else(|| DEFAULT_ZONE.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

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
        ] {
            let e = zone_from(Some(bad)).unwrap_err();
            assert!(e.contains("EPR_SITE_TIME_ZONE") && e.contains(bad), "{e}");
        }
    }
}
