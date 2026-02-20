use anyhow::Context;
use sqlx::Row;
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteRow;
use uuid::Uuid;

use crate::{ApiError, RankingCohort, RankingRow};

use super::kpi_details::fetch_latest_kpi_map;

/// Converts raw ranking snapshot rows into API rows plus cohort metadata.
pub(super) async fn materialize_ranking_rows(
    pool: &SqlitePool,
    rows: Vec<SqliteRow>,
    ranking_type: &str,
    timeframe: &str,
    temperature_bin: &str,
) -> Result<(Vec<RankingRow>, RankingCohort), ApiError> {
    let mut ranking_rows = Vec::new();
    let mut cohort = RankingCohort {
        cohort_key: "unknown".to_string(),
        cohort_size: 0,
        sample_gate_passed: false,
    };

    for row in rows {
        let vehicle_uid_str: String = row
            .try_get("vehicle_uid")
            .context("failed to parse ranking vehicle_uid")?;
        let vehicle_uid =
            Uuid::parse_str(&vehicle_uid_str).context("invalid UUID stored in ranking row")?;

        // Materialize KPI details for each row so rankings are self-explanatory to clients.
        let kpis = fetch_latest_kpi_map(
            pool,
            &vehicle_uid_str,
            ranking_type,
            timeframe,
            temperature_bin,
        )
        .await?;

        cohort = RankingCohort {
            cohort_key: row
                .try_get("cohort_key")
                .context("failed to parse cohort_key")?,
            cohort_size: row
                .try_get("cohort_size")
                .context("failed to parse cohort_size")?,
            sample_gate_passed: row
                .try_get::<i64, _>("sample_gate_passed")
                .context("failed to parse sample_gate_passed")?
                == 1,
        };

        ranking_rows.push(RankingRow {
            rank: row
                .try_get("rank_position")
                .context("failed to parse rank_position")?,
            vehicle_uid,
            score: row.try_get("score").context("failed to parse score")?,
            confidence_level: row
                .try_get("confidence_level")
                .context("failed to parse confidence_level")?,
            kpis,
        });
    }

    Ok((ranking_rows, cohort))
}
