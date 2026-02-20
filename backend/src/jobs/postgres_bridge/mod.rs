use sqlx::{PgPool, SqlitePool};

use crate::{ApiError, JobResponse};

mod from_postgres;
mod to_postgres;

use from_postgres::sync_job_inputs_from_postgres;
use to_postgres::sync_job_outputs_to_postgres;

/// Runs the KPI job in Postgres mode by synchronizing job inputs/outputs through
/// the existing SQLite computation pipeline.
pub(super) async fn run_kpi_job_postgres(
    sqlite_pool: &SqlitePool,
    pg_pool: &PgPool,
) -> Result<JobResponse, ApiError> {
    sync_job_inputs_from_postgres(pg_pool, sqlite_pool).await?;
    let response = super::run_kpi_job(sqlite_pool).await?;
    sync_job_outputs_to_postgres(sqlite_pool, pg_pool).await?;
    Ok(response)
}
