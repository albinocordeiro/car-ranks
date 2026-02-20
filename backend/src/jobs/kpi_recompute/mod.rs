use anyhow::Result;
use sqlx::SqlitePool;

mod non_temperature;
mod snapshot_writer;
mod temperature;

/// Facade used by the orchestration layer to recompute temperature KPI sets.
pub(crate) async fn recompute_temperature_kpis(pool: &SqlitePool) -> Result<(usize, usize)> {
    temperature::recompute_temperature_kpis(pool).await
}

/// Facade used by the orchestration layer to recompute non-temperature KPI sets.
pub(crate) async fn recompute_non_temperature_kpis(pool: &SqlitePool) -> Result<(usize, usize)> {
    non_temperature::recompute_non_temperature_kpis(pool).await
}
