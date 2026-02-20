use super::range_efficiency_series::RangeEfficiencySeries;
use super::range_efficiency_snapshots::RangeEfficiencySnapshot;

mod efficiency_points;
mod snapshot_state;
mod speed_segments;

/// Stateful accumulator that derives range-efficiency series from snapshots.
pub(super) struct RangeEfficiencyAccumulator {
    current_odo: Option<f64>,
    current_soc: Option<f64>,
    current_speed: Option<f64>,
    prev_filled: Option<(f64, f64, Option<f64>)>,
    km_per_soc_points: Vec<f64>,
    wh_per_km_points: Vec<f64>,
    urban_wh_per_km_points: Vec<f64>,
    highway_wh_per_km_points: Vec<f64>,
    power_windows: Vec<(i64, Option<f64>, Option<f64>)>,
    latest_soc: Option<f64>,
}

impl RangeEfficiencyAccumulator {
    /// Creates an empty accumulator for one ordered observation stream.
    pub(super) fn new() -> Self {
        Self {
            current_odo: None,
            current_soc: None,
            current_speed: None,
            prev_filled: None,
            km_per_soc_points: Vec::new(),
            wh_per_km_points: Vec::new(),
            urban_wh_per_km_points: Vec::new(),
            highway_wh_per_km_points: Vec::new(),
            power_windows: Vec::new(),
            latest_soc: None,
        }
    }

    /// Consumes one timestamp snapshot and updates derived series points.
    pub(super) fn observe_snapshot(
        &mut self,
        ts_seconds: i64,
        snapshot: &RangeEfficiencySnapshot,
        default_usable_battery_kwh: f64,
    ) {
        // Refresh the rolling sensor state from the sparse snapshot row.
        self.refresh_latest_snapshot_state(ts_seconds, snapshot);

        // Convert state deltas into efficiency points that downstream KPI scoring expects.
        self.capture_efficiency_points(default_usable_battery_kwh);
    }

    /// Finalizes accumulated points into the shared series container.
    pub(super) fn into_series(self) -> RangeEfficiencySeries {
        RangeEfficiencySeries {
            km_per_soc_points: self.km_per_soc_points,
            wh_per_km_points: self.wh_per_km_points,
            urban_wh_per_km_points: self.urban_wh_per_km_points,
            highway_wh_per_km_points: self.highway_wh_per_km_points,
            power_windows: self.power_windows,
            latest_soc: self.latest_soc,
        }
    }
}
