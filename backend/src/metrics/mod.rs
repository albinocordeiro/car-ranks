use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::MetricCalc;

mod core;
mod range_efficiency;
mod range_efficiency_accumulator;
mod range_efficiency_additional;
mod range_efficiency_baseline;
mod range_efficiency_regeneration;
mod range_efficiency_scoring;
mod range_efficiency_series;
mod range_efficiency_snapshots;
mod temperature_charge_retention;
mod temperature_drive_metrics;
mod temperature_impact;
mod temperature_impact_accumulator;
mod temperature_impact_scoring;
mod temperature_impact_series;
mod temperature_impact_snapshots;
mod temperature_regression;

#[allow(unused_imports)]
pub(crate) use core::TemperatureSampleGates;

pub(crate) use core::{
    confidence_from_kpi_metrics, confidence_from_samples, max_value, mean, median,
    score_from_kpi_map, score_temperature_impact, wh_per_km_from_soc_delta,
};

/// Read and materialize temperature KPI gate settings.
///
/// A wrapper is kept at the crate-level `metrics` module so existing callers do
/// not need to know about the internal `core` submodule.
pub(crate) fn temperature_sample_gates() -> core::TemperatureSampleGates {
    core::temperature_sample_gates()
}

/// Rebuilds range-efficiency KPIs directly from Postgres observations.
pub(crate) async fn compute_range_efficiency_metrics_postgres(
    pool: &PgPool,
    vehicle_uid: &str,
    cutoff: DateTime<Utc>,
) -> Result<Vec<MetricCalc>> {
    range_efficiency::compute_range_efficiency_metrics_postgres(pool, vehicle_uid, cutoff).await
}

/// Rebuilds temperature-impact KPIs directly from Postgres observations.
pub(crate) async fn compute_vehicle_metrics_postgres(
    pool: &PgPool,
    vehicle_uid: &str,
    cutoff: DateTime<Utc>,
) -> Result<Vec<MetricCalc>> {
    temperature_impact::compute_vehicle_metrics_postgres(pool, vehicle_uid, cutoff).await
}
