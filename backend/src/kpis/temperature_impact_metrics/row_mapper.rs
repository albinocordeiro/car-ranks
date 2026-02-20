use anyhow::Context;
use sqlx::Row;
use sqlx::sqlite::SqliteRow;

use crate::{ApiError, KpiMetric};

/// Maps one SQL row into the `KpiMetric` API shape with parse context.
pub(super) fn map_temperature_kpi_row(row: &SqliteRow) -> Result<KpiMetric, ApiError> {
    let kpi_key: String = row.try_get("kpi_key").context("failed to parse kpi_key")?;
    let value: f64 = row
        .try_get("kpi_value")
        .context("failed to parse kpi_value")?;
    let unit = row
        .try_get::<Option<String>, _>("kpi_unit")
        .context("failed to parse kpi_unit")?
        .unwrap_or_else(|| "score".to_string());
    let direction: String = row
        .try_get("direction")
        .context("failed to parse direction")?;
    let confidence_level: String = row
        .try_get("confidence_level")
        .context("failed to parse confidence_level")?;
    let sample_count: i64 = row
        .try_get("sample_count")
        .context("failed to parse sample_count")?;

    Ok(KpiMetric {
        kpi_key,
        value,
        unit,
        direction,
        confidence_level,
        sample_count,
    })
}
