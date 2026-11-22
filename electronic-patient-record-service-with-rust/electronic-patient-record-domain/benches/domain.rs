//! Benchmarks for the domain rules each request applies (spec/performance-testing.md): the record decision, a
//! patient's consents, an observation's checks, and the task rules (TL-3).
//!
//! `cargo bench -p electronic-patient-record-domain --bench domain` measures; add `-- --test` to run each benchmark
//! once, as a check that it still works. Pure functions only: no database, no web. The numbers are for one machine
//! and say nothing about a deployment.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic
)] // a benchmark may panic

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use electronic_patient_record_domain::consent::{self, ConsentRecord, Context};
use electronic_patient_record_domain::observations::{self, Observation, Range};
use electronic_patient_record_domain::policy::{
    self, Action as PolicyAction, Consent, Purpose, Relationship, Request, Resource, Role, Subject,
};
use electronic_patient_record_domain::task_rules::{self, Event, Late};
use jiff::civil::Date;
use std::collections::{BTreeMap, BTreeSet};
use std::hint::black_box;

/// A patient's consents reduced to one value, on every disclosure of a record (SE-2): 1, 10 and 50 active consents,
/// each a deny or permit for some roles and purposes, none deciding until the last.
fn consent_decide(c: &mut Criterion) {
    let mut g = c.benchmark_group("consent/decide");
    let ctx = Context {
        roles: BTreeSet::from([Role::Clinician]),
        purpose: Purpose::Treatment,
        resource: Resource::PatientIdentity,
        labels: BTreeSet::new(),
        now: 1_763_373_600,
    };
    for n in [1usize, 10, 50] {
        let records: Vec<(String, ConsentRecord)> = (0..n)
            .map(|i| {
                let effect = if i + 1 == n { "deny" } else { "permit" };
                let json = format!(
                    r#"{{"status":"active","period":{{"start":"2026-01-01","end":"2027-12-31"}},
                        "provisions":[{{"effect":"{effect}","actors":["clinician","nurse"],
                        "purposes":["treatment","emergency"],"resources":["PatientIdentity"]}}]}}"#
                );
                (
                    format!("consent-{i}"),
                    serde_json::from_str(&json).expect("a consent"),
                )
            })
            .collect();
        g.bench_with_input(BenchmarkId::from_parameter(n), &records, |b, records| {
            b.iter(|| {
                consent::decide(
                    records.iter().map(|(id, r)| (id.as_str(), r)),
                    black_box(&ctx),
                )
            })
        });
    }
    g.finish();
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

/// `n` approved rules on a filed result, each with a different condition so none is a duplicate (TL-14); one in two
/// matches an abnormal result.
fn rules_text(n: usize) -> String {
    (0..n)
        .map(|i| {
            let value = if i % 2 == 0 { "true" } else { "false" };
            format!(
                r#"
[[rule]]
id = "rule-{i}"
version = 1
title = "Rule {i}"
approved_by = "A Clinician"
approved_on = "2026-11-17"
event = "result-filed"
cancel_on = ["result-signed-off"]
[[rule.when]]
fact = "abnormal"
any_of = ["{value}"]
[[rule.when]]
fact = "test"
any_of = ["test-{i}"]
[rule.task]
team = "team-{i}"
title = "Review"
due_working_days = 5
remind_working_days_before = [1, 3]
late_reminder = "send-once"
"#
            )
        })
        .collect()
}

/// A year of a site's holidays: eight days, as a calendar holds them.
fn holidays() -> BTreeSet<Date> {
    [
        "2026-12-25",
        "2026-12-28",
        "2027-01-01",
        "2027-03-29",
        "2027-05-03",
        "2027-05-31",
        "2027-08-30",
        "2027-12-27",
    ]
    .iter()
    .map(|d| d.parse().expect("a date"))
    .collect()
}

/// The task rules: the file checked when it loads (1, 10 and 50 rules); what one event would create, which is also
/// the dry run (`plan`, over 1, 10 and 50 rules); a due date 60 working days on, the most a rule allows, across a
/// holiday season; and one reminder decision (TL-4).
fn task_rules(c: &mut Criterion) {
    let mut g = c.benchmark_group("task_rules/parse");
    for n in [1usize, 10, 50] {
        let text = rules_text(n);
        g.bench_with_input(BenchmarkId::from_parameter(n), &text, |b, text| {
            b.iter(|| task_rules::parse(black_box(text), "bench").expect("rules"))
        });
    }
    g.finish();

    let holidays = holidays();
    let day: Date = "2026-12-23".parse().expect("a date");
    let mut g = c.benchmark_group("task_rules/plan");
    for n in [1usize, 10, 50] {
        let rules = task_rules::parse(&rules_text(n), "bench").expect("rules");
        let facts: BTreeMap<String, String> = [
            ("abnormal".to_string(), "true".to_string()),
            ("test".to_string(), "test-0".to_string()),
        ]
        .into_iter()
        .collect();
        g.bench_with_input(BenchmarkId::from_parameter(n), &rules, |b, rules| {
            b.iter(|| {
                task_rules::plan(
                    black_box(rules),
                    Event::ResultFiled,
                    black_box(&facts),
                    day,
                    &holidays,
                )
            })
        });
    }
    g.finish();

    let mut g = c.benchmark_group("task_rules");
    g.bench_function("add_working_days_60", |b| {
        b.iter(|| task_rules::add_working_days(black_box(day), black_box(60), &holidays))
    });
    let on: Date = "2026-12-24".parse().expect("a date");
    let today: Date = "2026-12-29".parse().expect("a date");
    g.bench_function("reminder_now_late", |b| {
        b.iter(|| task_rules::reminder_now(black_box(on), today, Some(day), Late::Skip))
    });
    g.finish();
}

criterion_group!(benches, consent_decide, policy, observation, task_rules);
criterion_main!(benches);
