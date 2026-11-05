//! What every store shares (CQ-12): building a Postgres statement, turning a database error into a
//! [`StoreError`] that names the store, and appending a version. Each module that writes versions
//! calls [`store_errors!`](crate::store_errors) once for its local `db_err`.

use crate::patients_store::StoreError;
use sqlx::Postgres;
use sqlx::postgres::{PgArguments, PgRow};
use sqlx::query::Query;

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

        /// A database failure in this store, as a `StoreError::Db` that names the store.
        fn db_err(e: impl std::fmt::Display) -> $crate::patients_store::StoreError {
            $crate::store::db_err(STORE, e)
        }
    };
}

/// What appending a version did.
pub(crate) enum Appended {
    /// The new row, as `RETURNING` gave it.
    Row(PgRow),
    /// Another writer took this version first.
    Taken,
}

/// Run `insert` (an `INSERT … RETURNING` of the next version) in a savepoint. A failed statement
/// aborts a Postgres transaction, so only the savepoint is rolled back and the caller's transaction,
/// if any, can still read the version that won. The caller turns [`Appended::Taken`] into its own
/// `Conflict` with that version. The primary key is the version tables' only unique constraint, so a
/// unique violation means the version was already taken; that is read from the driver's error kind,
/// not from the message.
///
/// # Errors
///
/// [`StoreError::Db`], naming `store`, for any other failure.
pub(crate) async fn append_version(
    db: impl Into<crate::pool::Db<'_>>,
    store: &str,
    insert: Query<'_, Postgres, PgArguments>,
) -> Result<Appended, StoreError> {
    let mut conn = db.into().acquire().await.map_err(|e| db_err(store, e))?;
    let mut sp = sqlx::Connection::begin(&mut *conn)
        .await
        .map_err(|e| db_err(store, e))?;
    match insert.fetch_optional(&mut *sp).await {
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
            if axa::sqlx::is_unique_violation(&e) {
                Ok(Appended::Taken)
            } else {
                Err(db_err(store, e))
            }
        }
    }
}
