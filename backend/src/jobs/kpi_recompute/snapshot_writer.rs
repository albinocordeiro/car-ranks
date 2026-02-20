use anyhow::{Context, Result};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::MetricCalc;

/// Validates and persists one locked KPI snapshot row.
///
/// The KPI lock check guarantees ranking rebuilds only consume approved formulas.
pub(super) async fn insert_kpi_snapshot(
    pool: &SqlitePool,
    vehicle_uid: &str,
    ranking_type: &str,
    timeframe: &str,
    metric: &MetricCalc,
    temperature_bin: &str,
    baseline_temperature_bin: Option<&str>,
    compare_temperature_bin: Option<&str>,
    snapshot_ts: &str,
) -> Result<()> {
    let Some((formula, required_signals, optional_signals)) =
        crate::kpi_specs::locked_kpi_spec_details(ranking_type, metric.key)
    else {
        return Err(anyhow::anyhow!(
            "kpi_key {} is not locked for ranking_type {}",
            metric.key,
            ranking_type
        ));
    };
    if metric.sample_count < 0 {
        return Err(anyhow::anyhow!(
            "kpi_key {} has invalid negative sample_count {}",
            metric.key,
            metric.sample_count
        ));
    }

    tracing::debug!(
        ranking_type,
        kpi_key = metric.key,
        formula,
        ?required_signals,
        ?optional_signals,
        "persisting locked KPI snapshot"
    );

    sqlx::query(
        r#"
        INSERT INTO vehicle_kpi_snapshot (
            snapshot_id,
            vehicle_uid,
            ranking_type,
            timeframe,
            kpi_key,
            kpi_value,
            kpi_unit,
            direction,
            confidence_level,
            sample_count,
            temperature_bin,
            baseline_temperature_bin,
            compare_temperature_bin,
            computed_at,
            source_job_id
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(vehicle_uid)
    .bind(ranking_type)
    .bind(timeframe)
    .bind(metric.key)
    .bind(metric.value)
    .bind(metric.unit)
    .bind(metric.direction)
    .bind(metric.confidence_level)
    .bind(metric.sample_count)
    .bind(temperature_bin)
    .bind(baseline_temperature_bin)
    .bind(compare_temperature_bin)
    .bind(snapshot_ts)
    .bind("internal_recompute")
    .execute(pool)
    .await
    .context("failed to insert KPI snapshot row")?;
    Ok(())
}
