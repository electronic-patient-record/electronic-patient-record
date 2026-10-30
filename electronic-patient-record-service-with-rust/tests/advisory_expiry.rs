//! Advisory ignores lapse (OS-12). Every `RUSTSEC` ignore in a `deny.toml` must carry
//! `expires YYYY-MM-DD` on its line; the test fails when a date is missing, has passed, or is more
//! than about six months ahead. An ignore is a decision to carry a known advisory; this makes the
//! decision be taken again on a date instead of being forgotten. It reads comments, so it proves a
//! date is written, not that the reason beside it is right.

use std::{fs, path::Path, time::SystemTime};

const ROOT: &str = env!("CARGO_MANIFEST_DIR");
const FILES: [&str; 2] = [
    "deny.toml",
    "../electronic-patient-record-matcher-with-rust/deny.toml",
];
const MAX_DAYS_AHEAD: i64 = 190;

/// Days since 1970-01-01 for a civil date (Howard Hinnant's algorithm).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn today() -> i64 {
    let secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("clock after 1970")
        .as_secs();
    i64::try_from(secs / 86_400).expect("fits")
}

/// `expires YYYY-MM-DD` in `line`, as days since the epoch.
fn expiry(line: &str) -> Option<i64> {
    let rest = line.split("expires ").nth(1)?;
    let date = rest.get(..10)?;
    let mut parts = date.split('-');
    let y: i64 = parts.next()?.parse().ok()?;
    let m: i64 = parts.next()?.parse().ok()?;
    let d: i64 = parts.next()?.parse().ok()?;
    ((1..=12).contains(&m) && (1..=31).contains(&d)).then(|| days_from_civil(y, m, d))
}

/// The ignore lines of an `[advisories]` table: lines that start with a quoted RUSTSEC id.
fn ignores(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|l| l.trim_start().starts_with("\"RUSTSEC-"))
        .collect()
}

fn problems(text: &str, today: i64) -> Vec<String> {
    ignores(text)
        .into_iter()
        .filter_map(|line| match expiry(line) {
            None => Some(format!("no `expires YYYY-MM-DD`: {}", line.trim())),
            Some(d) if d < today => Some(format!("expired: {}", line.trim())),
            Some(d) if d > today + MAX_DAYS_AHEAD => {
                Some(format!("expires too far ahead: {}", line.trim()))
            }
            Some(_) => None,
        })
        .collect()
}

#[test]
fn every_advisory_ignore_has_a_live_expiry() {
    let mut all = Vec::new();
    for file in FILES {
        let text = fs::read_to_string(Path::new(ROOT).join(file)).expect("deny.toml readable");
        for p in problems(&text, today()) {
            all.push(format!("{file}: {p}"));
        }
    }
    assert!(all.is_empty(), "{}", all.join("\n"));
}

#[test]
fn the_check_catches_a_missing_a_past_and_a_far_date() {
    let now = days_from_civil(2026, 10, 30);
    let text = r#"
ignore = [
    "RUSTSEC-2000-0001", # no date here
    "RUSTSEC-2000-0002", # expires 2026-10-29
    "RUSTSEC-2000-0003", # expires 2030-01-01
    "RUSTSEC-2000-0004", # fine; expires 2027-01-30
    "RUSTSEC-2000-0005", # expires 2026-13-40
]
"#;
    let p = problems(text, now);
    assert_eq!(p.len(), 4, "{p:?}");
    assert!(p[0].starts_with("no `expires") && p[0].contains("0001"));
    assert!(p[1].starts_with("expired") && p[1].contains("0002"));
    assert!(p[2].starts_with("expires too far") && p[2].contains("0003"));
    assert!(
        p[3].starts_with("no `expires") && p[3].contains("0005"),
        "a bad date is a missing date"
    );
}

#[test]
fn the_calendar_arithmetic_is_right() {
    assert_eq!(days_from_civil(1970, 1, 1), 0);
    assert_eq!(days_from_civil(2000, 3, 1), 11_017);
    assert_eq!(
        days_from_civil(2026, 10, 30) - days_from_civil(2026, 10, 29),
        1
    );
    assert_eq!(
        days_from_civil(2028, 3, 1) - days_from_civil(2028, 2, 28),
        2,
        "2028 is a leap year"
    );
}
