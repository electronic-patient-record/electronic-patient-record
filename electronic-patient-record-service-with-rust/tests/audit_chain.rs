//! DB-gated: the audit writer appends a verifiable hash chain.
//!
//! Each test gets its own scratch database, so rows written by other tests cannot affect the chain.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test audit_chain -- --ignored --test-threads=1
//! ```

use electronic_patient_record::audit::{self, Action, Chain, Event, Verdict};
use electronic_patient_record::auth::Agent;
use electronic_patient_record::controllers::patients::{CompareRequest, compare_audited};
use electronic_patient_record_matcher::{HumanName, Patient};
use electronic_patient_record_migration::{Migrator, MigratorTrait};
use integrity_mac::{KeyConfig, KeySet};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection};

const KEY: &str = "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";
const OTHER_KEY: &str = "1f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";

fn keyed(key: &str) -> Chain {
    Chain::new(KeySet::load(&KeyConfig::new("t", "T"), Some(key), None))
}

/// A migrated scratch database, dropped when the test ends (best effort).
struct Scratch {
    db: DatabaseConnection,
    admin: DatabaseConnection,
    name: String,
}

impl Scratch {
    async fn new() -> Self {
        let url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for --ignored tests");
        let (base, _) = url.rsplit_once('/').expect("a database URL");
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let name = format!("epr_scratch_{}_{}", std::process::id(), nanos);
        let admin = Database::connect(url.clone()).await.expect("connect admin");
        admin
            .execute_unprepared(&format!("CREATE DATABASE {name}"))
            .await
            .expect("create scratch db");
        let db = Database::connect(format!("{base}/{name}"))
            .await
            .expect("connect scratch");
        Migrator::up(&db, None).await.expect("migrate");
        Self { db, admin, name }
    }

    /// Turn the immutability triggers off, to simulate someone with database ownership.
    async fn tamper(&self, sql: &str) {
        self.db
            .execute_unprepared("ALTER TABLE audit_events DISABLE TRIGGER USER")
            .await
            .unwrap();
        self.db.execute_unprepared(sql).await.unwrap();
        self.db
            .execute_unprepared("ALTER TABLE audit_events ENABLE TRIGGER USER")
            .await
            .unwrap();
    }

    async fn finish(self) {
        let Scratch { db, admin, name } = self;
        drop(db);
        let _ = admin
            .execute_unprepared(&format!("DROP DATABASE IF EXISTS {name} WITH (FORCE)"))
            .await;
    }
}

