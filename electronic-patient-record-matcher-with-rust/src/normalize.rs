//! Text folding. NFKC and lowercase; diacritics are preserved, never stripped.

use unicode_normalization::UnicodeNormalization;

/// Fold a name part: NFKC, lowercase, letters only (hyphen and apostrophe
/// become a space and are collapsed).
pub fn fold_name(s: &str) -> String {
    let folded: String = s
        .nfkc()
        .flat_map(char::to_lowercase)
        .map(|c| if c.is_alphabetic() { c } else { ' ' })
        .collect();
    folded.split_whitespace().collect::<Vec<_>>().join(" ")
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
    Some(PartialDate { year, month, day })
}
