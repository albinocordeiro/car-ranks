use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use crate::MetricCalc;

use super::range_efficiency_scoring::score_range_efficiency_series;
use super::range_efficiency_series::build_range_efficiency_series;

/// Rebuilds range-efficiency KPIs from raw driving observations.
pub(super) async fn compute_range_efficiency_metrics(
    pool: &SqlitePool,
    vehicle_uid: &str,
    cutoff: DateTime<Utc>,
) -> Result<Vec<MetricCalc>> {
    let obs_rows = sqlx::query(
        r#"
        SELECT signal_key, value_number, observed_at
        FROM vehicle_signal_observation
        WHERE vehicle_uid = ?
          AND observed_at >= ?
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
    .context("failed to fetch observation rows for range-efficiency KPIs")?;

    let default_usable_battery_kwh =
        crate::read_positive_env_f64("DEFAULT_USABLE_BATTERY_KWH", 75.0);
    let series = build_range_efficiency_series(obs_rows, default_usable_battery_kwh)?;
    Ok(score_range_efficiency_series(
        series,
        default_usable_battery_kwh,
    ))
}
