//! Nicknames (CQ-19): `Bill` is `William`, but a nickname never auto-links on the name alone.

use electronic_patient_record_matcher::*;

fn person(given: &str) -> Patient {
    Patient {
        names: vec![HumanName {
            family: Some("Hughes".into()),
            given: vec![given.into()],
        }],
        birth_date: Some("1980-03-14".into()),
        sex: Some(AdministrativeSex::Male),
        ..Default::default()
    }
}

fn given(a: &str, b: &str) -> f64 {
    let r = MatchingEngine::default().compare(&person(a), &person(b));
    r.fields
        .iter()
        .find(|f| f.field == "given_name")
        .unwrap()
        .similarity
        .unwrap()
}

#[test]
fn a_nickname_matches_its_formal_name_both_ways() {
    for (a, b) in [
        ("Bill", "William"),
        ("Liz", "Elizabeth"),
        ("Bob", "Robert"),
        ("Bill", "Will"),
    ] {
        assert!(given(a, b) >= 0.8, "{a} {b}");
        assert_eq!(given(a, b), given(b, a));
    }
}

#[test]
fn a_nickname_goes_to_review_never_auto_link() {
    let r = MatchingEngine::default().compare(&person("Bill"), &person("William"));
    assert_eq!(r.decision, Decision::Review, "{r:?}");
    assert!(
        r.conflicts.iter().any(|c| c.contains("given name")),
        "{:?}",
        r.conflicts
    );
}

#[test]
fn a_shared_short_form_does_not_join_two_different_names() {
    // Alex is short for both, but Alexander and Alexandra are not one name.
    assert!(given("Alexander", "Alexandra") < 0.97);
    assert!(given("Alex", "Alexander") >= 0.8);
    assert!(given("Alex", "Alexandra") >= 0.8);
    assert!(!are_nicknames_pub("alexander", "alexandra"));
}

fn are_nicknames_pub(a: &str, b: &str) -> bool {
    electronic_patient_record_matcher::normalize::are_nicknames(a, b)
}

#[test]
fn unrelated_and_identical_names_are_unchanged() {
    assert!(!are_nicknames_pub("bill", "bob"));
    assert!(!are_nicknames_pub("william", "william"));
    assert!(given("Bill", "Bob") < 0.8);
    assert_eq!(given("William", "William"), 1.0);
}
