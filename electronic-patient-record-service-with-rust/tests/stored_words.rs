//! DB-gated (PR-3): the words the code stores match the `CHECK (... IN (...))` list the database enforces.
//! A word added to the code without the migration fails here, and so does the reverse.
//!
//! ```text
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test stored_words -- --ignored --test-threads=1
//! ```
//!
//! Covered: referral states, allergy, consent, death assertion, adjustment and result statuses, and suspicion outcome kinds. Not covered: amendment states (a tagged
//! enum with no list), observation statuses, queue kinds and clearances (private lists), and a
//! variant left out of its `ALL` array (the compiler cannot see that).

use sqlx::{PgPool, Row};
use std::collections::BTreeSet;

use electronic_patient_record::{
    adjustments, allergies, consent, death, referral, results, suspicion,
};

async fn db() -> PgPool {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for --ignored tests");
    let db = PgPool::connect(&url).await.expect("connect");
    electronic_patient_record::MIGRATOR
        .run(&db)
        .await
        .expect("migrate");
    db
}

/// The words in the `IN (...)` check on `table.column`, read from the catalogue.
async fn allowed(db: &PgPool, table: &str, column: &str) -> BTreeSet<String> {
    let rows = sqlx::query(
        "SELECT pg_get_constraintdef(c.oid) AS def FROM pg_constraint c
             JOIN pg_class t ON t.oid = c.conrelid
             WHERE c.contype = 'c' AND t.relname = $1 AND pg_get_constraintdef(c.oid) LIKE $2",
    )
    .bind(table)
    .bind(format!("%{column} = ANY%"))
    .fetch_all(db)
    .await
    .expect("catalogue");
    let mut words = BTreeSet::new();
    for r in rows {
        let def: String = r.try_get("def").expect("def");
        // CHECK ((state = ANY (ARRAY['requested'::text, ...])))
        let mut parts = def.split('\'');
        parts.next();
        while let (Some(word), Some(_)) = (parts.next(), parts.next()) {
            words.insert(word.to_string());
        }
    }
    words
}

async fn same(db: &PgPool, table: &str, column: &str, code: Vec<&str>) {
    let db_words = allowed(db, table, column).await;
    let code_words: BTreeSet<String> = code.into_iter().map(String::from).collect();
    assert!(
        !db_words.is_empty(),
        "{table}.{column}: no CHECK list found"
    );
    let only_code: Vec<_> = code_words.difference(&db_words).collect();
    let only_db: Vec<_> = db_words.difference(&code_words).collect();
    assert!(
        only_code.is_empty() && only_db.is_empty(),
        "{table}.{column}: only in code {only_code:?}, only in the database {only_db:?}"
    );
}

#[tokio::test]
#[ignore = "needs DATABASE_URL"]
async fn stored_words_match_the_database_checks() {
    let db = db().await;
    same(
        &db,
        "referral_suspicion_outcomes",
        "kind",
        suspicion::OutcomeKind::ALL
            .iter()
            .map(|k| k.word())
            .collect(),
    )
    .await;
    same(
        &db,
        "referral_versions",
        "state",
        referral::State::ALL.iter().map(|s| s.as_str()).collect(),
    )
    .await;
    same(
        &db,
        "allergy_versions",
        "status",
        allergies::Status::ALL.iter().map(|s| s.as_str()).collect(),
    )
    .await;
    same(
        &db,
        "consent_versions",
        "status",
        consent::Status::ALL.iter().map(|s| s.as_str()).collect(),
    )
    .await;
    same(
        &db,
        "death_assertion_versions",
        "status",
        death::Status::ALL.iter().map(|s| s.word()).collect(),
    )
    .await;
    same(
        &db,
        "adjustment_versions",
        "status",
        adjustments::Status::ALL.iter().map(|s| s.word()).collect(),
    )
    .await;
    same(
        &db,
        "result_versions",
        "status",
        results::Status::ALL.iter().map(|s| s.word()).collect(),
    )
    .await;
}

#[test]
fn the_word_reader_is_given_a_real_list() {
    // The parser is exercised by the database test; this keeps the code lists non-empty without one.
    assert_eq!(referral::State::ALL.len(), 12);
    assert_eq!(allergies::Status::ALL.len(), 3);
    assert_eq!(consent::Status::ALL.len(), 3);
}
