use anyhow::{Context, Result};
use sqlx::{Row, SqlitePool};

use super::snapshot_writer::insert_kpi_snapshot;

/// Rebuilds the temperature-impact KPI family for each supported timeframe.
pub(super) async fn recompute_temperature_kpis(pool: &SqlitePool) -> Result<(usize, usize)> {
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
