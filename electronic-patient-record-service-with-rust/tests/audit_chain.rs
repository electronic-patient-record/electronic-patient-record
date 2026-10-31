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
    assert_eq!(body["archive_rows"], 0, "nothing archived");

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
async fn the_patient_log_narrows_by_day_by_who_and_by_refusal() {
    let chain = Chain::unkeyed();
    let s = Scratch::new().await;
    let mine = "3b0c6f0e-1111-4111-8111-000000000009";
    // 0: the caller, ok. 1: staff, ok. 2: staff, refused.
    let mut ids = vec![];
    for (agent, outcome) in [("me", 0), ("dr", 0), ("dr", 4)] {
        let mut e = event(0);
        e.purpose_of_use = Some(
            if agent == "me" {
                "patient-access"
            } else {
                "treatment"
            }
            .to_string(),
        );
        e.agent = agent.to_string();
        e.outcome = outcome;
        e.patient_ref = Some(format!("Patient/{mine}"));
        ids.push(audit::append(&s.db, &chain, &e).await.unwrap());
    }
    let log = |f: audit::PatientLogFilter| {
        let db = &s.db;
        async move {
            audit::patient_log(db, mine, "me", &f, 10)
                .await
                .map(|rows| rows.iter().map(|r| r.id).collect::<Vec<_>>())
        }
    };
    let all = vec![ids[2], ids[1], ids[0]];
    assert_eq!(log(Default::default()).await.unwrap(), all);
    let f = |from: Option<&str>, to: Option<&str>| audit::PatientLogFilter {
        from: from.map(Into::into),
        to: to.map(Into::into),
        ..Default::default()
    };
    // Days: from is inclusive, to is exclusive; an instant is exact.
    assert_eq!(
        log(f(Some("2000-01-01"), Some("2999-01-01")))
            .await
            .unwrap(),
        all
    );
    assert!(log(f(Some("2999-01-01"), None)).await.unwrap().is_empty());
    assert!(log(f(None, Some("2000-01-01"))).await.unwrap().is_empty());
    assert_eq!(
        log(f(Some("2000-01-01T00:00:00Z"), None)).await.unwrap(),
        all
    );
    for bad in ["yesterday", "2026-13-45", "2026-10-01T10:00", "2026-1-1"] {
        assert!(
            matches!(
                log(f(Some(bad), None)).await,
                Err(audit::SearchError::BadQuery(_))
            ),
            "{bad}"
        );
    }
    let by = |by_caller: Option<bool>, refused: Option<bool>| audit::PatientLogFilter {
        by_caller,
        refused,
        ..Default::default()
    };
    assert_eq!(log(by(Some(true), None)).await.unwrap(), vec![ids[0]]);
    assert_eq!(
        log(by(Some(false), None)).await.unwrap(),
        vec![ids[2], ids[1]]
    );
    assert_eq!(log(by(None, Some(true))).await.unwrap(), vec![ids[2]]);
    assert_eq!(
        log(by(None, Some(false))).await.unwrap(),
        vec![ids[1], ids[0]]
    );
    assert_eq!(
        log(by(Some(false), Some(false))).await.unwrap(),
        vec![ids[1]]
    );
    assert!(log(by(Some(true), Some(true))).await.unwrap().is_empty());
    let purpose = |p: &str| audit::PatientLogFilter {
        purpose: Some(p.into()),
        ..Default::default()
    };
    assert_eq!(
        log(purpose("treatment")).await.unwrap(),
        vec![ids[2], ids[1]]
    );
    assert_eq!(log(purpose("patient-access")).await.unwrap(), vec![ids[0]]);
    assert!(log(purpose("research")).await.unwrap().is_empty());
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
    let page = audit::patient_log(
        &s.db,
        mine,
        "nobody",
        &audit::PatientLogFilter::default(),
        3,
    )
    .await
    .unwrap();
    assert_eq!(
        page.iter().map(|r| r.id).collect::<Vec<_>>(),
        vec![ids[4], ids[2], ids[1]],
        "newest first, other patient excluded, upper-case ref included"
    );
    let rest = audit::patient_log(
        &s.db,
        mine,
        "nobody",
        &audit::PatientLogFilter {
            before: Some(ids[1]),
            ..Default::default()
        },
        3,
    )
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

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn the_audit_search_filters_pages_newest_first_and_refuses_bad_criteria() {
    use electronic_patient_record::audit::{AuditQuery, SearchError, search};

    let chain = Chain::unkeyed();
    let s = Scratch::new().await;
    let p1 = "3b0c6f0e-2222-4222-8222-000000000001";
    let mut ids = vec![];
    for n in 0..6 {
        let mut e = event(n); // agents user-0.., actions alternate R and U
        e.patient_ref = Some(format!("Patient/{}", if n < 4 { p1 } else { "other" }));
        e.purpose_of_use = Some(if n == 2 { "emergency" } else { "treatment" }.into());
        e.break_glass = n == 2;
        e.outcome = if n == 5 { 4 } else { 0 };
        ids.push(audit::append(&s.db, &chain, &e).await.unwrap());
    }
    let ids_of = |rows: Vec<electronic_patient_record::audit::AuditRow>| -> Vec<i64> {
        rows.into_iter().map(|r| r.id).collect()
    };
    let q = |f: &dyn Fn(&mut AuditQuery)| {
        let mut q = AuditQuery::default();
        f(&mut q);
        q
    };
    // Newest first; no criteria returns everything (here, 6 rows).
    let all = search(&s.db, &AuditQuery::default()).await.unwrap();
    assert_eq!(ids_of(all), ids.iter().rev().copied().collect::<Vec<_>>());
    // One criterion at a time.
    let by_agent = search(&s.db, &q(&|q| q.agent = Some("user-1".into())))
        .await
        .unwrap();
    assert_eq!(ids_of(by_agent), vec![ids[1]]);
    let by_patient = search(&s.db, &q(&|q| q.patient = Some(p1.to_uppercase())))
        .await
        .unwrap();
    assert_eq!(ids_of(by_patient), vec![ids[3], ids[2], ids[1], ids[0]]);
    let by_action = search(&s.db, &q(&|q| q.action = Some("U".into())))
        .await
        .unwrap();
    assert_eq!(ids_of(by_action), vec![ids[5], ids[3], ids[1]]);
    let by_outcome = search(&s.db, &q(&|q| q.outcome = Some(4))).await.unwrap();
    assert_eq!(ids_of(by_outcome), vec![ids[5]]);
    let glass = search(&s.db, &q(&|q| q.break_glass = Some(true)))
        .await
        .unwrap();
    assert_eq!(ids_of(glass), vec![ids[2]]);
    let purpose = search(&s.db, &q(&|q| q.purpose = Some("emergency".into())))
        .await
        .unwrap();
    assert_eq!(ids_of(purpose), vec![ids[2]]);
    // Combined, and paged by `before`.
    let both = search(
        &s.db,
        &q(&|q| {
            q.patient = Some(p1.into());
            q.action = Some("R".into());
        }),
    )
    .await
    .unwrap();
    assert_eq!(ids_of(both), vec![ids[2], ids[0]]);
    let older = search(&s.db, &q(&|q| q.before = Some(ids[3])))
        .await
        .unwrap();
    assert_eq!(ids_of(older), vec![ids[2], ids[1], ids[0]]);
    // Dates: everything happened today; nothing before 2000, everything from 2000, none from 2999.
    assert!(
        search(&s.db, &q(&|q| q.to = Some("2000-01-01".into())))
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        search(&s.db, &q(&|q| q.from = Some("2000-01-01T00:00:00Z".into())))
            .await
            .unwrap()
            .len(),
        6
    );
    assert!(
        search(&s.db, &q(&|q| q.from = Some("2999-01-01".into())))
            .await
            .unwrap()
            .is_empty()
    );
    // A blank text criterion (an empty form field) is no criterion.
    let blank = q(&|q| {
        q.agent = Some(String::new());
        q.purpose = Some("  ".into());
        q.from = Some(String::new());
    });
    assert_eq!(search(&s.db, &blank).await.unwrap().len(), 6);
    assert!(blank.normalized().used().is_empty());
    // Bad criteria are a BadQuery, not a database error.
    for bad in [
        q(&|q| q.action = Some("X".into())),
        q(&|q| q.outcome = Some(5)),
        q(&|q| q.from = Some("yesterday".into())),
        q(&|q| q.to = Some("2026-02-30".into())),
        q(&|q| q.from = Some("2026-01-01'; DROP TABLE audit_events; --".into())),
    ] {
        assert!(
            matches!(search(&s.db, &bad).await, Err(SearchError::BadQuery(_))),
            "{bad:?}"
        );
    }
    // The search is read-only and the table is intact.
    assert_eq!(
        audit::verify(&s.db, &chain).await.unwrap(),
        Verdict::Intact { rows: 6 }
    );
    // A page is at most SEARCH_LIMIT + 1 rows: the extra one says there is more.
    for n in 6..(6 + audit::SEARCH_LIMIT as usize) {
        audit::append(&s.db, &chain, &event(n)).await.unwrap();
    }
    let page = search(&s.db, &AuditQuery::default()).await.unwrap();
    assert_eq!(page.len() as i64, audit::SEARCH_LIMIT + 1);
    s.finish().await;
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_write_and_its_audit_row_commit_or_roll_back_together() {
    use electronic_patient_record::patients_store::{self as patients, StoreError};
    use electronic_patient_record_matcher::{HumanName, Patient};
    use sea_orm::{ConnectionTrait, TransactionTrait};

    let person = |family: &str| Patient {
        names: vec![HumanName {
            family: Some(family.into()),
            given: vec!["Tx".into()],
        }],
        birth_date: Some("1970-01-01".into()),
        ..Default::default()
    };
    let chain = Chain::unkeyed();
    let s = Scratch::new().await;

    // Both land when the transaction commits.
    let tx = s.db.begin().await.unwrap();
    let made = patients::create(&tx, "a", None, &person("Both"))
        .await
        .unwrap();
    audit::append(&tx, &chain, &event(0)).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(patients::current(&s.db, &made.id).await.unwrap().version, 1);
    assert_eq!(
        audit::verify(&s.db, &chain).await.unwrap(),
        Verdict::Intact { rows: 1 }
    );

    // If the audit row cannot be written, the patient is not created.
    s.db.execute_unprepared(
        "CREATE FUNCTION refuse_test_agent() RETURNS trigger AS $$
         BEGIN IF NEW.agent = 'cannot-be-audited' THEN RAISE EXCEPTION 'audit refused'; END IF; RETURN NEW; END
         $$ LANGUAGE plpgsql",
    )
    .await
    .unwrap();
    s.db.execute_unprepared(
        "CREATE TRIGGER refuse_test_agent BEFORE INSERT ON audit_events
         FOR EACH ROW EXECUTE FUNCTION refuse_test_agent()",
    )
    .await
    .unwrap();
    let tx = s.db.begin().await.unwrap();
    let doomed = patients::create(&tx, "a", None, &person("Doomed"))
        .await
        .unwrap();
    let mut bad = event(1);
    bad.agent = "cannot-be-audited".into();
    assert!(audit::append(&tx, &chain, &bad).await.is_err());
    drop(tx); // the handler returns 500 and the transaction ends without a commit
    assert_eq!(
        patients::current(&s.db, &doomed.id).await,
        Err(StoreError::NotFound),
        "an access that cannot be audited does not happen"
    );
    assert_eq!(
        audit::verify(&s.db, &chain).await.unwrap(),
        Verdict::Intact { rows: 1 }
    );

    // A stale update inside a transaction does not poison it: the audit row can still be written.
    let tx = s.db.begin().await.unwrap();
    let v2 = patients::update(&tx, &made.id, 1, "a", None, &person("Both"))
        .await
        .unwrap();
    assert_eq!(v2.version, 2);
    assert_eq!(
        patients::update(&tx, &made.id, 1, "b", None, &person("Both")).await,
        Err(StoreError::Conflict { current: 2 })
    );
    audit::append(&tx, &chain, &event(2)).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(patients::current(&s.db, &made.id).await.unwrap().version, 2);
    assert_eq!(
        audit::verify(&s.db, &chain).await.unwrap(),
        Verdict::Intact { rows: 2 }
    );

    // A write that is rolled back leaves no audit row and no version.
    let tx = s.db.begin().await.unwrap();
    patients::update(&tx, &made.id, 2, "a", None, &person("Rolled"))
        .await
        .unwrap();
    audit::append(&tx, &chain, &event(3)).await.unwrap();
    tx.rollback().await.unwrap();
    assert_eq!(patients::current(&s.db, &made.id).await.unwrap().version, 2);
    assert_eq!(
        audit::verify(&s.db, &chain).await.unwrap(),
        Verdict::Intact { rows: 2 }
    );
    s.finish().await;
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn after_and_through_are_exact_for_instants_and_cover_whole_days_for_dates() {
    use electronic_patient_record::audit::{AuditQuery, export, search};

    let chain = Chain::unkeyed();
    let s = Scratch::new().await;
    let mut ids = vec![];
    for n in 0..5 {
        ids.push(audit::append(&s.db, &chain, &event(n)).await.unwrap());
    }
    let all = search(&s.db, &AuditQuery::default()).await.unwrap();
    let at = |id: i64| all.iter().find(|r| r.id == id).unwrap().at.clone();
    let ids_of = |rows: Vec<electronic_patient_record::audit::AuditRow>| -> Vec<i64> {
        rows.into_iter().map(|r| r.id).collect()
    };
    let q = |f: &dyn Fn(&mut AuditQuery)| {
        let mut q = AuditQuery::default();
        f(&mut q);
        q
    };
    // An instant: `through` includes the row at that instant, `after` excludes it.
    let t = at(ids[2]);
    let through = search(&s.db, &q(&|q| q.through = Some(t.clone())))
        .await
        .unwrap();
    assert_eq!(ids_of(through), vec![ids[2], ids[1], ids[0]]);
    let after = search(&s.db, &q(&|q| q.after = Some(t.clone())))
        .await
        .unwrap();
    assert_eq!(ids_of(after), vec![ids[4], ids[3]]);
    // A bare date covers the whole day: `through` today has everything, `after` today nothing,
    // `after` yesterday everything, `through` yesterday nothing.
    let today = t[..10].to_string();
    let day_before = {
        let rows =
            s.db.query_one_raw(sea_orm::Statement::from_string(
                sea_orm::DbBackend::Postgres,
                "SELECT to_char(current_date - 1, 'YYYY-MM-DD') AS d".to_string(),
            ))
            .await
            .unwrap()
            .unwrap();
        rows.try_get::<String>("", "d").unwrap()
    };
    assert_eq!(
        search(&s.db, &q(&|q| q.through = Some(today.clone())))
            .await
            .unwrap()
            .len(),
        5
    );
    assert!(
        search(&s.db, &q(&|q| q.after = Some(today.clone())))
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        search(&s.db, &q(&|q| q.after = Some(day_before.clone())))
            .await
            .unwrap()
            .len(),
        5
    );
    assert!(
        search(&s.db, &q(&|q| q.through = Some(day_before.clone())))
            .await
            .unwrap()
            .is_empty()
    );
    // Bad values are refused like the other bounds.
    assert!(
        search(&s.db, &q(&|q| q.after = Some("soon".into())))
            .await
            .is_err()
    );
    // An export carries the digests a search leaves out, and chains them.
    let exported = export(&s.db, &AuditQuery::default()).await.unwrap();
    assert_eq!(exported.len(), 5);
    assert!(exported.iter().all(|r| r.digest.is_some()));
    assert_eq!(
        exported[0].prev_digest, exported[1].digest,
        "newest first: each row names its predecessor"
    );
    assert!(exported[4].prev_digest.is_none());
    assert!(
        all.iter()
            .all(|r| r.digest.is_none() && r.prev_digest.is_none())
    );
    s.finish().await;
}
