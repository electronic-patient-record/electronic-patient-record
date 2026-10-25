//! DB-gated: merge and unmerge (task ID-5). A merge is a link and nothing else moves, so an unmerge
//! restores exactly what was there; the property test checks that against a model over random
//! sequences of adding allergies, merging and unmerging.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test links_store -- --ignored --test-threads=1
//! ```

use electronic_patient_record::allergies::{self, AllergyRecord, Status, Summary};
use electronic_patient_record::links;
use electronic_patient_record::patients_store::{self, Search, StoreError};
use electronic_patient_record_matcher::{HumanName, Patient};
use electronic_patient_record_migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, TransactionTrait};
use std::collections::{BTreeSet, HashMap};

async fn db() -> DatabaseConnection {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for --ignored tests");
    let db = Database::connect(url).await.expect("connect");
    Migrator::up(&db, None).await.expect("migrate");
    db
}

fn tag(label: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("Zz{label}{nanos}")
}

/// The same person every time (so the matcher agrees to merge), under a family name unique to the
/// test so searches see only this test's rows.
async fn person(db: &DatabaseConnection, family: &str) -> String {
    let p = Patient {
        names: vec![HumanName {
            family: Some(family.into()),
            given: vec!["Sam".into()],
        }],
        birth_date: Some("1975-06-01".into()),
        ..Default::default()
    };
    patients_store::create(db, "t", None, &p).await.unwrap().id
}

fn allergy(substance: &str) -> AllergyRecord {
    AllergyRecord {
        substance: substance.into(),
        coding: None,
        category: None,
        criticality: None,
        reaction: None,
        onset: None,
        status: Status::Active,
    }
}

async fn merge(
    db: &DatabaseConnection,
    target: &str,
    source: &str,
) -> Result<links::StoredLink, StoreError> {
    let tx = db.begin().await.unwrap();
    let r = links::merge(&tx, target, source, "admin", "same person").await;
    if r.is_ok() {
        tx.commit().await.unwrap();
    }
    r
}

async fn unmerge(
    db: &DatabaseConnection,
    target: &str,
    source: &str,
) -> Result<links::StoredLink, StoreError> {
    let tx = db.begin().await.unwrap();
    let r = links::unmerge(&tx, target, source, "admin", "mistake").await;
    if r.is_ok() {
        tx.commit().await.unwrap();
    }
    r
}

