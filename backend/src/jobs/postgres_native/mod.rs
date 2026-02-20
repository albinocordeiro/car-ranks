use anyhow::Result;
use sqlx::PgPool;

mod charging_sessions;
mod composite;
mod kpi_recompute;
mod ranking_snapshots;

/// Runs native Postgres stages that must happen before bridge output sync.
///
/// These stages produce charging artifacts that are preserved when SQLite output
/// rows are copied back into Postgres.
pub(super) async fn run_native_postgres_pre_bridge_stages(
    pg_pool: &PgPool,
) -> Result<NativePreBridgeSummary> {
    let charging_sessions_upserted = charging_sessions::build_charging_sessions(pg_pool).await?;
    let charging_kpi_rows_upserted =
        kpi_recompute::recompute_charging_performance_kpis_postgres(pg_pool).await?;
    let charging_ranking_rows_upserted =
        ranking_snapshots::rebuild_charging_rankings_postgres(pg_pool).await?;

    Ok(NativePreBridgeSummary {
        charging_sessions_upserted,
        charging_kpi_rows_upserted,
        charging_ranking_rows_upserted,
    })
}

/// Runs native Postgres stages that depend on bridge-synced non-charging families.
pub(super) async fn run_native_postgres_post_bridge_stages(
    pg_pool: &PgPool,
) -> Result<NativePostBridgeSummary> {
    let range_ranking_rows_upserted =
        ranking_snapshots::rebuild_range_rankings_postgres(pg_pool).await?;
    let composite_summary = composite::recompute_composite_outputs_postgres(pg_pool).await?;
    Ok(NativePostBridgeSummary {
        range_ranking_rows_upserted,
        composite_kpi_rows_upserted: composite_summary.composite_kpi_rows_upserted,
        composite_ranking_rows_upserted: composite_summary.composite_ranking_rows_upserted,
    })
}

/// Output summary of pre-bridge native Postgres stages.
pub(super) struct NativePreBridgeSummary {
    pub(super) charging_sessions_upserted: usize,
    pub(super) charging_kpi_rows_upserted: usize,
    pub(super) charging_ranking_rows_upserted: usize,
}

/// Output summary of post-bridge native Postgres stages.
pub(super) struct NativePostBridgeSummary {
    pub(super) range_ranking_rows_upserted: usize,
    pub(super) composite_kpi_rows_upserted: usize,
    pub(super) composite_ranking_rows_upserted: usize,
}
