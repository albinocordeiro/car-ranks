use std::collections::BTreeMap;

use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::Row;
use sqlx::sqlite::SqliteRow;

/// Intermediate series derived from raw observations before KPI scoring.
pub(super) struct RangeEfficiencySeries {
    pub(super) km_per_soc_points: Vec<f64>,
    pub(super) wh_per_km_points: Vec<f64>,
    pub(super) urban_wh_per_km_points: Vec<f64>,
    pub(super) highway_wh_per_km_points: Vec<f64>,
    pub(super) power_windows: Vec<(i64, Option<f64>, Option<f64>)>,
    pub(super) latest_soc: Option<f64>,
}

/// Normalizes raw observation rows into aligned series used by KPI builders.
pub(super) fn build_range_efficiency_series(
    obs_rows: Vec<SqliteRow>,
    default_usable_battery_kwh: f64,
) -> Result<RangeEfficiencySeries> {
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

    Ok(RangeEfficiencySeries {
        km_per_soc_points,
        wh_per_km_points,
        urban_wh_per_km_points,
        highway_wh_per_km_points,
        power_windows,
        latest_soc,
    })
}
