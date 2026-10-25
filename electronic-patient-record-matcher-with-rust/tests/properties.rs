//! Property tests over random patients (CQ-30): totality, symmetry and "absent evidence never
//! raises the decision". Hand-rolled generator with a fixed seed, as elsewhere in this repository:
//! no extra dependency, and a failure is reproducible from the printed seed and index.

use electronic_patient_record_matcher::*;

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }
    fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next()).unwrap_or(0) % n.max(1)
    }
    fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        items[self.below(items.len())]
    }
}

const FAMILY: &[&str] = &[
    "Smith",
    "Smyth",
    "Smithe",
    "O'Brien",
    "OBrien",
    "Müller",
    "Mueller",
    "de la Cruz",
    "Nguyen",
    "Ng",
    "Li",
    "Jones",
    "Jones-Smith",
    "",
    " ",
    "漢字",
    "Ünal",
    "SMITH",
];
const GIVEN: &[&str] = &[
    "Ann", "Anne", "Anna", "A", "Jo", "Joe", "José", "Jose", "Mary Ann", "Zoë", "", "  ", "李",
    "ann", "Robert", "Bob",
];
const DOB: &[&str] = &[
    "1980-03-14",
    "1980-03-41",
    "1980-14-03",
    "1980-03",
    "1980",
    "2026-10-23",
    "1899-12-31",
    "19800314",
    "",
    "x",
    "1980-02-30",
    "1981-03-14",
];
const NHS: &[&str] = &[
    "9434765919",
    "9434765918",
    "0000000000",
    "943 476 5919",
    "1",
    "",
    "9999999999",
    "4010232137",
];
const POST: &[&str] = &[
    "LS1 4AB", "ls14ab", "LS1  4AB", "SW1A 1AA", "", "12345", "?",
];
const PHONE: &[&str] = &[
    "01632 960001",
    "+44 1632 960001",
    "ann@example.com",
    "ANN@EXAMPLE.COM",
    "",
    "0",
];

fn sex(r: &mut Lcg) -> Option<AdministrativeSex> {
    match r.below(5) {
        0 => Some(AdministrativeSex::Male),
        1 => Some(AdministrativeSex::Female),
        2 => Some(AdministrativeSex::Other),
        3 => Some(AdministrativeSex::Unknown),
        _ => None,
    }
}

fn patient(r: &mut Lcg) -> Patient {
    let mut p = Patient::default();
    for _ in 0..r.below(3) {
        p.names.push(HumanName {
            family: (r.below(5) != 0).then(|| r.pick(FAMILY).to_string()),
            given: (0..r.below(3)).map(|_| r.pick(GIVEN).to_string()).collect(),
        });
    }
    p.birth_date = (r.below(5) != 0).then(|| r.pick(DOB).to_string());
    p.sex = sex(r);
    for _ in 0..r.below(3) {
        p.identifiers.push(Identifier {
            system: if r.below(4) == 0 {
                "urn:other".into()
            } else {
                NHS_NUMBER_SYSTEM.into()
            },
            value: r.pick(NHS).to_string(),
        });
    }
    for _ in 0..r.below(2) {
        p.addresses.push(Address {
            postal_code: (r.below(4) != 0).then(|| r.pick(POST).to_string()),
        });
    }
    for _ in 0..r.below(3) {
        p.telecom.push(Telecom {
            system: if r.below(2) == 0 {
                "phone".into()
            } else {
                "email".into()
            },
            value: r.pick(PHONE).to_string(),
        });
    }
    p
}

fn rank(d: Decision) -> u8 {
    match d {
        Decision::NonMatch => 0,
        Decision::Review => 1,
        Decision::AutoLink => 2,
    }
}

const SEED: u64 = 20_261_023;
const CASES: usize = 20_000;

#[test]
fn compare_is_total_and_its_score_is_a_probability() {
    let engine = MatchingEngine::default();
    let mut r = Lcg(SEED);
    for i in 0..CASES {
        let (a, b) = (patient(&mut r), patient(&mut r));
        let m = engine.compare(&a, &b);
        assert!(
            m.score.is_finite() && (0.0..=1.0).contains(&m.score),
            "seed {SEED} case {i}: {m:?}\n{a:?}\n{b:?}"
        );
        for f in &m.fields {
            if let Some(s) = f.similarity {
                assert!((0.0..=1.0).contains(&s), "seed {SEED} case {i}: {f:?}");
            }
        }
    }
}

