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

/// The fewest digits a phone number can have and still count as evidence of identity.
pub const MIN_PHONE_DIGITS: usize = 7;

/// Normalise a phone number towards E.164 digits (no `+`), or `""` when it is too short to be evidence.
///
/// `+44 7700 900123`, `0044 7700 900123` and `07700 900123` (with `default_code` 44) all give
/// `447700900123`; the trunk `0` written after the country code, `+44 (0)7700 900123`, is dropped.
/// A number with no `+`, no `00` and no leading `0` is kept as written, because its country is unknown.
pub fn normalize_phone(value: &str, default_code: u16) -> String {
    let digits: String = value.chars().filter(char::is_ascii_digit).collect();
    let code = default_code.to_string();
    let international = value.trim_start().starts_with('+');
    let full = if international {
        digits
    } else if let Some(rest) = digits.strip_prefix("00") {
        rest.to_string()
    } else if let Some(rest) = digits.strip_prefix('0') {
        format!("{code}{rest}")
    } else {
        return if digits.len() >= MIN_PHONE_DIGITS {
            digits
        } else {
            String::new()
        };
    };
    let full = match full.strip_prefix(code.as_str()) {
        Some(rest) if rest.starts_with('0') => format!("{code}{}", &rest[1..]),
        _ => full,
    };
    if full.len() >= MIN_PHONE_DIGITS {
        full
    } else {
        String::new()
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

/// American Soundex of a folded name, one code per word joined by a space, or `None` when a word has
/// no Latin letters to code (another script) so no claim is made about it. Applied after `transliterate`,
/// so `Müller` codes as `Mueller`. A coarse sound signal, never a similarity (CQ-19).
pub fn soundex(folded: &str) -> Option<String> {
    let code = |c: char| match c {
        'b' | 'f' | 'p' | 'v' => Some('1'),
        'c' | 'g' | 'j' | 'k' | 'q' | 's' | 'x' | 'z' => Some('2'),
        'd' | 't' => Some('3'),
        'l' => Some('4'),
        'm' | 'n' => Some('5'),
        'r' => Some('6'),
        _ => None,
    };
    let words: Vec<String> = transliterate(folded)
        .split(' ')
        .map(|w| {
            let letters: Vec<char> = w.chars().collect();
            let first = *letters.first()?;
            if !letters.iter().all(char::is_ascii_lowercase) {
                return None;
            }
            let mut out = String::from(first.to_ascii_uppercase());
            let mut last = code(first);
            for &c in &letters[1..] {
                let d = code(c);
                if let Some(digit) = d
                    && d != last
                {
                    out.push(digit);
                }
                // `h` and `w` do not separate two letters with the same code; a vowel does.
                if c != 'h' && c != 'w' {
                    last = d;
                }
                if out.len() == 4 {
                    break;
                }
            }
            while out.len() < 4 {
                out.push('0');
            }
            Some(out)
        })
        .collect::<Option<_>>()?;
    (!words.is_empty()).then(|| words.join(" "))
}

/// Groups of English given names that people use for one another. A name matches another in its group, and
/// nothing outside it: `alex` is in the groups of both `alexander` and `alexandra`, which are not in one group
/// and so never match each other. A short, cautious list, not a dictionary (CQ-19).
const NICKNAME_GROUPS: &[&[&str]] = &[
    &["william", "will", "bill", "billy", "willy", "liam"],
    &["robert", "rob", "bob", "bobby", "robbie", "bert"],
    &["richard", "rick", "dick", "rich", "richie"],
    &["james", "jim", "jimmy", "jamie"],
    &["john", "jack", "johnny", "jon"],
    &["thomas", "tom", "tommy"],
    &["edward", "ed", "eddie", "ted", "teddy", "ned"],
    &["margaret", "maggie", "meg", "peggy", "marge", "greta"],
    &[
        "elizabeth",
        "liz",
        "lizzie",
        "beth",
        "betty",
        "eliza",
        "libby",
        "betsy",
    ],
    &[
        "katherine",
        "catherine",
        "kate",
        "katie",
        "kathy",
        "cathy",
        "kitty",
        "kat",
    ],
    &["susan", "sue", "susie", "suzy"],
    &["jennifer", "jenny", "jen"],
    &["alexander", "alex", "sandy", "xander"],
    &["alexandra", "alex", "sandra", "sandy", "lexi"],
    &["christopher", "chris", "kit"],
    &["christine", "christina", "chris", "chrissie", "tina"],
    &["michael", "mike", "mick", "mickey"],
    &["anthony", "tony"],
    &["stephen", "steven", "steve"],
    &["patrick", "pat", "paddy"],
    &["patricia", "pat", "trish", "tricia"],
    &["daniel", "dan", "danny"],
    &["samuel", "sam", "sammy"],
    &["benjamin", "ben", "benny"],
    &["nicholas", "nick", "nicky"],
    &["matthew", "matt"],
    &["andrew", "andy", "drew"],
    &["joseph", "joe", "joey"],
    &["charles", "charlie", "chuck", "chas"],
    &["frederick", "fred", "freddie"],
    &["henry", "harry", "hal"],
    &["david", "dave", "davey"],
    &["deborah", "debra", "debbie", "deb"],
    &["rebecca", "becky", "becca"],
    &["victoria", "vicky", "vicki", "tori"],
    &["abigail", "abby", "gail"],
    &["jessica", "jess", "jessie"],
    &["mohammed", "mohammad", "muhammad", "mohamed", "mohd"],
];

/// Whether two folded given names are in one nickname group (and are not the same word).
pub fn are_nicknames(a: &str, b: &str) -> bool {
    a != b
        && NICKNAME_GROUPS
            .iter()
            .any(|g| g.contains(&a) && g.contains(&b))
}
