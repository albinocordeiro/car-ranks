use anyhow::Result;
use sqlx::sqlite::SqliteRow;

use super::range_efficiency_accumulator::RangeEfficiencyAccumulator;
use super::range_efficiency_snapshots::normalize_range_efficiency_snapshots;

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
    let by_ts = normalize_range_efficiency_snapshots(obs_rows)?;
    let mut accumulator = RangeEfficiencyAccumulator::new();
    for (ts, snapshot) in &by_ts {
        accumulator.observe_snapshot(ts.timestamp(), snapshot, default_usable_battery_kwh);
    }

    Ok(accumulator.into_series())
}
