use crate::MetricCalc;

/// Integrates traction and regeneration power traces into one recovery ratio KPI.
pub(super) fn regeneration_recovery_ratio_metric(
    power_windows: &[(i64, Option<f64>, Option<f64>)],
) -> Option<MetricCalc> {
    let mut regen_wh = 0.0;
    let mut traction_wh = 0.0;
    let mut regen_windows = 0_i64;

    for window in power_windows.windows(2) {
        let dt_seconds = window[1].0 - window[0].0;
        if !(1..=300).contains(&dt_seconds) {
            continue;
        }

        let dt_hours = dt_seconds as f64 / 3600.0;
        let mut has_power_sample = false;

        if let Some(regen_kw) = window[0]
            .1
            .filter(|value| value.is_finite() && *value > 0.0)
        {
            regen_wh += regen_kw * dt_hours * 1000.0;
            has_power_sample = true;
        }
        if let Some(traction_kw) = window[0]
            .2
            .filter(|value| value.is_finite() && *value > 0.0)
        {
            traction_wh += traction_kw * dt_hours * 1000.0;
            has_power_sample = true;
        }

        if has_power_sample {
            regen_windows += 1;
        }
    }

    if regen_wh <= 0.0 || (regen_wh + traction_wh) <= 0.0 {
        return None;
    }

    let sample_count = regen_windows.max(1);
    let regen_ratio = (100.0 * regen_wh / (regen_wh + traction_wh)).clamp(0.0, 100.0);
    Some(MetricCalc {
        key: "regeneration_recovery_ratio",
        value: regen_ratio,
        unit: "%",
        direction: "higher_is_better",
        sample_count,
        confidence_level: super::confidence_from_samples(sample_count),
    })
}
