//! Transliteration (CQ-19): `Müller` and `Mueller` are one name; unlike names stay apart.

use electronic_patient_record_matcher::normalize::transliterate;
use electronic_patient_record_matcher::*;

fn person(family: &str, given: &str) -> Patient {
    Patient {
        names: vec![HumanName {
            family: Some(family.into()),
            given: vec![given.into()],
        }],
        birth_date: Some("1980-03-14".into()),
        ..Default::default()
    }
}

fn field(a: &Patient, b: &Patient, name: &str) -> f64 {
    let r = MatchingEngine::default().compare(a, b);
    r.fields
        .iter()
        .find(|f| f.field == name)
        .unwrap()
        .similarity
        .unwrap()
}

#[test]
fn transliterate_expands_german_letters_and_strips_latin_marks() {
    assert_eq!(transliterate("müller"), "mueller");
    assert_eq!(transliterate("strauß"), "strauss");
    assert_eq!(transliterate("josé"), "jose");
    assert_eq!(transliterate("çelik"), "celik");
}

#[test]
fn other_scripts_are_left_alone() {
    assert_eq!(transliterate("मोहन"), "मोहन");
    assert_eq!(transliterate("محمد"), "محمد");
}

#[test]
fn a_family_name_written_with_or_without_umlaut_matches_fully() {
    let (a, b) = (person("Müller", "Hans"), person("Mueller", "Hans"));
    assert!(field(&a, &b, "family_name") >= 0.99);
    assert!(field(&b, &a, "family_name") >= 0.99);
}

#[test]
fn a_given_name_written_with_or_without_an_accent_matches_fully() {
    let (a, b) = (person("Garcia", "José"), person("Garcia", "Jose"));
    assert!(field(&a, &b, "given_name") >= 0.99);
}

#[test]
fn unlike_names_do_not_gain_from_transliteration() {
    let (a, b) = (person("Müller", "Hans"), person("Miller", "Hans"));
    assert!(field(&a, &b, "family_name") < 0.95);
}
