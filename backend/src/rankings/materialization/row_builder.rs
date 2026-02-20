use std::collections::BTreeMap;

use crate::RankingRow;

use super::row_mapper::RankingRowSeed;

/// Builds the API ranking row from a typed seed and hydrated KPI details.
pub(super) fn build_ranking_row(seed: RankingRowSeed, kpis: BTreeMap<String, f64>) -> RankingRow {
    RankingRow {
        rank: seed.rank,
        vehicle_uid: seed.vehicle_uid,
        score: seed.score,
        confidence_level: seed.confidence_level,
        kpis,
    }
}
