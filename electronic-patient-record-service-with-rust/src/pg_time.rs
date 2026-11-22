//! jiff times as Postgres parameters and columns (DEP-1). sqlx 0.9 knows chrono and `time` but not jiff, so these
//! two wrappers carry a [`Timestamp`] as `timestamptz` and a [`Date`] as `date`, in Postgres's own binary encoding,
//! exactly as sqlx's chrono support did: microseconds, and days, since 2000-01-01. A query binds
//! `PgTimestamp(now)` where it bound a chrono time, and its SQL does not change.
//!
//! Postgres keeps microseconds: a time with nanoseconds is truncated toward the past on the way in, as chrono's was.
//! `infinity` and `-infinity` are refused on the way out, since no column here holds them.

// RCA-ENF-3: this module is converted, so the arithmetic lints are denied here; the rest of the crate follows.
#![deny(
    clippy::arithmetic_side_effects,
    clippy::default_numeric_fallback,
    clippy::integer_division,
    clippy::modulo_arithmetic,
    clippy::as_conversions,
    clippy::string_slice
)]

use jiff::civil::Date;
use jiff::{SignedDuration, Timestamp};
use sqlx::decode::Decode;
use sqlx::encode::{Encode, IsNull};
use sqlx::error::BoxDynError;
use sqlx::postgres::types::Oid;
use sqlx::postgres::{PgArgumentBuffer, PgTypeInfo, PgValueFormat, PgValueRef, Postgres};
use sqlx::types::Type;

/// 2000-01-01T00:00:00Z, the instant Postgres counts `timestamptz` from.
const PG_EPOCH_SECONDS: i64 = 946_684_800;

/// 2000-01-01, the day Postgres counts `date` from.
fn pg_epoch_date() -> Date {
    jiff::civil::date(2000, 1, 1)
}

/// A [`Timestamp`] bound to, or read from, a `timestamptz`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PgTimestamp(pub Timestamp);

/// A [`Date`] bound to, or read from, a `date`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PgDate(pub Date);

impl Type<Postgres> for PgTimestamp {
    fn type_info() -> PgTypeInfo {
        // 1184 is `timestamptz`, fixed in Postgres's catalogue.
        PgTypeInfo::with_oid(Oid(1184))
    }
}

impl Type<Postgres> for PgDate {
    fn type_info() -> PgTypeInfo {
        // 1082 is `date`.
        PgTypeInfo::with_oid(Oid(1082))
    }
}

/// Microseconds since the Postgres epoch, truncated toward the past (a floor, so a time before the epoch with
/// nanoseconds goes to the earlier microsecond).
///
/// # Errors
///
/// A time the column cannot hold: it is refused, not moved to the nearest bound (RCA-IN-2). Not reachable from a
/// `Timestamp`, whose range is far inside `timestamptz`'s.
fn to_pg_micros(t: Timestamp) -> Result<i64, BoxDynError> {
    let out_of_range = || BoxDynError::from("a time outside the range of timestamptz");
    let since = t
        .as_duration()
        .checked_sub(SignedDuration::from_secs(PG_EPOCH_SECONDS))
        .ok_or_else(out_of_range)?;
    let micros = since
        .as_nanos()
        .checked_div_euclid(NANOS_PER_MICRO)
        .ok_or_else(out_of_range)?;
    i64::try_from(micros).map_err(|_| out_of_range())
}

/// Nanoseconds in a microsecond, the unit Postgres keeps.
const NANOS_PER_MICRO: i128 = 1_000;

fn from_pg_micros(micros: i64) -> Result<Timestamp, BoxDynError> {
    if micros == i64::MAX || micros == i64::MIN {
        return Err("a timestamptz that is infinity".into());
    }
    let since = SignedDuration::from_micros(micros)
        .checked_add(SignedDuration::from_secs(PG_EPOCH_SECONDS))
        .ok_or("a timestamptz outside the range of a time")?;
    Ok(Timestamp::from_duration(since)?)
}

/// Postgres's text form, `2026-11-17 10:05:09.123456+00` (the offset in hours, or hours and minutes).
fn parse_pg_text(text: &str) -> Result<Timestamp, BoxDynError> {
    let iso = text.replacen(' ', "T", 1);
    // `+00` is an offset in hours only; RFC 3339 wants `+00:00`.
    let tail = iso
        .rfind(['+', '-'])
        .filter(|&i| i > 10)
        .and_then(|i| iso.get(i.checked_add(1)?..));
    let iso = match tail {
        Some(t) if t.len() == 2 => format!("{iso}:00"),
        _ => iso,
    };
    Ok(iso.parse::<Timestamp>()?)
}

