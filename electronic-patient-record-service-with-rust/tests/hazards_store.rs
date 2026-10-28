//! DB-gated (AS-17): the hazard register is append-only versions; the rules of `hazard.rs` hold through the
//! store; a retired hazard keeps its whole history; and the database itself refuses to edit, delete or
//! truncate a version, so retiring cannot erase anything.
//!
//! ```text
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test hazards_store -- --ignored --test-threads=1
//! ```

use chrono::Utc;
use electronic_patient_record::hazard::{Actor, Event, Evidence, EvidenceKind, State};
use electronic_patient_record::hazard_store as store;
use electronic_patient_record::patients_store::StoreError;
use electronic_patient_record_migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, Statement};

async fn db() -> DatabaseConnection {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for --ignored tests");
    let db = Database::connect(url).await.expect("connect");
    Migrator::up(&db, None).await.expect("migrate");
    db
}

fn evidence() -> Evidence {
    Evidence {
        kind: EvidenceKind::TestRun,
        reference: "tests/example.rs".into(),
        taken_at: Utc::now() - chrono::Duration::hours(1),
        sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".into(),
    }
}

fn retire() -> Event {
    Event::Retire {
        evidence: vec![evidence()],
        note: "control verified".into(),
    }
}

const CSO: Actor<'static> = Actor {
    id: "cso-1",
    is_cso: true,
};
const CLINICIAN: Actor<'static> = Actor {
    id: "dr-1",
    is_cso: false,
};

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_hazard_is_retired_only_by_the_cso_with_evidence_and_keeps_its_history() {
    let db = db().await;
    let now = Utc::now();
    let h = store::raise(&db, "dr-1", "Wrong patient overlay", 5, 2, now)
        .await
        .unwrap();
    assert_eq!((h.version, h.hazard.state), (1, State::Open));

    // A clinician records a control; only an open hazard takes one.
    let c = store::advance(
        &db,
        &h.id,
        1,
        &CLINICIAN,
        Event::Control {
            text: "second check at sign".into(),
        },
        now,
    )
    .await
    .unwrap();
    assert_eq!(c.version, 2);

    // The clinician cannot retire it; the Clinical Safety Officer cannot do so without evidence.
    let refused = store::advance(&db, &h.id, 2, &CLINICIAN, retire(), now).await;
    assert!(
        matches!(refused, Err(StoreError::BadQuery(m)) if m.contains("Clinical Safety Officer"))
    );
    let none = Event::Retire {
        evidence: vec![],
        note: "n".into(),
    };
    let refused = store::advance(&db, &h.id, 2, &CSO, none, now).await;
    assert!(matches!(refused, Err(StoreError::BadQuery(m)) if m.contains("without evidence")));
    assert_eq!(store::current(&db, &h.id).await.unwrap().version, 2);

    // A stale version is a conflict, not a lost update.
    let stale = store::advance(&db, &h.id, 1, &CSO, retire(), now).await;
    assert!(matches!(stale, Err(StoreError::Conflict { current: 2 })));

    // The Clinical Safety Officer retires it with evidence.
    let done = store::advance(&db, &h.id, 2, &CSO, retire(), now)
        .await
        .unwrap();
    assert_eq!(done.hazard.state, State::Retired);
    assert_eq!(done.hazard.retired_by.as_deref(), Some("cso-1"));
    assert_eq!(done.hazard.evidence.len(), 1);

    // Anyone reopens it; the retirement is still in the versions.
    let back = store::advance(
        &db,
        &h.id,
        3,
        &CLINICIAN,
        Event::Reopen {
            reason: "seen again".into(),
        },
        now,
    )
    .await
    .unwrap();
    assert_eq!((back.version, back.hazard.state), (4, State::Open));
    assert_eq!(back.hazard.reopened, 1);
    let kept = db
        .query_all_raw(Statement::from_sql_and_values(
            sea_orm::DbBackend::Postgres,
            "SELECT state FROM hazard_versions WHERE hazard_id = $1::uuid ORDER BY version",
            [h.id.clone().into()],
        ))
        .await
        .unwrap();
    let states: Vec<String> = kept
        .iter()
        .map(|r| r.try_get("", "state").unwrap())
        .collect();
    assert_eq!(states, ["open", "open", "retired", "open"]);
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn the_database_refuses_to_edit_delete_or_truncate_a_version() {
    let db = db().await;
    let h = store::raise(&db, "dr-1", "Append only", 3, 3, Utc::now())
        .await
        .unwrap();
    for sql in [
        format!(
            "UPDATE hazard_versions SET author = 'x' WHERE hazard_id = '{}'",
            h.id
        ),
        format!("DELETE FROM hazard_versions WHERE hazard_id = '{}'", h.id),
        "TRUNCATE hazard_versions".to_string(),
    ] {
        let err = db.execute_unprepared(&sql).await.unwrap_err().to_string();
        assert!(err.contains("append-only"), "{sql}: {err}");
    }
    assert_eq!(
        store::current(&db, &h.id).await.unwrap().hazard.title,
        "Append only"
    );
}
