//! Australian Individual Healthcare Identifier (IHI) validation: sixteen digits, the prefix `800360`, and a
//! correct Luhn check digit. The rule is opt-in (`MatchConfig::use_ihi`).

/// FHIR identifier system for the IHI.
pub const IHI_SYSTEM: &str = "http://ns.electronichealth.net.au/id/hi/ihi/1.0";

/// True if `value` (spaces and hyphens allowed) is a well-formed IHI.
pub fn is_valid_ihi(value: &str) -> bool {
    let digits: Vec<u32> = value
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .map(|c| c.to_digit(10))
        .collect::<Option<Vec<_>>>()
        .unwrap_or_default();
    if digits.len() != 16 || digits[..6] != [8, 0, 0, 3, 6, 0] {
        return false;
    }
    let sum: u32 = digits
        .iter()
        .rev()
        .enumerate()
        .map(|(i, d)| {
            if i % 2 == 1 {
                let x = d * 2;
                if x > 9 { x - 9 } else { x }
            } else {
                *d
            }
        })
        .sum();
    sum.is_multiple_of(10)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Complete `prefix` (fifteen digits) with its Luhn check digit.
    fn with_check(prefix: &str) -> String {
        (0..10)
            .map(|c| format!("{prefix}{c}"))
            .find(|s| is_valid_ihi(s))
            .unwrap()
    }

    #[test]
    fn one_check_digit_in_ten_completes_a_number_and_spacing_is_allowed() {
        let n = with_check("800360816669001");
        assert_eq!(n.len(), 16);
        assert!(is_valid_ihi(&n));
        let spaced = format!("{} {} {} {}", &n[..4], &n[4..8], &n[8..12], &n[12..]);
        assert!(is_valid_ihi(&spaced));
        let wrong = (0..10)
            .map(|c| format!("800360816669001{c}"))
            .filter(|s| is_valid_ihi(s))
            .count();
        assert_eq!(wrong, 1);
    }

    #[test]
    fn a_wrong_prefix_length_or_character_is_not_an_ihi() {
        let n = with_check("800360816669001");
        assert!(!is_valid_ihi(&n[..15]));
        assert!(!is_valid_ihi(&format!("{n}0")));
        assert!((0..10).all(|c| !is_valid_ihi(&format!("800361816669001{c}"))));
        assert!(!is_valid_ihi(&format!("{}x", &n[..15])));
        assert!(!is_valid_ihi(""));
    }
}
