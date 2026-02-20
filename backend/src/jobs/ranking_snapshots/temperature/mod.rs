use anyhow::{Context, Result};
use sqlx::SqlitePool;

use self::cohorts::persist_ranked_temperature_cohorts;
use self::seeds::fetch_temperature_ranking_seeds;

mod cohorts;
mod seeds;

/// Rebuild temperature-impact rankings from the gated KPI snapshots.
pub(super) async fn rebuild_temperature_rankings(pool: &SqlitePool) -> Result<usize> {
    let mut upserted_rows = 0usize;

    for timeframe in ["30d", "90d", "180d"] {
        let ranking_snapshot_ts = crate::now_str();
        sqlx::query(
            r#"
            DELETE FROM cohort_ranking_snapshot
            WHERE ranking_type = 'ev_temperature_impact'
              AND timeframe = ?
            "#,
        )
        .bind(timeframe)
        .execute(pool)
        .await
        .with_context(|| format!("failed to clear rankings for timeframe {}", timeframe))?;

        let seeds = fetch_temperature_ranking_seeds(pool, timeframe).await?;
        upserted_rows +=
            persist_ranked_temperature_cohorts(pool, timeframe, &ranking_snapshot_ts, seeds)
                .await?;
    }

    Ok(upserted_rows)
}
