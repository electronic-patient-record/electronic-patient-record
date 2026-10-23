//! A `\p{M}`-aware fold (CQ-19): combining marks are part of a name, so Indic and Persian names are not split.

use electronic_patient_record_matcher::normalize::fold_name;
use electronic_patient_record_matcher::*;

#[test]
fn an_indic_name_with_virama_and_vowel_signs_stays_one_word() {
    // कृष्ण: ka + vowel sign vocalic r + ssa with virama + nna.
    let n = "कृष्ण";
    assert_eq!(fold_name(n), n);
    assert!(!fold_name(n).contains(' '));
}

#[test]
fn an_arabic_script_name_keeps_its_vowel_marks() {
    let n = "مُحَمَّد";
    assert_eq!(fold_name(n), n);
}

#[test]
fn a_mark_with_nothing_to_attach_to_is_dropped() {
    assert_eq!(fold_name("\u{0301}Ann"), "ann");
    assert_eq!(fold_name("Ann -\u{0301}Lee"), "ann lee");
}

#[test]
fn hyphen_and_apostrophe_still_separate_words() {
    assert_eq!(fold_name("O'Brien-Smith"), "o brien smith");
}

fn person(family: &str) -> Patient {
    Patient {
        names: vec![HumanName {
            family: Some(family.into()),
            given: vec!["Asha".into()],
        }],
        birth_date: Some("1980-03-14".into()),
        ..Default::default()
    }
}

#[test]
fn a_one_word_indic_family_name_matches_itself_and_not_a_different_name() {
    let e = MatchingEngine::default();
    let full = e.compare(&person("कृष्ण"), &person("कृष्ण"));
    let fam = |r: &MatchResult| {
        r.fields
            .iter()
            .find(|f| f.field == "family_name")
            .unwrap()
            .similarity
    };
    assert_eq!(fam(&full), Some(1.0));
    let other = e.compare(&person("कृष्ण"), &person("कमल"));
    assert!(fam(&other).unwrap() < 0.9);
}
