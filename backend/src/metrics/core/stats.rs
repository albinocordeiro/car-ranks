/// Converts SOC drop and distance to energy efficiency (Wh/km).
pub(crate) fn wh_per_km_from_soc_delta(
    delta_soc_pct: f64,
    delta_km: f64,
    usable_battery_kwh: f64,
) -> Option<f64> {
    if !delta_soc_pct.is_finite()
        || !delta_km.is_finite()
        || !usable_battery_kwh.is_finite()
        || delta_soc_pct <= 0.0
        || delta_km <= 0.0
        || usable_battery_kwh <= 0.0
    {
        return None;
    }

    let energy_wh = (delta_soc_pct / 100.0) * usable_battery_kwh * 1000.0;
    let wh_per_km = energy_wh / delta_km;
    if wh_per_km.is_finite() && wh_per_km > 0.0 {
        Some(wh_per_km)
    } else {
        None
    }
}

/// Mean helper that returns `None` for empty inputs.
pub(crate) fn mean(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    Some(values.iter().sum::<f64>() / values.len() as f64)
}

/// Max helper that ignores non-finite values.
pub(crate) fn max_value(values: &[f64]) -> Option<f64> {
    values
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .max_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal))
}

/// Median helper that ignores non-finite values.
pub(crate) fn median(mut values: Vec<f64>) -> Option<f64> {
    values.retain(|value| value.is_finite());
    if values.is_empty() {
        return None;
    }
    values.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    let mid = values.len() / 2;
    if values.len() % 2 == 0 {
        Some((values[mid - 1] + values[mid]) / 2.0)
    } else {
        Some(values[mid])
    }
}

/// Converts a raw sample count into the API confidence enum.
pub(crate) fn confidence_from_samples(sample_count: i64) -> &'static str {
    if sample_count >= 60 {
        "stable"
    } else if sample_count >= 20 {
        "medium"
    } else {
        "preview"
    }
}
