use chrono::{Duration, Utc};

use crate::{ApiError, AppState, DatabaseBackend};

mod postgres;
mod sqlite;

const JOB_LOCK_TTL_MINUTES: i64 = 10;

/// Active lock metadata surfaced by job status APIs.
pub(crate) struct ActiveJobLock {
    pub(crate) owner_token: String,
    pub(crate) expires_at: String,
}

/// Acquires a short-lived lease for one internal job kind.
pub(crate) async fn acquire_job_lock(
    state: &AppState,
    job_kind: &str,
    owner_token: &str,
) -> Result<(), ApiError> {
    let acquired_at = Utc::now();
    let expires_at = acquired_at + Duration::minutes(JOB_LOCK_TTL_MINUTES);
    let acquired_at_ts = acquired_at.to_rfc3339();
    let expires_at_ts = expires_at.to_rfc3339();

    let acquired = match state.backend {
        DatabaseBackend::Sqlite => {
            sqlite::try_acquire(
                &state.sqlite_pool,
                job_kind,
                owner_token,
                &acquired_at_ts,
                &expires_at_ts,
            )
            .await?
        }
        DatabaseBackend::Postgres => {
            let pg_pool = state
                .pg_pool
                .as_ref()
                .ok_or_else(|| ApiError::internal("postgres pool is not configured"))?;
            postgres::try_acquire(
                pg_pool,
                job_kind,
                owner_token,
                &acquired_at_ts,
                &expires_at_ts,
            )
            .await?
        }
    };

    if acquired {
        Ok(())
    } else {
        Err(ApiError::conflict(
            "internal job already running for requested job_kind",
        ))
    }
}

/// Fetches active lock metadata for one job kind (if any).
pub(crate) async fn fetch_active_job_lock(
    state: &AppState,
    job_kind: &str,
) -> Result<Option<ActiveJobLock>, ApiError> {
    let now_ts = Utc::now().to_rfc3339();
    match state.backend {
        DatabaseBackend::Sqlite => {
            sqlite::fetch_active(&state.sqlite_pool, job_kind, &now_ts).await
        }
        DatabaseBackend::Postgres => {
            let pg_pool = state
                .pg_pool
                .as_ref()
                .ok_or_else(|| ApiError::internal("postgres pool is not configured"))?;
            postgres::fetch_active(pg_pool, job_kind, &now_ts).await
        }
    }
}

/// Releases a previously acquired lease.
pub(crate) async fn release_job_lock(
    state: &AppState,
    job_kind: &str,
    owner_token: &str,
) -> Result<(), ApiError> {
    match state.backend {
        DatabaseBackend::Sqlite => sqlite::release(&state.sqlite_pool, job_kind, owner_token).await,
        DatabaseBackend::Postgres => {
            let pg_pool = state
                .pg_pool
                .as_ref()
                .ok_or_else(|| ApiError::internal("postgres pool is not configured"))?;
            postgres::release(pg_pool, job_kind, owner_token).await
        }
    }
}
