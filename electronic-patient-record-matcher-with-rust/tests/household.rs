//! Household evidence (CQ-19): a shared postcode or phone counts at half weight, a shared email in full, and
//! relatives at one address cannot reach the review band on that evidence.

use electronic_patient_record_matcher::*;

fn person(given: &str, post: Option<&str>, tel: &[(&str, &str)]) -> Patient {
    Patient {
        names: vec![HumanName {
            family: Some("Okafor".into()),
            given: vec![given.into()],
        }],
        birth_date: Some("1980-03-14".into()),
        addresses: post
            .map(|p| Address {
                postal_code: Some(p.into()),
            })
            .into_iter()
            .collect(),
        telecom: tel
            .iter()
            .map(|(s, v)| Telecom {
                system: (*s).into(),
                value: (*v).into(),
            })
            .collect(),
        ..Default::default()
    }
}

fn weight(a: &Patient, b: &Patient, field: &str) -> f64 {
    let r = MatchingEngine::default().compare(a, b);
    r.fields.iter().find(|f| f.field == field).unwrap().weight
}

#[test]
fn a_postcode_counts_at_half_weight() {
    let (a, b) = (
        person("Ada", Some("LS1 4AB"), &[]),
        person("Ada", Some("ls14ab"), &[]),
    );
    assert_eq!(weight(&a, &b, "postal_code"), 0.05);
}

#[test]
fn a_phone_counts_at_half_weight_and_a_shared_email_in_full() {
    let a = person(
        "Ada",
        None,
        &[("phone", "07700 900123"), ("email", "ada@example.org")],
    );
    let phone_only = person("Ada", None, &[("phone", "+44 7700 900123")]);
    let email = person("Ada", None, &[("email", "ADA@example.org")]);
    assert_eq!(weight(&a, &phone_only, "telecom"), 0.05);
    assert_eq!(weight(&a, &email, "telecom"), 0.10);
}

#[test]
fn a_household_cannot_lift_relatives_out_of_non_match() {
    // Same family name, address and phone; different given name and birth date.
    let a = person("Ada", Some("LS1 4AB"), &[("phone", "07700 900123")]);
    let mut b = person("Tobenna", Some("LS1 4AB"), &[("phone", "07700 900123")]);
    b.birth_date = Some("2011-09-02".into());
    let r = MatchingEngine::default().compare(&a, &b);
    assert_eq!(r.decision, Decision::NonMatch, "{r:?}");
    assert!(r.score < 0.7, "{}", r.score);
}

#[test]
fn the_same_person_still_auto_links_and_scores_one() {
    let a = person("Ada", Some("LS1 4AB"), &[("phone", "07700 900123")]);
    let r = MatchingEngine::default().compare(&a, &a.clone());
    assert_eq!(r.decision, Decision::AutoLink, "{r:?}");
    assert!(r.score > 0.99);
}

#[test]
fn the_weight_change_is_symmetric() {
    let a = person("Ada", Some("LS1 4AB"), &[("phone", "07700 900123")]);
    let b = person("Ada", Some("LS2 9ZZ"), &[("email", "a@b.org")]);
    let e = MatchingEngine::default();
    assert_eq!(e.compare(&a, &b), e.compare(&b, &a));
}
