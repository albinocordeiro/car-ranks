use crate::MetricCalc;

/// Computes the cold-weather charge retention KPI when sampling gates pass.
pub(super) fn cold_charge_retention_metric(
    cold_power: &[f64],
    mild_power: &[f64],
    gates: super::TemperatureSampleGates,
) -> Option<(MetricCalc, f64)> {
    if !gates.charge_gate_passed(cold_power.len(), mild_power.len()) {
        return None;
    }

    if let (Some(cold_median), Some(mild_median)) = (
        super::median(cold_power.to_vec()),
        super::median(mild_power.to_vec()),
    ) {
        if mild_median > 0.0 {
            let retention = (100.0 * cold_median / mild_median).clamp(0.0, 200.0);
            let retention_samples = cold_power.len().min(mild_power.len()) as i64;
            let metric = MetricCalc {
                key: "cold_weather_charge_speed_retention",
                value: retention,
                unit: "%",
                direction: "higher_is_better",
                sample_count: retention_samples,
                confidence_level: super::confidence_from_samples(retention_samples),
            };
            return Some((metric, retention));
        }
    }

    None
}
