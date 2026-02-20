use anyhow::{Context, Result};
use sqlx::SqlitePool;

use super::super::{KPI_TIMEFRAMES, TEMPERATURE_RANKING_TYPE};

/// Clears existing temperature-impact snapshots before recomputation.
pub(super) async fn clear_temperature_snapshots(pool: &SqlitePool) -> Result<()> {
    for timeframe in KPI_TIMEFRAMES {
        sqlx::query(
            r#"
            DELETE FROM vehicle_kpi_snapshot
            WHERE ranking_type = ?
              AND timeframe = ?
            "#,
        )
        .bind(TEMPERATURE_RANKING_TYPE)
        .bind(timeframe)
        .execute(pool)
        .await
        .with_context(|| format!("failed to clear KPI snapshots for timeframe {}", timeframe))?;
    }

    Ok(())
}
