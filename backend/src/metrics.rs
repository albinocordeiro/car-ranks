use std::collections::BTreeMap;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};

use crate::MetricCalc;

#[derive(Debug, Clone, Copy)]
pub(crate) struct TemperatureSampleGates {
    pub(crate) min_cold_distance_km: f64,
    pub(crate) min_mild_distance_km: f64,
    pub(crate) min_cold_charge_sessions: usize,
    pub(crate) min_mild_charge_sessions: usize,
    pub(crate) min_sensitivity_points: usize,
}

impl TemperatureSampleGates {
    pub(crate) fn range_gate_passed(self, cold_distance_km: f64, mild_distance_km: f64) -> bool {
        cold_distance_km >= self.min_cold_distance_km
            && mild_distance_km >= self.min_mild_distance_km
    }

    pub(crate) fn charge_gate_passed(self, cold_sessions: usize, mild_sessions: usize) -> bool {
        cold_sessions >= self.min_cold_charge_sessions
            && mild_sessions >= self.min_mild_charge_sessions
    }
}

#[derive(Debug)]
struct VehiclePoint {
    temperature_c: f64,
    km_per_soc: f64,
}

/// Score temperature-impact rankings from the three retained KPI signals.
/// The weighting keeps range retention as the dominant signal while still
/// accounting for charging retention and thermal sensitivity.
pub(crate) fn score_temperature_impact(
    range_retention: Option<f64>,
    charge_retention: Option<f64>,
    sensitivity: Option<f64>,
) -> f64 {
    let range = range_retention.unwrap_or(0.0).clamp(0.0, 200.0);
    let charge = charge_retention.unwrap_or(0.0).clamp(0.0, 200.0);
    let sensitivity = sensitivity.unwrap_or(50.0).clamp(0.0, 100.0);

    let sensitivity_component = (100.0 - (sensitivity * 2.0).clamp(0.0, 100.0)).clamp(0.0, 100.0);

    (0.45 * range + 0.35 * charge + 0.20 * sensitivity_component).clamp(0.0, 100.0)
}

/// Convert KPI snapshots into a single ranking score for non-temperature rankings.
pub(crate) fn score_from_kpi_map(ranking_type: &str, kpis: &BTreeMap<String, f64>) -> f64 {
    match ranking_type {
        "ev_range_efficiency" => kpis
            .get("ev_range_efficiency_score")
            .copied()
            .or_else(|| {
                let est = kpis.get("ev_estimated_practical_range").copied()?;
                let efficiency_component =
                    if let Some(net_eff) = kpis.get("ev_net_energy_efficiency").copied() {
                        (100.0 - (net_eff / 3.0)).clamp(0.0, 100.0)
                    } else {
                        let depletion = kpis
                            .get("soc_depletion_rate_per_100km")
                            .copied()
                            .unwrap_or(50.0);
                        (100.0 - depletion).clamp(0.0, 100.0)
                    };
                let range_component = (est / 4.0).clamp(0.0, 100.0);
                Some((0.65 * efficiency_component + 0.35 * range_component).clamp(0.0, 100.0))
            })
            .unwrap_or(0.0),
        "ev_charging_performance" => kpis
            .get("charging_performance_score")
            .copied()
            .or_else(|| {
                let acceptance = kpis
                    .get("temp_adjusted_charge_acceptance_score")
                    .copied()
                    .unwrap_or(0.0);
                let retention = kpis
                    .get("cold_weather_charge_speed_retention")
                    .copied()
                    .unwrap_or(acceptance);
                Some((0.6 * acceptance + 0.4 * retention).clamp(0.0, 100.0))
            })
            .unwrap_or(0.0),
        "ev_composite" => kpis
            .get("ev_composite_score")
            .copied()
            .unwrap_or(0.0)
            .clamp(0.0, 100.0),
        _ => 0.0,
    }
}

pub(crate) fn confidence_from_kpi_metrics(kpis: &[crate::KpiMetric]) -> &'static str {
    if kpis.is_empty() {
        return "preview";
    }
    if kpis.iter().any(|k| k.confidence_level == "preview") {
        "preview"
    } else if kpis.iter().any(|k| k.confidence_level == "medium") {
        "medium"
    } else {
        "stable"
    }
}

