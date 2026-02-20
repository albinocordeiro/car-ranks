use std::collections::BTreeMap;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};

use crate::MetricCalc;

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

    #[derive(Default)]
    struct Snapshot {
        odo: Option<f64>,
        soc: Option<f64>,
        speed: Option<f64>,
        regen_power_kw: Option<f64>,
        traction_power_kw: Option<f64>,
    }

    let mut by_ts: BTreeMap<DateTime<Utc>, Snapshot> = BTreeMap::new();
    for row in obs_rows {
        let signal_key: String = row.try_get("signal_key")?;
        let value: Option<f64> = row.try_get("value_number")?;
        let observed_at: String = row.try_get("observed_at")?;
        let Some(ts) = crate::parse_ts(&observed_at) else {
            continue;
        };
        let entry = by_ts.entry(ts).or_default();
        match (signal_key.as_str(), value) {
            ("distance.odometer", Some(v)) => entry.odo = Some(v),
            ("ev.soc_pct", Some(v)) => entry.soc = Some(v),
            ("speed.vehicle", Some(v)) => entry.speed = Some(v),
            ("ev.regen_power_kw", Some(v)) => entry.regen_power_kw = Some(v),
            ("ev.traction_power_kw", Some(v)) => entry.traction_power_kw = Some(v),
            _ => {}
        }
    }

    let default_usable_battery_kwh =
        crate::read_positive_env_f64("DEFAULT_USABLE_BATTERY_KWH", 75.0);

    let mut current_odo: Option<f64> = None;
    let mut current_soc: Option<f64> = None;
    let mut current_speed: Option<f64> = None;
    let mut prev_filled: Option<(f64, f64, Option<f64>)> = None;

    let mut km_per_soc_points = Vec::new();
    let mut wh_per_km_points = Vec::new();
    let mut urban_wh_per_km_points = Vec::new();
    let mut highway_wh_per_km_points = Vec::new();
    let mut power_windows: Vec<(i64, Option<f64>, Option<f64>)> = Vec::new();
    let mut latest_soc: Option<f64> = None;

    for (ts, snapshot) in &by_ts {
        if snapshot.odo.is_some() {
            current_odo = snapshot.odo;
        }
        if snapshot.soc.is_some() {
            current_soc = snapshot.soc;
        }
        if snapshot.speed.is_some() {
            current_speed = snapshot.speed;
        }
        if snapshot.regen_power_kw.is_some() || snapshot.traction_power_kw.is_some() {
            power_windows.push((
                ts.timestamp(),
                snapshot.regen_power_kw,
                snapshot.traction_power_kw,
            ));
        }
        latest_soc = current_soc;

        if let (Some(odo), Some(soc)) = (current_odo, current_soc) {
            if let Some((prev_odo, prev_soc, prev_speed)) = prev_filled {
                let delta_km = odo - prev_odo;
                let delta_soc = prev_soc - soc;
                if delta_km > 0.05 && delta_soc > 0.05 && delta_soc < 30.0 {
                    let km_per_soc = delta_km / delta_soc;
                    if km_per_soc.is_finite() && km_per_soc > 0.0 && km_per_soc < 50.0 {
                        km_per_soc_points.push(km_per_soc);
                    }

                    if let Some(wh_per_km) = super::wh_per_km_from_soc_delta(
                        delta_soc,
                        delta_km,
                        default_usable_battery_kwh,
                    ) {
                        wh_per_km_points.push(wh_per_km);
                        if let Some(segment_speed) = current_speed.or(prev_speed) {
                            if segment_speed < 45.0 {
                                urban_wh_per_km_points.push(wh_per_km);
                            }
                            if segment_speed >= 80.0 {
                                highway_wh_per_km_points.push(wh_per_km);
                            }
                        }
                    }
                }
            }
            prev_filled = Some((odo, soc, current_speed));
        }
    }

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
