use super::super::temperature_regression::VehiclePoint;
use super::TemperatureImpactAccumulator;

impl TemperatureImpactAccumulator {
    /// Captures one temperature-impact point from the latest complete state pair.
    ///
    /// The delta gates reject tiny jitter and unrealistic jumps before KPI scoring.
    pub(super) fn capture_drive_point(&mut self) {
        if let (Some(odo), Some(soc), Some(temp)) =
            (self.current_odo, self.current_soc, self.current_temp)
        {
            if let Some((prev_odo, prev_soc, _prev_temp)) = self.prev_filled {
                let delta_km = odo - prev_odo;
                let delta_soc = prev_soc - soc;
                if self.accept_delta_window(delta_km, delta_soc) {
                    let km_per_soc = delta_km / delta_soc;
                    if self.accept_km_per_soc(km_per_soc) {
                        self.points.push(VehiclePoint {
                            temperature_c: temp,
                            km_per_soc,
                        });
                        self.bucket_temperature_distance(temp, km_per_soc, delta_km);
                    }
                }
            }

            // Save the newest complete tuple so the next snapshot can form deltas.
            self.prev_filled = Some((odo, soc, temp));
        }
    }

    fn accept_delta_window(&self, delta_km: f64, delta_soc: f64) -> bool {
        delta_km > 0.05 && delta_soc > 0.05 && delta_soc < 30.0
    }

    fn accept_km_per_soc(&self, km_per_soc: f64) -> bool {
        km_per_soc.is_finite() && km_per_soc > 0.0 && km_per_soc < 50.0
    }
}