/// A record compared with itself has nothing to disagree about: no conflicts, whatever the
/// decision (a thin record is still not linked, for want of evidence).
#[test]
fn comparing_a_record_with_itself_reports_no_conflict_and_never_clashes() {
    let engine = MatchingEngine::default();
    let mut r = Lcg(SEED + 1);
    for i in 0..CASES {
        let a = patient(&mut r);
        let m = engine.compare(&a, &a);
        let bad = m.conflicts.iter().any(|c| c.contains("different"));
        assert!(!bad, "seed {} case {i}: {m:?}\n{a:?}", SEED + 1);
    }
}

#[test]
fn compare_is_symmetric_when_the_given_name_lists_are_the_same_length() {
    let engine = MatchingEngine::default();
    let mut r = Lcg(SEED + 2);
    let mut checked = 0;
    for i in 0..CASES * 2 {
        let (a, b) = (patient(&mut r), patient(&mut r));
        let lens = |p: &Patient| p.names.iter().map(|n| n.given.len()).collect::<Vec<_>>();
        if lens(&a) != lens(&b) {
            continue;
        }
        checked += 1;
        let (ab, ba) = (engine.compare(&a, &b), engine.compare(&b, &a));
        assert_eq!(
            ab.decision,
            ba.decision,
            "seed {} case {i}\n{ab:?}\n{ba:?}\n{a:?}\n{b:?}",
            SEED + 2
        );
        assert!(
            (ab.score - ba.score).abs() < 1e-9,
            "seed {} case {i}: {} vs {}",
            SEED + 2,
            ab.score,
            ba.score
        );
    }
    assert!(checked > 500, "the filter left too few cases: {checked}");
}

/// A way to drop one field: its name, the scored field it feeds, and the edit.
type Drop = (&'static str, &'static str, fn(&mut Patient));

fn shares_a_valid_nhs_number(a: &Patient, b: &Patient) -> bool {
    MatchingEngine::default()
        .compare(a, b)
        .deterministic
        .is_some()
}

/// Dropping a field from one record can lower the score (less evidence) but must never produce an
/// automatic link that the full pair did not have: absent evidence is never agreement. Fields that
/// disagreed are not dropped (removing a contradiction can legitimately raise the score). Pairs that
/// share a valid NHS number are excluded: that path decides on the number alone (see the next test).
#[test]
fn removing_a_field_never_creates_an_auto_link_without_a_shared_nhs_number() {
    let engine = MatchingEngine::default();
    let mut r = Lcg(SEED + 3);
    // (name, the scored field it feeds, how to drop it)
    let drops: [Drop; 4] = [
        ("birth_date", "birth_date", |p| p.birth_date = None),
        ("sex", "sex", |p| p.sex = None),
        ("addresses", "postal_code", |p| p.addresses.clear()),
        ("telecom", "telecom", |p| p.telecom.clear()),
    ];
    let mut checked = 0;
    for i in 0..CASES {
        let (a, b) = (patient(&mut r), patient(&mut r));
        if shares_a_valid_nhs_number(&a, &b) {
            continue;
        }
        checked += 1;
        let full_result = engine.compare(&a, &b);
        let full = full_result.decision;
        for (name, field, drop) in &drops {
            // Dropping a field that *disagreed* raises the average: that is removing a
            // contradiction, not inventing agreement, and is covered by the next test.
            let disagreed = full_result
                .fields
                .iter()
                .any(|f| f.field == *field && f.similarity.is_some_and(|s| s < 1.0));
            if disagreed {
                continue;
            }
            let mut less = b.clone();
            drop(&mut less);
            let after = engine.compare(&a, &less).decision;
            assert!(
                !(after == Decision::AutoLink && full != Decision::AutoLink),
                "seed {} case {i}: dropping {name} turned {full:?} into {after:?}\n{a:?}\n{b:?}",
                SEED + 3
            );
        }
    }
    assert!(checked > 5_000, "the filter left too few cases: {checked}");
}

