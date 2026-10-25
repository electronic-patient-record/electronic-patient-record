//! Field-level pins found by mutation testing (CQ-30): sex, postcode, telephone and email, the
//! precision rules for birth dates, the initial rule for given names, the evidence floor and the
//! config accessor. Each assertion kills at least one mutant `cargo mutants` reported as missed.

use electronic_patient_record_matcher::*;

fn sim(a: &Patient, b: &Patient, field: &str) -> Option<f64> {
    let m = MatchingEngine::default().compare(a, b);
    m.fields
        .iter()
        .find(|f| f.field == field)
        .unwrap()
        .similarity
}

fn dob(d: &str) -> Patient {
    Patient {
        birth_date: Some(d.into()),
        ..Default::default()
    }
}

fn name(family: &str, given: &str) -> Patient {
    Patient {
        names: vec![HumanName {
            family: Some(family.into()),
            given: vec![given.into()],
        }],
        ..Default::default()
    }
}

fn sex(s: Option<AdministrativeSex>) -> Patient {
    Patient {
        sex: s,
        ..Default::default()
    }
}

fn post(codes: &[&str]) -> Patient {
    Patient {
        addresses: codes
            .iter()
            .map(|c| Address {
                postal_code: Some((*c).into()),
            })
            .collect(),
        ..Default::default()
    }
}

fn tel(items: &[(&str, &str)]) -> Patient {
    Patient {
        telecom: items
            .iter()
            .map(|(s, v)| Telecom {
                system: (*s).into(),
                value: (*v).into(),
            })
            .collect(),
        ..Default::default()
    }
}

#[test]
fn sex_agrees_disagrees_or_is_absent() {
    use AdministrativeSex::*;
    assert_eq!(
        sim(&sex(Some(Female)), &sex(Some(Female)), "sex"),
        Some(1.0)
    );
    assert_eq!(sim(&sex(Some(Female)), &sex(Some(Male)), "sex"), Some(0.0));
    assert_eq!(
        sim(&sex(Some(Unknown)), &sex(Some(Unknown)), "sex"),
        None,
        "unknown is not agreement"
    );
    assert_eq!(sim(&sex(Some(Female)), &sex(Some(Unknown)), "sex"), None);
    assert_eq!(sim(&sex(Some(Female)), &sex(None), "sex"), None);
}

#[test]
fn postcodes_compare_folded_and_blank_ones_do_not_count() {
    assert_eq!(
        sim(&post(&["LS1 4AB"]), &post(&["ls14ab"]), "postal_code"),
        Some(1.0)
    );
    assert_eq!(
        sim(&post(&["LS1 4AB"]), &post(&["SW1A 1AA"]), "postal_code"),
        Some(0.0)
    );
    assert_eq!(
        sim(
            &post(&["LS1 4AB", "SW1A 1AA"]),
            &post(&["sw1a1aa"]),
            "postal_code"
        ),
        Some(1.0)
    );
    assert_eq!(
        sim(&post(&[""]), &post(&["LS1 4AB"]), "postal_code"),
        None,
        "a blank code is absent"
    );
    assert_eq!(sim(&post(&[" "]), &post(&[" "]), "postal_code"), None);
    assert_eq!(sim(&post(&[]), &post(&["LS1 4AB"]), "postal_code"), None);
    assert_eq!(sim(&post(&["LS1 4AB"]), &post(&[]), "postal_code"), None);
    let none = Patient {
        addresses: vec![Address { postal_code: None }],
        ..Default::default()
    };
    assert_eq!(sim(&none, &post(&["LS1 4AB"]), "postal_code"), None);
}

#[test]
fn telephone_and_email_compare_folded_and_blank_ones_do_not_count() {
    let a = tel(&[("email", "Ann@Example.com")]);
    assert_eq!(
        sim(&a, &tel(&[("email", "ann@example.com")]), "telecom"),
        Some(1.0)
    );
    assert_eq!(
        sim(&a, &tel(&[("email", "bob@example.com")]), "telecom"),
        Some(0.0)
    );
    let two = tel(&[("phone", "01632 960001"), ("email", "x@y.z")]);
    assert_eq!(sim(&two, &tel(&[("email", "X@Y.Z")]), "telecom"), Some(1.0));
    assert_eq!(sim(&tel(&[("phone", "")]), &a, "telecom"), None);
    assert_eq!(
        sim(&tel(&[("phone", " ")]), &tel(&[("phone", " ")]), "telecom"),
        None
    );
    assert_eq!(sim(&tel(&[]), &a, "telecom"), None);
    assert_eq!(sim(&a, &tel(&[]), "telecom"), None);
}

