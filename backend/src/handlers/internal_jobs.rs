use anyhow::Result;
use axum::Json;
use axum::extract::State;

use crate::{ApiError, AppState, JobResponse};

pub(crate) async fn post_recompute_kpis(
    State(state): State<AppState>,
) -> Result<Json<JobResponse>, ApiError> {
    crate::jobs::run_kpi_job_by_backend(&state).await.map(Json)
}

pub(crate) async fn post_build_rankings(
    State(state): State<AppState>,
) -> Result<Json<JobResponse>, ApiError> {
    crate::jobs::run_kpi_job_by_backend(&state).await.map(Json)
}
