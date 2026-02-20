use crate::MetricCalc;

use super::charging_performance_buckets::ChargingPowerBuckets;

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
    let all_median = super::median(all_power.clone()).unwrap_or(0.0);
    let mild_median = super::median(mild_power.clone()).unwrap_or(all_median.max(1e-6));
    let acceptance_score = if mild_median > 0.0 {
        (100.0 * all_median / mild_median).clamp(0.0, 120.0)
    } else {
        100.0
    };

    metrics.push(build_metric(
        "temp_adjusted_charge_acceptance_score",
        acceptance_score,
        "score",
        "higher_is_better",
        sample_count,
    ));

    let mut retention_score = None;
    if gates.charge_gate_passed(cold_power.len(), mild_power.len()) {
        if let (Some(cold_median), Some(mild_median)) = (
            super::median(cold_power.clone()),
            super::median(mild_power.clone()),
        ) {
            if mild_median > 0.0 {
                let retention = (100.0 * cold_median / mild_median).clamp(0.0, 200.0);
                let retention_samples = cold_power.len().min(mild_power.len()) as i64;
                metrics.push(build_metric(
                    "cold_weather_charge_speed_retention",
                    retention,
                    "%",
                    "higher_is_better",
                    retention_samples,
                ));
                retention_score = Some(retention);
            }
        }
    }

    let charging_score = if let Some(retention_score) = retention_score {
        (0.6 * acceptance_score + 0.4 * retention_score).clamp(0.0, 100.0)
    } else {
        acceptance_score.clamp(0.0, 100.0)
    };

    metrics.push(build_metric(
        "charging_performance_score",
        charging_score,
        "score",
        "higher_is_better",
        sample_count,
    ));

    metrics
}

/// Shared metric constructor for charging KPI rows.
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
