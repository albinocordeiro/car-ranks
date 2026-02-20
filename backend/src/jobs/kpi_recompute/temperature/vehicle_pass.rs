use anyhow::{Context, Result};
use sqlx::SqlitePool;

use super::super::snapshot_writer::insert_kpi_snapshot;
use super::super::{
    TEMPERATURE_BASELINE_BIN, TEMPERATURE_COMPARE_BIN, TEMPERATURE_OUTPUT_BINS,
    TEMPERATURE_RANKING_TYPE,
};

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
        for temp_bin in TEMPERATURE_OUTPUT_BINS {
            insert_kpi_snapshot(
                pool,
                vehicle_uid,
                TEMPERATURE_RANKING_TYPE,
                timeframe,
                &metric,
                temp_bin,
                Some(TEMPERATURE_BASELINE_BIN),
                Some(TEMPERATURE_COMPARE_BIN),
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
