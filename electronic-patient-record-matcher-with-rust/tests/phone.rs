//! E.164 phone normalisation (CQ-19): one number written three ways is one number.

use electronic_patient_record_matcher::normalize::normalize_phone;
use electronic_patient_record_matcher::*;

#[test]
fn national_international_and_00_forms_agree() {
    for v in [
        "+44 7700 900123",
        "0044 7700 900123",
        "07700 900123",
        "+44 (0)7700 900123",
        "(07700) 900-123",
    ] {
        assert_eq!(normalize_phone(v, 44), "447700900123", "{v}");
    }
}

#[test]
fn a_different_country_code_is_a_different_number() {
    assert_ne!(
        normalize_phone("+353 7700 900123", 44),
        normalize_phone("07700 900123", 44)
    );
}

#[test]
fn a_number_with_no_prefix_is_kept_as_written_and_a_short_one_is_not_evidence() {
    assert_eq!(normalize_phone("7700 900123", 44), "7700900123");
    assert_eq!(normalize_phone("999", 44), "");
    assert_eq!(normalize_phone("+44 12", 44), "");
}

fn with_phone(v: &str) -> Patient {
    Patient {
        names: vec![HumanName {
            family: Some("Ng".into()),
            given: vec!["Ann".into()],
        }],
        telecom: vec![Telecom {
            system: "phone".into(),
            value: v.into(),
        }],
        ..Default::default()
    }
}

fn telecom(a: &Patient, b: &Patient, code: u16) -> Option<f64> {
    let e = MatchingEngine::new(MatchConfig {
        default_calling_code: code,
        ..Default::default()
    });
    e.compare(a, b)
        .fields
        .iter()
        .find(|f| f.field == "telecom")
        .unwrap()
        .similarity
}

#[test]
fn the_engine_matches_a_national_number_to_its_international_form() {
    let (a, b) = (with_phone("07700 900123"), with_phone("+44 7700 900123"));
    assert_eq!(telecom(&a, &b, 44), Some(1.0));
    assert_eq!(telecom(&b, &a, 44), Some(1.0));
}

#[test]
fn the_configured_calling_code_decides_what_a_leading_zero_means() {
    let (a, b) = (with_phone("07700 900123"), with_phone("+44 7700 900123"));
    assert_eq!(telecom(&a, &b, 353), Some(0.0));
}

#[test]
fn a_number_too_short_to_be_evidence_is_absent_not_a_match() {
    let (a, b) = (with_phone("999"), with_phone("999"));
    assert_eq!(telecom(&a, &b, 44), None);
}

#[test]
fn a_calling_code_outside_1_to_999_is_refused() {
    for bad in [0, 1000] {
        let c = MatchConfig {
            default_calling_code: bad,
            ..Default::default()
        };
        assert_eq!(c.validate(), Err(ConfigError::InvalidCallingCode));
    }
}
