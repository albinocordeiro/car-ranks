use anyhow::Result;

use super::temperature_impact_accumulator::TemperatureImpactAccumulator;
use super::temperature_impact_snapshots::{
    TemperatureObservationRow, normalize_temperature_impact_snapshots,
};
use super::temperature_regression::VehiclePoint;

/// Derived driving series used by temperature-impact KPI builders.
pub(super) struct TemperatureImpactDriveSeries {
    pub(super) points: Vec<VehiclePoint>,
    pub(super) cold_values: Vec<f64>,
    pub(super) mild_values: Vec<f64>,
    pub(super) cold_distance_km: f64,
    pub(super) mild_distance_km: f64,
}

/// Converts raw observation rows into aligned series for temperature KPIs.
pub(super) fn build_drive_series(
    obs_rows: Vec<TemperatureObservationRow>,
) -> Result<TemperatureImpactDriveSeries> {
    let by_ts = normalize_temperature_impact_snapshots(obs_rows)?;
    let mut accumulator = TemperatureImpactAccumulator::new();
    for snapshot in by_ts.values() {
        accumulator.observe_snapshot(snapshot);
    }

    Ok(accumulator.into_series())
}
