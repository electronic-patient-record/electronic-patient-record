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

#[test]
fn equal_length_given_names_are_symmetric() {
    // Regression (CQ-1): with given-name lists of equal length the average ran from `a` only.
    let e = MatchingEngine::default();
    let a = patient("Smith", &["John", "J"], "1980-03-14", M);
    let b = patient("Smith", &["John", "Smith"], "1980-03-14", M);
    let (ab, ba) = (e.compare(&a, &b), e.compare(&b, &a));
    assert_eq!(ab, ba);
    assert_ne!(ab.decision, Decision::AutoLink, "{ab:?}");
}

#[test]
fn compare_is_symmetric_over_a_grid() {
    // Every ordered pair from a grid of name, date and sex variants, with valid and partial dates.
    let e = MatchingEngine::default();
    let families = ["Smith", "Smyth", "Smith-Jones", "", "Müller"];
    let givens: [&[&str]; 7] = [
        &["John"],
        &["John", "J"],
        &["John", "Smith"],
        &["J", "John"],
        &["Jon", "Paul"],
        &[],
        &["-"],
    ];
    let dates = ["1980-03-14", "1980-14-03", "1980-03", "1980", "1980-03-15"];
    let mut people = Vec::new();
    for f in families {
        for g in givens {
            for d in dates {
                for s in [M, F] {
                    people.push(patient(f, g, d, s));
                }
            }
        }
    }
    for a in &people {
        for b in &people {
            assert_eq!(e.compare(a, b), e.compare(b, a), "{a:?} vs {b:?}");
        }
    }
}

#[test]
fn repeated_digit_nhs_numbers_are_not_valid() {
    // Regression (CQ-2): every repeated-digit number passes modulus 11, and none is ever issued.
    for d in 0..=9 {
        let n = d.to_string().repeat(10);
        assert!(!is_valid_nhs_number(&n), "{n}");
        assert!(is_placeholder_nhs_number(&n), "{n}");
    }
    assert!(!is_placeholder_nhs_number("9434765919"));
    assert!(!is_valid_nhs_number("000 000 0000"));
    // The 999 test range stays usable for demo data.
    assert!(is_valid_nhs_number("9990000018"));
}

#[test]
fn shared_placeholder_nhs_number_does_not_decide() {
    let e = MatchingEngine::default();
    let a = with_nhs(patient("Smith", &["John"], "1980-03-14", M), "0000000000");
    let b = with_nhs(
        patient("Okafor", &["Chiamaka"], "1980-03-14", M),
        "0000000000",
    );
    let r = e.compare(&a, &b);
    assert_eq!(r.deterministic, None, "{r:?}");
    assert_ne!(r.decision, Decision::AutoLink, "{r:?}");
}

#[test]
fn shared_nhs_number_with_different_names_goes_to_review() {
    let e = MatchingEngine::default();
    let a = with_nhs(patient("Smith", &["John"], "1980-03-14", M), "9434765919");
    let b = with_nhs(
        patient("Okafor", &["Chiamaka"], "1980-03-14", M),
        "9434765919",
    );
    let r = e.compare(&a, &b);
    assert_eq!(r.decision, Decision::Review, "{r:?}");
    assert!(
        r.conflicts.iter().any(|c| c.contains("different names")),
        "{r:?}"
    );
}

#[test]
fn shared_nhs_number_with_a_changed_family_name_still_links() {
    // Marriage: the family name changes, the given name does not.
    let e = MatchingEngine::default();
    let a = with_nhs(patient("Smith", &["Mary"], "1980-03-14", F), "9434765919");
    let b = with_nhs(patient("Okafor", &["Mary"], "1980-03-14", F), "9434765919");
    assert_eq!(e.compare(&a, &b).decision, Decision::AutoLink);
}

#[test]
fn shared_nhs_number_with_any_different_full_birth_date_goes_to_review() {
    // Regression (CQ-3): spec §4 says any two different full dates go to Review; a near date did not.
    let e = MatchingEngine::default();
    for other in ["1980-01-28", "1980-02-01", "1981-01-01", "1980-10-10"] {
        let a = with_nhs(patient("Smith", &["John"], "1980-01-01", M), "9434765919");
        let b = with_nhs(patient("Smith", &["John"], other, M), "9434765919");
        let r = e.compare(&a, &b);
        assert_eq!(r.decision, Decision::Review, "{other}: {r:?}");
        assert!(
            r.conflicts
                .iter()
                .any(|c| c.contains("different birth dates")),
            "{r:?}"
        );
    }
}

fn birth_date_score(x: &str, y: &str) -> Option<f64> {
    let e = MatchingEngine::default();
    let r = e.compare(
        &patient("Smith", &["John"], x, M),
        &patient("Smith", &["John"], y, M),
    );
    r.fields
        .into_iter()
        .find(|f| f.field == "birth_date")
        .unwrap()
        .similarity
}

#[test]
fn birth_date_typo_credit_needs_a_one_digit_slip() {
    // Regression (CQ-4): any different day, month or year was scored as a typo (0.6).
    for (x, y) in [
        ("1980-03-14", "1980-03-15"), // day, one digit
        ("1980-03-12", "1980-03-21"), // day, digits swapped
        ("1980-03-14", "1980-08-14"), // month, one digit
        ("1980-03-14", "1981-03-14"), // year, one digit
        ("1980-03-14", "1908-03-14"), // year, adjacent digits swapped
    ] {
        assert_eq!(birth_date_score(x, y), Some(0.6), "{x} vs {y}");
    }
    for (x, y) in [
        ("1980-03-14", "1980-03-27"), // day, both digits
        ("1950-06-02", "2010-06-02"), // parent and child sharing a birthday
        ("1980-03-14", "1980-12-14"), // month, both digits
    ] {
        assert_eq!(birth_date_score(x, y), Some(0.0), "{x} vs {y}");
    }
    // Day and month swapped keeps its own credit.
    assert_eq!(birth_date_score("1980-03-04", "1980-04-03"), Some(0.8));
}

#[test]
fn impossible_dates_are_absent() {
    for bad in ["2020-02-31", "2021-02-29", "1980-04-31", "1900-02-29"] {
        assert_eq!(birth_date_score("1980-03-14", bad), None, "{bad}");
    }
    // Leap days that exist.
    assert_eq!(birth_date_score("2020-02-29", "2020-02-29"), Some(1.0));
    assert_eq!(birth_date_score("2000-02-29", "2000-02-29"), Some(1.0));
}
