//! Text folding. NFKC and lowercase; `fold_name` keeps diacritics; `transliterate` is applied only as a second comparison.

use unicode_normalization::{UnicodeNormalization, char::is_combining_mark};

/// Fold a name part: NFKC, lowercase, letters and the combining marks that follow a letter
/// (hyphen and apostrophe become a space and are collapsed). A mark is part of its letter, so a
/// virama or vowel sign does not split an Indic name; a mark with no letter before it is dropped.
pub fn fold_name(s: &str) -> String {
    let mut folded = String::with_capacity(s.len());
    let mut after_letter = false;
    for c in s.nfkc().flat_map(char::to_lowercase) {
        if c.is_alphabetic() {
            folded.push(c);
            after_letter = true;
        } else if after_letter && is_combining_mark(c) {
            folded.push(c);
        } else {
            folded.push(' ');
            after_letter = false;
        }
    }
    folded.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Transliterate a folded name to plain Latin letters for comparison only: `ä ö ü` become `ae oe ue`,
/// `ß` becomes `ss`, and a Latin letter with a mark (`é`, `ñ`) becomes its base letter. Characters of
/// other scripts, and their combining marks, are left alone (a `\p{M}` fold is a separate rule).
pub fn transliterate(folded: &str) -> String {
    let mut out = String::with_capacity(folded.len());
    for c in folded.chars() {
        match c {
            'ä' => out.push_str("ae"),
            'ö' => out.push_str("oe"),
            'ü' => out.push_str("ue"),
            'ß' => out.push_str("ss"),
            _ => {
                let mut d = std::iter::once(c).nfd();
                match (d.next(), c.is_ascii()) {
                    (Some(base), false) if base.is_ascii_alphabetic() => out.push(base),
                    _ => out.push(c),
                }
            }
        }
    }
    out
}

/// Fold a postcode: alphanumerics only, lowercase.
pub fn fold_postcode(s: &str) -> String {
    s.nfkc()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// Fold a phone number to its digits. Email is lowercased and trimmed.
pub fn fold_telecom(system: &str, value: &str) -> String {
    if system.eq_ignore_ascii_case("email") {
        value.trim().to_lowercase()
    } else {
        value.chars().filter(char::is_ascii_digit).collect()
    }
}

/// A parsed, possibly partial, ISO 8601 date.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartialDate {
    pub year: u16,
    pub month: Option<u8>,
    pub day: Option<u8>,
}

/// Parse `YYYY`, `YYYY-MM` or `YYYY-MM-DD`. Impossible months or days give `None`.
pub fn parse_date(s: &str) -> Option<PartialDate> {
    let mut parts = s.trim().split('-');
    let year: u16 = parts
        .next()?
        .parse()
        .ok()
        .filter(|y| (1000..=9999).contains(y))?;
    let month = match parts.next() {
        None => None,
        Some(m) => Some(m.parse::<u8>().ok().filter(|m| (1..=12).contains(m))?),
    };
    let day = match parts.next() {
        None => None,
        Some(d) => Some(d.parse::<u8>().ok().filter(|d| (1..=31).contains(d))?),
    };
    if parts.next().is_some() || (month.is_none() && day.is_some()) {
        return None;
    }
    if let (Some(m), Some(d)) = (month, day)
        && d > days_in_month(year, m)
    {
        return None;
    }
    Some(PartialDate { year, month, day })
}

fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        4 | 6 | 9 | 11 => 30,
        2 if year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400)) => {
            29
        }
        2 => 28,
        _ => 31,
    }
}
