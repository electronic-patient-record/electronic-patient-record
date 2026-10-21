//! SNOMED CT identifier checks (crate `snomed`).
//!
//! Only the *shape* of a code is checked here: a SNOMED CT identifier (SCTID) has a Verhoeff check
//! digit and a partition that says what kind of component it names. Whether the concept exists or
//! is active needs a terminology release or server, which this service does not have (plan OQ-7).

use snomed::prelude::{ComponentType, SctId};

/// The FHIR system URI for SNOMED CT.
pub const SNOMED_SYSTEM: &str = "http://snomed.info/sct";

/// Check a coding. A SNOMED CT code must be a valid SCTID naming a *concept*; a coding in any
/// other system is not checked (`Ok`) because nothing here knows its rules.
///
/// # Errors
///
/// A message saying what is wrong with a SNOMED CT code.
pub fn validate_coding(system: &str, code: &str) -> Result<(), String> {
    if system != SNOMED_SYSTEM {
        return Ok(());
    }
    let id = SctId::parse(code.trim()).map_err(|e| format!("SNOMED CT code {code:?}: {e}"))?;
    match id.component_type() {
        Some(ComponentType::Concept) => Ok(()),
        Some(other) => Err(format!(
            "SNOMED CT code {code:?} names a {other:?}, not a concept"
        )),
        None => Err(format!("SNOMED CT code {code:?} has an unknown partition")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_real_concept_id_passes() {
        // |Myocardial infarction|
        assert_eq!(validate_coding(SNOMED_SYSTEM, "22298006"), Ok(()));
        assert_eq!(validate_coding(SNOMED_SYSTEM, " 22298006 "), Ok(()));
    }

    #[test]
    fn a_wrong_check_digit_or_garbage_fails() {
        for bad in ["22298007", "22298", "abc", "", "0", "-22298006"] {
            assert!(
                validate_coding(SNOMED_SYSTEM, bad).is_err(),
                "{bad:?} should fail"
            );
        }
    }

    #[test]
    fn a_description_id_is_not_a_concept() {
        // Partition 01 names a description. Try each check digit; exactly one is valid.
        let description = (0..10u32)
            .map(|check| format!("123456701{check}"))
            .find(|c| {
                SctId::parse(c)
                    .is_ok_and(|id| id.component_type() == Some(ComponentType::Description))
            })
            .expect("one check digit makes a valid description id");
        let err = validate_coding(SNOMED_SYSTEM, &description).unwrap_err();
        assert!(err.contains("not a concept"), "{err}");
    }

    #[test]
    fn other_systems_are_not_judged() {
        assert_eq!(
            validate_coding("http://loinc.org", "not a snomed code"),
            Ok(())
        );
    }
}
