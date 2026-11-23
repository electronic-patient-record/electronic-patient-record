//! The time the service acts on (task PR-5), and how it writes a time down (DEP-1). Code that decides by the clock
//! takes a [`Clock`] instead of asking the system, so a test moves time instead of rewriting rows. [`SystemClock`]
//! is the real one; [`FixedClock`] stands still until told to move.
//!
//! First slice: idempotency expiry takes the time from here. Break-glass expiry and the referral
//! timeouts still ask the database's `now()` in SQL and are not yet on this clock.
//!
//! Times are [`jiff::Timestamp`] (an instant) and [`jiff::civil::Date`] (a day). The formats below are the ones the
//! service wrote with chrono before DEP-1, kept byte for byte, since stored JSON and answers carry them.

use std::sync::Mutex;

use jiff::{SignedDuration, Timestamp};

/// A source of the current time.
pub trait Clock: Send + Sync {
    /// The time now.
    fn now(&self) -> Timestamp;
}

/// The real time.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        Timestamp::now()
    }
}

/// A clock that shows one time until it is moved. For tests.
#[derive(Debug)]
pub struct FixedClock(Mutex<Timestamp>);

impl FixedClock {
    /// A clock showing `at`.
    #[must_use]
    pub fn new(at: Timestamp) -> Self {
        FixedClock(Mutex::new(at))
    }

    /// Move the clock forward (or back, with a negative duration).
    pub fn advance(&self, by: SignedDuration) {
        let mut at = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        // ARITH-1: a test clock moved past the representable range stays where it was (it is never read in production).
        *at = at.checked_add(by).unwrap_or(*at);
    }
}

