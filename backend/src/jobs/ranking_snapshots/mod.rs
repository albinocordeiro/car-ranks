use anyhow::Result;
use sqlx::SqlitePool;

mod non_temperature;
mod persistence;
mod temperature;

/// Common snapshot timeframes used by all ranking rebuild jobs.
pub(super) const SNAPSHOT_TIMEFRAMES: [&str; 3] = ["30d", "90d", "180d"];

/// Ranking families that are rebuilt through the non-temperature job path.
pub(super) const NON_TEMPERATURE_RANKING_TYPES: [&str; 3] = [
    "ev_range_efficiency",
    "ev_charging_performance",
    "ev_composite",
];

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
