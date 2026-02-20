use sqlx::{PgPool, SqlitePool};

use crate::{ApiError, JobResponse};

mod from_postgres;
mod to_postgres;

use from_postgres::sync_job_inputs_from_postgres;
use to_postgres::{OutputSyncOptions, sync_job_outputs_to_postgres};

/// Runs the KPI job in Postgres mode by synchronizing job inputs/outputs through
/// the existing SQLite computation pipeline.
pub(super) async fn run_kpi_job_postgres(
    sqlite_pool: &SqlitePool,
    pg_pool: &PgPool,
) -> Result<JobResponse, ApiError> {
    // Stage 1 runs natively in Postgres so charging sessions do not depend on
    // bridge round-trips. KPI/ranking stages still reuse SQLite materialization.
    let native_summary = super::postgres_native::run_native_postgres_stages(pg_pool).await?;
    tracing::debug!(
        charging_sessions_upserted = native_summary.charging_sessions_upserted,
        charging_kpi_rows_upserted = native_summary.charging_kpi_rows_upserted,
        "native postgres job stages completed"
    );

    sync_job_inputs_from_postgres(pg_pool, sqlite_pool).await?;
    let mut response = super::run_kpi_job(sqlite_pool).await?;
    sync_job_outputs_to_postgres(
        sqlite_pool,
        pg_pool,
        &OutputSyncOptions {
            sync_charging_sessions: false,
            sync_charging_kpi_snapshots: false,
        },
    )
    .await?;

    // Keep public job response aligned with the native Postgres charging stage.
    response.charging_sessions_upserted = native_summary.charging_sessions_upserted;
    Ok(response)
}
