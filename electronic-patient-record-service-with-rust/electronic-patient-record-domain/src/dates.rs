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

#[cfg(test)]
mod tests {
    use super::*;

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
