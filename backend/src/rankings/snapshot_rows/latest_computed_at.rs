use anyhow::Context;
use sqlx::{Row, SqlitePool};

use crate::ApiError;

/// Finds the most recent ranking snapshot timestamp for the requested filter.
pub(crate) async fn fetch_latest_computed_at(
    pool: &SqlitePool,
    ranking_type: &str,
    timeframe: &str,
    temperature_bin: &str,
) -> Result<String, ApiError> {
    let latest_computed = sqlx::query(
        r#"
        SELECT MAX(computed_at) AS computed_at
        FROM cohort_ranking_snapshot
        WHERE ranking_type = ?
          AND timeframe = ?
          AND temperature_bin = ?
        "#,
    )
    .bind(ranking_type)
    .bind(timeframe)
    .bind(temperature_bin)
    .fetch_one(pool)
    .await
    .context("failed to query ranking snapshot timestamp")?
    .try_get::<Option<String>, _>("computed_at")
    .context("failed to parse ranking computed_at")?;

    latest_computed
        .ok_or_else(|| ApiError::not_found("no ranking snapshot available for this filter"))
}
