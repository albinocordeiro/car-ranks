use std::collections::HashSet;
use std::sync::Arc;

use sqlx::PgPool;

/// Shared application state injected into handlers.
#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) pg_pool: PgPool,
    pub(crate) signal_keys: Arc<HashSet<String>>,
}
