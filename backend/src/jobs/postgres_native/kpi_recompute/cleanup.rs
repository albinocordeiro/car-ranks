use anyhow::{Context, Result};
use sqlx::PgPool;

/// Removes existing native charging KPI rows before rebuilding snapshots.
pub(super) async fn clear_native_charging_kpi_snapshots_postgres(pool: &PgPool) -> Result<()> {
    sqlx::query(
        r#"
        DELETE FROM vehicle_kpi_snapshot
        WHERE ranking_type = 'ev_charging_performance'
        "#,
    )
    .execute(pool)
    .await
    .context("failed to clear native postgres charging KPI snapshots")?;

    Ok(())
}
