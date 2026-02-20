use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use sqlx::Row;
use sqlx::postgres::PgRow;

use crate::MetricCalc;

use super::temperature_charge_retention::{
    ChargingPowerSampleRow, score_charge_retention_metric, split_charge_power_by_temperature_bin,
};
use super::temperature_impact_scoring::score_drive_metrics;
use super::temperature_impact_series::build_drive_series;
use super::temperature_impact_snapshots::TemperatureObservationRow;

/// Rebuilds temperature-impact KPIs from driving and charging observations in Postgres.
pub(super) async fn compute_vehicle_metrics_postgres(
    pool: &PgPool,
    vehicle_uid: &str,
    cutoff: DateTime<Utc>,
) -> Result<Vec<MetricCalc>> {
    let gates = super::temperature_sample_gates();

    let obs_rows_raw = sqlx::query(
        r#"
        SELECT
          signal_key,
          value_number::double precision AS value_number,
          observed_at
        FROM vehicle_signal_observation
        WHERE vehicle_uid = $1
          AND observed_at >= $2
          AND signal_key IN ('distance.odometer', 'ev.soc_pct', 'environment.ambient_temp_c')
        ORDER BY observed_at ASC
        "#,
    )
    .bind(vehicle_uid)
    .bind(cutoff.to_rfc3339())
    .fetch_all(pool)
    .await
    .context("failed to fetch postgres observation rows for KPI computation")?;

    let obs_rows = map_temperature_observation_rows(obs_rows_raw)?;
    let drive_series = build_drive_series(obs_rows)?;
    let mut metrics = score_drive_metrics(drive_series, gates);

    let charge_rows_raw = sqlx::query(
        r#"
        SELECT
          avg_charge_power_kw::double precision AS avg_charge_power_kw,
          temperature_bin
        FROM vehicle_charging_session
        WHERE vehicle_uid = $1
          AND started_at >= $2
          AND avg_charge_power_kw IS NOT NULL
        "#,
    )
    .bind(vehicle_uid)
    .bind(cutoff.to_rfc3339())
    .fetch_all(pool)
    .await
    .context("failed to fetch postgres charging sessions for KPI computation")?;

    let charge_rows = map_charge_power_rows(charge_rows_raw)?;
    let (cold_charge, mild_charge) = split_charge_power_by_temperature_bin(charge_rows)?;
    if let Some(metric) = score_charge_retention_metric(cold_charge, mild_charge, gates) {
        metrics.push(metric);
    }

    Ok(metrics)
}

fn map_temperature_observation_rows(rows: Vec<PgRow>) -> Result<Vec<TemperatureObservationRow>> {
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(TemperatureObservationRow {
            signal_key: row
                .try_get::<String, _>("signal_key")
                .context("failed to parse signal_key for temperature metrics")?,
            value_number: row
                .try_get::<Option<f64>, _>("value_number")
                .context("failed to parse value_number for temperature metrics")?,
            observed_at: row
                .try_get::<String, _>("observed_at")
                .context("failed to parse observed_at for temperature metrics")?,
        });
    }
    Ok(out)
}

fn map_charge_power_rows(rows: Vec<PgRow>) -> Result<Vec<ChargingPowerSampleRow>> {
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(ChargingPowerSampleRow {
            avg_charge_power_kw: row
                .try_get::<Option<f64>, _>("avg_charge_power_kw")
                .context("failed to parse avg_charge_power_kw for temperature metrics")?,
            temperature_bin: row
                .try_get::<Option<String>, _>("temperature_bin")
                .context("failed to parse temperature_bin for temperature metrics")?,
        });
    }
    Ok(out)
}