fn event(n: usize) -> Event {
    let mut e = Event::new(
        if n.is_multiple_of(2) {
            Action::Read
        } else {
            Action::Update
        },
        format!("user-{n}"),
    );
    e.patient_ref = Some(format!("Patient/{n}"));
    e.detail = Some(serde_json::json!({ "n": n, "note": "café" }));
    e
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn appended_rows_form_an_intact_chain_keyed_and_unkeyed() {
    for chain in [keyed(KEY), Chain::unkeyed()] {
        let s = Scratch::new().await;
        for n in 0..5 {
            audit::append(&s.db, &chain, &event(n)).await.unwrap();
        }
        assert_eq!(
            audit::verify(&s.db, &chain).await.unwrap(),
            Verdict::Intact { rows: 5 }
        );
        s.finish().await;
    }
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn an_empty_table_is_intact() {
    let s = Scratch::new().await;
    assert_eq!(
        audit::verify(&s.db, &Chain::unkeyed()).await.unwrap(),
        Verdict::Intact { rows: 0 }
    );
    s.finish().await;
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn editing_a_row_breaks_the_chain_at_that_row() {
    let chain = keyed(KEY);
    let s = Scratch::new().await;
    let mut ids = vec![];
    for n in 0..4 {
        ids.push(audit::append(&s.db, &chain, &event(n)).await.unwrap());
    }
    s.tamper(&format!(
        "UPDATE audit_events SET agent = 'mallory' WHERE id = {}",
        ids[2]
    ))
    .await;
    match audit::verify(&s.db, &chain).await.unwrap() {
        Verdict::Broken { id, .. } => assert_eq!(id, ids[2]),
        other => panic!("expected the chain to break, got {other:?}"),
    }
    s.finish().await;
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn deleting_a_row_breaks_the_chain_at_the_next_row() {
    let chain = keyed(KEY);
    let s = Scratch::new().await;
    let mut ids = vec![];
    for n in 0..4 {
        ids.push(audit::append(&s.db, &chain, &event(n)).await.unwrap());
    }
    s.tamper(&format!("DELETE FROM audit_events WHERE id = {}", ids[1]))
        .await;
    match audit::verify(&s.db, &chain).await.unwrap() {
        Verdict::Broken { id, .. } => assert_eq!(id, ids[2]),
        other => panic!("expected the chain to break, got {other:?}"),
    }
    s.finish().await;
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn recomputing_digests_without_the_key_is_detected() {
    // Someone with only the database edits a row and recomputes its digest as plain SHA-256.
    let chain = keyed(KEY);
    let s = Scratch::new().await;
    let id = audit::append(&s.db, &chain, &event(0)).await.unwrap();
    s.tamper(&format!(
        "UPDATE audit_events SET digest = 'sha256:' || repeat('0', 64) WHERE id = {id}"
    ))
    .await;
    assert!(matches!(
        audit::verify(&s.db, &chain).await.unwrap(),
        Verdict::Broken { .. }
    ));
    // And a chain checked with the wrong key does not verify.
    let s2 = Scratch::new().await;
    audit::append(&s2.db, &chain, &event(0)).await.unwrap();
    assert!(matches!(
        audit::verify(&s2.db, &keyed(OTHER_KEY)).await.unwrap(),
        Verdict::Broken { .. }
    ));
    s.finish().await;
    s2.finish().await;
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn concurrent_appends_do_not_fork_the_chain() {
    let s = Scratch::new().await;
    let chain = std::sync::Arc::new(keyed(KEY));
    let mut tasks = vec![];
    for n in 0..20 {
        let (db, chain) = (s.db.clone(), chain.clone());
        tasks.push(tokio::spawn(async move {
            audit::append(&db, &chain, &event(n)).await.unwrap()
        }));
    }
    for t in tasks {
        t.await.unwrap();
    }
    assert_eq!(
        audit::verify(&s.db, &chain).await.unwrap(),
        Verdict::Intact { rows: 20 }
    );
    s.finish().await;
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn comparing_patients_writes_an_audit_row_without_the_demographics() {
    let chain = keyed(KEY);
    let s = Scratch::new().await;
    let patient = |family: &str| Patient {
        names: vec![HumanName {
            family: Some(family.into()),
            given: vec!["Zebediah".into()],
        }],
        birth_date: Some("1971-02-03".into()),
        ..Default::default()
    };
    let request = CompareRequest {
        a: patient("Quimby"),
        b: patient("Quimby"),
    };
    let result = compare_audited(&s.db, &chain, &Agent("user-42".into()), &request)
        .await
        .unwrap();
    assert_eq!(serde_json::to_value(result.decision).unwrap(), "auto_link");

    let row = s
        .db
        .query_one_raw(sea_orm::Statement::from_string(
            sea_orm::DbBackend::Postgres,
            "SELECT action::text AS action, agent, resource_ref, detail::text AS detail FROM audit_events",
        ))
        .await
        .unwrap()
        .expect("one audit row");
    assert_eq!(row.try_get::<String>("", "action").unwrap(), "E");
    assert_eq!(row.try_get::<String>("", "agent").unwrap(), "user-42");
    assert_eq!(
        row.try_get::<String>("", "resource_ref").unwrap(),
        "Patient/$compare"
    );
    let detail = row.try_get::<String>("", "detail").unwrap();
    for secret in ["Quimby", "Zebediah", "1971"] {
        assert!(
            !detail.contains(secret),
            "audit detail leaked {secret}: {detail}"
        );
    }
    assert_eq!(
        audit::verify(&s.db, &chain).await.unwrap(),
        Verdict::Intact { rows: 1 }
    );
    s.finish().await;
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn verifying_in_small_batches_gives_the_same_answer_and_finds_a_break_across_a_boundary() {
    let chain = keyed(KEY);
    let s = Scratch::new().await;
    let mut ids = vec![];
    for n in 0..7 {
        ids.push(audit::append(&s.db, &chain, &event(n)).await.unwrap());
    }
    for batch in [1, 2, 3, 7, 8, 1000] {
        assert_eq!(
            audit::verify_in_batches(&s.db, &chain, batch)
                .await
                .unwrap(),
            Verdict::Intact { rows: 7 },
            "batch {batch}"
        );
    }
    // Rows 3 and 4 sit either side of a boundary when the batch is 4 (ids[0..4] | ids[4..7]).
    s.tamper(&format!(
        "UPDATE audit_events SET agent = 'mallory' WHERE id = {}",
        ids[4]
    ))
    .await;
    for batch in [1, 2, 4, 5, 1000] {
        match audit::verify_in_batches(&s.db, &chain, batch)
            .await
            .unwrap()
        {
            Verdict::Broken { id, .. } => assert_eq!(id, ids[4], "batch {batch}"),
            other => panic!("batch {batch}: expected the chain to break, got {other:?}"),
        }
    }
    s.finish().await;
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn the_verify_endpoint_logic_reports_intact_then_409_and_audits_each_check() {
    use axum::http::StatusCode;
    use electronic_patient_record::controllers::audit::verify_audited;

    let chain = keyed(KEY);
    let s = Scratch::new().await;
    let mut ids = vec![];
    for n in 0..3 {
        ids.push(audit::append(&s.db, &chain, &event(n)).await.unwrap());
    }
    let who = Agent("auditor-1".into());

    let (status, body) = verify_audited(&s.db, &chain, &who, None).await.unwrap();
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["intact"], true);
    assert_eq!(body["rows"], 3);
    assert_eq!(body["keyed"], true);

    s.tamper(&format!(
        "UPDATE audit_events SET agent = 'mallory' WHERE id = {}",
        ids[1]
    ))
    .await;
    let (status, body) = verify_audited(&s.db, &chain, &who, None).await.unwrap();
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["intact"], false);
    assert_eq!(body["first_broken_id"], ids[1]);

    // Each check wrote one row naming the auditor, with the outcome and no row contents.
    let rows = s
        .db
        .query_all_raw(sea_orm::Statement::from_sql_and_values(
            sea_orm::DbBackend::Postgres,
            "SELECT resource_ref, purpose_of_use, detail::text AS detail FROM audit_events WHERE agent = $1 ORDER BY id",
            ["auditor-1".into()],
        ))
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
    for r in &rows {
        assert_eq!(
            r.try_get::<String>("", "resource_ref").unwrap(),
            "AuditLog/$verify"
        );
        assert_eq!(r.try_get::<String>("", "purpose_of_use").unwrap(), "audit");
    }
    let second: String = rows[1].try_get("", "detail").unwrap();
    assert!(second.contains("\"intact\": false") && second.contains(&ids[1].to_string()));
    assert!(!second.contains("mallory"));

    // An unkeyed chain says so.
    let s2 = Scratch::new().await;
    audit::append(&s2.db, &Chain::unkeyed(), &event(0))
        .await
        .unwrap();
    let (_, body) = verify_audited(&s2.db, &Chain::unkeyed(), &who, None)
        .await
        .unwrap();
    assert_eq!(body["keyed"], false);
    assert!(body["assurance"].as_str().unwrap().contains("unkeyed"));
    s2.finish().await;
    s.finish().await;
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_published_head_catches_removed_newest_rows_and_a_rewritten_head() {
    use axum::http::StatusCode;
    use electronic_patient_record::audit::{Head, Verdict, verify_against};
    use electronic_patient_record::controllers::audit::verify_audited;

    let chain = keyed(KEY);
    let s = Scratch::new().await;
    let mut ids = vec![];
    for n in 0..5 {
        ids.push(audit::append(&s.db, &chain, &event(n)).await.unwrap());
    }
    let who = Agent("auditor-2".into());

    // The first answer carries the head to publish.
    let (status, body) = verify_audited(&s.db, &chain, &who, None).await.unwrap();
    assert_eq!(status, StatusCode::OK);
    let head: Head = serde_json::from_value(body["head"].clone()).unwrap();
    assert_eq!(head.id, ids[4]);
    assert_eq!(body["checkpoint_confirmed"], false);

    // More rows after the checkpoint: still fine, and the checkpoint is confirmed.
    audit::append(&s.db, &chain, &event(5)).await.unwrap();
    let (status, body) = verify_audited(&s.db, &chain, &who, Some(&head))
        .await
        .unwrap();
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["checkpoint_confirmed"], true);

    // Without a checkpoint, removing the newest rows goes unnoticed (the documented gap)...
    s.tamper(&format!("DELETE FROM audit_events WHERE id >= {}", ids[3]))
        .await;
    let (v, _) = verify_against(&s.db, &chain, 2, None).await.unwrap();
    assert_eq!(v, Verdict::Intact { rows: 3 });
    // ...and with one, it is caught at the checkpoint's row, across small batches too.
    for batch in [1, 2, 100] {
        let (v, _) = verify_against(&s.db, &chain, batch, Some(&head))
            .await
            .unwrap();
        match v {
            Verdict::Broken { id, reason } => {
                assert_eq!(id, head.id);
                assert!(reason.contains("missing"), "{reason}");
            }
            other => panic!("batch {batch}: expected Broken, got {other:?}"),
        }
    }
    let (status, body) = verify_audited(&s.db, &chain, &who, Some(&head))
        .await
        .unwrap();
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["first_broken_id"], head.id);

    // A checkpoint whose digest differs from the stored row is a rewrite.
    let wrong = Head {
        id: ids[2],
        digest: "sha256:00".into(),
    };
    let (v, _) = verify_against(&s.db, &chain, 100, Some(&wrong))
        .await
        .unwrap();
    assert!(matches!(v, Verdict::Broken { id, .. } if id == ids[2]));

    // A checkpoint on an empty chain is missing too.
    let s2 = Scratch::new().await;
    let (v, h) = verify_against(&s2.db, &chain, 100, Some(&head))
        .await
        .unwrap();
    assert!(matches!(v, Verdict::Broken { .. }) && h.is_none());
    s2.finish().await;
    s.finish().await;
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn the_patient_log_is_one_patients_rows_newest_first_in_pages_without_detail() {
    let chain = Chain::unkeyed();
    let s = Scratch::new().await;
    let mine = "3b0c6f0e-1111-4111-8111-000000000001";
    let other = "3b0c6f0e-1111-4111-8111-000000000002";
    let mut ids = vec![];
    for n in 0..5 {
        let mut e = event(n);
        e.patient_ref = Some(format!("Patient/{}", if n == 3 { other } else { mine }));
        // The row's case is whatever the caller used.
        if n == 1 {
            e.patient_ref = Some(format!("Patient/{}", mine.to_uppercase()));
        }
        e.detail = Some(serde_json::json!({ "secret": "do-not-show" }));
        // Only the newest row is written inside a request, so it alone records an actor.
        let actor = audit::Actor {
            roles: vec!["clinician".into()],
            organisations: vec!["riverside".into()],
        };
        if n == 4 {
            ids.push(
                audit::ACTOR
                    .scope(actor, audit::append(&s.db, &chain, &e))
                    .await
                    .unwrap(),
            );
        } else {
            ids.push(audit::append(&s.db, &chain, &e).await.unwrap());
        }
    }
    let page = audit::patient_log(&s.db, mine, None, 3).await.unwrap();
    assert_eq!(
        page.iter().map(|r| r.id).collect::<Vec<_>>(),
        vec![ids[4], ids[2], ids[1]],
        "newest first, other patient excluded, upper-case ref included"
    );
    let rest = audit::patient_log(&s.db, mine, Some(ids[1]), 3)
        .await
        .unwrap();
    assert_eq!(rest.iter().map(|r| r.id).collect::<Vec<_>>(), vec![ids[0]]);
    assert!(!format!("{page:?}").contains("do-not-show"));
    assert_eq!(page[0].roles, vec!["clinician".to_string()]);
    assert_eq!(page[0].organisations, vec!["riverside".to_string()]);
    assert!(page[1].roles.is_empty(), "an older row has no roles");
    // The chain still verifies with the actor inside the hashed detail.
    assert_eq!(
        audit::verify(&s.db, &chain).await.unwrap(),
        Verdict::Intact { rows: 5 }
    );
    s.finish().await;
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn rows_under_a_key_we_do_not_hold_are_unchecked_not_broken_and_a_real_break_still_wins() {
    use electronic_patient_record::audit::verify_against;
    use electronic_patient_record::controllers::audit::verify_audited;

    let key_as = |id: &str| {
        Chain::new(KeySet::load_with_domains(
            &KeyConfig::new("t", "T"),
            Some(KEY),
            None,
            Some(id),
            &[],
        ))
    };
    let (old, new) = (key_as("k1"), key_as("k2"));
    let s = Scratch::new().await;
    let mut ids = vec![];
    // Two rows under the old key, then the key is rotated and two more are written under k2.
    for n in 0..2 {
        ids.push(audit::append(&s.db, &old, &event(n)).await.unwrap());
    }
    for n in 2..4 {
        ids.push(audit::append(&s.db, &new, &event(n)).await.unwrap());
    }
    // The operator forgot to keep the old key: the first two rows cannot be checked.
    match audit::verify(&s.db, &new).await.unwrap() {
        Verdict::Unchecked {
            rows,
            unchecked,
            first_id,
            causes,
        } => {
            assert_eq!((rows, unchecked, first_id), (4, 2, ids[0]));
            assert_eq!(causes, vec!["key k1 is not held".to_string()]);
        }
        other => panic!("expected Unchecked, got {other:?}"),
    }
    // The endpoint says so without calling it tampering: 424, not 409, and names the cause.
    let who = Agent("auditor-3".into());
    let (status, body) = verify_audited(&s.db, &new, &who, None).await.unwrap();
    assert_eq!(status, axum::http::StatusCode::FAILED_DEPENDENCY);
    assert_eq!(body["tampering_found"], false);
    assert_eq!(body["error"], "key-not-held");
    assert!(
        body["assurance"]
            .as_str()
            .unwrap()
            .contains("not evidence of tampering")
    );
    // With the old key kept as retired, the whole chain is intact.
    let both = Chain::new(KeySet::load_with_domains(
        &KeyConfig::new("t", "T"),
        Some(OTHER_KEY),
        Some(&format!("k1:{KEY}")),
        Some("k2"),
        &[],
    ));
    // (That key set holds k1 = KEY and k2 = OTHER_KEY, which is not what wrote k2 above, so the
    // two k2 rows are now a mismatch: a held key that disagrees is tampering evidence.)
    assert!(matches!(
        audit::verify(&s.db, &both).await.unwrap(),
        Verdict::Broken { id, .. } if id == ids[2]
    ));
    // A row that cannot be checked does not hide a break further down the chain.
    s.tamper(&format!(
        "UPDATE audit_events SET agent = 'mallory' WHERE id = {}",
        ids[3]
    ))
    .await;
    assert!(matches!(
        audit::verify(&s.db, &new).await.unwrap(),
        Verdict::Broken { id, .. } if id == ids[3]
    ));
    // With no key at all every row is unchecked (the four above and the verify row the endpoint wrote), and a head is still reported: it needs no key.
    let (v, head) = verify_against(&s.db, &Chain::unkeyed(), 2, None)
        .await
        .unwrap();
    assert!(
        matches!(v, Verdict::Unchecked { unchecked: 5, .. }),
        "{v:?}"
    );
    assert!(head.is_some());
    s.finish().await;
}
