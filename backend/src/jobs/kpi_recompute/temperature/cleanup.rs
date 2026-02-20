use anyhow::{Context, Result};
use sqlx::SqlitePool;

/// Clears existing temperature-impact snapshots before recomputation.
pub(super) async fn clear_temperature_snapshots(pool: &SqlitePool) -> Result<()> {
    for timeframe in super::KPI_TIMEFRAMES {
        sqlx::query(
            r#"
            DELETE FROM vehicle_kpi_snapshot
            WHERE ranking_type = 'ev_temperature_impact'
              AND timeframe = ?
            "#,
        )
        .bind(timeframe)
        .execute(pool)
        .await
        .with_context(|| format!("failed to clear KPI snapshots for timeframe {}", timeframe))?;
    }

    Ok(())
}
