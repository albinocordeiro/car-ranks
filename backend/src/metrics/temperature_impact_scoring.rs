use crate::MetricCalc;

use super::temperature_drive_metrics::{
    cold_range_retention_metric, temperature_sensitivity_metric,
};
use super::temperature_impact_series::TemperatureImpactDriveSeries;

/// Scores driving-derived temperature KPIs from a normalized series.
///
/// The caller is responsible for collecting source observations; this helper
/// focuses only on gate checks and deterministic metric math.
pub(super) fn score_drive_metrics(
    drive_series: TemperatureImpactDriveSeries,
    gates: super::TemperatureSampleGates,
) -> Vec<MetricCalc> {
    let TemperatureImpactDriveSeries {
        points,
        cold_values,
        mild_values,
        cold_distance_km,
        mild_distance_km,
    } = drive_series;

    if !gates.range_gate_passed(cold_distance_km, mild_distance_km) {
        return Vec::new();
    }

    let mut metrics = Vec::new();
    if let Some((retention_metric, mild_km_per_soc)) =
        cold_range_retention_metric(&cold_values, &mild_values)
    {
        metrics.push(retention_metric);
        if let Some(sensitivity_metric) =
            temperature_sensitivity_metric(&points, mild_km_per_soc, gates)
        {
            metrics.push(sensitivity_metric);
        }
    }

    metrics
}
