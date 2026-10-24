//! Placeholder names (EC-2): `Unknown`, `UNK-001` and the like say nothing about who a person is, so the
//! matcher treats them as absent. Two unidentified patients never link, and an unidentified patient never
//! auto-links to a real record.

use electronic_patient_record_matcher::*;

fn person(family: &str, given: &str, dob: &str) -> Patient {
    Patient {
        names: vec![HumanName {
            family: Some(family.into()),
            given: vec![given.into()],
        }],
        birth_date: Some(dob.into()),
        sex: Some(AdministrativeSex::Male),
        ..Default::default()
    }
}

fn sim(a: &Patient, b: &Patient, field: &str) -> Option<f64> {
    MatchingEngine::default()
        .compare(a, b)
        .fields
        .into_iter()
        .find(|f| f.field == field)
        .unwrap()
        .similarity
}

#[test]
fn a_placeholder_name_is_absent_not_a_match() {
    for placeholder in [
        "Unknown",
        "UNKNOWN",
        "UNK-001",
        "unk 17",
        "Unidentified",
        "Anonymous",
    ] {
        let a = person(placeholder, "Unknown", "1975-04-02");
        let b = person(placeholder, "Unknown", "1975-04-02");
        assert_eq!(sim(&a, &b, "family_name"), None, "{placeholder}");
        assert_eq!(sim(&a, &b, "given_name"), None, "{placeholder}");
    }
}

#[test]
fn two_unidentified_patients_with_the_same_estimate_never_auto_link() {
    let a = person("UNK-001", "Unknown", "1975-04-02");
    let b = person("UNK-002", "Unknown", "1975-04-02");
    let m = MatchingEngine::default().compare(&a, &b);
    assert_ne!(m.decision, Decision::AutoLink, "{m:?}");
}

#[test]
fn an_unidentified_patient_does_not_auto_link_to_a_real_record() {
    let unknown = person("UNK-001", "Unknown", "1975-04-02");
    let real = person("Johnson", "Michael", "1975-04-02");
    let m = MatchingEngine::default().compare(&unknown, &real);
    assert_ne!(m.decision, Decision::AutoLink, "{m:?}");
    assert!(
        m.conflict_codes
            .iter()
            .any(|c| matches!(c, ConflictCode::FieldUnavailable { .. })),
        "{m:?}"
    );
}

#[test]
fn a_real_name_that_contains_a_placeholder_word_still_counts() {
    // Only the whole name part is a placeholder; `Unkind` and `Doe` are names.
    let a = person("Doe", "Unkind", "1975-04-02");
    assert_eq!(sim(&a, &a.clone(), "family_name"), Some(1.0));
    assert_eq!(sim(&a, &a.clone(), "given_name"), Some(1.0));
}

#[test]
fn comparison_with_a_placeholder_is_symmetric() {
    let e = MatchingEngine::default();
    let a = person("UNK-001", "Unknown", "1975-04-02");
    let b = person("Johnson", "Michael", "1975-04-02");
    let (ab, ba) = (e.compare(&a, &b), e.compare(&b, &a));
    assert_eq!(ab.decision, ba.decision);
    assert!((ab.score - ba.score).abs() < f64::EPSILON);
}
