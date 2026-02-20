use anyhow::Result;
use sqlx::SqlitePool;

use crate::MetricCalc;

use super::super::snapshot_writer::insert_kpi_snapshot;

/// Recomputes all non-temperature KPI families for one vehicle/timeframe pair.
pub(super) async fn recompute_vehicle_timeframe_non_temperature(
    pool: &SqlitePool,
    vehicle_uid: &str,
    timeframe: &str,
) -> Result<usize> {
    let cutoff = crate::timeframe_cutoff(timeframe)?;
    let range_metrics =
        crate::metrics::compute_range_efficiency_metrics(pool, vehicle_uid, cutoff).await?;
    let charging_metrics =
        crate::metrics::compute_charging_performance_metrics(pool, vehicle_uid, cutoff).await?;
    let composite_metrics = crate::metrics::compute_composite_metrics(
        pool,
        vehicle_uid,
        cutoff,
        &range_metrics,
        &charging_metrics,
    )
    .await?;

    let snapshot_ts = crate::now_str();
    let mut rows_inserted = 0usize;
    rows_inserted += persist_metric_family(
        pool,
        vehicle_uid,
        "ev_range_efficiency",
        timeframe,
        &range_metrics,
        &snapshot_ts,
    )
    .await?;
    rows_inserted += persist_metric_family(
        pool,
        vehicle_uid,
        "ev_charging_performance",
        timeframe,
        &charging_metrics,
        &snapshot_ts,
    )
    .await?;
    rows_inserted += persist_metric_family(
        pool,
        vehicle_uid,
        "ev_composite",
        timeframe,
        &composite_metrics,
        &snapshot_ts,
    )
    .await?;

    Ok(rows_inserted)
}

/// Persists one ranking family's metrics and returns inserted row count.
async fn persist_metric_family(
    pool: &SqlitePool,
    vehicle_uid: &str,
    ranking_type: &str,
    timeframe: &str,
    metrics: &[MetricCalc],
    snapshot_ts: &str,
) -> Result<usize> {
    for metric in metrics {
        insert_kpi_snapshot(
            pool,
            vehicle_uid,
            ranking_type,
            timeframe,
            metric,
            "all",
            None,
            None,
            snapshot_ts,
        )
        .await?;
    }

    Ok(metrics.len())
}
