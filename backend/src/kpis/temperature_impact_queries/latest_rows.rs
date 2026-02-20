use anyhow::Context;
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteRow;

use crate::ApiError;

/// Loads the latest temperature-impact KPI rows for a vehicle and bin pair.
pub(crate) async fn fetch_temperature_kpi_rows(
    pool: &SqlitePool,
    vehicle_uid: &str,
    timeframe: &str,
    baseline_bin: &str,
    compare_bin: &str,
) -> Result<Vec<SqliteRow>, ApiError> {
    sqlx::query(
        r#"
        SELECT kpi_key, kpi_value, kpi_unit, direction, confidence_level, sample_count
        FROM vehicle_kpi_snapshot ks
        WHERE vehicle_uid = ?
          AND ranking_type = 'ev_temperature_impact'
          AND timeframe = ?
          AND temperature_bin = 'cold'
          AND baseline_temperature_bin = ?
          AND compare_temperature_bin = ?
          AND computed_at = (
              SELECT MAX(ks2.computed_at)
              FROM vehicle_kpi_snapshot ks2
              WHERE ks2.vehicle_uid = ks.vehicle_uid
                AND ks2.ranking_type = ks.ranking_type
                AND ks2.timeframe = ks.timeframe
                AND ks2.temperature_bin = ks.temperature_bin
                AND ks2.baseline_temperature_bin = ks.baseline_temperature_bin
                AND ks2.compare_temperature_bin = ks.compare_temperature_bin
                AND ks2.kpi_key = ks.kpi_key
          )
        ORDER BY kpi_key ASC
        "#,
    )
    .bind(vehicle_uid)
    .bind(timeframe)
    .bind(baseline_bin)
    .bind(compare_bin)
    .fetch_all(pool)
    .await
    .context("failed to fetch KPI rows")
    .map_err(Into::into)
}
