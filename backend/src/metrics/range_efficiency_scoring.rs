use crate::MetricCalc;

use super::range_efficiency_baseline::compute_range_efficiency_baseline;
use super::range_efficiency_regeneration::regeneration_recovery_ratio_metric;
use super::range_efficiency_series::RangeEfficiencySeries;

/// Converts a normalized range-efficiency series into persisted KPI metrics.
///
/// This function is intentionally pure so the caller can keep I/O concerns
/// (database fetching) separate from KPI math and confidence scoring.
pub(super) fn score_range_efficiency_series(
    series: RangeEfficiencySeries,
    default_usable_battery_kwh: f64,
) -> Vec<MetricCalc> {
    let RangeEfficiencySeries {
        km_per_soc_points,
        wh_per_km_points,
        urban_wh_per_km_points,
        highway_wh_per_km_points,
        power_windows,
        latest_soc,
    } = series;

    let Some(baseline) = compute_range_efficiency_baseline(
        &km_per_soc_points,
        &wh_per_km_points,
        latest_soc,
        default_usable_battery_kwh,
    ) else {
        return Vec::new();
    };

    let mut metrics = Vec::new();
    let sample_count = baseline.sample_count;

    metrics.push(build_metric(
        "ev_net_energy_efficiency",
        baseline.net_energy_efficiency,
        "Wh_per_km",
        "lower_is_better",
        sample_count,
    ));
    metrics.push(build_metric(
        "ev_estimated_practical_range",
        baseline.estimated_range,
        "km",
        "higher_is_better",
        sample_count,
    ));
    metrics.push(build_metric(
        "soc_depletion_rate_per_100km",
        baseline.soc_depletion_per_100km,
        "%_per_100km",
        "lower_is_better",
        sample_count,
    ));
    metrics.push(build_metric(
        "ev_range_efficiency_score",
        baseline.range_efficiency_score,
        "score",
        "higher_is_better",
        sample_count,
    ));

    if let Some(urban_efficiency) = super::median(urban_wh_per_km_points.clone()) {
        let urban_samples = urban_wh_per_km_points.len() as i64;
        metrics.push(build_metric(
            "ev_urban_efficiency",
            urban_efficiency,
            "Wh_per_km",
            "lower_is_better",
            urban_samples,
        ));
    }

    if let Some(highway_efficiency) = super::median(highway_wh_per_km_points.clone()) {
        let highway_samples = highway_wh_per_km_points.len() as i64;
        metrics.push(build_metric(
            "ev_highway_efficiency",
            highway_efficiency,
            "Wh_per_km",
            "lower_is_better",
            highway_samples,
        ));
    }

    if let Some(regen_metric) = regeneration_recovery_ratio_metric(&power_windows) {
        metrics.push(regen_metric);
    }

    metrics
}

/// Standardizes metric construction so every KPI row gets the same confidence
/// calculation policy and avoids duplicated field wiring.
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
