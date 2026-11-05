//! DB-gated (PR-5): the 24-hour life of an idempotency key is tested by moving a clock, not by rewriting a row.
//!
//! ```text
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test idempotency_clock -- --ignored --test-threads=1
//! ```
//!
//! Each test runs in a transaction that is rolled back, so the shared database stays clean.

use chrono::Duration;
use electronic_patient_record::clock::FixedClock;
use electronic_patient_record::idempotency::{self, Reservation, TTL_SECS};
use sqlx::PgPool;
use sqlx::Row;

/// How many keys the test's subject holds.
async fn ours(tx: &mut sqlx::PgConnection) -> i64 {
    sqlx::query("SELECT count(*) AS n FROM idempotency_keys WHERE subject = 'clock-test'")
        .fetch_optional(&mut *tx)
        .await
        .unwrap()
        .unwrap()
        .try_get::<i64, _>("n")
        .unwrap()
}

#[tokio::test]
#[ignore = "needs DATABASE_URL"]
async fn a_key_is_replayed_until_the_clock_passes_its_life_and_then_runs_again() {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for --ignored tests");
    let db = PgPool::connect(&url).await.expect("connect");
    electronic_patient_record::MIGRATOR
        .run(&db)
        .await
        .expect("migrate");
    let mut tx = db.begin().await.expect("begin");
    let clock = FixedClock::new("2026-11-04T09:00:00Z".parse().unwrap());
    let first = idempotency::reserve_at(
        &mut *tx,
        &clock,
        "clock-test",
        "k1",
        "POST",
        "/api/x",
        "sha256:a",
    )
    .await;
    assert!(matches!(first, Ok(Reservation::Fresh)));

    // Just inside the life: the same key is still held (the first request has not finished).
    clock.advance(Duration::seconds(TTL_SECS - 60));
    let inside = idempotency::reserve_at(
        &mut *tx,
        &clock,
        "clock-test",
        "k1",
        "POST",
        "/api/x",
        "sha256:a",
    )
    .await;
    assert!(matches!(inside, Ok(Reservation::Busy)));

    // Past it: the old key is removed and the request is new.
    clock.advance(Duration::seconds(120));
    let past = idempotency::reserve_at(
        &mut *tx,
        &clock,
        "clock-test",
        "k1",
        "POST",
        "/api/x",
        "sha256:a",
    )
    .await;
    assert!(matches!(past, Ok(Reservation::Fresh)));

    // The purge uses the same clock. Other rows in a shared database may be old, so look at ours only.
    idempotency::purge_expired_at(&mut *tx, &clock)
        .await
        .unwrap();
    assert_eq!(ours(&mut tx).await, 1, "not yet old enough to purge");
    clock.advance(Duration::seconds(TTL_SECS + 1));
    idempotency::purge_expired_at(&mut *tx, &clock)
        .await
        .unwrap();
    assert_eq!(
        ours(&mut tx).await,
        0,
        "purged once the clock passed its life"
    );
    tx.rollback().await.expect("rollback");
}