pub(crate) fn temperature_sample_gates() -> TemperatureSampleGates {
    let min_cold_charge_sessions =
        crate::read_positive_env("CAR_RANKS_TEMP_GATE_MIN_COLD_CHARGE_SESSIONS", 1);
    let min_mild_charge_sessions =
        crate::read_positive_env("CAR_RANKS_TEMP_GATE_MIN_MILD_CHARGE_SESSIONS", 1);
    let min_sensitivity_points =
        crate::read_positive_env("CAR_RANKS_TEMP_GATE_MIN_SENSITIVITY_POINTS", 6);

    TemperatureSampleGates {
        min_cold_distance_km: crate::read_positive_env_f64(
            "CAR_RANKS_TEMP_GATE_MIN_COLD_DISTANCE_KM",
            20.0,
        ),
        min_mild_distance_km: crate::read_positive_env_f64(
            "CAR_RANKS_TEMP_GATE_MIN_MILD_DISTANCE_KM",
            20.0,
        ),
        min_cold_charge_sessions: min_cold_charge_sessions as usize,
        min_mild_charge_sessions: min_mild_charge_sessions as usize,
        min_sensitivity_points: min_sensitivity_points as usize,
    }
}

pub(crate) fn wh_per_km_from_soc_delta(
    delta_soc_pct: f64,
    delta_km: f64,
    usable_battery_kwh: f64,
) -> Option<f64> {
    if !delta_soc_pct.is_finite()
        || !delta_km.is_finite()
        || !usable_battery_kwh.is_finite()
        || delta_soc_pct <= 0.0
        || delta_km <= 0.0
        || usable_battery_kwh <= 0.0
    {
        return None;
    }

    let energy_wh = (delta_soc_pct / 100.0) * usable_battery_kwh * 1000.0;
    let wh_per_km = energy_wh / delta_km;
    if wh_per_km.is_finite() && wh_per_km > 0.0 {
        Some(wh_per_km)
    } else {
        None
    }
}

pub(crate) fn mean(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    Some(values.iter().sum::<f64>() / values.len() as f64)
}

pub(crate) fn max_value(values: &[f64]) -> Option<f64> {
    values
        .iter()
        .copied()
        .filter(|v| v.is_finite())
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
}

pub(crate) fn median(mut values: Vec<f64>) -> Option<f64> {
    values.retain(|v| v.is_finite());
    if values.is_empty() {
        return None;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = values.len() / 2;
    if values.len() % 2 == 0 {
        Some((values[mid - 1] + values[mid]) / 2.0)
    } else {
        Some(values[mid])
    }
}

fn linear_regression_slope(points: &[VehiclePoint]) -> Option<f64> {
    if points.len() < 2 {
        return None;
    }

    let n = points.len() as f64;
    let mean_x = points.iter().map(|p| p.temperature_c).sum::<f64>() / n;
    let mean_y = points.iter().map(|p| p.km_per_soc).sum::<f64>() / n;

    let numerator = points
        .iter()
        .map(|p| (p.temperature_c - mean_x) * (p.km_per_soc - mean_y))
        .sum::<f64>();

    let denominator = points
        .iter()
        .map(|p| (p.temperature_c - mean_x).powi(2))
        .sum::<f64>();

    if denominator <= f64::EPSILON {
        return None;
    }

    Some(numerator / denominator)
}

pub(crate) fn confidence_from_samples(sample_count: i64) -> &'static str {
    if sample_count >= 60 {
        "stable"
    } else if sample_count >= 20 {
        "medium"
    } else {
        "preview"
    }
}

