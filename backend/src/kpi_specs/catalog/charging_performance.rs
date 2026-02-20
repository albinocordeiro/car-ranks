use super::super::LockedKpiSpec;

pub(super) const TEMP_ADJUSTED_CHARGE_ACCEPTANCE_SCORE: LockedKpiSpec = LockedKpiSpec {
    ranking_type: "ev_charging_performance",
    kpi_key: "temp_adjusted_charge_acceptance_score",
    formula: "clamp(100 * median(all_charge_kw) / median(mild_charge_kw), 0, 120)",
    required_signals: &["ev.charging_state", "ev.charge_power_kw", "ev.soc_pct"],
    optional_signals: &[
        "ev.battery_temp_c",
        "environment.ambient_temp_c",
        "ev.charger_type",
    ],
};

pub(super) const COLD_WEATHER_CHARGE_SPEED_RETENTION: LockedKpiSpec = LockedKpiSpec {
    ranking_type: "ev_charging_performance",
    kpi_key: "cold_weather_charge_speed_retention",
    formula: "100 * median(cold_charge_kw) / median(mild_charge_kw)",
    required_signals: &["ev.charging_state", "ev.charge_power_kw", "ev.soc_pct"],
    optional_signals: &[
        "ev.battery_temp_c",
        "environment.ambient_temp_c",
        "ev.charger_type",
    ],
};

pub(super) const CHARGING_PERFORMANCE_SCORE: LockedKpiSpec = LockedKpiSpec {
    ranking_type: "ev_charging_performance",
    kpi_key: "charging_performance_score",
    formula: "0.6 * temp_adjusted_charge_acceptance_score + 0.4 * cold_weather_charge_speed_retention",
    required_signals: &["ev.charging_state", "ev.charge_power_kw", "ev.soc_pct"],
    optional_signals: &["ev.battery_temp_c", "environment.ambient_temp_c"],
};
