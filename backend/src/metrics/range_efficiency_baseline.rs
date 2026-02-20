/// Baseline derived values used to emit range-efficiency KPI metrics.
pub(super) struct RangeEfficiencyBaseline {
    pub(super) sample_count: i64,
    pub(super) soc_depletion_per_100km: f64,
    pub(super) estimated_range: f64,
    pub(super) net_energy_efficiency: f64,
    pub(super) range_efficiency_score: f64,
}

/// Computes baseline range-efficiency values from series medians.
pub(super) fn compute_range_efficiency_baseline(
    km_per_soc_points: &[f64],
    wh_per_km_points: &[f64],
    latest_soc: Option<f64>,
    default_usable_battery_kwh: f64,
) -> Option<RangeEfficiencyBaseline> {
    // The median distance-per-SOC is the anchor for all downstream range KPIs.
    let median_km_per_soc = super::median(km_per_soc_points.to_vec())?;

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
    let net_energy_efficiency = super::median(wh_per_km_points.to_vec())
        .unwrap_or((soc_depletion_per_100km * default_usable_battery_kwh / 10.0).max(0.0));
    let efficiency_component = (100.0 - (net_energy_efficiency / 3.0)).clamp(0.0, 100.0);
    let range_component = (estimated_range / 4.0).clamp(0.0, 100.0);
    let range_efficiency_score =
        (0.65 * efficiency_component + 0.35 * range_component).clamp(0.0, 100.0);

    Some(RangeEfficiencyBaseline {
        sample_count,
        soc_depletion_per_100km,
        estimated_range,
        net_energy_efficiency,
        range_efficiency_score,
    })
}
