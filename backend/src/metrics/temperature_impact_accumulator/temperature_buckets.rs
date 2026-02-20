use super::TemperatureImpactAccumulator;

impl TemperatureImpactAccumulator {
    /// Buckets distance and efficiency points into cold and mild temperature bands.
    pub(super) fn bucket_temperature_distance(
        &mut self,
        temp: f64,
        km_per_soc: f64,
        delta_km: f64,
    ) {
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
