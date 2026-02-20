use anyhow::Context;
use sqlx::{Row, SqlitePool};

use crate::{ApiError, JobResponse, JobRunStatusResponse};

pub(super) async fn insert_started(
    pool: &SqlitePool,
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
        ) VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(job_run_id)
    .bind(job_kind)
    .bind(backend)
    .bind("running")
    .bind(started_at)
    .execute(pool)
    .await
    .context("failed to insert sqlite internal job run start")?;

    Ok(())
}

pub(super) async fn mark_succeeded(
    pool: &SqlitePool,
    job_run_id: &str,
    finished_at: &str,
    response: &JobResponse,
) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        UPDATE internal_job_run
        SET
          status = ?,
          finished_at = ?,
          response_job_id = ?,
          charging_sessions_upserted = ?,
          kpi_rows_upserted = ?,
          ranking_rows_upserted = ?,
          recomputed_vehicles = ?
        WHERE job_run_id = ?
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
    .context("failed to mark sqlite internal job run as succeeded")?;

    Ok(())
}

pub(super) async fn mark_failed(
    pool: &SqlitePool,
    job_run_id: &str,
    finished_at: &str,
    error_message: &str,
) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        UPDATE internal_job_run
        SET
          status = ?,
          finished_at = ?,
          error_message = ?
        WHERE job_run_id = ?
        "#,
    )
    .bind("failed")
    .bind(finished_at)
    .bind(error_message)
    .bind(job_run_id)
    .execute(pool)
    .await
    .context("failed to mark sqlite internal job run as failed")?;

    Ok(())
}

pub(super) async fn fetch_latest(
    pool: &SqlitePool,
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
        WHERE job_kind = ?
        ORDER BY started_at DESC
        LIMIT 1
        "#,
    )
    .bind(job_kind)
    .fetch_optional(pool)
    .await
    .context("failed to fetch latest sqlite internal job run")?;

    let Some(row) = row else {
        return Ok(None);
    };

    Ok(Some(JobRunStatusResponse {
        job_run_id: row
            .try_get("job_run_id")
            .context("failed to parse sqlite job_run_id")?,
        job_kind: row
            .try_get("job_kind")
            .context("failed to parse sqlite job_kind")?,
        backend: row
            .try_get("backend")
            .context("failed to parse sqlite backend")?,
        status: row
            .try_get("status")
            .context("failed to parse sqlite status")?,
        started_at: row
            .try_get("started_at")
            .context("failed to parse sqlite started_at")?,
        finished_at: row
            .try_get("finished_at")
            .context("failed to parse sqlite finished_at")?,
        error_message: row
            .try_get("error_message")
            .context("failed to parse sqlite error_message")?,
        response_job_id: row
            .try_get("response_job_id")
            .context("failed to parse sqlite response_job_id")?,
        charging_sessions_upserted: row
            .try_get("charging_sessions_upserted")
            .context("failed to parse sqlite charging_sessions_upserted")?,
        kpi_rows_upserted: row
            .try_get("kpi_rows_upserted")
            .context("failed to parse sqlite kpi_rows_upserted")?,
        ranking_rows_upserted: row
            .try_get("ranking_rows_upserted")
            .context("failed to parse sqlite ranking_rows_upserted")?,
        recomputed_vehicles: row
            .try_get("recomputed_vehicles")
            .context("failed to parse sqlite recomputed_vehicles")?,
        active_lock_owner_token: None,
        active_lock_expires_at: None,
    }))
}
