use std::collections::HashSet;
use std::sync::Arc;

use sqlx::{PgPool, SqlitePool};

/// Active database backend for the current process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DatabaseBackend {
    Sqlite,
    Postgres,
}

/// Shared application state injected into handlers.
#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) sqlite_pool: SqlitePool,
    pub(crate) pg_pool: Option<PgPool>,
    pub(crate) backend: DatabaseBackend,
    pub(crate) signal_keys: Arc<HashSet<String>>,
}