impl Encode<'_, Postgres> for PgTimestamp {
    fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> Result<IsNull, BoxDynError> {
        Encode::<Postgres>::encode(to_pg_micros(self.0)?, buf)
    }

    fn size_hint(&self) -> usize {
        std::mem::size_of::<i64>()
    }
}

impl<'r> Decode<'r, Postgres> for PgTimestamp {
    fn decode(value: PgValueRef<'r>) -> Result<Self, BoxDynError> {
        match value.format() {
            PgValueFormat::Binary => {
                let micros: i64 = Decode::<Postgres>::decode(value)?;
                from_pg_micros(micros).map(PgTimestamp)
            }
            PgValueFormat::Text => parse_pg_text(value.as_str()?).map(PgTimestamp),
        }
    }
}

impl Encode<'_, Postgres> for PgDate {
    fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> Result<IsNull, BoxDynError> {
        let days = self.0.since(pg_epoch_date())?.get_days();
        Encode::<Postgres>::encode(days, buf)
    }

    fn size_hint(&self) -> usize {
        std::mem::size_of::<i32>()
    }
}

impl<'r> Decode<'r, Postgres> for PgDate {
    fn decode(value: PgValueRef<'r>) -> Result<Self, BoxDynError> {
        match value.format() {
            PgValueFormat::Binary => {
                let days: i32 = Decode::<Postgres>::decode(value)?;
                Ok(PgDate(
                    pg_epoch_date().checked_add(jiff::Span::new().days(i64::from(days)))?,
                ))
            }
            PgValueFormat::Text => Ok(PgDate(value.as_str()?.parse::<Date>()?)),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_ends_of_the_jiff_range_fit_timestamptz_and_round_trip_to_the_microsecond() {
        for t in [Timestamp::MIN, Timestamp::MAX] {
            let micros = to_pg_micros(t).unwrap();
            let back = from_pg_micros(micros).unwrap();
            let gap = t.as_duration().checked_sub(back.as_duration()).unwrap();
            assert!(
                gap >= SignedDuration::ZERO && gap < SignedDuration::from_micros(1),
                "{t}: truncated toward the past by under a microsecond"
            );
        }
    }

    use super::*;

    #[test]
    fn the_postgres_epoch_is_zero_and_microseconds_truncate_toward_the_past() {
        let epoch: Timestamp = "2000-01-01T00:00:00Z".parse().unwrap();
        assert_eq!(to_pg_micros(epoch).unwrap(), 0);
        let t: Timestamp = "2026-11-17T10:05:09.123456789Z".parse().unwrap();
        let micros = to_pg_micros(t).unwrap();
        assert_eq!(
            from_pg_micros(micros).unwrap(),
            "2026-11-17T10:05:09.123456Z".parse::<Timestamp>().unwrap()
        );
        // Before the epoch, a fraction of a microsecond goes back, not forward (chrono's rule).
        let before: Timestamp = "1999-12-31T23:59:59.999999500Z".parse().unwrap();
        assert_eq!(to_pg_micros(before).unwrap(), -1);
        assert_eq!(
            from_pg_micros(-1).unwrap(),
            "1999-12-31T23:59:59.999999Z".parse::<Timestamp>().unwrap()
        );
        assert!(from_pg_micros(i64::MAX).is_err(), "infinity");
        assert!(from_pg_micros(i64::MIN).is_err(), "-infinity");
    }

    #[test]
    fn postgres_text_times_read_with_an_hour_offset_or_a_full_one() {
        let want: Timestamp = "2026-11-17T10:05:09.123456Z".parse().unwrap();
        assert_eq!(
            parse_pg_text("2026-11-17 10:05:09.123456+00").unwrap(),
            want
        );
        assert_eq!(
            parse_pg_text("2026-11-17 11:05:09.123456+01:00").unwrap(),
            want
        );
        assert_eq!(
            parse_pg_text("2026-11-17 05:35:09.123456-04:30").unwrap(),
            want
        );
    }

    #[test]
    fn days_count_from_2000_01_01() {
        assert_eq!(
            pg_epoch_date().since(pg_epoch_date()).unwrap().get_days(),
            0_i32
        );
        let d = jiff::civil::date(2026, 11, 17);
        assert_eq!(d.since(pg_epoch_date()).unwrap().get_days(), 9_817_i32);
        let back = pg_epoch_date()
            .checked_add(jiff::Span::new().days(9_817_i32))
            .unwrap();
        assert_eq!(back, d);
    }
}
