use anyhow::{Context, Result};
use sqlx::{PgPool, Row};

use crate::MetricCalc;

use self::cleanup::clear_native_charging_kpi_snapshots_postgres;
use self::vehicle_pass::recompute_vehicle_timeframe_charging_kpis_postgres;

mod cleanup;
mod range_pass;
mod snapshot_writer;
mod temperature_pass;
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

/// Rebuilds range-efficiency KPI snapshots directly in Postgres.
pub(super) async fn recompute_range_efficiency_kpis_postgres(pool: &PgPool) -> Result<usize> {
    range_pass::recompute_range_efficiency_kpis_postgres(pool).await
}

/// Rebuilds temperature-impact KPI snapshots directly in Postgres.
pub(super) async fn recompute_temperature_impact_kpis_postgres(pool: &PgPool) -> Result<usize> {
    temperature_pass::recompute_temperature_impact_kpis_postgres(pool).await
}

/// Persists one native KPI snapshot row for any native ranking family.
///
/// This wrapper keeps write logic encapsulated in `snapshot_writer` while exposing
/// a narrow API to sibling Postgres-native stages (for example, composite).
pub(in crate::jobs::postgres_native) async fn insert_native_kpi_snapshot_postgres(
    pool: &PgPool,
    ranking_type: &str,
    vehicle_uid: &str,
    timeframe: &str,
    metric: &MetricCalc,
    snapshot_ts: &str,
) -> Result<()> {
    snapshot_writer::insert_native_kpi_snapshot_postgres(
        pool,
        ranking_type,
        vehicle_uid,
        timeframe,
        metric,
        snapshot_ts,
    )
    .await
}

/// Persists one native KPI snapshot row with explicit temperature bins.
pub(in crate::jobs::postgres_native) async fn insert_native_kpi_snapshot_with_bins_postgres(
    pool: &PgPool,
    ranking_type: &str,
    vehicle_uid: &str,
    timeframe: &str,
    metric: &MetricCalc,
    temperature_bin: &str,
    baseline_temperature_bin: Option<&str>,
    compare_temperature_bin: Option<&str>,
    snapshot_ts: &str,
) -> Result<()> {
    snapshot_writer::insert_native_kpi_snapshot_with_bins_postgres(
        pool,
        ranking_type,
        vehicle_uid,
        timeframe,
        metric,
        temperature_bin,
        baseline_temperature_bin,
        compare_temperature_bin,
        snapshot_ts,
    )
    .await
}
