//! The database pool, on `axa`'s context.
//!
//! `axa`'s `AppContext` holds no sqlx pool yet, so the pool lives in its shared store, put there once by
//! `App::context`. This is the one place that reads it.

use axa::AppContext;
use sqlx::pool::PoolConnection;
use sqlx::{PgConnection, PgPool, Postgres};
use std::ops::{Deref, DerefMut};

/// The database pool of a context.
pub trait Pool {
    /// A handle to the pool. Cloning one is cheap: it shares the connections.
    ///
    /// # Panics
    ///
    /// When the context did not come from `App::context`, which is a fault in this crate.
    fn pool(&self) -> PgPool;
}

impl Pool for AppContext {
    fn pool(&self) -> PgPool {
        self.shared_store
            .get::<PgPool>()
            .expect("App::context stores the pool")
    }
}

/// Where a store function gets its connection: the pool, or a connection the caller holds, usually an
/// open transaction. `impl sqlx::Acquire` would say the same, but its associated types stop the futures
/// being `Send`, which every Axum handler and `tokio::spawn` needs. Below this, store functions use plain
/// `sqlx` on a `&mut PgConnection`.
pub enum Db<'a> {
    /// Take a connection from the pool for the length of the call.
    Pool(&'a PgPool),
    /// Use the caller's connection.
    Conn(&'a mut PgConnection),
}

impl<'a> From<&'a PgPool> for Db<'a> {
    fn from(pool: &'a PgPool) -> Self {
        Self::Pool(pool)
    }
}

impl<'a> From<&'a mut PgConnection> for Db<'a> {
    fn from(conn: &'a mut PgConnection) -> Self {
        Self::Conn(conn)
    }
}

/// A connection for the length of one store call.
pub enum Conn<'a> {
    /// Taken from the pool, and given back when dropped.
    Pooled(PoolConnection<Postgres>),
    /// The caller's.
    Borrowed(&'a mut PgConnection),
}

impl<'a> Db<'a> {
    /// The connection to use.
    ///
    /// # Errors
    ///
    /// When the pool has none to give.
    pub async fn acquire(self) -> Result<Conn<'a>, sqlx::Error> {
        match self {
            Self::Pool(pool) => Ok(Conn::Pooled(pool.acquire().await?)),
            Self::Conn(conn) => Ok(Conn::Borrowed(conn)),
        }
    }
}

impl Deref for Conn<'_> {
    type Target = PgConnection;

    fn deref(&self) -> &PgConnection {
        match self {
            Self::Pooled(conn) => conn,
            Self::Borrowed(conn) => conn,
        }
    }
}

impl DerefMut for Conn<'_> {
    fn deref_mut(&mut self) -> &mut PgConnection {
        match self {
            Self::Pooled(conn) => conn,
            Self::Borrowed(conn) => conn,
        }
    }
}
