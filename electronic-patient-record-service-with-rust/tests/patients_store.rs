//! DB-gated: patient versions are append-only, updates are optimistic, and search finds current
//! versions only.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test patients_store -- --ignored --test-threads=1
//! ```

use electronic_patient_record::patients_store::{self as store, Search, StoreError};
use electronic_patient_record_matcher::{HumanName, Identifier, Patient};
use electronic_patient_record_migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection};

async fn db() -> DatabaseConnection {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for --ignored tests");
    let db = Database::connect(url).await.expect("connect");
    Migrator::up(&db, None).await.expect("migrate");
    db
}

/// A family name no other test uses, so searches see only this test's rows.
fn tag(label: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("Zz{label}{nanos}")
}

fn patient(family: &str, dob: &str) -> Patient {
    Patient {
        names: vec![HumanName {
            family: Some(family.into()),
            given: vec!["Test".into()],
        }],
        birth_date: Some(dob.into()),
        ..Default::default()
    }
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn create_assigns_an_id_and_version_one_and_ignores_a_supplied_id() {
    let db = db().await;
    let mut p = patient(&tag("create"), "1980-03-14");
    p.id = Some("client-chosen".into());
    let s = store::create(&db, "tester", Some("registration"), &p)
        .await
        .unwrap();
    assert_eq!(s.version, 1);
    assert_ne!(s.id, "client-chosen");
    assert_eq!(s.patient.id.as_deref(), Some(s.id.as_str()));
    assert_eq!(store::current(&db, &s.id).await.unwrap(), s);
    assert_eq!(
        store::current(&db, "not-a-uuid").await,
        Err(StoreError::NotFound)
    );
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn an_update_appends_a_version_and_keeps_the_old_one() {
    let db = db().await;
    let family = tag("update");
    let v1 = store::create(&db, "alice", None, &patient(&family, "1980-03-14"))
        .await
        .unwrap();
    let v2 = store::update(
        &db,
        &v1.id,
        1,
        "bob",
        Some("corrected birth date"),
        &patient(&family, "1980-03-15"),
    )
    .await
    .unwrap();
    assert_eq!(v2.version, 2);
    assert_eq!(v2.author, "bob");
    assert_eq!(v2.patient.birth_date.as_deref(), Some("1980-03-15"));
    // The current view shows v2; both versions are still in the table with their authors.
    assert_eq!(store::current(&db, &v1.id).await.unwrap().version, 2);
    let rows = db
        .query_all_raw(sea_orm::Statement::from_sql_and_values(
            sea_orm::DbBackend::Postgres,
            "SELECT version, author, reason FROM patient_versions WHERE patient_id = $1::uuid ORDER BY version",
            [v1.id.clone().into()],
        ))
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].try_get::<String>("", "author").unwrap(), "alice");
    assert_eq!(
        rows[1]
            .try_get::<Option<String>>("", "reason")
            .unwrap()
            .as_deref(),
        Some("corrected birth date")
    );
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_stale_update_is_a_conflict_and_changes_nothing() {
    let db = db().await;
    let family = tag("stale");
    let v1 = store::create(&db, "a", None, &patient(&family, "1980"))
        .await
        .unwrap();
    store::update(&db, &v1.id, 1, "a", None, &patient(&family, "1981"))
        .await
        .unwrap();
    let late = store::update(&db, &v1.id, 1, "b", None, &patient(&family, "1999")).await;
    assert_eq!(late, Err(StoreError::Conflict { current: 2 }));
    assert_eq!(
        store::current(&db, &v1.id)
            .await
            .unwrap()
            .patient
            .birth_date
            .as_deref(),
        Some("1981")
    );
    assert_eq!(
        store::update(
            &db,
            "00000000-0000-4000-8000-000000000000",
            1,
            "a",
            None,
            &patient(&family, "1980")
        )
        .await,
        Err(StoreError::NotFound)
    );
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn two_writers_racing_for_the_same_version_get_one_winner() {
    let db = db().await;
    let family = tag("race");
    let v1 = store::create(&db, "a", None, &patient(&family, "1980"))
        .await
        .unwrap();
    let (pa, pb) = (patient(&family, "1981"), patient(&family, "1982"));
    let (a, b) = tokio::join!(
        store::update(&db, &v1.id, 1, "a", None, &pa),
        store::update(&db, &v1.id, 1, "b", None, &pb),
    );
    assert_eq!(
        [a.is_ok(), b.is_ok()].iter().filter(|ok| **ok).count(),
        1,
        "exactly one writer wins: {a:?} / {b:?}"
    );
    assert_eq!(store::current(&db, &v1.id).await.unwrap().version, 2);
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn versions_cannot_be_changed_or_removed() {
    let db = db().await;
    let s = store::create(&db, "a", None, &patient(&tag("immutable"), "1980"))
        .await
        .unwrap();
    for sql in [
        format!(
            "UPDATE patient_versions SET author = 'x' WHERE patient_id = '{}'",
            s.id
        ),
        format!("DELETE FROM patient_versions WHERE patient_id = '{}'", s.id),
        "TRUNCATE patient_versions".to_string(),
    ] {
        let e = db.execute_unprepared(&sql).await.unwrap_err().to_string();
        assert!(e.contains("append-only"), "{sql}: {e}");
    }
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn search_finds_current_versions_by_name_prefix_birth_date_and_identifier() {
    let db = db().await;
    let family = tag("search");
    let mut p = patient(&family, "1975-06-01");
    p.identifiers = vec![Identifier {
        system: "urn:example:mrn".into(),
        value: family.clone(),
    }];
    let v1 = store::create(&db, "a", None, &p).await.unwrap();
    // Change the name: the old name must no longer find the patient.
    let renamed = tag("renamed");
    let mut p2 = p.clone();
    p2.names[0].family = Some(renamed.clone());
    store::update(&db, &v1.id, 1, "a", None, &p2).await.unwrap();

    let by = |family: Option<&str>, dob: Option<&str>, ident: Option<String>| Search {
        family: family.map(Into::into),
        birth_date: dob.map(Into::into),
        identifier: ident,
    };
    assert!(
        store::search(&db, &by(Some(&family), None, None))
            .await
            .unwrap()
            .is_empty()
    );
    let hit = store::search(
        &db,
        &by(
            Some(&renamed[..renamed.len() - 3].to_lowercase()),
            Some("1975-06-01"),
            None,
        ),
    )
    .await
    .unwrap();
    assert_eq!(hit.len(), 1, "prefix, case-insensitive, with birth date");
    assert_eq!(hit[0].id, v1.id);
    assert_eq!(hit[0].version, 2);
    let by_ident = store::search(
        &db,
        &by(None, None, Some(format!("urn:example:mrn|{family}"))),
    )
    .await
    .unwrap();
    assert_eq!(by_ident.len(), 1);
    assert!(
        store::search(&db, &by(None, None, Some("urn:example:mrn|nobody".into())))
            .await
            .unwrap()
            .is_empty()
    );
    assert!(matches!(
        store::search(&db, &by(None, None, Some("no-bar".into()))).await,
        Err(StoreError::BadQuery(_))
    ));
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn wildcards_in_a_search_are_literal_and_results_are_capped() {
    let db = db().await;
    let base = tag("many");
    for i in 0..55 {
        store::create(&db, "a", None, &patient(&format!("{base}{i:02}"), "1990"))
            .await
            .unwrap();
    }
    let q = Search {
        family: Some(base.clone()),
        ..Default::default()
    };
    assert_eq!(store::search(&db, &q).await.unwrap().len(), 50);
    // "%" must not match everything.
    let q = Search {
        family: Some("%".into()),
        ..Default::default()
    };
    assert!(store::search(&db, &q).await.unwrap().is_empty());
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn candidates_share_an_identifier_a_birth_date_or_a_family_name() {
    let db = db().await;
    let family = tag("cand");
    let mut p = patient(&family, "1969-02-03");
    p.identifiers = vec![Identifier {
        system: "urn:example:mrn".into(),
        value: family.clone(),
    }];
    let stored = store::create(&db, "a", None, &p).await.unwrap();

    let ids = |c: Vec<store::Stored>| c.into_iter().map(|s| s.id).collect::<Vec<_>>();
    // Same family name, nothing else (case differs).
    let q = patient(&family.to_uppercase(), "2001-01-01");
    assert!(ids(store::candidates(&db, &q).await.unwrap()).contains(&stored.id));
    // Same identifier only.
    let mut q = patient("Unrelated", "2001-01-01");
    q.identifiers = p.identifiers.clone();
    assert!(ids(store::candidates(&db, &q).await.unwrap()).contains(&stored.id));
    // Same birth date only.
    let q = patient("Unrelated", "1969-02-03");
    assert!(ids(store::candidates(&db, &q).await.unwrap()).contains(&stored.id));
    // Nothing in common: not a candidate (the documented blind spot).
    let q = patient("Unrelated", "2001-01-01");
    assert!(!ids(store::candidates(&db, &q).await.unwrap()).contains(&stored.id));
    // An old version's name does not make a candidate.
    let renamed = tag("candnew");
    let mut p2 = p.clone();
    p2.names[0].family = Some(renamed);
    p2.birth_date = Some("1970-01-01".into());
    p2.identifiers = vec![];
    store::update(&db, &stored.id, 1, "a", None, &p2)
        .await
        .unwrap();
    let q = patient(&family, "2001-01-01");
    assert!(!ids(store::candidates(&db, &q).await.unwrap()).contains(&stored.id));
}

/// CQ-7: two registrations of the same person at once. Each takes the registration lock, looks for
/// candidates on its own transaction and creates only if there are none, as the handler and the HL7
/// feed do. Without the lock both see nothing and both create.
#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn concurrent_registrations_of_one_person_create_one_record() {
    use sea_orm::TransactionTrait;
    let db = db().await;
    let family = tag("race");
    let person = patient(&family, "1980-03-14");
    // Candidates also match on birth date, which earlier tests share: count only this test's rows.
    let ours = |rows: Vec<store::Stored>, family: &str| {
        rows.into_iter()
            .filter(|r| r.patient.names[0].family.as_deref() == Some(family))
            .count()
    };
    let attempt = |db: DatabaseConnection, p: Patient, family: String| async move {
        let tx = db.begin().await.unwrap();
        store::lock_registration(&tx).await.unwrap();
        let found = ours(store::candidates(&tx, &p).await.unwrap(), &family);
        // Widen the window: without the lock the other attempt searches now and sees nothing.
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        let created = if found == 0 {
            store::create(&tx, "tester", Some("race"), &p)
                .await
                .unwrap();
            true
        } else {
            false
        };
        tx.commit().await.unwrap();
        created
    };
    let (a, b) = tokio::join!(
        attempt(db.clone(), person.clone(), family.clone()),
        attempt(db.clone(), person.clone(), family.clone())
    );
    assert!(a ^ b, "exactly one attempt creates: {a} {b}");
    assert_eq!(
        ours(store::candidates(&db, &person).await.unwrap(), &family),
        1
    );
}

/// CQ-9: an id in upper case is found, a malformed one is not, and the lookup seeks the index.
#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn ids_are_found_in_any_case_and_looked_up_by_index() {
    let db = db().await;
    let s = store::create(&db, "tester", None, &patient(&tag("case"), "1980-03-14"))
        .await
        .unwrap();
    assert_eq!(store::current(&db, &s.id.to_uppercase()).await.unwrap(), s);
    for bad in ["not-a-uuid", "", "44839249b00a4b9ab092675a7a42e324"] {
        assert_eq!(
            store::current(&db, bad).await,
            Err(StoreError::NotFound),
            "{bad:?}"
        );
    }
    // The shape every id lookup now has: the plan must seek on the id, not filter a whole scan. A
    // small test table would be scanned anyway, so sequential scans are off for this transaction.
    use sea_orm::TransactionTrait;
    let tx = db.begin().await.unwrap();
    tx.execute_unprepared("SET LOCAL enable_seqscan = off")
        .await
        .unwrap();
    let plan: Vec<String> = tx
        .query_all_raw(sea_orm::Statement::from_sql_and_values(
            sea_orm::DbBackend::Postgres,
            "EXPLAIN SELECT * FROM patient_current WHERE patient_id = $1::uuid",
            [s.id.clone().into()],
        ))
        .await
        .unwrap()
        .iter()
        .map(|r| r.try_get_by_index::<String>(0).unwrap())
        .collect();
    assert!(
        plan.iter().any(|l| l.contains("Index Cond: (patient_id =")),
        "{plan:#?}"
    );
}

/// A valid, non-placeholder NHS number from `seed`.
fn nhs(seed: u64) -> String {
    let mut s = seed;
    loop {
        let digits: Vec<u32> = format!("{:09}", s % 1_000_000_000)
            .bytes()
            .map(|b| u32::from(b - b'0'))
            .collect();
        let sum: u32 = digits
            .iter()
            .enumerate()
            .map(|(i, d)| d * (10 - i as u32))
            .sum();
        let check = match 11 - sum % 11 {
            11 => 0,
            10 => {
                s += 1;
                continue;
            }
            n => n,
        };
        let n = format!(
            "{}{check}",
            digits.iter().map(u32::to_string).collect::<String>()
        );
        if !electronic_patient_record_matcher::is_placeholder_nhs_number(&n) {
            return n;
        }
        s += 1;
    }
}

/// CQ-36: with more than the candidate limit sharing a birth date, the strongest candidates (a
/// shared identifier; the same family name and birth date) are still found, and the cut is reported.
#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn the_strongest_candidates_survive_a_crowded_birth_date() {
    let db = db().await;
    let nanos = u64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            % 1_000_000_000_000,
    )
    .unwrap();
    // A birth date nobody else uses, shared by more people than the limit.
    let dob = format!(
        "18{:02}-{:02}-{:02}",
        nanos % 100,
        1 + nanos / 100 % 12,
        1 + nanos / 1200 % 28
    );
    let crowd = usize::try_from(store::CANDIDATE_LIMIT).unwrap() + 50;
    for i in 0..crowd {
        store::create(&db, "t", None, &patient(&tag(&format!("crowd{i}")), &dob))
            .await
            .unwrap();
    }
    let family = tag("same");
    let number = nhs(nanos);
    let mut by_number = patient(&tag("other"), "1950-01-01");
    by_number.identifiers.push(Identifier {
        system: electronic_patient_record_matcher::NHS_NUMBER_SYSTEM.into(),
        value: number.clone(),
    });
    let by_number = store::create(&db, "t", None, &by_number).await.unwrap();
    let by_name = store::create(&db, "t", None, &patient(&family, &dob))
        .await
        .unwrap();

    let mut probe = patient(&family, &dob);
    probe.identifiers.push(Identifier {
        system: electronic_patient_record_matcher::NHS_NUMBER_SYSTEM.into(),
        value: number,
    });
    let (found, cut) = store::candidates_ranked(&db, &probe).await.unwrap();
    let ids: Vec<&str> = found.iter().map(|s| s.id.as_str()).collect();
    assert!(cut, "more than the limit matched");
    assert_eq!(
        found.len(),
        usize::try_from(store::CANDIDATE_LIMIT).unwrap()
    );
    assert_eq!(ids[0], by_number.id, "a shared identifier ranks first");
    assert_eq!(
        ids[1], by_name.id,
        "then family name and birth date together"
    );
}

/// CQ-12: two writers pass the version check at once; the primary key refuses the second, which
/// must come back as a version conflict naming the version now current, not a database error.
#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_lost_race_for_a_version_is_a_conflict() {
    use sea_orm::TransactionTrait;
    let db = db().await;
    let p = patient(&tag("race2"), "1980-03-14");
    let s = store::create(&db, "t", None, &p).await.unwrap();
    // The first writer has version 2 written but not committed.
    let first = db.begin().await.unwrap();
    store::update(&first, &s.id, 1, "first", None, &p)
        .await
        .unwrap();
    // The second still sees version 1, and its insert of version 2 waits on the first.
    let second = {
        let (db, id, p) = (db.clone(), s.id.clone(), p.clone());
        tokio::spawn(async move { store::update(&db, &id, 1, "second", None, &p).await })
    };
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    first.commit().await.unwrap();
    assert_eq!(
        second.await.unwrap(),
        Err(StoreError::Conflict { current: 2 })
    );
}
