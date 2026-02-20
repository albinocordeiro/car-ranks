use crate::MetricCalc;

/// Builds urban/highway supplemental efficiency KPIs from segmented samples.
pub(super) fn speed_segment_efficiency_metrics(
    urban_wh_per_km_points: &[f64],
    highway_wh_per_km_points: &[f64],
) -> Vec<MetricCalc> {
    let mut metrics = Vec::new();

    if let Some(urban_efficiency) = super::median(urban_wh_per_km_points.to_vec()) {
        let urban_samples = urban_wh_per_km_points.len() as i64;
        metrics.push(build_metric(
            "ev_urban_efficiency",
            urban_efficiency,
            "Wh_per_km",
            "lower_is_better",
            urban_samples,
        ));
    }

    if let Some(highway_efficiency) = super::median(highway_wh_per_km_points.to_vec()) {
        let highway_samples = highway_wh_per_km_points.len() as i64;
        metrics.push(build_metric(
            "ev_highway_efficiency",
            highway_efficiency,
            "Wh_per_km",
            "lower_is_better",
            highway_samples,
        ));
    }

    metrics
}

/// Shared metric constructor for supplemental range-efficiency rows.
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
