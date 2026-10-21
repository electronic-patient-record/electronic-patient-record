//! DB-gated: patient photos are append-only versions; the type comes from the bytes; a stale write
//! is a conflict; withdrawal is a version, not a delete.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test photos_store -- --ignored --test-threads=1
//! ```

use electronic_patient_record::patients_store::{self, StoreError};
use electronic_patient_record::photos_store::{self as photos, MAX_BYTES};
use electronic_patient_record_matcher::{HumanName, Patient};
use electronic_patient_record_migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection};

const PNG: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 1, 2, 3];
const JPG: &[u8] = &[0xFF, 0xD8, 0xFF, 0xE0, 9, 9, 9];

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
            family: Some(format!("Zzphoto{nanos}")),
            given: vec!["Pic".into()],
        }],
        birth_date: Some("1990-01-01".into()),
        ..Default::default()
    };
    patients_store::create(db, "t", None, &p).await.unwrap().id
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn a_photo_is_versioned_replaced_and_withdrawn_without_losing_history() {
    let db = db().await;
    let id = patient(&db).await;
    assert_eq!(photos::current(&db, &id).await.unwrap(), None);

    let v1 = photos::put(&db, &id, 0, "alice", Some("first"), Some(PNG))
        .await
        .unwrap();
    assert_eq!(
        (v1.version, v1.content_type.as_deref()),
        (1, Some("image/png"))
    );
    let v2 = photos::put(&db, &id, 1, "bob", None, Some(JPG))
        .await
        .unwrap();
    assert_eq!(
        (v2.version, v2.content_type.as_deref()),
        (2, Some("image/jpeg"))
    );
    assert_eq!(
        photos::current(&db, &id)
            .await
            .unwrap()
            .unwrap()
            .bytes
            .as_deref(),
        Some(JPG)
    );

    // Withdrawal is a new version with no bytes.
    let v3 = photos::put(&db, &id, 2, "bob", Some("wrong person"), None)
        .await
        .unwrap();
    assert_eq!((v3.version, v3.bytes), (3, None));
    assert!(
        photos::current(&db, &id)
            .await
            .unwrap()
            .unwrap()
            .bytes
            .is_none()
    );
    // Nothing to withdraw twice.
    assert!(matches!(
        photos::put(&db, &id, 3, "bob", None, None).await,
        Err(StoreError::BadQuery(_))
    ));
    // All three versions are still there, with their authors.
    let rows = db
        .query_all_raw(sea_orm::Statement::from_sql_and_values(
            sea_orm::DbBackend::Postgres,
            "SELECT version, author, content_type FROM patient_photos WHERE patient_id = $1::uuid ORDER BY version",
            [id.clone().into()],
        ))
        .await
        .unwrap();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].try_get::<String>("", "author").unwrap(), "alice");
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn bad_files_stale_writes_and_unknown_patients_are_refused() {
    let db = db().await;
    let id = patient(&db).await;
    for bad in [&b"<svg onload=alert(1)>"[..], b"GIF89a", b"", b"%PDF-1.4"] {
        assert!(
            matches!(
                photos::put(&db, &id, 0, "a", None, Some(bad)).await,
                Err(StoreError::BadQuery(_))
            ),
            "{bad:?}"
        );
    }
    let mut big = PNG.to_vec();
    big.resize(MAX_BYTES + 1, 0);
    assert!(matches!(
        photos::put(&db, &id, 0, "a", None, Some(&big)).await,
        Err(StoreError::BadQuery(_))
    ));
    assert_eq!(
        photos::current(&db, &id).await.unwrap(),
        None,
        "a refused file stores nothing"
    );

    photos::put(&db, &id, 0, "a", None, Some(PNG))
        .await
        .unwrap();
    assert_eq!(
        photos::put(&db, &id, 0, "b", None, Some(JPG)).await,
        Err(StoreError::Conflict { current: 1 })
    );
    assert_eq!(
        photos::put(
            &db,
            "00000000-0000-4000-8000-000000000000",
            0,
            "a",
            None,
            Some(PNG)
        )
        .await,
        Err(StoreError::NotFound)
    );
    assert_eq!(
        photos::current(&db, "not-a-uuid").await,
        Err(StoreError::NotFound)
    );
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn two_writers_racing_for_the_same_photo_version_get_one_winner() {
    let db = db().await;
    let id = patient(&db).await;
    let (a, b) = tokio::join!(
        photos::put(&db, &id, 0, "a", None, Some(PNG)),
        photos::put(&db, &id, 0, "b", None, Some(JPG)),
    );
    assert_eq!(
        [a.is_ok(), b.is_ok()].iter().filter(|x| **x).count(),
        1,
        "{a:?} / {b:?}"
    );
    assert_eq!(photos::current(&db, &id).await.unwrap().unwrap().version, 1);
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
async fn photo_versions_cannot_be_changed_or_removed() {
    let db = db().await;
    let id = patient(&db).await;
    photos::put(&db, &id, 0, "a", None, Some(PNG))
        .await
        .unwrap();
    for sql in [
        format!("UPDATE patient_photos SET author = 'x' WHERE patient_id = '{id}'"),
        format!("DELETE FROM patient_photos WHERE patient_id = '{id}'"),
        "TRUNCATE patient_photos".to_string(),
    ] {
        let e = db.execute_unprepared(&sql).await.unwrap_err().to_string();
        assert!(e.contains("append-only"), "{sql}: {e}");
    }
}
