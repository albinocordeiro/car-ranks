/// Computes the final charging-performance score from acceptance and retention components.
///
/// When retention is unavailable, acceptance becomes the fallback headline score.
pub(super) fn compute_charging_performance_score(
    acceptance_score: f64,
    retention_score: Option<f64>,
) -> f64 {
    if let Some(retention_score) = retention_score {
        (0.6 * acceptance_score + 0.4 * retention_score).clamp(0.0, 100.0)
    } else {
        acceptance_score.clamp(0.0, 100.0)
    }
}
