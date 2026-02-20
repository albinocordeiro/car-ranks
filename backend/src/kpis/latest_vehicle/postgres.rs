use anyhow::{Context, Result};
use sqlx::PgPool;

use crate::KpiMetric;

use super::row_mapper::map_postgres_kpi_metric;

/// Fetches the latest KPI value per key for a vehicle from PostgreSQL storage.
pub(crate) async fn fetch_latest_vehicle_kpis_postgres(
    pool: &PgPool,
    vehicle_uid: &str,
    ranking_type: &str,
    timeframe: &str,
    temperature_bin: &str,
) -> Result<Vec<KpiMetric>> {
    let rows = sqlx::query(
        r#"
        SELECT
          kpi_key,
          kpi_value::double precision AS kpi_value,
          kpi_unit,
          direction,
          confidence_level,
          sample_count::bigint AS sample_count
        FROM vehicle_kpi_snapshot ks
        WHERE vehicle_uid = $1
          AND ranking_type = $2
          AND timeframe = $3
          AND temperature_bin = $4
          AND computed_at = (
              SELECT MAX(ks2.computed_at)
              FROM vehicle_kpi_snapshot ks2
              WHERE ks2.vehicle_uid = ks.vehicle_uid
                AND ks2.ranking_type = ks.ranking_type
                AND ks2.timeframe = ks.timeframe
                AND ks2.temperature_bin = ks.temperature_bin
                AND ks2.kpi_key = ks.kpi_key
          )
        ORDER BY kpi_key ASC
        "#,
    )
    .bind(vehicle_uid)
    .bind(ranking_type)
    .bind(timeframe)
    .bind(temperature_bin)
    .fetch_all(pool)
    .await
    .context("failed to fetch latest vehicle KPIs from postgres")?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(map_postgres_kpi_metric(&row)?);
    }

    Ok(out)
}
