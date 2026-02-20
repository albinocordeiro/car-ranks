use anyhow::Context;
use sqlx::PgPool;

use crate::ApiError;

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
