//! DB-gated, through the real router with login **on** (RS-1, RS-6): a suspicion on a referral under a
//! pathway. Raised once (a second raise is 409); amended only with a reason and a change, from the current
//! version; each version shown with what it changed and the pathway's priority and due days; a band that
//! follows a result needs a result on this patient's record; a referral from another patient is a miss;
//! reception and the patient-access purpose are refused; and nothing of the suspicion reaches the audit.
//! One test, because the token guard is configured once per process.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test http_suspicions -- --ignored --test-threads=1
//! ```

use authentication_verifier::test_idp::{AUDIENCE, REALM, TestIdp};
use axa_test::prelude::request;
use axum::http::{HeaderName, HeaderValue, StatusCode};
use electronic_patient_record::app::App;
use electronic_patient_record::pool::Pool;
use electronic_patient_record_matcher::{HumanName, Patient};
use serde_json::{Value, json};
use sqlx::{AssertSqlSafe, Row};

fn nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

fn dob(seed: u128) -> String {
    let n = nanos() / 1000 + seed;
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

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_suspicion_keeps_its_point_of_suspicion_and_follows_its_pathway() {
    let idp = TestIdp::start().await;
    let roles = ["clinical-admin", "clinician", "receptionist"];
    let map: Value = roles
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
    const DOCTOR: &str = "0c4f1e2a-0000-4000-8000-0000000000d1";
    const RECIPIENT: &str = "0c4f1e2a-0000-4000-8000-0000000000d3";
    let admin = token("clinical-admin", "0c4f1e2a-0000-4000-8000-0000000000d0");
    let doctor = token("clinician", DOCTOR);
    let receptionist = token("receptionist", "0c4f1e2a-0000-4000-8000-0000000000d2");
    let recipient = token("clinician", RECIPIENT);

    request::<App, _, _>(|server, ctx| async move {
        let (ah, av) = bearer(&admin);
        let (dh, dv) = bearer(&doctor);
        let (rh, rv) = bearer(&receptionist);
        let (th, tv) = bearer(&recipient);
        let patient = |seed: u128| {
            let server = &server;
            let (ah, av) = (ah.clone(), av.clone());
            async move {
                let family = format!("suspect{}", nanos() + seed);
                let made = server
                    .post("/api/patients")
                    .add_header(ah.clone(), av.clone())
                    .json(&json!({ "patient": serde_json::to_value(Patient {
                        names: vec![HumanName { family: Some(family.clone()), given: vec![family] }],
                        birth_date: Some(dob(seed)),
                        ..Default::default()
                    }).unwrap() }))
                    .await;
                assert_eq!(made.status_code(), StatusCode::CREATED, "{}", made.text());
                let pid = made.json::<Value>()["id"].as_str().unwrap().to_string();
                let rel = server
                    .post(&format!("/api/patients/{pid}/relationships"))
                    .add_header(ah, av)
                    .json(&json!({ "relationship": { "subject": DOCTOR, "kind": "care-team", "status": "active" },
                                   "reason": "test" }))
                    .await;
                assert_eq!(rel.status_code(), StatusCode::CREATED, "{}", rel.text());
                pid
            }
        };
        let (pid, other) = (patient(0).await, patient(7_777).await);
        let referral = |pid: String| {
            let server = &server;
            let (dh, dv) = (dh.clone(), dv.clone());
            async move {
                let r = server
                    .post(&format!("/api/patients/{pid}/referrals"))
                    .add_header(dh, dv)
                    .json(&json!({ "referral": { "recipient": "Colorectal clinic", "reason": "bleeding",
                                   "question": "please assess", "priority": "urgent" } }))
                    .await;
                assert_eq!(r.status_code(), StatusCode::CREATED, "{}", r.text());
                r.json::<Value>()["id"].as_str().unwrap().to_string()
            }
        };
        let rid = referral(pid.clone()).await;
        let url = format!("/api/patients/{pid}/referrals/{rid}/suspicion");
        let cancer = json!({
            "condition": "Suspected lower gastrointestinal cancer",
            "point_of_suspicion": "2026-11-09T09:30:00Z",
            "source": "clinician", "by": "Dr Gwen Hughes",
            "grounds": { "kind": "criterion", "detail": "NG12 unexplained rectal bleeding" },
            "patient_told": "not-told",
            "pathway": "suspected-cancer-wales", "pathway_version": 1
        });

        // Nothing yet: an empty history, not a miss.
        let none = server.get(&url).add_header(dh.clone(), dv.clone()).await;
        assert_eq!(none.status_code(), StatusCode::OK, "{}", none.text());
        assert_eq!(none.json::<Value>()["current"], Value::Null);
        let pathways = none.json::<Value>()["pathways"].clone();
        let wales = pathways.as_array().unwrap().iter()
            .find(|p| p["id"] == "suspected-cancer-wales").unwrap().clone();
        assert_eq!(wales["version"], 2, "only the newest version is offered");
        assert_eq!(wales["subtypes"]["required"], true);

        let raised = server
            .post(&url)
            .add_header(dh.clone(), dv.clone())
            .json(&json!({ "suspicion": cancer }))
            .await;
        assert_eq!(raised.status_code(), StatusCode::CREATED, "{}", raised.text());
        let raised: Value = raised.json();
        assert_eq!(raised["plan"]["priority"], "urgent");
        assert_eq!(raised["plan"]["approved"], false, "a draft pathway says so");
        assert_eq!(raised["plan"]["targets"][0]["due_on"], "2026-12-07");
        assert_eq!(raised["plan"]["targets"][1]["due_on"], "2027-01-10");

        // Once only; then amend from version 1, with a reason and a change.
        let again = server
            .post(&url)
            .add_header(dh.clone(), dv.clone())
            .json(&json!({ "suspicion": cancer }))
            .await;
        assert_eq!(again.status_code(), StatusCode::CONFLICT, "{}", again.text());
        let mut earlier = cancer.clone();
        earlier["point_of_suspicion"] = json!("2026-11-02T10:00:00Z");
        for (body, status) in [
            (json!({ "suspicion": earlier, "expected_version": 1 }), StatusCode::UNPROCESSABLE_ENTITY),
            (json!({ "suspicion": cancer, "expected_version": 1, "reason": "checked" }), StatusCode::UNPROCESSABLE_ENTITY),
            (json!({ "suspicion": earlier, "expected_version": 2, "reason": "seen a week before" }), StatusCode::CONFLICT),
            (json!({ "suspicion": earlier, "expected_version": 1, "reason": "seen a week before" }), StatusCode::OK),
        ] {
            let r = server.put(&url).add_header(dh.clone(), dv.clone()).json(&body).await;
            assert_eq!(r.status_code(), status, "{body}: {}", r.text());
        }
        let history: Value = server.get(&url).add_header(dh.clone(), dv.clone()).await.json();
        assert_eq!(history["versions"].as_array().unwrap().len(), 2);
        let current = &history["current"];
        assert_eq!(current["version"], 2);
        assert_eq!(current["reason"], "seen a week before");
        assert_eq!(current["changed"], json!(["point_of_suspicion"]));
        assert_eq!(current["plan"]["targets"][0]["due_on"], "2026-11-30", "the clock moved with it");
        assert_eq!(history["versions"][0]["plan"]["targets"][0]["due_on"], "2026-12-07");

        // Version 2 of the Welsh pathway needs a primary cancer site from its own list, kept with its scheme.
        let rid_site = referral(pid.clone()).await;
        let url_site = format!("/api/patients/{pid}/referrals/{rid_site}/suspicion");
        let mut v2 = cancer.clone();
        v2["pathway_version"] = json!(2);
        let mut other_list = v2.clone();
        other_list["subtype"] = json!({ "scheme": "generic-dementia-type", "code": "03" });
        for body in [v2.clone(), other_list] {
            let r = server.post(&url_site).add_header(dh.clone(), dv.clone())
                .json(&json!({ "suspicion": body })).await;
            assert_eq!(r.status_code(), StatusCode::UNPROCESSABLE_ENTITY, "{body}: {}", r.text());
        }
        v2["subtype"] = json!({ "scheme": "wales-primary-cancer-site", "code": "03" });
        let r = server.post(&url_site).add_header(dh.clone(), dv.clone())
            .json(&json!({ "suspicion": v2 })).await;
        assert_eq!(r.status_code(), StatusCode::CREATED, "{}", r.text());
        let r: Value = r.json();
        assert_eq!(r["suspicion"]["subtype"]["code"], "03");
        assert_eq!(r["plan"]["subtype"]["name"], "Lower GI", "named from the pathway's list");
        assert_eq!(r["plan"]["targets"][1]["due_on"], "2027-01-10", "the site moves no date");

        // Heart failure: a band is needed, and the high band follows a result on this record.
        let rid2 = referral(pid.clone()).await;
        let url2 = format!("/api/patients/{pid}/referrals/{rid2}/suspicion");
        let filed = server
            .post(&format!("/api/patients/{pid}/results"))
            .add_header(dh.clone(), dv.clone())
            .json(&json!({ "result": { "test": "NT-proBNP", "value": "2600", "unit": "ng/L",
                           "status": "final", "issued": "2026-11-09T08:00:00Z" } }))
            .await;
        assert_eq!(filed.status_code(), StatusCode::CREATED, "{}", filed.text());
        let result_id = filed.json::<Value>()["id"].as_str().unwrap().to_string();
        let heart = |band: Option<&str>, grounds: Value| {
            let mut s = json!({
                "condition": "Suspected heart failure", "point_of_suspicion": "2026-11-09T09:00:00Z",
                "source": "result", "by": "NT-proBNP", "grounds": grounds,
                "patient_told": "told", "pathway": "heart-failure", "pathway_version": 1
            });
            if let Some(b) = band {
                s["band"] = json!(b);
            }
            json!({ "suspicion": s })
        };
        let judgement = json!({ "kind": "judgement", "detail": "breathless" });
        let elsewhere = json!({ "kind": "result", "detail": "2600 ng/L",
                                "result_id": "0c4f1e2a-0000-4000-8000-00000000dead" });
        let measured = json!({ "kind": "result", "detail": "2600 ng/L", "result_id": result_id });
        for body in [
            heart(None, measured.clone()),
            heart(Some("nt-probnp-above-2000"), judgement),
            heart(Some("nt-probnp-above-2000"), elsewhere),
        ] {
            let r = server.post(&url2).add_header(dh.clone(), dv.clone()).json(&body).await;
            assert_eq!(r.status_code(), StatusCode::UNPROCESSABLE_ENTITY, "{body}: {}", r.text());
        }
        let r = server
            .post(&url2)
            .add_header(dh.clone(), dv.clone())
            .json(&heart(Some("nt-probnp-above-2000"), measured))
            .await;
        assert_eq!(r.status_code(), StatusCode::CREATED, "{}", r.text());
        let r: Value = r.json();
        assert_eq!(r["plan"]["targets"][0]["days"], 14);
        assert_eq!(r["plan"]["targets"][0]["due_on"], "2026-11-23");

        // RS-2. Triage and rebooking change the referral, never the suspicion.
        let rel = server
            .post(&format!("/api/patients/{pid}/relationships"))
            .add_header(ah.clone(), av.clone())
            .json(&json!({ "relationship": { "subject": RECIPIENT, "kind": "care-team", "status": "active" },
                           "reason": "test" }))
            .await;
        assert_eq!(rel.status_code(), StatusCode::CREATED, "{}", rel.text());
        let step = |rid: String, event: &'static str, version: i32, note: Option<&'static str>| {
            let server = &server;
            let pid = pid.clone();
            let (th, tv) = (th.clone(), tv.clone());
            async move {
                let r = server
                    .post(&format!("/api/patients/{pid}/referrals/{rid}/events"))
                    .add_header(th, tv)
                    .json(&json!({ "event": event, "expected_version": version, "note": note }))
                    .await;
                assert_eq!(r.status_code(), StatusCode::OK, "{event}: {}", r.text());
            }
        };
        step(rid2.clone(), "accept", 1, None).await;
        step(rid2.clone(), "book", 2, None).await;
        step(rid2.clone(), "record-no-show", 3, None).await;
        step(rid2.clone(), "book", 4, None).await;
        let kept: Value = server.get(&url2).add_header(dh.clone(), dv.clone()).await.json();
        assert_eq!(kept["current"]["version"], 1, "untouched by triage and rebooking: {kept}");
        assert_eq!(kept["current"]["suspicion"]["point_of_suspicion"], "2026-11-09T09:00:00Z");

        // A redirect ends the referral; its replacement carries the suspicion, point of suspicion and all.
        step(rid.clone(), "redirect", 1, Some("colorectal team at the other site")).await;
        let replace = |old: &str| {
            let server = &server;
            let pid = pid.clone();
            let (dh, dv) = (dh.clone(), dv.clone());
            let body = json!({ "referral": { "recipient": "Colorectal clinic, other site", "reason": "bleeding",
                               "question": "please assess", "priority": "urgent", "replaces": old } });
            async move {
                server
                    .post(&format!("/api/patients/{pid}/referrals"))
                    .add_header(dh, dv)
                    .json(&body)
                    .await
            }
        };
        let new = replace(&rid).await;
        assert_eq!(new.status_code(), StatusCode::CREATED, "{}", new.text());
        let new_id = new.json::<Value>()["id"].as_str().unwrap().to_string();
        let carried: Value = server
            .get(&format!("/api/patients/{pid}/referrals/{new_id}/suspicion"))
            .add_header(dh.clone(), dv.clone())
            .await
            .json();
        let c = &carried["current"];
        assert_eq!(c["version"], 1);
        assert_eq!(c["carried_from"], json!(rid));
        assert_eq!(c["suspicion"]["point_of_suspicion"], "2026-11-02T10:00:00Z");
        assert_eq!(c["plan"]["targets"][0]["due_on"], "2026-11-30", "the clock did not restart");
        for old in [rid.as_str(), rid2.as_str()] {
            let again = replace(old).await;
            assert_eq!(again.status_code(), StatusCode::UNPROCESSABLE_ENTITY,
                "replaced twice, or not redirected: {}", again.text());
        }

        // RS-3. A missed appointment never ends it, and nor does a decline: the closed referral goes on
        // the referrer's list until the suspicion has an outcome. A redirected one whose suspicion
        // was carried is not listed.
        step(rid2.clone(), "record-no-show", 5, None).await;
        let still: Value = server.get(&url2).add_header(dh.clone(), dv.clone()).await.json();
        assert_eq!(still["open"], true, "a no-show is not an outcome");
        step(rid2.clone(), "decline", 6, Some("not suitable for this clinic")).await;
        let listed = |rid: String| {
            let server = &server;
            let (dh, dv) = (dh.clone(), dv.clone());
            async move {
                let w: Value = server
                    .get("/api/referrals/worklist")
                    .add_header(dh, dv)
                    .await
                    .json();
                w["items"]
                    .as_array()
                    .unwrap_or_else(|| panic!("worklist: {w}"))
                    .iter()
                    .filter(|i| i["referral_id"] == json!(rid))
                    .map(|i| i["why"].as_str().unwrap().to_string())
                    .collect::<Vec<_>>()
            }
        };
        assert_eq!(listed(rid2.clone()).await, ["reply"], "the decline is shown first");
        let ack = server
            .post(&format!("/api/referrals/{rid2}/acknowledge"))
            .add_header(dh.clone(), dv.clone())
            .json(&json!({ "version": 7 }))
            .await;
        assert!(ack.status_code().is_success(), "{}", ack.text());
        assert_eq!(
            listed(rid2.clone()).await,
            ["open-suspicion"],
            "acknowledging the decline does not end the suspicion"
        );
        assert!(
            !listed(rid.clone()).await.contains(&"open-suspicion".to_string()),
            "carried to its replacement"
        );

        // Ended only by an outcome, once, with what it needs; then it is closed to amendment.
        let end_url = format!("{url2}/outcome");
        for (body, status) in [
            (json!({ "outcome": { "kind": "ruled-out", "detail": "normal echocardiogram" },
                     "expected_version": 1 }), StatusCode::UNPROCESSABLE_ENTITY),
            (json!({ "outcome": { "kind": "ruled-out", "by": "Dr Owen", "detail": "normal echocardiogram" },
                     "expected_version": 2 }), StatusCode::CONFLICT),
            (json!({ "outcome": { "kind": "ruled-out", "by": "Dr Owen", "detail": "normal echocardiogram" },
                     "expected_version": 1 }), StatusCode::CREATED),
            (json!({ "outcome": { "kind": "withdrawn", "detail": "again" },
                     "expected_version": 1 }), StatusCode::UNPROCESSABLE_ENTITY),
        ] {
            let r = server.post(&end_url).add_header(dh.clone(), dv.clone()).json(&body).await;
            assert_eq!(r.status_code(), status, "{body}: {}", r.text());
        }
        let ended: Value = server.get(&url2).add_header(dh.clone(), dv.clone()).await.json();
        assert_eq!(ended["open"], false);
        assert_eq!(ended["outcome"]["outcome"]["kind"], "ruled-out");
        assert_eq!(ended["outcome"]["suspicion_version"], 1);
        let mut late = ended["current"]["suspicion"].clone();
        late["instruction"] = json!("seen");
        let r = server
            .put(&url2)
            .add_header(dh.clone(), dv.clone())
            .json(&json!({ "suspicion": late, "expected_version": 1, "reason": "after the end" }))
            .await;
        assert_eq!(r.status_code(), StatusCode::UNPROCESSABLE_ENTITY, "{}", r.text());
        assert!(listed(rid2.clone()).await.is_empty(), "off the list once it has an outcome");

        // RS-7. An outcome recorded in error is marked so, with a reason, naming its seq: the suspicion
        // is open again, back on the list, may be amended and ended anew; the first outcome is kept.
        assert_eq!(ended["outcome"]["seq"], 1);
        let error_url = format!("{end_url}/entered-in-error");
        for (body, status) in [
            (json!({ "seq": 1, "reason": " " }), StatusCode::UNPROCESSABLE_ENTITY),
            (json!({ "seq": 2, "reason": "wrong patient's echo" }), StatusCode::CONFLICT),
            (json!({ "seq": 1, "reason": "wrong patient's echo" }), StatusCode::OK),
            (json!({ "seq": 1, "reason": "again" }), StatusCode::CONFLICT),
        ] {
            let r = server.post(&error_url).add_header(dh.clone(), dv.clone()).json(&body).await;
            assert_eq!(r.status_code(), status, "{body}: {}", r.text());
        }
        let reopened: Value = server.get(&url2).add_header(dh.clone(), dv.clone()).await.json();
        assert_eq!(reopened["open"], true);
        assert_eq!(reopened["outcome"], Value::Null);
        assert_eq!(reopened["outcomes_in_error"][0]["outcome"]["outcome"]["kind"], "ruled-out");
        assert_eq!(reopened["outcomes_in_error"][0]["reason"], "wrong patient's echo");
        assert_eq!(listed(rid2.clone()).await, ["open-suspicion"], "back on the list");
        let r = server
            .put(&url2)
            .add_header(dh.clone(), dv.clone())
            .json(&json!({ "suspicion": late, "expected_version": 1, "reason": "seen in clinic" }))
            .await;
        assert_eq!(r.status_code(), StatusCode::OK, "amendable again: {}", r.text());
        let r = server
            .post(&end_url)
            .add_header(dh.clone(), dv.clone())
            .json(&json!({ "outcome": { "kind": "confirmed", "diagnosis": "Heart failure with reduced ejection fraction",
                                        "detail": "echocardiogram" }, "expected_version": 2 }))
            .await;
        assert_eq!(r.status_code(), StatusCode::CREATED, "{}", r.text());
        let r: Value = r.json();
        assert_eq!((r["outcome"]["seq"].clone(), r["outcome"]["suspicion_version"].clone()), (json!(2), json!(2)));
        assert_eq!(r["outcomes_in_error"].as_array().unwrap().len(), 1, "the first is kept");
        assert!(listed(rid2.clone()).await.is_empty());

        // Another patient's path to this referral is a miss; reception and patient access are refused.
        let crossed = server
            .get(&format!("/api/patients/{other}/referrals/{rid}/suspicion"))
            .add_header(dh.clone(), dv.clone())
            .await;
        assert_eq!(crossed.status_code(), StatusCode::NOT_FOUND, "{}", crossed.text());
        assert_eq!(
            server.get(&url).add_header(rh.clone(), rv.clone()).await.status_code(),
            StatusCode::FORBIDDEN
        );
        let portal = server
            .get(&url)
            .add_header(dh.clone(), dv.clone())
            .add_header(
                HeaderName::from_static("x-purpose-of-use"),
                HeaderValue::from_static("patient-access"),
            )
            .await;
        assert_eq!(portal.status_code(), StatusCode::FORBIDDEN, "{}", portal.text());

        // The trail names referrals, versions and changed fields, never what is suspected.
        let rows = sqlx::query(AssertSqlSafe(format!(
            "SELECT detail::text AS d FROM audit_events WHERE resource_ref = 'Condition' AND patient_ref = 'Patient/{pid}'"
        )))
        .fetch_all(&ctx.pool())
        .await
        .unwrap();
        assert!(rows.len() >= 5, "raises, an amendment and reads: {}", rows.len());
        for row in rows {
            let d: String = row.try_get("d").unwrap();
            for secret in ["gastrointestinal", "Gwen", "rectal", "week before", "2600", "heart", "cancer", "echocardiogram", "Owen", "not suitable"] {
                assert!(!d.contains(secret), "{secret} leaked into the audit row: {d}");
            }
        }
    })
    .await;
}
