use anyhow::{Context, Result};
use sqlx::Row;
use sqlx::postgres::PgRow;
use sqlx::sqlite::SqliteRow;

use crate::KpiMetric;

/// Maps one SQLite row from `vehicle_kpi_snapshot` into the API model.
pub(super) fn map_sqlite_kpi_metric(row: &SqliteRow) -> Result<KpiMetric> {
    let kpi_key = row
        .try_get("kpi_key")
        .context("failed to parse kpi_key in fetch_latest_vehicle_kpis_sqlite")?;
    let value = row
        .try_get("kpi_value")
        .context("failed to parse kpi_value in fetch_latest_vehicle_kpis_sqlite")?;
    let unit = row
        .try_get::<Option<String>, _>("kpi_unit")
        .context("failed to parse kpi_unit in fetch_latest_vehicle_kpis_sqlite")?
        .unwrap_or_else(|| "score".to_string());
    let direction = row
        .try_get("direction")
        .context("failed to parse direction in fetch_latest_vehicle_kpis_sqlite")?;
    let confidence_level = row
        .try_get("confidence_level")
        .context("failed to parse confidence_level in fetch_latest_vehicle_kpis_sqlite")?;
    let sample_count = row
        .try_get("sample_count")
        .context("failed to parse sample_count in fetch_latest_vehicle_kpis_sqlite")?;

    Ok(KpiMetric {
        kpi_key,
        value,
        unit,
        direction,
        confidence_level,
        sample_count,
    })
}

/// Maps one PostgreSQL row from `vehicle_kpi_snapshot` into the API model.
pub(super) fn map_postgres_kpi_metric(row: &PgRow) -> Result<KpiMetric> {
    let kpi_key = row
        .try_get("kpi_key")
        .context("failed to parse kpi_key in fetch_latest_vehicle_kpis_postgres")?;
    let value = row
        .try_get("kpi_value")
        .context("failed to parse kpi_value in fetch_latest_vehicle_kpis_postgres")?;
    let unit = row
        .try_get::<Option<String>, _>("kpi_unit")
        .context("failed to parse kpi_unit in fetch_latest_vehicle_kpis_postgres")?
        .unwrap_or_else(|| "score".to_string());
    let direction = row
        .try_get("direction")
        .context("failed to parse direction in fetch_latest_vehicle_kpis_postgres")?;
    let confidence_level = row
        .try_get("confidence_level")
        .context("failed to parse confidence_level in fetch_latest_vehicle_kpis_postgres")?;
    let sample_count = row
        .try_get("sample_count")
        .context("failed to parse sample_count in fetch_latest_vehicle_kpis_postgres")?;

    Ok(KpiMetric {
        kpi_key,
        value,
        unit,
        direction,
        confidence_level,
        sample_count,
    })
}
