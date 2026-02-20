use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};

use crate::MetricCalc;

use super::temperature_impact_series::{build_drive_series, linear_regression_slope};

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
    let points = drive_series.points;
    let cold_values = drive_series.cold_values;
    let mild_values = drive_series.mild_values;
    let cold_distance_km = drive_series.cold_distance_km;
    let mild_distance_km = drive_series.mild_distance_km;

    let mut metrics = Vec::new();

    if gates.range_gate_passed(cold_distance_km, mild_distance_km) {
        let cold_median = super::median(cold_values.clone());
        let mild_median = super::median(mild_values.clone());
        if let (Some(cold), Some(mild)) = (cold_median, mild_median) {
            if mild > 0.0 {
                let retention = (100.0 * cold / mild).clamp(0.0, 200.0);
                let sample_count = cold_values.len().min(mild_values.len()) as i64;
                metrics.push(MetricCalc {
                    key: "cold_weather_range_retention",
                    value: retention,
                    unit: "%",
                    direction: "higher_is_better",
                    sample_count,
                    confidence_level: super::confidence_from_samples(sample_count),
                });

                if points.len() >= gates.min_sensitivity_points {
                    if let Some(slope) = linear_regression_slope(&points) {
                        let loss_pct_per_10c_drop = if slope < 0.0 {
                            ((-slope * 10.0) / mild) * 100.0
                        } else {
                            0.0
                        }
                        .clamp(0.0, 100.0);

                        metrics.push(MetricCalc {
                            key: "range_temperature_sensitivity_index",
                            value: loss_pct_per_10c_drop,
                            unit: "%_loss_per_10C_drop",
                            direction: "lower_is_better",
                            sample_count: points.len() as i64,
                            confidence_level: super::confidence_from_samples(points.len() as i64),
                        });
                    }
                }
            }
        }
    }

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

    let mut cold_charge = Vec::new();
    let mut mild_charge = Vec::new();

    for row in charge_rows {
        let power: Option<f64> = row.try_get("avg_charge_power_kw")?;
        let bin: Option<String> = row.try_get("temperature_bin")?;

        if let (Some(power), Some(bin)) = (power, bin) {
            if power <= 0.0 || !power.is_finite() {
                continue;
            }
            if bin == "cold" || bin == "very_cold" {
                cold_charge.push(power);
            }
            if bin == "mild" {
                mild_charge.push(power);
            }
        }
    }

    if gates.charge_gate_passed(cold_charge.len(), mild_charge.len()) {
        if let (Some(cold), Some(mild)) = (
            super::median(cold_charge.clone()),
            super::median(mild_charge.clone()),
        ) {
            if mild > 0.0 {
                let retention = (100.0 * cold / mild).clamp(0.0, 200.0);
                let sample_count = cold_charge.len().min(mild_charge.len()) as i64;
                metrics.push(MetricCalc {
                    key: "cold_weather_charge_speed_retention",
                    value: retention,
                    unit: "%",
                    direction: "higher_is_better",
                    sample_count,
                    confidence_level: super::confidence_from_samples(sample_count),
                });
            }
        }
    }

    Ok(metrics)
}
