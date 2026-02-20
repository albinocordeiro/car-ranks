use crate::RankingCohort;

use super::row_mapper::RankingRowSeed;

/// Builds cohort metadata from a parsed ranking row seed.
pub(super) fn cohort_from_seed(seed: &RankingRowSeed) -> RankingCohort {
    RankingCohort {
        cohort_key: seed.cohort_key.clone(),
        cohort_size: seed.cohort_size,
        sample_gate_passed: seed.sample_gate_passed,
    }
}
