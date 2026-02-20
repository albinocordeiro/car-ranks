use super::super::wh_per_km_from_soc_delta;
use super::RangeEfficiencyAccumulator;

impl RangeEfficiencyAccumulator {
    /// Converts the latest and previous fully-observed state into KPI point samples.
    ///
    /// Each sample requires both odometer and SOC readings. The delta gates
    /// intentionally drop tiny jitter and implausible jumps before scoring.
    pub(super) fn capture_efficiency_points(&mut self, default_usable_battery_kwh: f64) {
        if let (Some(odo), Some(soc)) = (self.current_odo, self.current_soc) {
            if let Some((prev_odo, prev_soc, prev_speed)) = self.prev_filled {
                let delta_km = odo - prev_odo;
                let delta_soc = prev_soc - soc;
                if self.accept_delta_window(delta_km, delta_soc) {
                    let km_per_soc = delta_km / delta_soc;
                    if self.accept_km_per_soc(km_per_soc) {
                        self.km_per_soc_points.push(km_per_soc);
                    }

                    if let Some(wh_per_km) =
                        wh_per_km_from_soc_delta(delta_soc, delta_km, default_usable_battery_kwh)
                    {
                        self.wh_per_km_points.push(wh_per_km);
                        if let Some(segment_speed) = self.current_speed.or(prev_speed) {
                            self.push_speed_segment_efficiency(segment_speed, wh_per_km);
                        }
                    }
                }
            }

            // Save the newest complete state so the next snapshot can form a delta pair.
            self.prev_filled = Some((odo, soc, self.current_speed));
        }
    }

    fn accept_delta_window(&self, delta_km: f64, delta_soc: f64) -> bool {
        delta_km > 0.05 && delta_soc > 0.05 && delta_soc < 30.0
    }

    fn accept_km_per_soc(&self, km_per_soc: f64) -> bool {
        km_per_soc.is_finite() && km_per_soc > 0.0 && km_per_soc < 50.0
    }
}