impl Clock for FixedClock {
    fn now(&self) -> Timestamp {
        *self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// `2026-11-17T10:05:09.123456Z`: UTC to the microsecond, always six digits (chrono's
/// `to_rfc3339_opts(SecondsFormat::Micros, true)`).
#[must_use]
pub fn rfc3339_micros(t: Timestamp) -> String {
    t.strftime("%Y-%m-%dT%H:%M:%S%.6fZ").to_string()
}

/// `2026-11-17T10:05:09Z`: UTC to the second, the fraction dropped (chrono's
/// `to_rfc3339_opts(SecondsFormat::Secs, true)`).
#[must_use]
pub fn rfc3339_secs(t: Timestamp) -> String {
    t.strftime("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// `n` divided by `unit` when it divides exactly, else `None`.
fn exactly(n: i32, unit: i32) -> Option<i32> {
    if n.checked_rem(unit)? == 0 {
        n.checked_div(unit)
    } else {
        None
    }
}

/// The fraction of a second as chrono wrote it: in groups of three digits, none when it is zero.
fn fraction(t: Timestamp) -> String {
    let nanos = t.subsec_nanosecond();
    if nanos == 0 {
        String::new()
    } else if let Some(millis) = exactly(nanos, 1_000_000_i32) {
        format!(".{millis:03}")
    } else if let Some(micros) = exactly(nanos, 1_000_i32) {
        format!(".{micros:06}")
    } else {
        format!(".{nanos:09}")
    }
}

/// `2026-11-17T10:05:09.123456+00:00`: chrono's `to_rfc3339()` of a UTC time, with `+00:00` and the fraction in
/// groups of three digits, none when it is zero.
#[must_use]
pub fn rfc3339_offset(t: Timestamp) -> String {
    format!("{}{}+00:00", t.strftime("%Y-%m-%dT%H:%M:%S"), fraction(t))
}

/// `2026-11-17T10:05:09.123456Z`: how chrono's serde wrote a UTC time, the fraction in groups of three digits, none
/// when it is zero.
#[must_use]
pub fn rfc3339_z(t: Timestamp) -> String {
    format!("{}{}Z", t.strftime("%Y-%m-%dT%H:%M:%S"), fraction(t))
}

/// Serde for a [`Timestamp`] field, written as chrono wrote one ([`rfc3339_z`]) and read from any RFC 3339 time, so
/// answers and stored JSON keep their form: `#[serde(with = "crate::clock::utc")]`.
pub mod utc {
    use jiff::Timestamp;
    use serde::{Deserialize, Deserializer, Serializer};

    /// Write `t` as [`super::rfc3339_z`].
    ///
    /// # Errors
    ///
    /// The serializer's.
    pub fn serialize<S: Serializer>(t: &Timestamp, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&super::rfc3339_z(*t))
    }

    /// Read any RFC 3339 time.
    ///
    /// # Errors
    ///
    /// When the text is not an RFC 3339 time.
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Timestamp, D::Error> {
        let text = String::deserialize(d)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

/// [`utc`] for an `Option<Timestamp>` field: `#[serde(default, with = "crate::clock::utc_option")]`.
pub mod utc_option {
    use jiff::Timestamp;
    use serde::{Deserialize, Deserializer, Serializer};

    /// Write `t` as [`super::rfc3339_z`], or `null`.
    ///
    /// # Errors
    ///
    /// The serializer's.
    pub fn serialize<S: Serializer>(t: &Option<Timestamp>, s: S) -> Result<S::Ok, S::Error> {
        match t {
            Some(t) => s.serialize_str(&super::rfc3339_z(*t)),
            None => s.serialize_none(),
        }
    }

    /// Read any RFC 3339 time, or `null`.
    ///
    /// # Errors
    ///
    /// When the text is not an RFC 3339 time.
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Timestamp>, D::Error> {
        match Option::<String>::deserialize(d)? {
            Some(text) => text.parse().map(Some).map_err(serde::de::Error::custom),
            None => Ok(None),
        }
    }
}

/// An RFC 3339 time with any offset, as an instant; `None` when it is not one. As strict as chrono's
/// `parse_from_rfc3339` was: a date, `T` (or `t` or a space), hours, minutes and seconds, then a fraction if any and
/// `Z` or an offset. jiff alone would also take a time without seconds, or a bracketed zone name.
#[must_use]
pub fn parse_rfc3339(text: &str) -> Option<Timestamp> {
    let b = text.as_bytes();
    let digits =
        |r: std::ops::Range<usize>| b.get(r).is_some_and(|d| d.iter().all(u8::is_ascii_digit));
    let shape = b.len() >= 20
        && digits(0..4)
        && b.get(4) == Some(&b'-')
        && digits(5..7)
        && b.get(7) == Some(&b'-')
        && digits(8..10)
        && matches!(b.get(10), Some(b'T' | b't' | b' '))
        && digits(11..13)
        && b.get(13) == Some(&b':')
        && digits(14..16)
        && b.get(16) == Some(&b':')
        && digits(17..19)
        && !text.contains('[');
    if !shape {
        return None;
    }
    text.parse::<Timestamp>().ok()
}

/// The UTC calendar day of `t` (chrono's `date_naive()` of a UTC time).
#[must_use]
pub fn utc_date(t: Timestamp) -> jiff::civil::Date {
    t.to_zoned_datetime(jiff::tz::TimeZone::UTC).date()
}

/// A `YYYY-MM-DD` day, or `None` (chrono's `NaiveDate::parse_from_str(text, "%Y-%m-%d")`).
#[must_use]
pub fn parse_day(text: &str) -> Option<jiff::civil::Date> {
    jiff::civil::Date::strptime("%Y-%m-%d", text).ok()
}

/// The day `year`-`month`-`day`, or `None` when there is no such day (chrono's `NaiveDate::from_ymd_opt`).
#[must_use]
pub fn ymd(year: i32, month: u32, day: u32) -> Option<jiff::civil::Date> {
    let year = i16::try_from(year).ok()?;
    let month = i8::try_from(month).ok()?;
    let day = i8::try_from(day).ok()?;
    jiff::civil::Date::new(year, month, day).ok()
}

/// 00:00 UTC on `day`, as an instant.
#[must_use]
pub fn day_start_utc(day: jiff::civil::Date) -> Option<Timestamp> {
    day.to_zoned_datetime(jiff::tz::TimeZone::UTC)
        .ok()
        .map(|z| z.timestamp())
}

/// A whole number of days as a duration (a day is 24 hours here: an instant has no calendar), or `None` when the
/// duration does not fit (RCA-INT-1).
#[must_use]
pub fn days(n: i64) -> Option<SignedDuration> {
    n.checked_mul(86_400).map(SignedDuration::from_secs)
}

/// `at` plus `n` days, or `None` when the result is out of range (RCA-TIME-3).
#[must_use]
pub fn add_days(at: Timestamp, n: i64) -> Option<Timestamp> {
    at.checked_add(days(n)?).ok()
}

/// `at` minus `n` days, or `None` when the result is out of range (RCA-TIME-3).
#[must_use]
pub fn sub_days(at: Timestamp, n: i64) -> Option<Timestamp> {
    at.checked_sub(days(n)?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fixed_clock_stands_still_until_moved() {
        let start: Timestamp = "2026-11-04T09:00:00Z".parse().unwrap();
        let c = FixedClock::new(start);
        assert_eq!(c.now(), start);
        assert_eq!(c.now(), start);
        c.advance(SignedDuration::from_hours(25));
        assert_eq!(c.now(), start + SignedDuration::from_hours(25));
        c.advance(SignedDuration::from_hours(-25));
        assert_eq!(c.now(), start);
    }

    #[test]
    fn the_formats_are_the_ones_written_before_dep_1() {
        let t: Timestamp = "2026-11-17T10:05:09.123456789Z".parse().unwrap();
        assert_eq!(rfc3339_micros(t), "2026-11-17T10:05:09.123456Z");
        assert_eq!(rfc3339_secs(t), "2026-11-17T10:05:09Z");
        assert_eq!(rfc3339_offset(t), "2026-11-17T10:05:09.123456789+00:00");
        let whole: Timestamp = "2026-11-17T10:05:09Z".parse().unwrap();
        assert_eq!(rfc3339_micros(whole), "2026-11-17T10:05:09.000000Z");
        assert_eq!(rfc3339_offset(whole), "2026-11-17T10:05:09+00:00");
        let milli: Timestamp = "2026-11-17T10:05:09.120Z".parse().unwrap();
        assert_eq!(rfc3339_offset(milli), "2026-11-17T10:05:09.120+00:00");
        let micro: Timestamp = "2026-11-17T10:05:09.000120Z".parse().unwrap();
        assert_eq!(rfc3339_offset(micro), "2026-11-17T10:05:09.000120+00:00");
        assert_eq!(rfc3339_z(milli), "2026-11-17T10:05:09.120Z");
        assert_eq!(rfc3339_z(whole), "2026-11-17T10:05:09Z");
        assert_eq!(parse_rfc3339("2026-11-17T11:05:09+01:00"), Some(whole));
        assert_eq!(parse_rfc3339("not a time"), None);
        // What chrono's parser refused, refused still.
        for loose in [
            "2026-11-17T10:05Z",
            "2026-11-17T10:05:09",
            "2026-11-17T10:05:09Z[Europe/London]",
            "20261117T100509Z",
        ] {
            assert_eq!(parse_rfc3339(loose), None, "{loose}");
        }
        assert_eq!(parse_rfc3339("2026-11-17 10:05:09Z"), Some(whole));
        assert_eq!(utc_date(whole).to_string(), "2026-11-17");
        assert_eq!(parse_day("2026-02-29"), None);
        assert_eq!(
            parse_day("2028-02-29").map(|d| d.to_string()).as_deref(),
            Some("2028-02-29")
        );
        assert_eq!(ymd(2026, 2, 30), None);
        assert_eq!(ymd(2026, 11, 17), parse_day("2026-11-17"));
        assert_eq!(
            day_start_utc(parse_day("2026-11-17").unwrap()),
            parse_rfc3339("2026-11-17T00:00:00Z")
        );
        assert_eq!(days(2), Some(SignedDuration::from_hours(48)));
        assert_eq!(
            days(i64::MAX),
            None,
            "the first overflow is None, not a clamp"
        );
        assert_eq!(
            days(i64::MAX / 86_400),
            Some(SignedDuration::from_secs(i64::MAX / 86_400 * 86_400))
        );
        let end = Timestamp::MAX;
        assert_eq!(add_days(end, 1), None);
        assert_eq!(sub_days(Timestamp::MIN, 1), None);
        assert_eq!(
            add_days(Timestamp::UNIX_EPOCH, 1).map(|t| t.as_second()),
            Some(86_400)
        );
    }
}
