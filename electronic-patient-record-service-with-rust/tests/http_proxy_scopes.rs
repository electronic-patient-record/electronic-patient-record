//! DB-gated, through the real router with login **on** (PA-5): a proxy reads only the parts of a record its terms
//! list, loses them when the terms end or the relationship is ended, and cannot be recorded without terms. One
//! test, because the token guard is configured once per process.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test http_proxy_scopes -- --ignored --test-threads=1
//! ```

use authentication_verifier::test_idp::{AUDIENCE, REALM, TestIdp};
use axum::http::{HeaderName, HeaderValue};
use electronic_patient_record::app::App;
use electronic_patient_record_matcher::{HumanName, Patient};
use loco_rs::testing::prelude::request;
use serde_json::{Value, json};

fn nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

fn unix() -> i64 {
    i64::try_from(nanos() / 1_000_000_000).unwrap()
}

/// A birth date no earlier run shares, so the duplicate check does not shortlist their patients.
fn dob() -> String {
    let n = nanos() / 1000;
    format!(
        "{}-{:02}-{:02}",
        1900 + n % 100,
        1 + n / 100 % 12,
        1 + n / 1200 % 28
    )
}

fn bearer(token: &str) -> (HeaderName, HeaderValue) {
    (
        HeaderName::from_static("authorization"),
        HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
    )
}

