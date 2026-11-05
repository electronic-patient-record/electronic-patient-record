//! BV-8: the deaths a record has to handle, as test data. The rules are `spec/bereavement/death-assertion.md`.
//! This project has no seeded synthetic patients yet, so the scenarios live here, written in plain words
//! and invented, and a generator can take them from this table when one exists.
//!
//! Each runs through the pure rules in `src/death.rs`, step by step, and says what the one answer is for
//! each record at the end. Database-free. The routes, the store and the hold have their own tests.

use chrono::{DateTime, TimeZone, Utc};
use electronic_patient_record::death::{
    Actor, Answer, Assertion, Occurred, Refusal, Source, Status, Step, advance, answer, report,
};

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 5, 12, 0, 0).unwrap()
}

fn person(id: &str) -> Actor {
    Actor::Person(id.into())
}

fn feed() -> Actor {
    Actor::Feed("mllp:adt-feed".into())
}

fn known(at: &str) -> Occurred {
    Occurred::Known { at: at.into() }
}

fn not_known(reason: &str) -> Occurred {
    Occurred::NotKnown {
        reason: reason.into(),
    }
}

fn reported(source: Source, by: Actor, occurred: Occurred, reason: &str) -> Assertion {
    report(source, by, occurred, reason, now()).unwrap()
}

fn step(a: &Assertion, to: Status, by: &str, reason: &str) -> Assertion {
    advance(
        a,
        &Step {
            to,
            by: person(by),
            reason: reason.into(),
        },
    )
    .unwrap()
}

fn asserted(status: Status, confirmed: bool, occurred: Option<&str>, differ: bool) -> Answer {
    Answer::Asserted {
        status,
        confirmed,
        occurred: occurred.map(str::to_string),
        dates_differ: differ,
    }
}

struct Record {
    name: &'static str,
    assertions: Vec<Assertion>,
    expected: Answer,
}

struct Scenario {
    name: &'static str,
    records: Vec<Record>,
}

/// An expected death in a hospice: every step taken, in order.
fn expected() -> Scenario {
    let a = reported(
        Source::Hospital,
        person("nurse-1"),
        known("2026-10-12"),
        "Died on the ward with family present.",
    );
    let a = step(
        &a,
        Status::Verified,
        "nurse-1",
        "Death verified on the ward.",
    );
    let a = step(&a, Status::Certified, "clinician-1", "Certificate issued.");
    let a = step(
        &a,
        Status::Registered,
        "clinician-1",
        "Registrar confirmed.",
    );
    Scenario {
        name: "expected",
        records: vec![Record {
            name: "the patient",
            assertions: vec![a],
            expected: asserted(Status::Registered, true, Some("2026-10-12"), false),
        }],
    }
}

/// A sudden death at home: a relative rings before anyone has a date, and the hospital adds one.
fn sudden() -> Scenario {
    let family = reported(
        Source::FamilyMember,
        person("receptionist-1"),
        not_known("The caller did not give a date."),
        "A relative rang the practice.",
    );
    let family = step(
        &family,
        Status::Verified,
        "clinician-1",
        "Confirmed with the ambulance service.",
    );
    let hospital = reported(
        Source::Hospital,
        person("nurse-1"),
        known("2026-10-20"),
        "Brought to the emergency department; recognition of life extinct recorded.",
    );
    Scenario {
        name: "sudden",
        records: vec![Record {
            name: "the patient",
            assertions: vec![family, hospital],
            expected: asserted(Status::Verified, true, Some("2026-10-20"), false),
        }],
    }
}

/// A death referred to a coroner: confirmed, and not certified for as long as the inquest takes. The
/// coroner's date is less precise, which says "these differ" so that a person looks.
fn coroner() -> Scenario {
    let hospital = reported(
        Source::Hospital,
        person("nurse-1"),
        known("2026-09-14"),
        "Died in hospital; referred to the coroner.",
    );
    let hospital = step(
        &hospital,
        Status::Verified,
        "clinician-1",
        "Death verified; the cause is not yet known.",
    );
    let coroner = reported(
        Source::Coroner,
        person("clinical-admin-1"),
        known("2026-09"),
        "Letter from the coroner's office opening an investigation.",
    );
    Scenario {
        name: "coroner",
        records: vec![Record {
            name: "the patient",
            assertions: vec![hospital, coroner],
            expected: asserted(Status::Verified, true, Some("2026-09-14"), true),
        }],
    }
}

/// A report filed against the wrong record: it comes off one person and goes on the other.
fn wrong_patient() -> Scenario {
    let wrong = reported(
        Source::OtherSystem,
        feed(),
        known("2026-10-03"),
        "HL7 feed: PID-29.",
    );
    let wrong = step(
        &wrong,
        Status::WithdrawnInError,
        "clinician-1",
        "Filed against the wrong record: two people with the same name.",
    );
    let right = reported(
        Source::Hospital,
        person("nurse-1"),
        known("2026-10-03"),
        "Moved from the record it was filed against in error.",
    );
    Scenario {
        name: "wrong patient",
        records: vec![
            Record {
                name: "the person wrongly reported",
                assertions: vec![wrong],
                expected: Answer::NotKnownToHaveDied,
            },
            Record {
                name: "the person who died",
                assertions: vec![right],
                expected: asserted(Status::Reported, false, Some("2026-10-03"), false),
            },
        ],
    }
}

