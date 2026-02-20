use anyhow::Result;
use axum::Json;
use axum::extract::{Query, State};

use crate::{ApiError, AppState, JobResponse, JobRunStatusResponse, JobStatusQuery};

pub(crate) async fn post_recompute_kpis(
    State(state): State<AppState>,
) -> Result<Json<JobResponse>, ApiError> {
    run_tracked_job(&state, crate::job_runs::JOB_KIND_RECOMPUTE_KPIS).await
}

pub(crate) async fn post_build_rankings(
    State(state): State<AppState>,
) -> Result<Json<JobResponse>, ApiError> {
    run_tracked_job(&state, crate::job_runs::JOB_KIND_BUILD_RANKINGS).await
}

pub(crate) async fn get_latest_job_status(
    State(state): State<AppState>,
    Query(params): Query<JobStatusQuery>,
) -> Result<Json<JobRunStatusResponse>, ApiError> {
    let job_kind = crate::job_runs::normalize_job_kind(params.job_kind)?;
    let latest = crate::job_runs::fetch_latest_job_run_status(&state, &job_kind).await?;
    let Some(mut latest) = latest else {
        return Err(ApiError::not_found(
            "no internal job run found for requested job_kind",
        ));
    };

    if let Some(lock) = crate::job_locks::fetch_active_job_lock(&state, &job_kind).await? {
        latest.active_lock_owner_token = Some(lock.owner_token);
        latest.active_lock_expires_at = Some(lock.expires_at);
    }

    Ok(Json(latest))
}

async fn run_tracked_job(state: &AppState, job_kind: &str) -> Result<Json<JobResponse>, ApiError> {
    let lock_owner = uuid::Uuid::new_v4().to_string();
    crate::job_locks::acquire_job_lock(state, job_kind, &lock_owner).await?;

    let run_result = run_tracked_job_with_lock(state, job_kind).await;
    if let Err(release_error) =
        crate::job_locks::release_job_lock(state, job_kind, &lock_owner).await
    {
        eprintln!(
            "failed to release internal job lock for {}: {}",
            job_kind, release_error.message
        );
        if run_result.is_ok() {
            return Err(release_error);
        }
    }

    run_result
}

async fn run_tracked_job_with_lock(
    state: &AppState,
    job_kind: &str,
) -> Result<Json<JobResponse>, ApiError> {
    let job_run_id = crate::job_runs::record_job_run_started(state, job_kind).await?;
    match crate::jobs::run_kpi_job_by_backend(state).await {
        Ok(response) => {
            crate::job_runs::record_job_run_succeeded(state, &job_run_id, &response).await?;
            Ok(Json(response))
        }
        Err(error) => {
            if let Err(update_error) =
                crate::job_runs::record_job_run_failed(state, &job_run_id, &error.message).await
            {
                eprintln!(
                    "failed to persist internal job failure metadata: {}",
                    update_error.message
                );
            }
            Err(error)
        }
    }
}
