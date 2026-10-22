//! What every store shares (CQ-12): building a Postgres statement, turning a database error into a
//! [`StoreError`] that names the store, and appending a version. Each module that writes versions
//! calls [`store_errors!`](crate::store_errors) once for its local `db_err`.

use crate::patients_store::StoreError;
use sea_orm::{
    ConnectionTrait, DbBackend, DbErr, QueryResult, SqlErr, Statement, TransactionSession,
    TransactionTrait, Value,
};

/// A Postgres statement with bound values.
pub(crate) fn pg(sql: &str, values: Vec<Value>) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, sql, values)
}

/// A database failure in `store`, kept as text with the store's name so the log says where.
pub(crate) fn db_err(store: &str, e: impl std::fmt::Display) -> StoreError {
    StoreError::Db(format!("{store}: {e}"))
}

/// Defines the module's `db_err`, naming its store in every database error.
#[macro_export]
macro_rules! store_errors {
    ($store:literal) => {
        /// The store's name in its errors.
        const STORE: &str = $store;

        fn db_err(e: impl std::fmt::Display) -> $crate::patients_store::StoreError {
            $crate::store::db_err(STORE, e)
        }
    };
}

/// Whether the statement was refused by a unique constraint. The version tables have no unique
/// constraint but their primary key, so for them this means the version was already taken. Read from
/// the driver's error kind, not from the message.
pub(crate) fn is_unique_violation(e: &DbErr) -> bool {
    matches!(e.sql_err(), Some(SqlErr::UniqueConstraintViolation(_)))
}

/// What appending a version did.
pub(crate) enum Appended {
    /// The new row, as `RETURNING` gave it.
    Row(QueryResult),
    /// Another writer took this version first.
    Taken,
}

/// Run `insert` (an `INSERT … RETURNING` of the next version) in a savepoint. A failed statement
/// aborts a Postgres transaction, so only the savepoint is rolled back and the caller's transaction,
/// if any, can still read the version that won. The caller turns [`Appended::Taken`] into its own
/// `Conflict` with that version.
///
/// # Errors
///
/// [`StoreError::Db`], naming `store`, for any other failure.
pub(crate) async fn append_version(
    db: &(impl ConnectionTrait + TransactionTrait),
    store: &str,
    insert: Statement,
) -> Result<Appended, StoreError> {
    let sp = db.begin().await.map_err(|e| db_err(store, e))?;
    match sp.query_one_raw(insert).await {
        Ok(Some(row)) => {
            sp.commit().await.map_err(|e| db_err(store, e))?;
            Ok(Appended::Row(row))
        }
        Ok(None) => {
            let _ = sp.rollback().await;
            Err(db_err(store, "insert returned no row"))
        }
        Err(e) => {
            let _ = sp.rollback().await;
            if is_unique_violation(&e) {
                Ok(Appended::Taken)
            } else {
                Err(db_err(store, e))
            }
        }
    }
}
