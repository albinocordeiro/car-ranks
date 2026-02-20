use std::str::FromStr;

use anyhow::{Context, Result};
use sqlx::postgres::PgPoolOptions;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{PgPool, SqlitePool};

/// Initialize runtime database resources and decide the active backend.
///
/// This function owns all environment-based backend selection and pool creation
/// so startup orchestration can stay focused on app assembly/serving.
pub(crate) async fn initialize_database()
-> Result<(crate::DatabaseBackend, SqlitePool, Option<PgPool>)> {
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://car_ranks.db".to_string());
    let backend = determine_backend(&database_url);
    let (sqlite_pool, pg_pool) = initialize_pools(backend, &database_url).await?;
    Ok((backend, sqlite_pool, pg_pool))
}

/// Pick runtime backend based on the configured URL scheme.
fn determine_backend(database_url: &str) -> crate::DatabaseBackend {
    if database_url.starts_with("postgres://") || database_url.starts_with("postgresql://") {
        crate::DatabaseBackend::Postgres
    } else {
        crate::DatabaseBackend::Sqlite
    }
}

/// Establish pools and apply migrations for the selected backend.
async fn initialize_pools(
    backend: crate::DatabaseBackend,
    database_url: &str,
) -> Result<(SqlitePool, Option<PgPool>)> {
    match backend {
        crate::DatabaseBackend::Sqlite => {
            let connect_options = SqliteConnectOptions::from_str(database_url)
                .context("invalid sqlite DATABASE_URL")?
                .create_if_missing(true)
                .foreign_keys(true);

            let sqlite_pool = SqlitePoolOptions::new()
                .max_connections(10)
                .connect_with(connect_options)
                .await
                .context("failed to connect sqlite")?;
            crate::apply_schema(&sqlite_pool).await?;
            Ok((sqlite_pool, None))
        }
        crate::DatabaseBackend::Postgres => {
            let pg_pool = PgPoolOptions::new()
                .max_connections(10)
                .connect(database_url)
                .await
                .context("failed to connect postgres")?;
            crate::apply_postgres_schema(&pg_pool).await?;

            // Keep sqlite-only code paths available while postgres rollout is incremental.
            let sqlite_pool = SqlitePoolOptions::new()
                .max_connections(1)
                .connect("sqlite::memory:")
                .await
                .context("failed to create sqlite fallback pool")?;
            crate::apply_schema(&sqlite_pool).await?;
            Ok((sqlite_pool, Some(pg_pool)))
        }
    }
}
