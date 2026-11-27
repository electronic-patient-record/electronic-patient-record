//! DB-gated, through the real router with login on: patient search fills its page past patients whose refusal of
//! disclosure hides them from the caller (`axa::cursor_pagination` 0.1.4, `CursorPage::fill`), so that a short page does
//! not say how many were hidden; its `Link: rel="next"` holds only for the same caller and purpose.
//!
//! ```text
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test main http_patient_search_pages:: -- --ignored --test-threads=1
//! ```

use authentication_verifier::test_idp::{AUDIENCE, REALM, TestIdp};
use axa_test::prelude::request_with;
use axum::http::{HeaderName, HeaderValue, StatusCode};
use electronic_patient_record::app::App;
use serde_json::{Value, json};

fn letters() -> String {
    let mut n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mut s = String::new();
    while n > 0 {
        s.push((b'a' + (n % 26) as u8) as char);
        n /= 26;
    }
    s
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_search_page_is_filled_past_who_is_hidden_and_its_next_link_is_the_callers() {
    let idp = TestIdp::start().await;
    let map: Value = ["clinical-admin", "clinician", "nurse"]
        .iter()
        .map(|r| ((*r).to_string(), json!({ "role": [r] })))
        .collect();
    let db_url = std::env::var("DB_URL").expect("DB_URL must be set for --ignored tests");
    let role_map = map.to_string();
    let settings: Vec<(&str, &str)> = vec![
        ("DB_URL", &db_url),
        ("EPR_REQUIRE_AUTH", "true"),
        ("EPR_KEYCLOAK_URL", &idp.base_url),
        ("EPR_KEYCLOAK_REALM", REALM),
        ("EPR_KEYCLOAK_AUDIENCES", AUDIENCE),
        ("EPR_KEYCLOAK_ROLE_MAP", &role_map),
    ];
    let token = |role: &str, n: u8| {
        idp.token(|c| {
            c["realm_access"] = json!({ "roles": [role] });
            c["sub"] = json!(format!("0c4f1e2a-0000-4000-8000-00000000f4{n:02x}"));
        })
    };
    let (admin, doctor, nurse) = (
        token("clinical-admin", 0),
        token("clinician", 1),
        token("nurse", 2),
    );
    request_with::<App, _, _>(&settings, |server, _ctx| async move {
        let server = &server;
        let call = |token: &str, path: String, body: Option<Value>| {
            let auth = HeaderValue::from_str(&format!("Bearer {token}")).unwrap();
            async move {
                let r = match &body {
                    Some(b) => server.post(&path).json(b),
                    None => server.get(&path),
                };
                r.add_header(HeaderName::from_static("authorization"), auth)
                    .add_header(HeaderName::from_static("x-purpose-of-use"), HeaderValue::from_static("treatment"))
                    .await
            }
        };
        // Five patients in search order; the first two refuse disclosure for treatment.
        let base = format!("sp{}", letters());
        let mut ids = Vec::new();
        for (i, tail) in ["a", "b", "c", "d", "e"].iter().enumerate() {
            let family = format!("{base}{tail}");
            let made = call(&admin, "/api/patients".into(), Some(json!({
                "patient": { "names": [{ "family": family, "given": [format!("G{i}")] }], "birth_date": "1960-01-01" },
                "override_reason": "test setup"
            }))).await;
            assert_eq!(made.status_code(), StatusCode::CREATED, "{}", made.text());
            ids.push(made.json::<Value>()["id"].as_str().unwrap().to_string());
        }
        for id in &ids[..2] {
            let refused = call(&admin, format!("/api/patients/{id}/consents"), Some(json!({ "consent": {
                "kind": "general", "status": "active",
                "provisions": [{ "effect": "deny", "purposes": ["treatment"] }]
            }}))).await;
            assert_eq!(refused.status_code(), StatusCode::CREATED, "{}", refused.text());
        }

        // A page of two is two the caller may see, not two minus the hidden ones.
        let first = call(&doctor, format!("/api/patients?family={base}&limit=2"), None).await;
        assert_eq!(first.status_code(), StatusCode::OK, "{}", first.text());
        let shown: Vec<String> = first.json::<Value>()["results"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["id"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(shown, ids[2..4].to_vec());
        let link = first
            .headers()
            .get("link")
            .expect("more follow: a next link")
            .to_str()
            .unwrap()
            .to_string();
        assert!(link.starts_with(&format!("</api/patients?family={base}&limit=2&cursor=")), "{link}");
        let next_path = link.trim_start_matches('<').split('>').next().unwrap().to_string();

        // The next page starts after the last one shown, and the list ends there.
        let second = call(&doctor, next_path.clone(), None).await;
        assert_eq!(second.status_code(), StatusCode::OK, "{}", second.text());
        let rest = second.json::<Value>()["results"].as_array().unwrap().clone();
        assert_eq!(rest.len(), 1);
        assert_eq!(rest[0]["id"], ids[4].as_str());
        assert!(second.headers().get("link").is_none());

        // The cursor is the caller's: another caller, who may see other rows, is refused it; so is a forged one.
        assert_eq!(call(&nurse, next_path, None).await.status_code(), StatusCode::BAD_REQUEST);
        let forged = call(&doctor, format!("/api/patients?family={base}&limit=2&cursor=AAAA"), None).await;
        assert_eq!(forged.status_code(), StatusCode::BAD_REQUEST);
        // A page larger than the search limit is refused, as before there was none.
        let big = call(&doctor, format!("/api/patients?family={base}&limit=51"), None).await;
        assert_eq!(big.status_code(), StatusCode::BAD_REQUEST);

        // A page of the full size holds every patient the caller may see, and says no more follow. The refusal is for
        // treatment, so it hides the two from every caller searching for treatment, the administrator too.
        for who in [&doctor, &admin] {
            let all = call(who, format!("/api/patients?family={base}"), None).await;
            let found: Vec<String> = all.json::<Value>()["results"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| p["id"].as_str().unwrap().to_string())
                .collect();
            assert_eq!(found, ids[2..].to_vec());
            assert!(all.headers().get("link").is_none());
        }
    })
    .await;
}
