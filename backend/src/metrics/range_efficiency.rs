use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use crate::MetricCalc;

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
    let km_per_soc_points = series.km_per_soc_points;
    let wh_per_km_points = series.wh_per_km_points;
    let urban_wh_per_km_points = series.urban_wh_per_km_points;
    let highway_wh_per_km_points = series.highway_wh_per_km_points;
    let power_windows = series.power_windows;
    let latest_soc = series.latest_soc;

    let Some(median_km_per_soc) = super::median(km_per_soc_points.clone()) else {
        return Ok(Vec::new());
    };

    let mut metrics = Vec::new();
    let sample_count = km_per_soc_points.len() as i64;
    let soc_depletion_per_100km = if median_km_per_soc > 0.0 {
        100.0 / median_km_per_soc
    } else {
        100.0
    };
    let latest_soc = latest_soc.unwrap_or(50.0).clamp(0.0, 100.0);
    let estimated_range = (latest_soc * median_km_per_soc).max(0.0);

    let net_energy_efficiency = super::median(wh_per_km_points.clone())
        .unwrap_or((soc_depletion_per_100km * default_usable_battery_kwh / 10.0).max(0.0));
    let efficiency_component = (100.0 - (net_energy_efficiency / 3.0)).clamp(0.0, 100.0);
    let range_component = (estimated_range / 4.0).clamp(0.0, 100.0);
    let range_efficiency_score =
        (0.65 * efficiency_component + 0.35 * range_component).clamp(0.0, 100.0);

    metrics.push(MetricCalc {
        key: "ev_net_energy_efficiency",
        value: net_energy_efficiency,
        unit: "Wh_per_km",
        direction: "lower_is_better",
        sample_count,
        confidence_level: super::confidence_from_samples(sample_count),
    });

    metrics.push(MetricCalc {
        key: "ev_estimated_practical_range",
        value: estimated_range,
        unit: "km",
        direction: "higher_is_better",
        sample_count,
        confidence_level: super::confidence_from_samples(sample_count),
    });
    metrics.push(MetricCalc {
        key: "soc_depletion_rate_per_100km",
        value: soc_depletion_per_100km,
        unit: "%_per_100km",
        direction: "lower_is_better",
        sample_count,
        confidence_level: super::confidence_from_samples(sample_count),
    });
    metrics.push(MetricCalc {
        key: "ev_range_efficiency_score",
        value: range_efficiency_score,
        unit: "score",
        direction: "higher_is_better",
        sample_count,
        confidence_level: super::confidence_from_samples(sample_count),
    });

    if let Some(urban_efficiency) = super::median(urban_wh_per_km_points.clone()) {
        let urban_samples = urban_wh_per_km_points.len() as i64;
        metrics.push(MetricCalc {
            key: "ev_urban_efficiency",
            value: urban_efficiency,
            unit: "Wh_per_km",
            direction: "lower_is_better",
            sample_count: urban_samples,
            confidence_level: super::confidence_from_samples(urban_samples),
        });
    }

    if let Some(highway_efficiency) = super::median(highway_wh_per_km_points.clone()) {
        let highway_samples = highway_wh_per_km_points.len() as i64;
        metrics.push(MetricCalc {
            key: "ev_highway_efficiency",
            value: highway_efficiency,
            unit: "Wh_per_km",
            direction: "lower_is_better",
            sample_count: highway_samples,
            confidence_level: super::confidence_from_samples(highway_samples),
        });
    }

    let mut regen_wh = 0.0;
    let mut traction_wh = 0.0;
    let mut regen_windows = 0_i64;

    for window in power_windows.windows(2) {
        let dt_seconds = window[1].0 - window[0].0;
        if !(1..=300).contains(&dt_seconds) {
            continue;
        }

        let dt_hours = dt_seconds as f64 / 3600.0;
        let mut has_power_sample = false;

        if let Some(regen_kw) = window[0]
            .1
            .filter(|value| value.is_finite() && *value > 0.0)
        {
            regen_wh += regen_kw * dt_hours * 1000.0;
            has_power_sample = true;
        }
        if let Some(traction_kw) = window[0]
            .2
            .filter(|value| value.is_finite() && *value > 0.0)
        {
            traction_wh += traction_kw * dt_hours * 1000.0;
            has_power_sample = true;
        }

        if has_power_sample {
            regen_windows += 1;
        }
    }

    if regen_wh > 0.0 && (regen_wh + traction_wh) > 0.0 {
        let regen_ratio = (100.0 * regen_wh / (regen_wh + traction_wh)).clamp(0.0, 100.0);
        metrics.push(MetricCalc {
            key: "regeneration_recovery_ratio",
            value: regen_ratio,
            unit: "%",
            direction: "higher_is_better",
            sample_count: regen_windows.max(1),
            confidence_level: super::confidence_from_samples(regen_windows.max(1)),
        });
    }

    Ok(metrics)
}
