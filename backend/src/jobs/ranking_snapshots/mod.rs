use anyhow::Result;
use sqlx::SqlitePool;

mod non_temperature;
mod temperature;

/// Public entrypoint for temperature-impact ranking rebuilds.
///
/// The orchestration layer calls this wrapper so callers do not need to know
/// which file holds the concrete implementation.
pub(crate) async fn rebuild_temperature_rankings(pool: &SqlitePool) -> Result<usize> {
    temperature::rebuild_temperature_rankings(pool).await
}

/// Public entrypoint for non-temperature ranking rebuilds.
///
/// Keeping this wrapper in `mod.rs` gives the module a clean facade while
/// `non_temperature.rs` can focus on only one ranking family.
pub(crate) async fn rebuild_non_temperature_rankings(pool: &SqlitePool) -> Result<usize> {
    non_temperature::rebuild_non_temperature_rankings(pool).await
}
