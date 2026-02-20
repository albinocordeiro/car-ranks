use std::sync::Arc;

use anyhow::{Context, Result};
use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use sqlx::sqlite::SqlitePoolOptions;

use super::*;

async fn job_status_test_state() -> Result<AppState> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .context("failed to connect in-memory sqlite")?;
    crate::migrations::apply_schema(&pool).await?;

    Ok(AppState {
        sqlite_pool: pool,
        pg_pool: None,
        backend: DatabaseBackend::Sqlite,
        signal_keys: Arc::new(load_signal_keys()?),
    })
}

#[tokio::test]
async fn latest_job_status_returns_recent_successful_run() -> Result<()> {
    let state = job_status_test_state().await?;

    let Json(job_response) = crate::handlers::post_recompute_kpis(State(state.clone()))
        .await
        .map_err(|err| anyhow::anyhow!("recompute job failed: {}", err.message))?;
    assert!(job_response.ok);

    let Json(status_response) = crate::handlers::get_latest_job_status(
        State(state),
        Query(JobStatusQuery {
            job_kind: Some("recompute_kpis".to_string()),
        }),
    )
    .await
    .map_err(|err| anyhow::anyhow!("latest job status failed: {}", err.message))?;

    assert_eq!(status_response.job_kind, "recompute_kpis");
    assert_eq!(status_response.backend, "sqlite");
    assert_eq!(status_response.status, "succeeded");
    assert_eq!(
        status_response.response_job_id.as_deref(),
        Some(job_response.job_id.as_str())
    );
    assert!(status_response.finished_at.is_some());
    assert!(status_response.active_lock_owner_token.is_none());
    assert!(status_response.active_lock_expires_at.is_none());

    Ok(())
}

#[tokio::test]
async fn latest_job_status_rejects_unsupported_job_kind() -> Result<()> {
    let state = job_status_test_state().await?;

    let err = crate::handlers::get_latest_job_status(
        State(state),
        Query(JobStatusQuery {
            job_kind: Some("unknown_kind".to_string()),
        }),
    )
    .await
    .expect_err("expected unsupported job_kind rejection");

    assert_eq!(err.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(err.error, "unprocessable_entity");
    assert!(err.message.contains("unsupported job_kind"));

    Ok(())
}

#[tokio::test]
async fn latest_job_status_returns_not_found_when_no_runs_exist() -> Result<()> {
    let state = job_status_test_state().await?;

    let err = crate::handlers::get_latest_job_status(
        State(state),
        Query(JobStatusQuery {
            job_kind: Some("recompute_kpis".to_string()),
        }),
    )
    .await
    .expect_err("expected not_found for empty job history");

    assert_eq!(err.status, StatusCode::NOT_FOUND);
    assert_eq!(err.error, "not_found");

    Ok(())
}

#[tokio::test]
async fn recompute_job_releases_lock_after_success() -> Result<()> {
    let state = job_status_test_state().await?;

    let Json(job_response) = crate::handlers::post_recompute_kpis(State(state.clone()))
        .await
        .map_err(|err| anyhow::anyhow!("recompute job failed: {}", err.message))?;
    assert!(job_response.ok);

    let lock_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM internal_job_lock
        WHERE job_kind = 'recompute_kpis'
        "#,
    )
    .fetch_one(&state.sqlite_pool)
    .await
    .context("failed to count internal job locks after run")?;
    assert_eq!(lock_count, 0);

    Ok(())
}

#[tokio::test]
async fn latest_job_status_reports_active_lock_metadata() -> Result<()> {
    let state = job_status_test_state().await?;

    let Json(job_response) = crate::handlers::post_recompute_kpis(State(state.clone()))
        .await
        .map_err(|err| anyhow::anyhow!("recompute job failed: {}", err.message))?;
    assert!(job_response.ok);

    let now = chrono::Utc::now();
    let expected_owner = "status-lock-owner";
    sqlx::query(
        r#"
        INSERT INTO internal_job_lock (
          job_kind,
          owner_token,
          acquired_at,
          expires_at
        ) VALUES (?, ?, ?, ?)
        "#,
    )
    .bind("recompute_kpis")
    .bind(expected_owner)
    .bind(now.to_rfc3339())
    .bind((now + chrono::Duration::minutes(5)).to_rfc3339())
    .execute(&state.sqlite_pool)
    .await
    .context("failed to seed active lock for status lookup")?;

    let Json(status_response) = crate::handlers::get_latest_job_status(
        State(state),
        Query(JobStatusQuery {
            job_kind: Some("recompute_kpis".to_string()),
        }),
    )
    .await
    .map_err(|err| anyhow::anyhow!("latest job status failed: {}", err.message))?;

    assert_eq!(
        status_response.active_lock_owner_token.as_deref(),
        Some(expected_owner)
    );
    assert!(status_response.active_lock_expires_at.is_some());

    Ok(())
}

#[tokio::test]
async fn recompute_job_rejects_when_active_lock_exists() -> Result<()> {
    let state = job_status_test_state().await?;
    let now = chrono::Utc::now();

    sqlx::query(
        r#"
        INSERT INTO internal_job_lock (
          job_kind,
          owner_token,
          acquired_at,
          expires_at
        ) VALUES (?, ?, ?, ?)
        "#,
    )
    .bind("recompute_kpis")
    .bind("preexisting-owner")
    .bind(now.to_rfc3339())
    .bind((now + chrono::Duration::minutes(5)).to_rfc3339())
    .execute(&state.sqlite_pool)
    .await
    .context("failed to seed active job lock")?;

    let err = crate::handlers::post_recompute_kpis(State(state))
        .await
        .expect_err("expected recompute job lock conflict");

    assert_eq!(err.status, StatusCode::CONFLICT);
    assert_eq!(err.error, "conflict");
    assert!(err.message.contains("already running"));

    Ok(())
}
