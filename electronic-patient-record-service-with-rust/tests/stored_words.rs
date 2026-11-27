//! DB-gated (PR-3): the words the code stores match the `CHECK (... IN (...))` list the database enforces.
//! A word added to the code without the migration fails here, and so does the reverse.
//!
//! ```text
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test main stored_words:: -- --ignored --test-threads=1
//! ```
//!
//! **Every** list the database enforces is compared or listed with the reason it is not (PR-3), read from the catalogue,
//! so a new `CHECK` fails here until someone has looked at it. Enums are spelled out by [`words!`], which stops compiling
//! when a variant is added, so a variant left out of an `ALL` array is caught too.
//!
//! Compared long before that: referral states, allergy, problem, relative history, consent, death assertion, adjustment and result statuses, suspicion outcome kinds and watch list entry statuses.

use sqlx::{PgPool, Row};
use std::collections::BTreeSet;

use electronic_patient_record::{
    adjustments, allergies, consent, death, decision_ability, declined_refusal, emergency, problem,
    referral, relative_history, results, suspicion, treatment_refusal, typing, watch_list,
};

async fn db() -> PgPool {
    let url = std::env::var("DB_URL").expect("DB_URL must be set for --ignored tests");
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

/// The words serde writes for every variant named. The `match` has no catch-all, so a variant that is not named here does
/// not compile.
macro_rules! words {
    ($t:ty: $($v:ident),+ $(,)?) => {{
        fn every(value: $t) {
            match value { $(<$t>::$v => (),)+ }
        }
        [$(<$t>::$v),+]
            .into_iter()
            .map(|v| {
                every(v);
                serde_json::to_value(v).unwrap().as_str().unwrap().to_string()
            })
            .collect::<Vec<String>>()
    }};
}

/// The states of an amendment request, which carry data, so each is made once. No catch-all here either.
fn amendment_states() -> Vec<String> {
    use electronic_patient_record::amendment::{Ground, State};
    let every = |s: &State| match s {
        State::Requested
        | State::Accepted { .. }
        | State::Applied { .. }
        | State::Denied { .. }
        | State::Withdrawn => (),
    };
    let (by, reason) = ("a".to_string(), "r".to_string());
    [
        State::Requested,
        State::Accepted {
            by: by.clone(),
            reason: reason.clone(),
        },
        State::Applied { version: 2 },
        State::Denied {
            by,
            ground: Ground::AccurateAndComplete,
            reason,
            disagreement: None,
        },
        State::Withdrawn,
    ]
    .iter()
    .map(|s| {
        every(s);
        serde_json::to_value(s).unwrap()["state"]
            .as_str()
            .unwrap()
            .to_string()
    })
    .collect()
}

/// Where the code's words for a column come from.
enum Words {
    /// An enum's.
    Code(Vec<String>),
    /// Written out in this file of `src`, as `'word'` or `"word"`: each must be there.
    Literal(&'static str),
    /// Not compared, and why.
    NotOurs(&'static str),
}

fn owned<'a>(words: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    words.into_iter().map(String::from).collect()
}

/// Every column with a list, by `table.column`.
fn columns() -> Vec<(&'static str, Words)> {
    use Words::{Code, Literal, NotOurs};
    use electronic_patient_record as epr;
    vec![
        (
            "adjustment_versions.status",
            Code(words!(adjustments::Status: Active, Ended)),
        ),
        ("allergy_assertion_versions.kind", Literal("allergies.rs")),
        (
            "allergy_versions.status",
            Code(words!(allergies::Status: Active, Inactive, EnteredInError)),
        ),
        ("amendment_versions.state", Code(amendment_states())),
        (
            "audit_events.action",
            Code(
                [
                    epr::audit::Action::Create,
                    epr::audit::Action::Read,
                    epr::audit::Action::Update,
                    epr::audit::Action::Delete,
                    epr::audit::Action::Execute,
                ]
                .map(|a| a.code().to_string())
                .to_vec(),
            ),
        ),
        (
            "audit_events.outcome",
            NotOurs("numbers, FHIR's audit outcomes; written as numbers"),
        ),
        ("axa_idempotency.state", NotOurs("axa's own table")),
        ("axa_outbox_attempts.outcome", NotOurs("axa's own table")),
        ("axa_outbox_owed.state", NotOurs("axa's own table")),
        (
            "break_glass_reviews.outcome",
            Code(words!(epr::break_glass::Outcome: Appropriate, Inappropriate)),
        ),
        (
            "consent_versions.status",
            Code(words!(consent::Status: Active, Inactive, EnteredInError)),
        ),
        (
            "death_assertion_versions.status",
            Code(owned(death::Status::ALL.iter().map(|s| s.word()))),
        ),
        (
            "feed_alarm_states.state",
            Code(words!(epr::feed_health::State: Unknown, Healthy, Warning, Critical)),
        ),
        (
            "feed_refusal_resolutions.outcome",
            Code(words!(epr::feed_queue::Outcome: Handled, Dismissed)),
        ),
        (
            "feed_refusals.kind",
            Code(words!(epr::feed_queue::Kind: Unsupported, Conflict)),
        ),
        (
            "hazard_versions.state",
            Code(words!(epr::hazard::State: Open, Retired)),
        ),
        (
            "observation_versions.status",
            Code(words!(epr::observations_store::Status: Final, EnteredInError)),
        ),
        (
            "outbox_attempts.outcome",
            NotOurs("the outbox before axa's; nothing writes it"),
        ),
        ("entry_labels.act", Literal("entry_labels.rs")),
        (
            "entry_labels.basis",
            Code(words!(epr::labelling::Basis: Clinical, PatientAsked, Custody)),
        ),
        ("entry_labels.kind", Literal("entries_store.rs")),
        (
            "declined_refusal_versions.state",
            Code(words!(decision_ability::State: Recorded, EnteredInError)),
        ),
        (
            "declined_refusal_versions.ground",
            Code(words!(declined_refusal::Ground: Law, RiskToAnother, CannotBeDoneSafely)),
        ),
        (
            "asking_now_versions.state",
            Code(words!(decision_ability::State: Recorded, EnteredInError)),
        ),
        (
            "decision_ability_versions.state",
            Code(words!(decision_ability::State: Recorded, EnteredInError)),
        ),
        (
            "decision_ability_versions.decides",
            Code(words!(decision_ability::Decides: Can, Cannot, NotKnown)),
        ),
        (
            "break_glass_grants.threat",
            Code(words!(emergency::Threat: Life, Limb, Neither, NotKnown)),
        ),
        (
            "break_glass_grants.time_to_harm",
            Code(words!(emergency::TimeToHarm: Minutes, Hours, Days, NotKnown)),
        ),
        (
            "break_glass_grants.can_be_asked",
            Code(words!(emergency::CanBeAsked: Yes, No, NotKnown)),
        ),
        (
            "treatment_refusal_versions.state",
            Code(words!(treatment_refusal::State: Stands, Doubted, Withdrawn, EnteredInError)),
        ),
        (
            "treatment_refusal_versions.kind",
            Code(
                words!(treatment_refusal::Kind: SpokenNow, Advance, Order, Representative, Parent, Carried, Reported),
            ),
        ),
        (
            "treatment_refusal_versions.identity",
            Code(words!(treatment_refusal::IdentityOf: Certain, Uncertain)),
        ),
        ("patient_link_versions.status", Literal("links.rs")),
        ("patient_photos.content_type", Literal("photos_store.rs")),
        (
            "problem_versions.status",
            Code(words!(problem::Status: Active, Remission, Inactive, Resolved, EnteredInError)),
        ),
        (
            "referral_suspicion_outcomes.kind",
            Code(owned(suspicion::OutcomeKind::ALL.iter().map(|k| k.word()))),
        ),
        (
            "referral_unmatched.kind",
            Code(words!(epr::referral_unmatched::Kind: NoId, UnknownId, RefusedByTable)),
        ),
        (
            "referral_unmatched_clearances.how",
            Literal("referral_unmatched.rs"),
        ),
        (
            "referral_versions.state",
            Code(owned(referral::State::ALL.iter().map(|s| s.as_str()))),
        ),
        (
            "relationship_versions.kind",
            Code(words!(epr::relationships::Kind: CareTeam, ActiveEncounter, SelfPatient, Proxy)),
        ),
        (
            "relationship_versions.status",
            Code(words!(epr::relationships::Status: Active, Ended)),
        ),
        (
            "relative_history_versions.status",
            Code(words!(relative_history::Status: Recorded, EnteredInError)),
        ),
        (
            "result_versions.status",
            Code(words!(results::Status: Preliminary, Final, Amended, Corrected, EnteredInError)),
        ),
        (
            "task_reminder_decisions.decision",
            Literal("tasks_store.rs"),
        ),
        (
            "task_versions.status",
            Code(words!(epr::tasks_store::Status: Open, Done, Cancelled)),
        ),
        (
            "typing_versions.status",
            Code(words!(typing::Status: Reported, EnteredInError)),
        ),
        (
            "watch_entry_versions.status",
            Code(owned(watch_list::State::STORED.iter().map(|s| s.word()))),
        ),
    ]
}

#[tokio::test]
#[ignore = "needs DB_URL"]
async fn every_list_the_database_enforces_is_the_codes() {
    let db = db().await;
    // `table.column` for each CHECK that is a list, partitions left out: they carry their parent's.
    let rows = sqlx::query(
        "SELECT t.relname AS t, pg_get_constraintdef(c.oid) AS def FROM pg_constraint c
             JOIN pg_class t ON t.oid = c.conrelid
             WHERE c.contype = 'c' AND NOT t.relispartition AND pg_get_constraintdef(c.oid) LIKE '% = ANY %'",
    )
    .fetch_all(&db)
    .await
    .expect("catalogue");
    let mut enforced = BTreeSet::new();
    for r in rows {
        let (table, def): (String, String) = (r.get("t"), r.get("def"));
        let column = def
            .split(" = ANY ")
            .next()
            .and_then(|s| s.rsplit('(').next())
            .expect("a column")
            .to_string();
        enforced.insert(format!("{table}.{column}"));
    }
    let listed = columns();
    let named: BTreeSet<String> = listed.iter().map(|(c, _)| (*c).to_string()).collect();
    assert_eq!(named.len(), listed.len(), "each column once");
    assert_eq!(
        enforced, named,
        "a list the database enforces and this test does not know, or the reverse"
    );
    for (column, words) in &listed {
        let (table, name) = column.split_once('.').unwrap();
        match words {
            Words::Code(code) => {
                same(&db, table, name, code.iter().map(String::as_str).collect()).await
            }
            Words::Literal(file) => {
                let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("src")
                    .join(file);
                let text = std::fs::read_to_string(&path).unwrap();
                let code = text.split("#[cfg(test)]").next().unwrap();
                let allowed = allowed(&db, table, name).await;
                assert!(!allowed.is_empty(), "{column}");
                for word in allowed {
                    assert!(
                        code.contains(&format!("'{word}'"))
                            || code.contains(&format!("\"{word}\"")),
                        "{column}: {word} is not in {file}"
                    );
                }
            }
            Words::NotOurs(why) => assert!(!why.is_empty(), "{column}"),
        }
    }
}

#[test]
fn the_word_reader_is_given_a_real_list() {
    // The parser is exercised by the database test; this keeps the code lists non-empty without one.
    assert_eq!(referral::State::ALL.len(), 12);
    assert_eq!(allergies::Status::ALL.len(), 3);
    assert_eq!(consent::Status::ALL.len(), 3);
}
