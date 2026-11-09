//! Welsh names and addresses (WL-8): what the matcher does with them today. Each test states the
//! behaviour it found, so a change to it is seen. Nothing here changes the matcher; the gaps it
//! records (anglicised forms, address lines) are not built.

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

fn at(mut p: Patient, postcode: &str) -> Patient {
    p.addresses = vec![Address {
        postal_code: Some(postcode.into()),
    }];
    p
}

/// Both orders agree (decision and score), and the result is returned.
fn compare(a: &Patient, b: &Patient) -> MatchResult {
    let e = MatchingEngine::default();
    let (ab, ba) = (e.compare(a, b), e.compare(b, a));
    assert_eq!(ab.decision, ba.decision, "order of the records");
    assert_eq!(ab.score, ba.score, "order of the records");
    ab
}

fn sim(r: &MatchResult, field: &str) -> Option<f64> {
    r.fields
        .iter()
        .find(|f| f.field == field)
        .unwrap()
        .similarity
}

#[test]
fn a_welsh_name_with_its_diacritics_matches_itself_fully() {
    // ŵ ŷ â ê ï are kept by the fold (spec rule 6), so the same spelling is 1.0.
    let r = compare(
        &person("Gwŷn", &["Siân", "Mêr"]),
        &person("Gwŷn", &["Siân", "Mêr"]),
    );
    assert_eq!(r.decision, Decision::AutoLink);
    assert_eq!(sim(&r, "family_name"), Some(1.0));
    assert_eq!(sim(&r, "given_name"), Some(1.0));
    assert!(!r.phonetic_match, "an identical spelling is not news");
}

#[test]
fn a_name_keyed_without_its_circumflex_or_diaeresis_matches_by_transliteration() {
    // A keyboard without ŵ or ŷ drops the mark. Transliteration (a Latin letter with a mark to its
    // base letter) makes these 1.0, so they auto-link with the birth date; the phonetic flag says
    // the spellings differ.
    for (written, keyed) in [
        (("Gwŷn", "Siân"), ("Gwyn", "Sian")),
        (("Llŷr", "Dafydd"), ("Llyr", "Dafydd")),
        (("Rhŵn", "Ieuan"), ("Rhwn", "Ieuan")),
        (("Jones", "Gwenllïan"), ("Jones", "Gwenllian")),
        (("Gŵyn", "Siân"), ("GWYN", "SIAN")),
    ] {
        let r = compare(
            &person(written.0, &[written.1]),
            &person(keyed.0, &[keyed.1]),
        );
        assert_eq!(r.decision, Decision::AutoLink, "{written:?} / {keyed:?}");
        assert_eq!(sim(&r, "family_name"), Some(1.0), "{written:?}");
        assert_eq!(sim(&r, "given_name"), Some(1.0), "{written:?}");
        assert!(r.phonetic_match, "{written:?}: spelled differently");
    }
}

#[test]
fn a_patronymic_particle_is_a_part_of_the_family_name_that_may_be_gained_or_lost() {
    // `ap Rhys` against `Rhys`, `ferch Owain` against `Owain`: the per-part rule (spec, family name)
    // reads the particle as a part gained or lost, as `Smith` against `Smith-Jones`, so 1.0.
    for (with, without) in [
        ("ap Rhys", "Rhys"),
        ("ab Owen", "Owen"),
        ("ferch Owain", "Owain"),
    ] {
        let r = compare(&person(with, &["Nia"]), &person(without, &["Nia"]));
        assert_eq!(sim(&r, "family_name"), Some(1.0), "{with}");
        assert_eq!(r.decision, Decision::AutoLink, "{with}");
    }
}

#[test]
fn two_different_patronymics_that_share_a_particle_never_auto_link() {
    // The shared particle lifts the family similarity (each part is half the name), but not past
    // the family-name guard: these go to a person, not to an automatic link.
    for (a, b) in [
        ("ap Rhys", "ap Hywel"),
        ("ferch Dafydd", "ferch Owain"),
        ("ferch Owain", "ap Owain"),
    ] {
        let r = compare(&person(a, &["Nia"]), &person(b, &["Nia"]));
        let family = sim(&r, "family_name").unwrap();
        assert!(family < 0.85, "{a} / {b}: {family}");
        assert_ne!(r.decision, Decision::AutoLink, "{a} / {b}");
        assert_eq!(r.decision, Decision::Review, "{a} / {b}");
    }
}

#[test]
fn an_anglicised_patronymic_or_name_is_not_recognised_and_goes_to_review() {
    // Not built: no table of anglicised forms (`ab Owen` to `Bowen`, `ap Rhys` to `Price`,
    // `Dafydd` to `Davies`, `Ifan` to `Evan`). The matcher scores the letters only, so these are a
    // review for a person, never an automatic link.
    for (welsh, english) in [
        (("ab Owen", "Dafydd"), ("Bowen", "Dafydd")),
        (("ap Rhys", "Dafydd"), ("Price", "Dafydd")),
        (("Dafydd", "Ifan"), ("Davies", "Evan")),
    ] {
        let r = compare(
            &person(welsh.0, &[welsh.1]),
            &person(english.0, &[english.1]),
        );
        assert_eq!(r.decision, Decision::Review, "{welsh:?} / {english:?}");
        assert!(sim(&r, "family_name").unwrap() < 0.85, "{welsh:?}");
    }
}

#[test]
fn a_double_barrelled_welsh_family_name_matches_with_or_without_the_hyphen_or_a_part() {
    for (a, b) in [("Wyn Jones", "Wyn-Jones"), ("Jones-Williams", "Williams")] {
        let r = compare(&person(a, &["Rhian"]), &person(b, &["Rhian"]));
        assert_eq!(sim(&r, "family_name"), Some(1.0), "{a} / {b}");
        assert_eq!(r.decision, Decision::AutoLink, "{a} / {b}");
    }
}

#[test]
fn a_welsh_postcode_is_compared_as_folded_text_and_an_address_line_takes_no_part() {
    // Only the postcode is in the model (`Address.postal_code`): a street written in Welsh
    // (`Heol y Frenhines`) or in English (`Queen Street`) cannot be compared, because neither is
    // held. Case and spaces fold away.
    let r = compare(
        &at(person("Jones", &["Rhian"]), "CF10 3NQ"),
        &at(person("Jones", &["Rhian"]), "cf103nq"),
    );
    assert_eq!(sim(&r, "postal_code"), Some(1.0));
    let r = compare(
        &at(person("Jones", &["Rhian"]), "SA1 1AA"),
        &at(person("Jones", &["Rhian"]), "SA2 1AA"),
    );
    assert_eq!(sim(&r, "postal_code"), Some(0.0));
    // A different postcode lowers the score and blocks nothing: people move.
    assert_eq!(r.decision, Decision::AutoLink);
}
