//! `MatchConfig::validate` (CQ-19): each refusal has a test, and the default is valid.

use electronic_patient_record_matcher::*;

#[test]
fn the_default_configuration_is_valid() {
    assert_eq!(MatchConfig::default().validate(), Ok(()));
    assert!(MatchingEngine::try_new(MatchConfig::default()).is_ok());
}

#[test]
fn a_negative_or_non_finite_weight_is_refused() {
    for bad in [-0.1, f64::NAN, f64::INFINITY] {
        let mut c = MatchConfig::default();
        c.weights.given = bad;
        assert_eq!(c.validate(), Err(ConfigError::InvalidWeight));
    }
}

#[test]
fn all_weights_zero_is_refused() {
    let c = MatchConfig {
        weights: Weights {
            family: 0.0,
            given: 0.0,
            birth_date: 0.0,
            sex: 0.0,
            postal_code: 0.0,
            telecom: 0.0,
        },
        ..Default::default()
    };
    assert_eq!(c.validate(), Err(ConfigError::NoWeight));
}

#[test]
fn a_threshold_outside_zero_to_one_or_nan_is_refused() {
    for bad in [-0.01, 1.01, f64::NAN] {
        let c = MatchConfig {
            min_family_for_auto: bad,
            ..Default::default()
        };
        assert_eq!(c.validate(), Err(ConfigError::ThresholdOutOfRange));
        let c = MatchConfig {
            min_evidence_weight: bad,
            ..Default::default()
        };
        assert_eq!(c.validate(), Err(ConfigError::ThresholdOutOfRange));
    }
}

#[test]
fn a_review_line_above_the_auto_link_line_is_refused() {
    let c = MatchConfig {
        review_threshold: 0.95,
        ..Default::default()
    };
    assert_eq!(c.validate(), Err(ConfigError::ThresholdsInverted));
    assert!(MatchingEngine::try_new(c).is_err());
}
