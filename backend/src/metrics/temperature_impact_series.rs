use std::collections::BTreeMap;

use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::Row;
use sqlx::sqlite::SqliteRow;

/// Normalized per-segment point used for temperature sensitivity regression.
#[derive(Debug)]
pub(super) struct VehiclePoint {
    pub(super) temperature_c: f64,
    pub(super) km_per_soc: f64,
}

/// Derived driving series used by temperature-impact KPI builders.
pub(super) struct TemperatureImpactDriveSeries {
    pub(super) points: Vec<VehiclePoint>,
    pub(super) cold_values: Vec<f64>,
    pub(super) mild_values: Vec<f64>,
    pub(super) cold_distance_km: f64,
    pub(super) mild_distance_km: f64,
}

/// Converts raw observation rows into aligned series for temperature KPIs.
pub(super) fn build_drive_series(obs_rows: Vec<SqliteRow>) -> Result<TemperatureImpactDriveSeries> {
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

    Ok(TemperatureImpactDriveSeries {
        points,
        cold_values,
        mild_values,
        cold_distance_km,
        mild_distance_km,
    })
}

/// Returns the linear-regression slope of `km_per_soc` over temperature.
pub(super) fn linear_regression_slope(points: &[VehiclePoint]) -> Option<f64> {
    if points.len() < 2 {
        return None;
    }

    let n = points.len() as f64;
    let mean_x = points.iter().map(|point| point.temperature_c).sum::<f64>() / n;
    let mean_y = points.iter().map(|point| point.km_per_soc).sum::<f64>() / n;

    let numerator = points
        .iter()
        .map(|point| (point.temperature_c - mean_x) * (point.km_per_soc - mean_y))
        .sum::<f64>();

    let denominator = points
        .iter()
        .map(|point| (point.temperature_c - mean_x).powi(2))
        .sum::<f64>();

    if denominator <= f64::EPSILON {
        return None;
    }

    Some(numerator / denominator)
}
