use anyhow::{Context, Result};
use sqlx::{PgPool, Row};

use super::KPI_TIMEFRAMES;
use super::cleanup::clear_native_range_kpi_snapshots_postgres;
use super::insert_native_kpi_snapshot_postgres;

/// Rebuilds range-efficiency KPI snapshots directly in Postgres.
pub(super) async fn recompute_range_efficiency_kpis_postgres(pool: &PgPool) -> Result<usize> {
    let vehicles = sqlx::query(
        r#"
        SELECT vehicle_uid
        FROM vehicle
        ORDER BY created_at ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .context("failed to read vehicles for native postgres range KPI pass")?;

    clear_native_range_kpi_snapshots_postgres(pool).await?;

    let mut rows_inserted = 0usize;
    for vehicle_row in &vehicles {
        let vehicle_uid: String = vehicle_row
            .try_get("vehicle_uid")
            .context("invalid vehicle_uid in native postgres range KPI pass")?;

        for timeframe in KPI_TIMEFRAMES {
            rows_inserted +=
                recompute_vehicle_timeframe_range_kpis_postgres(pool, &vehicle_uid, timeframe)
                    .await?;
        }
    }

    Ok(rows_inserted)
}

/// Recomputes range-efficiency KPIs for one vehicle/timeframe pair.
async fn recompute_vehicle_timeframe_range_kpis_postgres(
    pool: &PgPool,
    vehicle_uid: &str,
    timeframe: &str,
) -> Result<usize> {
    let cutoff = crate::timeframe_cutoff(timeframe)?;
    let metrics =
        crate::metrics::compute_range_efficiency_metrics_postgres(pool, vehicle_uid, cutoff)
            .await
            .with_context(|| {
                format!(
                    "failed to compute native postgres range KPIs for vehicle {} timeframe {}",
                    vehicle_uid, timeframe
                )
            })?;
    if metrics.is_empty() {
        return Ok(0);
    }

    let snapshot_ts = crate::now_str();
    for metric in &metrics {
        insert_native_kpi_snapshot_postgres(
            pool,
            "ev_range_efficiency",
            vehicle_uid,
            timeframe,
            metric,
            &snapshot_ts,
        )
        .await
        .with_context(|| {
            format!(
                "failed to insert native postgres range KPI {} for vehicle {} timeframe {}",
                metric.key, vehicle_uid, timeframe
            )
        })?;
    }

    Ok(metrics.len())
}