#[test]
fn birth_dates_compare_at_the_precision_both_carry() {
    let s = |x: &str, y: &str| sim(&dob(x), &dob(y), "birth_date");
    assert_eq!(s("1980-03-14", "1980-03-14"), Some(1.0));
    assert_eq!(s("1980-03", "1980-03"), Some(0.8));
    assert_eq!(s("1980", "1980"), Some(0.6));
    // Same year, month known on both, day on one side only.
    assert_eq!(s("1980-03", "1980-03-14"), Some(0.8));
    assert_eq!(s("1980-03", "1980-04-14"), Some(0.0));
    assert_eq!(s("1980-03-14", "1980-03"), Some(0.8));
    assert_eq!(s("1980-03-14", "1980-04"), Some(0.0));
    // Year only on one side.
    assert_eq!(s("1980", "1980-03-14"), Some(0.6));
    assert_eq!(s("1980-03-14", "1980"), Some(0.6));
    // Keying slips, and swapped day and month.
    assert_eq!(s("1980-03-14", "1980-03-15"), Some(0.6));
    assert_eq!(s("1980-03-14", "1980-04-14"), Some(0.6));
    assert_eq!(s("1980-03-14", "1981-03-14"), Some(0.6));
    assert_eq!(s("1980-03-04", "1980-04-03"), Some(0.8));
    assert_eq!(s("1980-03-14", "1991-03-14"), Some(0.0));
    // Half a swap is not a swap: the month equals the other day only.
    assert_eq!(s("1980-03-14", "1980-05-03"), Some(0.0));
    // Two adjacent digits differ, but one is not the other's mirror.
    assert_eq!(s("1920-03-14", "1390-03-14"), Some(0.0));
    assert_eq!(s("1980-03-14", "1980-05-16"), Some(0.0));
    assert_eq!(
        s("1980-03-14", "1990-04-14"),
        Some(0.0),
        "year and month both differ"
    );
    assert_eq!(
        s("1980-03-14", "1990-03-15"),
        Some(0.0),
        "year and day both differ"
    );
}

#[test]
fn an_initial_matches_a_name_that_starts_with_it() {
    let g = |x: &str, y: &str| sim(&name("Smith", x), &name("Smith", y), "given_name");
    assert_eq!(g("A", "Anne"), Some(0.8));
    assert_eq!(g("Anne", "A"), Some(0.8));
    assert_ne!(g("A", "Bob"), Some(0.8));
    assert_ne!(
        g("Anne", "Anna"),
        Some(0.8),
        "two full names are not initials"
    );
}

fn shares_nhs(dob: &str) -> Patient {
    Patient {
        identifiers: vec![Identifier {
            system: NHS_NUMBER_SYSTEM.into(),
            value: "9434765919".into(),
        }],
        // Names that agree: a shared number with no name to check it against goes to Review (CQ-37),
        // which is not what this test is about.
        names: vec![HumanName {
            family: Some("Smith".into()),
            given: vec!["Ann".into()],
        }],
        birth_date: Some(dob.into()),
        ..Default::default()
    }
}

#[test]
fn only_complete_birth_dates_conflict_under_a_shared_nhs_number() {
    let e = MatchingEngine::default();
    let d = |x: &str, y: &str| e.compare(&shares_nhs(x), &shares_nhs(y)).decision;
    assert_eq!(d("1980-03-14", "1981-03-14"), Decision::Review);
    assert_eq!(
        d("1980-03", "1990-05-01"),
        Decision::AutoLink,
        "a year and month alone is not a full date"
    );
    assert_eq!(d("1980", "1990-05-01"), Decision::AutoLink);
}

