//! DB-gated: consent versions are append-only, updates are optimistic, and only the current,
//! active consents reach a decision.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test consents_store -- --ignored --test-threads=1
//! ```

use electronic_patient_record::consent::{
    self, ConsentRecord, Context, Effect, Kind, Period, Provision, Status,
};
use electronic_patient_record::consents_store as store;
use electronic_patient_record::patients_store::{self, StoreError};
use electronic_patient_record::policy::{self, Purpose, Resource, Role};
use electronic_patient_record_matcher::{HumanName, Patient};
use electronic_patient_record_migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection};
use std::collections::BTreeSet;

async fn db() -> DatabaseConnection {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for --ignored tests");
    let db = Database::connect(url).await.expect("connect");
    Migrator::up(&db, None).await.expect("migrate");
    db
}

async fn patient(db: &DatabaseConnection) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let p = Patient {
        names: vec![HumanName {
            family: Some(format!("Zz{nanos}")),
            given: vec![],
        }],
        ..Default::default()
    };
    patients_store::create(db, "t", None, &p).await.unwrap().id
}

fn deny_research() -> ConsentRecord {
    ConsentRecord {
        kind: Kind::General,
        status: Status::Active,
        provisions: vec![Provision {
            effect: Effect::Deny,
            actors: vec![],
            purposes: vec!["research".into()],
            resources: vec![],
            labels: vec![],
            period: None,
        }],
        period: None,
    }
}

