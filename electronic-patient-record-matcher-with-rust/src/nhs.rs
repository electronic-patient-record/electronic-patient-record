//! NHS number validation (modulus 11) and the identifier systems that count as
//! deterministic keys.

/// FHIR identifier system for the NHS number.
pub const NHS_NUMBER_SYSTEM: &str = "https://fhir.nhs.uk/Id/nhs-number";

/// True if `value` (spaces and hyphens allowed) is a well-formed NHS number:
/// ten digits whose modulus-11 check digit is correct, and not ten copies of one
/// digit. Every repeated-digit number passes modulus 11 and none is ever issued;
/// systems use them as placeholders, so they must never decide a link.
pub fn is_valid_nhs_number(value: &str) -> bool {
    let digits: Vec<u32> = value
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .map(|c| c.to_digit(10))
        .collect::<Option<Vec<_>>>()
        .unwrap_or_default();
    if digits.len() != 10 || is_placeholder_nhs_number(value) {
        return false;
    }
    let sum: u32 = digits[..9]
        .iter()
        .enumerate()
        .map(|(i, d)| d * (10 - i as u32))
        .sum();
    let check = match 11 - (sum % 11) {
        11 => 0,
        10 => return false,
        n => n,
    };
    check == digits[9]
}

/// True if `value` is ten copies of one digit (spaces and hyphens allowed): a
/// placeholder that passes modulus 11 but is never issued.
pub fn is_placeholder_nhs_number(value: &str) -> bool {
    let digits: Vec<char> = value
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .collect();
    digits.len() == 10 && digits[0].is_ascii_digit() && digits.iter().all(|c| *c == digits[0])
}

/// Canonical form of an identifier value: digits and letters only, lowercase.
pub fn canonical_identifier(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}
