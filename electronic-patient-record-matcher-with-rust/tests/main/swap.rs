//! Given/family swap detection (CQ-19): a swapped pair is Review, never AutoLink, and never
//! beats the same pair written the right way round.

use electronic_patient_record_matcher::*;

fn person(family: &str, given: &str) -> Patient {
    Patient {
        names: vec![HumanName {
            family: Some(family.into()),
            given: vec![given.into()],
        }],
        birth_date: Some("1980-03-14".into()),
        sex: Some(AdministrativeSex::Female),
        ..Default::default()
    }
}

#[test]
fn swapped_names_go_to_review_with_a_conflict_not_nonmatch_or_autolink() {
    let r =
        MatchingEngine::default().compare(&person("Taylor", "Morgan"), &person("Morgan", "Taylor"));
    assert_eq!(r.decision, Decision::Review);
    assert!(
        r.conflicts.iter().any(|c| c.contains("swapped")),
        "{:?}",
        r.conflicts
    );
}

#[test]
fn the_swap_is_symmetric() {
    let (a, b) = (person("Taylor", "Morgan"), person("Morgan", "Taylor"));
    let e = MatchingEngine::default();
    assert_eq!(e.compare(&a, &b), e.compare(&b, &a));
}

#[test]
fn a_swapped_pair_scores_below_the_same_pair_written_correctly() {
    let e = MatchingEngine::default();
    let swapped = e.compare(&person("Taylor", "Morgan"), &person("Morgan", "Taylor"));
    let same = e.compare(&person("Taylor", "Morgan"), &person("Taylor", "Morgan"));
    assert!(swapped.score < same.score);
}

#[test]
fn unlike_names_are_not_read_as_swapped() {
    let r =
        MatchingEngine::default().compare(&person("Taylor", "Morgan"), &person("Okafor", "Chidi"));
    assert_eq!(r.decision, Decision::NonMatch);
    assert!(r.conflicts.is_empty());
}

#[test]
fn one_name_matching_the_other_way_round_is_not_a_swap() {
    // Same given name, different family name: the direct given match rules a swap out.
    let r =
        MatchingEngine::default().compare(&person("Taylor", "Morgan"), &person("Morgan", "Morgan"));
    assert!(!r.conflicts.iter().any(|c| c.contains("swapped")));
}
