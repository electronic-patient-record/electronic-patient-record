//! Migration: `idempotency_keys` (task OF-1). A create sent with an `Idempotency-Key` header is run
//! once; a retry with the same key and the same request gets the stored answer instead of writing a
//! second record. `response` holds the answer as it was sent, so it can hold patient data: rows are
//! removed after a day. This is **not** clinical data (the record is written elsewhere, append-only),
//! which is why this table, unlike the clinical ones, may delete.

use sea_orm_migration::prelude::*;

/// The idempotency keys migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the table and its index.
    ///
    /// # Errors
    ///
    /// When any statement fails.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let db = m.get_connection();
        for sql in [
            "CREATE TABLE idempotency_keys (
                subject TEXT NOT NULL CHECK (subject <> ''),
                key TEXT NOT NULL CHECK (key <> ''),
                method TEXT NOT NULL,
                path TEXT NOT NULL,
                body_digest TEXT NOT NULL,
                state TEXT NOT NULL CHECK (state IN ('pending', 'done')),
                status SMALLINT,
                content_type TEXT,
                response BYTEA,
                created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                PRIMARY KEY (subject, key)
            )",
            "CREATE INDEX idempotency_keys_created_idx ON idempotency_keys (created_at)",
        ] {
            db.execute_unprepared(sql).await?;
        }
        Ok(())
    }

    /// Drop the table. It holds no clinical data, so this is allowed everywhere.
    ///
    /// # Errors
    ///
    /// When the statement fails.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS idempotency_keys")
            .await?;
        Ok(())
    }
}
