use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use sqlx::Row;
use sqlx::postgres::PgRow;

use crate::MetricCalc;

use super::range_efficiency_scoring::score_range_efficiency_series;
use super::range_efficiency_series::build_range_efficiency_series;
use super::range_efficiency_snapshots::RangeEfficiencyObservationRow;

/// Rebuilds range-efficiency KPIs from raw driving observations in Postgres.
pub(super) async fn compute_range_efficiency_metrics_postgres(
    pool: &PgPool,
    vehicle_uid: &str,
    cutoff: DateTime<Utc>,
) -> Result<Vec<MetricCalc>> {
    let rows = sqlx::query(
        r#"
        SELECT signal_key, value_number, observed_at
        FROM vehicle_signal_observation
        WHERE vehicle_uid = $1
          AND observed_at >= $2
          AND signal_key IN (
            'distance.odometer',
            'ev.soc_pct',
            'speed.vehicle',
            'ev.regen_power_kw',
            'ev.traction_power_kw'
          )
        ORDER BY observed_at ASC
        "#,
    )
    .bind(vehicle_uid)
    .bind(cutoff.to_rfc3339())
    .fetch_all(pool)
    .await
    .context("failed to fetch postgres observation rows for range-efficiency KPIs")?;

    let obs_rows = map_range_observation_rows(rows)?;
    score_range_metrics_from_rows(obs_rows)
}

fn score_range_metrics_from_rows(
    obs_rows: Vec<RangeEfficiencyObservationRow>,
) -> Result<Vec<MetricCalc>> {
    let default_usable_battery_kwh =
        crate::read_positive_env_f64("DEFAULT_USABLE_BATTERY_KWH", 75.0);
    let series = build_range_efficiency_series(obs_rows, default_usable_battery_kwh)?;
    Ok(score_range_efficiency_series(
        series,
        default_usable_battery_kwh,
    ))
}

fn map_range_observation_rows(rows: Vec<PgRow>) -> Result<Vec<RangeEfficiencyObservationRow>> {
    let mut mapped = Vec::with_capacity(rows.len());
    for row in rows {
        mapped.push(RangeEfficiencyObservationRow {
            signal_key: row
                .try_get::<String, _>("signal_key")
                .context("failed to parse signal_key for range metrics")?,
            value_number: row
                .try_get::<Option<f64>, _>("value_number")
                .context("failed to parse value_number for range metrics")?,
            observed_at: row
                .try_get::<String, _>("observed_at")
                .context("failed to parse observed_at for range metrics")?,
        });
    }
    Ok(mapped)
}