fn ctx(purpose: Purpose) -> Context {
    Context {
        roles: BTreeSet::from([Role::Clinician]),
        purpose,
        resource: Resource::PatientIdentity,
        labels: BTreeSet::new(),
        now: 1_790_000_000,
    }
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_consent_needs_an_existing_patient_and_starts_at_version_one() {
    let db = db().await;
    let missing = store::create(
        &db,
        "00000000-0000-4000-8000-000000000000",
        "a",
        None,
        &deny_research(),
    )
    .await;
    assert_eq!(missing, Err(StoreError::NotFound));
    assert_eq!(
        store::create(&db, "not-a-uuid", "a", None, &deny_research()).await,
        Err(StoreError::NotFound)
    );

    let pid = patient(&db).await;
    let c = store::create(&db, &pid, "admin", Some("on the form"), &deny_research())
        .await
        .unwrap();
    assert_eq!((c.version, c.patient_id.as_str()), (1, pid.as_str()));
    assert_eq!(c.consent, deny_research());
    assert_eq!(store::current(&db, &pid, &c.id).await.unwrap(), c);
    // The same consent id under another patient is not found.
    let other = patient(&db).await;
    assert_eq!(
        store::current(&db, &other, &c.id).await,
        Err(StoreError::NotFound)
    );
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn withdrawing_appends_a_version_and_keeps_the_old_one() {
    let db = db().await;
    let pid = patient(&db).await;
    let v1 = store::create(&db, &pid, "admin", None, &deny_research())
        .await
        .unwrap();
    let mut withdrawn = deny_research();
    withdrawn.status = Status::Inactive;
    let v2 = store::update(
        &db,
        &pid,
        &v1.id,
        1,
        "admin2",
        Some("withdrawn"),
        &withdrawn,
    )
    .await
    .unwrap();
    assert_eq!((v2.version, v2.author.as_str()), (2, "admin2"));

    // Both versions are in the table; the list and the decision see only the current one.
    let rows = db
        .query_all_raw(sea_orm::Statement::from_sql_and_values(
            sea_orm::DbBackend::Postgres,
            "SELECT version, status, reason FROM consent_versions WHERE consent_id = $1::uuid ORDER BY version",
            [v1.id.clone().into()],
        ))
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].try_get::<String>("", "status").unwrap(), "active");
    assert_eq!(rows[1].try_get::<String>("", "status").unwrap(), "inactive");
    let listed = store::list(&db, &pid).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].version, 2);
    assert!(
        store::active_for(&db, std::slice::from_ref(&pid))
            .await
            .unwrap()
            .is_empty(),
        "a withdrawn consent is not active"
    );
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_stale_or_racing_update_is_a_conflict_and_changes_nothing() {
    let db = db().await;
    let pid = patient(&db).await;
    let v1 = store::create(&db, &pid, "a", None, &deny_research())
        .await
        .unwrap();
    store::update(&db, &pid, &v1.id, 1, "a", None, &deny_research())
        .await
        .unwrap();
    assert_eq!(
        store::update(&db, &pid, &v1.id, 1, "b", None, &deny_research()).await,
        Err(StoreError::Conflict { current: 2 })
    );
    let (r1, r2) = (deny_research(), deny_research());
    let (a, b) = tokio::join!(
        store::update(&db, &pid, &v1.id, 2, "a", None, &r1),
        store::update(&db, &pid, &v1.id, 2, "b", None, &r2),
    );
    assert_eq!(
        [a.is_ok(), b.is_ok()].iter().filter(|ok| **ok).count(),
        1,
        "exactly one writer wins: {a:?} / {b:?}"
    );
    assert_eq!(store::current(&db, &pid, &v1.id).await.unwrap().version, 3);
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn versions_cannot_be_changed_or_removed() {
    let db = db().await;
    let pid = patient(&db).await;
    let c = store::create(&db, &pid, "a", None, &deny_research())
        .await
        .unwrap();
    for sql in [
        format!(
            "UPDATE consent_versions SET author = 'x' WHERE consent_id = '{}'",
            c.id
        ),
        format!("DELETE FROM consent_versions WHERE consent_id = '{}'", c.id),
        "TRUNCATE consent_versions".to_string(),
    ] {
        let e = db.execute_unprepared(&sql).await.unwrap_err().to_string();
        assert!(e.contains("append-only"), "{sql}: {e}");
    }
    // The status column only takes the three states.
    let bad = db
        .execute_unprepared(&format!(
            "INSERT INTO consent_versions (consent_id, version, patient_id, author, status, body)
             VALUES (gen_random_uuid(), 1, '{pid}', 'x', 'maybe', '{{}}')"
        ))
        .await;
    assert!(bad.is_err());
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn the_stored_consents_drive_the_decision_and_other_patients_are_untouched() {
    let db = db().await;
    let (a, b, none) = (patient(&db).await, patient(&db).await, patient(&db).await);
    let on_a = store::create(&db, &a, "admin", None, &deny_research())
        .await
        .unwrap();
    // B has a consent that only applies in the past.
    let mut expired = deny_research();
    expired.period = Some(Period {
        start: None,
        end: Some("2020-01-01".into()),
    });
    store::create(&db, &b, "admin", None, &expired)
        .await
        .unwrap();

    let ids = vec![a.clone(), b.clone(), none.clone()];
    let active = store::active_for(&db, &ids).await.unwrap();
    assert!(!active.contains_key(&none));
    let verdict = |pid: &str, purpose| {
        let records = active.get(pid).cloned().unwrap_or_default();
        consent::decide(
            records.iter().map(|c| (c.id.as_str(), &c.consent)),
            &ctx(purpose),
        )
    };
    let v = verdict(&a, Purpose::Research);
    assert_eq!(v.consent, policy::Consent::Deny);
    assert_eq!(v.decided_by, vec![on_a.id.clone()]);
    assert_eq!(
        verdict(&a, Purpose::Treatment).consent,
        policy::Consent::NotRecorded
    );
    assert_eq!(
        verdict(&b, Purpose::Research).consent,
        policy::Consent::NotRecorded
    );
    assert_eq!(
        verdict(&none, Purpose::Research).consent,
        policy::Consent::NotRecorded
    );
    // No ids, no query.
    assert!(store::active_for(&db, &[]).await.unwrap().is_empty());
}

/// A person the matcher will agree may be merged with another made with the same family name. The
/// birth date comes from the name, so these people do not crowd other tests' candidate searches.
async fn alike(db: &DatabaseConnection, family: &str) -> String {
    let n = family
        .bytes()
        .fold(0u32, |h, b| h.wrapping_mul(31).wrapping_add(u32::from(b)));
    let p = Patient {
        names: vec![HumanName {
            family: Some(family.into()),
            given: vec!["Sam".into()],
        }],
        birth_date: Some(format!(
            "{}-{:02}-{:02}",
            1900 + n % 100,
            1 + n / 100 % 12,
            1 + n / 1200 % 28
        )),
        ..Default::default()
    };
    patients_store::create(db, "t", None, &p).await.unwrap().id
}

async fn merge(db: &DatabaseConnection, target: &str, source: &str) {
    use sea_orm::TransactionTrait;
    let tx = db.begin().await.unwrap();
    electronic_patient_record::links::merge(&tx, target, source, "admin", "same person")
        .await
        .unwrap();
    tx.commit().await.unwrap();
}

/// The way the duplicate check and search read consent before CQ-10: one patient at a time.
async fn one_at_a_time(
    db: &DatabaseConnection,
    ids: &[String],
) -> std::collections::BTreeMap<String, Vec<String>> {
    let ids: Vec<String> = ids.iter().map(|i| i.to_lowercase()).collect();
    let mut active = store::active_for(db, &ids).await.unwrap();
    for id in &ids {
        let sources = electronic_patient_record::links::sources_of(db, id)
            .await
            .unwrap();
        if sources.is_empty() {
            continue;
        }
        let inherited = store::active_for(db, &sources).await.unwrap();
        active
            .entry(id.clone())
            .or_default()
            .extend(inherited.into_values().flatten());
    }
    shape(active)
}

fn shape(
    m: std::collections::HashMap<String, Vec<store::StoredConsent>>,
) -> std::collections::BTreeMap<String, Vec<String>> {
    m.into_iter()
        .map(|(k, v)| {
            let mut ids: Vec<String> = v.into_iter().map(|c| c.id).collect();
            ids.sort();
            (k, ids)
        })
        .collect()
}

/// CQ-10: consents for many patients, merged-in records included, in a fixed number of queries,
/// with the same answer as one patient at a time.
#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn consents_for_many_patients_match_one_at_a_time() {
    let db = db().await;
    let family = format!(
        "Zzbatch{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    // C merged into B, B into A: A carries the refusals of A and C.
    let (a, b, c) = (
        alike(&db, &family).await,
        alike(&db, &family).await,
        alike(&db, &family).await,
    );
    merge(&db, &b, &c).await;
    merge(&db, &a, &b).await;
    for id in [&a, &c] {
        store::create(&db, id, "t", None, &deny_research())
            .await
            .unwrap();
    }
    // D has its own refusal; E has nothing; F has a merged-in G and no consent anywhere.
    let d = patient(&db).await;
    store::create(&db, &d, "t", None, &deny_research())
        .await
        .unwrap();
    let e = patient(&db).await;
    let (f, g) = (
        alike(&db, &format!("{family}f")).await,
        alike(&db, &format!("{family}f")).await,
    );
    merge(&db, &f, &g).await;

    let ids = vec![
        a.to_uppercase(),
        d.clone(),
        e.clone(),
        f.clone(),
        "not-a-uuid".into(),
    ];
    let batched = shape(store::active_with_merged(&db, &ids).await.unwrap());
    assert_eq!(batched, one_at_a_time(&db, &ids).await);
    assert_eq!(batched[&a.to_lowercase()].len(), 2, "{batched:?}");
    assert_eq!(batched[&d].len(), 1);
    assert!(!batched.contains_key(&e));
    assert_eq!(
        batched.get(&f),
        Some(&Vec::new()),
        "a merged-in source keeps an entry"
    );
}

/// CQ-12: as for patients, a lost race for a consent version is a conflict, not a database error.
#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_lost_race_for_a_consent_version_is_a_conflict() {
    use sea_orm::TransactionTrait;
    let db = db().await;
    let pid = patient(&db).await;
    let c = store::create(&db, &pid, "t", None, &deny_research())
        .await
        .unwrap();
    let first = db.begin().await.unwrap();
    store::update(&first, &pid, &c.id, 1, "first", None, &deny_research())
        .await
        .unwrap();
    let second = {
        let (db, pid, cid) = (db.clone(), pid.clone(), c.id.clone());
        tokio::spawn(async move {
            store::update(&db, &pid, &cid, 1, "second", None, &deny_research()).await
        })
    };
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    first.commit().await.unwrap();
    assert!(
        matches!(
            second.await.unwrap(),
            Err(StoreError::Conflict { current: 2 })
        ),
        "a lost race is a conflict"
    );
}
