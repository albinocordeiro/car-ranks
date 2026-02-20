use anyhow::{Context, Result};
use sqlx::PgPool;

/// Removes existing native charging KPI rows before rebuilding snapshots.
pub(super) async fn clear_native_charging_kpi_snapshots_postgres(pool: &PgPool) -> Result<()> {
    clear_kpi_snapshots_for_ranking_type(pool, "ev_charging_performance").await
}

/// Removes existing native range-efficiency KPI rows before rebuilding snapshots.
pub(super) async fn clear_native_range_kpi_snapshots_postgres(pool: &PgPool) -> Result<()> {
    clear_kpi_snapshots_for_ranking_type(pool, "ev_range_efficiency").await
}

/// Removes existing native temperature-impact KPI rows before rebuilding snapshots.
pub(super) async fn clear_native_temperature_kpi_snapshots_postgres(pool: &PgPool) -> Result<()> {
    clear_kpi_snapshots_for_ranking_type(pool, "ev_temperature_impact").await
}

async fn clear_kpi_snapshots_for_ranking_type(pool: &PgPool, ranking_type: &str) -> Result<()> {
    sqlx::query(
        r#"
        DELETE FROM vehicle_kpi_snapshot
        WHERE ranking_type = $1
        "#,
    )
    .bind(ranking_type)
    .execute(pool)
    .await
    .with_context(|| {
        format!(
            "failed to clear native postgres KPI snapshots for {}",
            ranking_type
        )
    })?;

    Ok(())
}
