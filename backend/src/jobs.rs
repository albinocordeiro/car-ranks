use anyhow::{Context, Result};
use sqlx::PgPool;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{ApiError, AppState, DatabaseBackend, JobResponse};

mod charging_sessions;
mod kpi_recompute;
mod postgres_bridge;
mod postgres_native;
mod ranking_snapshots;

pub(crate) async fn run_kpi_job(pool: &SqlitePool) -> Result<JobResponse, ApiError> {
    let job_id = Uuid::new_v4().to_string();

    // Rebuild charging sessions first so downstream KPI jobs read the latest aggregates.
    let charging_sessions_upserted = charging_sessions::build_charging_sessions(pool)
        .await
        .context("failed to build charging sessions")?;

    let (kpi_rows_upserted, recomputed_vehicles) = recompute_all_kpis(pool)
        .await
        .context("failed to recompute KPIs")?;

    let ranking_rows_upserted = rebuild_all_rankings(pool)
        .await
        .context("failed to rebuild ranking snapshots for all ranking types")?;

    Ok(JobResponse {
        ok: true,
        job_id,
        charging_sessions_upserted,
        kpi_rows_upserted,
        ranking_rows_upserted,
        recomputed_vehicles,
    })
}

/// Runs the KPI/rebuild job against the active runtime backend.
pub(crate) async fn run_kpi_job_by_backend(state: &AppState) -> Result<JobResponse, ApiError> {
    match state.backend {
        DatabaseBackend::Sqlite => run_kpi_job(&state.sqlite_pool).await,
        DatabaseBackend::Postgres => {
            let pg_pool = state
                .pg_pool
                .as_ref()
                .ok_or_else(|| ApiError::internal("postgres pool is not configured"))?;
            run_kpi_job_postgres_bridge(&state.sqlite_pool, pg_pool).await
        }
    }
}

async fn run_kpi_job_postgres_bridge(
    sqlite_pool: &SqlitePool,
    pg_pool: &PgPool,
) -> Result<JobResponse, ApiError> {
    postgres_bridge::run_kpi_job_postgres(sqlite_pool, pg_pool).await
}

pub(crate) async fn recompute_all_kpis(pool: &SqlitePool) -> Result<(usize, usize)> {
    let (temp_rows, temp_vehicles) = recompute_temperature_kpis(pool).await?;
    let (other_rows, other_vehicles) = recompute_non_temperature_kpis(pool).await?;
    Ok((temp_rows + other_rows, temp_vehicles.max(other_vehicles)))
}

pub(crate) async fn recompute_temperature_kpis(pool: &SqlitePool) -> Result<(usize, usize)> {
    kpi_recompute::recompute_temperature_kpis(pool).await
}

pub(crate) async fn recompute_non_temperature_kpis(pool: &SqlitePool) -> Result<(usize, usize)> {
    kpi_recompute::recompute_non_temperature_kpis(pool).await
}

pub(crate) async fn rebuild_all_rankings(pool: &SqlitePool) -> Result<usize> {
    let temp_rows = rebuild_temperature_rankings(pool).await?;
    let non_temp_rows = rebuild_non_temperature_rankings(pool).await?;
    Ok(temp_rows + non_temp_rows)
}

// These narrow wrappers keep the historical `crate::jobs::*` call surface
// stable for tests while delegating implementation details to submodules.
pub(crate) async fn rebuild_temperature_rankings(pool: &SqlitePool) -> Result<usize> {
    ranking_snapshots::rebuild_temperature_rankings(pool).await
}

pub(crate) async fn rebuild_non_temperature_rankings(pool: &SqlitePool) -> Result<usize> {
    ranking_snapshots::rebuild_non_temperature_rankings(pool).await
}
