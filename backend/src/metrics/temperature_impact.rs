use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use crate::MetricCalc;

use super::temperature_impact_scoring::{
    score_charge_retention_metric, score_drive_metrics, split_charge_power_by_temperature_bin,
};
use super::temperature_impact_series::build_drive_series;

/// Rebuilds temperature-impact KPIs from driving and charging observations.
pub(super) async fn compute_vehicle_metrics(
    pool: &SqlitePool,
    vehicle_uid: &str,
    cutoff: DateTime<Utc>,
) -> Result<Vec<MetricCalc>> {
    let gates = super::temperature_sample_gates();

    let obs_rows = sqlx::query(
        r#"
        SELECT signal_key, value_number, observed_at
        FROM vehicle_signal_observation
        WHERE vehicle_uid = ?
          AND observed_at >= ?
          AND signal_key IN ('distance.odometer', 'ev.soc_pct', 'environment.ambient_temp_c')
        ORDER BY observed_at ASC
        "#,
    )
    .bind(vehicle_uid)
    .bind(cutoff.to_rfc3339())
    .fetch_all(pool)
    .await
    .context("failed to fetch observation rows for KPI computation")?;

    let drive_series = build_drive_series(obs_rows)?;
    let mut metrics = score_drive_metrics(drive_series, gates);

    let charge_rows = sqlx::query(
        r#"
        SELECT avg_charge_power_kw, temperature_bin
        FROM vehicle_charging_session
        WHERE vehicle_uid = ?
          AND started_at >= ?
          AND avg_charge_power_kw IS NOT NULL
        "#,
    )
    .bind(vehicle_uid)
    .bind(cutoff.to_rfc3339())
    .fetch_all(pool)
    .await
    .context("failed to fetch charging sessions for KPI computation")?;

    let (cold_charge, mild_charge) = split_charge_power_by_temperature_bin(charge_rows)?;
    if let Some(metric) = score_charge_retention_metric(cold_charge, mild_charge, gates) {
        metrics.push(metric);
    }

    Ok(metrics)
}
