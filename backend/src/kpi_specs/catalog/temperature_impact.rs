use super::super::LockedKpiSpec;

pub(super) const COLD_WEATHER_RANGE_RETENTION: LockedKpiSpec = LockedKpiSpec {
    ranking_type: "ev_temperature_impact",
    kpi_key: "cold_weather_range_retention",
    formula: "100 * median(cold_km_per_soc) / median(mild_km_per_soc)",
    required_signals: &[
        "distance.odometer",
        "ev.soc_pct",
        "environment.ambient_temp_c",
    ],
    optional_signals: &["ev.battery_temp_c"],
};

pub(super) const RANGE_TEMPERATURE_SENSITIVITY_INDEX: LockedKpiSpec = LockedKpiSpec {
    ranking_type: "ev_temperature_impact",
    kpi_key: "range_temperature_sensitivity_index",
    formula: "max(0, -slope(km_per_soc vs temp_c) * 10 / mild_km_per_soc * 100)",
    required_signals: &[
        "distance.odometer",
        "ev.soc_pct",
        "environment.ambient_temp_c",
    ],
    optional_signals: &["ev.battery_temp_c"],
};

pub(super) const COLD_WEATHER_CHARGE_SPEED_RETENTION: LockedKpiSpec = LockedKpiSpec {
    ranking_type: "ev_temperature_impact",
    kpi_key: "cold_weather_charge_speed_retention",
    formula: "100 * median(cold_charge_kw) / median(mild_charge_kw)",
    required_signals: &["ev.charging_state", "ev.charge_power_kw", "ev.soc_pct"],
    optional_signals: &["ev.battery_temp_c", "environment.ambient_temp_c"],
};
