//! DB-gated (AG-12): a national number is found however it was spaced, by registration, search and the feed's lookup; any
//! other identifier is still exact; the record keeps what was typed; and the database's key is the Rust one.
//!
//! ```text
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test main identifier_keys:: -- --ignored --test-threads=1
//! ```

use electronic_patient_record::patients_store::{self, Search, identifier_key};
use electronic_patient_record_matcher::{
    HumanName, IHI_SYSTEM, Identifier, NATIONAL_NUMBER_SYSTEM, Patient,
};
use sqlx::PgPool;

async fn db() -> PgPool {
    let url = std::env::var("DB_URL").expect("DB_URL must be set for --ignored tests");
    let db = PgPool::connect(&url).await.expect("connect");
    electronic_patient_record::MIGRATOR
        .run(&db)
        .await
        .expect("migrate");
    db
}

fn stamp() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

/// A valid national number no earlier run used.
fn number() -> String {
    (0_u128..)
        .flat_map(|seed| {
            let nine = format!("{:09}", (stamp() / 1000 + seed) % 1_000_000_000);
            (0..10).map(move |check| format!("{nine}{check}"))
        })
        .find(|n| electronic_patient_record_matcher::is_valid_national_number(n))
        .expect("one check digit in eleven")
}

fn with(system: &str, value: &str) -> Patient {
    Patient {
        names: vec![HumanName {
            family: Some(format!("Zzkey{}", stamp())),
            given: vec!["K".into()],
        }],
        identifiers: vec![Identifier {
            system: system.into(),
            value: value.into(),
        }],
        ..Default::default()
    }
}

fn find(system: &str, value: &str) -> Search {
    Search {
        identifier: Some(format!("{system}|{value}")),
        ..Search::default()
    }
}

#[test]
fn only_a_national_number_loses_its_spaces() {
    for system in [NATIONAL_NUMBER_SYSTEM, IHI_SYSTEM] {
        assert_eq!(
            identifier_key(system, " 943-476\u{a0}5919"),
            format!("{system}|9434765919")
        );
    }
    assert_eq!(identifier_key("urn:local", "AB 12-3"), "urn:local|AB 12-3");
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn the_database_makes_the_key_the_service_asks_for() {
    let db = db().await;
    // Every character Rust calls white space, so that a new one in either fails here.
    let spaces: String = ('\u{1}'..=char::MAX)
        .filter(|c| c.is_whitespace())
        .collect();
    assert!(spaces.chars().count() >= 25);
    for system in [NATIONAL_NUMBER_SYSTEM, IHI_SYSTEM, "urn:local", "urn:a|b"] {
        for value in [
            "9434765919",
            "943 476 5919",
            "943-476-5919",
            "",
            "\u{200b}1\u{2011}2",
            &format!("9{spaces}4-3"),
        ] {
            let made: String = sqlx::query_scalar("SELECT identifier_key($1, $2)")
                .bind(system)
                .bind(value)
                .fetch_one(&db)
                .await
                .unwrap();
            assert_eq!(made, identifier_key(system, value), "{system} {value:?}");
        }
    }
    // A body with no identifiers, or with something else in their place, has none and is not an error.
    for body in [
        "{}",
        r#"{"identifiers": null}"#,
        r#"{"identifiers": "x"}"#,
        r#"{"identifiers": [{"system": "s"}]}"#,
    ] {
        let keys: Vec<String> = sqlx::query_scalar("SELECT identifier_keys($1::jsonb)")
            .bind(serde_json::from_str::<serde_json::Value>(body).unwrap())
            .fetch_one(&db)
            .await
            .unwrap();
        assert!(keys.is_empty(), "{body}");
    }
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_number_is_found_however_it_was_typed_and_kept_as_it_was() {
    let db = db().await;
    let n = number();
    let spaced = format!("{} {} {}", &n[..3], &n[3..6], &n[6..]);
    let hyphened = spaced.replace(' ', "-");
    for (typed, asked) in [(&n, &spaced), (&spaced, &n), (&hyphened, &spaced)] {
        let stored = patients_store::create(&db, "t", None, &with(NATIONAL_NUMBER_SYSTEM, typed))
            .await
            .unwrap();
        assert_eq!(
            &stored.patient.identifiers[0].value, typed,
            "kept as it was typed"
        );
        let seen = |found: &[patients_store::Stored]| found.iter().any(|f| f.id == stored.id);
        // The HL7 feed finds the patient of a message through this search.
        let found = patients_store::search(&db, &find(NATIONAL_NUMBER_SYSTEM, asked))
            .await
            .unwrap();
        assert!(seen(&found), "search {typed} by {asked}");
        let found = patients_store::candidates(&db, &with(NATIONAL_NUMBER_SYSTEM, asked))
            .await
            .unwrap();
        assert!(seen(&found), "registration {typed} by {asked}");
    }
    // Another number is another person, and another system's value is exact.
    let other = number();
    let found = patients_store::search(&db, &find(NATIONAL_NUMBER_SYSTEM, &other))
        .await
        .unwrap();
    assert!(found.is_empty());
    let local = format!("AB {}", stamp());
    let kept = patients_store::create(&db, "t", None, &with("urn:local", &local))
        .await
        .unwrap();
    let exact = patients_store::search(&db, &find("urn:local", &local))
        .await
        .unwrap();
    assert_eq!(exact.len(), 1);
    assert_eq!(exact[0].id, kept.id);
    let loose = patients_store::search(&db, &find("urn:local", &local.replace(' ', "")))
        .await
        .unwrap();
    assert!(
        loose.is_empty(),
        "a local number with a space is not one without"
    );
    // The number under another system is not the national number.
    let found = patients_store::search(&db, &find("urn:local", &n))
        .await
        .unwrap();
    assert!(found.is_empty());
}
