use anyhow::{Context, Result};
use sqlx::SqlitePool;

use super::super::{KPI_TIMEFRAMES, NON_TEMPERATURE_RANKING_TYPES};

/// Clears existing non-temperature snapshots before a full recompute pass.
pub(super) async fn clear_non_temperature_snapshots(pool: &SqlitePool) -> Result<()> {
    for timeframe in KPI_TIMEFRAMES {
        for ranking_type in NON_TEMPERATURE_RANKING_TYPES {
            sqlx::query(
                r#"
                DELETE FROM vehicle_kpi_snapshot
                WHERE ranking_type = ?
                  AND timeframe = ?
                "#,
            )
            .bind(ranking_type)
            .bind(timeframe)
            .execute(pool)
            .await
            .with_context(|| {
                format!(
                    "failed to clear KPI snapshots for ranking_type {} timeframe {}",
                    ranking_type, timeframe
                )
            })?;
        }
    }

    Ok(())
}
