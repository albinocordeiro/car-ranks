/// Normalized per-segment point used for temperature sensitivity regression.
#[derive(Debug)]
pub(super) struct VehiclePoint {
    pub(super) temperature_c: f64,
    pub(super) km_per_soc: f64,
}

/// Returns the linear-regression slope of `km_per_soc` over temperature.
pub(super) fn linear_regression_slope(points: &[VehiclePoint]) -> Option<f64> {
    if points.len() < 2 {
        return None;
    }

    let n = points.len() as f64;
    let mean_x = points.iter().map(|point| point.temperature_c).sum::<f64>() / n;
    let mean_y = points.iter().map(|point| point.km_per_soc).sum::<f64>() / n;

    let numerator = points
        .iter()
        .map(|point| (point.temperature_c - mean_x) * (point.km_per_soc - mean_y))
        .sum::<f64>();

    let denominator = points
        .iter()
        .map(|point| (point.temperature_c - mean_x).powi(2))
        .sum::<f64>();

    if denominator <= f64::EPSILON {
        return None;
    }

    Some(numerator / denominator)
}
