//! Only a verified identifier that both records carry can lead to an automatic link (0.9.0; decided 2026-11-25). A name
//! and a birth date alone never do, and nor do an address or a telephone number, which everyone in one place shares.

use electronic_patient_record_matcher::*;

fn person(family: &str, given: &str, born: &str) -> Patient {
    Patient {
        names: vec![HumanName {
            family: Some(family.into()),
            given: vec![given.into()],
        }],
        birth_date: Some(born.into()),
        ..Patient::default()
    }
}

fn at_one_place(mut p: Patient) -> Patient {
    p.addresses = vec![Address {
        postal_code: Some("AB1 2CD".into()),
    }];
    p.telecom = vec![Telecom {
        system: "phone".into(),
        value: "01632 960001".into(),
    }];
    p.sex = Some(AdministrativeSex::Male);
    p
}

fn numbered(mut p: Patient, system: &str, value: &str) -> Patient {
    p.identifiers.push(Identifier {
        system: system.into(),
        value: value.into(),
    });
    p
}

#[test]
fn no_pair_without_a_shared_verified_identifier_is_linked_automatically() {
    let families = [
        "Osei",
        "Evans",
        "Evens",
        "Reed",
        "Reid",
        "Jones",
        "Wyn-Jones",
    ];
    let given = ["Kofi", "Kofe", "Hassan", "Hussain", "David"];
    let born = ["1988-06-14", "1988-06-15", "1988", "1961-02-27"];
    let people: Vec<Patient> = families
        .iter()
        .flat_map(|f| given.iter().map(move |g| (f, g)))
        .flat_map(|(f, g)| born.iter().map(move |b| at_one_place(person(f, g, b))))
        .collect();
    // With the thresholds as they are, and with every threshold and guard as low as it goes.
    let lowest = MatchConfig {
        auto_link_threshold: 0.0,
        review_threshold: 0.0,
        min_given_for_auto: 0.0,
        min_family_for_auto: 0.0,
        min_birth_date_for_auto: 0.0,
        min_evidence_weight: 0.0,
        ..MatchConfig::default()
    };
    for engine in [MatchingEngine::default(), MatchingEngine::new(lowest)] {
        for a in &people {
            for b in &people {
                let r = engine.compare(a, b);
                assert_ne!(r.decision, Decision::AutoLink, "{a:?} {b:?}");
            }
        }
    }
}

#[test]
fn the_reason_is_given_last_and_in_words() {
    let twins = [
        person("Osei", "Kofi", "1992-11-30"),
        person("Osei", "Kofe", "1992-11-30"),
    ];
    let r = MatchingEngine::default().compare(&twins[0], &twins[1]);
    assert!(crate::only_an_identifier_is_missing(&r), "{r:?}");
    assert_eq!(
        r.conflicts,
        ["no verified identifier is shared, so a person must decide"]
    );
    assert_eq!(
        serde_json::to_value(&r.conflict_codes).unwrap(),
        serde_json::json!([{"code": "no_shared_identifier"}])
    );
    // With another reason, it comes after it.
    let r = MatchingEngine::default().compare(
        &person("Jones", "David", "1988-06-04"),
        &person("Jones", "David", "1988-04-06"),
    );
    if r.conflict_codes.len() > 1 {
        assert_eq!(
            r.conflict_codes.last(),
            Some(&ConflictCode::NoSharedIdentifier)
        );
    }
}

#[test]
fn a_shared_valid_number_still_links_and_one_that_is_not_verified_does_not() {
    let e = MatchingEngine::default();
    let p = || at_one_place(person("Jones", "David", "1988-06-14"));
    let linked = |system: &str, value: &str| {
        e.compare(&numbered(p(), system, value), &numbered(p(), system, value))
            .decision
    };
    assert_eq!(
        linked(NATIONAL_NUMBER_SYSTEM, "9434765919"),
        Decision::AutoLink
    );
    // A failed check digit, a placeholder, a hospital's own number: none is verified.
    for (system, value) in [
        (NATIONAL_NUMBER_SYSTEM, "9434765918"),
        (NATIONAL_NUMBER_SYSTEM, "0000000000"),
        ("urn:mrn", "12345"),
    ] {
        assert_eq!(linked(system, value), Decision::Review, "{system} {value}");
    }
    // On one record only.
    assert_eq!(
        e.compare(&numbered(p(), NATIONAL_NUMBER_SYSTEM, "9434765919"), &p())
            .decision,
        Decision::Review
    );
}
