use std::collections::BTreeMap;

use anyhow::Context;
use sqlx::Row;
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteRow;

use super::temperature_impact_queries::fetch_cohort_kpi_values;
use crate::{ApiError, KpiMetric, percentile_rank};

/// Materialized KPI payload and benchmark metadata for temperature-impact APIs.
pub(super) struct TemperatureImpactMetricPayload {
    pub(super) metrics: Vec<KpiMetric>,
    pub(super) cohort_size: usize,
    pub(super) percentiles: BTreeMap<String, i64>,
}

/// Converts raw KPI rows into API metrics and percentile benchmarks.
///
/// This helper keeps row parsing and cohort percentile lookups out of the HTTP
/// handler so request orchestration and metric materialization stay separate.
pub(super) async fn build_temperature_impact_metric_payload(
    pool: &SqlitePool,
    rows: Vec<SqliteRow>,
    timeframe: &str,
    baseline_bin: &str,
    compare_bin: &str,
    make: &str,
    model: &str,
) -> Result<TemperatureImpactMetricPayload, ApiError> {
    let mut metrics = Vec::with_capacity(rows.len());
    let mut percentiles = BTreeMap::new();
    let mut cohort_size = 0usize;

    for row in rows {
        let metric = map_temperature_kpi_row(&row)?;
        let values = fetch_cohort_kpi_values(
            pool,
            &metric.kpi_key,
            timeframe,
            baseline_bin,
            compare_bin,
            make,
            model,
        )
        .await?;

        cohort_size = cohort_size.max(values.len());
        let percentile = percentile_rank(
            &values,
            metric.value,
            metric.direction.as_str() == "higher_is_better",
        );
        percentiles.insert(metric.kpi_key.clone(), percentile);
        metrics.push(metric);
    }

    Ok(TemperatureImpactMetricPayload {
        metrics,
        cohort_size,
        percentiles,
    })
}

/// Maps one SQL row into the `KpiMetric` API shape with parse context.
fn map_temperature_kpi_row(row: &SqliteRow) -> Result<KpiMetric, ApiError> {
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
