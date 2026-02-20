use crate::MetricCalc;

use super::temperature_impact_series::TemperatureImpactDriveSeries;
use super::temperature_regression::linear_regression_slope;

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
    let cold_median = super::median(cold_values.clone());
    let mild_median = super::median(mild_values.clone());
    if let (Some(cold), Some(mild)) = (cold_median, mild_median) {
        if mild > 0.0 {
            let retention = (100.0 * cold / mild).clamp(0.0, 200.0);
            let sample_count = cold_values.len().min(mild_values.len()) as i64;
            metrics.push(build_metric(
                "cold_weather_range_retention",
                retention,
                "%",
                "higher_is_better",
                sample_count,
            ));

            // The sensitivity index is only meaningful when enough paired points
            // are available for a stable linear fit.
            if points.len() >= gates.min_sensitivity_points {
                if let Some(slope) = linear_regression_slope(&points) {
                    let loss_pct_per_10c_drop = if slope < 0.0 {
                        ((-slope * 10.0) / mild) * 100.0
                    } else {
                        0.0
                    }
                    .clamp(0.0, 100.0);

                    let sample_count = points.len() as i64;
                    metrics.push(build_metric(
                        "range_temperature_sensitivity_index",
                        loss_pct_per_10c_drop,
                        "%_loss_per_10C_drop",
                        "lower_is_better",
                        sample_count,
                    ));
                }
            }
        }
    }

    metrics
}

/// Centralizes metric construction so confidence behavior stays consistent.
fn build_metric(
    key: &'static str,
    value: f64,
    unit: &'static str,
    direction: &'static str,
    sample_count: i64,
) -> MetricCalc {
    MetricCalc {
        key,
        value,
        unit,
        direction,
        sample_count,
        confidence_level: super::confidence_from_samples(sample_count),
    }
}
