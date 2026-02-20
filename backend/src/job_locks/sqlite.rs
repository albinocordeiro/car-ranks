use anyhow::Context;
use sqlx::SqlitePool;

use crate::ApiError;

pub(super) async fn try_acquire(
    pool: &SqlitePool,
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
        ) VALUES (?, ?, ?, ?)
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
    .context("failed to acquire sqlite internal job lock")?;

    Ok(result.rows_affected() > 0)
}

pub(super) async fn release(
    pool: &SqlitePool,
    job_kind: &str,
    owner_token: &str,
) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        DELETE FROM internal_job_lock
        WHERE job_kind = ?
          AND owner_token = ?
        "#,
    )
    .bind(job_kind)
    .bind(owner_token)
    .execute(pool)
    .await
    .context("failed to release sqlite internal job lock")?;

    Ok(())
}
