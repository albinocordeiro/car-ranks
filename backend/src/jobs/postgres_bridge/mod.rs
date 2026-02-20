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
    // Stage 1 runs natively in Postgres so charging sessions/KPIs/rankings do not
    // depend on bridge round-trips.
    let pre_bridge_summary =
        super::postgres_native::run_native_postgres_pre_bridge_stages(pg_pool).await?;
    tracing::debug!(
        charging_sessions_upserted = pre_bridge_summary.charging_sessions_upserted,
        charging_kpi_rows_upserted = pre_bridge_summary.charging_kpi_rows_upserted,
        charging_ranking_rows_upserted = pre_bridge_summary.charging_ranking_rows_upserted,
        "native postgres pre-bridge job stages completed"
    );

    sync_job_inputs_from_postgres(pg_pool, sqlite_pool).await?;
    let mut response = super::run_kpi_job(sqlite_pool).await?;
    sync_job_outputs_to_postgres(
        sqlite_pool,
        pg_pool,
        &OutputSyncOptions {
            sync_charging_sessions: false,
            sync_charging_kpi_snapshots: false,
            sync_charging_rankings: false,
            sync_composite_kpi_snapshots: false,
            sync_composite_rankings: false,
            sync_range_rankings: false,
        },
    )
    .await?;

    // Stage 2 runs natively in Postgres after bridge sync so composite can read
    // bridge-synced range and native charging KPI families from one backend.
    let post_bridge_summary =
        super::postgres_native::run_native_postgres_post_bridge_stages(pg_pool).await?;
    tracing::debug!(
        range_ranking_rows_upserted = post_bridge_summary.range_ranking_rows_upserted,
        composite_kpi_rows_upserted = post_bridge_summary.composite_kpi_rows_upserted,
        composite_ranking_rows_upserted = post_bridge_summary.composite_ranking_rows_upserted,
        "native postgres post-bridge job stages completed"
    );

    // Keep public job response aligned with the native Postgres charging stage.
    response.charging_sessions_upserted = pre_bridge_summary.charging_sessions_upserted;
    Ok(response)
}
