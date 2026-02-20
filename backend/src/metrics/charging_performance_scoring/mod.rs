use crate::MetricCalc;

use super::charging_performance_buckets::ChargingPowerBuckets;
use super::charging_performance_retention::cold_charge_retention_metric;

mod acceptance_score;
mod final_score;
mod metric_builder;

use acceptance_score::compute_acceptance_score;
use final_score::compute_charging_performance_score;
use metric_builder::build_metric;

/// Scores charging-performance KPIs from normalized power buckets.
///
/// This pure helper keeps median math and KPI composition independent from
/// database row parsing and query orchestration.
pub(super) fn score_charging_power_buckets(
    buckets: ChargingPowerBuckets,
    gates: super::TemperatureSampleGates,
) -> Vec<MetricCalc> {
    let ChargingPowerBuckets {
        all_power,
        cold_power,
        mild_power,
    } = buckets;

    if all_power.is_empty() {
        return Vec::new();
    }

    let mut metrics = Vec::new();
    let sample_count = all_power.len() as i64;
    let acceptance_score = compute_acceptance_score(&all_power, &mild_power);

    metrics.push(build_metric(
        "temp_adjusted_charge_acceptance_score",
        acceptance_score,
        "score",
        "higher_is_better",
        sample_count,
    ));

    let mut retention_score = None;
    if let Some((retention_metric, retention)) =
        cold_charge_retention_metric(&cold_power, &mild_power, gates)
    {
        metrics.push(retention_metric);
        retention_score = Some(retention);
    }

    let charging_score = compute_charging_performance_score(acceptance_score, retention_score);

    metrics.push(build_metric(
        "charging_performance_score",
        charging_score,
        "score",
        "higher_is_better",
        sample_count,
    ));

    metrics
}
