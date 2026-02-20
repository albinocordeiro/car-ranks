use anyhow::{Context, Result};
use sqlx::{PgPool, Row};

use crate::{MetricCalc, metrics};

use super::snapshot_writer::insert_native_charging_kpi_snapshot_postgres;

/// Rebuilds charging KPI snapshots for one vehicle/timeframe pair in Postgres.
pub(super) async fn recompute_vehicle_timeframe_charging_kpis_postgres(
    pool: &PgPool,
    vehicle_uid: &str,
    timeframe: &str,
) -> Result<usize> {
    let cutoff = crate::timeframe_cutoff(timeframe)?;
    let gates = metrics::temperature_sample_gates();

    let charge_rows = sqlx::query(
        r#"
        SELECT avg_charge_power_kw, temperature_bin
        FROM vehicle_charging_session
        WHERE vehicle_uid = $1
          AND started_at >= $2
          AND avg_charge_power_kw IS NOT NULL
        "#,
    )
    .bind(vehicle_uid)
    .bind(cutoff.to_rfc3339())
    .fetch_all(pool)
    .await
    .context("failed to fetch postgres charging sessions for native KPI scoring")?;

    let mut all_power = Vec::new();
    let mut cold_power = Vec::new();
    let mut mild_power = Vec::new();

    for row in charge_rows {
        let power: Option<f64> = row
            .try_get("avg_charge_power_kw")
            .context("invalid avg_charge_power_kw in native postgres charging KPI pass")?;
        let bin: Option<String> = row
            .try_get("temperature_bin")
            .context("invalid temperature_bin in native postgres charging KPI pass")?;
        let (Some(power), Some(bin)) = (power, bin) else {
            continue;
        };
        if !power.is_finite() || power <= 0.0 {
            continue;
        }

        all_power.push(power);
        if bin == "cold" || bin == "very_cold" {
            cold_power.push(power);
        }
        if bin == "mild" {
            mild_power.push(power);
        }
    }

    if all_power.is_empty() {
        return Ok(0);
    }

    let sample_count = all_power.len() as i64;
    let all_median = metrics::median(all_power.clone()).unwrap_or(0.0);
    let mild_median = metrics::median(mild_power.clone()).unwrap_or(all_median.max(1e-6));
    let acceptance_score = if mild_median > 0.0 {
        (100.0 * all_median / mild_median).clamp(0.0, 120.0)
    } else {
        100.0
    };

    let mut snapshot_metrics = Vec::new();
    snapshot_metrics.push(MetricCalc {
        key: "temp_adjusted_charge_acceptance_score",
        value: acceptance_score,
        unit: "score",
        direction: "higher_is_better",
        sample_count,
        confidence_level: metrics::confidence_from_samples(sample_count),
    });

    let mut retention_score = None;
    if gates.charge_gate_passed(cold_power.len(), mild_power.len()) {
        if let (Some(cold_median), Some(mild_median)) = (
            metrics::median(cold_power.clone()),
            metrics::median(mild_power.clone()),
        ) {
            if mild_median > 0.0 {
                let retention = (100.0 * cold_median / mild_median).clamp(0.0, 200.0);
                let retention_samples = cold_power.len().min(mild_power.len()) as i64;
                snapshot_metrics.push(MetricCalc {
                    key: "cold_weather_charge_speed_retention",
                    value: retention,
                    unit: "%",
                    direction: "higher_is_better",
                    sample_count: retention_samples,
                    confidence_level: metrics::confidence_from_samples(retention_samples),
                });
                retention_score = Some(retention);
            }
        }
    }

    let charging_score = if let Some(retention_score) = retention_score {
        (0.6 * acceptance_score + 0.4 * retention_score).clamp(0.0, 100.0)
    } else {
        acceptance_score.clamp(0.0, 100.0)
    };
    snapshot_metrics.push(MetricCalc {
        key: "charging_performance_score",
        value: charging_score,
        unit: "score",
        direction: "higher_is_better",
        sample_count,
        confidence_level: metrics::confidence_from_samples(sample_count),
    });

    let snapshot_ts = crate::now_str();
    for metric in &snapshot_metrics {
        insert_native_charging_kpi_snapshot_postgres(
            pool,
            vehicle_uid,
            timeframe,
            metric,
            &snapshot_ts,
        )
        .await?;
    }

    Ok(snapshot_metrics.len())
}
