use crate::MetricCalc;

use super::range_efficiency_additional::speed_segment_efficiency_metrics;
use super::range_efficiency_baseline::compute_range_efficiency_baseline;
use super::range_efficiency_regeneration::regeneration_recovery_ratio_metric;
use super::range_efficiency_series::RangeEfficiencySeries;

mod baseline_metrics;
mod metric_builder;

use baseline_metrics::build_baseline_metrics;

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

    // Start with the four core KPI rows built from the same baseline sample set.
    let mut metrics = build_baseline_metrics(&baseline);

    // Add segmented city/highway efficiency KPIs when those buckets exist.
    metrics.extend(speed_segment_efficiency_metrics(
        &urban_wh_per_km_points,
        &highway_wh_per_km_points,
    ));

    // Regeneration is optional because some traces have no valid power windows.
    if let Some(regen_metric) = regeneration_recovery_ratio_metric(&power_windows) {
        metrics.push(regen_metric);
    }

    metrics
}
