use super::range_efficiency_series::RangeEfficiencySeries;
use super::range_efficiency_snapshots::RangeEfficiencySnapshot;

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
        if snapshot.odo.is_some() {
            self.current_odo = snapshot.odo;
        }
        if snapshot.soc.is_some() {
            self.current_soc = snapshot.soc;
        }
        if snapshot.speed.is_some() {
            self.current_speed = snapshot.speed;
        }
        if snapshot.regen_power_kw.is_some() || snapshot.traction_power_kw.is_some() {
            self.power_windows.push((
                ts_seconds,
                snapshot.regen_power_kw,
                snapshot.traction_power_kw,
            ));
        }
        self.latest_soc = self.current_soc;

        if let (Some(odo), Some(soc)) = (self.current_odo, self.current_soc) {
            if let Some((prev_odo, prev_soc, prev_speed)) = self.prev_filled {
                let delta_km = odo - prev_odo;
                let delta_soc = prev_soc - soc;
                if delta_km > 0.05 && delta_soc > 0.05 && delta_soc < 30.0 {
                    let km_per_soc = delta_km / delta_soc;
                    if km_per_soc.is_finite() && km_per_soc > 0.0 && km_per_soc < 50.0 {
                        self.km_per_soc_points.push(km_per_soc);
                    }

                    if let Some(wh_per_km) = super::wh_per_km_from_soc_delta(
                        delta_soc,
                        delta_km,
                        default_usable_battery_kwh,
                    ) {
                        self.wh_per_km_points.push(wh_per_km);
                        if let Some(segment_speed) = self.current_speed.or(prev_speed) {
                            self.push_speed_segment_efficiency(segment_speed, wh_per_km);
                        }
                    }
                }
            }
            self.prev_filled = Some((odo, soc, self.current_speed));
        }
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

    fn push_speed_segment_efficiency(&mut self, segment_speed: f64, wh_per_km: f64) {
        if segment_speed < 45.0 {
            self.urban_wh_per_km_points.push(wh_per_km);
        }
        if segment_speed >= 80.0 {
            self.highway_wh_per_km_points.push(wh_per_km);
        }
    }
}
