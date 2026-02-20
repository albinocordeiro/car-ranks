use sqlx::SqlitePool;
use sqlx::sqlite::SqliteRow;

use crate::{ApiError, RankingCohort, RankingRow};

use super::kpi_details::fetch_latest_kpi_map;

mod cohort_mapper;
mod row_builder;
mod row_mapper;

use cohort_mapper::cohort_from_seed;
use row_builder::build_ranking_row;
use row_mapper::map_ranking_row_seed;

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
        // Parse all persisted ranking fields into a typed seed first so downstream
        // steps do not have to repeat SQL conversion and error context.
        let seed = map_ranking_row_seed(&row)?;

        // Materialize KPI details for each row so rankings are self-explanatory to clients.
        let kpis = fetch_latest_kpi_map(
            pool,
            &seed.vehicle_uid_str,
            ranking_type,
            timeframe,
            temperature_bin,
        )
        .await?;

        cohort = cohort_from_seed(&seed);
        ranking_rows.push(build_ranking_row(seed, kpis));
    }

    Ok((ranking_rows, cohort))
}
