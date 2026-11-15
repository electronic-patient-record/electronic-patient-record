//! Benchmarks for the service's pure work on each request (spec/performance-testing.md).
//!
//! `cargo bench --bench service` measures; `cargo bench --bench service -- --test` runs each benchmark once, as a
//! check that it still works. Nothing here touches the database: the database's share of a request is what the
//! load tests in `scripts/load/` measure. The numbers are for one machine and say nothing about a deployment.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use electronic_patient_record::audit::{Action, Chain, Event};
use electronic_patient_record::interop::hl7_adt::patient_from_adt;
use electronic_patient_record::observations::{self, Observation, Range};
use electronic_patient_record::policy::{
    self, Action as PolicyAction, Consent, Purpose, Relationship, Request, Resource, Role, Subject,
};
use electronic_patient_record::{metrics, packs, patients_store};
use electronic_patient_record_matcher::{HumanName, Identifier, NATIONAL_NUMBER_SYSTEM, Patient};
use std::collections::BTreeSet;
use std::hint::black_box;

fn patient() -> Patient {
    Patient {
        names: vec![HumanName {
            family: Some("Jones".to_string()),
            given: vec!["Ffion".to_string()],
        }],
        birth_date: Some("1984-02-29".to_string()),
        identifiers: vec![Identifier {
            system: NATIONAL_NUMBER_SYSTEM.to_string(),
            value: "9434765919".to_string(),
        }],
        ..Patient::default()
    }
}

/// The audit row's digest: computed for every audited request, inside the chain's lock (CQ-18).
fn audit(c: &mut Criterion) {
    let chain = Chain::unkeyed();
    let mut event = Event::new(Action::Read, "0c4f1e2a-0000-4000-8000-0000000000e1");
    event.patient_ref = Some("Patient/6f1c1b8e-9d7a-4a7e-8f00-000000000001".to_string());
    event.purpose_of_use = Some("treatment".to_string());
    event.detail = Some(serde_json::json!({ "route": "/api/patients/{id}", "status": 200 }));
    let prev = "sha256:".to_string() + &"ab".repeat(32);
    c.bench_function("audit/digest_unkeyed", |b| {
        b.iter(|| {
            chain.digest(
                black_box(Some(&prev)),
                black_box("2026-11-15T10:00:00.000000Z"),
                black_box(&event),
            )
        })
    });
}

/// The record decision, made on every read and write of a record.
fn policy(c: &mut Criterion) {
    let mut g = c.benchmark_group("policy");
    let cases = [
        (
            "clinician_reads",
            Role::Clinician,
            PolicyAction::Read,
            Relationship::CareTeam,
            Purpose::Treatment,
        ),
        (
            "nurse_writes",
            Role::Nurse,
            PolicyAction::Create,
            Relationship::CareTeam,
            Purpose::Treatment,
        ),
        (
            "no_relationship",
            Role::Clinician,
            PolicyAction::Read,
            Relationship::None,
            Purpose::Treatment,
        ),
    ];
    for (name, role, action, relationship, purpose) in cases {
        let request = Request {
            subject: Subject {
                roles: BTreeSet::from([role]),
                ..Subject::default()
            },
            resource: Resource::PatientIdentity,
            action,
            labels: BTreeSet::new(),
            relationship,
            purpose,
            consent: Consent::NotRecorded,
            break_glass: None,
            now: 1_000,
        };
        g.bench_with_input(BenchmarkId::from_parameter(name), &request, |b, r| {
            b.iter(|| policy::evaluate(black_box(r)))
        });
    }
    g.finish();
}

/// What a registration does before it reaches the database: validate, then score against a candidate.
fn patients(c: &mut Criterion) {
    let p = patient();
    let body =
        serde_json::to_string(&serde_json::json!({ "patient": p, "reason": "benchmark" })).unwrap();
    let mut g = c.benchmark_group("patients");
    g.bench_function("parse_request", |b| {
        b.iter(|| serde_json::from_str::<serde_json::Value>(black_box(&body)).unwrap())
    });
    g.bench_function("validate_coded", |b| {
        b.iter(|| patients_store::validate_coded(black_box(&p)))
    });
    let engine = packs::engine();
    let mut other = patient();
    other.names[0].family = Some("Jonse".to_string());
    g.bench_function("compare_with_site_rules", |b| {
        b.iter(|| engine.compare(black_box(&p), black_box(&other)))
    });
    g.finish();
}

fn observation(c: &mut Criterion) {
    let o = Observation {
        code: "8867-4".to_string(),
        value: Some(112.0),
        unit: Some("/min".to_string()),
        absent_reason: None,
        range: Some(Range {
            low: Some(60.0),
            high: Some(100.0),
        }),
        critical: None,
    };
    let mut g = c.benchmark_group("observation");
    g.bench_function("validate", |b| {
        b.iter(|| observations::validate(black_box(&o)))
    });
    g.bench_function("flag", |b| b.iter(|| observations::flag(black_box(&o))));
    g.finish();
}

/// One inbound HL7 v2 registration, parsed into a patient (the MLLP feed's per-message work).
fn hl7(c: &mut Criterion) {
    let message = "MSH|^~\\&|REG|CLINIC|EPR|HOSP|20261021090000||ADT^A04^ADT_A01|MSG0001|P|2.5\r\
                   EVN|A04|20261021090000\r\
                   PID|1||12345^^^LOCAL||Jones^Ffion||19840229|F\r";
    assert!(
        patient_from_adt(message).is_ok(),
        "the benchmark's message must parse"
    );
    c.bench_function("hl7/patient_from_adt", |b| {
        b.iter(|| patient_from_adt(black_box(message)))
    });
}

/// The request-duration histogram, recorded for every request.
fn metrics_observe(c: &mut Criterion) {
    c.bench_function("metrics/observe", |b| {
        b.iter(|| {
            metrics::observe(
                black_box("GET"),
                black_box("/api/patients/{id}"),
                black_box(200),
                black_box(0.004),
            )
        })
    });
}

criterion_group!(
    benches,
    audit,
    policy,
    patients,
    observation,
    hl7,
    metrics_observe
);
criterion_main!(benches);
