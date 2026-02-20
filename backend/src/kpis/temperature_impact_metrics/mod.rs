use std::collections::BTreeMap;

use sqlx::SqlitePool;
use sqlx::sqlite::SqliteRow;

use super::temperature_impact_queries::fetch_cohort_kpi_values;
use crate::{ApiError, KpiMetric};

mod percentile_benchmarks;
mod row_mapper;

use percentile_benchmarks::compute_percentile_benchmark;
use row_mapper::map_temperature_kpi_row;

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
        // Translate each SQL row into the stable API metric shape first.
        let metric = map_temperature_kpi_row(&row)?;

        // Pull the peer cohort for this KPI so percentile math runs per metric key.
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

        // Track the largest cohort and compute the benchmark percentile for the metric.
        cohort_size = cohort_size.max(values.len());
        let percentile = compute_percentile_benchmark(&values, &metric);
        percentiles.insert(metric.kpi_key.clone(), percentile);
        metrics.push(metric);
    }

    Ok(TemperatureImpactMetricPayload {
        metrics,
        cohort_size,
        percentiles,
    })
}
