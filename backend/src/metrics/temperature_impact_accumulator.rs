use super::temperature_impact_series::TemperatureImpactDriveSeries;
use super::temperature_impact_snapshots::TemperatureTimestampSnapshot;
use super::temperature_regression::VehiclePoint;

/// Stateful accumulator that derives temperature-impact drive series points.
pub(super) struct TemperatureImpactAccumulator {
    current_odo: Option<f64>,
    current_soc: Option<f64>,
    current_temp: Option<f64>,
    prev_filled: Option<(f64, f64, f64)>,
    points: Vec<VehiclePoint>,
    cold_values: Vec<f64>,
    mild_values: Vec<f64>,
    cold_distance_km: f64,
    mild_distance_km: f64,
}

impl TemperatureImpactAccumulator {
    /// Creates an empty accumulator for one ordered observation stream.
    pub(super) fn new() -> Self {
        Self {
            current_odo: None,
            current_soc: None,
            current_temp: None,
            prev_filled: None,
            points: Vec::new(),
            cold_values: Vec::new(),
            mild_values: Vec::new(),
            cold_distance_km: 0.0,
            mild_distance_km: 0.0,
        }
    }

    /// Consumes one snapshot and updates series points and distance buckets.
    pub(super) fn observe_snapshot(&mut self, snapshot: &TemperatureTimestampSnapshot) {
        if snapshot.odo.is_some() {
            self.current_odo = snapshot.odo;
        }
        if snapshot.soc.is_some() {
            self.current_soc = snapshot.soc;
        }
        if snapshot.temp.is_some() {
            self.current_temp = snapshot.temp;
        }

        if let (Some(odo), Some(soc), Some(temp)) =
            (self.current_odo, self.current_soc, self.current_temp)
        {
            if let Some((prev_odo, prev_soc, _prev_temp)) = self.prev_filled {
                let delta_km = odo - prev_odo;
                let delta_soc = prev_soc - soc;
                if delta_km > 0.05 && delta_soc > 0.05 && delta_soc < 30.0 {
                    let km_per_soc = delta_km / delta_soc;
                    if km_per_soc.is_finite() && km_per_soc > 0.0 && km_per_soc < 50.0 {
                        self.points.push(VehiclePoint {
                            temperature_c: temp,
                            km_per_soc,
                        });
                        self.bucket_temperature_distance(temp, km_per_soc, delta_km);
                    }
                }
            }
            self.prev_filled = Some((odo, soc, temp));
        }
    }

    /// Finalizes accumulated vectors into the shared series container.
    pub(super) fn into_series(self) -> TemperatureImpactDriveSeries {
        TemperatureImpactDriveSeries {
            points: self.points,
            cold_values: self.cold_values,
            mild_values: self.mild_values,
            cold_distance_km: self.cold_distance_km,
            mild_distance_km: self.mild_distance_km,
        }
    }

    fn bucket_temperature_distance(&mut self, temp: f64, km_per_soc: f64, delta_km: f64) {
        if temp <= 5.0 {
            self.cold_values.push(km_per_soc);
            self.cold_distance_km += delta_km;
        }
        if temp > 15.0 && temp <= 25.0 {
            self.mild_values.push(km_per_soc);
            self.mild_distance_km += delta_km;
        }
    }
}
