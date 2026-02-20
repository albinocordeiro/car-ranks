use super::LockedKpiSpec;

mod charging_performance;
mod composite;
mod range_efficiency;
mod temperature_impact;

/// Static KPI catalog used for provenance logging and contract validation.
pub(crate) const LOCKED_KPI_SPECS: &[LockedKpiSpec] = &[
    range_efficiency::EV_NET_ENERGY_EFFICIENCY,
    range_efficiency::EV_ESTIMATED_PRACTICAL_RANGE,
    range_efficiency::EV_URBAN_EFFICIENCY,
    range_efficiency::EV_HIGHWAY_EFFICIENCY,
    range_efficiency::REGENERATION_RECOVERY_RATIO,
    range_efficiency::SOC_DEPLETION_RATE_PER_100KM,
    range_efficiency::EV_RANGE_EFFICIENCY_SCORE,
    charging_performance::TEMP_ADJUSTED_CHARGE_ACCEPTANCE_SCORE,
    charging_performance::COLD_WEATHER_CHARGE_SPEED_RETENTION,
    charging_performance::CHARGING_PERFORMANCE_SCORE,
    temperature_impact::COLD_WEATHER_RANGE_RETENTION,
    temperature_impact::RANGE_TEMPERATURE_SENSITIVITY_INDEX,
    temperature_impact::COLD_WEATHER_CHARGE_SPEED_RETENTION,
    composite::EV_COMPOSITE_BASE_SCORE,
    composite::EV_HEALTH_MODIFIER_PENALTY,
    composite::EV_COMPOSITE_SCORE,
];