pub(crate) async fn compute_range_efficiency_metrics(
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

                    if let Some(wh_per_km) =
                        wh_per_km_from_soc_delta(delta_soc, delta_km, default_usable_battery_kwh)
                    {
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

    let Some(median_km_per_soc) = median(km_per_soc_points.clone()) else {
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

    let net_energy_efficiency = median(wh_per_km_points.clone())
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
        confidence_level: confidence_from_samples(sample_count),
    });

    metrics.push(MetricCalc {
        key: "ev_estimated_practical_range",
        value: estimated_range,
        unit: "km",
        direction: "higher_is_better",
        sample_count,
        confidence_level: confidence_from_samples(sample_count),
    });
    metrics.push(MetricCalc {
        key: "soc_depletion_rate_per_100km",
        value: soc_depletion_per_100km,
        unit: "%_per_100km",
        direction: "lower_is_better",
        sample_count,
        confidence_level: confidence_from_samples(sample_count),
    });
    metrics.push(MetricCalc {
        key: "ev_range_efficiency_score",
        value: range_efficiency_score,
        unit: "score",
        direction: "higher_is_better",
        sample_count,
        confidence_level: confidence_from_samples(sample_count),
    });

    if let Some(urban_efficiency) = median(urban_wh_per_km_points.clone()) {
        let urban_samples = urban_wh_per_km_points.len() as i64;
        metrics.push(MetricCalc {
            key: "ev_urban_efficiency",
            value: urban_efficiency,
            unit: "Wh_per_km",
            direction: "lower_is_better",
            sample_count: urban_samples,
            confidence_level: confidence_from_samples(urban_samples),
        });
    }

    if let Some(highway_efficiency) = median(highway_wh_per_km_points.clone()) {
        let highway_samples = highway_wh_per_km_points.len() as i64;
        metrics.push(MetricCalc {
            key: "ev_highway_efficiency",
            value: highway_efficiency,
            unit: "Wh_per_km",
            direction: "lower_is_better",
            sample_count: highway_samples,
            confidence_level: confidence_from_samples(highway_samples),
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
            confidence_level: confidence_from_samples(regen_windows.max(1)),
        });
    }

    Ok(metrics)
}

pub(crate) async fn compute_charging_performance_metrics(
    pool: &SqlitePool,
    vehicle_uid: &str,
    cutoff: DateTime<Utc>,
) -> Result<Vec<MetricCalc>> {
    let gates = temperature_sample_gates();

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
    .context("failed to fetch charging sessions for charging KPIs")?;

    let mut all_power = Vec::new();
    let mut cold_power = Vec::new();
    let mut mild_power = Vec::new();
    for row in charge_rows {
        let power: Option<f64> = row.try_get("avg_charge_power_kw")?;
        let bin: Option<String> = row.try_get("temperature_bin")?;
        if let (Some(p), Some(b)) = (power, bin) {
            if p <= 0.0 || !p.is_finite() {
                continue;
            }
            all_power.push(p);
            if b == "cold" || b == "very_cold" {
                cold_power.push(p);
            }
            if b == "mild" {
                mild_power.push(p);
            }
        }
    }

    if all_power.is_empty() {
        return Ok(Vec::new());
    }

    let mut metrics = Vec::new();
    let sample_count = all_power.len() as i64;
    let all_median = median(all_power.clone()).unwrap_or(0.0);
    let mild_median = median(mild_power.clone()).unwrap_or(all_median.max(1e-6));
    let acceptance_score = if mild_median > 0.0 {
        (100.0 * all_median / mild_median).clamp(0.0, 120.0)
    } else {
        100.0
    };

    metrics.push(MetricCalc {
        key: "temp_adjusted_charge_acceptance_score",
        value: acceptance_score,
        unit: "score",
        direction: "higher_is_better",
        sample_count,
        confidence_level: confidence_from_samples(sample_count),
    });

    if gates.charge_gate_passed(cold_power.len(), mild_power.len()) {
        if let (Some(cold_median), Some(mild_median)) =
            (median(cold_power.clone()), median(mild_power.clone()))
        {
            if mild_median > 0.0 {
                let retention = (100.0 * cold_median / mild_median).clamp(0.0, 200.0);
                let retention_samples = cold_power.len().min(mild_power.len()) as i64;
                metrics.push(MetricCalc {
                    key: "cold_weather_charge_speed_retention",
                    value: retention,
                    unit: "%",
                    direction: "higher_is_better",
                    sample_count: retention_samples,
                    confidence_level: confidence_from_samples(retention_samples),
                });
            }
        }
    }

    let charging_score = if let Some(retention_metric) = metrics
        .iter()
        .find(|m| m.key == "cold_weather_charge_speed_retention")
    {
        (0.6 * acceptance_score + 0.4 * retention_metric.value).clamp(0.0, 100.0)
    } else {
        acceptance_score.clamp(0.0, 100.0)
    };

    metrics.push(MetricCalc {
        key: "charging_performance_score",
        value: charging_score,
        unit: "score",
        direction: "higher_is_better",
        sample_count,
        confidence_level: confidence_from_samples(sample_count),
    });

    Ok(metrics)
}

