use chrono::{Duration, Utc};

use crate::{ApiError, AppState};

mod postgres;

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

    let acquired = postgres::try_acquire(
        &state.pg_pool,
        job_kind,
        owner_token,
        &acquired_at_ts,
        &expires_at_ts,
    )
    .await?;

    if acquired {
        return Ok(());
    }

    if let Some(lock) = fetch_active_job_lock(state, job_kind).await? {
        return Err(ApiError::conflict(format!(
            "internal job already running for requested job_kind (owner_token={}, expires_at={})",
            lock.owner_token, lock.expires_at
        )));
    }
    Err(ApiError::conflict(
        "internal job already running for requested job_kind",
    ))
}

/// Fetches active lock metadata for one job kind (if any).
pub(crate) async fn fetch_active_job_lock(
    state: &AppState,
    job_kind: &str,
) -> Result<Option<ActiveJobLock>, ApiError> {
    let now_ts = Utc::now().to_rfc3339();
    postgres::fetch_active(&state.pg_pool, job_kind, &now_ts).await
}

/// Releases a previously acquired lease.
pub(crate) async fn release_job_lock(
    state: &AppState,
    job_kind: &str,
    owner_token: &str,
) -> Result<(), ApiError> {
    postgres::release(&state.pg_pool, job_kind, owner_token).await
}
