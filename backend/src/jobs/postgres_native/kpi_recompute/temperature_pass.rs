use anyhow::{Context, Result};
use sqlx::{PgPool, Row};

use super::KPI_TIMEFRAMES;
use super::cleanup::clear_native_temperature_kpi_snapshots_postgres;
use super::insert_native_kpi_snapshot_with_bins_postgres;

const TEMPERATURE_RANKING_TYPE: &str = "ev_temperature_impact";
const TEMPERATURE_BASELINE_BIN: &str = "mild";
const TEMPERATURE_COMPARE_BIN: &str = "cold";
const TEMPERATURE_OUTPUT_BINS: [&str; 2] = ["all", "cold"];

/// Rebuilds temperature-impact KPI snapshots directly in Postgres.
pub(super) async fn recompute_temperature_impact_kpis_postgres(pool: &PgPool) -> Result<usize> {
    let vehicles = sqlx::query(
        r#"
        SELECT vehicle_uid
        FROM vehicle
        ORDER BY created_at ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .context("failed to read vehicles for native postgres temperature KPI pass")?;

    clear_native_temperature_kpi_snapshots_postgres(pool).await?;

    let mut rows_inserted = 0usize;
    for vehicle_row in &vehicles {
        let vehicle_uid: String = vehicle_row
            .try_get("vehicle_uid")
            .context("invalid vehicle_uid in native postgres temperature KPI pass")?;

        for timeframe in KPI_TIMEFRAMES {
            rows_inserted += recompute_vehicle_timeframe_temperature_kpis_postgres(
                pool,
                &vehicle_uid,
                timeframe,
            )
            .await?;
        }
    }

    Ok(rows_inserted)
}

/// Recomputes temperature-impact KPIs for one vehicle/timeframe pair.
async fn recompute_vehicle_timeframe_temperature_kpis_postgres(
    pool: &PgPool,
    vehicle_uid: &str,
    timeframe: &str,
) -> Result<usize> {
    let cutoff = crate::timeframe_cutoff(timeframe)?;
    let metrics = crate::metrics::compute_vehicle_metrics_postgres(pool, vehicle_uid, cutoff)
        .await
        .with_context(|| {
            format!(
                "failed to compute native postgres temperature KPIs for vehicle {} timeframe {}",
                vehicle_uid, timeframe
            )
        })?;
    if metrics.is_empty() {
        return Ok(0);
    }

    let snapshot_ts = crate::now_str();
    let mut rows_inserted = 0usize;
    for metric in &metrics {
        for temp_bin in TEMPERATURE_OUTPUT_BINS {
            insert_native_kpi_snapshot_with_bins_postgres(
                pool,
                TEMPERATURE_RANKING_TYPE,
                vehicle_uid,
                timeframe,
                metric,
                temp_bin,
                Some(TEMPERATURE_BASELINE_BIN),
                Some(TEMPERATURE_COMPARE_BIN),
                &snapshot_ts,
            )
            .await
            .with_context(|| {
                format!(
                    "failed to insert native postgres temperature KPI {} for vehicle {} timeframe {} bin {}",
                    metric.key, vehicle_uid, timeframe, temp_bin
                )
            })?;
            rows_inserted += 1;
        }
    }

    Ok(rows_inserted)
}
