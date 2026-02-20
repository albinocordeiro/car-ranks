use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use crate::MetricCalc;

mod charging_performance;
mod charging_performance_buckets;
mod charging_performance_scoring;
mod composite;
mod composite_health;
mod core;
mod range_efficiency;
mod range_efficiency_regeneration;
mod range_efficiency_scoring;
mod range_efficiency_series;
mod range_efficiency_snapshots;
mod temperature_charge_retention;
mod temperature_impact;
mod temperature_impact_scoring;
mod temperature_impact_series;
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

pub(crate) async fn compute_range_efficiency_metrics(
    pool: &SqlitePool,
    vehicle_uid: &str,
    cutoff: DateTime<Utc>,
) -> Result<Vec<MetricCalc>> {
    range_efficiency::compute_range_efficiency_metrics(pool, vehicle_uid, cutoff).await
}

pub(crate) async fn compute_charging_performance_metrics(
    pool: &SqlitePool,
    vehicle_uid: &str,
    cutoff: DateTime<Utc>,
) -> Result<Vec<MetricCalc>> {
    charging_performance::compute_charging_performance_metrics(pool, vehicle_uid, cutoff).await
}

pub(crate) async fn compute_composite_metrics(
    pool: &SqlitePool,
    vehicle_uid: &str,
    cutoff: DateTime<Utc>,
    range_metrics: &[MetricCalc],
    charging_metrics: &[MetricCalc],
) -> Result<Vec<MetricCalc>> {
    composite::compute_composite_metrics(pool, vehicle_uid, cutoff, range_metrics, charging_metrics)
        .await
}

pub(crate) async fn compute_vehicle_metrics(
    pool: &SqlitePool,
    vehicle_uid: &str,
    cutoff: DateTime<Utc>,
) -> Result<Vec<MetricCalc>> {
    temperature_impact::compute_vehicle_metrics(pool, vehicle_uid, cutoff).await
}
