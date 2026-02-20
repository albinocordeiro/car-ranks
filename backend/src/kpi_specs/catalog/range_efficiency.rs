use super::super::LockedKpiSpec;

pub(super) const EV_NET_ENERGY_EFFICIENCY: LockedKpiSpec = LockedKpiSpec {
    ranking_type: "ev_range_efficiency",
    kpi_key: "ev_net_energy_efficiency",
    formula: "median(((delta_soc_pct/100) * DEFAULT_USABLE_BATTERY_KWH * 1000) / delta_km)",
    required_signals: &["distance.odometer", "ev.soc_pct"],
    optional_signals: &["power.battery_power_kw", "environment.ambient_temp_c"],
};

pub(super) const EV_ESTIMATED_PRACTICAL_RANGE: LockedKpiSpec = LockedKpiSpec {
    ranking_type: "ev_range_efficiency",
    kpi_key: "ev_estimated_practical_range",
    formula: "latest_soc_pct * median(delta_km / delta_soc_pct)",
    required_signals: &["distance.odometer", "ev.soc_pct"],
    optional_signals: &["environment.ambient_temp_c"],
};

pub(super) const EV_URBAN_EFFICIENCY: LockedKpiSpec = LockedKpiSpec {
    ranking_type: "ev_range_efficiency",
    kpi_key: "ev_urban_efficiency",
    formula: "median(ev_net_energy_efficiency for segments where speed.vehicle < 45 km/h)",
    required_signals: &["distance.odometer", "ev.soc_pct", "speed.vehicle"],
    optional_signals: &[],
};

pub(super) const EV_HIGHWAY_EFFICIENCY: LockedKpiSpec = LockedKpiSpec {
    ranking_type: "ev_range_efficiency",
    kpi_key: "ev_highway_efficiency",
    formula: "median(ev_net_energy_efficiency for segments where speed.vehicle >= 80 km/h)",
    required_signals: &["distance.odometer", "ev.soc_pct", "speed.vehicle"],
    optional_signals: &[],
};

pub(super) const REGENERATION_RECOVERY_RATIO: LockedKpiSpec = LockedKpiSpec {
    ranking_type: "ev_range_efficiency",
    kpi_key: "regeneration_recovery_ratio",
    formula: "100 * regen_energy_wh / (regen_energy_wh + traction_energy_wh) over integrated power windows",
    required_signals: &["ev.regen_power_kw", "ev.traction_power_kw"],
    optional_signals: &[],
};

pub(super) const SOC_DEPLETION_RATE_PER_100KM: LockedKpiSpec = LockedKpiSpec {
    ranking_type: "ev_range_efficiency",
    kpi_key: "soc_depletion_rate_per_100km",
    formula: "100 / median(delta_km / delta_soc_pct)",
    required_signals: &["distance.odometer", "ev.soc_pct"],
    optional_signals: &[],
};

pub(super) const EV_RANGE_EFFICIENCY_SCORE: LockedKpiSpec = LockedKpiSpec {
    ranking_type: "ev_range_efficiency",
    kpi_key: "ev_range_efficiency_score",
    formula: "0.65 * normalized_efficiency_component + 0.35 * normalized_estimated_range_component",
    required_signals: &["distance.odometer", "ev.soc_pct"],
    optional_signals: &["speed.vehicle", "environment.ambient_temp_c"],
};
