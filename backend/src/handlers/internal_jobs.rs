use anyhow::Result;
use axum::Json;
use axum::extract::State;

use crate::{ApiError, AppState, JobResponse};

pub(crate) async fn post_recompute_kpis(
    State(state): State<AppState>,
) -> Result<Json<JobResponse>, ApiError> {
    if state.backend != crate::DatabaseBackend::Sqlite {
        return Err(crate::errors::postgres_rollout_not_enabled(
            "/internal/jobs/recompute-kpis",
        ));
    }
    crate::jobs::run_kpi_job(&state.sqlite_pool).await.map(Json)
}

pub(crate) async fn post_build_rankings(
    State(state): State<AppState>,
) -> Result<Json<JobResponse>, ApiError> {
    if state.backend != crate::DatabaseBackend::Sqlite {
        return Err(crate::errors::postgres_rollout_not_enabled(
            "/internal/jobs/build-ranking-snapshots",
        ));
    }
    crate::jobs::run_kpi_job(&state.sqlite_pool).await.map(Json)
}
