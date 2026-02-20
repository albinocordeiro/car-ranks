use anyhow::Result;
use sqlx::PgPool;

mod charging_sessions;
mod kpi_recompute;

/// Runs native Postgres stages that no longer require SQLite bridging.
pub(super) async fn run_native_postgres_stages(pg_pool: &PgPool) -> Result<NativeStageSummary> {
    let charging_sessions_upserted = charging_sessions::build_charging_sessions(pg_pool).await?;
    let charging_kpi_rows_upserted =
        kpi_recompute::recompute_charging_performance_kpis_postgres(pg_pool).await?;

    Ok(NativeStageSummary {
        charging_sessions_upserted,
        charging_kpi_rows_upserted,
    })
}

/// Output summary of native Postgres job stages.
pub(super) struct NativeStageSummary {
    pub(super) charging_sessions_upserted: usize,
    pub(super) charging_kpi_rows_upserted: usize,
}
