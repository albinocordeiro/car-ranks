use anyhow::Result;
use sqlx::SqlitePool;

mod non_temperature;
mod snapshot_writer;
mod temperature;

/// Timeframes materialized by both KPI recompute jobs.
pub(super) const KPI_TIMEFRAMES: [&str; 3] = ["30d", "90d", "180d"];

/// Ranking families recomputed by the non-temperature KPI job.
pub(super) const NON_TEMPERATURE_RANKING_TYPES: [&str; 3] = [
    "ev_range_efficiency",
    "ev_charging_performance",
    "ev_composite",
];

/// Ranking type and bin metadata for temperature-impact KPI snapshots.
pub(super) const TEMPERATURE_RANKING_TYPE: &str = "ev_temperature_impact";
pub(super) const TEMPERATURE_BASELINE_BIN: &str = "mild";
pub(super) const TEMPERATURE_COMPARE_BIN: &str = "cold";
pub(super) const TEMPERATURE_OUTPUT_BINS: [&str; 2] = ["all", "cold"];

/// Facade used by the orchestration layer to recompute temperature KPI sets.
pub(crate) async fn recompute_temperature_kpis(pool: &SqlitePool) -> Result<(usize, usize)> {
    temperature::recompute_temperature_kpis(pool).await
}

/// Facade used by the orchestration layer to recompute non-temperature KPI sets.
pub(crate) async fn recompute_non_temperature_kpis(pool: &SqlitePool) -> Result<(usize, usize)> {
    non_temperature::recompute_non_temperature_kpis(pool).await
}
