use anyhow::{Context, Result};
use sqlx::{Row, SqlitePool};

use self::cleanup::clear_temperature_snapshots;
use self::vehicle_pass::recompute_vehicle_timeframe_temperature;
use super::KPI_TIMEFRAMES;

mod cleanup;
mod vehicle_pass;

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

    clear_temperature_snapshots(pool).await?;

    let mut rows_inserted = 0usize;
    for vehicle_row in &vehicles {
        let vehicle_uid: String = vehicle_row
            .try_get("vehicle_uid")
            .context("invalid vehicle_uid in vehicles list")?;

        for timeframe in KPI_TIMEFRAMES {
            rows_inserted +=
                recompute_vehicle_timeframe_temperature(pool, &vehicle_uid, timeframe).await?;
        }
    }

    Ok((rows_inserted, vehicles.len()))
}
