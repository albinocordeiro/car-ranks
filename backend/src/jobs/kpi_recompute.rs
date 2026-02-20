use anyhow::{Context, Result};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::MetricCalc;

/// Rebuilds the temperature-impact KPI family for each supported timeframe.
pub(crate) async fn recompute_temperature_kpis(pool: &SqlitePool) -> Result<(usize, usize)> {
    let vehicles = sqlx::query(
        r#"
        SELECT vehicle_uid
        FROM vehicle
        ORDER BY created_at ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .context("failed to read vehicles")?;

    let mut rows_inserted = 0usize;

    for timeframe in ["30d", "90d", "180d"] {
        sqlx::query(
            r#"
            DELETE FROM vehicle_kpi_snapshot
            WHERE ranking_type = 'ev_temperature_impact'
              AND timeframe = ?
            "#,
        )
        .bind(timeframe)
        .execute(pool)
        .await
        .with_context(|| format!("failed to clear KPI snapshots for timeframe {}", timeframe))?;
    }

    for vehicle_row in &vehicles {
        let vehicle_uid: String = vehicle_row
            .try_get("vehicle_uid")
            .context("invalid vehicle_uid in vehicles list")?;

        for timeframe in ["30d", "90d", "180d"] {
            let cutoff = crate::timeframe_cutoff(timeframe)?;
            let metrics =
                crate::metrics::compute_vehicle_metrics(pool, &vehicle_uid, cutoff).await?;
            let snapshot_ts = crate::now_str();

            for metric in metrics {
                for temp_bin in ["all", "cold"] {
                    insert_kpi_snapshot(
                        pool,
                        &vehicle_uid,
                        "ev_temperature_impact",
                        timeframe,
                        &metric,
                        temp_bin,
                        Some("mild"),
                        Some("cold"),
                        &snapshot_ts,
                    )
                    .await
                    .with_context(|| {
                        format!(
                            "failed to insert temperature KPI {} for vehicle {} timeframe {}",
                            metric.key, vehicle_uid, timeframe
                        )
                    })?;

                    rows_inserted += 1;
                }
            }
        }
    }

    Ok((rows_inserted, vehicles.len()))
}

/// Rebuilds range, charging, and composite KPI families for each vehicle/timeframe.
pub(crate) async fn recompute_non_temperature_kpis(pool: &SqlitePool) -> Result<(usize, usize)> {
    let vehicles = sqlx::query(
        r#"
        SELECT vehicle_uid
        FROM vehicle
        ORDER BY created_at ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .context("failed to read vehicles for non-temperature KPIs")?;

    let mut rows_inserted = 0usize;

    for timeframe in ["30d", "90d", "180d"] {
        for ranking_type in [
            "ev_range_efficiency",
            "ev_charging_performance",
            "ev_composite",
        ] {
            sqlx::query(
                r#"
                DELETE FROM vehicle_kpi_snapshot
                WHERE ranking_type = ?
                  AND timeframe = ?
                "#,
            )
            .bind(ranking_type)
            .bind(timeframe)
            .execute(pool)
            .await
            .with_context(|| {
                format!(
                    "failed to clear KPI snapshots for ranking_type {} timeframe {}",
                    ranking_type, timeframe
                )
            })?;
        }
    }

    for vehicle_row in &vehicles {
        let vehicle_uid: String = vehicle_row
            .try_get("vehicle_uid")
            .context("invalid vehicle_uid in non-temperature KPI pass")?;

        for timeframe in ["30d", "90d", "180d"] {
            let cutoff = crate::timeframe_cutoff(timeframe)?;
            let range_metrics =
                crate::metrics::compute_range_efficiency_metrics(pool, &vehicle_uid, cutoff)
                    .await?;
            let charging_metrics =
                crate::metrics::compute_charging_performance_metrics(pool, &vehicle_uid, cutoff)
                    .await?;
            let composite_metrics = crate::metrics::compute_composite_metrics(
                pool,
                &vehicle_uid,
                cutoff,
                &range_metrics,
                &charging_metrics,
            )
            .await?;

            let snapshot_ts = crate::now_str();
            for metric in &range_metrics {
                insert_kpi_snapshot(
                    pool,
                    &vehicle_uid,
                    "ev_range_efficiency",
                    timeframe,
                    metric,
                    "all",
                    None,
                    None,
                    &snapshot_ts,
                )
                .await?;
                rows_inserted += 1;
            }

            for metric in &charging_metrics {
                insert_kpi_snapshot(
                    pool,
                    &vehicle_uid,
                    "ev_charging_performance",
                    timeframe,
                    metric,
                    "all",
                    None,
                    None,
                    &snapshot_ts,
                )
                .await?;
                rows_inserted += 1;
            }

            for metric in &composite_metrics {
                insert_kpi_snapshot(
                    pool,
                    &vehicle_uid,
                    "ev_composite",
                    timeframe,
                    metric,
                    "all",
                    None,
                    None,
                    &snapshot_ts,
                )
                .await?;
                rows_inserted += 1;
            }
        }
    }

    Ok((rows_inserted, vehicles.len()))
}

/// Validates and persists one locked KPI snapshot row.
///
/// The KPI lock check guarantees ranking rebuilds only consume approved formulas.
async fn insert_kpi_snapshot(
    pool: &SqlitePool,
    vehicle_uid: &str,
    ranking_type: &str,
    timeframe: &str,
    metric: &MetricCalc,
    temperature_bin: &str,
    baseline_temperature_bin: Option<&str>,
    compare_temperature_bin: Option<&str>,
    snapshot_ts: &str,
) -> Result<()> {
    let Some((formula, required_signals, optional_signals)) =
        crate::kpi_specs::locked_kpi_spec_details(ranking_type, metric.key)
    else {
        return Err(anyhow::anyhow!(
            "kpi_key {} is not locked for ranking_type {}",
            metric.key,
            ranking_type
        ));
    };
    if metric.sample_count < 0 {
        return Err(anyhow::anyhow!(
            "kpi_key {} has invalid negative sample_count {}",
            metric.key,
            metric.sample_count
        ));
    }

    tracing::debug!(
        ranking_type,
        kpi_key = metric.key,
        formula,
        ?required_signals,
        ?optional_signals,
        "persisting locked KPI snapshot"
    );

    sqlx::query(
        r#"
        INSERT INTO vehicle_kpi_snapshot (
            snapshot_id,
            vehicle_uid,
            ranking_type,
            timeframe,
            kpi_key,
            kpi_value,
            kpi_unit,
            direction,
            confidence_level,
            sample_count,
            temperature_bin,
            baseline_temperature_bin,
            compare_temperature_bin,
            computed_at,
            source_job_id
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(vehicle_uid)
    .bind(ranking_type)
    .bind(timeframe)
    .bind(metric.key)
    .bind(metric.value)
    .bind(metric.unit)
    .bind(metric.direction)
    .bind(metric.confidence_level)
    .bind(metric.sample_count)
    .bind(temperature_bin)
    .bind(baseline_temperature_bin)
    .bind(compare_temperature_bin)
    .bind(snapshot_ts)
    .bind("internal_recompute")
    .execute(pool)
    .await
    .context("failed to insert KPI snapshot row")?;
    Ok(())
}
