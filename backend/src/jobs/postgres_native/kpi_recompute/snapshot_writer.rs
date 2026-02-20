use anyhow::{Context, Result};
use sqlx::PgPool;
use uuid::Uuid;

use crate::MetricCalc;

/// Persists one native Postgres KPI snapshot row.
pub(super) async fn insert_native_kpi_snapshot_postgres(
    pool: &PgPool,
    ranking_type: &str,
    vehicle_uid: &str,
    timeframe: &str,
    metric: &MetricCalc,
    snapshot_ts: &str,
) -> Result<()> {
    if crate::kpi_specs::locked_kpi_spec_details(ranking_type, metric.key).is_none() {
        return Err(anyhow::anyhow!(
            "kpi_key {} is not locked for ranking_type {}",
            metric.key,
            ranking_type
        ));
    }
    if metric.sample_count < 0 {
        return Err(anyhow::anyhow!(
            "kpi_key {} has invalid negative sample_count {}",
            metric.key,
            metric.sample_count
        ));
    }

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
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
            $11, $12, $13, $14, $15
        )
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
    .bind("all")
    .bind(None::<String>)
    .bind(None::<String>)
    .bind(snapshot_ts)
    .bind("internal_recompute")
    .execute(pool)
    .await
    .with_context(|| {
        format!(
            "failed to insert native postgres KPI snapshot for {}",
            ranking_type
        )
    })?;

    Ok(())
}
