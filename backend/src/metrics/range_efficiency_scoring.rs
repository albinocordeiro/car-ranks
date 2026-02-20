use crate::MetricCalc;

use super::range_efficiency_series::RangeEfficiencySeries;

/// Converts a normalized range-efficiency series into persisted KPI metrics.
///
/// This function is intentionally pure so the caller can keep I/O concerns
/// (database fetching) separate from KPI math and confidence scoring.
pub(super) fn score_range_efficiency_series(
    series: RangeEfficiencySeries,
    default_usable_battery_kwh: f64,
) -> Vec<MetricCalc> {
    let RangeEfficiencySeries {
        km_per_soc_points,
        wh_per_km_points,
        urban_wh_per_km_points,
        highway_wh_per_km_points,
        power_windows,
        latest_soc,
    } = series;

    // The median distance-per-SOC is the anchor for all downstream range KPIs.
    let Some(median_km_per_soc) = super::median(km_per_soc_points.clone()) else {
        return Vec::new();
    };

    let mut metrics = Vec::new();
    let sample_count = km_per_soc_points.len() as i64;
    let soc_depletion_per_100km = if median_km_per_soc > 0.0 {
        100.0 / median_km_per_soc
    } else {
        100.0
    };
    let latest_soc = latest_soc.unwrap_or(50.0).clamp(0.0, 100.0);
    let estimated_range = (latest_soc * median_km_per_soc).max(0.0);

    // Prefer direct Wh/km observations, but keep a deterministic fallback based
    // on SOC depletion so the score remains stable when energy samples are sparse.
    let net_energy_efficiency = super::median(wh_per_km_points.clone())
        .unwrap_or((soc_depletion_per_100km * default_usable_battery_kwh / 10.0).max(0.0));
    let efficiency_component = (100.0 - (net_energy_efficiency / 3.0)).clamp(0.0, 100.0);
    let range_component = (estimated_range / 4.0).clamp(0.0, 100.0);
    let range_efficiency_score =
        (0.65 * efficiency_component + 0.35 * range_component).clamp(0.0, 100.0);

    metrics.push(build_metric(
        "ev_net_energy_efficiency",
        net_energy_efficiency,
        "Wh_per_km",
        "lower_is_better",
        sample_count,
    ));
    metrics.push(build_metric(
        "ev_estimated_practical_range",
        estimated_range,
        "km",
        "higher_is_better",
        sample_count,
    ));
    metrics.push(build_metric(
        "soc_depletion_rate_per_100km",
        soc_depletion_per_100km,
        "%_per_100km",
        "lower_is_better",
        sample_count,
    ));
    metrics.push(build_metric(
        "ev_range_efficiency_score",
        range_efficiency_score,
        "score",
        "higher_is_better",
        sample_count,
    ));

    if let Some(urban_efficiency) = super::median(urban_wh_per_km_points.clone()) {
        let urban_samples = urban_wh_per_km_points.len() as i64;
        metrics.push(build_metric(
            "ev_urban_efficiency",
            urban_efficiency,
            "Wh_per_km",
            "lower_is_better",
            urban_samples,
        ));
    }

    if let Some(highway_efficiency) = super::median(highway_wh_per_km_points.clone()) {
        let highway_samples = highway_wh_per_km_points.len() as i64;
        metrics.push(build_metric(
            "ev_highway_efficiency",
            highway_efficiency,
            "Wh_per_km",
            "lower_is_better",
            highway_samples,
        ));
    }

    if let Some(regen_metric) = regeneration_recovery_ratio_metric(&power_windows) {
        metrics.push(regen_metric);
    }

    metrics
}

/// Standardizes metric construction so every KPI row gets the same confidence
/// calculation policy and avoids duplicated field wiring.
fn build_metric(
    key: &'static str,
    value: f64,
    unit: &'static str,
    direction: &'static str,
    sample_count: i64,
) -> MetricCalc {
    MetricCalc {
        key,
        value,
        unit,
        direction,
        sample_count,
        confidence_level: super::confidence_from_samples(sample_count),
    }
}

/// Integrates traction and regeneration power traces into one recovery ratio KPI.
fn regeneration_recovery_ratio_metric(
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

    let regen_ratio = (100.0 * regen_wh / (regen_wh + traction_wh)).clamp(0.0, 100.0);
    Some(build_metric(
        "regeneration_recovery_ratio",
        regen_ratio,
        "%",
        "higher_is_better",
        regen_windows.max(1),
    ))
}
