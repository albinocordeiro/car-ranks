use anyhow::Context;
use sqlx::{PgPool, Row};

use crate::{ApiError, JobResponse, JobRunStatusResponse};

pub(super) async fn insert_started(
    pool: &PgPool,
    job_run_id: &str,
    job_kind: &str,
    backend: &str,
    started_at: &str,
) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        INSERT INTO internal_job_run (
            job_run_id,
            job_kind,
            backend,
            status,
            started_at
        ) VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(job_run_id)
    .bind(job_kind)
    .bind(backend)
    .bind("running")
    .bind(started_at)
    .execute(pool)
    .await
    .context("failed to insert postgres internal job run start")?;

    Ok(())
}

pub(super) async fn mark_succeeded(
    pool: &PgPool,
    job_run_id: &str,
    finished_at: &str,
    response: &JobResponse,
) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        UPDATE internal_job_run
        SET
          status = $1,
          finished_at = $2,
          response_job_id = $3,
          charging_sessions_upserted = $4,
          kpi_rows_upserted = $5,
          ranking_rows_upserted = $6,
          recomputed_vehicles = $7
        WHERE job_run_id = $8
        "#,
    )
    .bind("succeeded")
    .bind(finished_at)
    .bind(&response.job_id)
    .bind(response.charging_sessions_upserted as i64)
    .bind(response.kpi_rows_upserted as i64)
    .bind(response.ranking_rows_upserted as i64)
    .bind(response.recomputed_vehicles as i64)
    .bind(job_run_id)
    .execute(pool)
    .await
    .context("failed to mark postgres internal job run as succeeded")?;

    Ok(())
}

pub(super) async fn mark_failed(
    pool: &PgPool,
    job_run_id: &str,
    finished_at: &str,
    error_message: &str,
) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        UPDATE internal_job_run
        SET
          status = $1,
          finished_at = $2,
          error_message = $3
        WHERE job_run_id = $4
        "#,
    )
    .bind("failed")
    .bind(finished_at)
    .bind(error_message)
    .bind(job_run_id)
    .execute(pool)
    .await
    .context("failed to mark postgres internal job run as failed")?;

    Ok(())
}

pub(super) async fn fetch_latest(
    pool: &PgPool,
    job_kind: &str,
) -> Result<Option<JobRunStatusResponse>, ApiError> {
    let row = sqlx::query(
        r#"
        SELECT
          job_run_id,
          job_kind,
          backend,
          status,
          started_at,
          finished_at,
          error_message,
          response_job_id,
          charging_sessions_upserted,
          kpi_rows_upserted,
          ranking_rows_upserted,
          recomputed_vehicles
        FROM internal_job_run
        WHERE job_kind = $1
        ORDER BY started_at DESC
        LIMIT 1
        "#,
    )
    .bind(job_kind)
    .fetch_optional(pool)
    .await
    .context("failed to fetch latest postgres internal job run")?;

    let Some(row) = row else {
        return Ok(None);
    };

    Ok(Some(JobRunStatusResponse {
        job_run_id: row
            .try_get("job_run_id")
            .context("failed to parse postgres job_run_id")?,
        job_kind: row
            .try_get("job_kind")
            .context("failed to parse postgres job_kind")?,
        backend: row
            .try_get("backend")
            .context("failed to parse postgres backend")?,
        status: row
            .try_get("status")
            .context("failed to parse postgres status")?,
        started_at: row
            .try_get("started_at")
            .context("failed to parse postgres started_at")?,
        finished_at: row
            .try_get("finished_at")
            .context("failed to parse postgres finished_at")?,
        error_message: row
            .try_get("error_message")
            .context("failed to parse postgres error_message")?,
        response_job_id: row
            .try_get("response_job_id")
            .context("failed to parse postgres response_job_id")?,
        charging_sessions_upserted: row
            .try_get("charging_sessions_upserted")
            .context("failed to parse postgres charging_sessions_upserted")?,
        kpi_rows_upserted: row
            .try_get("kpi_rows_upserted")
            .context("failed to parse postgres kpi_rows_upserted")?,
        ranking_rows_upserted: row
            .try_get("ranking_rows_upserted")
            .context("failed to parse postgres ranking_rows_upserted")?,
        recomputed_vehicles: row
            .try_get("recomputed_vehicles")
            .context("failed to parse postgres recomputed_vehicles")?,
        active_lock_owner_token: None,
        active_lock_expires_at: None,
    }))
}
