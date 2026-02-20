use std::collections::BTreeMap;

use anyhow::Context;
use sqlx::{Row, SqlitePool};

use crate::ApiError;

/// Fetches the newest KPI values used to explain one ranking row.
pub(super) async fn fetch_latest_kpi_map(
    pool: &SqlitePool,
    vehicle_uid: &str,
    ranking_type: &str,
    timeframe: &str,
    temperature_bin: &str,
) -> Result<BTreeMap<String, f64>, ApiError> {
    let kpi_rows = sqlx::query(
        r#"
        SELECT kpi_key, kpi_value
        FROM vehicle_kpi_snapshot ks
        WHERE vehicle_uid = ?
          AND ranking_type = ?
          AND timeframe = ?
          AND temperature_bin = ?
          AND computed_at = (
              SELECT MAX(ks2.computed_at)
              FROM vehicle_kpi_snapshot ks2
              WHERE ks2.vehicle_uid = ks.vehicle_uid
                AND ks2.ranking_type = ks.ranking_type
                AND ks2.timeframe = ks.timeframe
                AND ks2.temperature_bin = ks.temperature_bin
                AND ks2.kpi_key = ks.kpi_key
          )
        "#,
    )
    .bind(vehicle_uid)
    .bind(ranking_type)
    .bind(timeframe)
    .bind(temperature_bin)
    .fetch_all(pool)
    .await
    .context("failed to fetch KPI details for ranking row")?;

    let mut kpis = BTreeMap::new();
    for kpi_row in kpi_rows {
        let key: String = kpi_row
            .try_get("kpi_key")
            .context("failed to parse kpi_key in ranking detail")?;
        let value: f64 = kpi_row
            .try_get("kpi_value")
            .context("failed to parse kpi_value in ranking detail")?;
        kpis.insert(key, value);
    }

    Ok(kpis)
}
