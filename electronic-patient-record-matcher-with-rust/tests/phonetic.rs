//! Phonetic signal (CQ-19): kept apart from string similarity. It never changes a score and never auto-links;
//! it only sends a pair with the same birth date, otherwise scored as unrelated, to Review.

use electronic_patient_record_matcher::normalize::soundex;
use electronic_patient_record_matcher::*;

/// A person who has moved and changed phone since the other record, so only names and birth date speak.
fn person(family: &str, given: &str, dob: &str, home: &str) -> Patient {
    Patient {
        names: vec![HumanName {
            family: Some(family.into()),
            given: vec![given.into()],
        }],
        birth_date: Some(dob.into()),
        addresses: vec![Address {
            postal_code: Some(home.into()),
        }],
        telecom: vec![Telecom {
            system: "phone".into(),
            value: format!("0770090{}", home.len() * 111),
        }],
        ..Default::default()
    }
}

fn pair(dob_b: &str) -> (Patient, Patient) {
    (
        person("Wu", "Li", "1985-06-02", "LS1 4AB"),
        person("Woo", "Lee", dob_b, "SW1A 1AAA"),
    )
}

#[test]
fn soundex_codes_match_the_standard_examples() {
    assert_eq!(soundex("robert").as_deref(), Some("R163"));
    assert_eq!(soundex("rupert").as_deref(), Some("R163"));
    assert_eq!(soundex("tymczak").as_deref(), Some("T522"));
    assert_eq!(soundex("pfister").as_deref(), Some("P236"));
    assert_eq!(soundex("ashcraft").as_deref(), Some("A261"));
    assert_eq!(soundex("lee").as_deref(), Some("L000"));
    assert_eq!(soundex("müller"), soundex("mueller"));
    assert_eq!(soundex("smith jones").as_deref(), Some("S530 J520"));
}

#[test]
fn another_script_makes_no_claim() {
    assert_eq!(soundex("कृष्ण"), None);
}

#[test]
fn sound_alike_names_with_the_same_birth_date_go_to_review_not_non_match() {
    let (a, b) = pair("1985-06-02");
    let r = MatchingEngine::default().compare(&a, &b);
    assert!(r.phonetic_match);
    assert_eq!(r.decision, Decision::Review, "{r:?}");
    assert!(r.conflicts.iter().any(|c| c.contains("sound alike")));
}

#[test]
fn a_different_birth_date_leaves_the_pair_a_non_match_and_the_score_is_the_string_score() {
    let (a, b) = pair("1990-01-01");
    let r = MatchingEngine::default().compare(&a, &b);
    assert!(r.phonetic_match);
    assert_eq!(r.decision, Decision::NonMatch, "{r:?}");
    assert!(r.conflicts.is_empty());
}

#[test]
fn the_signal_never_auto_links_and_is_symmetric() {
    let (a, b) = pair("1985-06-02");
    let e = MatchingEngine::default();
    assert_ne!(e.compare(&a, &b).decision, Decision::AutoLink);
    assert_eq!(e.compare(&a, &b), e.compare(&b, &a));
}

#[test]
fn identical_names_are_not_a_phonetic_match() {
    let a = person("Wu", "Li", "1985-06-02", "LS1 4AB");
    assert!(
        !MatchingEngine::default()
            .compare(&a, &a.clone())
            .phonetic_match
    );
}
