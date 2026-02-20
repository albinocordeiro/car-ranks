use anyhow::{Context, Result};
use sqlx::SqlitePool;

use super::super::snapshot_writer::insert_kpi_snapshot;

/// Recomputes temperature-impact KPIs for one vehicle/timeframe pair.
pub(super) async fn recompute_vehicle_timeframe_temperature(
    pool: &SqlitePool,
    vehicle_uid: &str,
    timeframe: &str,
) -> Result<usize> {
    let cutoff = crate::timeframe_cutoff(timeframe)?;
    let metrics = crate::metrics::compute_vehicle_metrics(pool, vehicle_uid, cutoff).await?;
    let snapshot_ts = crate::now_str();
    let mut rows_inserted = 0usize;

    for metric in metrics {
        for temp_bin in ["all", "cold"] {
            insert_kpi_snapshot(
                pool,
                vehicle_uid,
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

    Ok(rows_inserted)
}
