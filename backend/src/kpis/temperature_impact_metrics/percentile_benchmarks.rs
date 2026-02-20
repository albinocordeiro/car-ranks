use crate::{KpiMetric, percentile_rank};

/// Computes the cohort percentile for one KPI metric.
///
/// Temperature-impact metrics mix "higher is better" and "lower is better"
/// dimensions, so direction controls whether rank ordering is reversed.
pub(super) fn compute_percentile_benchmark(values: &[f64], metric: &KpiMetric) -> i64 {
    percentile_rank(
        values,
        metric.value,
        metric.direction.as_str() == "higher_is_better",
    )
}
