//! What a place many people share does to a comparison (XP-5): a prison, a barracks, a hostel, a care home. Its postcode
//! and its telephone number are on every record, and the matcher counts both as evidence about one person.
//!
//! Every pair here is of **two people**. Each is compared with nothing shared, with the postcode, with the telephone
//! number, and with both, and what that moves is pinned, so a change to the matcher has to say what it did to them. It
//! measures invented pairs, and not how often they happen.

use electronic_patient_record_matcher::{
    Address, Decision, HumanName, MatchingEngine, Patient, Telecom,
};

fn person(family: &str, given: &str, born: Option<&str>) -> Patient {
    Patient {
        names: vec![HumanName {
            family: Some(family.into()),
            given: vec![given.into()],
        }],
        birth_date: born.map(String::from),
        ..Patient::default()
    }
}

/// Pairs of two people who live in one place.
fn pairs() -> Vec<(&'static str, Patient, Patient)> {
    let p = person;
    let born = Some("1988-06-14");
    vec![
        (
            "one family name, given names unlike, one birth date",
            p("Khan", "Adil", born),
            p("Khan", "Yusuf", born),
        ),
        (
            "one family name, given names alike, one birth date",
            p("Khan", "Adil", born),
            p("Khan", "Adib", born),
        ),
        (
            "one letter apart in the family name, one birth date",
            p("Evans", "Rhys", born),
            p("Evens", "Rhys", born),
        ),
        (
            "one name, births a year apart",
            p("Jones", "David", born),
            p("Jones", "David", Some("1989-06-14")),
        ),
        (
            "one name, births one digit apart",
            p("Jones", "David", born),
            p("Jones", "David", Some("1988-06-15")),
        ),
        (
            "one name, day and month the other way",
            p("Jones", "David", Some("1988-06-04")),
            p("Jones", "David", Some("1988-04-06")),
        ),
        (
            "one name, births far apart",
            p("Jones", "David", born),
            p("Jones", "David", Some("1961-02-27")),
        ),
        (
            "one name, one birth year only",
            p("Jones", "David", Some("1988")),
            p("Jones", "David", Some("1988")),
        ),
        (
            "one name, no birth date on one",
            p("Jones", "David", born),
            p("Jones", "David", None),
        ),
        (
            "one name and one birth date",
            p("Jones", "David", born),
            p("Jones", "David", born),
        ),
        (
            "names alike, one birth date: Reed and Reid",
            p("Reed", "Hassan", born),
            p("Reid", "Hussain", born),
        ),
        (
            "names alike, one birth date: Hanson and Harrison",
            p("Hanson", "Martin", born),
            p("Harrison", "Marie", born),
        ),
        (
            "nothing else in common",
            p("Okafor", "Chidi", born),
            p("Nowak", "Piotr", Some("1971-11-02")),
        ),
        (
            "one birth date and nothing else",
            p("Okafor", "Chidi", born),
            p("Nowak", "Piotr", born),
        ),
    ]
}

fn at(mut p: Patient, postcode: bool, phone: bool) -> Patient {
    if postcode {
        p.addresses = vec![Address {
            postal_code: Some("AB1 2CD".into()),
        }];
    }
    if phone {
        p.telecom = vec![Telecom {
            system: "phone".into(),
            value: "01632 960001".into(),
        }];
    }
    p
}

fn word(d: Decision) -> char {
    match d {
        Decision::AutoLink => 'A',
        Decision::Review => 'R',
        Decision::NonMatch => 'n',
    }
}

/// The four decisions for a pair: nothing shared, the postcode, the telephone, both.
fn decisions(engine: &MatchingEngine, a: &Patient, b: &Patient) -> String {
    [(false, false), (true, false), (false, true), (true, true)]
        .into_iter()
        .map(|(post, phone)| {
            let r = engine.compare(&at(a.clone(), post, phone), &at(b.clone(), post, phone));
            word(r.decision)
        })
        .collect()
}

/// `n` not a match, `R` to a person, `A` the matcher would link. Since matcher 0.9.0 (AG-13) none is `A`: no pair here has a
/// verified identifier. Before it, a postcode alone turned "ask a person" into "link" for the two pairs of like names, and
/// three pairs were `A` with nothing shared.
const PINNED: [(&str, &str); 14] = [
    // Two people with one birth date, and nothing else, reach the review list by living in one place: work, not harm.
    ("one birth date and nothing else", "nnnR"),
    (
        "one family name, given names unlike, one birth date",
        "nRRR",
    ),
    ("names alike, one birth date: Reed and Reid", "RRRR"),
    ("names alike, one birth date: Hanson and Harrison", "RRRR"),
    ("nothing else in common", "nnnn"),
    ("one name, births far apart", "nnnn"),
    ("one name, births a year apart", "RRRR"),
    ("one name, births one digit apart", "RRRR"),
    ("one name, day and month the other way", "RRRR"),
    ("one name, one birth year only", "RRRR"),
    ("one name, no birth date on one", "RRRR"),
    ("one name and one birth date", "RRRR"),
    ("one family name, given names alike, one birth date", "RRRR"),
    (
        "one letter apart in the family name, one birth date",
        "RRRR",
    ),
];

#[test]
fn what_a_shared_postcode_and_telephone_move_is_known() {
    let engine = MatchingEngine::default();
    let mut pinned: Vec<(&str, &str)> = PINNED.to_vec();
    let mut found = Vec::new();
    let all: Vec<(&str, String)> = pairs()
        .iter()
        .map(|(what, a, b)| (*what, decisions(&engine, a, b)))
        .collect();
    for (what, got) in &all {
        println!("| {what} | {got} |");
        found.push((*what, got.as_str()));
    }
    pinned.sort_unstable();
    found.sort_unstable();
    assert_eq!(found, pinned);
}
