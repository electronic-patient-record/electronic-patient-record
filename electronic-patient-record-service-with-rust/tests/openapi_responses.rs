//! DB-gated, through the real router with login off (OS-7): real responses from the routes the
//! OpenAPI document describes validate against the schemas that document serves.
//!
//! The validator is small and strict where it matters: an object must have its `required` keys and,
//! unless the schema allows more, no key outside `properties`; an `enum` must contain the value;
//! types are checked. It does not implement all of JSON Schema (no `allOf`, `oneOf`, formats or
//! length limits), and it only sees what these requests produce: optional fields left out of a
//! request are not exercised.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test openapi_responses -- --ignored --test-threads=1
//! ```

use axum::http::StatusCode;
use electronic_patient_record::app::App;
use loco_rs::testing::prelude::request;
use serde_json::{Value, json};

mod openapi_support;
use openapi_support::{check, response_schema};

fn letters(label: &str) -> String {
    let mut n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mut s = String::new();
    while n > 0 {
        s.push((b'a' + (n % 26) as u8) as char);
        n /= 26;
    }
    format!("{label}{s}")
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn real_responses_validate_against_the_served_schemas() {
    // SAFETY: set before the app boots; the suite runs single-threaded.
    unsafe { std::env::set_var("EPR_REQUIRE_AUTH", "false") };
    request::<App, _, _>(|server, _ctx| async move {
        let doc: Value = server.get("/api/openapi.json").await.json();
        let mut problems: Vec<String> = Vec::new();
        let mut checked = 0;
        let mut expect = |label: &str,
                          path: &str,
                          method: &str,
                          status: StatusCode,
                          body: &Value,
                          problems: &mut Vec<String>| {
            let schema = response_schema(&doc, path, method, status.as_str());
            let mut found = Vec::new();
            check(&doc, schema, body, "$", &mut found);
            checked += 1;
            problems.extend(
                found
                    .into_iter()
                    .map(|p| format!("{label} ({method} {path}): {p}")),
            );
        };

        // Patient: create, read, search, update.
        let family = letters("spec");
        let created = server
            .post("/api/patients")
            .json(&json!({ "patient": {
                "names": [{"family": family, "given": ["Sam"]}],
                "birth_date": "1931-02-03", "sex": "other",
                "identifiers": [{"system": "urn:test", "value": "1"}],
                "addresses": [{"postal_code": "AB1 2CD"}],
                "telecom": [{"system": "phone", "value": "0123"}]
            }, "override_reason": "spec check" }))
            .await;
        assert_eq!(
            created.status_code(),
            StatusCode::CREATED,
            "{}",
            created.text()
        );
        let stored: Value = created.json();
        expect(
            "create",
            "/api/patients",
            "post",
            StatusCode::CREATED,
            &stored,
            &mut problems,
        );
        let id = stored["id"].as_str().unwrap().to_string();

        let read = server.get(&format!("/api/patients/{id}")).await;
        assert_eq!(read.status_code(), StatusCode::OK);
        expect(
            "read",
            "/api/patients/{id}",
            "get",
            StatusCode::OK,
            &read.json(),
            &mut problems,
        );

        let found = server.get(&format!("/api/patients?family={family}")).await;
        assert_eq!(found.status_code(), StatusCode::OK, "{}", found.text());
        expect(
            "search",
            "/api/patients",
            "get",
            StatusCode::OK,
            &found.json(),
            &mut problems,
        );

        let updated = server
            .put(&format!("/api/patients/{id}"))
            .json(&json!({ "patient": stored["patient"], "expected_version": 1, "reason": "r" }))
            .await;
        assert_eq!(updated.status_code(), StatusCode::OK, "{}", updated.text());
        expect(
            "update",
            "/api/patients/{id}",
            "put",
            StatusCode::OK,
            &updated.json(),
            &mut problems,
        );

        let stale = server
            .put(&format!("/api/patients/{id}"))
            .json(&json!({ "patient": stored["patient"], "expected_version": 1 }))
            .await;
        assert_eq!(stale.status_code(), StatusCode::CONFLICT);
        expect(
            "stale update",
            "/api/patients/{id}",
            "put",
            StatusCode::CONFLICT,
            &stale.json(),
            &mut problems,
        );

        // Allergy: create, list, update.
        let allergy = json!({ "substance": "penicillin", "category": "medication",
            "criticality": "high", "reaction": "rash", "onset": "2020", "status": "active" });
        let made = server
            .post(&format!("/api/patients/{id}/allergies"))
            .json(&json!({ "allergy": allergy, "reason": "r" }))
            .await;
        assert_eq!(made.status_code(), StatusCode::CREATED, "{}", made.text());
        let made: Value = made.json();
        let p = "/api/patients/{id}/allergies";
        expect(
            "allergy create",
            p,
            "post",
            StatusCode::CREATED,
            &made,
            &mut problems,
        );
        let list = server.get(&format!("/api/patients/{id}/allergies")).await;
        assert_eq!(list.status_code(), StatusCode::OK);
        expect(
            "allergy list",
            p,
            "get",
            StatusCode::OK,
            &list.json(),
            &mut problems,
        );
        let changed = server
            .put(&format!(
                "/api/patients/{id}/allergies/{}",
                made["id"].as_str().unwrap()
            ))
            .json(&json!({ "allergy": allergy, "expected_version": 1 }))
            .await;
        assert_eq!(changed.status_code(), StatusCode::OK, "{}", changed.text());
        expect(
            "allergy update",
            "/api/patients/{id}/allergies/{allergy_id}",
            "put",
            StatusCode::OK,
            &changed.json(),
            &mut problems,
        );

        // Allergy statement: refused while an allergy is active, so make a patient with none.
        let blocked = server
            .put(&format!("/api/patients/{id}/allergy-statement"))
            .json(&json!({ "none_known": true, "expected_version": 0 }))
            .await;
        assert_eq!(blocked.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        let ended = server
            .put(&format!(
                "/api/patients/{id}/allergies/{}",
                made["id"].as_str().unwrap()
            ))
            .json(
                &json!({ "allergy": {"substance": "penicillin", "status": "inactive"},
                "expected_version": 2 }),
            )
            .await;
        assert_eq!(ended.status_code(), StatusCode::OK, "{}", ended.text());
        let stated = server
            .put(&format!("/api/patients/{id}/allergy-statement"))
            .json(&json!({ "none_known": true, "expected_version": 0, "reason": "r" }))
            .await;
        assert_eq!(stated.status_code(), StatusCode::OK, "{}", stated.text());
        expect(
            "allergy statement",
            "/api/patients/{id}/allergy-statement",
            "put",
            StatusCode::OK,
            &stated.json(),
            &mut problems,
        );
        let stale = server
            .put(&format!("/api/patients/{id}/allergy-statement"))
            .json(&json!({ "none_known": false, "expected_version": 0 }))
            .await;
        assert_eq!(
            stale.status_code(),
            StatusCode::CONFLICT,
            "{}",
            stale.text()
        );
        expect(
            "allergy statement stale",
            "/api/patients/{id}/allergy-statement",
            "put",
            StatusCode::CONFLICT,
            &stale.json(),
            &mut problems,
        );

        // Merge, links, unmerge: a second record of the same person, then a refusal.
        let twin = server
            .post("/api/patients")
            .json(&json!({ "patient": stored["patient"], "override_reason": "same person, for the check" }))
            .await;
        assert_eq!(twin.status_code(), StatusCode::CREATED, "{}", twin.text());
        let twin_id = twin.json::<Value>()["id"].as_str().unwrap().to_string();
        let merged = server
            .post(&format!("/api/patients/{id}/merge"))
            .json(&json!({ "source": twin_id, "reason": "same person" }))
            .await;
        assert_eq!(merged.status_code(), StatusCode::CREATED, "{}", merged.text());
        expect("merge", "/api/patients/{id}/merge", "post", StatusCode::CREATED, &merged.json(), &mut problems);
        let links = server.get(&format!("/api/patients/{id}/links")).await;
        assert_eq!(links.status_code(), StatusCode::OK, "{}", links.text());
        expect("links", "/api/patients/{id}/links", "get", StatusCode::OK, &links.json(), &mut problems);
        let refused = server
            .post(&format!("/api/patients/{id}/merge"))
            .json(&json!({ "source": id, "reason": "same" }))
            .await;
        assert_eq!(refused.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        expect("merge refused", "/api/patients/{id}/merge", "post", StatusCode::UNPROCESSABLE_ENTITY, &refused.json(), &mut problems);
        let unmerged = server
            .post(&format!("/api/patients/{id}/unmerge"))
            .json(&json!({ "source": twin_id, "reason": "mistake" }))
            .await;
        assert_eq!(unmerged.status_code(), StatusCode::OK, "{}", unmerged.text());
        expect("unmerge", "/api/patients/{id}/unmerge", "post", StatusCode::OK, &unmerged.json(), &mut problems);

        // Photo: put, read, stale put, missing version, withdraw, read after withdraw.
        let photo = format!("/api/patients/{id}/photo");
        let png: Vec<u8> = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 1, 2, 3];
        let put_png = |query: &str| {
            server
                .put(&format!("{photo}{query}"))
                .add_header(
                    axum::http::HeaderName::from_static("content-type"),
                    axum::http::HeaderValue::from_static("image/png"),
                )
                .bytes(png.clone().into())
        };
        let stored_photo = put_png("?expected_version=0").await;
        assert_eq!(
            stored_photo.status_code(),
            StatusCode::OK,
            "{}",
            stored_photo.text()
        );
        expect(
            "photo put",
            "/api/patients/{id}/photo",
            "put",
            StatusCode::OK,
            &stored_photo.json(),
            &mut problems,
        );
        let shown = server.get(&photo).await;
        assert_eq!(shown.status_code(), StatusCode::OK);
        assert_eq!(shown.header("content-type"), "image/png");
        assert_eq!(shown.header("cache-control"), "private, no-store");
        assert_eq!(shown.as_bytes().as_ref(), png.as_slice());
        let stale_photo = put_png("?expected_version=0").await;
        assert_eq!(
            stale_photo.status_code(),
            StatusCode::CONFLICT,
            "{}",
            stale_photo.text()
        );
        expect(
            "photo stale",
            "/api/patients/{id}/photo",
            "put",
            StatusCode::CONFLICT,
            &stale_photo.json(),
            &mut problems,
        );
        let no_version = put_png("").await;
        assert_eq!(
            no_version.status_code(),
            StatusCode::BAD_REQUEST,
            "{}",
            no_version.text()
        );
        expect(
            "photo no version",
            "/api/patients/{id}/photo",
            "put",
            StatusCode::BAD_REQUEST,
            &no_version.json(),
            &mut problems,
        );
        let withdrawn = server
            .post(&format!("{photo}/withdraw?expected_version=1&reason=r"))
            .await;
        assert_eq!(
            withdrawn.status_code(),
            StatusCode::OK,
            "{}",
            withdrawn.text()
        );
        expect(
            "photo withdraw",
            "/api/patients/{id}/photo/withdraw",
            "post",
            StatusCode::OK,
            &withdrawn.json(),
            &mut problems,
        );
        let gone = server.get(&photo).await;
        assert_eq!(gone.status_code(), StatusCode::NOT_FOUND, "{}", gone.text());
        expect(
            "photo gone",
            "/api/patients/{id}/photo",
            "get",
            StatusCode::NOT_FOUND,
            &gone.json(),
            &mut problems,
        );

        // Observation: create (with a value, and without one), list, versions, update.
        let value_obs = json!({ "code": "8867-4", "value": 72.0, "unit": "/min",
            "range": {"low": 60.0, "high": 100.0}, "critical": {"low": 40.0},
            "effective_at": "2026-10-26T09:30:00Z", "status": "final" });
        let absent_obs = json!({ "code": "8867-4", "absent_reason": "declined",
            "effective_at": "2026-10-26T09:31:00Z", "status": "final" });
        let p = "/api/patients/{id}/observations";
        let mut observation_id = String::new();
        for obs in [value_obs.clone(), absent_obs] {
            let r = server
                .post(&format!("/api/patients/{id}/observations"))
                .json(&json!({ "observation": obs }))
                .await;
            assert_eq!(r.status_code(), StatusCode::CREATED, "{}", r.text());
            let body: Value = r.json();
            observation_id = body["id"].as_str().unwrap().to_string();
            expect(
                "observation create",
                p,
                "post",
                StatusCode::CREATED,
                &body,
                &mut problems,
            );
        }
        let list = server
            .get(&format!("/api/patients/{id}/observations"))
            .await;
        assert_eq!(list.status_code(), StatusCode::OK);
        expect(
            "observation list",
            p,
            "get",
            StatusCode::OK,
            &list.json(),
            &mut problems,
        );
        let versions = server
            .get(&format!(
                "/api/patients/{id}/observations/{observation_id}/versions"
            ))
            .await;
        assert_eq!(versions.status_code(), StatusCode::OK);
        expect(
            "observation versions",
            "/api/patients/{id}/observations/{observation_id}/versions",
            "get",
            StatusCode::OK,
            &versions.json(),
            &mut problems,
        );

        // Consent and relationship: create, list.
        let consent = server
            .post(&format!("/api/patients/{id}/consents"))
            .json(&json!({ "consent": { "kind": "general", "status": "active",
                "provisions": [{ "effect": "deny", "purposes": ["research"] }],
                "period": { "start": "2026-01-01" } }, "reason": "r" }))
            .await;
        assert_eq!(
            consent.status_code(),
            StatusCode::CREATED,
            "{}",
            consent.text()
        );
        expect(
            "consent create",
            "/api/patients/{id}/consents",
            "post",
            StatusCode::CREATED,
            &consent.json(),
            &mut problems,
        );
        let list = server.get(&format!("/api/patients/{id}/consents")).await;
        assert_eq!(list.status_code(), StatusCode::OK);
        expect(
            "consent list",
            "/api/patients/{id}/consents",
            "get",
            StatusCode::OK,
            &list.json(),
            &mut problems,
        );

        let relationship = server
            .post(&format!("/api/patients/{id}/relationships"))
            .json(
                &json!({ "relationship": { "subject": "0c4f1e2a-0000-4000-8000-0000000000c1",
                "kind": "care-team", "status": "active" } }),
            )
            .await;
        assert_eq!(
            relationship.status_code(),
            StatusCode::CREATED,
            "{}",
            relationship.text()
        );
        expect(
            "relationship create",
            "/api/patients/{id}/relationships",
            "post",
            StatusCode::CREATED,
            &relationship.json(),
            &mut problems,
        );
        let list = server
            .get(&format!("/api/patients/{id}/relationships"))
            .await;
        assert_eq!(list.status_code(), StatusCode::OK);
        expect(
            "relationship list",
            "/api/patients/{id}/relationships",
            "get",
            StatusCode::OK,
            &list.json(),
            &mut problems,
        );

        // Updates and stale updates: every PUT the document describes, on fresh records.
        let puts: [(&str, &str, &str, Value); 3] = [
            (
                "observation",
                "observations",
                "/api/patients/{id}/observations/{observation_id}",
                json!({ "observation": { "code": "8867-4", "value": 70.0, "unit": "/min",
                    "effective_at": "2026-10-26T10:00:00Z", "status": "final" } }),
            ),
            (
                "consent",
                "consents",
                "/api/patients/{id}/consents/{consent_id}",
                json!({ "consent": { "kind": "general", "status": "active",
                    "provisions": [{ "effect": "permit" }] } }),
            ),
            (
                "relationship",
                "relationships",
                "/api/patients/{id}/relationships/{relationship_id}",
                json!({ "relationship": { "subject": "0c4f1e2a-0000-4000-8000-0000000000c2",
                    "kind": "care-team", "status": "active" } }),
            ),
        ];
        for (name, plural, template, body) in puts {
            let made = server
                .post(&format!("/api/patients/{id}/{plural}"))
                .json(&body)
                .await;
            assert_eq!(
                made.status_code(),
                StatusCode::CREATED,
                "{name}: {}",
                made.text()
            );
            let made: Value = made.json();
            let url = format!(
                "/api/patients/{id}/{plural}/{}",
                made["id"].as_str().unwrap()
            );
            let mut change = body.clone();
            change["expected_version"] = json!(1);
            change["reason"] = json!("check");
            let ok = server.put(&url).json(&change).await;
            assert_eq!(ok.status_code(), StatusCode::OK, "{name}: {}", ok.text());
            expect(
                &format!("{name} update"),
                template,
                "put",
                StatusCode::OK,
                &ok.json(),
                &mut problems,
            );
            let stale = server.put(&url).json(&change).await;
            assert_eq!(
                stale.status_code(),
                StatusCode::CONFLICT,
                "{name}: {}",
                stale.text()
            );
            expect(
                &format!("{name} stale update"),
                template,
                "put",
                StatusCode::CONFLICT,
                &stale.json(),
                &mut problems,
            );
        }
        let stale = server
            .put(&format!(
                "/api/patients/{id}/allergies/{}",
                made["id"].as_str().unwrap()
            ))
            .json(&json!({ "allergy": allergy, "expected_version": 1 }))
            .await;
        assert_eq!(
            stale.status_code(),
            StatusCode::CONFLICT,
            "{}",
            stale.text()
        );
        expect(
            "allergy stale update",
            "/api/patients/{id}/allergies/{allergy_id}",
            "put",
            StatusCode::CONFLICT,
            &stale.json(),
            &mut problems,
        );

        // Amendments: ask, list, a step, and a stale step.
        let asked = server
            .post(&format!("/api/patients/{id}/amendments"))
            .json(&json!({ "kind": "Observation", "target_id": observation_id,
                "version": 1, "text": "the reading was taken at rest" }))
            .await;
        assert_eq!(asked.status_code(), StatusCode::CREATED, "{}", asked.text());
        let asked: Value = asked.json();
        let p = "/api/patients/{id}/amendments";
        expect(
            "amendment create",
            p,
            "post",
            StatusCode::CREATED,
            &asked,
            &mut problems,
        );
        let list = server.get(&format!("/api/patients/{id}/amendments")).await;
        assert_eq!(list.status_code(), StatusCode::OK, "{}", list.text());
        expect(
            "amendment list",
            p,
            "get",
            StatusCode::OK,
            &list.json(),
            &mut problems,
        );
        let url = format!(
            "/api/patients/{id}/amendments/{}",
            asked["id"].as_str().unwrap()
        );
        let step = server
            .put(&url)
            .json(&json!({ "expected_version": 1, "step": "withdraw" }))
            .await;
        assert_eq!(step.status_code(), StatusCode::OK, "{}", step.text());
        let put = "/api/patients/{id}/amendments/{amendment_id}";
        expect(
            "amendment step",
            put,
            "put",
            StatusCode::OK,
            &step.json(),
            &mut problems,
        );
        let stale = server
            .put(&url)
            .json(&json!({ "expected_version": 1, "step": "withdraw" }))
            .await;
        assert_eq!(
            stale.status_code(),
            StatusCode::CONFLICT,
            "{}",
            stale.text()
        );
        expect(
            "amendment stale step",
            put,
            "put",
            StatusCode::CONFLICT,
            &stale.json(),
            &mut problems,
        );

        assert!(checked >= 38, "only {checked} responses were checked");
        assert!(
            problems.is_empty(),
            "responses that do not match their schema:\n{}",
            problems.join("\n")
        );
    })
    .await;
}

#[test]
fn the_validator_finds_what_it_is_meant_to_find() {
    let doc = json!({ "components": { "schemas": {
        "T": { "type": "object", "required": ["a"],
               "properties": { "a": { "enum": ["x", "y"] }, "n": { "type": ["integer", "null"] } } }
    } } });
    let schema = json!({ "$ref": "#/components/schemas/T" });
    let run = |v: Value| {
        let mut out = Vec::new();
        check(&doc, &schema, &v, "$", &mut out);
        out
    };
    assert!(run(json!({ "a": "x", "n": null })).is_empty());
    assert!(
        run(json!({ "n": 1 }))
            .iter()
            .any(|p| p.contains("lacks required a"))
    );
    assert!(
        run(json!({ "a": "z" }))
            .iter()
            .any(|p| p.contains("not one of"))
    );
    assert!(
        run(json!({ "a": "x", "extra": 1 }))
            .iter()
            .any(|p| p.contains("not in the schema"))
    );
    assert!(
        run(json!({ "a": "x", "n": "s" }))
            .iter()
            .any(|p| p.contains("expected one of"))
    );
}
