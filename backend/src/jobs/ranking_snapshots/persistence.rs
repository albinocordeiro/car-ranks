use anyhow::{Context, Result};
use sqlx::SqlitePool;
use uuid::Uuid;

/// Inserts one `cohort_ranking_snapshot` row.
///
/// Centralizing this SQL keeps schema-coupled writes consistent between
/// temperature and non-temperature ranking rebuilders.
pub(super) async fn insert_cohort_ranking_snapshot(
    pool: &SqlitePool,
    ranking_type: &str,
    timeframe: &str,
    temperature_bin: &str,
    cohort_key: &str,
    cohort_size: i64,
    sample_gate_passed: bool,
    vehicle_uid: &str,
    rank_position: i64,
    score: f64,
    confidence_level: &str,
    computed_at: &str,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO cohort_ranking_snapshot (
            ranking_snapshot_id,
            ranking_type,
            timeframe,
            temperature_bin,
            cohort_key,
            cohort_size,
            sample_gate_passed,
            vehicle_uid,
            rank_position,
            score,
            confidence_level,
            computed_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(ranking_type)
    .bind(timeframe)
    .bind(temperature_bin)
    .bind(cohort_key)
    .bind(cohort_size)
    .bind(i64::from(sample_gate_passed))
    .bind(vehicle_uid)
    .bind(rank_position)
    .bind(score)
    .bind(confidence_level)
    .bind(computed_at)
    .execute(pool)
    .await
    .context("failed to insert cohort ranking snapshot")?;

    Ok(())
}
