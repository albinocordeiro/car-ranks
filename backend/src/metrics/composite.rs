use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use crate::MetricCalc;

use super::composite_health::compute_health_modifier_penalty;

/// Rebuilds composite EV score metrics from range/charging families and health diagnostics.
pub(super) async fn compute_composite_metrics(
    pool: &SqlitePool,
    vehicle_uid: &str,
    cutoff: DateTime<Utc>,
    range_metrics: &[MetricCalc],
    charging_metrics: &[MetricCalc],
) -> Result<Vec<MetricCalc>> {
    let range_score = range_metrics
        .iter()
        .find(|metric| metric.key == "ev_range_efficiency_score")
        .map(|metric| metric.value);
    let charging_score = charging_metrics
        .iter()
        .find(|metric| metric.key == "charging_performance_score")
        .map(|metric| metric.value);

    let Some(base_composite_score) = (match (range_score, charging_score) {
        (Some(range), Some(charging)) => Some((0.6 * range + 0.4 * charging).clamp(0.0, 100.0)),
        (Some(range), None) => Some(range.clamp(0.0, 100.0)),
        (None, Some(charging)) => Some(charging.clamp(0.0, 100.0)),
        (None, None) => None,
    }) else {
        return Ok(Vec::new());
    };

    let (health_penalty, health_sample_count) =
        compute_health_modifier_penalty(pool, vehicle_uid, cutoff).await?;
    let adjusted_score = (base_composite_score - health_penalty).clamp(0.0, 100.0);

    let sample_count = (range_metrics
        .iter()
        .find(|metric| metric.key == "ev_range_efficiency_score")
        .map(|metric| metric.sample_count)
        .unwrap_or(0))
    .max(
        charging_metrics
            .iter()
            .find(|metric| metric.key == "charging_performance_score")
            .map(|metric| metric.sample_count)
            .unwrap_or(0),
    )
    .max(health_sample_count);

    Ok(vec![
        MetricCalc {
            key: "ev_composite_base_score",
            value: base_composite_score,
            unit: "score",
            direction: "higher_is_better",
            sample_count,
            confidence_level: super::confidence_from_samples(sample_count),
        },
        MetricCalc {
            key: "ev_health_modifier_penalty",
            value: health_penalty,
            unit: "score_points",
            direction: "lower_is_better",
            sample_count: health_sample_count,
            confidence_level: super::confidence_from_samples(health_sample_count),
        },
        MetricCalc {
            key: "ev_composite_score",
            value: adjusted_score,
            unit: "score",
            direction: "higher_is_better",
            sample_count,
            confidence_level: super::confidence_from_samples(sample_count),
        },
    ])
}
