//! Enum codes (CQ-19): every conflict has a stable, translatable code with its numbers, in the same order as
//! the English text, which is unchanged.

use electronic_patient_record_matcher::*;

fn person(family: &str, given: &[&str], dob: Option<&str>) -> Patient {
    Patient {
        names: vec![HumanName {
            family: Some(family.into()),
            given: given.iter().map(|g| (*g).into()).collect(),
        }],
        birth_date: dob.map(Into::into),
        ..Default::default()
    }
}

fn nhs(mut p: Patient, n: &str) -> Patient {
    p.identifiers.push(Identifier {
        system: NHS_NUMBER_SYSTEM.into(),
        value: n.into(),
    });
    p
}

fn codes(a: &Patient, b: &Patient) -> MatchResult {
    MatchingEngine::default().compare(a, b)
}

#[test]
fn a_missing_field_is_unavailable_with_its_name() {
    let r = codes(
        &person("Hughes", &["Ann"], None),
        &person("Hughes", &["Ann"], Some("1980-03-14")),
    );
    assert!(r.conflict_codes.contains(&ConflictCode::FieldUnavailable {
        field: GuardField::BirthDate
    }));
    assert!(
        r.conflicts
            .contains(&"birth date not available to confirm the link".to_string())
    );
}

#[test]
fn a_low_field_carries_its_similarity_and_minimum() {
    // Auto-link needs a score of 0.92, so lift the review line and the family guard to see the family code.
    let c = MatchConfig {
        auto_link_threshold: 0.5,
        review_threshold: 0.4,
        ..Default::default()
    };
    let r = MatchingEngine::new(c).compare(
        &person("Hughes", &["Ann"], Some("1980-03-14")),
        &person("Hewitt", &["Ann"], Some("1980-03-14")),
    );
    let found = r.conflict_codes.iter().find_map(|c| match c {
        ConflictCode::FieldBelowMinimum {
            field: GuardField::FamilyName,
            similarity,
            minimum,
        } => Some((*similarity, *minimum)),
        _ => None,
    });
    let (s, m) = found.expect("a family-name conflict code");
    assert!(s < m && m == 0.85, "{s} {m}");
}

#[test]
fn the_text_is_the_message_of_each_code_in_order() {
    let r = codes(
        &person("Hughes", &["John"], Some("1980-03-14")),
        &person("Hughes", &["John", "Paul"], Some("1980-03-14")),
    );
    assert!(!r.conflict_codes.is_empty());
    let text: Vec<String> = r.conflict_codes.iter().map(ConflictCode::message).collect();
    assert_eq!(text, r.conflicts);
    assert!(r.conflict_codes.contains(&ConflictCode::ExtraGivenName));
}

#[test]
fn nhs_number_conflicts_have_codes() {
    // 9434765919 and 9434765870 both pass modulus 11.
    let a = nhs(person("Hughes", &["Ann"], Some("1980-03-14")), "9434765919");
    let b = nhs(person("Hughes", &["Ann"], Some("1980-03-14")), "9434765870");
    let r = codes(&a, &b);
    assert_eq!(r.conflict_codes, vec![ConflictCode::DifferentNhsNumbers]);
    let c = nhs(person("Hughes", &["Ann"], Some("1981-03-14")), "9434765919");
    let r = codes(&a, &c);
    assert!(
        r.conflict_codes
            .contains(&ConflictCode::SameNhsDifferentBirthDates)
    );
}

#[test]
fn a_code_serialises_with_a_snake_case_tag_and_older_json_still_loads() {
    let json = serde_json::to_value(ConflictCode::FieldUnavailable {
        field: GuardField::GivenName,
    })
    .unwrap();
    assert_eq!(
        json,
        serde_json::json!({"code": "field_unavailable", "field": "given_name"})
    );
    let r = codes(&person("A", &["B"], None), &person("A", &["B"], None));
    let mut v = serde_json::to_value(&r).unwrap();
    v.as_object_mut().unwrap().remove("conflict_codes");
    let back: MatchResult = serde_json::from_value(v).unwrap();
    assert!(back.conflict_codes.is_empty());
}
