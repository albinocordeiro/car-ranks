use anyhow::Result;
use sqlx::PgPool;

mod charging_sessions;
mod composite;
mod kpi_recompute;
mod ranking_snapshots;

/// Runs the full KPI/ranking recompute pipeline natively in Postgres.
pub(super) async fn run_native_postgres_job(pg_pool: &PgPool) -> Result<NativeJobSummary> {
    let charging_sessions_upserted = charging_sessions::build_charging_sessions(pg_pool).await?;
    let charging_kpi_rows_upserted =
        kpi_recompute::recompute_charging_performance_kpis_postgres(pg_pool).await?;
    let range_kpi_rows_upserted =
        kpi_recompute::recompute_range_efficiency_kpis_postgres(pg_pool).await?;
    let temperature_kpi_rows_upserted =
        kpi_recompute::recompute_temperature_impact_kpis_postgres(pg_pool).await?;

    let composite_summary = composite::recompute_composite_outputs_postgres(pg_pool).await?;

    let charging_ranking_rows_upserted =
        ranking_snapshots::rebuild_charging_rankings_postgres(pg_pool).await?;
    let range_ranking_rows_upserted =
        ranking_snapshots::rebuild_range_rankings_postgres(pg_pool).await?;
    let temperature_ranking_rows_upserted =
        ranking_snapshots::rebuild_temperature_rankings_postgres(pg_pool).await?;

    Ok(NativeJobSummary {
        charging_sessions_upserted,
        charging_kpi_rows_upserted,
        range_kpi_rows_upserted,
        temperature_kpi_rows_upserted,
        composite_kpi_rows_upserted: composite_summary.composite_kpi_rows_upserted,
        charging_ranking_rows_upserted,
        range_ranking_rows_upserted,
        composite_ranking_rows_upserted: composite_summary.composite_ranking_rows_upserted,
        temperature_ranking_rows_upserted,
    })
}

/// Counts vehicles available for recompute reporting.
pub(super) async fn count_postgres_vehicles(pg_pool: &PgPool) -> Result<usize> {
    let count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM vehicle
        "#,
    )
    .fetch_one(pg_pool)
    .await?;
    Ok(count.max(0) as usize)
}

/// Output summary of native Postgres recompute stages.
pub(super) struct NativeJobSummary {
    pub(super) charging_sessions_upserted: usize,
    pub(super) charging_kpi_rows_upserted: usize,
    pub(super) range_kpi_rows_upserted: usize,
    pub(super) temperature_kpi_rows_upserted: usize,
    pub(super) composite_kpi_rows_upserted: usize,
    pub(super) charging_ranking_rows_upserted: usize,
    pub(super) range_ranking_rows_upserted: usize,
    pub(super) composite_ranking_rows_upserted: usize,
    pub(super) temperature_ranking_rows_upserted: usize,
}

impl NativeJobSummary {
    /// Total KPI snapshot rows upserted by this native run.
    pub(super) fn total_kpi_rows_upserted(&self) -> usize {
        self.charging_kpi_rows_upserted
            + self.range_kpi_rows_upserted
            + self.temperature_kpi_rows_upserted
            + self.composite_kpi_rows_upserted
    }

    /// Total ranking snapshot rows upserted by this native run.
    pub(super) fn total_ranking_rows_upserted(&self) -> usize {
        self.charging_ranking_rows_upserted
            + self.range_ranking_rows_upserted
            + self.composite_ranking_rows_upserted
            + self.temperature_ranking_rows_upserted
    }
}
