//! DB-gated (QA-5), through the real router: nothing that identifies a person, and nothing clinical, reaches a log line.
//! Every level is captured, `TRACE` included, while one marked patient is registered, found, read, changed, refused and
//! recorded about. A marker is in every value sent: names, numbers, postcode, telephone, birth date, free text; and
//! the record's own id is looked for too.
//!
//! What it does not show: lines a background task writes on another thread, a line written only when the database or
//! the identity provider fails, routes not called here, and the container's or a proxy's own access log.
//!
//! ```text
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test main http_log_privacy:: -- --ignored --test-threads=1
//! ```

use axa_test::prelude::request_with;
use axum::http::StatusCode;
use electronic_patient_record::app::App;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
struct Lines(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for Lines {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

/// Letters no earlier run shares.
fn marker() -> String {
    let mut n = nanos();
    let mut letters = "Qzlog".to_string();
    while n > 0 {
        letters.push(char::from(b'a' + u8::try_from(n % 26).unwrap()));
        n /= 26;
    }
    letters
}

/// A national number that passes its check digit, from `seed`.
fn national_number(seed: u128) -> String {
    (0..)
        .map(|i| 100_000_000 + (seed + i) % 899_999_999)
        .find_map(|n| {
            let digits: Vec<u128> = n
                .to_string()
                .chars()
                .map(|c| u128::from(c.to_digit(10).unwrap()))
                .collect();
            let sum: u128 = digits
                .iter()
                .zip((2..=10).rev())
                .fold(0, |total, (d, w)| total + d * w);
            match 11 - sum % 11 {
                11 => Some(format!("{n}0")),
                10 => None,
                check => Some(format!("{n}{check}")),
            }
        })
        .unwrap()
}

fn settings() -> Vec<(&'static str, &'static str)> {
    let url: &'static str = Box::leak(
        std::env::var("DB_URL")
            .expect("DB_URL must be set for --ignored tests")
            .into_boxed_str(),
    );
    vec![("DB_URL", url), ("EPR_REQUIRE_AUTH", "false")]
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn nothing_about_a_person_reaches_a_log_line() {
    let lines = Lines::default();
    let writer = lines.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_writer(move || writer.clone())
        .with_ansi(false)
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);

    let mark = marker();
    let number = national_number(nanos() / 1000);
    let born = format!(
        "{}-03-{:02}",
        1900 + nanos() / 1000 % 100,
        1 + nanos() / 100_000 % 28
    );
    let (m, n, b) = (mark.clone(), number.clone(), born.clone());
    let record = Arc::new(Mutex::new(String::new()));
    let kept = record.clone();
    request_with::<App, _, _>(&settings(), |server, _ctx| async move {
        let person = json!({
            "names": [{"family": m, "given": [m]}],
            "birth_date": b,
            "identifiers": [{"system": "https://fhir.nhs.uk/Id/nhs-number", "value": n}],
            "addresses": [{"postal_code": format!("{m}1")}],
            "telecom": [{"system": "phone", "value": "01632 960987"}],
        });
        let create = json!({"patient": person, "override_reason": format!("{m} twin")});
        let made = server.post("/api/patients").json(&create).await;
        assert_eq!(made.status_code(), StatusCode::CREATED, "{}", made.text());
        let id = made.json::<Value>()["id"].as_str().unwrap().to_string();
        kept.lock().unwrap().clone_from(&id);

        // Found by each criterion, in the query string and through FHIR.
        for url in [
            format!("/api/patients?family={m}"),
            format!("/api/patients?birth_date={b}"),
            format!("/api/patients?identifier=https://fhir.nhs.uk/Id/nhs-number|{n}"),
            format!("/fhir/R4/Patient?family={m}&birthdate={b}"),
            format!("/fhir/R5/Patient?identifier=https://fhir.nhs.uk/Id/nhs-number|{n}"),
            format!("/api/patients/{id}"),
            format!("/fhir/R4/Patient/{id}"),
            format!("/api/patients/{id}/allergies"),
            format!("/fhir/R4/AllergyIntolerance?patient={id}"),
            format!("/api/patients/{id}/observations"),
        ] {
            let got = server.get(&url).await;
            assert_eq!(got.status_code(), StatusCode::OK, "{url}: {}", got.text());
        }

        // Written, and refused: a duplicate, a bad body, a bad query, a stale version, an unknown route.
        let allergy = json!({"allergy": {"substance": format!("{m} pollen"), "status": "active", "reaction": m}, "reason": m});
        let made = server
            .post(&format!("/api/patients/{id}/allergies"))
            .json(&allergy)
            .await;
        assert_eq!(made.status_code(), StatusCode::CREATED, "{}", made.text());
        let problem = json!({"record": {"title": format!("{m} disease"), "certainty": "confirmed", "status": "active"}});
        let made = server
            .post(&format!("/api/patients/{id}/problems"))
            .json(&problem)
            .await;
        assert_eq!(made.status_code(), StatusCode::CREATED, "{}", made.text());
        let entry = made.json::<Value>()["id"].as_str().unwrap().to_string();
        for (method, url, body, status) in [
            ("POST", "/api/patients".to_string(), json!({"patient": create["patient"]}), 409),
            ("POST", "/api/patients".to_string(), json!({"patient": {"names": m}}), 422),
            ("POST", "/api/patients".to_string(), json!({"patient": {"names": [{"family": m}], "birth_date": m}}), 422),
            ("GET", format!("/fhir/R4/Patient?{m}={n}"), Value::Null, 400),
            ("GET", format!("/api/patients/{m}"), Value::Null, 404),
            ("GET", format!("/api/{m}/{n}"), Value::Null, 404),
            ("PUT", format!("/api/patients/{id}/problems/{entry}"),
             json!({"record": problem["record"], "expected_version": 9, "reason": m}), 409),
            ("POST", format!("/api/patients/{id}/problems"),
             json!({"record": {"title": m, "certainty": m, "status": "active"}}), 422),
        ] {
            let request = match method {
                "GET" => server.get(&url),
                "PUT" => server.put(&url).json(&body),
                _ => server.post(&url).json(&body),
            };
            let got = request.await;
            assert_eq!(got.status_code().as_u16(), status, "{method} {url}: {}", got.text());
        }
    })
    .await;

    let captured = String::from_utf8(lines.0.lock().unwrap().clone()).unwrap();
    // Every request above was seen to finish, so the capture reaches the code that answers.
    let finished = captured.matches("finished processing request").count();
    assert!(finished >= 20, "{finished} requests captured:\n{captured}");
    let mut told: Vec<&str> = Vec::new();
    let id = record.lock().unwrap().clone();
    for secret in [
        mark.as_str(),
        &number,
        &born,
        &id,
        "01632 960987",
        "01632960987",
    ] {
        for line in captured.lines().filter(|l| l.contains(secret)) {
            told.push(line);
        }
    }
    told.dedup();
    assert!(
        told.is_empty(),
        "{} lines name the person, such as:\n{}",
        told.len(),
        told.iter().take(12).copied().collect::<Vec<_>>().join("\n")
    );
}
