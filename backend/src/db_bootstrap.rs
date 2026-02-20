use anyhow::{Context, Result};
use sqlx::postgres::PgPoolOptions;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{PgPool, SqlitePool};

/// Initialize runtime database resources and decide the active backend.
///
/// This function owns all environment-based backend selection and pool creation
/// so startup orchestration can stay focused on app assembly/serving.
pub(crate) async fn initialize_database()
-> Result<(crate::DatabaseBackend, SqlitePool, Option<PgPool>)> {
    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL is required and must point to Postgres")?;
    let backend = determine_backend(&database_url)?;
    let (sqlite_pool, pg_pool) = initialize_pools(backend, &database_url).await?;
    Ok((backend, sqlite_pool, pg_pool))
}

/// Pick runtime backend based on the configured URL scheme.
fn determine_backend(database_url: &str) -> Result<crate::DatabaseBackend> {
    if database_url.starts_with("postgres://") || database_url.starts_with("postgresql://") {
        Ok(crate::DatabaseBackend::Postgres)
    } else {
        Err(anyhow::anyhow!(
            "DATABASE_URL must use a Postgres scheme (postgres:// or postgresql://)"
        ))
    }
}

/// Establish pools and apply migrations for the selected backend.
async fn initialize_pools(
    backend: crate::DatabaseBackend,
    database_url: &str,
) -> Result<(SqlitePool, Option<PgPool>)> {
    match backend {
        crate::DatabaseBackend::Postgres => {
            let pg_pool = PgPoolOptions::new()
                .max_connections(10)
                .connect(database_url)
                .await
                .context("failed to connect postgres")?;
            crate::migrations::apply_postgres_schema(&pg_pool).await?;

            // Keep an internal SQLite bridge pool until all remaining compute stages
            // are fully native in Postgres. SQLite is not supported as a runtime backend.
            let sqlite_pool = SqlitePoolOptions::new()
                .max_connections(1)
                .connect("sqlite::memory:")
                .await
                .context("failed to create sqlite fallback pool")?;
            crate::migrations::apply_schema(&sqlite_pool).await?;
            Ok((sqlite_pool, Some(pg_pool)))
        }
        crate::DatabaseBackend::Sqlite => Err(anyhow::anyhow!(
            "SQLite runtime backend is no longer supported"
        )),
    }
}
