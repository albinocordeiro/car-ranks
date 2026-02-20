use anyhow::{Context, Result};
use sqlx::SqlitePool;

use self::cohort_build::build_non_temperature_cohorts;
use self::persist::persist_ranked_non_temperature_cohorts;
use self::vehicle_catalog::fetch_vehicle_catalog_rows;
use super::{NON_TEMPERATURE_RANKING_TYPES, SNAPSHOT_TIMEFRAMES};

mod cohort_build;
mod persist;
mod vehicle_catalog;

/// Rebuild non-temperature rankings using the latest per-vehicle KPI sets.
pub(super) async fn rebuild_non_temperature_rankings(pool: &SqlitePool) -> Result<usize> {
    let mut upserted_rows = 0usize;
    let vehicle_rows = fetch_vehicle_catalog_rows(pool).await?;

    for timeframe in SNAPSHOT_TIMEFRAMES {
        for ranking_type in NON_TEMPERATURE_RANKING_TYPES {
            sqlx::query(
                r#"
                DELETE FROM cohort_ranking_snapshot
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
                    "failed to clear ranking snapshots for {} {}",
                    ranking_type, timeframe
                )
            })?;

            let ranking_snapshot_ts = crate::now_str();
            let cohorts =
                build_non_temperature_cohorts(pool, &vehicle_rows, ranking_type, timeframe).await?;
            upserted_rows += persist_ranked_non_temperature_cohorts(
                pool,
                ranking_type,
                timeframe,
                &ranking_snapshot_ts,
                cohorts,
            )
            .await?;
        }
    }

    Ok(upserted_rows)
}
