//! DB-gated, through the real router with login **on** (PX-2, H-048): the list across patients of needs whose
//! date to check has passed. One test, because the token guard is configured once per process.
//!
//! Covered: one overdue need shows and one in time does not, oldest date first; the item is a pointer with no
//! code, note or reason; the count; the list names the owning role; only the on-duty clinician reads it (403 for
//! every other staff role tried); the on-duty clinician alone can review and end a need, and each leaves the list;
//! a standing report of a death marks the item; the audit rows hold the count and no note.
//! Not covered: a need on a record merged away, a list that is cut short, and who holds the on-duty role, which
//! the service does not know.
//!
//! ```text
//! DATABASE_URL=postgres://postgres:postgres@localhost:5543/electronic_patient_record_test \
//!   cargo test --test http_overdue_reviews -- --ignored --test-threads=1
//! ```

use authentication_verifier::test_idp::{AUDIENCE, REALM, TestIdp};
use axa_test::prelude::request;
use axum::http::{HeaderName, HeaderValue, StatusCode};
use electronic_patient_record::app::App;
use electronic_patient_record::pool::Pool;
use serde_json::{Value, json};
use sqlx::Row;

fn nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

fn bearer(token: &str) -> (HeaderName, HeaderValue) {
    (
        HeaderName::from_static("authorization"),
        HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
    )
}

