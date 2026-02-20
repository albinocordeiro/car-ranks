use anyhow::Context;
use sqlx::{PgPool, Row};

use crate::{ApiError, job_locks::ActiveJobLock};

pub(super) async fn try_acquire(
    pool: &PgPool,
    job_kind: &str,
    owner_token: &str,
    acquired_at: &str,
    expires_at: &str,
) -> Result<bool, ApiError> {
    let result = sqlx::query(
        r#"
        INSERT INTO internal_job_lock (
          job_kind,
          owner_token,
          acquired_at,
          expires_at
        ) VALUES ($1, $2, $3, $4)
        ON CONFLICT(job_kind) DO UPDATE SET
          owner_token = excluded.owner_token,
          acquired_at = excluded.acquired_at,
          expires_at = excluded.expires_at
        WHERE internal_job_lock.expires_at < excluded.acquired_at
        "#,
    )
    .bind(job_kind)
    .bind(owner_token)
    .bind(acquired_at)
    .bind(expires_at)
    .execute(pool)
    .await
    .context("failed to acquire postgres internal job lock")?;

    Ok(result.rows_affected() > 0)
}

pub(super) async fn release(
    pool: &PgPool,
    job_kind: &str,
    owner_token: &str,
) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        DELETE FROM internal_job_lock
        WHERE job_kind = $1
          AND owner_token = $2
        "#,
    )
    .bind(job_kind)
    .bind(owner_token)
    .execute(pool)
    .await
    .context("failed to release postgres internal job lock")?;

    Ok(())
}

pub(super) async fn fetch_active(
    pool: &PgPool,
    job_kind: &str,
    now_ts: &str,
) -> Result<Option<ActiveJobLock>, ApiError> {
    let row = sqlx::query(
        r#"
        SELECT owner_token, expires_at
        FROM internal_job_lock
        WHERE job_kind = $1
          AND expires_at > $2
        LIMIT 1
        "#,
    )
    .bind(job_kind)
    .bind(now_ts)
    .fetch_optional(pool)
    .await
    .context("failed to fetch postgres internal job lock status")?;

    let Some(row) = row else {
        return Ok(None);
    };

    Ok(Some(ActiveJobLock {
        owner_token: row
            .try_get("owner_token")
            .context("failed to parse postgres lock owner_token")?,
        expires_at: row
            .try_get("expires_at")
            .context("failed to parse postgres lock expires_at")?,
    }))
}
