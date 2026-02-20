/// Computes the temperature-adjusted acceptance score from all vs mild charging power.
///
/// Mild-weather median is the baseline expectation; all-weather median reflects
/// observed real-world charging behavior across conditions.
pub(super) fn compute_acceptance_score(all_power: &[f64], mild_power: &[f64]) -> f64 {
    let all_median = super::super::median(all_power.to_vec()).unwrap_or(0.0);
    let mild_median = super::super::median(mild_power.to_vec()).unwrap_or(all_median.max(1e-6));
    if mild_median > 0.0 {
        (100.0 * all_median / mild_median).clamp(0.0, 120.0)
    } else {
        100.0
    }
}
