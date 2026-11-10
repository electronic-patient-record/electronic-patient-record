//! The database pool, on `axa`'s context, and where a store function gets its connection.

use axa::AppContext;
use sqlx::PgPool;

pub use axa::postgres_sqlx::{Conn, Db};

/// The database pool of a context.
pub trait Pool {
    /// A handle to the pool. Cloning one is cheap: it shares the connections.
    fn pool(&self) -> PgPool;
}

impl Pool for AppContext {
    fn pool(&self) -> PgPool {
        self.db.clone()
    }
}