fn terms(scopes: &[&str], expires_at: i64) -> Value {
    json!({ "basis": "parental-responsibility", "scopes": scopes, "expires_at": expires_at })
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_proxy_reads_only_what_its_terms_list_and_only_until_they_end() {
    let idp = TestIdp::start().await;
    let map: Value = ["clinical-admin", "proxy"]
        .iter()
        .map(|r| ((*r).to_string(), json!({ "role": [r] })))
        .collect();
    // SAFETY: set before the app boots and before any other thread reads them; one test per process.
    unsafe {
        std::env::set_var("EPR_REQUIRE_AUTH", "true");
        std::env::set_var("EPR_KEYCLOAK_URL", &idp.base_url);
        std::env::set_var("EPR_KEYCLOAK_REALM", REALM);
        std::env::set_var("EPR_KEYCLOAK_AUDIENCES", AUDIENCE);
        std::env::set_var("EPR_KEYCLOAK_ROLE_MAP", map.to_string());
    }
    let token = |role: &str, sub: &str| {
        idp.token(|c| {
            c["realm_access"] = json!({ "roles": [role] });
            c["sub"] = json!(sub);
        })
    };
    let admin = token("clinical-admin", "0c4f1e2a-0000-4000-8000-0000000000a2");
    let long = token("proxy", "0c4f1e2a-0000-4000-8000-0000000000b1");
    let short = token("proxy", "0c4f1e2a-0000-4000-8000-0000000000b2");

    request::<App, _, _>(|server, _ctx| async move {
        let (ah, av) = bearer(&admin);
        let (lh, lv) = bearer(&long);
        let (sh, sv) = bearer(&short);
        let purpose = (
            HeaderName::from_static("x-purpose-of-use"),
            HeaderValue::from_static("patient-access"),
        );
        let family = format!("proxyscope{}", nanos());
        let made = server
            .post("/api/patients")
            .add_header(ah.clone(), av.clone())
            .json(&json!({ "patient": serde_json::to_value(Patient {
                names: vec![HumanName { family: Some(family.clone()), given: vec![family.clone()] }],
                birth_date: Some(dob()),
                ..Default::default()
            }).unwrap() }))
            .await;
        assert_eq!(made.status_code(), 201, "{}", made.text());
        let pid = made.json::<Value>()["id"].as_str().unwrap().to_string();
        let rel_url = format!("/api/patients/{pid}/relationships");
        let record = |sub: &str, t: Value| {
            json!({ "relationship": { "subject": sub, "kind": "proxy", "status": "active", "proxy": t },
                     "reason": "parental responsibility seen" })
        };

        // A proxy cannot be recorded without terms, with no scope, or with an expiry in the past.
        for body in [
            json!({ "relationship": { "subject": "x", "kind": "proxy", "status": "active" } }),
            record("x", terms(&[], unix() + 3600)),
            record("x", terms(&["identity"], unix() - 1)),
            record("x", terms(&["everything"], unix() + 3600)),
            json!({ "relationship": { "subject": "x", "kind": "care-team", "status": "active",
                     "proxy": terms(&["identity"], unix() + 3600) } }),
        ] {
            let r = server
                .post(&rel_url)
                .add_header(ah.clone(), av.clone())
                .json(&body)
                .await;
            assert!(
                matches!(r.status_code().as_u16(), 400 | 422),
                "{body}: {} {}",
                r.status_code(),
                r.text()
            );
        }

        // Identity only: the record reads, nothing else does.
        let sub_long = "0c4f1e2a-0000-4000-8000-0000000000b1";
        let created = server
            .post(&rel_url)
            .add_header(ah.clone(), av.clone())
            .json(&record(sub_long, terms(&["identity"], unix() + 3600)))
            .await;
        assert_eq!(created.status_code(), 201, "{}", created.text());
        let created: Value = created.json();
        let get = |path: String| {
            server
                .get(&path)
                .add_header(lh.clone(), lv.clone())
                .add_header(purpose.0.clone(), purpose.1.clone())
        };
        assert_eq!(get(format!("/api/patients/{pid}")).await.status_code(), 200);
        let mine = get("/api/me/patients".to_string()).await;
        assert_eq!(mine.status_code(), 200);
        assert!(mine.text().contains(&pid), "listed with identity scope");
        for path in [
            format!("/api/patients/{pid}/allergies"),
            format!("/api/patients/{pid}/photo"),
            format!("/api/me/patients/{pid}/access-log"),
        ] {
            assert_eq!(get(path.clone()).await.status_code(), 403, "{path}");
        }

        // Adding a scope is a new version; the others stay closed.
        let wider = server
            .put(&format!("{rel_url}/{}", created["id"].as_str().unwrap()))
            .add_header(ah.clone(), av.clone())
            .json(&json!({
                "relationship": { "subject": sub_long, "kind": "proxy", "status": "active",
                    "proxy": terms(&["identity", "allergies", "access-log"], unix() + 3600) },
                "expected_version": created["version"],
                "reason": "asked to see allergies",
            }))
            .await;
        assert_eq!(wider.status_code(), 200, "{}", wider.text());
        let wider: Value = wider.json();
        assert_eq!(
            get(format!("/api/patients/{pid}/allergies")).await.status_code(),
            200
        );
        assert_eq!(
            get(format!("/api/patients/{pid}/photo")).await.status_code(),
            403
        );
        assert_eq!(
            get(format!("/api/me/patients/{pid}/access-log"))
                .await
                .status_code(),
            200
        );

        // The log narrows by day, by who and by refusal (SE-6); a filter it cannot read is 400.
        let log = format!("/api/me/patients/{pid}/access-log");
        for bad in ["by=nobody", "from=yesterday", "to=2026-13-45", "refused=maybe", "purpose=curiosity"] {
            assert_eq!(get(format!("{log}?{bad}")).await.status_code(), 400, "{bad}");
        }
        let none = get(format!("{log}?from=2999-01-01")).await;
        assert_eq!(none.status_code(), 200);
        assert_eq!(none.json::<Value>()["entries"].as_array().unwrap().len(), 0);
        let some = get(format!("{log}?from=2000-01-01&to=2999-01-01&by=staff&refused=false&purpose=treatment")).await;
        assert_eq!(some.status_code(), 200, "{}", some.text());

        // The log as a file: CSV, the same filters, no staff identifier, and a format it does not know is 400.
        let file = get(format!("{log}?format=csv&from=2000-01-01&by=staff")).await;
        assert_eq!(file.status_code(), 200, "{}", file.text());
        assert!(file.header("content-type").to_str().unwrap().starts_with("text/csv"));
        assert!(file.header("content-disposition").to_str().unwrap().contains("access-log.csv"));
        assert_eq!(file.header("x-content-type-options"), "nosniff");
        let body = file.text();
        assert!(
            body.starts_with("id,at_utc,by,roles,organisations,did,what,refused,purpose,emergency_access\n"),
            "{body}"
        );
        assert!(!body.contains(sub_long), "no staff identifier in the file: {body}");
        assert_eq!(get(format!("{log}?format=xml")).await.status_code(), 400);

        // Ending the relationship is the revocation.
        let ended = server
            .put(&format!("{rel_url}/{}", created["id"].as_str().unwrap()))
            .add_header(ah.clone(), av.clone())
            .json(&json!({
                "relationship": { "subject": sub_long, "kind": "proxy", "status": "ended",
                    "proxy": terms(&["identity", "allergies"], unix() + 3600) },
                "expected_version": wider["version"],
                "reason": "no longer responsible",
            }))
            .await;
        assert_eq!(ended.status_code(), 200, "{}", ended.text());
        assert_eq!(get(format!("/api/patients/{pid}")).await.status_code(), 403);

        // An expiry ends it by itself.
        let sub_short = "0c4f1e2a-0000-4000-8000-0000000000b2";
        let r = server
            .post(&rel_url)
            .add_header(ah.clone(), av.clone())
            .json(&record(sub_short, terms(&["identity"], unix() + 2)))
            .await;
        assert_eq!(r.status_code(), 201, "{}", r.text());
        let read_short = || {
            server
                .get(&format!("/api/patients/{pid}"))
                .add_header(sh.clone(), sv.clone())
                .add_header(purpose.0.clone(), purpose.1.clone())
        };
        assert_eq!(read_short().await.status_code(), 200);
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        assert_eq!(read_short().await.status_code(), 403);
    })
    .await;
}
