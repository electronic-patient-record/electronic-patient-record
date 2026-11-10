//! The PAS contract for suspicions (RS-5): every body the EPR would send is checked against the PAS's
//! own request schemas, copied from the PAS repository at commit b64b7e2 into `tests/fixtures/pas/`.
//! The check is the subset of JSON Schema those schemas use: required fields, no field the schema does
//! not name, `string`/`null` types and enums. No database, no network. The PAS side runs its own tests
//! against the same schemas; nothing here proves the copy is current.

use chrono::{DateTime, Utc};
use electronic_patient_record::pas_suspicion::{self, ActiveProfile};
use electronic_patient_record::suspicion::{
    Grounds, GroundsKind, Outcome, OutcomeKind, PatientTold, Source, Subtype, Suspicion,
};
use serde_json::Value;

fn schemas() -> Value {
    let v: Value =
        serde_json::from_str(include_str!("fixtures/pas/suspicion-schemas.json")).unwrap();
    v["schemas"].clone()
}

fn profile(text: &str) -> ActiveProfile {
    serde_json::from_str(text).unwrap()
}

/// The problems with `body` against the schema `name`, resolving `$ref`s within the copy.
fn conform(all: &Value, name: &str, body: &Value) -> Vec<String> {
    let schema = &all[name];
    let mut out = Vec::new();
    let props = schema["properties"].as_object().expect(name);
    for r in schema["required"].as_array().into_iter().flatten() {
        let r = r.as_str().unwrap();
        if body.get(r).is_none() {
            out.push(format!("{name}: {r} is required"));
        }
    }
    for (k, v) in body.as_object().unwrap() {
        let Some(p) = props.get(k) else {
            out.push(format!("{name}: {k} is not in the PAS schema"));
            continue;
        };
        let p = match p.get("$ref").and_then(Value::as_str) {
            Some(r) => &all[r.trim_start_matches("#/components/schemas/")],
            None => p,
        };
        if let Some(e) = p.get("enum").and_then(Value::as_array)
            && !e.contains(v)
        {
            out.push(format!("{name}: {k} = {v} is not one of the PAS's values"));
        }
        let types: Vec<&str> = match &p["type"] {
            Value::String(t) => vec![t.as_str()],
            Value::Array(ts) => ts.iter().filter_map(Value::as_str).collect(),
            _ => vec![],
        };
        let actual = match v {
            Value::Null => "null",
            Value::String(_) => "string",
            _ => "other",
        };
        if !types.is_empty() && !types.contains(&actual) {
            out.push(format!("{name}: {k} is {actual}, not {types:?}"));
        }
        if p.get("format").and_then(Value::as_str) == Some("date") {
            let ok = v
                .as_str()
                .is_some_and(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").is_ok());
            if !ok {
                out.push(format!("{name}: {k} is not a date"));
            }
        }
    }
    out
}

fn suspicion(pathway: &str, version: i32, subtype: Option<(&str, &str)>) -> Suspicion {
    Suspicion {
        condition: "Suspected".into(),
        code: None,
        subtype: subtype.map(|(s, c)| Subtype {
            scheme: s.into(),
            code: c.into(),
        }),
        point_of_suspicion: "2026-11-09T09:30:00Z".into(),
        source: Source::Clinician,
        by: "Dr Gwen Hughes".into(),
        grounds: Grounds {
            kind: GroundsKind::Judgement,
            detail: "seen".into(),
            result_id: None,
        },
        instruction: None,
        patient_told: PatientTold::Told,
        pathway: pathway.into(),
        pathway_version: version,
        band: None,
        factors: vec![],
        agreed: None,
    }
}

#[test]
fn every_body_the_epr_sends_meets_the_pas_schemas() {
    let all = schemas();
    let wales = profile(include_str!("fixtures/pas/profile-wales.json"));
    let generic = profile(include_str!("fixtures/pas/profile-generic.json"));
    let cases = [
        (
            suspicion(
                "suspected-cancer-wales",
                2,
                Some(("wales-primary-cancer-site", "03")),
            ),
            &wales,
        ),
        (suspicion("type-1-diabetes-child", 1, None), &generic),
        (
            suspicion("dementia", 2, Some(("generic-dementia-type", "vascular"))),
            &generic,
        ),
        (suspicion("parkinsons", 1, None), &wales),
        (suspicion("heart-failure", 1, None), &generic),
        (suspicion("obstructive-sleep-apnoea", 1, None), &generic),
        (suspicion("suspected-cancer-england", 1, None), &generic),
    ];
    for (s, p) in &cases {
        let body = pas_suspicion::new_suspicion(s, "Rhys Evans", p)
            .unwrap_or_else(|h| panic!("{}: {}", s.pathway, h.0));
        let body = serde_json::to_value(body).unwrap();
        assert_eq!(
            conform(&all, "NewSuspicion", &body),
            Vec::<String>::new(),
            "{body}"
        );
        // The condition is one the profile tracks, as the PAS checks.
        let code = body["condition"].as_str().unwrap();
        assert!(p.conditions.iter().any(|c| c.code == code), "{code}");
    }

    let was = &cases[0].0;
    let mut now = was.clone();
    now.point_of_suspicion = "2026-11-02T10:00:00Z".into();
    now.subtype.as_mut().unwrap().code = "02".into();
    let amend = pas_suspicion::amendment(was, &now, "seen a week before", "Rhys Evans", &wales)
        .unwrap()
        .unwrap();
    let amend = serde_json::to_value(amend).unwrap();
    assert_eq!(
        conform(&all, "Amendment", &amend),
        Vec::<String>::new(),
        "{amend}"
    );

    let at: DateTime<Utc> = "2026-11-20T10:00:00Z".parse().unwrap();
    for kind in [OutcomeKind::RuledOut, OutcomeKind::Withdrawn] {
        let o = Outcome {
            kind,
            diagnosis: None,
            code: None,
            by: Some("Mr Jones".into()),
            detail: "colonoscopy".into(),
        };
        let stop = pas_suspicion::stop(&o, at, "Rhys Evans", &wales)
            .unwrap()
            .unwrap();
        let stop = serde_json::to_value(stop).unwrap();
        assert_eq!(
            conform(&all, "handlers_suspicions.StopBody", &stop),
            Vec::<String>::new(),
            "{stop}"
        );
    }
}

#[test]
fn the_checker_itself_refuses_a_bad_body() {
    let all = schemas();
    let bad = serde_json::json!({ "condition": "cancer", "grounds": "x", "suspected_on": "9 Nov" });
    let problems = conform(&all, "NewSuspicion", &bad);
    for words in [
        "suspected_by is required",
        "grounds is not in",
        "not a date",
    ] {
        assert!(problems.iter().any(|p| p.contains(words)), "{problems:?}");
    }
    let stop =
        serde_json::json!({ "reason": "ruled-out", "stopped_on": "2026-11-20", "stopped_by": "R" });
    assert!(!conform(&all, "handlers_suspicions.StopBody", &stop).is_empty());
}
