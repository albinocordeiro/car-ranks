use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};

use crate::MetricCalc;

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

/// Computes health penalty points from active DTC and MIL status.
pub(super) async fn compute_health_modifier_penalty(
    pool: &SqlitePool,
    vehicle_uid: &str,
    cutoff: DateTime<Utc>,
) -> Result<(f64, i64)> {
    let dtc_row = sqlx::query(
        r#"
        SELECT COUNT(DISTINCT code) AS dtc_count
        FROM vehicle_diagnostic_event
        WHERE vehicle_uid = ?
          AND observed_at >= ?
          AND event_type = 'DTC_ACTIVE'
          AND code IS NOT NULL
        "#,
    )
    .bind(vehicle_uid)
    .bind(cutoff.to_rfc3339())
    .fetch_one(pool)
    .await
    .context("failed to compute active DTC count for health modifier")?;

    let dtc_count: i64 = dtc_row
        .try_get("dtc_count")
        .context("failed to parse active DTC count")?;

    let mil_row = sqlx::query(
        r#"
        SELECT event_type
        FROM vehicle_diagnostic_event
        WHERE vehicle_uid = ?
          AND observed_at >= ?
          AND event_type IN ('MIL_ON', 'MIL_OFF')
        ORDER BY observed_at DESC
        LIMIT 1
        "#,
    )
    .bind(vehicle_uid)
    .bind(cutoff.to_rfc3339())
    .fetch_optional(pool)
    .await
    .context("failed to load MIL status for health modifier")?;

    let mil_event_type = mil_row.and_then(|row| row.try_get::<String, _>("event_type").ok());
    let mil_on = mil_event_type
        .as_deref()
        .map(|event_type| event_type == "MIL_ON")
        .unwrap_or(false);

    let mil_penalty = if mil_on { 6.0 } else { 0.0 };
    let dtc_penalty = (dtc_count.max(0) as f64 * 0.5).min(4.0);
    let penalty = (mil_penalty + dtc_penalty).min(10.0);

    let sample_count = dtc_count.max(0) + if mil_event_type.is_some() { 1 } else { 0 };
    Ok((penalty, sample_count.max(1)))
}
