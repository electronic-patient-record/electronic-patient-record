//! DB-gated, through the real router with login off: `GET /fhir/R4/AuditEvent` pages by row id. A search with more
//! than one page of matches answers a full page and a `next` link that keeps the filters and adds `_before`; following
//! it gives the rest and no further link; the pages share no row. A query the route does not know is refused.
//!
//! ```text
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test suite fhir_audit_paging:: -- --ignored --test-threads=1
//! ```

use axa_test::prelude::request;
use electronic_patient_record::app::App;
use electronic_patient_record::audit::{self, Action, Event};
use electronic_patient_record::pool::Pool;
use serde_json::Value;

fn ids(page: &Value) -> Vec<String> {
    page["entry"].as_array().map_or_else(Vec::new, |a| {
        a.iter()
            .filter_map(|e| e["resource"]["id"].as_str().map(String::from))
            .collect()
    })
}

fn next(page: &Value) -> Option<String> {
    page["link"]
        .as_array()?
        .iter()
        .find(|l| l["relation"] == "next")?["url"]
        .as_str()
        .map(String::from)
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_search_with_more_than_a_page_of_matches_links_to_the_rest() {
    request::<App, _, _>(|server, ctx| async move {
        let agent = format!(
            "paging-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        for _ in 0..(audit::SEARCH_LIMIT + 7) {
            let event = Event::new(Action::Read, agent.clone());
            audit::append(&ctx.pool(), &audit::chain(), &event)
                .await
                .unwrap();
        }
        let first: Value = server
            .get(&format!("/fhir/R4/AuditEvent?agent={agent}"))
            .await
            .json();
        let first_ids = ids(&first);
        assert_eq!(
            first_ids.len(),
            usize::try_from(audit::SEARCH_LIMIT).unwrap()
        );
        let link = next(&first).expect("a full page links to the next");
        assert!(link.contains(&format!("agent={agent}")), "{link}");
        assert!(link.contains("_before="), "{link}");

        let path = link
            .split_once("/fhir/R4")
            .map_or(link.as_str(), |(_, p)| p);
        let second: Value = server.get(&format!("/fhir/R4{path}")).await.json();
        let second_ids = ids(&second);
        assert_eq!(second_ids.len(), 7, "{second}");
        assert!(second_ids.iter().all(|i| !first_ids.contains(i)));
        assert!(next(&second).is_none(), "the last page has no next link");

        // A query string the route does not know is refused, not ignored.
        let bad = server.get("/fhir/R4/AuditEvent?agnet=x").await;
        assert_eq!(bad.status_code(), 400);
    })
    .await;
}
