//! Benchmarks for the matcher (SF-2, spec/performance-testing.md).
//!
//! `cargo bench --bench matcher` measures; `cargo bench --bench matcher -- --test` runs each benchmark once, as a
//! check that it still works. The numbers are for one machine and say nothing about a deployment.
//!
//! The groups follow the work a registration does: fold and compare names, check identifiers and dates, compare
//! two records, and score one record against a register of candidates (the service's search returns at most 50).

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use electronic_patient_record_matcher::normalize::{
    are_nicknames, fold_name, fold_postcode, normalize_phone, parse_date, soundex, transliterate,
};
use electronic_patient_record_matcher::{
    Address, AdministrativeSex, HumanName, Identifier, MatchingEngine, NATIONAL_NUMBER_SYSTEM,
    Patient, Telecom, is_valid_ihi, is_valid_national_number,
};
use std::hint::black_box;

/// A valid national number (modulus 11), synthetic.
const NATIONAL: &str = "9434765919";

fn patient(family: &str, given: &str, born: &str, national: Option<&str>) -> Patient {
    Patient {
        names: vec![HumanName {
            family: Some(family.to_string()),
            given: vec![given.to_string()],
        }],
        birth_date: Some(born.to_string()),
        sex: Some(AdministrativeSex::Female),
        identifiers: national
            .map(|v| {
                vec![Identifier {
                    system: NATIONAL_NUMBER_SYSTEM.to_string(),
                    value: v.to_string(),
                }]
            })
            .unwrap_or_default(),
        addresses: vec![Address {
            postal_code: Some("CF10 1AA".to_string()),
        }],
        telecom: vec![Telecom {
            system: "phone".to_string(),
            value: "029 2000 0000".to_string(),
        }],
        ..Patient::default()
    }
}

fn normalize(c: &mut Criterion) {
    let mut g = c.benchmark_group("normalize");
    for name in [
        "Smith",
        "O'Brien-Lloyd",
        "Ffion Siân Ap Gruffudd",
        "Zoë Ångström-Müller",
    ] {
        g.bench_with_input(BenchmarkId::new("fold_name", name), name, |b, n| {
            b.iter(|| fold_name(black_box(n)))
        });
    }
    let folded = fold_name("Zoë Ångström-Müller");
    g.bench_function("transliterate", |b| {
        b.iter(|| transliterate(black_box(&folded)))
    });
    g.bench_function("soundex", |b| b.iter(|| soundex(black_box("ashcraft"))));
    g.bench_function("are_nicknames", |b| {
        b.iter(|| are_nicknames(black_box("william"), black_box("bill")))
    });
    g.bench_function("parse_date", |b| {
        b.iter(|| parse_date(black_box("1984-02-29")))
    });
    g.bench_function("fold_postcode", |b| {
        b.iter(|| fold_postcode(black_box(" cf10 1aa ")))
    });
    g.bench_function("normalize_phone", |b| {
        b.iter(|| normalize_phone(black_box("029 2000 0000"), 44))
    });
    g.finish();
}

fn identifiers(c: &mut Criterion) {
    assert!(
        is_valid_national_number(NATIONAL),
        "the benchmark's national number must be valid"
    );
    let mut g = c.benchmark_group("identifiers");
    g.bench_function("national_number_valid", |b| {
        b.iter(|| is_valid_national_number(black_box(NATIONAL)))
    });
    g.bench_function("national_number_invalid", |b| {
        b.iter(|| is_valid_national_number(black_box("9434765918")))
    });
    g.bench_function("ihi", |b| {
        b.iter(|| is_valid_ihi(black_box("8003608166690503")))
    });
    g.finish();
}

fn compare(c: &mut Criterion) {
    let engine = MatchingEngine::default();
    let a = patient("Jones", "Ffion", "1984-02-29", Some(NATIONAL));
    let cases = [
        ("identical", a.clone()),
        (
            "typo_in_family",
            patient("Jonse", "Ffion", "1984-02-29", Some(NATIONAL)),
        ),
        (
            "nickname_no_identifier",
            patient("Jones", "Fi", "1984-02-29", None),
        ),
        (
            "conflicting_identifier",
            patient("Jones", "Ffion", "1984-02-29", Some("4010232137")),
        ),
        (
            "different_person",
            patient("Pritchard", "Owen", "1951-11-03", None),
        ),
        (
            "sparse",
            Patient {
                names: vec![HumanName {
                    family: Some("Jones".to_string()),
                    given: vec![],
                }],
                ..Patient::default()
            },
        ),
    ];
    let mut g = c.benchmark_group("compare");
    for (name, b_patient) in &cases {
        g.bench_with_input(BenchmarkId::from_parameter(name), b_patient, |b, other| {
            b.iter(|| engine.compare(black_box(&a), black_box(other)))
        });
    }
    g.finish();
}

/// One incoming record scored against a register page, as `POST /api/patients` does with the candidates it finds.
fn register(c: &mut Criterion) {
    let engine = MatchingEngine::default();
    let incoming = patient("Jones", "Ffion", "1984-02-29", None);
    let families = [
        "Jones", "Jonas", "Johns", "Jameson", "Davies", "Evans", "Thomas", "Roberts",
    ];
    let givens = [
        "Ffion", "Fiona", "Sian", "Megan", "Owen", "Rhys", "Cerys", "Dylan",
    ];
    let mut g = c.benchmark_group("register");
    for size in [1usize, 50, 1_000] {
        let candidates: Vec<Patient> = (0..size)
            .map(|i| {
                let born = format!("19{:02}-{:02}-{:02}", 40 + i % 60, 1 + i % 12, 1 + i % 28);
                patient(
                    families[i % families.len()],
                    givens[(i / 8) % givens.len()],
                    &born,
                    None,
                )
            })
            .collect();
        g.throughput(Throughput::Elements(size as u64));
        g.bench_with_input(BenchmarkId::from_parameter(size), &candidates, |b, list| {
            b.iter(|| {
                list.iter()
                    .map(|p| engine.compare(black_box(&incoming), p).score)
                    .fold(0.0_f64, f64::max)
            })
        });
    }
    g.finish();
}

criterion_group!(benches, normalize, identifiers, compare, register);
criterion_main!(benches);