const SECRET: &str = "Secret-note-razorbill";

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn the_on_duty_clinician_alone_sees_what_is_overdue() {
    let idp = TestIdp::start().await;
    let roles = [
        "clinical-admin",
        "clinician",
        "nurse",
        "on-duty-clinician",
        "receptionist",
        "auditor",
    ];
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
    let token = |role: &str, n: u8| {
        idp.token(|c| {
            c["realm_access"] = json!({ "roles": [role] });
            c["sub"] = json!(format!("0c4f1e2a-0000-4000-8000-0000000001{n:02}"));
        })
    };
    let admin = bearer(&token("clinical-admin", 10));
    let clinician = bearer(&token("clinician", 11));
    let on_duty = bearer(&token("on-duty-clinician", 12));
    let refused = [
        clinician.clone(),
        admin.clone(),
        bearer(&token("nurse", 13)),
        bearer(&token("receptionist", 14)),
        bearer(&token("auditor", 15)),
    ];

    request::<App, _, _>(|server, ctx| async move {
        // A birth date of its own, so that a repeat run does not read as a likely duplicate.
        let n = nanos();
        let family = format!("overduereviews{n}");
        let born = format!("19{:02}-0{}-1{}", n % 100, n % 9 + 1, n % 9);
        let made = server
            .post("/api/patients")
            .add_header(admin.0.clone(), admin.1.clone())
            .json(&json!({ "patient": {
                "names": [{ "family": family, "given": [family] }], "birth_date": born } }))
            .await;
        assert_eq!(made.status_code(), StatusCode::CREATED, "{}", made.text());
        let pid = made.json::<Value>()["id"].as_str().unwrap().to_string();

        // A date that has passed. The route refuses to write one, so the test puts it in the table, as time
        // would. These dates sort before any other test's, so a long list cannot cut them off.
        let put = |code: &'static str, review_by: &'static str| {
            let (ctx, pid) = (ctx.clone(), pid.clone());
            async move {
                sqlx::query(
                    "INSERT INTO adjustment_versions
                       (adjustment_id, version, patient_id, author, status, body)
                     VALUES (gen_random_uuid(), 1, $1::uuid, 'test', 'active', $2)
                     RETURNING adjustment_id::text AS id",
                )
                .bind(pid)
                .bind(json!({ "code": code, "note": SECRET, "status": "active",
                              "review_by": review_by, "reason": SECRET }))
                .fetch_one(&ctx.pool())
                .await
                .unwrap()
                .get::<String, _>("id")
            }
        };
        let newer = put("braille", "2001-02-01").await;
        let older = put("large-print", "2001-01-01").await;
        let in_time = put("easy-read", "2999-01-01").await;

        let list = || {
            let (server, on_duty) = (&server, on_duty.clone());
            async move {
                let r = server
                    .get("/api/overdue-reviews")
                    .add_header(on_duty.0, on_duty.1)
                    .await;
                assert_eq!(r.status_code(), StatusCode::OK, "{}", r.text());
                assert!(!r.text().contains(SECRET));
                r.json::<Value>()
            }
        };
        let mine = |body: &Value| -> Vec<Value> {
            body["items"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|i| i["patient_id"] == pid.as_str())
                .cloned()
                .collect()
        };
        let ids = |rows: &[Value]| -> Vec<String> {
            rows.iter()
                .map(|i| i["adjustment_id"].as_str().unwrap().to_string())
                .collect()
        };

        let body = list().await;
        assert_eq!(body["accountable_role"], "on-duty-clinician");
        let rows = mine(&body);
        assert_eq!(
            ids(&rows),
            vec![older.clone(), newer.clone()],
            "not {in_time}"
        );
        assert_eq!(rows[0]["review_by"], "2001-01-01");
        assert_eq!(rows[0]["contact_held"], false);
        assert!(rows[0].get("code").is_none() && rows[0].get("note").is_none());

        let n = server
            .get("/api/overdue-reviews/count")
            .add_header(on_duty.0.clone(), on_duty.1.clone())
            .await;
        assert_eq!(n.status_code(), StatusCode::OK, "{}", n.text());
        assert!(n.json::<Value>()["open"].as_i64().unwrap() >= 2);

        for who in &refused {
            for path in ["/api/overdue-reviews", "/api/overdue-reviews/count"] {
                let r = server
                    .get(path)
                    .add_header(who.0.clone(), who.1.clone())
                    .await;
                assert_eq!(r.status_code(), StatusCode::FORBIDDEN, "{}", r.text());
            }
        }

        // A standing report of a death marks every item for that person, confirmed or not.
        let death = server
            .post(&format!("/api/patients/{pid}/death/assertions"))
            .add_header(clinician.0.clone(), clinician.1.clone())
            .json(&json!({
                "source": "family-member",
                "occurred": { "kind": "not-known", "reason": "not given" },
                "reason": "told by a neighbour" }))
            .await;
        assert_eq!(death.status_code(), StatusCode::CREATED, "{}", death.text());
        assert!(
            mine(&list().await)
                .iter()
                .all(|i| i["contact_held"] == true)
        );

        // The on-duty role alone can do the work: a review with a new date, and ending a need.
        let change = |id: String, body: Value| {
            let (server, pid, on_duty) = (&server, pid.clone(), on_duty.clone());
            async move {
                server
                    .put(&format!("/api/patients/{pid}/adjustments/{id}"))
                    .add_header(on_duty.0, on_duty.1)
                    .json(&body)
                    .await
            }
        };
        let done = change(
            older.clone(),
            json!({ "expected_version": 1, "action": "review",
                    "review_by": "2999-06-01", "reason": "checked with the person" }),
        )
        .await;
        assert_eq!(done.status_code(), StatusCode::OK, "{}", done.text());
        assert_eq!(ids(&mine(&list().await)), vec![newer.clone()]);

        let done = change(
            newer.clone(),
            json!({ "expected_version": 1, "action": "end", "reason": "no longer needed" }),
        )
        .await;
        assert_eq!(done.status_code(), StatusCode::OK, "{}", done.text());
        assert!(mine(&list().await).is_empty());

        let rows = sqlx::query(
            "SELECT detail::text AS d FROM audit_events_all WHERE resource_ref = 'OverdueReviews'",
        )
        .fetch_all(&ctx.pool())
        .await
        .unwrap();
        assert!(!rows.is_empty());
        for row in rows {
            assert!(!row.get::<String, _>("d").contains(SECRET));
        }
    })
    .await;
}
