use electronic_patient_record_matcher::*;

fn name(family: &str, given: &[&str]) -> HumanName {
    HumanName {
        family: Some(family.into()),
        given: given.iter().map(|s| s.to_string()).collect(),
    }
}

fn patient(family: &str, given: &[&str], dob: &str, sex: AdministrativeSex) -> Patient {
    Patient {
        names: vec![name(family, given)],
        birth_date: Some(dob.into()),
        sex: Some(sex),
        ..Default::default()
    }
}

fn with_nhs(mut p: Patient, n: &str) -> Patient {
    p.identifiers.push(Identifier {
        system: NHS_NUMBER_SYSTEM.into(),
        value: n.into(),
    });
    p
}

fn with_postcode(mut p: Patient, c: &str) -> Patient {
    p.addresses.push(Address {
        postal_code: Some(c.into()),
    });
    p
}

const M: AdministrativeSex = AdministrativeSex::Male;
const F: AdministrativeSex = AdministrativeSex::Female;

#[test]
fn nhs_number_check_digit() {
    assert!(is_valid_nhs_number("943 476 5919"));
    assert!(is_valid_nhs_number("9434765919"));
    assert!(!is_valid_nhs_number("9434765918"));
    assert!(!is_valid_nhs_number("123"));
    assert!(!is_valid_nhs_number("94347659AB"));
    // check digit computes to 10: never valid
    assert!(!is_valid_nhs_number("0000000100"));
}

#[test]
fn identical_records_auto_link() {
    let e = MatchingEngine::default();
    let a = with_postcode(patient("Smith", &["John"], "1980-03-14", M), "AB1 2CD");
    let b = with_postcode(patient("Smith", &["John"], "1980-03-14", M), "ab12cd");
    let r = e.compare(&a, &b);
    assert_eq!(r.decision, Decision::AutoLink, "{r:?}");
    assert!(r.score > 0.99);
    assert!(r.conflicts.is_empty());
}

#[test]
fn compare_is_symmetric_and_deterministic() {
    let e = MatchingEngine::default();
    let a = patient("Smyth", &["Jon", "Paul"], "1980-03-14", M);
    let b = patient("Smith", &["John"], "1980-14-03", M);
    assert_eq!(e.compare(&a, &b), e.compare(&b, &a));
    assert_eq!(e.compare(&a, &b), e.compare(&a, &b));
}

#[test]
fn twins_never_auto_link() {
    // Same family name, birth date, sex, address; different given names.
    let e = MatchingEngine::default();
    let a = with_postcode(patient("Jones", &["Oliver"], "2015-06-02", M), "CF10 1AA");
    let b = with_postcode(patient("Jones", &["Noah"], "2015-06-02", M), "CF10 1AA");
    let r = e.compare(&a, &b);
    assert_ne!(r.decision, Decision::AutoLink, "{r:?}");
}

#[test]
fn missing_given_name_blocks_auto_link() {
    let e = MatchingEngine::default();
    let a = with_postcode(patient("Jones", &["Oliver"], "2015-06-02", M), "CF10 1AA");
    let mut b = with_postcode(patient("Jones", &[], "2015-06-02", M), "CF10 1AA");
    b.names[0].given.clear();
    let r = e.compare(&a, &b);
    assert_ne!(r.decision, Decision::AutoLink, "{r:?}");
    assert_eq!(
        r.fields
            .iter()
            .find(|f| f.field == "given_name")
            .unwrap()
            .similarity,
        None
    );
}

#[test]
fn transposed_day_and_month_goes_to_review() {
    let e = MatchingEngine::default();
    let a = with_postcode(patient("Smith", &["John"], "1980-03-04", M), "AB1 2CD");
    let b = with_postcode(patient("Smith", &["John"], "1980-04-03", M), "AB1 2CD");
    let r = e.compare(&a, &b);
    assert_eq!(r.decision, Decision::Review, "{r:?}");
}

#[test]
fn different_people_do_not_match() {
    let e = MatchingEngine::default();
    let a = patient("Smith", &["John"], "1980-03-14", M);
    let b = patient("Okafor", &["Chiamaka"], "1992-11-30", F);
    assert_eq!(e.compare(&a, &b).decision, Decision::NonMatch);
}

#[test]
fn junior_and_senior_with_same_name_need_birth_date() {
    let e = MatchingEngine::default();
    let a = patient("Brown", &["James"], "1950-01-01", M);
    let b = patient("Brown", &["James"], "1981-07-19", M);
    assert_ne!(e.compare(&a, &b).decision, Decision::AutoLink);
}

#[test]
fn shared_valid_nhs_number_is_deterministic() {
    let e = MatchingEngine::default();
    let a = with_nhs(patient("Smith", &["Jane"], "1970-01-01", F), "943 476 5919");
    let b = with_nhs(patient("Smythe", &["Janet"], "1970-01-01", F), "9434765919");
    let r = e.compare(&a, &b);
    assert_eq!(r.decision, Decision::AutoLink);
    assert_eq!(r.score, 1.0);
    assert!(r.deterministic.is_some());
}

#[test]
fn shared_nhs_number_with_different_birth_dates_goes_to_review() {
    let e = MatchingEngine::default();
    let a = with_nhs(patient("Smith", &["Jane"], "1970-01-01", F), "9434765919");
    let b = with_nhs(patient("Smith", &["Jane"], "1985-09-09", F), "9434765919");
    let r = e.compare(&a, &b);
    assert_eq!(r.decision, Decision::Review);
    assert!(!r.conflicts.is_empty());
}

#[test]
fn different_valid_nhs_numbers_forbid_a_link() {
    let e = MatchingEngine::default();
    let a = with_nhs(patient("Smith", &["Jane"], "1970-01-01", F), "9434765919");
    let b = with_nhs(patient("Smith", &["Jane"], "1970-01-01", F), "9434765910");
    // 9434765910 has a wrong check digit, so it is ignored and the demographics decide.
    assert_ne!(e.compare(&a, &b).decision, Decision::NonMatch);
    let c = with_nhs(patient("Smith", &["Jane"], "1970-01-01", F), "4010232137");
    let r = e.compare(&a, &c);
    assert_eq!(r.decision, Decision::NonMatch, "{r:?}");
    assert!(r.conflicts.iter().any(|c| c.contains("NHS")));
}

#[test]
fn diacritics_are_preserved() {
    let e = MatchingEngine::default();
    let a = patient("Müller", &["Jürgen"], "1975-05-05", M);
    let b = patient("Muller", &["Jurgen"], "1975-05-05", M);
    let same = e.compare(&a, &a.clone());
    let accent_dropped = e.compare(&a, &b);
    assert!(same.score > accent_dropped.score);
}

#[test]
fn json_roundtrip() {
    let a = patient("Smith", &["John"], "1980-03-14", M);
    let json = serde_json::to_string(&a).unwrap();
    assert_eq!(serde_json::from_str::<Patient>(&json).unwrap(), a);
}
