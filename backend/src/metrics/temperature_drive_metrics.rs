use crate::MetricCalc;

use super::temperature_regression::{VehiclePoint, linear_regression_slope};

/// Computes the cold-weather range-retention KPI from segmented drive samples.
pub(super) fn cold_range_retention_metric(
    cold_values: &[f64],
    mild_values: &[f64],
) -> Option<(MetricCalc, f64)> {
    if let (Some(cold), Some(mild)) = (
        super::median(cold_values.to_vec()),
        super::median(mild_values.to_vec()),
    ) {
        if mild > 0.0 {
            let retention = (100.0 * cold / mild).clamp(0.0, 200.0);
            let sample_count = cold_values.len().min(mild_values.len()) as i64;
            let metric = MetricCalc {
                key: "cold_weather_range_retention",
                value: retention,
                unit: "%",
                direction: "higher_is_better",
                sample_count,
                confidence_level: super::confidence_from_samples(sample_count),
            };
            return Some((metric, mild));
        }
    }

    None
}

/// Computes the temperature sensitivity KPI when enough regression samples exist.
pub(super) fn temperature_sensitivity_metric(
    points: &[VehiclePoint],
    mild_km_per_soc: f64,
    gates: super::TemperatureSampleGates,
) -> Option<MetricCalc> {
    // The sensitivity index is only meaningful when enough paired points
    // are available for a stable linear fit.
    if points.len() < gates.min_sensitivity_points {
        return None;
    }

    let slope = linear_regression_slope(points)?;
    let loss_pct_per_10c_drop = if slope < 0.0 {
        ((-slope * 10.0) / mild_km_per_soc) * 100.0
    } else {
        0.0
    }
    .clamp(0.0, 100.0);

    let sample_count = points.len() as i64;
    Some(MetricCalc {
        key: "range_temperature_sensitivity_index",
        value: loss_pct_per_10c_drop,
        unit: "%_loss_per_10C_drop",
        direction: "lower_is_better",
        sample_count,
        confidence_level: super::confidence_from_samples(sample_count),
    })
}
