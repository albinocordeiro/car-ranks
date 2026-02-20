use anyhow::{Context, Result};
use uuid::Uuid;

use crate::{ApiError, AppState, JobResponse};

mod postgres_native;

/// Runs the full KPI/ranking recompute pipeline against Postgres.
pub(crate) async fn run_kpi_job_by_backend(state: &AppState) -> Result<JobResponse, ApiError> {
    let job_id = Uuid::new_v4().to_string();

    let native_summary = postgres_native::run_native_postgres_job(&state.pg_pool)
        .await
        .context("failed to run native postgres recompute pipeline")?;
    let recomputed_vehicles = postgres_native::count_postgres_vehicles(&state.pg_pool)
        .await
        .context("failed to count postgres vehicles after recompute")?;

    Ok(JobResponse {
        ok: true,
        job_id,
        charging_sessions_upserted: native_summary.charging_sessions_upserted,
        kpi_rows_upserted: native_summary.total_kpi_rows_upserted(),
        ranking_rows_upserted: native_summary.total_ranking_rows_upserted(),
        recomputed_vehicles,
    })
}
