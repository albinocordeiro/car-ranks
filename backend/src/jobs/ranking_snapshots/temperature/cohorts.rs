use std::collections::HashMap;

use anyhow::Result;
use sqlx::SqlitePool;

use crate::jobs::ranking_snapshots::persistence::insert_cohort_ranking_snapshot;

use super::seeds::VehicleRankingSeed;

/// Scores seeds, groups them by cohort, and persists ranked snapshot rows.
pub(super) async fn persist_ranked_temperature_cohorts(
    pool: &SqlitePool,
    timeframe: &str,
    ranking_snapshot_ts: &str,
    seeds: Vec<VehicleRankingSeed>,
) -> Result<usize> {
    let mut cohorts: HashMap<String, Vec<(VehicleRankingSeed, f64)>> = HashMap::new();

    for seed in seeds {
        let score = crate::metrics::score_temperature_impact(
            seed.range_retention,
            seed.charge_retention,
            seed.sensitivity,
        );
        let cohort_key = format!(
            "bev|{}|{}|{}|{}",
            seed.make,
            seed.model,
            seed.trim,
            crate::year_band(seed.model_year)
        );
        cohorts.entry(cohort_key).or_default().push((seed, score));
    }

    let mut upserted_rows = 0usize;
    for (cohort_key, entries) in cohorts {
        let mut entries = entries;
        entries.sort_by(|a, b| crate::cmp_f64_desc(a.1, b.1));
        let cohort_size = entries.len() as i64;
        let sample_gate_passed = cohort_size >= 10;

        for (index, (seed, score)) in entries.into_iter().enumerate() {
            for bin in ["all", "cold"] {
                insert_cohort_ranking_snapshot(
                    pool,
                    "ev_temperature_impact",
                    timeframe,
                    bin,
                    &cohort_key,
                    cohort_size,
                    sample_gate_passed,
                    &seed.vehicle_uid,
                    (index + 1) as i64,
                    score,
                    &seed.confidence_level,
                    ranking_snapshot_ts,
                )
                .await?;

                upserted_rows += 1;
            }
        }
    }

    Ok(upserted_rows)
}
