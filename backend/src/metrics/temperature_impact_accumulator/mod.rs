use super::temperature_impact_series::TemperatureImpactDriveSeries;
use super::temperature_impact_snapshots::TemperatureTimestampSnapshot;
use super::temperature_regression::VehiclePoint;

mod point_collection;
mod snapshot_state;
mod temperature_buckets;

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
        // Update sparse sensor fields into the running snapshot state.
        self.refresh_latest_snapshot_state(snapshot);

        // Convert state deltas into temperature-impact driving points.
        self.capture_drive_point();
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
}