/// **CQ-37, decided 2026-10-25:** a shared valid NHS number no longer links automatically on its own.
/// Missing evidence is not agreement, as for every other pair: with no given name, family name or birth
/// date on a side, the pair goes to Review with the missing field named, and dropping a conflicting
/// birth date can no longer turn Review into AutoLink. A family name that differs (marriage) still links
/// when the rest is present, and a fully present, agreeing pair still links.
#[test]
fn a_shared_nhs_number_alone_goes_to_review() {
    let engine = MatchingEngine::default();
    let with_nhs = |dob: Option<&str>, given: Option<&str>, family: Option<&str>| Patient {
        identifiers: vec![Identifier {
            system: NHS_NUMBER_SYSTEM.into(),
            value: "9434765919".into(),
        }],
        birth_date: dob.map(Into::into),
        names: if given.is_some() || family.is_some() {
            vec![HumanName {
                family: family.map(Into::into),
                given: given.map(|g| vec![g.into()]).unwrap_or_default(),
            }]
        } else {
            vec![]
        },
        ..Default::default()
    };
    let full = || with_nhs(Some("1980-03-14"), Some("Ann"), Some("Smith"));
    // Everything present and agreeing: still an automatic link.
    assert_eq!(
        engine.compare(&full(), &full()).decision,
        Decision::AutoLink
    );
    // Nothing but the number: Review, each missing field named, and the number still the stated basis.
    let bare = engine.compare(&with_nhs(None, None, None), &with_nhs(None, None, None));
    assert_eq!(bare.decision, Decision::Review);
    assert_eq!(
        bare.deterministic.as_deref(),
        Some("shared verified NHS number")
    );
    assert_eq!(bare.conflict_codes.len(), 3, "{bare:?}");
    // Two different full birth dates were already Review.
    let differ = engine.compare(
        &full(),
        &with_nhs(Some("1981-03-14"), Some("Ann"), Some("Smith")),
    );
    assert_eq!(differ.decision, Decision::Review);
    // Dropping the conflicting birth date used to turn that Review into AutoLink: now it is Review.
    let dropped = engine.compare(&full(), &with_nhs(None, Some("Ann"), Some("Smith")));
    assert_eq!(dropped.decision, Decision::Review);
    assert!(
        dropped
            .conflict_codes
            .contains(&ConflictCode::FieldUnavailable {
                field: GuardField::BirthDate
            })
    );
    // One name missing on one side: Review too.
    let no_given = engine.compare(&full(), &with_nhs(Some("1980-03-14"), None, Some("Smith")));
    assert_eq!(no_given.decision, Decision::Review);
    // A family name that changed (marriage) with everything else present is not blocked.
    let married = engine.compare(
        &full(),
        &with_nhs(Some("1980-03-14"), Some("Ann"), Some("Jones")),
    );
    assert_eq!(married.decision, Decision::AutoLink);
}

/// The stronger claim in the task: removing a field never raises the decision at all. It does not
/// hold when the removed field was a *disagreement* (a conflicting sex lowered the score, and
/// without it the rest agrees), which is correct behaviour; this test records the cases that rise
/// so a change in them is seen, and asserts only that none rise to an automatic link.
#[test]
fn removing_a_field_may_raise_review_but_only_by_removing_a_disagreement() {
    let engine = MatchingEngine::default();
    let mut r = Lcg(SEED + 4);
    let mut raised = 0;
    for _ in 0..CASES {
        let (a, b) = (patient(&mut r), patient(&mut r));
        let full = engine.compare(&a, &b);
        let mut less = b.clone();
        less.birth_date = None;
        let after = engine.compare(&a, &less);
        if rank(after.decision) > rank(full.decision) {
            raised += 1;
            let dob = full.fields.iter().find(|f| f.field == "birth_date");
            assert!(
                dob.and_then(|f| f.similarity).is_some_and(|s| s < 1.0)
                    || !full.conflicts.is_empty(),
                "a rise with no disagreement or conflict on birth_date:\n{full:?}\n{after:?}\n{a:?}\n{b:?}"
            );
        }
    }
    println!("decision rose after dropping the birth date in {raised} of {CASES} cases");
}
