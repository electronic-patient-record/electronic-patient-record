//! National health number validation (modulus 11) and the identifier systems that count as
//! deterministic keys.

/// FHIR identifier system for the national health number. The value is the URI real records carry for
/// it, kept as it is so those records are recognised; it is the one place the matcher spells it out.
pub const NATIONAL_NUMBER_SYSTEM: &str = "https://fhir.nhs.uk/Id/nhs-number";

/// True if `value` (spaces and hyphens allowed) is a well-formed national health number:
/// ten digits whose modulus-11 check digit is correct, and not ten copies of one
/// digit. Every repeated-digit number passes modulus 11 and none is ever issued;
/// systems use them as placeholders, so they must never decide a link.
pub fn is_valid_national_number(value: &str) -> bool {
    let Some(digits) = value
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .map(|c| c.to_digit(10))
        .collect::<Option<Vec<u32>>>()
    else {
        return false;
    };
    if digits.len() != 10 || is_placeholder_national_number(value) {
        return false;
    }
    // The first nine digits weigh 10, 9, … 2.
    let Some(sum) = digits[..9]
        .iter()
        .zip((2_u32..=10).rev())
        .try_fold(0_u32, |acc, (d, weight)| {
            acc.checked_add(d.checked_mul(weight)?)
        })
    else {
        return false;
    };
    let Some(remainder) = sum.checked_rem(11) else {
        return false;
    };
    let check = match 11_u32.checked_sub(remainder) {
        Some(11) => 0,
        Some(10) | None => return false,
        Some(n) => n,
    };
    check == digits[9]
}

/// True if `value` is ten copies of one digit (spaces and hyphens allowed): a
/// placeholder that passes modulus 11 but is never issued.
pub fn is_placeholder_national_number(value: &str) -> bool {
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