pub(crate) async fn compute_composite_metrics(
    pool: &SqlitePool,
    vehicle_uid: &str,
    cutoff: DateTime<Utc>,
    range_metrics: &[MetricCalc],
    charging_metrics: &[MetricCalc],
) -> Result<Vec<MetricCalc>> {
    let range_score = range_metrics
        .iter()
        .find(|m| m.key == "ev_range_efficiency_score")
        .map(|m| m.value);
    let charging_score = charging_metrics
        .iter()
        .find(|m| m.key == "charging_performance_score")
        .map(|m| m.value);

    let Some(base_composite_score) = (match (range_score, charging_score) {
        (Some(r), Some(c)) => Some((0.6 * r + 0.4 * c).clamp(0.0, 100.0)),
        (Some(r), None) => Some(r.clamp(0.0, 100.0)),
        (None, Some(c)) => Some(c.clamp(0.0, 100.0)),
        (None, None) => None,
    }) else {
        return Ok(Vec::new());
    };

    let (health_penalty, health_sample_count) =
        compute_health_modifier_penalty(pool, vehicle_uid, cutoff).await?;
    let adjusted_score = (base_composite_score - health_penalty).clamp(0.0, 100.0);

    let sample_count = (range_metrics
        .iter()
        .find(|m| m.key == "ev_range_efficiency_score")
        .map(|m| m.sample_count)
        .unwrap_or(0))
    .max(
        charging_metrics
            .iter()
            .find(|m| m.key == "charging_performance_score")
            .map(|m| m.sample_count)
            .unwrap_or(0),
    )
    .max(health_sample_count);

    Ok(vec![
        MetricCalc {
            key: "ev_composite_base_score",
            value: base_composite_score,
            unit: "score",
            direction: "higher_is_better",
            sample_count,
            confidence_level: confidence_from_samples(sample_count),
        },
        MetricCalc {
            key: "ev_health_modifier_penalty",
            value: health_penalty,
            unit: "score_points",
            direction: "lower_is_better",
            sample_count: health_sample_count,
            confidence_level: confidence_from_samples(health_sample_count),
        },
        MetricCalc {
            key: "ev_composite_score",
            value: adjusted_score,
            unit: "score",
            direction: "higher_is_better",
            sample_count,
            confidence_level: confidence_from_samples(sample_count),
        },
    ])
}

pub(crate) async fn compute_health_modifier_penalty(
    pool: &SqlitePool,
    vehicle_uid: &str,
    cutoff: DateTime<Utc>,
) -> Result<(f64, i64)> {
    let dtc_row = sqlx::query(
        r#"
        SELECT COUNT(DISTINCT code) AS dtc_count
        FROM vehicle_diagnostic_event
        WHERE vehicle_uid = ?
          AND observed_at >= ?
          AND event_type = 'DTC_ACTIVE'
          AND code IS NOT NULL
        "#,
    )
    .bind(vehicle_uid)
    .bind(cutoff.to_rfc3339())
    .fetch_one(pool)
    .await
    .context("failed to compute active DTC count for health modifier")?;

    let dtc_count: i64 = dtc_row
        .try_get("dtc_count")
        .context("failed to parse active DTC count")?;

    let mil_row = sqlx::query(
        r#"
        SELECT event_type
        FROM vehicle_diagnostic_event
        WHERE vehicle_uid = ?
          AND observed_at >= ?
          AND event_type IN ('MIL_ON', 'MIL_OFF')
        ORDER BY observed_at DESC
        LIMIT 1
        "#,
    )
    .bind(vehicle_uid)
    .bind(cutoff.to_rfc3339())
    .fetch_optional(pool)
    .await
    .context("failed to load MIL status for health modifier")?;

    let mil_event_type = mil_row.and_then(|row| row.try_get::<String, _>("event_type").ok());
    let mil_on = mil_event_type
        .as_deref()
        .map(|event_type| event_type == "MIL_ON")
        .unwrap_or(false);

    let mil_penalty = if mil_on { 6.0 } else { 0.0 };
    let dtc_penalty = (dtc_count.max(0) as f64 * 0.5).min(4.0);
    let penalty = (mil_penalty + dtc_penalty).min(10.0);

    let sample_count = dtc_count.max(0) + if mil_event_type.is_some() { 1 } else { 0 };
    Ok((penalty, sample_count.max(1)))
}

