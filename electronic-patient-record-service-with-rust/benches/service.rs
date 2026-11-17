//! Benchmarks for the service's pure work on each request (spec/performance-testing.md).
//!
//! `cargo bench --bench service` measures; `cargo bench --bench service -- --test` runs each benchmark once, as a
//! check that it still works. The domain rules alone (policy, consent, observations, task rules) are measured in
//! `electronic-patient-record-domain/benches/domain.rs`, so changing one rebuilds only that crate. Nothing here touches the database: the database's share of a request is what the
//! load tests in `scripts/load/` measure. The numbers are for one machine and say nothing about a deployment.

use criterion::{Criterion, criterion_group, criterion_main};
use electronic_patient_record::audit::{Action, Chain, Event};
use electronic_patient_record::feed_health::{self, Facts, Thresholds};
use electronic_patient_record::interop::hl7_adt::{
    Rejection, acknowledgement, message_time, msa_text, patient_from_adt,
};
use electronic_patient_record::interop::redact;
use electronic_patient_record::mllp;
use electronic_patient_record::{metrics, packs, patients_store};
use electronic_patient_record_matcher::{HumanName, Identifier, NATIONAL_NUMBER_SYSTEM, Patient};
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

/// The same digest under a key (`EPR_INTEGRITY_MAC_KEY`, HMAC-SHA-256), as a production deployment computes it, and the
/// keyed fingerprint that replaces an HL7 control id before anything is written (IH-3).
fn audit_keyed(c: &mut Criterion) {
    // A made-up key, built here so no key-shaped text sits in the source (the secret scan, OS-11): the SHA-256 of a
    // fixed phrase, in hex. A key of one repeated byte is refused, so it must look like a real one.
    let key: String =
        <sha2::Sha256 as sha2::Digest>::digest(b"electronic-patient-record benchmark key")
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
    let chain = Chain::new(integrity_mac::KeySet::load(
        &integrity_mac::KeyConfig::new("bench", "BENCH"),
        Some(&key),
        None,
    ));
    assert!(chain.keyed(), "the benchmark's chain must be keyed");
    let mut event = Event::new(Action::Read, "0c4f1e2a-0000-4000-8000-0000000000e1");
    event.patient_ref = Some("Patient/6f1c1b8e-9d7a-4a7e-8f00-000000000001".to_string());
    event.detail = Some(serde_json::json!({ "route": "/api/patients/{id}", "status": 200 }));
    let prev = "hmac-sha256:".to_string() + &"ab".repeat(32);
    c.bench_function("audit/digest_keyed", |b| {
        b.iter(|| {
            chain.digest(
                black_box(Some(&prev)),
                black_box("2026-11-15T10:00:00.000000Z"),
                black_box(&event),
            )
        })
    });
    c.bench_function("audit/fingerprint_keyed", |b| {
        b.iter(|| {
            chain.fingerprint(
                black_box("feed-control-id"),
                black_box("mllp:adt-feed\u{1f}MSG0001"),
            )
        })
    });
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

/// The rest of an HL7 message's work outside the database (IH-2, IH-4, IH-5): the message time, the masked shape
/// that is all a log line may show, the acknowledgement for an accepted and a kept message, and MLLP framing, with
/// ten messages pipelined in one read.
fn hl7_feed(c: &mut Criterion) {
    let message = "MSH|^~\\&|REG|CLINIC|EPR|HOSP|20261021090000||ADT^A08^ADT_A01|MSG0001|P|2.5\r\
                   EVN|A08|20261021090000\r\
                   PID|1||12345^^^LOCAL~9434765919^^^NHS^NH||Jones^Ffion^Mair||19840229|F|||\
                   1 Test Street^^Testtown^^TT1 1TT||01632960001\r";
    let read = patient_from_adt(message);
    assert!(read.is_ok(), "the benchmark's message must parse");
    let kept = Err(Rejection::Kept(vec![
        "kept for review by a person; do not resend".to_string(),
        "this update is older than one this feed has already applied for this patient".to_string(),
    ]));
    let mut g = c.benchmark_group("hl7");
    g.bench_function("message_time", |b| {
        b.iter(|| message_time(black_box("20261021090000+0100")))
    });
    g.bench_function("redact_er7", |b| b.iter(|| redact::er7(black_box(message))));
    g.bench_function("acknowledgement_aa", |b| {
        b.iter(|| {
            acknowledgement(
                black_box(message),
                black_box(&read),
                "EPR1",
                "20261021090001",
            )
        })
    });
    g.bench_function("acknowledgement_kept", |b| {
        b.iter(|| {
            acknowledgement(
                black_box(message),
                black_box(&kept),
                "EPR1",
                "20261021090001",
            )
        })
    });
    let long = "a reason that goes on; ".repeat(40);
    g.bench_function("msa_text_long", |b| b.iter(|| msa_text(black_box(&long))));
    let pipelined: Vec<u8> = (0..10)
        .flat_map(|_| mllp::frame(message.as_bytes()))
        .collect();
    g.bench_function("mllp_take_ten_frames", |b| {
        b.iter(|| {
            let mut buf = pipelined.clone();
            let mut n = 0;
            while let Ok(Some(frame)) = mllp::take_frame(&mut buf, 1 << 20) {
                n += black_box(frame).len();
            }
            n
        })
    });
    g.finish();
}

/// A feed's health judged, once a minute per feed while the listener runs (IH-8).
fn feed_health_assess(c: &mut Criterion) {
    let thresholds = Thresholds::from_lookup(|_| None).expect("the default thresholds");
    let now = "2026-11-17T10:30:00Z"
        .parse::<jiff::Timestamp>()
        .expect("a time");
    let london = jiff::tz::TimeZone::get("Europe/London").expect("a zone");
    let facts = Facts {
        last_message_at: Some(now - jiff::SignedDuration::from_mins(7)),
        refusals_last_hour: 3,
        open_refusals: 2,
    };
    c.bench_function("feed_health/assess", |b| {
        b.iter(|| feed_health::assess(black_box(now), &london, black_box(&facts), &thresholds))
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
    audit_keyed,
    patients,
    hl7,
    hl7_feed,
    feed_health_assess,
    metrics_observe
);
criterion_main!(benches);
