use anyhow::{Context, Result};
use sqlx::{PgPool, Row};

use self::cleanup::clear_native_charging_kpi_snapshots_postgres;
use self::vehicle_pass::recompute_vehicle_timeframe_charging_kpis_postgres;

mod cleanup;
mod snapshot_writer;
mod vehicle_pass;

const KPI_TIMEFRAMES: [&str; 3] = ["30d", "90d", "180d"];

/// Rebuilds charging-performance KPI snapshots directly in Postgres.
pub(super) async fn recompute_charging_performance_kpis_postgres(pool: &PgPool) -> Result<usize> {
    let vehicles = sqlx::query(
        r#"
        SELECT vehicle_uid
        FROM vehicle
        ORDER BY created_at ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .context("failed to read vehicles for native postgres charging KPI pass")?;

    clear_native_charging_kpi_snapshots_postgres(pool).await?;

    let mut rows_inserted = 0usize;
    for vehicle_row in &vehicles {
        let vehicle_uid: String = vehicle_row
            .try_get("vehicle_uid")
            .context("invalid vehicle_uid in native postgres charging KPI pass")?;

        for timeframe in KPI_TIMEFRAMES {
            rows_inserted +=
                recompute_vehicle_timeframe_charging_kpis_postgres(pool, &vehicle_uid, timeframe)
                    .await?;
        }
    }

    Ok(rows_inserted)
}