async fn ids_of(db: &DatabaseConnection, patient: &str) -> BTreeSet<String> {
    allergies::list(db, patient)
        .await
        .unwrap()
        .into_iter()
        .map(|a| a.id)
        .collect()
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_merge_hides_the_old_record_from_search_and_an_unmerge_brings_it_back() {
    let db = db().await;
    let family = tag("merge");
    let (target, source) = (person(&db, &family).await, person(&db, &family).await);
    let search = || Search {
        family: Some(family.clone()),
        ..Default::default()
    };
    assert_eq!(
        patients_store::search(&db, &search()).await.unwrap().len(),
        2
    );

    let link = merge(&db, &target, &source).await.unwrap();
    assert_eq!((link.version, link.status.as_str()), (1, "active"));
    assert_eq!(
        (link.source.as_str(), link.target.as_str()),
        (source.as_str(), target.as_str())
    );
    assert!(link.match_decision.is_some() && link.match_score.is_some());
    assert_eq!(
        links::merged_into(&db, &source).await.unwrap().as_deref(),
        Some(target.as_str())
    );
    assert_eq!(links::merged_into(&db, &target).await.unwrap(), None);
    assert_eq!(
        links::sources_of(&db, &target).await.unwrap(),
        vec![source.clone()]
    );
    // The old record is not a live patient: not in search, not a duplicate candidate; its own row is unchanged.
    let found = patients_store::search(&db, &search()).await.unwrap();
    assert_eq!(
        found.iter().map(|s| s.id.clone()).collect::<Vec<_>>(),
        vec![target.clone()]
    );
    let probe = Patient {
        names: vec![HumanName {
            family: Some(family.clone()),
            given: vec!["Sam".into()],
        }],
        birth_date: Some("1975-06-01".into()),
        ..Default::default()
    };
    let candidates = patients_store::candidates(&db, &probe).await.unwrap();
    assert!(candidates.iter().all(|c| c.id != source) && candidates.iter().any(|c| c.id == target));
    assert_eq!(
        patients_store::current(&db, &source).await.unwrap().version,
        1
    );
    assert_eq!(links::list(&db, &source).await.unwrap().len(), 1);
    assert_eq!(links::list(&db, &target).await.unwrap().len(), 1);

    let undone = unmerge(&db, &target, &source).await.unwrap();
    assert_eq!(
        (undone.id.as_str(), undone.version, undone.status.as_str()),
        (link.id.as_str(), 2, "reversed")
    );
    assert_eq!(links::merged_into(&db, &source).await.unwrap(), None);
    assert!(links::sources_of(&db, &target).await.unwrap().is_empty());
    assert_eq!(
        patients_store::search(&db, &search()).await.unwrap().len(),
        2
    );
    // Both versions of the link remain, with authors and reasons.
    let rows = db
        .query_all_raw(sea_orm::Statement::from_sql_and_values(
            sea_orm::DbBackend::Postgres,
            "SELECT version, status, reason FROM patient_link_versions WHERE link_id = $1::uuid ORDER BY version",
            [link.id.clone().into()],
        ))
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1].try_get::<String>("", "reason").unwrap(), "mistake");
    // Merging again after an unmerge is a new link.
    let again = merge(&db, &target, &source).await.unwrap();
    assert_ne!(again.id, link.id);
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn bad_merges_are_refused_with_a_reason() {
    let db = db().await;
    let family = tag("refuse");
    let (a, b, c) = (
        person(&db, &family).await,
        person(&db, &family).await,
        person(&db, &family).await,
    );
    let msg = |e: Result<links::StoredLink, StoreError>| match e {
        Err(StoreError::BadQuery(m) | StoreError::NotSamePerson { message: m, .. }) => m,
        other => panic!("expected a refusal, got {other:?}"),
    };
    assert!(msg(merge(&db, &a, &a).await).contains("itself"));
    // A blank reason.
    let tx = db.begin().await.unwrap();
    assert!(matches!(
        links::merge(&tx, &a, &b, "x", "   ").await,
        Err(StoreError::BadQuery(_))
    ));
    drop(tx);
    assert_eq!(
        merge(&db, "00000000-0000-4000-8000-000000000000", &b).await,
        Err(StoreError::NotFound)
    );
    assert_eq!(
        merge(&db, &a, "not-a-uuid").await,
        Err(StoreError::NotFound)
    );

    merge(&db, &a, &b).await.unwrap();
    // The record merged away cannot be merged again, and cannot be a surviving record.
    assert!(msg(merge(&db, &c, &b).await).contains("already merged"));
    assert!(msg(merge(&db, &b, &c).await).contains("itself merged into"));
    // No cycles: A survives, so A cannot be merged into B (which is merged into A).
    assert!(msg(merge(&db, &b, &a).await).contains("itself merged into"));
    // Unmerging what is not merged.
    assert!(msg(unmerge(&db, &a, &c).await).contains("not currently merged"));
    assert!(msg(unmerge(&db, &c, &b).await).contains("not currently merged"));

    // Different people, and two different verified NHS numbers, are refused.
    let other = Patient {
        names: vec![HumanName {
            family: Some(tag("other")),
            given: vec!["Zed".into()],
        }],
        birth_date: Some("1999-12-31".into()),
        ..Default::default()
    };
    let o = patients_store::create(&db, "t", None, &other)
        .await
        .unwrap()
        .id;
    assert!(msg(merge(&db, &a, &o).await).contains("do not look like the same person"));
    let numbers = |n: &str| Patient {
        names: vec![HumanName {
            family: Some(family.clone()),
            given: vec!["Sam".into()],
        }],
        birth_date: Some("1975-06-01".into()),
        identifiers: vec![electronic_patient_record_matcher::Identifier {
            system: electronic_patient_record_matcher::NHS_NUMBER_SYSTEM.into(),
            value: n.into(),
        }],
        ..Default::default()
    };
    let n1 = patients_store::create(&db, "t", None, &numbers("9434765919"))
        .await
        .unwrap()
        .id;
    let n2 = patients_store::create(&db, "t", None, &numbers("4010232137"))
        .await
        .unwrap()
        .id;
    assert!(msg(merge(&db, &n1, &n2).await).contains("do not look like the same person"));
    // The refusal carries the reasons as stable codes for a client that translates them (CQ-38).
    match merge(&db, &a, &o).await {
        Err(StoreError::NotSamePerson { conflict_codes, .. }) => {
            assert!(conflict_codes.is_array(), "{conflict_codes}");
        }
        other => panic!("expected NotSamePerson, got {other:?}"),
    }
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_chain_deeper_than_sixteen_merges_is_an_error_not_a_wrong_answer() {
    let db = db().await;
    let family = tag("deep");
    let mut people = Vec::new();
    for _ in 0..18 {
        people.push(person(&db, &family).await);
    }
    for i in 0..16 {
        merge(&db, &people[i + 1], &people[i]).await.unwrap();
    }
    // Sixteen merges: the top is found.
    assert_eq!(
        links::merged_into(&db, &people[0])
            .await
            .unwrap()
            .as_deref(),
        Some(people[16].as_str())
    );
    // A seventeenth: the chain from the first record is too long to follow.
    merge(&db, &people[17], &people[16]).await.unwrap();
    let e = links::merged_into(&db, &people[0]).await.unwrap_err();
    assert!(e.to_string().contains("more than 16"), "{e}");
    // Shorter chains from the same top are unaffected.
    assert_eq!(
        links::merged_into(&db, &people[1])
            .await
            .unwrap()
            .as_deref(),
        Some(people[17].as_str())
    );
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn chains_resolve_to_the_top_and_unmerging_a_middle_link_splits_them() {
    let db = db().await;
    let family = tag("chain");
    let (a, b, c) = (
        person(&db, &family).await,
        person(&db, &family).await,
        person(&db, &family).await,
    );
    merge(&db, &b, &a).await.unwrap(); // A -> B
    merge(&db, &c, &b).await.unwrap(); // B -> C
    assert_eq!(
        links::merged_into(&db, &a).await.unwrap().as_deref(),
        Some(c.as_str())
    );
    assert_eq!(
        links::merged_into(&db, &b).await.unwrap().as_deref(),
        Some(c.as_str())
    );
    let mut sources = links::sources_of(&db, &c).await.unwrap();
    sources.sort();
    let mut want = vec![a.clone(), b.clone()];
    want.sort();
    assert_eq!(sources, want);
    unmerge(&db, &c, &b).await.unwrap(); // B is live again; A still points at B
    assert_eq!(links::merged_into(&db, &b).await.unwrap(), None);
    assert_eq!(
        links::merged_into(&db, &a).await.unwrap().as_deref(),
        Some(b.as_str())
    );
    assert_eq!(
        links::sources_of(&db, &c).await.unwrap(),
        Vec::<String>::new()
    );
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn allergies_follow_the_merge_by_provenance_and_return_on_unmerge() {
    let db = db().await;
    let family = tag("allergy");
    let (t, s) = (person(&db, &family).await, person(&db, &family).await);
    let on_source = allergies::create(&db, &s, "n", None, &allergy("Penicillin"))
        .await
        .unwrap();
    let on_target = allergies::create(&db, &t, "n", None, &allergy("Latex"))
        .await
        .unwrap();
    let (before_t, before_s) = (ids_of(&db, &t).await, ids_of(&db, &s).await);
    assert_eq!((before_t.len(), before_s.len()), (1, 1));

    merge(&db, &t, &s).await.unwrap();
    // The surviving record shows both, and each allergy still names the record it was made on.
    let both = allergies::list(&db, &t).await.unwrap();
    assert_eq!(both.len(), 2);
    assert!(
        both.iter()
            .any(|a| a.id == on_source.id && a.patient_id == s)
    );
    assert!(
        both.iter()
            .any(|a| a.id == on_target.id && a.patient_id == t)
    );
    let Summary::Active { items } = allergies::summary(&db, &t).await.unwrap() else {
        panic!()
    };
    assert_eq!(items.len(), 2);
    // The merged-away record reads as itself.
    assert_eq!(ids_of(&db, &s).await, before_s);

    // An allergy found through the surviving record can be ended there; it keeps its own patient.
    let mut ended = on_source.allergy.clone();
    ended.status = Status::Inactive;
    let v2 = allergies::update(&db, &t, &on_source.id, 1, "dr", Some("resolved"), &ended)
        .await
        .unwrap();
    assert_eq!((v2.version, v2.patient_id.as_str()), (2, s.as_str()));

    unmerge(&db, &t, &s).await.unwrap();
    assert_eq!(
        ids_of(&db, &t).await,
        before_t,
        "the surviving record has only its own again"
    );
    assert_eq!(
        ids_of(&db, &s).await,
        before_s,
        "the other record has its own again"
    );
    assert_eq!(
        allergies::current(&db, &s, &on_source.id)
            .await
            .unwrap()
            .version,
        2
    );
}

/// A tiny deterministic generator, so a failing sequence can be replayed from its seed.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self, n: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        usize::try_from((self.0 >> 33) % n as u64).unwrap()
    }
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn property_unmerging_everything_restores_every_record_exactly() {
    let db = db().await;
    for seed in 1..=6u64 {
        let family = tag(&format!("prop{seed}"));
        let mut rng = Lcg(seed * 7919);
        let people: Vec<String> = {
            let mut v = Vec::new();
            for _ in 0..4 {
                v.push(person(&db, &family).await);
            }
            v
        };
        // The model: who each allergy was made on, and the active merges (source index -> target).
        let mut own: Vec<Vec<String>> = vec![Vec::new(); 4];
        let mut parent: HashMap<usize, usize> = HashMap::new();
        let top = |parent: &HashMap<usize, usize>, mut i: usize| {
            while let Some(&p) = parent.get(&i) {
                i = p;
            }
            i
        };
        let expected = |own: &Vec<Vec<String>>,
                        parent: &HashMap<usize, usize>,
                        p: usize|
         -> BTreeSet<String> {
            // p plus everyone whose chain of merges ends at or passes through p.
            (0..4)
                .filter(|&q| {
                    let mut i = q;
                    loop {
                        if i == p {
                            return true;
                        }
                        match parent.get(&i) {
                            Some(&n) => i = n,
                            None => return false,
                        }
                    }
                })
                .flat_map(|q| own[q].clone())
                .collect()
        };

        for step in 0..40 {
            match rng.next(3) {
                0 => {
                    let p = rng.next(4);
                    let a = allergies::create(
                        &db,
                        &people[p],
                        "n",
                        None,
                        &allergy(&format!("Thing{seed}-{step}")),
                    )
                    .await
                    .unwrap();
                    own[p].push(a.id);
                }
                1 => {
                    let (s, t) = (rng.next(4), rng.next(4));
                    let allowed = s != t && !parent.contains_key(&s) && !parent.contains_key(&t);
                    let got = merge(&db, &people[t], &people[s]).await;
                    assert_eq!(
                        got.is_ok(),
                        allowed,
                        "seed {seed} step {step}: merge {s}->{t}: {got:?}"
                    );
                    if allowed {
                        parent.insert(s, t);
                    }
                }
                _ => {
                    let active: Vec<(usize, usize)> =
                        parent.iter().map(|(&s, &t)| (s, t)).collect();
                    if active.is_empty() {
                        continue;
                    }
                    let (s, t) = active[rng.next(active.len())];
                    unmerge(&db, &people[t], &people[s]).await.unwrap();
                    parent.remove(&s);
                }
            }
            for p in 0..4 {
                assert_eq!(
                    ids_of(&db, &people[p]).await,
                    expected(&own, &parent, p),
                    "seed {seed} step {step}: person {p}"
                );
                assert_eq!(
                    links::merged_into(&db, &people[p]).await.unwrap(),
                    parent
                        .contains_key(&p)
                        .then(|| people[top(&parent, p)].clone()),
                    "seed {seed} step {step}: merged_into {p}"
                );
            }
        }
        // Undo everything that is still merged: every record has exactly its own allergies again,
        // and no allergy row was ever moved or changed.
        let active: Vec<(usize, usize)> = parent.iter().map(|(&s, &t)| (s, t)).collect();
        for (s, t) in active {
            unmerge(&db, &people[t], &people[s]).await.unwrap();
        }
        for p in 0..4 {
            assert_eq!(
                ids_of(&db, &people[p]).await,
                own[p].iter().cloned().collect::<BTreeSet<_>>(),
                "seed {seed}: person {p} after unmerging everything"
            );
            for id in &own[p] {
                assert_eq!(
                    allergies::current(&db, &people[p], id)
                        .await
                        .unwrap()
                        .patient_id,
                    people[p]
                );
            }
        }
    }
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn two_merges_of_the_same_record_into_different_targets_get_one_winner() {
    let db = db().await;
    let family = tag("race");
    let (t1, t2, s) = (
        person(&db, &family).await,
        person(&db, &family).await,
        person(&db, &family).await,
    );
    let (a, b) = tokio::join!(merge(&db, &t1, &s), merge(&db, &t2, &s));
    assert_eq!(
        [a.is_ok(), b.is_ok()].iter().filter(|x| **x).count(),
        1,
        "{a:?} / {b:?}"
    );
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn link_versions_cannot_be_changed_or_removed() {
    let db = db().await;
    let family = tag("immutable");
    let (t, s) = (person(&db, &family).await, person(&db, &family).await);
    let link = merge(&db, &t, &s).await.unwrap();
    for sql in [
        format!(
            "UPDATE patient_link_versions SET reason = 'x' WHERE link_id = '{}'",
            link.id
        ),
        format!(
            "DELETE FROM patient_link_versions WHERE link_id = '{}'",
            link.id
        ),
        "TRUNCATE patient_link_versions".to_string(),
    ] {
        let e = db.execute_unprepared(&sql).await.unwrap_err().to_string();
        assert!(e.contains("append-only"), "{sql}: {e}");
    }
}