/// A living person confirmed as dead, then found to be alive. The withdrawal stays in the history.
fn withdrawn() -> Scenario {
    let a = reported(
        Source::NationalDemographics,
        feed(),
        known("2026-08-30"),
        "National demographics update.",
    );
    let a = step(
        &a,
        Status::Verified,
        "nurse-1",
        "Accepted from the national record.",
    );
    let a = step(
        &a,
        Status::WithdrawnInError,
        "clinician-1",
        "The patient attended clinic in person. National record corrected.",
    );
    Scenario {
        name: "withdrawn",
        records: vec![Record {
            name: "the patient",
            assertions: vec![a],
            expected: Answer::NotKnownToHaveDied,
        }],
    }
}

/// A baby who died hours after birth, so the time matters and is kept as given. A pregnancy that ended
/// before there was a patient record is not an assertion on anyone's record and is not modelled here.
fn baby() -> Scenario {
    let a = reported(
        Source::Hospital,
        person("nurse-1"),
        known("2026-10-27T04:10:00Z"),
        "Died on the neonatal unit on the day of birth.",
    );
    let a = step(
        &a,
        Status::Verified,
        "clinician-1",
        "Death verified on the unit.",
    );
    Scenario {
        name: "baby",
        records: vec![Record {
            name: "the baby",
            assertions: vec![a],
            expected: asserted(Status::Verified, true, Some("2026-10-27T04:10:00Z"), false),
        }],
    }
}

/// Twins share a birth date, an address and often a phone. One dying says nothing about the other: each
/// record has its own assertions.
fn twin() -> Scenario {
    let a = reported(
        Source::Hospital,
        person("nurse-1"),
        known("2026-10-15"),
        "Died on the paediatric ward.",
    );
    let a = step(
        &a,
        Status::Verified,
        "clinician-1",
        "Death verified on the ward.",
    );
    Scenario {
        name: "twin",
        records: vec![
            Record {
                name: "the twin who died",
                assertions: vec![a],
                expected: asserted(Status::Verified, true, Some("2026-10-15"), false),
            },
            Record {
                name: "the twin who lives",
                assertions: vec![],
                expected: Answer::NotKnownToHaveDied,
            },
        ],
    }
}

fn all() -> Vec<Scenario> {
    vec![
        expected(),
        sudden(),
        coroner(),
        wrong_patient(),
        withdrawn(),
        baby(),
        twin(),
    ]
}

#[test]
fn every_scenario_gives_the_answer_it_should() {
    for scenario in all() {
        for record in &scenario.records {
            assert_eq!(
                answer(&record.assertions),
                record.expected,
                "{}: {}",
                scenario.name,
                record.name
            );
        }
    }
}

#[test]
fn the_seven_scenarios_are_all_here() {
    let names: Vec<_> = all().iter().map(|s| s.name).collect();
    assert_eq!(
        names,
        [
            "expected",
            "sudden",
            "coroner",
            "wrong patient",
            "withdrawn",
            "baby",
            "twin"
        ]
    );
}

/// Only an assertion that stands holds contact, and it holds whether or not anyone has confirmed it.
#[test]
fn a_record_holds_contact_exactly_when_an_assertion_stands() {
    for scenario in all() {
        for record in &scenario.records {
            let stands = record
                .assertions
                .iter()
                .any(|a| a.status != Status::WithdrawnInError);
            assert_eq!(
                answer(&record.assertions).holds_contact(),
                stands,
                "{}: {}",
                scenario.name,
                record.name
            );
        }
    }
}

#[test]
fn a_feed_cannot_withdraw_the_report_it_made_and_a_withdrawal_is_final() {
    let a = reported(
        Source::OtherSystem,
        feed(),
        known("2026-10-03"),
        "HL7 feed.",
    );
    let retract = Step {
        to: Status::WithdrawnInError,
        by: feed(),
        reason: "A later message said N.".into(),
    };
    assert_eq!(advance(&a, &retract), Err(Refusal::PersonRequired));

    let gone = step(&a, Status::WithdrawnInError, "clinician-1", "Wrong record.");
    let back = Step {
        to: Status::Verified,
        by: person("clinician-1"),
        reason: "Changed my mind.".into(),
    };
    assert_eq!(advance(&gone, &back), Err(Refusal::NotAllowed));
}

/// A step changes the status and the reason, and leaves who reported it and the date as they were.
#[test]
fn a_step_keeps_the_report_it_came_from() {
    let scenario = expected();
    let a = &scenario.records[0].assertions[0];
    assert_eq!(a.source, Source::Hospital);
    assert_eq!(a.reported_by, person("nurse-1"));
    assert_eq!(a.occurred, known("2026-10-12"));
    assert_eq!(a.reason, "Registrar confirmed.");
}
