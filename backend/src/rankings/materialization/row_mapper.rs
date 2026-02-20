use anyhow::Context;
use sqlx::Row;
use sqlx::sqlite::SqliteRow;
use uuid::Uuid;

use crate::ApiError;

/// Fully parsed ranking row seed used before KPI map hydration.
pub(super) struct RankingRowSeed {
    pub(super) vehicle_uid_str: String,
    pub(super) vehicle_uid: Uuid,
    pub(super) rank: i64,
    pub(super) score: f64,
    pub(super) confidence_level: String,
    pub(super) cohort_key: String,
    pub(super) cohort_size: i64,
    pub(super) sample_gate_passed: bool,
}

/// Parses one ranking snapshot row into a typed seed.
pub(super) fn map_ranking_row_seed(row: &SqliteRow) -> Result<RankingRowSeed, ApiError> {
    let vehicle_uid_str: String = row
        .try_get("vehicle_uid")
        .context("failed to parse ranking vehicle_uid")?;
    let vehicle_uid =
        Uuid::parse_str(&vehicle_uid_str).context("invalid UUID stored in ranking row")?;

    Ok(RankingRowSeed {
        vehicle_uid_str,
        vehicle_uid,
        rank: row
            .try_get("rank_position")
            .context("failed to parse rank_position")?,
        score: row.try_get("score").context("failed to parse score")?,
        confidence_level: row
            .try_get("confidence_level")
            .context("failed to parse confidence_level")?,
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
    })
}
