use crate::MetricCalc;

use super::super::range_efficiency_baseline::RangeEfficiencyBaseline;
use super::metric_builder::build_metric;

/// Builds the core KPI rows that always come from the baseline range calculation.
///
/// Keeping this in a separate helper makes it easy to audit that all baseline
/// outputs share one sample count and one metric-construction policy.
pub(super) fn build_baseline_metrics(baseline: &RangeEfficiencyBaseline) -> Vec<MetricCalc> {
    let sample_count = baseline.sample_count;

    vec![
        build_metric(
            "ev_net_energy_efficiency",
            baseline.net_energy_efficiency,
            "Wh_per_km",
            "lower_is_better",
            sample_count,
        ),
        build_metric(
            "ev_estimated_practical_range",
            baseline.estimated_range,
            "km",
            "higher_is_better",
            sample_count,
        ),
        build_metric(
            "soc_depletion_rate_per_100km",
            baseline.soc_depletion_per_100km,
            "%_per_100km",
            "lower_is_better",
            sample_count,
        ),
        build_metric(
            "ev_range_efficiency_score",
            baseline.range_efficiency_score,
            "score",
            "higher_is_better",
            sample_count,
        ),
    ]
}
