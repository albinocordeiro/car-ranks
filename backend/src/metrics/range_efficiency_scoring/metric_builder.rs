use crate::MetricCalc;

/// Standardizes metric construction so every KPI row gets the same confidence
/// calculation policy and avoids duplicated field wiring.
pub(super) fn build_metric(
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
        confidence_level: super::super::confidence_from_samples(sample_count),
    }
}
