//! Calendar days as people write them: a birth date of a year, a month or a day.

/// Whether `d` is a plausible birth date written `YYYY`, `YYYY-MM` or `YYYY-MM-DD`, from the year 1800.
pub fn valid_birth_date(d: &str) -> bool {
    let parts: Vec<&str> = d.split('-').collect();
    let num = |s: &str, len: usize| {
        (s.len() == len && s.bytes().all(|b| b.is_ascii_digit()))
            .then(|| s.parse::<u32>().ok())
            .flatten()
    };
    match parts.as_slice() {
        [y] => num(y, 4).is_some_and(|y| y >= 1800),
        [y, m] => {
            num(y, 4).is_some_and(|y| y >= 1800) && num(m, 2).is_some_and(|m| (1..=12).contains(&m))
        }
        [y, m, day] => {
            let (Some(y), Some(m), Some(day)) = (num(y, 4), num(m, 2), num(day, 2)) else {
                return false;
            };
            let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
            let max = match m {
                1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
                4 | 6 | 9 | 11 => 30,
                2 => {
                    if leap {
                        29_u32
                    } else {
                        28_u32
                    }
                }
                _ => return false,
            };
            y >= 1800 && (1..=max).contains(&day)
        }
        _ => false,
    }
}

/// How two partial dates (`YYYY`, `YYYY-MM` or `YYYY-MM-DD`, each already checked by [`valid_birth_date`]) are ordered,
/// or `None` when that is not established: `2020` and `2020-05` may be either way round. A caller must not read `None`
/// as equal, or as in order.
#[must_use]
pub fn partial_order(a: &str, b: &str) -> Option<std::cmp::Ordering> {
    // Every part is zero-padded to a fixed width, so the first byte that differs decides.
    for (x, y) in a.bytes().zip(b.bytes()) {
        if x != y {
            return Some(x.cmp(&y));
        }
    }
    (a.len() == b.len()).then_some(std::cmp::Ordering::Equal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_dates_are_ordered_only_where_that_is_established() {
        use std::cmp::Ordering::{Equal, Greater, Less};
        assert_eq!(partial_order("2019", "2020"), Some(Less));
        assert_eq!(partial_order("2020-05", "2019-12-31"), Some(Greater));
        assert_eq!(partial_order("2020-05-17", "2020-05-17"), Some(Equal));
        assert_eq!(partial_order("2020", "2020"), Some(Equal));
        for (a, b) in [("2020", "2020-05"), ("2020-05-17", "2020-05")] {
            assert_eq!(partial_order(a, b), None, "{a} {b}");
            assert_eq!(partial_order(b, a), None, "{b} {a}");
        }
    }

    #[test]
    fn a_year_a_month_or_a_real_day_from_1800() {
        for good in ["1984", "1984-02", "1984-02-29", "2000-02-29", "1800-01-01"] {
            assert!(valid_birth_date(good), "{good}");
        }
        for bad in [
            "1799",
            "1900-02-29",
            "1984-13",
            "1984-2",
            "84",
            "1984-02-30",
            "1984-02-29-1",
            "",
        ] {
            assert!(!valid_birth_date(bad), "{bad}");
        }
    }
}
