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

fn shares_national_number(dob: &str) -> Patient {
    Patient {
        identifiers: vec![Identifier {
            system: NATIONAL_NUMBER_SYSTEM.into(),
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
fn only_complete_birth_dates_conflict_under_a_shared_national_number() {
    let e = MatchingEngine::default();
    let d = |x: &str, y: &str| {
        e.compare(&shares_national_number(x), &shares_national_number(y))
            .decision
    };
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
            ..Weights::default()
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
fn a_shared_national_number_with_names_exactly_at_the_minimum_is_not_a_conflict() {
    let config = MatchConfig {
        min_given_for_auto: 1.0,
        min_family_for_auto: 1.0,
        ..Default::default()
    };
    let with_national_number = |mut p: Patient| {
        p.identifiers.push(Identifier {
            system: NATIONAL_NUMBER_SYSTEM.into(),
            value: "9434765919".into(),
        });
        p
    };
    let a = with_national_number(full("Smith", "Ann", "1980-03-14"));
    let m = MatchingEngine::new(config).compare(&a, &a.clone());
    assert_eq!(m.decision, Decision::AutoLink, "{m:?}");
    // Only the given name at the minimum, family clearly below: still one name agreeing.
    let other = with_national_number(full("Jones", "Ann", "1980-03-14"));
    let m = MatchingEngine::new(config).compare(&a, &other);
    assert_eq!(m.decision, Decision::AutoLink, "{m:?}");
}

#[test]
fn a_family_name_exactly_at_the_minimum_still_counts_as_agreeing_when_the_given_name_does_not() {
    let config = MatchConfig {
        min_family_for_auto: 1.0,
        ..Default::default()
    };
    let with_national_number = |mut p: Patient| {
        p.identifiers.push(Identifier {
            system: NATIONAL_NUMBER_SYSTEM.into(),
            value: "9434765919".into(),
        });
        p
    };
    let a = with_national_number(full("Smith", "Ann", "1980-03-14"));
    let b = with_national_number(full("Smith", "Zed", "1980-03-14"));
    let m = MatchingEngine::new(config).compare(&a, &b);
    assert_eq!(m.decision, Decision::AutoLink, "{m:?}");
}

/// A date of death is weak evidence (OQ-BV3): it counts only when both records have one, it moves the
/// score a little either way, and it cannot make a link by itself.
#[test]
fn a_date_of_death_is_weak_evidence_and_never_enough_alone() {
    let engine = MatchingEngine::default();
    let person = |date: Option<&str>| Patient {
        names: vec![HumanName {
            family: Some("Smith".into()),
            given: vec!["Ann".into()],
        }],
        birth_date: Some("1980-03-14".into()),
        death_date: date.map(str::to_string),
        ..Patient::default()
    };
    let field = |r: &MatchResult| {
        r.fields
            .iter()
            .find(|f| f.field == "death_date")
            .unwrap()
            .similarity
    };
    let none = engine.compare(&person(None), &person(None));
    let one = engine.compare(&person(Some("2026-11-01")), &person(None));
    assert_eq!(field(&none), None);
    assert_eq!(field(&one), None);
    // One record having a date is the same as neither having one: not a disagreement, and not thin evidence.
    assert_eq!(none.score, one.score);
    assert_eq!(none.decision, one.decision);
    assert_eq!(none.conflicts, one.conflicts);

    let same = engine.compare(&person(Some("2026-11-01")), &person(Some("2026-11-01")));
    assert_eq!(field(&same), Some(1.0));
    let far = engine.compare(&person(Some("2026-11-01")), &person(Some("1999-02-03")));
    assert_eq!(field(&far), Some(0.0));
    assert!(far.score < same.score);
    // A disagreement alone blocks nothing, and is a small fall.
    assert!(far.conflicts.is_empty(), "{:?}", far.conflicts);
    assert!(none.score - far.score < 0.1);

    // Nothing else in common: a shared date of death is not a match.
    let bare = |family: &str| Patient {
        names: vec![HumanName {
            family: Some(family.into()),
            given: vec![],
        }],
        death_date: Some("2026-11-01".into()),
        ..Patient::default()
    };
    let r = engine.compare(&bare("Smith"), &bare("Okafor"));
    assert_eq!(r.decision, Decision::NonMatch);

    // It survives a temporary identity, and nothing else does.
    let temporary = Patient {
        temporary_identity: true,
        ..person(Some("2026-11-01"))
    };
    let r = engine.compare(&temporary, &person(Some("2026-11-01")));
    assert_eq!(field(&r), Some(1.0));
    assert_ne!(r.decision, Decision::AutoLink);

    // Empty or invalid is absent, and the JSON is unchanged when there is none.
    let bad = engine.compare(&person(Some("2026-02-31")), &person(Some("2026-11-01")));
    assert_eq!(field(&bad), None);
    let text = serde_json::to_string(&person(None)).unwrap();
    assert!(!text.contains("death_date"), "{text}");
}

// Pins from the 2026-11-17 mutation run (docs/maintainability/mutation-testing.md): each kills mutants that survived.

fn named(lists: &[&[&str]], dob: &str) -> Patient {
    Patient {
        names: lists
            .iter()
            .map(|given| HumanName {
                family: Some("Hughes".into()),
                given: given.iter().map(|g| (*g).into()).collect(),
            })
            .collect(),
        birth_date: Some(dob.into()),
        ..Default::default()
    }
}

fn extra_given(r: &MatchResult) -> bool {
    r.conflict_codes.contains(&ConflictCode::ExtraGivenName)
}

#[test]
fn the_best_pair_of_given_name_lists_wins_whichever_order_they_come_in() {
    let engine = MatchingEngine::default();
    let ann = named(&[&["Ann"]], "1980-03-14");
    // Against `Zoe` and against `Ann Mary`: the better pair (Ann, Ann Mary) decides, and it leaves Mary unmatched.
    for b in [
        named(&[&["Zoe"], &["Ann", "Mary"]], "1980-03-14"),
        named(&[&["Ann", "Mary"], &["Zoe"]], "1980-03-14"),
    ] {
        let r = engine.compare(&ann, &b);
        assert_eq!(sim(&ann, &b, "given_name"), Some(1.0), "{:?}", b.names);
        assert!(extra_given(&r), "{:?}", b.names);
    }
    // A tie between (Ann, Ann) and (Ann, Ann Mary): the pair with nothing unmatched wins, in either order.
    for b in [
        named(&[&["Ann"], &["Ann", "Mary"]], "1980-03-14"),
        named(&[&["Ann", "Mary"], &["Ann"]], "1980-03-14"),
    ] {
        let r = engine.compare(&ann, &b);
        assert_eq!(sim(&ann, &b, "given_name"), Some(1.0));
        assert!(!extra_given(&r), "{:?}", b.names);
    }
}

#[test]
fn a_longer_list_whose_every_name_is_matched_has_nothing_extra() {
    // `Ann` against `Ann Ann`: every name on the longer side matches one on the shorter, so nothing is extra; against
    // `Ann Mary`, Mary is.
    let engine = MatchingEngine::default();
    let ann = named(&[&["Ann"]], "1980-03-14");
    assert!(!extra_given(
        &engine.compare(&ann, &named(&[&["Ann", "Ann"]], "1980-03-14"))
    ));
    assert!(extra_given(
        &engine.compare(&ann, &named(&[&["Ann", "Mary"]], "1980-03-14"))
    ));
}

#[test]
fn swapped_names_score_at_nine_tenths() {
    let a = Patient {
        names: vec![HumanName {
            family: Some("Hughes".into()),
            given: vec!["Gareth".into()],
        }],
        birth_date: Some("1980-03-14".into()),
        ..Default::default()
    };
    let b = Patient {
        names: vec![HumanName {
            family: Some("Gareth".into()),
            given: vec!["Hughes".into()],
        }],
        ..a.clone()
    };
    let r = MatchingEngine::default().compare(&a, &b);
    for f in ["family_name", "given_name"] {
        let s = sim(&a, &b, f).unwrap();
        assert!((s - 0.9).abs() < 1e-9, "{f}: {s}");
    }
    assert!(!r.conflict_codes.is_empty() || r.decision != Decision::AutoLink);
}

#[test]
fn thin_evidence_reports_the_share_present_of_the_weight_counted() {
    // Names and a birth date only: AutoLink on the score, then refused for thin evidence when the floor is high.
    let c = MatchConfig {
        min_evidence_weight: 0.99,
        ..Default::default()
    };
    let a = named(&[&["Ann"]], "1980-03-14");
    let r = MatchingEngine::new(c).compare(&a, &a.clone());
    let counted: f64 = r
        .fields
        .iter()
        .filter(|f| f.field != "death_date" || f.similarity.is_some())
        .map(|f| f.weight)
        .sum();
    let present: f64 = r
        .fields
        .iter()
        .filter(|f| f.similarity.is_some())
        .map(|f| f.weight)
        .sum();
    let percent = r
        .conflict_codes
        .iter()
        .find_map(|c| match c {
            ConflictCode::EvidenceThin { percent_present } => Some(*percent_present),
            _ => None,
        })
        .expect("thin evidence");
    assert!(
        (percent - 100.0 * present / counted).abs() < 1e-9,
        "{percent}"
    );
    assert!(percent > 0.0 && percent < 99.0, "{percent}");
}

#[test]
fn equal_thresholds_are_a_valid_config() {
    let c = MatchConfig {
        auto_link_threshold: 0.9,
        review_threshold: 0.9,
        ..Default::default()
    };
    assert!(c.validate().is_ok());
}

#[test]
fn a_config_that_leaves_out_the_calling_code_gets_44() {
    let mut v = serde_json::to_value(MatchConfig::default()).unwrap();
    v.as_object_mut().unwrap().remove("default_calling_code");
    let c: MatchConfig = serde_json::from_value(v).unwrap();
    assert_eq!(c.default_calling_code, 44);
}

#[test]
fn a_telephone_compares_by_its_digits_alone() {
    let a = tel(&[("phone", "+44 (0)7700 900123")]);
    assert_eq!(
        sim(&a, &tel(&[("phone", "0044-07700-900123")]), "telecom"),
        Some(1.0)
    );
    assert_eq!(
        sim(&a, &tel(&[("phone", "07700 900124")]), "telecom"),
        Some(0.0)
    );
}
