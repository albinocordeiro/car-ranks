use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};

use crate::MetricCalc;

/// Rebuilds charging-performance KPIs from materialized charging sessions.
pub(super) async fn compute_charging_performance_metrics(
    pool: &SqlitePool,
    vehicle_uid: &str,
    cutoff: DateTime<Utc>,
) -> Result<Vec<MetricCalc>> {
    let gates = super::temperature_sample_gates();

    let charge_rows = sqlx::query(
        r#"
        SELECT avg_charge_power_kw, temperature_bin
        FROM vehicle_charging_session
        WHERE vehicle_uid = ?
          AND started_at >= ?
          AND avg_charge_power_kw IS NOT NULL
        "#,
    )
    .bind(vehicle_uid)
    .bind(cutoff.to_rfc3339())
    .fetch_all(pool)
    .await
    .context("failed to fetch charging sessions for charging KPIs")?;

    let mut all_power = Vec::new();
    let mut cold_power = Vec::new();
    let mut mild_power = Vec::new();
    for row in charge_rows {
        let power: Option<f64> = row.try_get("avg_charge_power_kw")?;
        let bin: Option<String> = row.try_get("temperature_bin")?;
        if let (Some(p), Some(b)) = (power, bin) {
            if p <= 0.0 || !p.is_finite() {
                continue;
            }
            all_power.push(p);
            if b == "cold" || b == "very_cold" {
                cold_power.push(p);
            }
            if b == "mild" {
                mild_power.push(p);
            }
        }
    }

    if all_power.is_empty() {
        return Ok(Vec::new());
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

    metrics.push(MetricCalc {
        key: "temp_adjusted_charge_acceptance_score",
        value: acceptance_score,
        unit: "score",
        direction: "higher_is_better",
        sample_count,
        confidence_level: super::confidence_from_samples(sample_count),
    });

    if gates.charge_gate_passed(cold_power.len(), mild_power.len()) {
        if let (Some(cold_median), Some(mild_median)) = (
            super::median(cold_power.clone()),
            super::median(mild_power.clone()),
        ) {
            if mild_median > 0.0 {
                let retention = (100.0 * cold_median / mild_median).clamp(0.0, 200.0);
                let retention_samples = cold_power.len().min(mild_power.len()) as i64;
                metrics.push(MetricCalc {
                    key: "cold_weather_charge_speed_retention",
                    value: retention,
                    unit: "%",
                    direction: "higher_is_better",
                    sample_count: retention_samples,
                    confidence_level: super::confidence_from_samples(retention_samples),
                });
            }
        }
    }

    let charging_score = if let Some(retention_metric) = metrics
        .iter()
        .find(|metric| metric.key == "cold_weather_charge_speed_retention")
    {
        (0.6 * acceptance_score + 0.4 * retention_metric.value).clamp(0.0, 100.0)
    } else {
        acceptance_score.clamp(0.0, 100.0)
    };

    metrics.push(MetricCalc {
        key: "charging_performance_score",
        value: charging_score,
        unit: "score",
        direction: "higher_is_better",
        sample_count,
        confidence_level: super::confidence_from_samples(sample_count),
    });

    Ok(metrics)
}
