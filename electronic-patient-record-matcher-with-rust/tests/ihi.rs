//! The Australian IHI as an opt-in deterministic identifier (OS-10, step c). It behaves as the NHS number
//! does, and does nothing unless `use_ihi` is on.

use electronic_patient_record_matcher::*;

/// Fifteen digits after the prefix, then the one check digit that makes it valid.
fn ihi(body: &str) -> String {
    (0..10)
        .map(|c| format!("800360{body}{c}"))
        .find(|s| is_valid_ihi(s))
        .unwrap()
}

fn person(family: &str, given: &str, dob: Option<&str>, ihis: &[&str]) -> Patient {
    Patient {
        names: vec![HumanName {
            family: Some(family.into()),
            given: vec![given.into()],
        }],
        birth_date: dob.map(Into::into),
        identifiers: ihis
            .iter()
            .map(|v| Identifier {
                system: IHI_SYSTEM.into(),
                value: (*v).into(),
            })
            .collect(),
        ..Default::default()
    }
}

fn on() -> MatchingEngine {
    MatchingEngine::new(MatchConfig {
        use_ihi: true,
        ..MatchConfig::default()
    })
}

#[test]
fn the_default_config_ignores_an_ihi() {
    assert!(!MatchConfig::default().use_ihi);
    let n = ihi("816669001");
    let a = person("Nguyen", "Lan", Some("1980-03-14"), &[&n]);
    let b = person("Okafor", "Tunde", Some("1975-01-01"), &[&n]);
    let r = MatchingEngine::default().compare(&a, &b);
    assert_ne!(r.decision, Decision::AutoLink);
    assert!(r.deterministic.is_none());
}

#[test]
fn a_shared_ihi_with_matching_demographics_links_and_says_why() {
    let n = ihi("816669001");
    let a = person("Nguyen", "Lan", Some("1980-03-14"), &[&n]);
    let b = person("Nguyen", "Lan", Some("1980-03-14"), &[&n]);
    let r = on().compare(&a, &b);
    assert_eq!(r.decision, Decision::AutoLink);
    assert_eq!(r.deterministic.as_deref(), Some("shared verified IHI"));
}

#[test]
fn a_shared_ihi_with_a_different_birth_date_goes_to_review() {
    let n = ihi("816669001");
    let a = person("Nguyen", "Lan", Some("1980-03-14"), &[&n]);
    let b = person("Nguyen", "Lan", Some("1981-03-14"), &[&n]);
    let r = on().compare(&a, &b);
    assert_eq!(r.decision, Decision::Review);
    assert!(
        r.conflict_codes
            .contains(&ConflictCode::SameIhiDifferentBirthDates)
    );
}

#[test]
fn a_shared_ihi_with_a_missing_birth_date_is_for_a_person() {
    let n = ihi("816669001");
    let a = person("Nguyen", "Lan", None, &[&n]);
    let b = person("Nguyen", "Lan", Some("1980-03-14"), &[&n]);
    let r = on().compare(&a, &b);
    assert_eq!(r.decision, Decision::Review);
}

#[test]
fn two_different_valid_ihis_forbid_a_link_however_alike_the_people_are() {
    let (n, m) = (ihi("816669001"), ihi("816669002"));
    assert_ne!(n, m);
    let a = person("Nguyen", "Lan", Some("1980-03-14"), &[&n]);
    let b = person("Nguyen", "Lan", Some("1980-03-14"), &[&m]);
    let r = on().compare(&a, &b);
    assert_eq!(r.decision, Decision::NonMatch);
    assert_eq!(r.conflict_codes, vec![ConflictCode::DifferentIhis]);
}

#[test]
fn an_invalid_ihi_is_ignored() {
    let n = ihi("816669001");
    let mut bad = n.clone();
    bad.replace_range(15.., if n.ends_with('0') { "1" } else { "0" });
    assert!(!is_valid_ihi(&bad));
    let a = person("Nguyen", "Lan", Some("1980-03-14"), &[&n]);
    let b = person("Nguyen", "Lan", Some("1980-03-14"), &[&bad]);
    let r = on().compare(&a, &b);
    assert!(r.deterministic.is_none());
    assert!(!r.conflict_codes.contains(&ConflictCode::DifferentIhis));
}

#[test]
fn a_config_saved_before_the_setting_existed_still_loads_with_it_off() {
    let json = serde_json::to_value(MatchConfig::default()).unwrap();
    let mut old = json.as_object().unwrap().clone();
    old.remove("use_ihi");
    let c: MatchConfig = serde_json::from_value(serde_json::Value::Object(old)).unwrap();
    assert!(!c.use_ihi);
}
