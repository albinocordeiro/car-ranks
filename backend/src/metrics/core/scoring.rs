use std::collections::BTreeMap;

/// Score temperature-impact rankings from the three retained KPI signals.
/// The weighting keeps range retention as the dominant signal while still
/// accounting for charging retention and thermal sensitivity.
pub(crate) fn score_temperature_impact(
    range_retention: Option<f64>,
    charge_retention: Option<f64>,
    sensitivity: Option<f64>,
) -> f64 {
    let range = range_retention.unwrap_or(0.0).clamp(0.0, 200.0);
    let charge = charge_retention.unwrap_or(0.0).clamp(0.0, 200.0);
    let sensitivity = sensitivity.unwrap_or(50.0).clamp(0.0, 100.0);

    let sensitivity_component = (100.0 - (sensitivity * 2.0).clamp(0.0, 100.0)).clamp(0.0, 100.0);

    (0.45 * range + 0.35 * charge + 0.20 * sensitivity_component).clamp(0.0, 100.0)
}

/// Convert KPI snapshots into a single ranking score for non-temperature rankings.
pub(crate) fn score_from_kpi_map(ranking_type: &str, kpis: &BTreeMap<String, f64>) -> f64 {
    match ranking_type {
        "ev_range_efficiency" => kpis
            .get("ev_range_efficiency_score")
            .copied()
            .or_else(|| {
                let est = kpis.get("ev_estimated_practical_range").copied()?;
                let efficiency_component =
                    if let Some(net_eff) = kpis.get("ev_net_energy_efficiency").copied() {
                        (100.0 - (net_eff / 3.0)).clamp(0.0, 100.0)
                    } else {
                        let depletion = kpis
                            .get("soc_depletion_rate_per_100km")
                            .copied()
                            .unwrap_or(50.0);
                        (100.0 - depletion).clamp(0.0, 100.0)
                    };
                let range_component = (est / 4.0).clamp(0.0, 100.0);
                Some((0.65 * efficiency_component + 0.35 * range_component).clamp(0.0, 100.0))
            })
            .unwrap_or(0.0),
        "ev_charging_performance" => kpis
            .get("charging_performance_score")
            .copied()
            .or_else(|| {
                let acceptance = kpis
                    .get("temp_adjusted_charge_acceptance_score")
                    .copied()
                    .unwrap_or(0.0);
                let retention = kpis
                    .get("cold_weather_charge_speed_retention")
                    .copied()
                    .unwrap_or(acceptance);
                Some((0.6 * acceptance + 0.4 * retention).clamp(0.0, 100.0))
            })
            .unwrap_or(0.0),
        "ev_composite" => kpis
            .get("ev_composite_score")
            .copied()
            .unwrap_or(0.0)
            .clamp(0.0, 100.0),
        _ => 0.0,
    }
}

/// Converts a set of KPI confidence levels to one aggregate confidence.
pub(crate) fn confidence_from_kpi_metrics(kpis: &[crate::KpiMetric]) -> &'static str {
    if kpis.is_empty() {
        return "preview";
    }
    if kpis.iter().any(|kpi| kpi.confidence_level == "preview") {
        "preview"
    } else if kpis.iter().any(|kpi| kpi.confidence_level == "medium") {
        "medium"
    } else {
        "stable"
    }
}