fn full(family: &str, given: &str, dob: &str) -> Patient {
    Patient {
        names: vec![HumanName {
            family: Some(family.into()),
            given: vec![given.into()],
        }],
        birth_date: Some(dob.into()),
        ..Default::default()
    }
}

#[test]
fn the_evidence_floor_blocks_an_auto_link_below_it_and_not_at_it() {
    // Name, given name and birth date are present; sex, postcode and telephone are not.
    let config = |present: f64, absent: f64| MatchConfig {
        weights: Weights {
            family: present,
            given: present,
            birth_date: present,
            sex: absent,
            postal_code: absent,
            telecom: absent,
        },
        ..Default::default()
    };
    let (a, b) = (
        full("Smith", "Ann", "1980-03-14"),
        full("Smith", "Ann", "1980-03-14"),
    );
    // 0.6 of 2.1 is under half the evidence.
    let low = MatchingEngine::new(config(0.2, 0.5)).compare(&a, &b);
    assert_eq!(low.decision, Decision::Review, "{low:?}");
    assert!(low.conflicts.iter().any(|c| c.contains("of the evidence")));
    // 0.75 of 1.5 (exact in binary) is exactly half, which is enough.
    let at = MatchingEngine::new(config(0.25, 0.25)).compare(&a, &b);
    assert_eq!(at.decision, Decision::AutoLink, "{at:?}");
}

#[test]
fn a_similarity_exactly_at_a_guard_minimum_is_enough() {
    let config = MatchConfig {
        min_given_for_auto: 1.0,
        min_family_for_auto: 1.0,
        min_birth_date_for_auto: 1.0,
        ..Default::default()
    };
    let a = full("Smith", "Ann", "1980-03-14");
    let m = MatchingEngine::new(config).compare(&a, &a.clone());
    assert_eq!(m.decision, Decision::AutoLink, "{m:?}");
    let blocked = MatchingEngine::new(MatchConfig {
        min_birth_date_for_auto: 1.0,
        ..Default::default()
    })
    .compare(&a, &full("Smith", "Ann", "1980-03-15"));
    assert_ne!(blocked.decision, Decision::AutoLink);
}

#[test]
fn the_engine_keeps_the_config_it_was_given() {
    let config = MatchConfig {
        auto_link_threshold: 0.123,
        ..Default::default()
    };
    assert!(
        (MatchingEngine::new(config).config().auto_link_threshold - 0.123).abs() < f64::EPSILON
    );
}

#[test]
fn a_shared_nhs_number_with_names_exactly_at_the_minimum_is_not_a_conflict() {
    let config = MatchConfig {
        min_given_for_auto: 1.0,
        min_family_for_auto: 1.0,
        ..Default::default()
    };
    let with_nhs = |mut p: Patient| {
        p.identifiers.push(Identifier {
            system: NHS_NUMBER_SYSTEM.into(),
            value: "9434765919".into(),
        });
        p
    };
    let a = with_nhs(full("Smith", "Ann", "1980-03-14"));
    let m = MatchingEngine::new(config).compare(&a, &a.clone());
    assert_eq!(m.decision, Decision::AutoLink, "{m:?}");
    // Only the given name at the minimum, family clearly below: still one name agreeing.
    let other = with_nhs(full("Jones", "Ann", "1980-03-14"));
    let m = MatchingEngine::new(config).compare(&a, &other);
    assert_eq!(m.decision, Decision::AutoLink, "{m:?}");
}

#[test]
fn a_family_name_exactly_at_the_minimum_still_counts_as_agreeing_when_the_given_name_does_not() {
    let config = MatchConfig {
        min_family_for_auto: 1.0,
        ..Default::default()
    };
    let with_nhs = |mut p: Patient| {
        p.identifiers.push(Identifier {
            system: NHS_NUMBER_SYSTEM.into(),
            value: "9434765919".into(),
        });
        p
    };
    let a = with_nhs(full("Smith", "Ann", "1980-03-14"));
    let b = with_nhs(full("Smith", "Zed", "1980-03-14"));
    let m = MatchingEngine::new(config).compare(&a, &b);
    assert_eq!(m.decision, Decision::AutoLink, "{m:?}");
}
