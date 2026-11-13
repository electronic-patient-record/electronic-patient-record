//! TT-1, hazard H-047: a temporary identity is not evidence of who a person is.

use electronic_patient_record_matcher::{
    ConflictCode, Decision, HumanName, Identifier, MatchingEngine, NATIONAL_NUMBER_SYSTEM, Patient,
};

/// The safer format from a 2018 national patient safety alert on naming unknown patients: looks real, with an estimated date.
fn temporary(family: &str, given: &str) -> Patient {
    Patient {
        names: vec![HumanName {
            family: Some(family.into()),
            given: vec![given.into()],
        }],
        birth_date: Some("1980-01-01".into()),
        temporary_identity: true,
        ..Patient::default()
    }
}

fn real(family: &str, given: &str) -> Patient {
    Patient {
        temporary_identity: false,
        ..temporary(family, given)
    }
}

fn with_national_number(mut p: Patient) -> Patient {
    p.identifiers.push(Identifier {
        system: NATIONAL_NUMBER_SYSTEM.into(),
        value: "9434765919".into(),
    });
    p
}

#[test]
fn without_the_flag_these_records_would_auto_link() {
    let r =
        MatchingEngine::default().compare(&real("Whisky", "Foxtrot"), &real("Whisky", "Foxtrot"));
    assert_eq!(
        r.decision,
        Decision::AutoLink,
        "the hazard this flag exists for"
    );
}

#[test]
fn a_temporary_identity_never_auto_links_either_way_round() {
    let m = MatchingEngine::default();
    for (a, b) in [
        (
            temporary("Whisky", "Foxtrot"),
            temporary("Whisky", "Foxtrot"),
        ),
        (temporary("Whisky", "Foxtrot"), real("Whisky", "Foxtrot")),
        (real("Whisky", "Foxtrot"), temporary("Whisky", "Foxtrot")),
        (
            with_national_number(temporary("Whisky", "Foxtrot")),
            with_national_number(real("Whisky", "Foxtrot")),
        ),
    ] {
        let r = m.compare(&a, &b);
        assert_ne!(r.decision, Decision::AutoLink, "{a:?} against {b:?}");
        assert_eq!(
            r.conflict_codes.first(),
            Some(&ConflictCode::TemporaryIdentity)
        );
        for f in r
            .fields
            .iter()
            .filter(|f| ["family_name", "given_name", "birth_date"].contains(&f.field.as_str()))
        {
            assert_eq!(f.similarity, None, "{} must be absent, not zero", f.field);
        }
    }
}

#[test]
fn the_comparison_stays_symmetric() {
    let m = MatchingEngine::default();
    let (a, b) = (temporary("Whisky", "Foxtrot"), real("Rivers", "Morgan"));
    assert_eq!(m.compare(&a, &b), m.compare(&b, &a));
}

#[test]
fn the_flag_is_left_out_of_json_when_false_and_defaults_to_false() {
    let json = serde_json::to_value(real("A", "B")).unwrap();
    assert!(json.get("temporary_identity").is_none());
    let p: Patient = serde_json::from_str(r#"{"names":[]}"#).unwrap();
    assert!(!p.temporary_identity);
    let t = serde_json::to_value(temporary("A", "B")).unwrap();
    assert_eq!(t["temporary_identity"], true);
}
