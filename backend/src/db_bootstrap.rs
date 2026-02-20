use anyhow::{Context, Result};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

/// Initializes the process-level Postgres pool and applies schema migrations.
///
/// Startup intentionally fails fast when `DATABASE_URL` is missing or invalid so
/// the service never boots in a partially configured state.
pub(crate) async fn initialize_database() -> Result<PgPool> {
    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL is required and must point to Postgres")?;
    if !database_url.starts_with("postgres://") && !database_url.starts_with("postgresql://") {
        return Err(anyhow::anyhow!(
            "DATABASE_URL must use a Postgres scheme (postgres:// or postgresql://)"
        ));
    }

    let pg_pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .context("failed to connect postgres")?;
    crate::migrations::apply_postgres_schema(&pg_pool).await?;

    Ok(pg_pool)
}
