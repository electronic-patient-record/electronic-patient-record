//! Real-world names (CQ-19): per-part family matching and the extra given name. Each test is one
//! rule; the symmetry and totality properties in `properties.rs` cover both.

use electronic_patient_record_matcher::*;

fn person(family: &str, given: &[&str]) -> Patient {
    Patient {
        names: vec![HumanName {
            family: Some(family.into()),
            given: given.iter().map(|g| (*g).into()).collect(),
        }],
        birth_date: Some("1980-03-14".into()),
        ..Default::default()
    }
}

fn family_sim(a: &Patient, b: &Patient) -> f64 {
    MatchingEngine::default()
        .compare(a, b)
        .fields
        .iter()
        .find(|f| f.field == "family_name")
        .unwrap()
        .similarity
        .unwrap()
}

#[test]
fn a_compound_family_name_does_not_match_a_longer_name_it_is_a_prefix_of() {
    let a = person("Smith-Jones", &["Ann"]);
    let b = person("Smithson", &["Ann"]);
    assert!(family_sim(&a, &b) < 0.85, "{}", family_sim(&a, &b));
    assert_eq!(family_sim(&a, &b), family_sim(&b, &a));
    // Before the rule: the whole string scored as close as a typo.
    assert!(family_sim(&person("Smith", &["Ann"]), &b) > 0.85);
}

#[test]
fn a_family_name_that_gained_or_lost_a_part_still_matches_and_part_order_does_not_matter() {
    let (single, double) = (person("Smith", &["Ann"]), person("Smith-Jones", &["Ann"]));
    assert_eq!(family_sim(&single, &double), 1.0);
    assert_eq!(family_sim(&double, &single), 1.0);
    assert_eq!(
        family_sim(&double, &person("Jones Smith", &["Ann"])),
        1.0,
        "order of the parts"
    );
    assert!(family_sim(&double, &person("Smyth-Jones", &["Ann"])) > 0.9);
}

fn decide(a: &Patient, b: &Patient) -> MatchResult {
    let (ab, ba) = {
        let e = MatchingEngine::default();
        (e.compare(a, b), e.compare(b, a))
    };
    assert_eq!(ab.decision, ba.decision, "order of the records");
    assert_eq!(ab.conflicts, ba.conflicts);
    ab
}

#[test]
fn john_and_john_paul_are_reviewed_never_linked_automatically() {
    let same = decide(&person("Smith", &["John"]), &person("Smith", &["John"]));
    assert_eq!(same.decision, Decision::AutoLink);
    let extra = decide(
        &person("Smith", &["John"]),
        &person("Smith", &["John", "Paul"]),
    );
    assert_eq!(extra.decision, Decision::Review);
    assert!(
        extra
            .conflicts
            .iter()
            .any(|c| c.contains("given name the other lacks")),
        "{:?}",
        extra.conflicts
    );
}

#[test]
fn a_middle_name_that_is_on_both_records_or_is_an_initial_of_one_is_not_a_conflict() {
    let both = decide(
        &person("Smith", &["John", "Paul"]),
        &person("Smith", &["John", "Paul"]),
    );
    assert_eq!(both.decision, Decision::AutoLink);
    let initial = decide(
        &person("Smith", &["John", "P"]),
        &person("Smith", &["John", "Paul"]),
    );
    assert!(initial.conflicts.is_empty(), "{:?}", initial.conflicts);
    assert_eq!(initial.decision, Decision::AutoLink);
}
