use chrono::{Duration, Utc};

use crate::{ApiError, AppState, DatabaseBackend, JobResponse, JobRunStatusResponse, now_str};

mod postgres;
mod sqlite;

pub(crate) const JOB_KIND_RECOMPUTE_KPIS: &str = "recompute_kpis";
pub(crate) const JOB_KIND_BUILD_RANKINGS: &str = "build_rankings";
const STALE_RUN_RECOVERY_MINUTES: i64 = 10;

/// Normalizes and validates user-provided internal job kinds.
pub(crate) fn normalize_job_kind(job_kind: Option<String>) -> Result<String, ApiError> {
    let job_kind = job_kind.unwrap_or_else(|| JOB_KIND_RECOMPUTE_KPIS.to_string());
    if matches!(
        job_kind.as_str(),
        JOB_KIND_RECOMPUTE_KPIS | JOB_KIND_BUILD_RANKINGS
    ) {
        Ok(job_kind)
    } else {
        Err(ApiError::unprocessable(format!(
            "unsupported job_kind: {}",
            job_kind
        )))
    }
}

/// Persists the start of an internal job run.
pub(crate) async fn record_job_run_started(
    state: &AppState,
    job_kind: &str,
) -> Result<String, ApiError> {
    let job_run_id = uuid::Uuid::new_v4().to_string();
    let started_at = now_str();
    let backend = match state.backend {
        DatabaseBackend::Sqlite => "sqlite",
        DatabaseBackend::Postgres => "postgres",
    };

    match state.backend {
        DatabaseBackend::Sqlite => {
            sqlite::insert_started(
                &state.sqlite_pool,
                &job_run_id,
                job_kind,
                backend,
                &started_at,
            )
            .await?;
        }
        DatabaseBackend::Postgres => {
            let pg_pool = state
                .pg_pool
                .as_ref()
                .ok_or_else(|| ApiError::internal("postgres pool is not configured"))?;
            postgres::insert_started(pg_pool, &job_run_id, job_kind, backend, &started_at).await?;
        }
    }

    Ok(job_run_id)
}

/// Persists successful completion metadata for an internal job run.
pub(crate) async fn record_job_run_succeeded(
    state: &AppState,
    job_run_id: &str,
    response: &JobResponse,
) -> Result<(), ApiError> {
    let finished_at = now_str();
    match state.backend {
        DatabaseBackend::Sqlite => {
            sqlite::mark_succeeded(&state.sqlite_pool, job_run_id, &finished_at, response).await
        }
        DatabaseBackend::Postgres => {
            let pg_pool = state
                .pg_pool
                .as_ref()
                .ok_or_else(|| ApiError::internal("postgres pool is not configured"))?;
            postgres::mark_succeeded(pg_pool, job_run_id, &finished_at, response).await
        }
    }
}

/// Persists failure metadata for an internal job run.
pub(crate) async fn record_job_run_failed(
    state: &AppState,
    job_run_id: &str,
    error_message: &str,
) -> Result<(), ApiError> {
    let finished_at = now_str();
    match state.backend {
        DatabaseBackend::Sqlite => {
            sqlite::mark_failed(&state.sqlite_pool, job_run_id, &finished_at, error_message).await
        }
        DatabaseBackend::Postgres => {
            let pg_pool = state
                .pg_pool
                .as_ref()
                .ok_or_else(|| ApiError::internal("postgres pool is not configured"))?;
            postgres::mark_failed(pg_pool, job_run_id, &finished_at, error_message).await
        }
    }
}

/// Fetches the latest run status for a requested job family.
pub(crate) async fn fetch_latest_job_run_status(
    state: &AppState,
    job_kind: &str,
) -> Result<Option<JobRunStatusResponse>, ApiError> {
    match state.backend {
        DatabaseBackend::Sqlite => sqlite::fetch_latest(&state.sqlite_pool, job_kind).await,
        DatabaseBackend::Postgres => {
            let pg_pool = state
                .pg_pool
                .as_ref()
                .ok_or_else(|| ApiError::internal("postgres pool is not configured"))?;
            postgres::fetch_latest(pg_pool, job_kind).await
        }
    }
}

/// Marks stale `running` rows as failed before a new run starts.
///
/// A run is considered stale when:
/// 1) the latest row is still `running`,
/// 2) `started_at` is older than the recovery window, and
/// 3) no active lock exists for the same `job_kind` (or the active lock is the
///    lock currently held by the caller).
pub(crate) async fn recover_stale_running_job(
    state: &AppState,
    job_kind: &str,
    caller_lock_owner: Option<&str>,
) -> Result<(), ApiError> {
    let latest = fetch_latest_job_run_status(state, job_kind).await?;
    let Some(latest) = latest else {
        return Ok(());
    };
    if latest.status != "running" {
        return Ok(());
    }

    let started_at = match chrono::DateTime::parse_from_rfc3339(&latest.started_at) {
        Ok(value) => value.with_timezone(&Utc),
        Err(_) => {
            // Invalid timestamp should not block new work; stale recovery is best-effort.
            return Ok(());
        }
    };
    let stale_before = Utc::now() - Duration::minutes(STALE_RUN_RECOVERY_MINUTES);
    if started_at >= stale_before {
        return Ok(());
    }

    if let Some(active_lock) = crate::job_locks::fetch_active_job_lock(state, job_kind).await? {
        let lock_is_owned_by_caller = caller_lock_owner
            .map(|owner| owner == active_lock.owner_token.as_str())
            .unwrap_or(false);
        if !lock_is_owned_by_caller {
            return Ok(());
        }
    }

    record_job_run_failed(
        state,
        &latest.job_run_id,
        "stale running job recovered automatically before new execution",
    )
    .await
}
