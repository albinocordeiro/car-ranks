use std::collections::HashMap;

use anyhow::{Context, Result};
use sqlx::SqlitePool;

use crate::jobs::ranking_snapshots::persistence::insert_cohort_ranking_snapshot;

use super::cohort_build::CohortEntry;

/// Sorts each cohort and persists ranked rows into snapshot storage.
pub(super) async fn persist_ranked_non_temperature_cohorts(
    pool: &SqlitePool,
    ranking_type: &str,
    timeframe: &str,
    ranking_snapshot_ts: &str,
    cohorts: HashMap<String, Vec<CohortEntry>>,
) -> Result<usize> {
    let mut upserted_rows = 0usize;

    for (cohort_key, mut entries) in cohorts {
        entries.sort_by(|a, b| crate::cmp_f64_desc(a.score, b.score));
        let cohort_size = entries.len() as i64;
        let sample_gate_passed = cohort_size >= 10;

        for (index, entry) in entries.into_iter().enumerate() {
            insert_cohort_ranking_snapshot(
                pool,
                ranking_type,
                timeframe,
                "all",
                &cohort_key,
                cohort_size,
                sample_gate_passed,
                &entry.vehicle_uid,
                (index + 1) as i64,
                entry.score,
                &entry.confidence_level,
                ranking_snapshot_ts,
            )
            .await
            .with_context(|| {
                format!(
                    "failed to insert non-temperature ranking row for {} {}",
                    ranking_type, timeframe
                )
            })?;

            upserted_rows += 1;
        }
    }

    Ok(upserted_rows)
}
