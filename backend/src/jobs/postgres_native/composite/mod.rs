use anyhow::Result;
use sqlx::PgPool;

mod health_penalty;
mod kpi_snapshots;

/// Rebuilds native Postgres composite KPI and ranking outputs.
pub(super) async fn recompute_composite_outputs_postgres(
    pool: &PgPool,
) -> Result<CompositeNativeSummary> {
    let composite_kpi_rows_upserted =
        kpi_snapshots::recompute_composite_kpis_postgres(pool).await?;
    let composite_ranking_rows_upserted =
        super::ranking_snapshots::rebuild_composite_rankings_postgres(pool).await?;

    Ok(CompositeNativeSummary {
        composite_kpi_rows_upserted,
        composite_ranking_rows_upserted,
    })
}

/// Summary of native Postgres composite output stages.
pub(super) struct CompositeNativeSummary {
    pub(super) composite_kpi_rows_upserted: usize,
    pub(super) composite_ranking_rows_upserted: usize,
}
