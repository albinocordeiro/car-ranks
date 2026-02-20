use super::super::LockedKpiSpec;

pub(super) const EV_COMPOSITE_BASE_SCORE: LockedKpiSpec = LockedKpiSpec {
    ranking_type: "ev_composite",
    kpi_key: "ev_composite_base_score",
    formula: "0.6 * ev_range_efficiency_score + 0.4 * charging_performance_score",
    required_signals: &[],
    optional_signals: &[],
};

pub(super) const EV_HEALTH_MODIFIER_PENALTY: LockedKpiSpec = LockedKpiSpec {
    ranking_type: "ev_composite",
    kpi_key: "ev_health_modifier_penalty",
    formula: "min(10, (MIL_ON ? 6 : 0) + min(4, 0.5 * distinct_active_dtc_count))",
    required_signals: &["diag.mil_on", "diag.dtcs_active"],
    optional_signals: &[],
};

pub(super) const EV_COMPOSITE_SCORE: LockedKpiSpec = LockedKpiSpec {
    ranking_type: "ev_composite",
    kpi_key: "ev_composite_score",
    formula: "clamp(ev_composite_base_score - ev_health_modifier_penalty, 0, 100)",
    required_signals: &[],
    optional_signals: &[],
};