pub(crate) async fn compute_vehicle_metrics(
    pool: &SqlitePool,
    vehicle_uid: &str,
    cutoff: DateTime<Utc>,
) -> Result<Vec<MetricCalc>> {
    let gates = temperature_sample_gates();

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

    #[derive(Default)]
    struct TimestampSnapshot {
        odo: Option<f64>,
        soc: Option<f64>,
        temp: Option<f64>,
    }

    let mut by_ts: BTreeMap<DateTime<Utc>, TimestampSnapshot> = BTreeMap::new();
    for row in obs_rows {
        let signal_key: String = row.try_get("signal_key")?;
        let value: Option<f64> = row.try_get("value_number")?;
        let observed_at: String = row.try_get("observed_at")?;
        let Some(ts) = crate::parse_ts(&observed_at) else {
            continue;
        };

        let snapshot = by_ts.entry(ts).or_default();
        match (signal_key.as_str(), value) {
            ("distance.odometer", Some(v)) => snapshot.odo = Some(v),
            ("ev.soc_pct", Some(v)) => snapshot.soc = Some(v),
            ("environment.ambient_temp_c", Some(v)) => snapshot.temp = Some(v),
            _ => {}
        }
    }

    let mut current_odo: Option<f64> = None;
    let mut current_soc: Option<f64> = None;
    let mut current_temp: Option<f64> = None;
    let mut prev_filled: Option<(f64, f64, f64)> = None;
    let mut points = Vec::new();
    let mut cold_values = Vec::new();
    let mut mild_values = Vec::new();
    let mut cold_distance_km = 0.0;
    let mut mild_distance_km = 0.0;

    for snapshot in by_ts.values() {
        if snapshot.odo.is_some() {
            current_odo = snapshot.odo;
        }
        if snapshot.soc.is_some() {
            current_soc = snapshot.soc;
        }
        if snapshot.temp.is_some() {
            current_temp = snapshot.temp;
        }

        if let (Some(odo), Some(soc), Some(temp)) = (current_odo, current_soc, current_temp) {
            if let Some((prev_odo, prev_soc, _prev_temp)) = prev_filled {
                let delta_km = odo - prev_odo;
                let delta_soc = prev_soc - soc;
                if delta_km > 0.05 && delta_soc > 0.05 && delta_soc < 30.0 {
                    let km_per_soc = delta_km / delta_soc;
                    if km_per_soc.is_finite() && km_per_soc > 0.0 && km_per_soc < 50.0 {
                        points.push(VehiclePoint {
                            temperature_c: temp,
                            km_per_soc,
                        });
                        if temp <= 5.0 {
                            cold_values.push(km_per_soc);
                            cold_distance_km += delta_km;
                        }
                        if temp > 15.0 && temp <= 25.0 {
                            mild_values.push(km_per_soc);
                            mild_distance_km += delta_km;
                        }
                    }
                }
            }
            prev_filled = Some((odo, soc, temp));
        }
    }

    let mut metrics = Vec::new();

    if gates.range_gate_passed(cold_distance_km, mild_distance_km) {
        let cold_median = median(cold_values.clone());
        let mild_median = median(mild_values.clone());
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
                    confidence_level: confidence_from_samples(sample_count),
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
                            confidence_level: confidence_from_samples(points.len() as i64),
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
        if let (Some(cold), Some(mild)) = (median(cold_charge.clone()), median(mild_charge.clone()))
        {
            if mild > 0.0 {
                let retention = (100.0 * cold / mild).clamp(0.0, 200.0);
                let sample_count = cold_charge.len().min(mild_charge.len()) as i64;
                metrics.push(MetricCalc {
                    key: "cold_weather_charge_speed_retention",
                    value: retention,
                    unit: "%",
                    direction: "higher_is_better",
                    sample_count,
                    confidence_level: confidence_from_samples(sample_count),
                });
            }
        }
    }

    Ok(metrics)
}
